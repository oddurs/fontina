// SPDX-License-Identifier: GPL-3.0-or-later
//
// fontina — a font manager.
// Copyright (C) 2026 Oddur Sigurdsson
//
// This program is free software: you can redistribute it and/or modify it under the
// terms of the GNU General Public License as published by the Free Software Foundation,
// either version 3 of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
// PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with this
// program. If not, see <https://www.gnu.org/licenses/>.

//! `fontina ui`: a keyboard-first browser over the index. Facets on the left, families
//! or faces in the middle, details and a shaped preview on the right. Every action is
//! one the CLI can do, and the status line shows the equivalent command.
//!
//! The palette is the terminal's own 16 colours; truecolor is used only for the
//! preview, so the screen looks native in any theme.

mod preview;

use anyhow::Result;
use fontina_core::index::FacetCount;
use fontina_core::{ActivationState, FaceFilter, FaceMetadata, FaceSummary, Facets, Family, Index};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

/// Which facet dimension a row belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Facet {
    Weight,
    Width,
    Style,
    Variable,
    Color,
    Script,
    License,
    Vendor,
    Tag,
    Collection,
    Activation,
    Container,
    Source,
}

impl Facet {
    fn label(self) -> &'static str {
        match self {
            Facet::Weight => "Weight",
            Facet::Width => "Width",
            Facet::Style => "Style",
            Facet::Variable => "Variable",
            Facet::Color => "Color",
            Facet::Script => "Script",
            Facet::License => "License",
            Facet::Vendor => "Vendor",
            Facet::Tag => "Tag",
            Facet::Collection => "Collection",
            Facet::Activation => "Activation",
            Facet::Container => "Container",
            Facet::Source => "Source",
        }
    }
    fn flag(self) -> &'static str {
        match self {
            Facet::Weight => "--weight",
            Facet::Width => "--width",
            Facet::Style => "--italic",
            Facet::Variable => "--variable",
            Facet::Color => "--color",
            Facet::Script => "--script",
            Facet::License => "--license",
            Facet::Vendor => "--vendor",
            Facet::Tag => "--tag",
            Facet::Collection => "--collection",
            Facet::Activation => "--activation",
            Facet::Container => "--container",
            Facet::Source => "--under",
        }
    }
}

struct FacetRow {
    facet: Facet,
    value: String,
    count: i64,
    header: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Facets,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputKind {
    Search,
    Tag,
    Collection,
    Text,
}

struct Input {
    kind: InputKind,
    buf: String,
}

pub struct App {
    index: Index,
    query: String,
    selected: BTreeMap<Facet, String>,
    facets: Facets,
    rows: Vec<FacetRow>,
    families: Vec<Family>,
    faces: Vec<FaceSummary>,
    /// `Some(name)` while a family is open.
    open_family: Option<String>,
    focus: Focus,
    list: ListState,
    facet_list: ListState,
    input: Option<Input>,
    status: String,
    help: bool,
    preview_text: Option<String>,
    preview_size: f32,
    detail: Option<FaceMetadata>,
    detail_id: Option<i64>,
    preview: preview::Cache,
}

pub fn run(db: &Path) -> Result<()> {
    let index = Index::open(db)?;
    let mut app = App::new(index)?;
    let mut terminal = ratatui::try_init()?;
    let result = app.event_loop(&mut terminal);
    ratatui::restore();
    result
}

impl App {
    fn new(index: Index) -> Result<Self> {
        let mut app = App {
            index,
            query: String::new(),
            selected: BTreeMap::new(),
            facets: Facets::default(),
            rows: Vec::new(),
            families: Vec::new(),
            faces: Vec::new(),
            open_family: None,
            focus: Focus::List,
            list: ListState::default(),
            facet_list: ListState::default(),
            input: None,
            status: String::new(),
            help: false,
            preview_text: None,
            preview_size: 28.0,
            detail: None,
            detail_id: None,
            preview: preview::Cache::default(),
        };
        app.reload()?;
        if app.families.is_empty() && app.selected.is_empty() && app.query.is_empty() {
            app.status =
                "index is empty: run `fontina scan <dir>` or `fontina scan --system`".into();
        }
        Ok(app)
    }

    // ----- data -----

    fn filter(&self) -> FaceFilter {
        let mut f = FaceFilter {
            query: (!self.query.is_empty()).then(|| self.query.clone()),
            family: self.open_family.clone(),
            ..Default::default()
        };
        for (facet, v) in &self.selected {
            match facet {
                Facet::Weight => {
                    let b: u16 = v.parse().unwrap_or(400);
                    f.weight = Some((b.saturating_sub(50), b + 49));
                }
                Facet::Width => {
                    let b: f32 = v.parse().unwrap_or(100.0);
                    f.width = Some(((b - 6.0).max(0.0) as u16, (b + 6.0) as u16));
                }
                Facet::Style => f.italic = Some(v == "italic"),
                Facet::Variable => f.variable = Some(true),
                Facet::Color => f.color = Some(true),
                Facet::Script => f.script = Some(v.clone()),
                Facet::License => f.license = Some(v.clone()),
                Facet::Vendor => f.vendor = Some(v.clone()),
                Facet::Tag => f.tag = Some(v.clone()),
                Facet::Collection => f.collection = Some(v.clone()),
                Facet::Activation => {
                    if v == "none" {
                        f.active = Some(false);
                    } else {
                        f.activation = v.parse().ok();
                    }
                }
                Facet::Container => f.container = Some(v.clone()),
                Facet::Source => f.path_prefix = Some(v.clone()),
            }
        }
        f
    }

    /// The CLI command that shows what the screen shows.
    fn command_line(&self) -> String {
        let mut s = String::from(if self.open_family.is_some() {
            "fontina list"
        } else {
            "fontina families"
        });
        if !self.query.is_empty() {
            s.push_str(&format!(" {:?}", self.query));
        }
        if let Some(f) = &self.open_family {
            s.push_str(&format!(" --family {f:?}"));
        }
        for (facet, v) in &self.selected {
            match facet {
                Facet::Variable | Facet::Color => s.push_str(&format!(" {}", facet.flag())),
                Facet::Style => s.push_str(&format!(" --italic={}", v == "italic")),
                Facet::Weight => {
                    let b: u16 = v.parse().unwrap_or(400);
                    s.push_str(&format!(" --weight {}-{}", b.saturating_sub(50), b + 49));
                }
                Facet::Width => {
                    let b: f32 = v.parse().unwrap_or(100.0);
                    s.push_str(&format!(
                        " --width {}-{}",
                        (b - 6.0).max(0.0) as u16,
                        (b + 6.0) as u16
                    ));
                }
                Facet::Activation if v == "none" => s.push_str(" --active=false"),
                _ => s.push_str(&format!(" {} {}", facet.flag(), shell_quote(v))),
            }
        }
        s
    }

    fn reload(&mut self) -> Result<()> {
        let filter = self.filter();
        self.facets = self.index.facets(&FaceFilter {
            family: None,
            ..filter.clone()
        })?;
        if self.open_family.is_some() {
            self.faces = self.index.list(&filter)?;
            self.families.clear();
        } else {
            self.families = self.index.families(&filter)?;
            self.faces.clear();
        }
        self.rows = build_rows(&self.facets, &self.selected);
        let len = self.list_len();
        let sel = self.list.selected().unwrap_or(0).min(len.saturating_sub(1));
        self.list.select((len > 0).then_some(sel));
        if self.facet_list.selected().is_none() && !self.rows.is_empty() {
            self.facet_list
                .select(Some(first_selectable(&self.rows, 0)));
        }
        self.refresh_detail()?;
        Ok(())
    }

    fn list_len(&self) -> usize {
        if self.open_family.is_some() {
            self.faces.len()
        } else {
            self.families.len()
        }
    }

    /// The face the right pane describes: the selected face, or a family's representative.
    fn current_face_id(&self) -> Option<i64> {
        let i = self.list.selected()?;
        if self.open_family.is_some() {
            self.faces.get(i).map(|f| f.id)
        } else {
            self.families.get(i).map(|f| f.representative)
        }
    }

    /// Every face the current selection stands for (all faces of a family).
    fn current_face_ids(&self) -> Vec<i64> {
        let Some(i) = self.list.selected() else {
            return Vec::new();
        };
        if self.open_family.is_some() {
            self.faces.get(i).map(|f| vec![f.id]).unwrap_or_default()
        } else {
            self.families
                .get(i)
                .map(|f| f.ids.clone())
                .unwrap_or_default()
        }
    }

    fn refresh_detail(&mut self) -> Result<()> {
        let id = self.current_face_id();
        if id != self.detail_id {
            self.detail_id = id;
            self.detail = match id {
                Some(id) => self.index.get_face(id)?,
                None => None,
            };
        }
        Ok(())
    }

    // ----- events -----

    fn event_loop(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;
            if !event::poll(Duration::from_millis(250))? {
                continue;
            }
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if self.input.is_some() {
                self.handle_input_key(key.code)?;
                continue;
            }
            if self.help {
                self.help = false;
                continue;
            }
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('c') if ctrl => return Ok(()),
                KeyCode::Esc => {
                    if self.open_family.is_some() {
                        self.close_family()?;
                    } else if !self.query.is_empty() || !self.selected.is_empty() {
                        self.query.clear();
                        self.selected.clear();
                        self.reload()?;
                    } else {
                        return Ok(());
                    }
                }
                KeyCode::Char('?') => self.help = true,
                KeyCode::Tab => {
                    self.focus = match self.focus {
                        Focus::Facets => Focus::List,
                        Focus::List => Focus::Facets,
                    }
                }
                KeyCode::Char('/') => self.start_input(InputKind::Search, self.query.clone()),
                KeyCode::Char('e') => {
                    let text = self.preview_text.clone().unwrap_or_default();
                    self.start_input(InputKind::Text, text)
                }
                KeyCode::Char('t') => self.start_input(InputKind::Tag, String::new()),
                KeyCode::Char('c') => self.start_input(InputKind::Collection, String::new()),
                KeyCode::Char('x') => {
                    self.selected.clear();
                    self.query.clear();
                    self.reload()?;
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    self.preview_size = (self.preview_size + 4.0).min(160.0)
                }
                KeyCode::Char('-') => self.preview_size = (self.preview_size - 4.0).max(8.0),
                KeyCode::Char('a') => self.activate(ActivationState::User)?,
                KeyCode::Char('A') => self.activate(ActivationState::Session)?,
                KeyCode::Char('i') => self.activate(ActivationState::Installed)?,
                KeyCode::Char('d') => self.deactivate(false)?,
                KeyCode::Char('u') => self.deactivate(true)?,
                KeyCode::Char('R') => self.rescan()?,
                KeyCode::Down | KeyCode::Char('j') => self.step(1)?,
                KeyCode::Up | KeyCode::Char('k') => self.step(-1)?,
                KeyCode::PageDown | KeyCode::Char('f') if ctrl || key.code == KeyCode::PageDown => {
                    self.step(15)?
                }
                KeyCode::PageUp | KeyCode::Char('b') if ctrl || key.code == KeyCode::PageUp => {
                    self.step(-15)?
                }
                KeyCode::Home | KeyCode::Char('g') => self.jump(0)?,
                KeyCode::End | KeyCode::Char('G') => self.jump(usize::MAX)?,
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Right | KeyCode::Char('l') => {
                    match self.focus {
                        Focus::Facets => self.toggle_facet()?,
                        Focus::List => self.open_family()?,
                    }
                }
                KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h')
                    if self.focus == Focus::List =>
                {
                    self.close_family()?
                }
                _ => {}
            }
        }
    }

    fn start_input(&mut self, kind: InputKind, buf: String) {
        if matches!(kind, InputKind::Tag | InputKind::Collection)
            && self.current_face_ids().is_empty()
        {
            self.status = "nothing selected".into();
            return;
        }
        self.input = Some(Input { kind, buf });
    }

    fn handle_input_key(&mut self, code: KeyCode) -> Result<()> {
        let Some(input) = self.input.as_mut() else {
            return Ok(());
        };
        match code {
            KeyCode::Esc => self.input = None,
            KeyCode::Backspace => {
                input.buf.pop();
                if input.kind == InputKind::Search {
                    self.query = self
                        .input
                        .as_ref()
                        .map(|i| i.buf.clone())
                        .unwrap_or_default();
                    self.reload()?;
                }
            }
            KeyCode::Enter => {
                let Input { kind, buf } = self.input.take().expect("checked");
                let value = buf.trim().to_string();
                match kind {
                    InputKind::Search => {
                        self.query = value;
                        self.reload()?;
                    }
                    InputKind::Text => {
                        self.preview_text = (!value.is_empty()).then_some(value);
                    }
                    InputKind::Tag => {
                        if !value.is_empty() {
                            let ids = self.current_face_ids();
                            let n = self.index.tag(&ids, &value)?;
                            self.status = format!(
                                "tagged {n} face(s) with {value:?}   (fontina tag add {} <targets>)",
                                shell_quote(&value)
                            );
                            self.reload()?;
                        }
                    }
                    InputKind::Collection => {
                        if !value.is_empty() {
                            let ids = self.current_face_ids();
                            let n = self.index.add_to_collection(&value, &ids)?;
                            self.status = format!(
                                "added {n} face(s) to {value:?}   (fontina collection add {} <targets>)",
                                shell_quote(&value)
                            );
                            self.reload()?;
                        }
                    }
                }
            }
            KeyCode::Char(c) => {
                input.buf.push(c);
                if input.kind == InputKind::Search {
                    self.query = self
                        .input
                        .as_ref()
                        .map(|i| i.buf.clone())
                        .unwrap_or_default();
                    self.reload()?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn step(&mut self, delta: i32) -> Result<()> {
        match self.focus {
            Focus::List => {
                let len = self.list_len();
                if len == 0 {
                    return Ok(());
                }
                let cur = self.list.selected().unwrap_or(0) as i32;
                let next = (cur + delta).clamp(0, len as i32 - 1) as usize;
                self.list.select(Some(next));
                self.refresh_detail()?;
            }
            Focus::Facets => {
                if self.rows.is_empty() {
                    return Ok(());
                }
                let cur = self.facet_list.selected().unwrap_or(0) as i32;
                let mut next = (cur + delta).clamp(0, self.rows.len() as i32 - 1) as usize;
                // Skip headers in the direction of travel.
                while self.rows[next].header {
                    let n = next as i32 + delta.signum();
                    if n < 0 || n >= self.rows.len() as i32 {
                        return Ok(());
                    }
                    next = n as usize;
                }
                self.facet_list.select(Some(next));
            }
        }
        Ok(())
    }

    fn jump(&mut self, to: usize) -> Result<()> {
        match self.focus {
            Focus::List => {
                let len = self.list_len();
                if len > 0 {
                    self.list.select(Some(to.min(len - 1)));
                    self.refresh_detail()?;
                }
            }
            Focus::Facets => {
                if !self.rows.is_empty() {
                    let i = to.min(self.rows.len() - 1);
                    self.facet_list
                        .select(Some(first_selectable(&self.rows, i)));
                }
            }
        }
        Ok(())
    }

    fn toggle_facet(&mut self) -> Result<()> {
        let Some(i) = self.facet_list.selected() else {
            return Ok(());
        };
        let Some(row) = self.rows.get(i) else {
            return Ok(());
        };
        if row.header {
            return Ok(());
        }
        let (facet, value) = (row.facet, row.value.clone());
        if self.selected.get(&facet) == Some(&value) {
            self.selected.remove(&facet);
        } else {
            self.selected.insert(facet, value);
        }
        self.list.select(Some(0));
        self.reload()
    }

    fn open_family(&mut self) -> Result<()> {
        if self.open_family.is_some() {
            return Ok(());
        }
        let Some(i) = self.list.selected() else {
            return Ok(());
        };
        let Some(fam) = self.families.get(i) else {
            return Ok(());
        };
        self.open_family = Some(fam.name.clone());
        self.list.select(Some(0));
        self.reload()
    }

    fn close_family(&mut self) -> Result<()> {
        let Some(name) = self.open_family.take() else {
            return Ok(());
        };
        self.reload()?;
        if let Some(i) = self.families.iter().position(|f| f.name == name) {
            self.list.select(Some(i));
            self.refresh_detail()?;
        }
        Ok(())
    }

    // ----- actions -----

    fn activate(&mut self, state: ActivationState) -> Result<()> {
        let ids = self.current_face_ids();
        if ids.is_empty() {
            self.status = "nothing selected".into();
            return Ok(());
        }
        let conflicts = crate::collect_conflicts(&self.index, &ids)?;
        if !conflicts.is_empty() {
            let c = &conflicts[0];
            self.status = format!(
                "{} conflict(s): {} {} ({}). Use `fontina activate --replace` to override.",
                conflicts.len(),
                c.face.family,
                c.face.subfamily,
                c.reason
            );
            return Ok(());
        }
        let activator = fontina_platform::activator();
        let verb = match state {
            ActivationState::Installed => "install",
            ActivationState::Session => "activate --session",
            ActivationState::User => "activate",
        };
        let mut n = 0;
        for (path, faces) in crate::files_for(&self.index, &ids)? {
            let result = match state {
                ActivationState::Installed => activator.install(&path).map(|p| {
                    self.index
                        .set_activation(&faces, state, Some(&p.to_string_lossy()))
                        .map(|_| ())
                        .map_err(|e| fontina_platform::PlatformError::Os(e.to_string()))
                }),
                _ => {
                    let scope = if state == ActivationState::Session {
                        fontina_platform::Scope::Session
                    } else {
                        fontina_platform::Scope::User
                    };
                    activator.activate(&path, scope).map(|_| {
                        self.index
                            .set_activation(&faces, state, None)
                            .map_err(|e| fontina_platform::PlatformError::Os(e.to_string()))
                    })
                }
            };
            match result.and_then(|r| r) {
                Ok(()) => n += faces.len(),
                Err(e) => {
                    self.status = format!("{}: {e}", path.display());
                    self.reload()?;
                    return Ok(());
                }
            }
        }
        self.status = format!("{verb}: {n} face(s)   (fontina {verb} <targets>)");
        self.reload()
    }

    fn deactivate(&mut self, uninstall: bool) -> Result<()> {
        let ids = self.current_face_ids();
        if ids.is_empty() {
            self.status = "nothing selected".into();
            return Ok(());
        }
        let activator = fontina_platform::activator();
        let mut n = 0;
        for (path, faces) in crate::files_for(&self.index, &ids)? {
            let record = self.index.activation(faces[0])?;
            let result = if uninstall {
                match record.and_then(|r| r.installed_path) {
                    Some(p) => activator.uninstall(Path::new(&p)),
                    None => continue,
                }
            } else {
                if record.is_none() {
                    continue;
                }
                activator.deactivate(&path)
            };
            match result {
                Ok(()) => {
                    self.index.clear_activation(&faces)?;
                    n += faces.len();
                }
                Err(e) => {
                    self.status = format!("{}: {e}", path.display());
                    self.reload()?;
                    return Ok(());
                }
            }
        }
        let verb = if uninstall { "uninstall" } else { "deactivate" };
        self.status = format!("{verb}: {n} face(s)   (fontina {verb} <targets>)");
        self.reload()
    }

    fn rescan(&mut self) -> Result<()> {
        let roots: Vec<std::path::PathBuf> = self
            .index
            .sources()?
            .into_iter()
            .filter(|s| Path::new(&s.path).is_dir())
            .map(|s| s.path.into())
            .collect();
        if roots.is_empty() {
            self.status = "no sources to rescan".into();
            return Ok(());
        }
        let report = fontina_core::scan::scan(
            &mut self.index,
            &roots,
            &fontina_core::ScanOptions {
                prune: true,
                ..Default::default()
            },
        )?;
        self.status = format!(
            "rescanned {} source(s): {} parsed, {} unchanged, {} removed, {} failed   (fontina scan --prune)",
            roots.len(),
            report.parsed,
            report.unchanged,
            report.removed,
            report.failed.len()
        );
        self.preview.clear();
        self.reload()
    }

    // ----- drawing -----

    fn draw(&mut self, f: &mut ratatui::Frame) {
        let area = f.area();
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(26),
                Constraint::Percentage(40),
                Constraint::Min(30),
            ])
            .split(vertical[0]);
        self.draw_facets(f, columns[0]);
        self.draw_list(f, columns[1]);
        self.draw_detail(f, columns[2]);
        self.draw_status(f, vertical[1]);
        self.draw_keys(f, vertical[2]);
        if self.help {
            self.draw_help(f, area);
        }
    }

    fn border(&self, focused: bool) -> Style {
        if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        }
    }

    fn draw_facets(&mut self, f: &mut ratatui::Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .rows
            .iter()
            .map(|r| {
                if r.header {
                    ListItem::new(Line::from(Span::styled(
                        r.facet.label().to_string(),
                        Style::default().add_modifier(Modifier::BOLD),
                    )))
                } else {
                    let on = self.selected.get(&r.facet) == Some(&r.value);
                    let mark = if on { "●" } else { " " };
                    let label = facet_value_label(r.facet, &r.value);
                    let width = area.width.saturating_sub(4) as usize;
                    let count = r.count.to_string();
                    let room = width.saturating_sub(count.len() + 2);
                    let text = format!(
                        "{mark} {:<room$} {count}",
                        truncate(&label, room),
                        room = room
                    );
                    let style = if on {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Line::from(Span::styled(text, style)))
                }
            })
            .collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.border(self.focus == Focus::Facets))
                    .title(format!(" {} faces ", self.facets.faces)),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, area, &mut self.facet_list);
    }

    fn draw_list(&mut self, f: &mut ratatui::Frame, area: Rect) {
        let width = area.width.saturating_sub(2) as usize;
        let items: Vec<ListItem> = if let Some(fam) = &self.open_family {
            let _ = fam;
            self.faces
                .iter()
                .map(|face| {
                    let flags = format!(
                        "{}{}{}",
                        if face.variable { "V" } else { " " },
                        if face.color { "C" } else { " " },
                        activation_mark(face.activation),
                    );
                    let tags = if face.tags.is_empty() {
                        String::new()
                    } else {
                        format!("  [{}]", face.tags.join(", "))
                    };
                    let left = format!("{} {}{}", face.subfamily, face.container, tags);
                    ListItem::new(Line::from(format!(
                        "{:<w$} {flags}",
                        truncate(&left, width.saturating_sub(5)),
                        w = width.saturating_sub(5)
                    )))
                })
                .collect()
        } else {
            self.families
                .iter()
                .map(|fam| {
                    let flags = format!(
                        "{}{}{}",
                        if fam.variable { "V" } else { " " },
                        if fam.color { "C" } else { " " },
                        if fam.active > 0 { "●" } else { " " },
                    );
                    let count = format!("{:>3}", fam.faces);
                    let room = width.saturating_sub(9);
                    ListItem::new(Line::from(format!(
                        "{:<room$} {count} {flags}",
                        truncate(&fam.name, room),
                        room = room
                    )))
                })
                .collect()
        };
        let title = match &self.open_family {
            Some(fam) => format!(" {} · {} face(s) ", fam, self.faces.len()),
            None => format!(" {} families ", self.families.len()),
        };
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.border(self.focus == Focus::List))
                    .title(title),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, area, &mut self.list);
    }

    fn draw_detail(&mut self, f: &mut ratatui::Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.border(false))
            .title(" Details ");
        let inner = block.inner(area);
        f.render_widget(block, area);
        let Some(face) = self.detail.clone() else {
            f.render_widget(Paragraph::new("Nothing selected."), inner);
            return;
        };
        let mut lines: Vec<Line> = Vec::new();
        let bold = Style::default().add_modifier(Modifier::BOLD);
        lines.push(Line::from(vec![
            Span::styled(face.names.family.clone(), bold),
            Span::raw(" "),
            Span::raw(face.names.subfamily.clone()),
        ]));
        let kv = |k: &str, v: String| {
            Line::from(vec![
                Span::styled(format!("{k:<10}"), Style::default().fg(Color::DarkGray)),
                Span::raw(v),
            ])
        };
        lines.push(kv(
            "style",
            format!(
                "weight {} · width {}% · {}",
                face.style.weight.round(),
                face.style.width.round(),
                face.style.css.style
            ),
        ));
        if let Some(v) = &face.variable {
            lines.push(kv(
                "axes",
                v.axes
                    .iter()
                    .map(|a| format!("{} {}–{} ({})", a.tag, a.min, a.max, a.default))
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }
        lines.push(kv(
            "glyphs",
            format!(
                "{} · {} codepoints · {}",
                face.glyph_count,
                face.coverage.codepoints,
                face.coverage
                    .scripts
                    .iter()
                    .take(5)
                    .map(|s| s.script.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        ));
        let feats = face.features.gsub.len() + face.features.gpos.len();
        if feats > 0 {
            lines.push(kv(
                "features",
                format!(
                    "{feats}: {}",
                    face.features
                        .gsub
                        .iter()
                        .chain(face.features.gpos.iter())
                        .take(12)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
            ));
        }
        lines.push(kv(
            "license",
            format!(
                "{}{}",
                face.license.spdx.as_deref().unwrap_or("none embedded"),
                face.os2
                    .as_ref()
                    .map(|o| format!(" · {:?}", o.embedding.level))
                    .unwrap_or_default()
            ),
        ));
        if let Some(d) = face
            .names
            .designer
            .as_deref()
            .or(face.names.manufacturer.as_deref())
        {
            lines.push(kv("designer", d.to_string()));
        }
        let summary = self
            .index
            .summaries(&[self.detail_id.unwrap_or(0)])
            .ok()
            .and_then(|v| v.into_iter().next());
        if let Some(s) = &summary {
            if !s.tags.is_empty() {
                lines.push(kv("tags", s.tags.join(", ")));
            }
            lines.push(kv(
                "state",
                match s.activation {
                    Some(a) => a.as_str().to_string(),
                    None => "not active".into(),
                },
            ));
        }
        lines.push(kv(
            "file",
            format!(
                "{}{}",
                face.file.path,
                if face.file.face_count > 1 {
                    format!(" #{}", face.index)
                } else {
                    String::new()
                }
            ),
        ));
        lines.push(Line::from(""));
        let text_rows = lines.len() as u16;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(text_rows.min(inner.height)),
                Constraint::Min(0),
            ])
            .split(inner);
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), chunks[0]);
        let preview_area = chunks[1];
        if preview_area.height < 2 || preview_area.width < 4 {
            return;
        }
        let text = self
            .preview_text
            .clone()
            .or_else(|| face.names.sample_text.clone())
            .unwrap_or_else(|| preview::sample_for(&face));
        let lines = self.preview.lines(
            &face,
            &text,
            self.preview_size,
            preview_area.width as u32,
            preview_area.height as u32 * 2,
        );
        f.render_widget(Paragraph::new(lines), preview_area);
    }

    fn draw_status(&self, f: &mut ratatui::Frame, area: Rect) {
        let line = if let Some(input) = &self.input {
            let prompt = match input.kind {
                InputKind::Search => "search",
                InputKind::Tag => "tag",
                InputKind::Collection => "collection",
                InputKind::Text => "preview text",
            };
            Line::from(vec![
                Span::styled(format!(" {prompt}: "), Style::default().fg(Color::Cyan)),
                Span::raw(input.buf.clone()),
                Span::styled("▏", Style::default().fg(Color::Cyan)),
            ])
        } else if !self.status.is_empty() {
            Line::from(Span::raw(format!(" {}", self.status)))
        } else {
            Line::from(Span::styled(
                format!(" $ {}", self.command_line()),
                Style::default().fg(Color::DarkGray),
            ))
        };
        f.render_widget(Paragraph::new(line), area);
    }

    fn draw_keys(&self, f: &mut ratatui::Frame, area: Rect) {
        let keys = " / search  ⇥ facets  ⏎ open  ⌫ back  t tag  c collection  a/A activate  d deactivate  i install  u uninstall  e text  +/- size  R rescan  ? help  q quit";
        f.render_widget(
            Paragraph::new(Span::styled(keys, Style::default().fg(Color::DarkGray))),
            area,
        );
    }

    fn draw_help(&self, f: &mut ratatui::Frame, area: Rect) {
        let text = "\
 fontina ui

 Move        j/k ↑/↓ PgUp/PgDn g/G        Tab switches between facets and the list
 Filter      / type to search  Esc clears   Enter/Space toggles a facet   x clears all
 Families    Enter opens a family, Backspace/Esc closes it
 Organise    t tag the selection   c add it to a collection
 Activate    a for the user, A until logout, i install a copy, d deactivate, u uninstall
 Preview     e sets the sample text   + / - change the size
 Index       R rescans every source (fontina scan --prune)
 Quit        q

 The status line shows the CLI command for what you see. Everything here is a command.

 any key to close";
        let w = 90.min(area.width);
        let h = 18.min(area.height);
        let rect = Rect::new(
            area.x + (area.width - w) / 2,
            area.y + (area.height - h) / 2,
            w,
            h,
        );
        f.render_widget(Clear, rect);
        f.render_widget(
            Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            ),
            rect,
        );
    }
}

fn build_rows(facets: &Facets, selected: &BTreeMap<Facet, String>) -> Vec<FacetRow> {
    let mut rows = Vec::new();
    let mut section = |facet: Facet, counts: &[FacetCount], cap: usize| {
        if counts.is_empty() {
            return;
        }
        rows.push(FacetRow {
            facet,
            value: String::new(),
            count: 0,
            header: true,
        });
        let chosen = selected.get(&facet);
        for c in counts.iter().take(cap) {
            rows.push(FacetRow {
                facet,
                value: c.value.clone(),
                count: c.count,
                header: false,
            });
        }
        // Keep a selected value visible even when it is past the cap.
        if let Some(v) = chosen
            && !counts.iter().take(cap).any(|c| &c.value == v)
            && let Some(c) = counts.iter().find(|c| &c.value == v)
        {
            rows.push(FacetRow {
                facet,
                value: c.value.clone(),
                count: c.count,
                header: false,
            });
        }
    };
    let flags = [FacetCount {
        value: "variable".into(),
        count: facets.variable,
    }];
    let color = [FacetCount {
        value: "color".into(),
        count: facets.color,
    }];
    section(Facet::Weight, &facets.weight, 9);
    section(Facet::Width, &facets.width, 9);
    section(Facet::Style, &facets.style, 2);
    if facets.variable > 0 {
        section(Facet::Variable, &flags, 1);
    }
    if facets.color > 0 {
        section(Facet::Color, &color, 1);
    }
    section(Facet::Script, &facets.script, 8);
    section(Facet::License, &facets.license, 6);
    section(Facet::Tag, &facets.tag, 10);
    section(Facet::Collection, &facets.collection, 10);
    section(Facet::Activation, &facets.activation, 4);
    section(Facet::Vendor, &facets.vendor, 6);
    section(Facet::Container, &facets.container, 5);
    section(Facet::Source, &facets.source, 6);
    rows
}

fn first_selectable(rows: &[FacetRow], from: usize) -> usize {
    (from..rows.len())
        .find(|&i| !rows[i].header)
        .or_else(|| (0..from).rev().find(|&i| !rows[i].header))
        .unwrap_or(0)
}

fn facet_value_label(facet: Facet, value: &str) -> String {
    match facet {
        Facet::Weight => format!(
            "{value} {}",
            fontina_core::index::weight_name(value.parse().unwrap_or(400))
        ),
        Facet::Width => format!(
            "{value}% {}",
            fontina_core::index::width_name(value.parse().unwrap_or(100.0))
        ),
        Facet::Source => Path::new(value)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| value.to_string()),
        _ => value.to_string(),
    }
}

fn activation_mark(a: Option<ActivationState>) -> &'static str {
    match a {
        Some(ActivationState::Session) => "s",
        Some(ActivationState::User) => "●",
        Some(ActivationState::Installed) => "i",
        None => " ",
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n.saturating_sub(1)).collect::<String>() + "…"
    }
}

fn shell_quote(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_alphanumeric() || "-_./:@+=".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}
