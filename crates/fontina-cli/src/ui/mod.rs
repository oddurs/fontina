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

mod controls;
mod filter;
mod glyphs;
mod history;
mod layout;
mod palette;
mod search;
mod theme;

use anyhow::Result;
use fontina_core::index::FacetCount;
use fontina_core::model::EmbeddingLevel;
use fontina_core::{ActivationState, FaceFilter, FaceMetadata, FaceSummary, Facets, Family, Index};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use std::collections::{BTreeMap, BTreeSet};
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
    Spacing,
    Script,
    Language,
    License,
    Freedom,
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
            Facet::Spacing => "Spacing",
            Facet::Script => "Script",
            Facet::Language => "Language",
            Facet::License => "License",
            Facet::Freedom => "Freedom",
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
            Facet::Spacing => "--mono",
            Facet::Script => "--script",
            Facet::Language => "--lang",
            Facet::License => "--license",
            Facet::Freedom => "--freedom",
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
    /// The face pane. Beside the others it is a readout and Tab only stops on it when
    /// the face offers axes or features to move; alone on a narrow screen it is the
    /// only view of a face there is, so it always takes focus. `layout::Shape` decides
    /// which of those is true, and `controls_active` is how the keys tell them apart.
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputKind {
    Search,
    Tag,
    Collection,
    /// A codepoint or a block name, in the glyph map.
    Glyph,
    /// The flags `fontina list` takes, applied to the panes as they are typed.
    Filter,
}

struct Input {
    kind: InputKind,
    buf: String,
}

pub struct App {
    index: Index,
    /// The palette, resolved once against what this terminal can show.
    theme: theme::Theme,
    query: String,
    selected: BTreeMap<Facet, String>,
    facets: Facets,
    rows: Vec<FacetRow>,
    families: Vec<Family>,
    faces: Vec<FaceSummary>,
    /// `Some(name)` while a family is open.
    open_family: Option<String>,
    /// Faces the next action will touch, when the reader has picked more than one.
    ///
    /// Empty means "the row under the cursor", which is what every action meant before
    /// there was a set, and still means when nobody has marked anything. Faces rather
    /// than rows, because a row is a family in one view and a face in another and the
    /// mark has to survive opening the family it stands for.
    marked: BTreeSet<i64>,
    /// The filter bar's line, while one is set.
    ///
    /// Kept as the text the reader typed rather than only as the parsed filter, because
    /// it is also the command line the status bar shows: rebuilding flags from a parsed
    /// `FaceFilter` would be a second renderer to keep in step with the first.
    filter_line: Option<String>,
    /// The same line, parsed. `None` while the line does not parse, which is most of
    /// the time somebody is typing one.
    filter_override: Option<FaceFilter>,
    /// What to put back if the reader presses Esc.
    filter_before: Option<(Option<String>, Option<FaceFilter>)>,
    /// Why the line does not parse, while it does not.
    filter_error: Option<String>,
    /// Where `v` was pressed, as a row, so the range can be redrawn as the cursor
    /// moves. Cleared when the range is committed or the view changes under it.
    mark_anchor: Option<usize>,
    focus: Focus,
    /// The cursor in the families or faces list. Only its selection is used: the pane
    /// windows the data itself before ratatui sees it, so the widget's own offset
    /// would be an answer to a question nobody asks it.
    list: ListState,
    facet_list: ListState,
    /// First row each of those panes drew on the last frame.
    ///
    /// Held here rather than derived from the selection, because a list scrolled
    /// halfway down and then filtered should stay where the reader left it, and
    /// because it is what tells the next frame which rows to build.
    list_offset: usize,
    facet_offset: usize,
    input: Option<Input>,
    status: String,
    help: bool,
    /// First line of the help the overlay is showing. Only ever non-zero on a terminal
    /// too short to hold all of it at once.
    help_scroll: u16,
    detail: Option<FaceMetadata>,
    detail_id: Option<i64>,
    /// The listing row for `detail_id`: tags and activation state, joined once per
    /// selection rather than once per frame.
    detail_summary: Option<FaceSummary>,
    /// Axis positions and feature toggles for the face on show. Rebuilt whenever the
    /// selection changes, because they describe that face and no other.
    controls: controls::Controls,
    /// The glyph map, while it is open. It covers the whole screen, so it is a mode
    /// rather than a pane: there is no room to browse and read coverage at once.
    glyphs: Option<glyphs::Glyphs>,
    /// Columns the glyph grid used when it was last drawn, so a PageDown moves by what
    /// the reader can see rather than by a guess.
    glyph_cols: usize,
    /// The waterfall or comparison, while it is open. Full-screen for the same reason
    /// the glyph map is: rendered type needs the width.
    /// Terminal lines the sheet had on the last frame, so a PageDown moves by a screen.
    /// How many panes the last frame had room for.
    ///
    /// Read by the key handler, written by the drawing: Tab has to cycle through the
    /// panes that are actually on the screen, and only the frame knows how wide the
    /// terminal was. A resize therefore reaches the keys one frame late, which is a
    /// frame the reader spends letting go of the mouse.
    shape: layout::Shape,
    /// How the browser registers a font with the operating system.
    ///
    /// A field rather than a call to `fontina_platform::activator()` at each use, so a
    /// test can drive `a`, `i`, `d` and `u` without touching the machine it runs on. It
    /// is not a nicety: the soak below presses every key hundreds of times, and against
    /// the real backend that meant copying fixtures into the developer's own font
    /// directory and registering them with the running session.
    activator: Box<dyn fontina_platform::FontActivator>,
    /// What the browser has done this session, and what it has taken back.
    history: history::History,
    /// The listing queries, on their own thread and their own connection.
    ///
    /// `None` when the index has no file behind it and there is nothing to reopen, in
    /// which case the browser answers its own questions the way it always did. Nothing
    /// else in here changes: a search is asked for the same way either way, and only
    /// the waiting is different.
    search: Option<search::Search>,
    /// The command palette, while it is open.
    palette: Option<palette::Palette>,
}

pub fn run(db: &Path) -> Result<()> {
    let index = Index::open(db)?;
    let mut app = App::new(index)?;
    app.use_depth(theme::Depth::detect());
    let mut terminal = ratatui::try_init()?;
    let result = app.event_loop(&mut terminal);
    ratatui::restore();
    result
}

/// What the event loop should do after a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    Continue,
    Quit,
}

impl App {
    fn new(index: Index) -> Result<Self> {
        Self::with_activator(index, fontina_platform::activator())
    }

    fn with_activator(
        index: Index,
        activator: Box<dyn fontina_platform::FontActivator>,
    ) -> Result<Self> {
        let mut app = App {
            index,
            query: String::new(),
            selected: BTreeMap::new(),
            facets: Facets::default(),
            rows: Vec::new(),
            theme: theme::Theme::default(),
            families: Vec::new(),
            faces: Vec::new(),
            open_family: None,
            marked: BTreeSet::new(),
            filter_line: None,
            filter_override: None,
            filter_before: None,
            filter_error: None,
            mark_anchor: None,
            focus: Focus::List,
            list: ListState::default(),
            facet_list: ListState::default(),
            list_offset: 0,
            facet_offset: 0,
            input: None,
            status: String::new(),
            help: false,
            help_scroll: 0,
            detail: None,
            detail_id: None,
            detail_summary: None,
            controls: controls::Controls::default(),
            glyphs: None,
            glyph_cols: 16,
            shape: layout::Shape::Three,
            activator,
            history: history::History::default(),
            search: None,
            palette: None,
        };
        app.search = search::Search::open(&app.index);
        app.reload()?;
        if app.families.is_empty() && app.selected.is_empty() && app.query.is_empty() {
            app.status =
                "index is empty: run `fontina scan <dir>` or `fontina scan --system`".into();
        }
        Ok(app)
    }

    // ----- data -----

    /// What the panes are showing, as a filter.
    ///
    /// The filter bar, when one is set, is the whole answer: it came from the command
    /// line's own parser and can say things the facet pane cannot, so mixing the two
    /// would mean deciding what `--variable=false` plus a "variable" facet means. The
    /// open family is the one thing that survives, because it is the pane the reader is
    /// standing in rather than part of the filter they wrote.
    fn filter(&self) -> FaceFilter {
        if let Some(f) = &self.filter_override {
            return FaceFilter {
                family: self.open_family.clone().or_else(|| f.family.clone()),
                ..f.clone()
            };
        }
        self.filter_from_facets()
    }

    fn filter_from_facets(&self) -> FaceFilter {
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
                Facet::Spacing => f.monospace = Some(v == "monospace"),
                Facet::Script => f.scripts = vec![v.clone()],
                Facet::Language => f.lang = Some(v.clone()),
                Facet::License => f.license = Some(v.clone()),
                Facet::Freedom => f.freedom = v.parse().ok(),
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
        // A filter that came from a typed line is already a command line; rebuilding it
        // from the parsed form would be a second renderer to keep in step with the
        // first, and the reader typed the flags they wanted to see.
        if let Some(line) = &self.filter_line {
            if !line.trim().is_empty() {
                s.push(' ');
                s.push_str(line.trim());
            }
            if let Some(f) = &self.open_family {
                s.push_str(&format!(" --family {f:?}"));
            }
            return s;
        }
        if !self.query.is_empty() {
            s.push_str(&format!(" {:?}", self.query));
        }
        if let Some(f) = &self.open_family {
            s.push_str(&format!(" --family {f:?}"));
        }
        for (facet, v) in &self.selected {
            match facet {
                Facet::Variable | Facet::Color => s.push_str(&format!(" {}", facet.flag())),
                // Two flags, neither taking a value. Without an arm here the generic one
                // below emits `--mono proportional`, which clap reads as `--mono` plus a
                // full-text search for "proportional": the opposite of what is on screen,
                // and it returns nothing rather than erring.
                Facet::Spacing => s.push_str(if v == "monospace" {
                    " --mono"
                } else {
                    " --proportional"
                }),
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

    /// Ask for the listing this filter describes, and do not wait for it.
    ///
    /// This is what a keystroke calls. It returns the generation asked for, which is
    /// what the callers that cannot carry on without the answer hand to `reload`.
    fn request_reload(&mut self) -> u64 {
        let ask = search::Ask {
            filter: self.filter(),
            open_family: self.open_family.is_some(),
        };
        match &mut self.search {
            Some(search) => search.ask(ask),
            None => 0,
        }
    }

    /// Ask, and wait.
    ///
    /// For everywhere the next thing that happens depends on the answer: the first
    /// frame, and the reload after a tag, an activation or a rescan, where the reader
    /// is about to look at exactly what changed. Typing is the one path that does not
    /// call this, which is the whole of the change.
    fn reload(&mut self) -> Result<()> {
        let generation = self.request_reload();
        let settled = match &mut self.search {
            Some(search) => search.settle(generation),
            None => {
                let ask = search::Ask {
                    filter: self.filter(),
                    open_family: self.open_family.is_some(),
                };
                Some(search::answer_here(&self.index, &ask))
            }
        };
        match settled {
            Some(listing) => self.apply(listing?)?,
            // The worker is gone, so nothing will ever arrive. Say so once and leave
            // the panes holding what they had: a stale listing a reader can still read
            // beats an empty one they cannot.
            None => self.status = "the index stopped answering; press R to try again".into(),
        }
        Ok(())
    }

    /// Put an answer on the screen.
    fn apply(&mut self, listing: search::Listing) -> Result<()> {
        self.facets = listing.facets;
        self.families = listing.families;
        self.faces = listing.faces;
        self.rows = build_rows(&self.facets, &self.selected);
        let len = self.list_len();
        let sel = self.list.selected().unwrap_or(0).min(len.saturating_sub(1));
        self.list.select((len > 0).then_some(sel));
        if self.facet_list.selected().is_none() && !self.rows.is_empty() {
            self.facet_list
                .select(Some(first_selectable(&self.rows, 0)));
        }
        self.prune_marks();
        // Anything the pane shows may have moved underneath it — a tag added, a face
        // activated, a rescan — so the cached detail is dropped and read again.
        self.detail = None;
        self.detail_summary = None;
        self.refresh_detail()?;
        Ok(())
    }

    /// Take whatever the worker has finished, if it is still wanted.
    ///
    /// Called once per turn of the event loop, between the frame and the key.
    fn collect(&mut self) -> Result<()> {
        let Some(search) = &mut self.search else {
            return Ok(());
        };
        let Some(listing) = search.take() else {
            return Ok(());
        };
        match listing {
            Ok(listing) => self.apply(listing)?,
            Err(e) => self.status = format!("search failed: {e}"),
        }
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
    /// Which of `ids` already carry `tag`.
    ///
    /// An undo has to put back what was there, not what a fresh action would leave
    /// behind: tagging a hundred faces of which forty already carried the tag has to
    /// undo to the same forty, or it takes something away that nobody added.
    fn already_tagged(&self, ids: &[i64], tag: &str) -> Result<BTreeSet<i64>> {
        let have: BTreeSet<i64> = self
            .index
            .list(&FaceFilter {
                tag: Some(tag.to_string()),
                ..Default::default()
            })?
            .into_iter()
            .map(|f| f.id)
            .collect();
        Ok(ids.iter().copied().filter(|id| have.contains(id)).collect())
    }

    /// The same question for a collection.
    fn already_collected(&self, ids: &[i64], name: &str) -> Result<BTreeSet<i64>> {
        let have: BTreeSet<i64> = self
            .index
            .list(&FaceFilter {
                collection: Some(name.to_string()),
                ..Default::default()
            })?
            .into_iter()
            .map(|f| f.id)
            .collect();
        Ok(ids.iter().copied().filter(|id| have.contains(id)).collect())
    }

    /// The activation each of `ids` has right now, which is what an undo restores.
    fn activation_before(&self, ids: &[i64]) -> Result<Vec<history::Prior>> {
        ids.iter()
            .map(|&id| Ok((id, self.index.activation(id)?.map(|r| r.state))))
            .collect()
    }

    /// The faces the next action touches: the marked set if there is one, otherwise
    /// the row under the cursor.
    ///
    /// One function, so every action inherits the selection at once and none of them
    /// can be the one that forgot. Everything that changes the index already came
    /// through here.
    fn current_face_ids(&self) -> Vec<i64> {
        if !self.marked.is_empty() {
            return self.marked.iter().copied().collect();
        }
        self.face_ids_at(self.list.selected())
    }

    /// The faces a row stands for: one for a face, the whole family for a family.
    fn face_ids_at(&self, row: Option<usize>) -> Vec<i64> {
        let Some(i) = row else {
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

    /// Every face the current filter matches, in the order the list shows them.
    fn visible_face_ids(&self) -> Vec<i64> {
        if self.open_family.is_some() {
            self.faces.iter().map(|f| f.id).collect()
        } else {
            self.families.iter().flat_map(|f| f.ids.clone()).collect()
        }
    }

    /// Whether the row at `i` is marked. A family counts as marked when every face it
    /// stands for is: a half-marked family is not one the reader chose.
    fn row_marked(&self, i: usize) -> bool {
        let ids = self.face_ids_at(Some(i));
        !ids.is_empty() && ids.iter().all(|id| self.marked.contains(id))
    }

    /// Mark or unmark the rows from the anchor to the cursor, or just the cursor.
    fn mark_rows(&mut self, from: usize, to: usize) {
        let (lo, hi) = if from <= to { (from, to) } else { (to, from) };
        // One decision for the whole range, taken from where it started: dragging over
        // a mixture should make them all the same rather than invert each one.
        let turning_on = !self.row_marked(from);
        for i in lo..=hi {
            for id in self.face_ids_at(Some(i)) {
                if turning_on {
                    self.marked.insert(id);
                } else {
                    self.marked.remove(&id);
                }
            }
        }
    }

    /// Drop from the mark anything the current filter no longer shows, and say so.
    ///
    /// A mark is a promise about which faces an action will touch, and a face that is
    /// no longer on the screen is one the reader can no longer see they have chosen.
    /// Keeping it would mean a later keystroke acting on something invisible.
    fn prune_marks(&mut self) {
        if self.marked.is_empty() {
            return;
        }
        let visible: BTreeSet<i64> = self.visible_face_ids().into_iter().collect();
        let before = self.marked.len();
        self.marked.retain(|id| visible.contains(id));
        let dropped = before - self.marked.len();
        if dropped > 0 {
            self.status = match self.marked.len() {
                0 => format!("the filter left none of the {before} marked faces"),
                left => format!("{dropped} marked face(s) no longer match; {left} left"),
            };
        }
    }

    /// Load everything the detail pane shows, once per selection. The draw path runs on
    /// every frame and only borrows what this leaves behind.
    fn refresh_detail(&mut self) -> Result<()> {
        let id = self.current_face_id();
        if id == self.detail_id && self.detail.is_some() {
            return Ok(());
        }
        let face = match id {
            Some(id) => self.index.get_face(id)?,
            None => None,
        };
        self.detail_summary = match (id, &face) {
            (Some(id), Some(_)) => self.index.summaries(&[id])?.into_iter().next(),
            _ => None,
        };
        // A row that has gone since the listing was built leaves no detail, so it must
        // leave no id either: the two always describe the same face.
        let next_id = id.filter(|_| face.is_some());
        // Rebuild the controls only when the face itself changes. `reload` clears
        // `detail` on every tag, activation, search and rescan, so rebuilding whenever
        // it is None would throw away axes and toggles the reader had set on a face
        // they never left.
        if next_id != self.detail_id || next_id.is_none() {
            self.controls = match &face {
                Some(f) => controls::Controls::for_face(f),
                None => controls::Controls::default(),
            };
        }
        self.detail_id = next_id;
        if self.focus == Focus::Detail && !self.detail_takes_focus() {
            self.focus = Focus::List;
        }
        self.detail = face;
        Ok(())
    }

    /// Whether the face pane is somewhere Tab can stop at this width.
    fn detail_takes_focus(&self) -> bool {
        self.shape.detail_takes_focus(!self.controls.is_empty())
    }

    /// Whether a key that moves an axis or toggles a feature should do so.
    ///
    /// The face pane takes focus on a narrow terminal whether or not the face has
    /// anything to adjust, so "the focus is in the face pane" and "the controls are
    /// listening" stopped being the same question.
    fn controls_active(&self) -> bool {
        self.focus == Focus::Detail && !self.controls.is_empty()
    }

    /// Tab: through the panes this width has, in the order they are drawn.
    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Facets => Focus::List,
            Focus::List if self.detail_takes_focus() => Focus::Detail,
            Focus::List | Focus::Detail => Focus::Facets,
        }
    }

    /// Which pane the layout should show, for a focus that may be in the controls.
    fn pane(&self) -> layout::Pane {
        match self.focus {
            Focus::Facets => layout::Pane::Facets,
            Focus::List => layout::Pane::List,
            Focus::Detail => layout::Pane::Detail,
        }
    }

    /// Resolve the palette against the terminal this run has.
    ///
    /// Asking the environment happens here, once, rather than inside `Theme::default`,
    /// so that building an App — which every test does — does not inherit whatever
    /// `TERM` and `NO_COLOR` the machine running the tests happens to have. Both the
    /// panes hold the answer, because two panes drawn at two depths would be one screen
    /// in two palettes.
    fn use_depth(&mut self, depth: theme::Depth) {
        self.theme = theme::Theme::new(depth);
    }

    // ----- events -----

    fn event_loop(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;
            self.collect()?;
            // Idle, the browser wakes four times a second, which is enough to notice
            // another process changing the index. While an answer is owed it wakes at
            // frame rate instead, so a result that arrives between keystrokes is on the
            // screen within a frame rather than within a quarter of a second.
            let wait = if self.search.as_ref().is_some_and(search::Search::waiting) {
                Duration::from_millis(16)
            } else {
                Duration::from_millis(250)
            };
            if !event::poll(wait)? {
                continue;
            }
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if self.on_key(key)? == Flow::Quit {
                return Ok(());
            }
        }
    }

    /// Apply one key press. Split out of the event loop so the browser can be driven
    /// without a terminal: every key a person can press reaches the same code a test
    /// does, which is what `tests::a_long_run_of_arbitrary_keys_keeps_every_invariant`
    /// relies on.
    fn on_key(&mut self, key: event::KeyEvent) -> Result<Flow> {
        if self.input.is_some() {
            self.handle_input_key(key)?;
            return Ok(Flow::Continue);
        }
        if self.help {
            // On a short terminal the help does not fit, so the keys that move
            // everything else move it too. Everything else closes it, which is what a
            // reader who pressed `?` by accident will try first.
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    // Saturating, not wrapping: the soak test holds a key down for
                    // hundreds of presses and a debug build panics on the overflow.
                    self.help_scroll = self.help_scroll.saturating_add(1)
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.help_scroll = self.help_scroll.saturating_sub(1)
                }
                _ => self.help = false,
            }
            return Ok(Flow::Continue);
        }
        if self.palette.is_some() && !ctrl_c(&key) {
            self.handle_palette_key(key.code)?;
            return Ok(Flow::Continue);
        }
        // Ctrl-C still quits from anywhere; a full-screen mode takes every other key,
        // so nothing underneath can move while it covers the panes.
        if !ctrl_c(&key) && self.glyphs.is_some() {
            self.handle_glyph_key(key.code)?;
            return Ok(Flow::Continue);
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('q') => return Ok(Flow::Quit),
            KeyCode::Char('c') if ctrl => return Ok(Flow::Quit),
            KeyCode::Esc => {
                // The mark first, because it is the thing a reader most recently did
                // and the thing an accidental action would act on.
                if !self.marked.is_empty() {
                    self.marked.clear();
                    self.mark_anchor = None;
                    self.status = "selection cleared".into();
                } else if self.open_family.is_some() {
                    self.close_family()?;
                } else if !self.query.is_empty() || !self.selected.is_empty() {
                    self.query.clear();
                    self.selected.clear();
                    self.reload()?;
                } else {
                    return Ok(Flow::Quit);
                }
            }
            KeyCode::Char('?') => {
                self.help = true;
                self.help_scroll = 0;
            }
            KeyCode::Char(':') => self.palette = Some(palette::Palette::new()),
            // Space marks, everywhere but the facet pane, where it is how a facet is
            // toggled. Enter still opens a family, so nothing that Space used to do is
            // lost — it did the same as Enter — and Space is what marks a row in every
            // other list a terminal has.
            KeyCode::Char(' ') if self.focus == Focus::List => {
                if let Some(i) = self.list.selected() {
                    self.mark_rows(i, i);
                    self.mark_anchor = None;
                    self.say_marked();
                }
            }
            // A range, the way vi does it: `v` here, move, `v` again.
            KeyCode::Char('v') if self.focus == Focus::List => {
                match (self.mark_anchor, self.list.selected()) {
                    (None, Some(i)) => {
                        self.mark_anchor = Some(i);
                        self.status = format!("range from row {}: move, then v", i + 1);
                    }
                    (Some(from), Some(to)) => {
                        self.mark_rows(from, to);
                        self.mark_anchor = None;
                        self.say_marked();
                    }
                    _ => {}
                }
            }
            // Everything the filter matches. `A` is taken by activating until logout,
            // and `*` is what a file manager uses for this.
            KeyCode::Char('*') => {
                let all: Vec<i64> = self.visible_face_ids();
                if self.marked.len() == all.len() && !all.is_empty() {
                    self.marked.clear();
                } else {
                    self.marked = all.into_iter().collect();
                }
                self.mark_anchor = None;
                self.say_marked();
            }
            KeyCode::Char('m') => self.open_glyphs(),
            KeyCode::Char('s') => self.open_specimen()?,
            KeyCode::Tab => self.cycle_focus(),
            KeyCode::Char('/') => self.start_input(InputKind::Search, self.query.clone()),
            KeyCode::Char('F') => self.open_filter_bar(),
            KeyCode::Char('t') => self.start_input(InputKind::Tag, String::new()),
            KeyCode::Char('c') => self.start_input(InputKind::Collection, String::new()),
            KeyCode::Char('x') => {
                self.selected.clear();
                self.query.clear();
                self.reload()?;
            }
            KeyCode::Char('a') => self.activate(ActivationState::User)?,
            KeyCode::Char('A') => self.activate(ActivationState::Session)?,
            KeyCode::Char('i') => self.activate(ActivationState::Installed)?,
            KeyCode::Char('d') => self.deactivate(false)?,
            KeyCode::Char('u') => self.deactivate(true)?,
            KeyCode::Char('R') => self.rescan()?,
            // `U` rather than `u`, which has meant uninstall since before there was
            // anything to undo, and Ctrl-R for redo the way an editor does it.
            KeyCode::Char('U') => self.undo()?,
            KeyCode::Char('r') if ctrl => self.redo()?,
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
            // In the controls the arrows move an axis, so they cannot also open a
            // family; Space and Enter still toggle the row under the cursor.
            KeyCode::Right | KeyCode::Char('l') if self.controls_active() => {
                self.controls.adjust(1);
                self.say_position();
            }
            KeyCode::Left | KeyCode::Char('h') if self.controls_active() => {
                self.controls.adjust(-1);
                self.say_position();
            }
            KeyCode::Char('L') if self.controls_active() => {
                self.controls.adjust(10);
                self.say_position();
            }
            KeyCode::Char('H') if self.controls_active() => {
                self.controls.adjust(-10);
                self.say_position();
            }
            KeyCode::Char('n') if self.controls_active() => {
                self.controls.cycle_instance(1);
                self.say_position();
            }
            KeyCode::Char('p') if self.controls_active() => {
                self.controls.cycle_instance(-1);
                self.say_position();
            }
            KeyCode::Char('0') if self.controls_active() => {
                self.controls.reset();
                self.say_position();
            }
            KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Right | KeyCode::Char('l') => {
                match self.focus {
                    Focus::Facets => self.toggle_facet()?,
                    Focus::List => self.open_family()?,
                    Focus::Detail => {
                        if self.controls_active() {
                            self.controls.toggle();
                            self.say_position();
                        }
                    }
                }
            }
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h')
                if self.focus == Focus::List =>
            {
                self.close_family()?
            }
            _ => {}
        }
        Ok(Flow::Continue)
    }

    /// Open the glyph map on the face on show. A face with no coverage at all — a
    /// broken font, or one still being scanned — has nothing to map.
    fn open_glyphs(&mut self) {
        let Some(face) = &self.detail else {
            self.status = "no face on show".into();
            return;
        };
        let map = glyphs::Glyphs::for_face(face);
        if map.is_empty() {
            self.status = "this face maps no codepoints".into();
            return;
        }
        self.status = format!(
            "{} codepoints in {} block(s)   (fontina glyphs {})",
            map.covered(),
            map.blocks().len(),
            self.detail_id.map(|id| id.to_string()).unwrap_or_default()
        );
        self.glyphs = Some(map);
    }

    /// Say where the axes and features now stand, as the command that would draw it.
    ///
    /// The controls used to move a picture in the pane below them. They move a position
    /// now, and a position with nothing to show for it is a knob attached to nothing —
    /// so it goes where every other action in this program goes: the status line, as the
    /// command line that would produce it. `fontina preview` draws a true image where
    /// the terminal has a protocol for one, and `s` opens the specimen with the sliders
    /// in it.
    fn say_position(&mut self) {
        let Some(id) = self.detail_id else { return };
        let axes = self
            .controls
            .variations()
            .iter()
            .map(|(tag, value)| format!("{tag}={}", (value * 100.0).round() / 100.0))
            .collect::<Vec<_>>()
            .join(",");
        let features = self
            .controls
            .forced_features()
            .iter()
            .map(|(tag, _)| tag.clone())
            .collect::<Vec<_>>()
            .join(",");
        let mut command = format!("fontina preview {id}");
        if !axes.is_empty() {
            command.push_str(&format!(" --axes {axes}"));
        }
        if !features.is_empty() {
            command.push_str(&format!(" --features {features}"));
        }
        self.status = command;
    }

    /// Write a specimen for the selection and hand it to the user's browser.
    ///
    /// A terminal is at its worst at exactly what choosing a typeface needs: real
    /// antialiasing at text sizes, spacing you can trust, hinting. `specimen.rs` already
    /// renders all of that, in a file that makes no network request and needs nothing
    /// installed. This is the one keystroke that reaches it — the whole of the graphical
    /// escape hatch, and the reason there is no second interface to maintain.
    fn open_specimen(&mut self) -> Result<()> {
        let ids = self.current_face_ids();
        if ids.is_empty() {
            self.status = "no face on show".into();
            return Ok(());
        }
        let mut faces = Vec::with_capacity(ids.len());
        for id in &ids {
            if let Some(face) = self.index.get_face(*id)? {
                faces.push(face);
            }
        }
        if faces.is_empty() {
            self.status = "no face on show".into();
            return Ok(());
        }
        let html = fontina_core::specimen::render(
            &faces,
            &fontina_core::specimen::SpecimenOptions {
                text: None,
                link: false,
                title: None,
            },
        )?;
        // One file per fontina, overwritten each time: a browser tab the user reloads is
        // better than a temp directory that fills up with every press of the key.
        let path =
            std::env::temp_dir().join(format!("fontina-specimen-{}.html", std::process::id()));
        std::fs::write(&path, &html)?;
        let ran = format!(
            "fontina specimen {} -o {}",
            ids.iter().map(i64::to_string).collect::<Vec<_>>().join(" "),
            path.display()
        );
        self.status = match fontina_platform::open::file(&path) {
            Ok(opened) => format!("opened in {}   ({ran})", opened.with),
            // A machine with no desktop is an ordinary place to run this. The file is
            // written either way, and its path is the useful half of the answer.
            Err(e) => format!("wrote {} but could not open it: {e}", path.display()),
        };
        Ok(())
    }

    /// Keys the glyph map owns while it is open. Returns whether it took the key.
    fn handle_glyph_key(&mut self, code: KeyCode) -> Result<bool> {
        // The grid is laid out at draw time; the columns used for scrolling are the
        // ones the last frame used, which is what the reader is looking at.
        let cols = self.glyph_cols.max(1);
        let Some(map) = self.glyphs.as_mut() else {
            return Ok(false);
        };
        match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('m') => self.glyphs = None,
            KeyCode::Down | KeyCode::Char('j') => map.scroll_by(1, cols),
            KeyCode::Up | KeyCode::Char('k') => map.scroll_by(-1, cols),
            KeyCode::PageDown => map.scroll_by(10, cols),
            KeyCode::PageUp => map.scroll_by(-10, cols),
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => map.select(1),
            KeyCode::Left | KeyCode::Char('h') => map.select(-1),
            KeyCode::Home | KeyCode::Char('g') => map.scroll_by(i32::MIN / 2, cols),
            KeyCode::End | KeyCode::Char('G') => map.scroll_by(i32::MAX / 2, cols),
            KeyCode::Char('/') => self.start_input(InputKind::Glyph, String::new()),
            // Everything else is swallowed. The map covers the panes, so a key that
            // activated a font or opened a family would leave it showing the coverage of
            // a face that is no longer on show, with no way to tell.
            _ => {}
        }
        Ok(true)
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

    fn handle_input_key(&mut self, key: event::KeyEvent) -> Result<()> {
        let code = key.code;
        // The filter bar is applied as it is typed, so every key that changes the line
        // has to re-apply it, and Esc has to put back what was there before. Checked
        // before the borrow below, because applying a line touches the whole browser.
        match self.input.as_ref().map(|i| i.kind) {
            None => return Ok(()),
            Some(InputKind::Filter) => return self.handle_filter_key(key),
            Some(_) => {}
        }
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
                    self.request_reload();
                }
            }
            KeyCode::Enter => {
                let Input { kind, buf } = self.input.take().expect("checked");
                let value = buf.trim().to_string();
                match kind {
                    InputKind::Filter => {
                        // Already applied, keystroke by keystroke. Enter just closes
                        // the prompt over the answer.
                        self.filter_before = None;
                        self.status = format!("{}   ({})", self.count_line(), self.command_line());
                    }
                    InputKind::Search => {
                        // Enter closes the box on a query already asked for by the last
                        // character typed. Waiting here would put the pause back at the
                        // one moment the reader has finished typing and is looking.
                        self.query = value;
                        self.request_reload();
                    }
                    InputKind::Glyph => {
                        let cols = self.glyph_cols.max(1);
                        if let Some(map) = self.glyphs.as_mut() {
                            self.status = if map.find(&value, cols) {
                                match map.found() {
                                    Some(cp) => {
                                        format!("U+{cp:04X} in {}", map.selected_map_name())
                                    }
                                    None => map.selected_map_name(),
                                }
                            } else {
                                format!("nothing covered matches {value:?}")
                            };
                        }
                    }
                    InputKind::Tag => {
                        if !value.is_empty() {
                            let ids = self.current_face_ids();
                            // Only the faces that gain the tag, so undo takes it off
                            // the ones that gained it and leaves the rest carrying
                            // what they came with.
                            let had = self.already_tagged(&ids, &value)?;
                            let moved: Vec<i64> =
                                ids.iter().copied().filter(|i| !had.contains(i)).collect();
                            let n = self.index.tag(&ids, &value)?;
                            self.history.record(history::Change::Tag {
                                ids: moved,
                                name: value.clone(),
                                added: true,
                            });
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
                            let had = self.already_collected(&ids, &value)?;
                            let moved: Vec<i64> =
                                ids.iter().copied().filter(|i| !had.contains(i)).collect();
                            let n = self.index.add_to_collection(&value, &ids)?;
                            self.history.record(history::Change::Collection {
                                ids: moved,
                                name: value.clone(),
                                added: true,
                            });
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
                    // Asked for, not waited on. This is the one key in the browser
                    // that a reader presses ten times in two seconds, and each press
                    // used to spend a query's worth of time before the next character
                    // could even be read.
                    self.request_reload();
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
                // Skip headers in the direction of travel. Row 0 is always a header, so
                // running off an end has to fall back to the nearest selectable row
                // rather than abandoning the move: PageUp from row 8 used to do nothing
                // at all, while Home on the same row settled on row 1.
                let mut ran_off = false;
                while self.rows[next].header {
                    let n = next as i32 + delta.signum();
                    if n < 0 || n >= self.rows.len() as i32 {
                        ran_off = true;
                        break;
                    }
                    next = n as usize;
                }
                if ran_off {
                    next = first_selectable(&self.rows, next);
                }
                self.facet_list.select(Some(next));
            }
            Focus::Detail => self.controls.move_cursor(delta),
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
            Focus::Detail => {
                let last = self.controls.len().saturating_sub(1);
                self.controls
                    .move_cursor(to.min(last) as i32 - self.controls.cursor() as i32);
            }
        }
        Ok(())
    }

    fn toggle_facet(&mut self) -> Result<()> {
        // A typed filter can say things a facet cannot — a range, two scripts at once,
        // "not variable" — so there is no sensible way to add one facet to one of
        // those. The facets take over again, and the line is not silently half-kept.
        if self.filter_override.take().is_some() {
            self.filter_line = None;
            self.status = "the filter bar was cleared: facets and a typed filter are \
                           two ways of saying the same thing"
                .into();
        }
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
        // Read before writing, so the history holds the state each face was in rather
        // than one state for the batch. A selection can hold faces in three.
        let prior = self.activation_before(&ids)?;
        self.history.record(history::Change::Activation { prior });
        self.activate_ids(&ids, state)
    }

    fn activate_ids(&mut self, ids: &[i64], state: ActivationState) -> Result<()> {
        let ids = ids.to_vec();
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
        let verb = match state {
            ActivationState::Installed => "install",
            ActivationState::Session => "activate --session",
            ActivationState::User => "activate",
        };
        let mut n = 0;
        // Failures are collected rather than returned. One unwritable file in a
        // selection of two hundred used to abandon the other hundred and ninety-nine
        // and report only the one, which left the reader with no way to know what had
        // happened and no way to ask again for just the rest.
        let mut failed: Vec<String> = Vec::new();
        for (path, faces) in crate::files_for(&self.index, &ids)? {
            let result = match state {
                ActivationState::Installed => self.activator.install(&path).map(|p| {
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
                    self.activator.activate(&path, scope).map(|_| {
                        self.index
                            .set_activation(&faces, state, None)
                            .map_err(|e| fontina_platform::PlatformError::Os(e.to_string()))
                    })
                }
            };
            match result.and_then(|r| r) {
                Ok(()) => n += faces.len(),
                Err(e) => failed.push(format!("{}: {e}", path.display())),
            }
        }
        self.status = report(verb, n, &failed);
        self.reload()
    }

    fn deactivate(&mut self, uninstall: bool) -> Result<()> {
        let ids = self.current_face_ids();
        if ids.is_empty() {
            self.status = "nothing selected".into();
            return Ok(());
        }
        let prior = self.activation_before(&ids)?;
        self.history.record(history::Change::Activation { prior });
        self.deactivate_ids(&ids, uninstall)
    }

    fn deactivate_ids(&mut self, ids: &[i64], uninstall: bool) -> Result<()> {
        let ids = ids.to_vec();
        let mut n = 0;
        let mut failed: Vec<String> = Vec::new();
        for (path, faces) in crate::files_for(&self.index, &ids)? {
            let record = self.index.activation(faces[0])?;
            let result = if uninstall {
                match record.and_then(|r| r.installed_path) {
                    Some(p) => self.activator.uninstall(Path::new(&p)).map(|()| true),
                    None => continue,
                }
            } else {
                if record.is_none() {
                    continue;
                }
                self.activator.deactivate(&path)
            };
            match result {
                Ok(_) => {
                    self.index.clear_activation(&faces)?;
                    n += faces.len();
                }
                Err(e) => failed.push(format!("{}: {e}", path.display())),
            }
        }
        let verb = if uninstall { "uninstall" } else { "deactivate" };
        self.status = report(verb, n, &failed);
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
        self.shape = layout::Shape::for_width(area.width);
        // The focus can outlive the pane it was in: drag a window narrow while the
        // cursor sits in the controls and the face pane stops taking focus. Settle
        // that before the split, so the frame draws the pane the focus ends up in.
        if self.focus == Focus::Detail && !self.detail_takes_focus() {
            self.focus = Focus::List;
        }
        let panes = layout::split(vertical[0], self.pane());
        // Beneath first, overlay last: at two panes the facets are drawn over the list.
        if let Some(area) = panes.list {
            self.draw_list(f, area);
        }
        if let Some(area) = panes.detail {
            self.draw_detail(f, area);
        }
        if let Some(area) = panes.facets {
            if panes.overlay {
                f.render_widget(Clear, area);
            }
            self.draw_facets(f, area, panes.overlay);
        }
        self.draw_status(f, vertical[1]);
        self.draw_keys(f, vertical[2]);
        if self.glyphs.is_some() {
            self.draw_glyphs(f, vertical[0]);
        }
        if self.palette.is_some() {
            self.draw_palette(f, area);
        }
        if self.help {
            self.draw_help(f, area);
        }
    }

    /// The command palette: what has been typed, and what still matches.
    ///
    /// Over the middle of the screen rather than beside anything, because it is about
    /// the whole program rather than about the pane underneath it.
    fn draw_palette(&mut self, f: &mut ratatui::Frame, area: Rect) {
        let Some(palette) = self.palette.as_ref() else {
            return;
        };
        let w = 76.min(area.width);
        let h = 20.min(area.height);
        if w < 24 || h < 5 {
            return;
        }
        let rect = Rect::new(
            area.x + (area.width - w) / 2,
            area.y + (area.height - h) / 2,
            w,
            h,
        );
        let matches = palette.matches();
        let title = match &palette.confirming {
            Some(cmd) => format!(" {cmd} writes to the disk — y to go ahead "),
            None => format!(
                " : {} — {} of {} ",
                palette.query,
                matches.len(),
                palette.total()
            ),
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.accent())
            .title(title);
        let inner = block.inner(rect);
        f.render_widget(Clear, rect);
        f.render_widget(block, rect);

        // The command it would run, in full, above the list: the palette teaches the
        // command line, and a name on its own teaches nothing.
        let mut rows = inner;
        if let Some(chosen) = palette.selected() {
            let head = Rect { height: 1, ..inner };
            rows = Rect {
                y: inner.y + 1,
                height: inner.height.saturating_sub(1),
                ..inner
            };
            f.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    truncate(&self.command_for(&chosen.path), inner.width as usize),
                    self.theme.accent(),
                )])),
                head,
            );
        }

        let visible = rows.height as usize;
        let win = layout::window(matches.len(), palette.cursor(), visible, 0);
        let items: Vec<ListItem> = matches[win.clone()]
            .iter()
            .map(|e| {
                let name = format!("{:<22}", truncate(&e.path, 22));
                let room = (rows.width as usize).saturating_sub(22 + e.hint().len() + 2);
                ListItem::new(Line::from(vec![
                    Span::styled(name, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!("{:<room$}", truncate(&e.about, room), room = room)),
                    Span::styled(format!("  {}", e.hint()), self.theme.dim()),
                ]))
            })
            .collect();
        let list =
            List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, rows, &mut windowed(Some(palette.cursor()), &win));
    }

    fn draw_glyphs(&mut self, f: &mut ratatui::Frame, area: Rect) {
        let Some(map) = &self.glyphs else { return };
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.accent())
            .title(" glyph map — / to search, m or Esc to close ");
        let inner = block.inner(area);
        f.render_widget(block, area);

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(34), Constraint::Min(20)])
            .split(inner);

        // Blocks, with the selected one marked and its coverage as a fraction.
        let selected = map.selected_index();
        let visible = columns[0].height as usize;
        let offset = selected.saturating_sub(visible.saturating_sub(1));
        let rows: Vec<Line> = map
            .blocks()
            .iter()
            .enumerate()
            .skip(offset)
            .take(visible)
            .map(|(i, b)| {
                let style = if i == selected {
                    self.theme.accent().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                Line::from(Span::styled(
                    format!(
                        "{} {:<22} {:>4}/{}",
                        if i == selected { ">" } else { " " },
                        truncate(&b.block, 22),
                        b.codepoints.len(),
                        b.block_size
                    ),
                    style,
                ))
            })
            .collect();
        f.render_widget(Paragraph::new(rows), columns[0]);

        let grid = columns[1];
        // Every cell is two columns wide whatever it holds, so the grid lines up whether
        // the block is Latin, CJK or combining marks. `LABEL` covers the widest possible
        // codepoint label, `10FFFF`, plus its separating space.
        const LABEL: usize = 7;
        const CELL: usize = 2;
        let cols = ((grid.width as usize).saturating_sub(LABEL) / CELL).max(1);
        self.glyph_cols = cols;
        // The pane may have been resized since the last scroll, so re-clamp against the
        // width being drawn now; otherwise a scrolled block can render blank.
        let Some(map) = self.glyphs.as_mut() else {
            return;
        };
        map.clamp_scroll(cols);
        let (start, found) = (map.scroll_row() * cols, map.found());
        let Some(current) = map.selected() else {
            return;
        };
        let mut lines: Vec<Line> = Vec::new();
        for row in current.codepoints[start.min(current.codepoints.len())..]
            .chunks(cols)
            .take(grid.height as usize)
        {
            let mut spans = vec![Span::styled(
                format!("{:<LABEL$}", format!("{:04X}", row[0])),
                self.theme.dim(),
            )];
            for &cp in row {
                let cell = fontina_core::unicode::cell_for(cp);
                let style = if Some(cp) == found {
                    self.theme.cursor()
                } else {
                    Style::default()
                };
                // Pad each cell out to CELL columns, so a double-width glyph takes the
                // space of one cell rather than pushing the rest of the row along.
                spans.push(Span::styled(
                    format!("{}{}", cell.glyph, " ".repeat(CELL - cell.width)),
                    style,
                ));
            }
            lines.push(Line::from(spans));
        }
        f.render_widget(Paragraph::new(lines), grid);
    }

    /// Take back the last thing that changed the index.
    ///
    /// The status line names what it did rather than saying "undone", because a reader
    /// pressing `U` twice needs to know which of two changes came back.
    fn undo(&mut self) -> Result<()> {
        let Some(change) = self.history.undo() else {
            self.status = "nothing to undo".into();
            return Ok(());
        };
        let inverse = self.invert(&change)?;
        self.status = format!("undone: {}", change.describe());
        self.history.undone(inverse);
        self.reload()
    }

    /// Do again what was just undone.
    fn redo(&mut self) -> Result<()> {
        let Some(change) = self.history.redo() else {
            self.status = "nothing to redo".into();
            return Ok(());
        };
        let inverse = self.invert(&change)?;
        self.status = format!("redone: {}", change.describe());
        self.history.redone(inverse);
        self.reload()
    }

    /// Apply a change's inverse, and return the inverse of *that*, so undo and redo
    /// are the same walk in opposite directions and neither needs its own code.
    fn invert(&mut self, change: &history::Change) -> Result<history::Change> {
        match change {
            history::Change::Tag { ids, name, added } => {
                if *added {
                    self.index.untag(ids, name)?;
                } else {
                    self.index.tag(ids, name)?;
                }
                Ok(history::Change::Tag {
                    ids: ids.clone(),
                    name: name.clone(),
                    added: !added,
                })
            }
            history::Change::Collection { ids, name, added } => {
                if *added {
                    self.index.remove_from_collection(name, ids)?;
                } else {
                    self.index.add_to_collection(name, ids)?;
                }
                Ok(history::Change::Collection {
                    ids: ids.clone(),
                    name: name.clone(),
                    added: !added,
                })
            }
            history::Change::Activation { prior } => {
                let now =
                    self.activation_before(&prior.iter().map(|(id, _)| *id).collect::<Vec<_>>())?;
                // Grouped by the state each face is going back to, so a selection that
                // held three states goes back to three states rather than to one.
                let mut off: Vec<i64> = Vec::new();
                // A list rather than a map: there are three activation states, so
                // finding one is a glance, and `ActivationState` is a value in the
                // schema rather than a key type that has to be ordered for this.
                let mut on: Vec<(ActivationState, Vec<i64>)> = Vec::new();
                for (id, state) in prior {
                    match state {
                        None => off.push(*id),
                        Some(state) => match on.iter_mut().find(|(s, _)| s == state) {
                            Some((_, ids)) => ids.push(*id),
                            None => on.push((*state, vec![*id])),
                        },
                    }
                }
                if !off.is_empty() {
                    // `false`: putting a face back to "not activated" undoes an
                    // activation, and deleting a file the reader did not ask to delete
                    // is not an undo of anything.
                    self.deactivate_ids(&off, false)?;
                }
                for (state, ids) in on {
                    self.activate_ids(&ids, state)?;
                }
                Ok(history::Change::Activation { prior: now })
            }
        }
    }

    /// Say how many faces the next action would touch, in the words the browser uses
    /// everywhere else: face counts, because a family is not a unit anything acts on.
    /// Open the filter bar over whatever the panes are showing.
    ///
    /// Pre-filled with the flags for the current screen — the same flags the status
    /// line has been showing all along — so the bar starts as an editable copy of what
    /// the reader can already see rather than as an empty box they have to guess at.
    fn open_filter_bar(&mut self) {
        self.filter_before = Some((self.filter_line.clone(), self.filter_override.clone()));
        let line = self.filter_flags();
        self.filter_error = None;
        self.start_input(InputKind::Filter, line);
    }

    /// The current filter as the flags `fontina list` would take.
    ///
    /// Taken off the command line the status bar already builds, rather than rendered
    /// a second way: two renderers of one filter drift, and the one that drifts is the
    /// one nobody is looking at.
    fn filter_flags(&self) -> String {
        let line = self.command_line();
        let flags = line
            .strip_prefix("fontina families")
            .or_else(|| line.strip_prefix("fontina list"))
            .unwrap_or("")
            .trim();
        // The open family is the pane the reader is standing in, not part of what they
        // typed, and `filter` puts it back on its own.
        match flags.find("--family ") {
            Some(i) => flags[..i].trim_end().to_string(),
            None => flags.to_string(),
        }
    }

    /// Keys while the filter bar is up.
    ///
    /// Every key that changes the line applies it again, so the panes behind the prompt
    /// are always showing what the line says and the count in the prompt is a fact
    /// rather than a promise.
    fn handle_filter_key(&mut self, key: event::KeyEvent) -> Result<()> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => {
                // Back to whatever was there when the bar opened, panes and all.
                self.input = None;
                self.filter_error = None;
                if let Some((line, parsed)) = self.filter_before.take() {
                    self.filter_line = line;
                    self.filter_override = parsed;
                    return self.reload();
                }
            }
            KeyCode::Enter => {
                self.filter_before = None;
                self.input = None;
                self.filter_error = None;
                self.status = format!("{}   ({})", self.count_line(), self.command_line());
            }
            // One keystroke from a composed filter to a collection of what it matched:
            // everything matching is marked and the collection prompt opens over it, so
            // the next thing typed is the name.
            KeyCode::Char('s') if ctrl => {
                self.input = None;
                self.filter_before = None;
                self.filter_error = None;
                self.marked = self.visible_face_ids().into_iter().collect();
                if self.marked.is_empty() {
                    self.status = "nothing to save: the filter matches no faces".into();
                    return Ok(());
                }
                self.start_input(InputKind::Collection, String::new());
            }
            KeyCode::Backspace => {
                if let Some(i) = self.input.as_mut() {
                    i.buf.pop();
                }
                return self.apply_filter_line();
            }
            KeyCode::Char(c) => {
                if let Some(i) = self.input.as_mut() {
                    i.buf.push(c);
                }
                return self.apply_filter_line();
            }
            _ => {}
        }
        Ok(())
    }

    /// Parse what has been typed and show it, or say what is wrong with it.
    ///
    /// A line that does not parse leaves the panes on the last one that did. Half a
    /// flag is not a filter, and blanking the list on every intermediate keystroke
    /// would make the count useless exactly while it is being composed.
    fn apply_filter_line(&mut self) -> Result<()> {
        let line = self
            .input
            .as_ref()
            .map(|i| i.buf.clone())
            .unwrap_or_default();
        match filter::parse(&line) {
            Ok(parsed) => {
                self.filter_error = None;
                self.filter_line = Some(line);
                self.filter_override = Some(parsed);
                self.reload()
            }
            Err(e) => {
                self.filter_error = Some(e);
                Ok(())
            }
        }
    }

    fn say_marked(&mut self) {
        self.status = match self.marked.len() {
            0 => "selection cleared".into(),
            n => format!("{n} face(s) selected — Esc clears, any action applies to all"),
        };
    }

    /// Keys while the palette is up.
    ///
    /// It takes all of them, because it is a prompt: a reader typing `install` is
    /// typing, not pressing `i` for install and then `n` for nothing.
    fn handle_palette_key(&mut self, code: KeyCode) -> Result<()> {
        // A command that writes to the disk waits for a yes first, and anything that is
        // not a yes is a no. There is no third answer worth having.
        if let Some(pending) = self.palette.as_ref().and_then(|p| p.confirming.clone()) {
            let go = matches!(code, KeyCode::Char('y') | KeyCode::Char('Y'));
            self.palette = None;
            if go {
                return self.run_command(&pending);
            }
            self.status = format!("{pending}: not run");
            return Ok(());
        }
        match code {
            KeyCode::Esc => self.palette = None,
            KeyCode::Down => {
                if let Some(p) = self.palette.as_mut() {
                    p.move_cursor(1)
                }
            }
            KeyCode::Up => {
                if let Some(p) = self.palette.as_mut() {
                    p.move_cursor(-1)
                }
            }
            KeyCode::Backspace => {
                if let Some(p) = self.palette.as_mut() {
                    p.backspace()
                }
            }
            KeyCode::Enter => {
                let chosen = self.palette.as_ref().and_then(|p| p.selected()).cloned();
                let Some(chosen) = chosen else {
                    self.palette = None;
                    return Ok(());
                };
                if matches!(chosen.reach, palette::Reach::Ask(_)) {
                    if let Some(p) = self.palette.as_mut() {
                        p.confirming = Some(chosen.path.clone());
                    }
                } else {
                    self.palette = None;
                    return self.run_command(&chosen.path);
                }
            }
            KeyCode::Char(c) => {
                if let Some(p) = self.palette.as_mut() {
                    p.type_char(c)
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Do what the palette chose.
    ///
    /// The browser presses its own key for the commands it implements, so there is one
    /// implementation of activating a font rather than two. For the rest it writes the
    /// command line out, with the filter and the selection already in it, because a
    /// command that prints wants a screen the browser is currently using.
    fn run_command(&mut self, path: &str) -> Result<()> {
        let Some(entry) = palette::entries().into_iter().find(|e| e.path == path) else {
            return Ok(());
        };
        match palette::key_for(entry.reach) {
            Some(key) => {
                self.on_key(event::KeyEvent::new(key, KeyModifiers::NONE))?;
                Ok(())
            }
            None => {
                self.status = format!(
                    "{}   (prints, so run it in another window)",
                    self.command_for(path)
                );
                Ok(())
            }
        }
    }

    /// The command line for `path`, carrying what is on the screen.
    ///
    /// The status line already shows the command for the listing; this shows it for any
    /// command in the palette, with the ids of whatever the reader has chosen, so it
    /// can be pasted into another window and give exactly what they are looking at.
    fn command_for(&self, path: &str) -> String {
        let ids = self.current_face_ids();
        match path {
            // The listing commands describe the whole screen, which the status line
            // already knows how to say.
            "list" | "families" | "facets" => self.command_line().replacen(
                if self.open_family.is_some() {
                    "fontina list"
                } else {
                    "fontina families"
                },
                &format!("fontina {path}"),
                1,
            ),
            _ if ids.is_empty() => format!("fontina {path}"),
            _ => format!(
                "fontina {path} {}",
                ids.iter().map(i64::to_string).collect::<Vec<_>>().join(" ")
            ),
        }
    }

    fn border(&self, focused: bool) -> Style {
        if focused {
            self.theme.accent()
        } else {
            self.theme.dim()
        }
    }

    fn draw_facets(&mut self, f: &mut ratatui::Frame, area: Rect, overlay: bool) {
        let win = layout::window(
            self.rows.len(),
            self.facet_list.selected().unwrap_or(0),
            pane_rows(area),
            self.facet_offset,
        );
        self.facet_offset = win.start;
        let items: Vec<ListItem> = self.rows[win.clone()]
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
                        self.theme.accent()
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
                    // A pane drawn over another has to say how to put it back, because
                    // nothing else on the screen explains where the list went.
                    .title(if overlay {
                        format!(" {} faces — ⇥ closes ", self.facets.faces)
                    } else {
                        format!(" {} faces ", self.facets.faces)
                    }),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, area, &mut windowed(self.facet_list.selected(), &win));
    }

    fn draw_list(&mut self, f: &mut ratatui::Frame, area: Rect) {
        let width = area.width.saturating_sub(2) as usize;
        let win = layout::window(
            self.list_len(),
            self.list.selected().unwrap_or(0),
            pane_rows(area),
            self.list_offset,
        );
        self.list_offset = win.start;
        // A mark column, but only while there is something marked or a range being
        // drawn: a column of spaces down every list for a feature nobody is using is
        // two columns of names given up for nothing.
        let marking = !self.marked.is_empty() || self.mark_anchor.is_some();
        let marks: Vec<String> = if marking {
            win.clone()
                .map(|i| {
                    let in_range = self.mark_anchor.is_some_and(|a| {
                        let cursor = self.list.selected().unwrap_or(a);
                        (a.min(cursor)..=a.max(cursor)).contains(&i)
                    });
                    match (self.row_marked(i), in_range) {
                        (true, _) => "● ".into(),
                        (false, true) => "· ".into(),
                        (false, false) => "  ".into(),
                    }
                })
                .collect()
        } else {
            vec![String::new(); win.len()]
        };
        let width = width.saturating_sub(if marking { 2 } else { 0 });
        let items: Vec<ListItem> = if let Some(fam) = &self.open_family {
            let _ = fam;
            self.faces[win.clone()]
                .iter()
                .enumerate()
                .map(|(row, face)| {
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
                        "{}{:<w$} {flags}",
                        marks[row],
                        truncate(&left, width.saturating_sub(5)),
                        w = width.saturating_sub(5)
                    )))
                })
                .collect()
        } else {
            self.families[win.clone()]
                .iter()
                .enumerate()
                .map(|(row, fam)| {
                    let flags = format!(
                        "{}{}{}",
                        if fam.variable { "V" } else { " " },
                        if fam.color { "C" } else { " " },
                        if fam.active > 0 { "●" } else { " " },
                    );
                    let count = format!("{:>3}", fam.faces);
                    let room = width.saturating_sub(9);
                    ListItem::new(Line::from(format!(
                        "{}{:<room$} {count} {flags}",
                        marks[row],
                        truncate(&fam.name, room),
                        room = room
                    )))
                })
                .collect()
        };
        let title = match (&self.open_family, self.marked.len()) {
            // The count goes in the title rather than only in the status line, which
            // the next message overwrites: what an action is about to touch has to be
            // readable at the moment the reader reaches for the key.
            (_, n) if n > 0 => format!(" {n} of {} selected ", self.visible_face_ids().len()),
            (Some(fam), _) => format!(" {} · {} face(s) ", fam, self.faces.len()),
            (None, _) => format!(" {} families ", self.families.len()),
        };
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.border(self.focus == Focus::List))
                    .title(title),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, area, &mut windowed(self.list.selected(), &win));
    }

    fn draw_detail(&mut self, f: &mut ratatui::Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.border(self.focus == Focus::Detail))
            .title(" Details ");
        let inner = block.inner(area);
        f.render_widget(block, area);
        // Borrowed, never cloned: this runs on every frame, four times a second while
        // the browser sits idle. `refresh_detail` is what queries the index.
        let Some(face) = self.detail.as_ref() else {
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
        lines.push(kv(
            "style",
            format!(
                "weight {} · width {}% · {}",
                face.style.weight.round(),
                face.style.width.round(),
                face.style.css.style
            ),
            &self.theme,
        ));
        if let Some(v) = &face.variable {
            lines.push(kv(
                "axes",
                v.axes
                    .iter()
                    .map(|a| format!("{} {}–{} ({})", a.tag, a.min, a.max, a.default))
                    .collect::<Vec<_>>()
                    .join(", "),
                &self.theme,
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
            &self.theme,
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
                &self.theme,
            ));
        }
        lines.extend(license_lines(face, &self.theme));
        if let Some(d) = face
            .names
            .designer
            .as_deref()
            .or(face.names.manufacturer.as_deref())
        {
            lines.push(kv("designer", d.to_string(), &self.theme));
        }
        if let Some(s) = self.detail_summary.as_ref() {
            if !s.tags.is_empty() {
                lines.push(kv("tags", s.tags.join(", "), &self.theme));
            }
            lines.push(kv(
                "state",
                match s.activation {
                    Some(a) => a.as_str().to_string(),
                    None => "not active".into(),
                },
                &self.theme,
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
            &self.theme,
        ));
        lines.push(Line::from(""));
        // Rows the block will actually occupy once wrapped, not how many lines were
        // pushed. The paragraph wraps, so a long value — a file path, a licence reason —
        // takes several rows, and counting them as one pushed the bottom of the pane off
        // the screen.
        let text_rows: u16 = lines.iter().map(|l| wrapped_rows(l, inner.width)).sum();
        // Controls take the rows they need, capped so the preview never disappears.
        // The pane asks for a title plus a row per control, but never takes so much that
        // the preview vanishes, and never less than a title plus one row: a pane Tab can
        // reach has to show the cursor sitting in it.
        let control_rows = if self.controls.is_empty() {
            0
        } else {
            let spare = inner.height.saturating_sub(text_rows + 4);
            // Either a title and at least one control, or nothing: a pane showing only
            // its own title would hide the cursor sitting in it.
            if spare < 2 {
                0
            } else {
                (self.controls.len() as u16 + 1).min(spare)
            }
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(text_rows.min(inner.height)),
                Constraint::Length(control_rows),
                Constraint::Min(0),
            ])
            .split(inner);
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), chunks[0]);
        if control_rows > 0 {
            self.draw_controls(f, chunks[1], face);
        }
        let rest = chunks[2];
        if rest.height < 2 || rest.width < 4 {
            return;
        }
        f.render_widget(
            Paragraph::new(self.measurements(face)).wrap(Wrap { trim: false }),
            rest,
        );
    }

    /// What the pane says below the controls: the numbers, and the coverage they came
    /// from.
    ///
    /// This was a rasterised sample of the face, drawn in half-block characters. A
    /// terminal cell is about one pixel wide and two tall, so a typeface met that way is
    /// met through a filter that changes its weight, loses its spacing and destroys its
    /// detail — a judgement made on something that is not the font. `s` writes a real
    /// specimen and opens it in a browser, and `fontina preview` draws a true image
    /// where the terminal has a protocol for one. What a terminal is good at is this:
    /// the measurements, in a column, comparable at a glance between two faces.
    fn measurements(&self, face: &FaceMetadata) -> Vec<Line<'static>> {
        let m = &face.metrics;
        let mut lines = vec![Line::from("")];
        lines.push(kv(
            "metrics",
            format!(
                "{} upm · asc {} · desc {} · gap {}",
                m.units_per_em, m.ascender, m.descender, m.line_gap
            ),
            &self.theme,
        ));
        let mut shape = Vec::new();
        if let Some(cap) = m.cap_height {
            shape.push(format!("cap {cap}"));
        }
        if let Some(x) = m.x_height {
            shape.push(format!("x-height {x}"));
            // The ratio is the number type people actually compare, and neither font
            // carries it: a face with a large x-height at the same size reads larger.
            if m.units_per_em > 0 {
                shape.push(format!(
                    "x/em {:.2}",
                    f64::from(x) / f64::from(m.units_per_em)
                ));
            }
        }
        if m.italic_angle.abs() > f32::EPSILON {
            shape.push(format!("italic {}°", m.italic_angle));
        }
        if !shape.is_empty() {
            lines.push(kv("shape", shape.join(" · "), &self.theme));
        }
        // Coverage by script, deepest first, with a bar for the eye and the count for
        // the answer. A face's scripts are already sorted by how much of each it has.
        let deepest = face
            .coverage
            .scripts
            .first()
            .map(|s| s.codepoints)
            .unwrap_or(0)
            .max(1);
        for script in face.coverage.scripts.iter().take(6) {
            let filled = (script.codepoints * 10).div_ceil(deepest).min(10) as usize;
            lines.push(kv(
                &script.script,
                format!(
                    "{:<10} {}",
                    "█".repeat(filled) + &"░".repeat(10 - filled),
                    script.codepoints
                ),
                &self.theme,
            ));
        }
        lines
    }

    /// Axes as `tag  value` with a bar, features as a checkbox, the selected row
    /// highlighted when the pane has focus.
    fn draw_controls(&self, f: &mut ratatui::Frame, area: Rect, face: &FaceMetadata) {
        let focused = self.focus == Focus::Detail;
        let mut lines: Vec<Line> = Vec::new();
        let title = match self.controls.instance_name(face) {
            Some(name) => format!("axes & features — {name}"),
            None if self.controls.is_variable() => "axes & features — custom".to_string(),
            // "custom" would be nonsense for a face with no axes to be custom about.
            None => "features".to_string(),
        };
        lines.push(Line::from(Span::styled(title, self.theme.dim())));
        // Scroll so the cursor is always on screen; without this a reader moves down,
        // the marker disappears, and the arrows adjust an axis they cannot see.
        let body = area.height.saturating_sub(1) as usize;
        let offset = self
            .controls
            .cursor()
            .saturating_sub(body.saturating_sub(1));
        for (i, row) in self.controls.rows().enumerate().skip(offset).take(body) {
            let selected = focused && i == self.controls.cursor();
            let marker = if selected { ">" } else { " " };
            let style = if selected {
                self.theme.accent()
            } else {
                Style::default()
            };
            let text = match row {
                controls::Row::Axis(a) => {
                    let span = (a.max - a.min).max(f32::EPSILON);
                    let filled = (((a.value - a.min) / span) * 12.0).round() as usize;
                    format!(
                        "{marker} {:<4} {:>8}  [{}{}] {}",
                        a.tag,
                        fmt_axis(a.value),
                        "=".repeat(filled.min(12)),
                        " ".repeat(12 - filled.min(12)),
                        // The designer's own name for the axis, when it differs from
                        // the tag; `wght` labelled "wght" is noise.
                        if a.label == a.tag { "" } else { &a.label },
                    )
                }
                controls::Row::Feature(feature) => format!(
                    "{marker} {:<4} [{}] {}",
                    feature.tag,
                    if feature.on { "x" } else { " " },
                    feature.label
                ),
            };
            lines.push(Line::from(Span::styled(text, style)));
        }
        f.render_widget(Paragraph::new(lines), area);
    }

    /// The filter prompt: the line, and what it is doing.
    ///
    /// The count comes from the panes behind it, which are already showing this filter
    /// — the line is applied as it is typed. Watching the count while composing is the
    /// whole point of the bar, so it cannot be something the reader has to ask for.
    fn filter_prompt(&self, f: &mut ratatui::Frame, area: Rect, line: &str) {
        let (tail, style) = match &self.filter_error {
            Some(e) => (format!("  ← {e}"), self.theme.bad()),
            None => (format!("  {}", self.count_line()), self.theme.dim()),
        };
        let head = " filter: ";
        let room = (area.width as usize).saturating_sub(head.len() + tail.chars().count() + 1);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(head, self.theme.accent()),
                Span::raw(truncate(line, room)),
                Span::styled("▏", self.theme.accent()),
                Span::styled(tail, style),
            ])),
            area,
        );
    }

    /// How many faces the filter leaves, in the browser's own words.
    fn count_line(&self) -> String {
        match self.facets.faces {
            1 => "1 face".into(),
            n => format!("{n} faces"),
        }
    }

    fn draw_status(&self, f: &mut ratatui::Frame, area: Rect) {
        let line = if let Some(input) = &self.input {
            let prompt = match input.kind {
                InputKind::Search => "search",
                InputKind::Tag => "tag",
                InputKind::Collection => "collection",
                InputKind::Glyph => "codepoint or block",
                // What the line is doing right now, so the reader is watching the
                // count rather than guessing at it: the panes behind the prompt are
                // already showing this filter.
                InputKind::Filter => return self.filter_prompt(f, area, &input.buf),
            };
            Line::from(vec![
                Span::styled(format!(" {prompt}: "), self.theme.accent()),
                Span::raw(input.buf.clone()),
                Span::styled("▏", self.theme.accent()),
            ])
        } else if !self.status.is_empty() {
            Line::from(Span::raw(format!(" {}", self.status)))
        } else {
            Line::from(Span::styled(
                format!(" $ {}", self.command_line()),
                self.theme.dim(),
            ))
        };
        f.render_widget(Paragraph::new(line), area);
    }

    fn draw_keys(&self, f: &mut ratatui::Frame, area: Rect) {
        f.render_widget(
            Paragraph::new(Span::styled(layout::keys(area.width), self.theme.dim())),
            area,
        );
    }

    fn draw_help(&self, f: &mut ratatui::Frame, area: Rect) {
        let text = "
 Move        j/k ↑/↓ PgUp/PgDn g/G        Tab cycles the panes
 Search      / types a query, Esc clears it, x clears every filter
 Filter bar  F takes the flags fontina list takes — ranges, two scripts at once,
             --variable=false — applied as you type, with the count beside them.
             Ctrl-S saves what it matched as a collection
 Facets      Enter or Space toggles the value under the cursor
 Families    Enter opens a family, Backspace or Esc closes it
 Select      Space marks a row, v starts and ends a range, * takes everything the
             filter matches. Actions below apply to the marks when there are any,
             to the cursor when there are not; Esc clears them
 Organise    t tags the selection, c adds it to a collection
 Activate    a for the user, A until logout, i installs a copy, d and u undo those
 Controls    h/l ←/→ move an axis (H/L by ten), Space toggles a feature, n/p step
             through named instances, 0 resets everything. The status line carries
             the command that draws what you have set
 Glyphs      m opens the glyph map: h/l pick a block, / finds a codepoint
             (U+0041, 0x41, 41) or a block by name
 Specimen    s writes an HTML specimen and opens it. A terminal cannot show a
             typeface honestly, so this program does not try: it shows what is in
             one, and hands the looking to something that can draw
 Panes       Three side by side at {three} columns and up; under that the facets
             move over the list; under {two}, one pane at a time. Tab reaches all
 Undo        U takes back the last change to the index, Ctrl-R does it again. A
             whole selection is one undo; what cannot be put back exactly is not
 Commands    : lists every command the program has, filtered as you type. What the
             browser implements it runs; what prints, it writes out for you
 Index       R rescans every source (fontina scan --prune)          Quit  q

 The status line shows the CLI command for what you see. Everything here is a command.

 any key to close";
        // The widths the panes change at are stated rather than described, so the
        // help cannot drift from the layout it is describing.
        let text = text
            .replace("{three}", &layout::THREE.to_string())
            .replace("{two}", &layout::TWO.to_string());
        let text = text.as_str();
        let w = 90.min(area.width);
        // Lines the text will take once the box has wrapped it, which on a narrow
        // terminal is more than the lines it was written as.
        let body = w.saturating_sub(2);
        let total: u16 = text
            .lines()
            .map(|l| wrapped_rows(&Line::from(l), body))
            .sum();
        let h = (total + 2).min(area.height);
        let rect = Rect::new(
            area.x + (area.width - w) / 2,
            area.y + (area.height - h) / 2,
            w,
            h,
        );
        // A help box that does not fit says so and says what to do about it, rather
        // than quietly ending three keys early on the one screen where a reader is
        // looking for a key they cannot find.
        let shown = h.saturating_sub(2);
        let scroll = self.help_scroll.min(total.saturating_sub(shown));
        let title = if total > shown {
            format!(" fontina ui — {}/{} lines, j/k scrolls ", shown, total)
        } else {
            " fontina ui ".into()
        };
        f.render_widget(Clear, rect);
        f.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(self.theme.accent())
                        .title(title),
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
    section(Facet::Spacing, &facets.spacing, 2);
    section(Facet::Script, &facets.script, 8);
    section(Facet::Language, &facets.language, 8);
    section(Facet::License, &facets.license, 6);
    // Four states at most, so nothing is ever hidden behind a cap.
    section(Facet::Freedom, &facets.freedom, 4);
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

/// A labelled line in the details pane.
fn kv(k: &str, v: String, theme: &theme::Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{k:<10}"), theme.dim()),
        Span::raw(v),
    ])
}

/// What the details pane says about a face's licence.
///
/// The verdict and its reason, not an SPDX string on its own: whether a font may be
/// studied, changed and passed on is the fact that decides whether it can be used at
/// all, and an identifier only answers that for a reader who already knows the list.
fn license_lines(face: &FaceMetadata, theme: &theme::Theme) -> Vec<Line<'static>> {
    let verdict = fontina_core::freedom::assess(face.license.spdx.as_deref());
    let mut lines = vec![
        kv(
            "license",
            face.license
                .spdx
                .clone()
                .unwrap_or_else(|| "none embedded".into()),
            theme,
        ),
        Line::from(vec![
            Span::styled(format!("{:<10}", "freedom"), theme.dim()),
            // Bold as well as coloured, because the verdict is the one thing in this
            // pane a reader may act on and colour alone cannot carry it: on a terminal
            // with none, every role here collapses to the same nothing.
            Span::styled(
                verdict.freedom.to_string(),
                match verdict.freedom {
                    fontina_core::Freedom::Free => theme.good(),
                    fontina_core::Freedom::Nonfree => theme.bad(),
                    _ => theme.warn(),
                }
                .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    if verdict.freedom != fontina_core::Freedom::Free {
        // "free" needs no explaining, and the pane shares its height with the controls
        // and the preview. Every other verdict is a reason to go and read something.
        lines.push(Line::from(Span::styled(
            format!("{:<10}{}", "", verdict.reason),
            theme.dim(),
        )));
    }
    if !face.license.reserved_font_names.is_empty() {
        lines.push(kv(
            "reserved",
            face.license.reserved_font_names.join(", "),
            theme,
        ));
    }
    if let Some(os2) = &face.os2
        && !matches!(os2.embedding.level, EmbeddingLevel::Installable)
    {
        // Reported, never acted on: these bits are the file's assertion about itself,
        // not a term of the licence. `freedom.rs` says why at length.
        lines.push(kv(
            "embedding",
            format!("{:?} (reported, not enforced)", os2.embedding.level),
            theme,
        ));
    }
    lines
}

/// The one key that quits from anywhere, including out of a full-screen mode.
fn ctrl_c(key: &event::KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

/// Axis values without trailing noise: `400`, not `400.0`; `87.5` kept as it is.
fn fmt_axis(v: f32) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v:.1}")
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

/// Rows a bordered pane has for its list.
fn pane_rows(area: Rect) -> usize {
    area.height.saturating_sub(2) as usize
}

/// A `ListState` for a pane that has already been handed only the rows it draws: the
/// widget starts at the top of what it was given, and the cursor is wherever the
/// selection falls inside that.
///
/// A fresh one each frame, deliberately. ratatui's own scrolling is what this whole
/// change is replacing, and a state that remembered an offset from a differently sized
/// window would be that scrolling coming back through a side door.
fn windowed(selected: Option<usize>, win: &std::ops::Range<usize>) -> ListState {
    let mut state = ListState::default();
    state.select(selected.filter(|i| win.contains(i)).map(|i| i - win.start));
    state
}

/// What an action did, including the part of it that did not work.
///
/// A selection makes partial success the ordinary case rather than the strange one:
/// one file in a directory nobody can write to should not hide the two hundred that
/// went through. The first failure is named because a status line is one line, and the
/// count says how many more there are to find.
fn report(verb: &str, done: usize, failed: &[String]) -> String {
    match failed {
        [] => format!("{verb}: {done} face(s)   (fontina {verb} <targets>)"),
        [only] => format!("{verb}: {done} face(s), 1 failed — {only}"),
        [first, rest @ ..] => format!(
            "{verb}: {done} face(s), {} failed — {first} (and {} more)",
            failed.len(),
            rest.len()
        ),
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

/// Rows a line takes once the paragraph has wrapped it.
///
/// Greedy, at whitespace, breaking a word only when it cannot fit on a line of its own,
/// which is how ratatui wraps. `width / columns` rounded up is a row short whenever a
/// break lands at a space: on an eighty column terminal that lost the `.ttf` off the end
/// of the file name.
///
/// Whitespace costs its own columns. The paragraph is drawn with `Wrap { trim: false }`,
/// and that keeps every space — including a run at the start of a line — so counting
/// words alone under-charges by the width of every gap. Two lines in this pane are made
/// almost entirely of such a gap: `kv` pads its label to ten columns, and the licence
/// reason is indented by ten. Under-counting there is what clips the bottom of the block,
/// which is the one thing this function exists to prevent.
fn wrapped_rows(line: &Line<'_>, cols: u16) -> u16 {
    let cols = cols.max(1) as usize;
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    let mut rows = 1usize;
    let mut used = 0usize;
    // Alternating runs of whitespace and non-whitespace. A gap is laid down as it comes;
    // a word moves to the next line whole if it does not fit and could fit alone.
    for token in tokens(&text) {
        let w = token.chars().count();
        let space = token.starts_with(char::is_whitespace);
        if used + w <= cols {
            used += w;
        } else if space {
            // A gap that runs off the end fills the line and carries the rest over.
            let over = used + w - cols;
            rows += over.div_ceil(cols);
            used = over % cols;
        } else if w <= cols {
            rows += 1;
            used = w;
        } else {
            let taken = w.div_ceil(cols);
            rows += taken;
            used = w - (taken - 1) * cols;
        }
    }
    rows.try_into().unwrap_or(u16::MAX)
}

/// `text` split into runs of whitespace and runs of everything else, in order.
fn tokens(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let space = rest.starts_with(char::is_whitespace);
        let end = rest
            .char_indices()
            .find(|(_, c)| c.is_whitespace() != space)
            .map(|(i, _)| i)
            .unwrap_or(rest.len());
        out.push(&rest[..end]);
        rest = &rest[end..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An app over the fixture fonts, with nothing drawn.
    /// An activator that answers like the real one and touches nothing.
    ///
    /// The soak presses every key hundreds of times, and `a`, `A`, `i`, `d` and `u` are
    /// keys. Against the real backend that copied fixtures into the developer's own font
    /// directory and registered them with the running login session — on every
    /// `cargo test`, on every machine, on CI. It was also most of the run time: a
    /// CoreText registration and a four-hundred-kilobyte copy, ten thousand times over.
    ///
    /// It still answers truthfully enough for the index to be exercised: `install`
    /// returns a path, `deactivate` says something was registered.
    use std::path::PathBuf;

    struct Harmless;

    impl fontina_platform::FontActivator for Harmless {
        fn install(&self, file: &Path) -> fontina_platform::Result<PathBuf> {
            Ok(file.with_extension("installed"))
        }
        fn uninstall(&self, _installed: &Path) -> fontina_platform::Result<()> {
            Ok(())
        }
        fn activate(
            &self,
            _file: &Path,
            _scope: fontina_platform::Scope,
        ) -> fontina_platform::Result<()> {
            Ok(())
        }
        fn deactivate(&self, _file: &Path) -> fontina_platform::Result<bool> {
            Ok(true)
        }
    }

    /// Where this process keeps the scanned fixtures and the copies made from them.
    ///
    /// Emptied once, on first use, so a run leaves one run's worth behind rather than
    /// every run's: the copies cannot be deleted as they are finished with, because the
    /// browser holding one is still open and Windows will not unlink an open file.
    fn scratch() -> &'static Path {
        static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
        DIR.get_or_init(|| {
            let dir = std::env::temp_dir().join(format!("fontina-ui-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            dir
        })
    }

    /// The fixtures, scanned once for the whole test binary.
    ///
    /// Parsing six fonts in a debug build costs seven seconds, and the soak builds a
    /// fresh browser for every key it holds down: fifty keys, six minutes of parsing to
    /// exercise key presses that take microseconds each. Scanning once and copying the
    /// result is the same index, arrived at the same way, without paying for it fifty
    /// times.
    fn template() -> &'static Path {
        static TEMPLATE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
        TEMPLATE.get_or_init(|| {
            let db = scratch().join("template.db");
            let _ = std::fs::remove_file(&db);
            let mut index = Index::open(&db).unwrap();
            let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
            fontina_core::scan::scan(&mut index, &[fixtures], &Default::default()).unwrap();
            db
        })
    }

    /// A browser over its own copy of the scanned fixtures.
    ///
    /// A copy, not a shared file: several of these tests write to the index — a tag, an
    /// activation record, a removed file — and one test's writes must not be another
    /// test's starting point.
    fn app() -> App {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let db = scratch().join(format!("app-{n}.db"));
        std::fs::copy(template(), &db).expect("copying the scanned fixtures");
        App::with_activator(Index::open(&db).unwrap(), Box::new(Harmless)).unwrap()
    }

    /// The activation keys record what they did, and the keys that undo them clear it.
    ///
    /// Nothing tested this before, because it could not be tested: the browser reached
    /// for the real backend, so a test of `a` would have registered a fixture with the
    /// developer's own session. With the activator behind a field the whole path is
    /// exercised — including the part that only the browser has, which is that the state
    /// a reader sees in the listing is the state that was recorded.
    #[test]
    fn the_activation_keys_record_what_they_did() {
        let press = |app: &mut App, c: char| {
            app.on_key(event::KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
                .unwrap()
        };

        for (key, want) in [
            ('a', ActivationState::User),
            ('A', ActivationState::Session),
            ('i', ActivationState::Installed),
        ] {
            let mut app = app();
            let ids = app.current_face_ids();
            assert!(!ids.is_empty(), "the first family is selected");
            press(&mut app, key);
            for id in &ids {
                let record = app.index.activation(*id).unwrap();
                assert_eq!(
                    record.as_ref().map(|r| r.state),
                    Some(want),
                    "{key:?} recorded {:?}",
                    record.as_ref().map(|r| r.state)
                );
            }
            assert!(
                app.detail_summary
                    .as_ref()
                    .is_some_and(|s| s.activation == Some(want)),
                "and the pane the reader is looking at says so"
            );

            // `d` for the two in-place states, `u` for the installed one: what the
            // command line calls deactivate and uninstall.
            press(
                &mut app,
                if want == ActivationState::Installed {
                    'u'
                } else {
                    'd'
                },
            );
            for id in &ids {
                assert!(
                    app.index.activation(*id).unwrap().is_none(),
                    "the record survived the key that undoes it"
                );
            }
        }
    }

    /// Pressing an activation key with nothing selected says so and changes nothing.
    #[test]
    fn activating_nothing_is_a_message_rather_than_a_mistake() {
        let mut app = app();
        app.query = "no font is called this".into();
        app.reload().unwrap();
        assert_eq!(app.list_len(), 0, "the listing is empty");
        app.on_key(event::KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE))
            .unwrap();
        assert!(
            app.status.contains("nothing selected"),
            "the status says why nothing happened: {:?}",
            app.status
        );
        assert!(app.index.activations().unwrap().is_empty());
    }

    /// The graphical escape hatch, and the whole of it: a specimen for what is selected,
    /// written and handed to the desktop.
    #[test]
    fn the_specimen_key_writes_a_file_for_the_selection() {
        // No desktop in a test runner, and `open_specimen` must survive that: the file is
        // the point and the handler is best-effort. `true` accepts the path and does
        // nothing, which is exactly the "it opened" branch without opening anything.
        // SAFETY: single-threaded test.
        unsafe { std::env::set_var("BROWSER", "true") };
        let mut app = app();
        let ids = app.current_face_ids();
        assert!(!ids.is_empty(), "the first family is selected");

        app.open_specimen().unwrap();
        unsafe { std::env::remove_var("BROWSER") };

        let path =
            std::env::temp_dir().join(format!("fontina-specimen-{}.html", std::process::id()));
        let html = std::fs::read_to_string(&path).expect("the specimen was written");
        assert!(html.starts_with("<!doctype html>") || html.starts_with("<!DOCTYPE html>"));
        assert!(
            html.contains("waterfall") || html.contains("compare"),
            "the specimen is where the type is actually set: {}",
            &html[..400.min(html.len())]
        );
        assert!(
            !html.contains("http://") && !html.contains("https://"),
            "a specimen makes no network request, which is why it can be opened from /tmp"
        );
        assert!(
            app.status.contains("fontina specimen"),
            "the status line names the command that would do the same thing: {}",
            app.status
        );
        let _ = std::fs::remove_file(&path);
    }

    /// Pressing it with nothing selected must say so rather than writing an empty page.
    #[test]
    fn the_specimen_key_says_when_there_is_nothing_to_show() {
        let mut app = app();
        app.list.select(None);
        app.open_specimen().unwrap();
        assert_eq!(app.status, "no face on show");
    }

    use crate::{Cli, Command};
    use clap::Parser as _;

    /// The status line is a promise: it names the command that would give you what is
    /// on screen. A facet whose flag takes no value must not be emitted with one.
    ///
    /// `Facet::Spacing` was, and clap did not complain — `list` has a positional query,
    /// so `--mono proportional` parsed as `--mono` plus a full-text search for
    /// "proportional". The copyable command meant the opposite of the screen and
    /// returned nothing.
    #[test]
    fn every_facet_emits_a_command_that_means_what_the_screen_says() {
        let mut app = app();
        // Every facet the browser can select, with a value it could really hold.
        for (facet, value) in [
            (Facet::Weight, "400"),
            (Facet::Width, "100"),
            (Facet::Style, "upright"),
            (Facet::Variable, "variable"),
            (Facet::Color, "color"),
            (Facet::Spacing, "monospace"),
            (Facet::Spacing, "proportional"),
            (Facet::Script, "Latn"),
            (Facet::Language, "en"),
            (Facet::License, "OFL-1.1"),
            (Facet::Freedom, "free"),
            (Facet::Vendor, "RSMS"),
            (Facet::Tag, "favourite"),
            (Facet::Collection, "Editorial"),
            (Facet::Activation, "none"),
            (Facet::Container, "ttf"),
        ] {
            app.selected.clear();
            app.selected.insert(facet, value.to_string());
            let line = app.command_line();
            let args: Vec<&str> = line.split_whitespace().skip(1).collect();
            // Parsing is the assertion: clap rejects an unknown flag or a value where
            // none belongs. A stray positional would not be rejected, so check for one.
            let parsed =
                Cli::try_parse_from(std::iter::once("fontina").chain(args.iter().copied()));
            let parsed =
                parsed.unwrap_or_else(|e| panic!("{facet:?}={value:?} emitted {line:?}: {e}"));
            // Both subcommands the browser names take a positional query, and a flag
            // emitted with a value it does not take lands there instead of erroring.
            // Matching only one of them is how this assertion goes quietly dead — the
            // browser opens on the family list, so `Command::List` alone never matched.
            let query = match &parsed.command {
                Command::List(args) | Command::Families(args) => args.query.clone(),
                _ => panic!("{line:?} names a subcommand this test cannot check for a query"),
            };
            assert!(
                query.is_none(),
                "{facet:?}={value:?} emitted {line:?}, which clap read as a search for {query:?}"
            );
        }
    }

    /// The two spacing buckets emit the two flags, not one flag and a word.
    #[test]
    fn the_spacing_facet_picks_the_flag_that_matches_the_bucket() {
        let mut app = app();
        app.selected.insert(Facet::Spacing, "monospace".into());
        assert!(
            app.command_line().contains("--mono"),
            "{}",
            app.command_line()
        );
        assert!(
            !app.command_line().contains("--proportional"),
            "{}",
            app.command_line()
        );
        app.selected.insert(Facet::Spacing, "proportional".into());
        assert!(
            app.command_line().contains("--proportional"),
            "{}",
            app.command_line()
        );
    }

    /// A deterministic pseudo-random source. Seeded, so a failure is reproducible from
    /// the seed the message prints, and dependency-free.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            // xorshift64*, good enough to shuffle key presses and small enough to read.
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_f491_4f6c_dd1d)
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    /// Every key the browser reacts to, plus a few it does not, because a person's
    /// keyboard has more keys than the ones we documented.
    fn key_alphabet() -> Vec<event::KeyEvent> {
        let ctrl = |c: char| event::KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
        let plain = |c: char| event::KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        let code = |k: KeyCode| event::KeyEvent::new(k, KeyModifiers::NONE);
        let mut keys: Vec<event::KeyEvent> = "jkhlgGfbaAiduRxetcmwCnp0+-/?LH \n"
            .chars()
            .map(plain)
            .collect();
        keys.extend([ctrl('f'), ctrl('b')]);
        keys.extend(
            [
                KeyCode::Down,
                KeyCode::Up,
                KeyCode::Left,
                KeyCode::Right,
                KeyCode::Enter,
                KeyCode::Esc,
                KeyCode::Tab,
                KeyCode::Backspace,
                KeyCode::Home,
                KeyCode::End,
                KeyCode::PageUp,
                KeyCode::PageDown,
                KeyCode::Delete,
                KeyCode::Insert,
                KeyCode::F(1),
            ]
            .map(code),
        );
        // Text that ends up in the search, tag and collection prompts.
        keys.extend("Amiri فونت 変".chars().map(plain));
        keys
    }

    /// Everything that has to be true of the browser between one key and the next,
    /// whatever the person pressed and in whatever order.
    fn check_invariants(app: &App, whence: &str) {
        let len = app.list_len();
        if let Some(i) = app.list.selected() {
            assert!(
                i < len.max(1),
                "{whence}: selection {i} outside a list of {len}"
            );
        }
        if len > 0 {
            assert!(
                app.list.selected().is_some(),
                "{whence}: a filled list with nothing selected"
            );
        }
        assert_eq!(
            app.detail.is_some(),
            app.detail_id.is_some(),
            "{whence}: the detail pane and the id it belongs to disagree"
        );
        if let (Some(s), Some(id)) = (&app.detail_summary, app.detail_id) {
            assert_eq!(
                s.id, id,
                "{whence}: the cached summary belongs to another face"
            );
        }
        if app.focus == Focus::Detail {
            assert!(
                !app.controls.is_empty(),
                "{whence}: focus sits on a controls pane the face does not have"
            );
        }
        if let Some(g) = &app.glyphs {
            assert!(
                g.is_empty() || g.selected_index() < g.blocks().len(),
                "{whence}: the glyph map points at a block it does not have"
            );
            assert!(
                g.covered() <= 0x11_0000,
                "{whence}: the glyph map counts more codepoints than Unicode has"
            );
        }
    }

    /// The test a daily driver needs: press keys, a great many of them, in an order
    /// nobody would choose, and require that the browser neither panics nor tells a lie
    /// about its own state. Every scripted test above walks a path someone thought of;
    /// this walks the ones nobody did.
    ///
    /// Deterministic: the seed is fixed, and a failure prints the key sequence that
    /// produced it, so it can be replayed.
    /// Hold each key down. Two hundred presses of one key, from a fresh browser, for
    /// every key there is.
    ///
    /// This is the half of the soak that randomness cannot reach: pressing `+` past the
    /// top of the preview size range takes thirty-three presses in a row, and a uniform
    /// stream of key presses will not produce that inside a run of any length anyone
    /// would wait for. It is also what a person does, by resting a finger on a key.
    /// The world changes while the browser is open.
    ///
    /// `fontina watch` is meant to run as a user service, so fonts appear and vanish
    /// from the index under a browser that is already showing them. The browser reloads
    /// after every action; this presses keys while the index is edited between them, and
    /// requires that nothing it is holding — a selection, a detail id, a cached summary —
    /// outlives what it points at.
    #[test]
    fn the_browser_survives_the_index_changing_underneath_it() {
        let mut app = app();
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
        let paths: Vec<String> = app
            .index
            .list(&Default::default())
            .unwrap()
            .into_iter()
            .map(|f| f.path)
            .collect();
        assert!(
            paths.len() >= 6,
            "the fixtures are all indexed to begin with"
        );

        let keys = "jkGgmwC\nl h";
        let mut removed = 0;
        for (round, path) in paths.iter().enumerate() {
            // A watcher drops a font that was deleted on disk.
            if app.index.remove_file(path).unwrap() {
                removed += 1;
            }
            app.reload().unwrap();
            check_invariants(&app, &format!("after {removed} face(s) vanished"));
            for c in keys.chars() {
                let key = event::KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
                app.on_key(key).unwrap();
                check_invariants(&app, &format!("round {round}, key {c:?}"));
            }
            let backend = ratatui::backend::TestBackend::new(100, 30);
            let mut term = ratatui::Terminal::new(backend).unwrap();
            term.draw(|f| app.draw(f)).unwrap();
        }
        assert_eq!(app.list_len(), 0, "every face was removed");

        // And they all come back.
        fontina_core::scan::scan(&mut app.index, &[fixtures], &Default::default()).unwrap();
        app.reload().unwrap();
        check_invariants(&app, "after everything was scanned again");
        assert!(app.list_len() > 0, "the listing came back");
    }

    #[test]
    fn holding_any_single_key_down_keeps_every_invariant() {
        for key in key_alphabet() {
            let mut app = app();
            for press in 1..=200 {
                let whence = format!("{:?} held for {press} presses", key.code);
                match app.on_key(key) {
                    Ok(Flow::Quit) => break,
                    Ok(Flow::Continue) => {}
                    Err(e) => panic!("{whence}: {e}"),
                }
                check_invariants(&app, &whence);
            }
            // And the screen still draws afterwards, at a size with no room to spare.
            let backend = ratatui::backend::TestBackend::new(40, 12);
            let mut term = ratatui::Terminal::new(backend).unwrap();
            term.draw(|f| app.draw(f))
                .unwrap_or_else(|e| panic!("drawing after {:?} was held: {e}", key.code));
        }
    }

    #[test]
    fn a_long_run_of_arbitrary_keys_keeps_every_invariant() {
        let alphabet = key_alphabet();
        // Sizes a person really has, including ones too small to lay anything out in.
        let sizes = [
            (80u16, 24u16),
            (120, 40),
            (200, 60),
            (40, 12),
            (20, 6),
            (8, 3),
            (1, 1),
        ];
        for seed in [0x5eed_0001, 0x00f0_111a, 0xdead_beef] {
            let mut rng = Rng(seed);
            let mut app = app();
            let mut pressed: Vec<String> = Vec::new();
            let mut step = 0usize;
            while step < 600 {
                // People hold keys down. A uniform stream of single presses never walks
                // to the end of a long list or the top of the preview size range, so a
                // bound that is wrong is never reached: an earlier version of this test
                // did not notice the size clamp raised from 160 to 1000. One press in
                // eight becomes a run.
                let key = alphabet[rng.below(alphabet.len())];
                let run = if rng.below(8) == 0 {
                    2 + rng.below(48)
                } else {
                    1
                };
                for _ in 0..run {
                    step += 1;
                    pressed.push(format!("{:?}", key.code));
                    let whence =
                        format!("seed {seed:#x}, step {step}, after [{}]", pressed.join(" "));
                    match app.on_key(key) {
                        Ok(Flow::Quit) => {
                            app = self::tests::app();
                            pressed.clear();
                            continue;
                        }
                        Ok(Flow::Continue) => {}
                        Err(e) => panic!("{whence}: {e}"),
                    }
                    check_invariants(&app, &whence);
                    // Draw now and then, at whatever size: the layout is where the
                    // arithmetic lives, and a pane too small to fit is where it breaks.
                    if step.is_multiple_of(7) {
                        let (w, h) = sizes[rng.below(sizes.len())];
                        let backend = ratatui::backend::TestBackend::new(w, h);
                        let mut term = ratatui::Terminal::new(backend).unwrap();
                        term.draw(|f| app.draw(f))
                            .unwrap_or_else(|e| panic!("{whence}: drawing {w}x{h}: {e}"));
                    }
                }
            }
        }
    }

    #[test]
    fn the_detail_summary_is_cached_alongside_the_face() {
        let mut app = app();
        assert!(app.detail.is_some(), "the first family is selected");
        assert_eq!(app.detail_summary.as_ref().map(|s| s.id), app.detail_id);
        // A new selection reloads both together.
        app.list.select(Some(1));
        app.refresh_detail().unwrap();
        assert!(app.detail.is_some());
        assert_eq!(app.detail_summary.as_ref().map(|s| s.id), app.detail_id);
        // Redrawing does not: the same selection is a no-op.
        let id = app.detail_id;
        app.refresh_detail().unwrap();
        assert_eq!(app.detail_id, id);
        // A change to the index still reaches the pane, through the reload that
        // follows every action.
        app.index.tag(&[id.unwrap()], "favourite").unwrap();
        app.reload().unwrap();
        assert_eq!(
            app.detail_summary.as_ref().unwrap().tags,
            ["favourite"],
            "the cache is dropped on reload"
        );
    }

    #[test]
    fn a_face_that_went_away_leaves_no_stale_id() {
        let mut app = app();
        let path = app.detail.as_ref().unwrap().file.path.clone();
        // The listing still names the face, but the index no longer holds it.
        assert!(app.index.remove_file(&path).unwrap());
        app.detail_id = None;
        app.refresh_detail().unwrap();
        assert!(app.detail.is_none());
        assert!(app.detail_id.is_none(), "a stale id outlived its face");
        assert!(app.detail_summary.is_none());
    }

    /// Select the first face whose family starts with `prefix`, as a reader would by
    /// walking the list.
    fn select_family(app: &mut App, prefix: &str) {
        let i = (0..app.list_len())
            .find(|i| {
                app.list.select(Some(*i));
                app.refresh_detail().is_ok()
                    && app
                        .detail
                        .as_ref()
                        .is_some_and(|f| f.names.family.starts_with(prefix))
            })
            .unwrap_or_else(|| panic!("no family starting with {prefix:?}"));
        app.list.select(Some(i));
        app.refresh_detail().unwrap();
    }

    #[test]
    fn the_controls_describe_the_face_on_show_and_no_other() {
        let mut app = app();
        select_family(&mut app, "Bricolage");
        assert!(!app.controls.is_empty(), "a variable face offers controls");
        let variable_len = app.controls.len();

        // Move an axis, then look at a different face: the setting does not follow it.
        app.focus = Focus::Detail;
        // This face's default sits at the top of its first axis, so the headroom is
        // downward — which is itself worth pinning: `adjust` at a bound is a no-op.
        assert!(!app.controls.adjust(1), "already at the axis maximum");
        assert!(app.controls.adjust(-5));
        let moved = app.controls.coords();
        select_family(&mut app, "Amiri");
        assert_ne!(app.controls.coords(), moved);
        select_family(&mut app, "Bricolage");
        assert_eq!(app.controls.len(), variable_len);
        assert_eq!(
            app.controls.coords(),
            fontina_core::typography::default_coords(
                app.detail.as_ref().unwrap().variable.as_ref().unwrap()
            ),
            "returning to a face starts it at its defaults again"
        );
    }

    /// Every fixture offers at least a feature toggle, so the fallback is reached by
    /// emptying the index instead: no face, no controls, nowhere for the focus to be.
    #[test]
    fn focus_never_rests_on_controls_a_face_does_not_have() {
        let mut app = app();
        select_family(&mut app, "Bricolage");
        app.focus = Focus::Detail;
        assert!(!app.controls.is_empty());

        let path = app.detail.as_ref().unwrap().file.path.clone();
        assert!(app.index.remove_file(&path).unwrap());
        app.detail_id = None;
        app.refresh_detail().unwrap();

        assert!(
            app.controls.is_empty(),
            "a face that is gone offers nothing"
        );
        assert_ne!(
            app.focus,
            Focus::Detail,
            "focus must leave a pane that no longer exists"
        );
    }

    /// Tagging, activating, searching and rescanning all call `reload`, which clears the
    /// detail. None of them changes which face is selected, so none of them may undo
    /// what the reader set on it.
    #[test]
    fn an_action_on_the_selected_face_keeps_its_axes_and_toggles() {
        let mut app = app();
        select_family(&mut app, "Bricolage");
        app.focus = Focus::Detail;
        assert!(app.controls.adjust(-4));
        app.controls.move_cursor(app.controls.len() as i32);
        assert!(app.controls.toggle(), "the last row is a feature");
        let (coords, features) = (app.controls.coords(), app.controls.forced_features());
        assert!(!features.is_empty());

        let id = app.detail_id.unwrap();
        app.index.tag(&[id], "favourite").unwrap();
        app.reload().unwrap();

        assert_eq!(app.controls.coords(), coords, "the axes survived a reload");
        assert_eq!(
            app.controls.forced_features(),
            features,
            "the toggles survived a reload"
        );
        assert_eq!(app.focus, Focus::Detail, "and so did the focus");
    }

    /// The pane is drawn from `Controls::rows`, so a rendering check is really a check
    /// that every control reaches the screen with its tag on it.
    #[test]
    fn every_control_is_drawn_with_its_tag() {
        let mut app = app();
        select_family(&mut app, "Bricolage");
        let face = app.detail.clone().unwrap();
        let drawn: Vec<String> = app
            .controls
            .rows()
            .map(|row| match row {
                controls::Row::Axis(a) => a.tag.clone(),
                controls::Row::Feature(f) => f.tag.clone(),
            })
            .collect();
        assert_eq!(drawn.len(), app.controls.len());
        for tag in &drawn {
            assert_eq!(tag.chars().count(), 4, "{tag} is not an OpenType tag");
        }
        // Axes come first, and the variable ones are exactly the face's own.
        let axes: Vec<&str> = face
            .variable
            .as_ref()
            .unwrap()
            .axes
            .iter()
            .filter(|a| !a.hidden)
            .map(|a| a.tag.as_str())
            .collect();
        assert_eq!(&drawn[..axes.len()], &axes[..]);
    }

    #[test]
    fn the_glyph_map_opens_on_the_face_on_show_and_closes_again() {
        let mut app = app();
        select_family(&mut app, "Amiri");
        assert!(app.glyphs.is_none());
        app.open_glyphs();
        let map = app.glyphs.as_ref().expect("Amiri maps codepoints");
        assert!(map.covered() > 0);
        assert!(app.status.contains("fontina glyphs"), "{}", app.status);

        // Every key it owns is taken from the panes underneath.
        assert!(app.handle_glyph_key(KeyCode::Char('j')).unwrap());
        assert!(app.handle_glyph_key(KeyCode::Char('l')).unwrap());
        // A key it does not own is swallowed rather than passed to the panes beneath:
        // activating a font from behind a full-screen map would leave the map describing
        // a face that is no longer on show.
        let id = app.detail_id;
        assert!(app.handle_glyph_key(KeyCode::Char('a')).unwrap());
        assert!(app.handle_glyph_key(KeyCode::Enter).unwrap());
        assert_eq!(app.detail_id, id, "nothing underneath moved");
        assert!(app.glyphs.is_some(), "and the map is still open");

        assert!(app.handle_glyph_key(KeyCode::Esc).unwrap());
        assert!(app.glyphs.is_none(), "Esc closes the map");
    }

    /// A pane that grew since the last keypress must not start past the end of the
    /// block and draw nothing.
    #[test]
    fn a_resize_pulls_the_scroll_back_into_range() {
        let mut app = app();
        select_family(&mut app, "Amiri");
        app.open_glyphs();
        let map = app.glyphs.as_mut().unwrap();
        // Scroll to the bottom of a narrow pane, then lay the same block out wide.
        map.scroll_by(i32::MAX / 2, 4);
        let narrow = map.scroll_row();
        assert!(narrow > 0);
        map.clamp_scroll(40);
        let covered = map.selected().unwrap().codepoints.len();
        assert!(
            map.scroll_row() * 40 < covered,
            "a widened pane still starts inside the block"
        );
    }

    /// Flatten a pane's lines to plain text, the way a reader sees them.
    fn text_of(lines: &[Line<'_>]) -> String {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_pane_gives_the_verdict_and_the_reason_for_it() {
        let mut app = app();
        select_family(&mut app, "Amiri");
        let mut face = app.detail.clone().unwrap();
        let shown = text_of(&license_lines(&face, &theme::Theme::default()));
        assert!(shown.contains("OFL-1.1"), "{shown}");
        // Not `contains("free")`: the label "freedom" contains it, so that would hold
        // even if the verdict were missing or said the opposite.
        assert!(
            shown.lines().any(|l| l.trim_end() == "freedom   free"),
            "the verdict is on its own line: {shown}"
        );
        assert!(
            !shown.contains("grants the freedom"),
            "\"free\" explains itself; the pane is shared with the preview: {shown}"
        );

        // A nonfree licence says so, and says why.
        face.license.spdx = Some("LicenseRef-Proprietary".into());
        let shown = text_of(&license_lines(&face, &theme::Theme::default()));
        assert!(shown.contains("nonfree"), "{shown}");
        assert!(shown.contains("withholds"), "{shown}");

        // A font with nothing embedded is not silently called free.
        face.license.spdx = None;
        let shown = text_of(&license_lines(&face, &theme::Theme::default()));
        assert!(shown.contains("none embedded"), "{shown}");
        assert!(shown.contains("unstated"), "{shown}");
        assert!(shown.contains("no permission"), "{shown}");
    }

    /// The pane sizes itself from wrapped rows, so a long value cannot push what comes
    /// after it off the bottom. Measured with the same arithmetic the layout uses.
    /// The paragraph wraps with `trim: false`, which keeps every space, so a gap costs
    /// columns. Counting words alone under-charged by the width of each one — and two
    /// lines in this pane are mostly gap: `kv` pads its label to ten columns and the
    /// licence reason is indented by ten.
    #[test]
    fn an_indent_costs_the_columns_it_occupies() {
        // Ten columns of indent plus twenty of text does not fit in twenty-five.
        let indented = Line::from(format!("{:<10}{}", "", "a b c d e f g h i j"));
        assert!(
            wrapped_rows(&indented, 25) > 1,
            "an indented line that overflows must be charged for the indent"
        );
        // The same words with no indent do fit.
        assert_eq!(wrapped_rows(&Line::from("a b c d e f g h i j"), 25), 1);

        // A padded label is charged its padding, not its text length.
        let padded = kv("file", "x".repeat(18), &theme::Theme::default());
        assert!(
            wrapped_rows(&padded, 25) > 1,
            "kv pads the label to ten columns, so this is 28 columns, not 22"
        );
    }

    #[test]
    fn a_long_value_does_not_cost_the_lines_below_it() {
        let long = kv(
            "file",
            "/a/very/long/path/".repeat(6),
            &theme::Theme::default(),
        );
        let short = kv("style", "Regular".into(), &theme::Theme::default());
        let rows = wrapped_rows;

        assert!(rows(&long, 30) > 1, "a long value wraps");
        assert_eq!(rows(&short, 30), 1);
        let counted: u16 = [&long, &short].iter().map(|l| rows(l, 30)).sum();
        assert!(
            counted > 2,
            "sizing by line count would have lost {} row(s)",
            counted - 2
        );
        assert_eq!(
            rows(&Line::from(""), 30),
            1,
            "an empty line still takes a row"
        );
    }

    #[test]
    fn restricted_embedding_is_shown_as_reported_not_enforced() {
        let mut app = app();
        select_family(&mut app, "Amiri");
        let mut face = app.detail.clone().unwrap();
        // Installable is the ordinary case and says nothing.
        assert!(!text_of(&license_lines(&face, &theme::Theme::default())).contains("embedding"));

        let os2 = face.os2.as_mut().unwrap();
        os2.fs_type = 0x0002;
        os2.embedding = fontina_core::model::EmbeddingRights::from_fs_type(0x0002);
        let shown = text_of(&license_lines(&face, &theme::Theme::default()));
        assert!(shown.contains("RestrictedLicense"), "{shown}");
        assert!(
            shown.contains("not enforced"),
            "the pane must not imply fontina obeys these bits: {shown}"
        );
    }

    #[test]
    fn the_freedom_facet_filters_the_listing() {
        let mut app = app();
        // Every fixture is OFL, so the whole library is free and the facet says so.
        let free = app
            .facets
            .freedom
            .iter()
            .find(|c| c.value == "free")
            .expect("a freedom facet");
        assert_eq!(
            free.count, app.facets.faces,
            "every fixture is OFL, so the whole library is free"
        );
        assert_eq!(
            app.facets.freedom.len(),
            1,
            "and nothing else is represented"
        );

        app.selected.insert(Facet::Freedom, "free".into());
        app.reload().unwrap();
        let with = app.list_len();
        assert!(with > 0, "the free fonts are still listed");
        assert_eq!(app.filter().freedom, Some(fontina_core::Freedom::Free));

        // And a state nothing is in empties the listing rather than being ignored.
        app.selected.insert(Facet::Freedom, "nonfree".into());
        app.reload().unwrap();
        assert_eq!(app.list_len(), 0, "no fixture is nonfree");
        assert_eq!(app.filter().freedom, Some(fontina_core::Freedom::Nonfree));
    }

    #[test]
    fn the_freedom_facet_reaches_the_command_line() {
        let mut app = app();
        app.selected.insert(Facet::Freedom, "free".into());
        app.reload().unwrap();
        assert!(
            app.command_line().contains("--freedom free"),
            "{}",
            app.command_line()
        );
    }

    #[test]
    fn searching_the_map_reports_what_it_found_or_did_not() {
        let mut app = app();
        select_family(&mut app, "Amiri");
        app.open_glyphs();

        app.start_input(InputKind::Glyph, String::new());
        for c in "U+0041".chars() {
            app.handle_input_key(event::KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
                .unwrap();
        }
        app.handle_input_key(event::KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap();
        assert!(app.status.starts_with("U+0041 in "), "{}", app.status);
        assert_eq!(app.glyphs.as_ref().unwrap().found(), Some(0x41));

        app.start_input(InputKind::Glyph, String::new());
        for c in "Tibetan".chars() {
            app.handle_input_key(event::KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
                .unwrap();
        }
        app.handle_input_key(event::KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap();
        assert!(app.status.contains("nothing covered"), "{}", app.status);
    }

    // ----- frames -----
    //
    // ratatui's `TestBackend` draws into a buffer instead of a terminal, so the frames
    // a reader meets can be looked at. Two things in a frame are not the browser's
    // doing and would make a snapshot say more about this machine than about the code:
    // the absolute path of the fixtures, which the details pane prints in full, and the
    // shaped preview, whose pixels belong to the rasteriser and would move under a
    // skrifa release. Both are pinned for the snapshots, and the preview has its own
    // test below.

    /// The browser drawn into an in-memory terminal, as the text a reader would see.
    fn frame(app: &mut App, width: u16, height: u16) -> String {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The same frame, made to depend on nothing but the browser: the sample text is a
    /// single space, so the preview is blank rather than a picture of whichever
    /// rasteriser is installed, and the file of the face on show is named relative to
    /// the crate rather than by the absolute path it was scanned under. The relative
    /// name still opens — the preview reads the file — because cargo runs a test with
    /// the package root as its working directory.
    fn stable_frame(app: &mut App, width: u16, height: u16) -> String {
        if let Some(face) = app.detail.as_mut() {
            let name = Path::new(&face.file.path)
                .file_name()
                .expect("a face is a file")
                .to_string_lossy()
                .into_owned();
            face.file.path = format!("../../fixtures/{name}");
        }
        frame(app, width, height)
    }

    /// The status line on its own, which is the row that tells a reader what the rest
    /// of the screen is.
    fn status_line(app: &App, width: u16) -> String {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 1)).unwrap();
        terminal.draw(|f| app.draw_status(f, f.area())).unwrap();
        let buffer = terminal.backend().buffer().clone();
        (0..buffer.area.width)
            .map(|x| buffer[(x, 0)].symbol())
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    /// The glyph map, which is what the browser is for.
    ///
    /// A mode rather than a pane: it covers the screen, because reading a font's coverage
    /// needs the width. Nothing in it comes from the rasteriser — it is the `cmap` laid
    /// out as characters — so the frame is the same everywhere and worth pinning.
    #[test]
    fn the_glyph_map_lists_the_blocks_and_lays_out_the_characters() {
        let mut app = app();
        select_family(&mut app, "Amiri");
        app.open_glyphs();
        let drawn = frame(&mut app, 120, 36);
        assert!(drawn.contains("Basic Latin"), "{drawn}");
        assert!(
            drawn.contains("Arabic"),
            "the fixture's own script is in there"
        );
        assert!(
            drawn.contains("fontina glyphs"),
            "the status line says the command that would print this"
        );
        insta::assert_snapshot!(drawn);
    }

    /// The controls pane, on a face that has axes.
    ///
    /// Every variable font is a family the browser can move through rather than a list
    /// of instances, and this is where that happens. Sliders and labels, no rasteriser.
    #[test]
    fn the_controls_pane_offers_the_axes_of_a_variable_face() {
        let mut app = app();
        select_family(&mut app, "Bricolage");
        app.focus = Focus::Detail;
        assert!(!app.controls.is_empty(), "a variable face offers controls");
        let drawn = stable_frame(&mut app, 120, 36);
        assert!(drawn.contains("wght"), "the weight axis is named: {drawn}");
        insta::assert_snapshot!(drawn);
    }

    /// A library of `n` families, made by cloning the fixtures' own.
    ///
    /// Straight into the field rather than through the index: what is being measured
    /// and asserted here is the drawing, and an index of ten thousand rows would put
    /// SQLite in the middle of a question that is not about SQLite.
    fn with_families(app: &mut App, n: usize) {
        let one = app.families[0].clone();
        app.families = (0..n)
            .map(|i| {
                let mut f = one.clone();
                f.name = format!("Family {i:05}");
                f
            })
            .collect();
        app.open_family = None;
    }

    /// The change itself: a pane draws a screenful whether it is holding six rows or
    /// ten thousand, and the window it drew is the window it says it drew.
    #[test]
    fn a_pane_draws_a_screenful_however_much_it_is_holding() {
        let mut app = app();
        with_families(&mut app, 10_000);
        app.list.select(Some(0));

        let drawn = stable_frame(&mut app, 120, 36);
        assert_eq!(app.list_offset, 0, "it opens at the top");
        assert!(drawn.contains("Family 00000"), "{drawn}");
        assert!(
            !drawn.contains("Family 00100"),
            "a pane thirty rows tall drew the hundredth row"
        );

        // Down the list: the cursor stays on the screen and the window follows it.
        app.jump(9_999).unwrap();
        let drawn = stable_frame(&mut app, 120, 36);
        assert!(drawn.contains("Family 09999"), "the last family: {drawn}");
        assert!(!drawn.contains("Family 00000"), "and not the first as well");
        assert!(
            app.list_offset > 9_900,
            "the window followed the cursor to the end, not to {}",
            app.list_offset
        );
    }

    /// A reader who scrolled somewhere and then narrowed the search should still be
    /// where they were, and a filter that shortens the list past the offset should
    /// bring the pane back rather than leave it looking at nothing.
    #[test]
    fn a_filter_leaves_the_pane_where_the_reader_left_it() {
        let mut app = app();
        with_families(&mut app, 1_000);
        app.list.select(Some(500));
        stable_frame(&mut app, 120, 36);
        let (offset, selected) = (app.list_offset, app.list.selected());
        assert!(offset > 400, "the pane scrolled to the cursor");

        // Nothing about the list changed, so nothing about the view should either.
        stable_frame(&mut app, 120, 36);
        assert_eq!((app.list_offset, app.list.selected()), (offset, selected));

        // And a list that shrinks under the offset comes back into view rather than
        // drawing an empty pane.
        with_families(&mut app, 12);
        app.list.select(Some(0));
        let drawn = stable_frame(&mut app, 120, 36);
        assert_eq!(app.list_offset, 0, "{drawn}");
        assert!(drawn.contains("Family 00000"), "{drawn}");
    }

    /// What it costs to draw a frame, at the three scales the item asks about.
    ///
    /// Ignored by default: it is a measurement rather than an assertion, and a number
    /// from a shared runner is not one worth failing a build over. `cargo test -p
    /// fontina-cli --bins -- --ignored --nocapture what_a_frame_costs`.
    #[test]
    #[ignore = "a measurement, not an assertion"]
    fn what_a_frame_costs_at_a_hundred_a_thousand_and_ten_thousand() {
        for n in [100usize, 1_000, 10_000] {
            let mut app = app();
            with_families(&mut app, n);
            app.list.select(Some(n / 2));
            frame(&mut app, 120, 36);
            let start = std::time::Instant::now();
            const FRAMES: u32 = 200;
            for _ in 0..FRAMES {
                frame(&mut app, 120, 36);
            }
            let per = start.elapsed().as_secs_f64() * 1000.0 / f64::from(FRAMES);
            println!("{n:>6} families: {per:.3} ms per frame");
        }
    }

    /// Turn the event loop's collecting step until the worker has nothing owed.
    ///
    /// What the loop does between frames, without the frames. Bounded, so a worker that
    /// never answers fails the test rather than hanging the suite.
    fn settle(app: &mut App) {
        // Generous: this is a net for a worker that never answers, not a measurement.
        // Every test in this file spawns one, and a loaded machine running them in
        // parallel is not the same as a browser answering a keystroke.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while app.search.as_ref().is_some_and(search::Search::waiting) {
            app.collect().unwrap();
            assert!(
                std::time::Instant::now() < deadline,
                "the worker never answered"
            );
            // Yield. Spinning here starves the very thread being waited on: the suite
            // runs these in parallel and every one of them owns a worker, so a hot
            // loop per test is a core each taken away from the work they are waiting
            // for. It passed alone and timed out in company, which is the signature.
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    /// The whole of the item, at the level a reader meets it: typing filters the list,
    /// and no keystroke waits for the index to say so.
    #[test]
    fn typing_filters_the_list_without_any_keystroke_waiting_for_it() {
        let mut app = app();
        assert!(
            app.search.is_some(),
            "the fixtures' index is a file, so there is a worker to talk to"
        );
        assert_eq!(app.families.len(), 5);

        let press = |app: &mut App, c: char| {
            app.on_key(event::KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
                .unwrap();
        };
        press(&mut app, '/');
        for c in "Amiri".chars() {
            press(&mut app, c);
            // Every keystroke leaves a browser that can still draw, which is the thing
            // that was not true when the query ran here.
            frame(&mut app, 120, 36);
        }
        assert_eq!(app.query, "Amiri", "the box has what was typed");

        settle(&mut app);
        assert_eq!(
            app.families
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            ["Amiri"],
            "the answer to the last thing typed is the one on the screen"
        );

        // And backspacing all the way out brings the rest back.
        for _ in 0.."Amiri".len() {
            app.on_key(event::KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
                .unwrap();
        }
        settle(&mut app);
        assert_eq!(app.families.len(), 5, "{:?}", app.families);
    }

    /// A burst of keystrokes leaves the browser showing the last one, not whichever
    /// answer happened to arrive last.
    #[test]
    fn a_burst_of_typing_ends_on_the_answer_to_the_last_character() {
        let mut app = app();
        app.on_key(event::KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE))
            .unwrap();
        // Type a query and then take it apart again, with no settling in between, so
        // several generations are in flight at once.
        for c in "Inter".chars() {
            app.on_key(event::KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
                .unwrap();
        }
        for _ in 0..3 {
            app.on_key(event::KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
                .unwrap();
        }
        assert_eq!(app.query, "In");
        settle(&mut app);
        assert_eq!(
            app.families
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            ["Inter"],
            "the list is the answer to \"In\", not to something typed on the way"
        );
    }

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(event::KeyEvent::new(code, KeyModifiers::NONE))
            .unwrap();
    }

    /// The point of the whole item: one key marks, and every action that takes a face
    /// takes the marked ones instead.
    #[test]
    fn a_marked_set_is_what_the_next_action_acts_on() {
        let mut app = app();
        assert_eq!(app.current_face_ids().len(), 1, "Amiri has one face");

        // Space marks the row under the cursor. A family row stands for its faces, so
        // marking Inter marks both of them.
        // Amiri, Bricolage, Inter, Nabla, Source Serif 4.
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Char(' '));
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Char(' '));
        assert_eq!(
            app.marked.len(),
            3,
            "Bricolage's one face and Inter's two: {:?}",
            app.marked
        );
        assert_eq!(
            app.current_face_ids().len(),
            3,
            "the action takes the mark, not the row under the cursor"
        );

        // And marking is a toggle.
        press(&mut app, KeyCode::Char(' '));
        assert_eq!(app.marked.len(), 1);

        // Esc clears it, and the cursor is what actions mean again.
        press(&mut app, KeyCode::Esc);
        assert!(app.marked.is_empty());
        assert_eq!(app.status, "selection cleared");
        assert_eq!(app.current_face_ids(), app.face_ids_at(app.list.selected()));
    }

    /// `v`, a move, `v`. The decision for the whole range is taken from where it
    /// started, so dragging over a mixture makes them all the same.
    #[test]
    fn a_range_marks_from_the_anchor_to_the_cursor() {
        let mut app = app();
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(
            app.mark_anchor,
            Some(0),
            "the anchor is where v was pressed"
        );
        assert!(app.marked.is_empty(), "and nothing is marked until it ends");
        for _ in 0..3 {
            press(&mut app, KeyCode::Down);
        }
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(app.mark_anchor, None, "the range is committed");
        // Amiri, Bricolage, Inter (two faces) and Nabla.
        assert_eq!(app.marked.len(), 5, "{:?}", app.marked);

        // Running it again over the same rows unmarks them, because the row the range
        // starts on is already marked.
        press(&mut app, KeyCode::Char('v'));
        for _ in 0..3 {
            press(&mut app, KeyCode::Up);
        }
        press(&mut app, KeyCode::Char('v'));
        assert!(app.marked.is_empty(), "{:?}", app.marked);
    }

    /// `*` takes everything the filter matches, and takes it back.
    #[test]
    fn star_marks_everything_the_filter_matches_and_then_nothing() {
        let mut app = app();
        press(&mut app, KeyCode::Char('*'));
        assert_eq!(app.marked.len(), 6, "every face in the fixtures");
        press(&mut app, KeyCode::Char('*'));
        assert!(app.marked.is_empty(), "and again is the way back");
    }

    /// A mark is a promise about what an action will touch, so a face the filter has
    /// taken off the screen cannot stay in it — and the reader has to be told.
    #[test]
    fn a_filter_drops_the_marks_it_no_longer_shows_and_says_so() {
        let mut app = app();
        press(&mut app, KeyCode::Char('*'));
        assert_eq!(app.marked.len(), 6);

        app.query = "Amiri".into();
        app.reload().unwrap();
        assert_eq!(app.marked.len(), 1, "only Amiri still matches");
        assert!(
            app.status.contains("no longer match"),
            "the reader was not told: {:?}",
            app.status
        );

        // And a filter that matches none of them says that instead of saying nothing.
        app.query = "Nabla".into();
        app.reload().unwrap();
        assert!(app.marked.is_empty());
        assert!(app.status.contains("none of the"), "{:?}", app.status);
    }

    /// A marked set makes partial success the ordinary case: one file nobody can write
    /// must not hide the ones that went through, or take them down with it.
    #[test]
    fn a_partial_failure_says_what_worked_and_what_did_not() {
        assert_eq!(
            report("activate", 12, &[]),
            "activate: 12 face(s)   (fontina activate <targets>)"
        );
        assert_eq!(
            report("activate", 11, &["/x/a.ttf: denied".into()]),
            "activate: 11 face(s), 1 failed — /x/a.ttf: denied"
        );
        let many = ["/x/a.ttf: denied".to_string(), "/x/b.ttf: denied".into()];
        assert_eq!(
            report("activate", 10, &many),
            "activate: 10 face(s), 2 failed — /x/a.ttf: denied (and 1 more)"
        );
    }

    /// The count belongs where the reader is looking when they reach for the key, not
    /// only in a status line the next message overwrites.
    #[test]
    fn the_pane_says_how_many_are_marked() {
        let mut app = app();
        let plain = stable_frame(&mut app, 120, 36);
        assert!(plain.contains("5 families"), "{plain}");

        press(&mut app, KeyCode::Char(' '));
        let marked = stable_frame(&mut app, 120, 36);
        assert!(marked.contains("1 of 6 selected"), "{marked}");
        assert!(marked.contains('●'), "and the row itself is marked");

        // The mark column is only there while it is doing something. Everything but
        // the status line comes back byte for byte; the status line is a message about
        // what just happened, and is meant to differ.
        press(&mut app, KeyCode::Esc);
        let cleared = stable_frame(&mut app, 120, 36);
        let panes = |f: &str| {
            f.lines()
                .filter(|l| !l.contains("selection cleared") && !l.contains("$ fontina"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(panes(&cleared), panes(&plain), "a column was left behind");
    }

    /// Type a tag or a collection name into the prompt the key opened.
    fn type_into(app: &mut App, open: char, text: &str) {
        press(app, KeyCode::Char(open));
        for c in text.chars() {
            press(app, KeyCode::Char(c));
        }
        press(app, KeyCode::Enter);
    }

    /// Undo puts back what was there, which is not the same as undoing what was asked
    /// for: a face that already carried the tag has to keep it.
    #[test]
    fn undoing_a_tag_takes_it_off_only_the_faces_that_gained_it() {
        let mut app = app();
        // Amiri gets the tag on its own first.
        type_into(&mut app, 't', "display");
        let amiri = app.current_face_ids();

        // Then everything gets it, Amiri included.
        press(&mut app, KeyCode::Char('*'));
        type_into(&mut app, 't', "display");
        let tagged = |app: &App| {
            app.index
                .list(&FaceFilter {
                    tag: Some("display".into()),
                    ..Default::default()
                })
                .unwrap()
                .len()
        };
        assert_eq!(tagged(&app), 6, "every face carries it now");

        press(&mut app, KeyCode::Char('U'));
        assert_eq!(
            tagged(&app),
            amiri.len(),
            "undo took the tag off the face that had it before anybody pressed anything"
        );
        assert!(app.status.starts_with("undone: removed"), "{}", app.status);
    }

    /// A selection holding three activation states goes back to three states, not to
    /// whichever one the batch happened to leave.
    #[test]
    fn undoing_an_activation_puts_every_face_back_in_its_own_state() {
        let mut app = app();
        // Amiri activated for the user, Bricolage until logout, the rest untouched.
        press(&mut app, KeyCode::Char('a'));
        let amiri = app.current_face_ids();
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Char('A'));
        let bricolage = app.current_face_ids();

        let state = |app: &App, ids: &[i64]| app.index.activation(ids[0]).unwrap().map(|r| r.state);
        assert_eq!(state(&app, &amiri), Some(ActivationState::User));
        assert_eq!(state(&app, &bricolage), Some(ActivationState::Session));

        // Now install everything in one action, over three different prior states.
        press(&mut app, KeyCode::Char('*'));
        press(&mut app, KeyCode::Char('i'));
        assert_eq!(state(&app, &amiri), Some(ActivationState::Installed));

        press(&mut app, KeyCode::Char('U'));
        assert_eq!(
            state(&app, &amiri),
            Some(ActivationState::User),
            "the face that was activated for the user is again"
        );
        assert_eq!(
            state(&app, &bricolage),
            Some(ActivationState::Session),
            "and the one that was activated until logout"
        );
        let untouched = app
            .index
            .list(&FaceFilter {
                family: Some("Nabla".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            state(&app, &[untouched[0].id]),
            None,
            "and the ones that were not activated at all are not activated at all"
        );
    }

    /// A whole selection is one undo, and Ctrl-R is the way back.
    #[test]
    fn a_selection_is_one_undo_and_ctrl_r_is_the_way_forward_again() {
        let mut app = app();
        press(&mut app, KeyCode::Char('*'));
        type_into(&mut app, 'c', "print");
        let in_collection = |app: &App| {
            app.index
                .list(&FaceFilter {
                    collection: Some("print".into()),
                    ..Default::default()
                })
                .unwrap()
                .len()
        };
        assert_eq!(in_collection(&app), 6);

        press(&mut app, KeyCode::Char('U'));
        assert_eq!(in_collection(&app), 0, "one keystroke, not six");
        assert!(app.status.starts_with("undone: took"), "{}", app.status);

        app.on_key(event::KeyEvent::new(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL,
        ))
        .unwrap();
        assert_eq!(in_collection(&app), 6, "and Ctrl-R puts it back");
        assert!(app.status.starts_with("redone:"), "{}", app.status);
    }

    /// Nothing to undo is a sentence, not a surprise. And a rescan records nothing,
    /// because what a rescan changed is what the disk says and there is no earlier
    /// state to restore.
    #[test]
    fn what_cannot_be_put_back_is_not_offered() {
        let mut app = app();
        press(&mut app, KeyCode::Char('U'));
        assert_eq!(app.status, "nothing to undo");

        app.rescan().unwrap();
        press(&mut app, KeyCode::Char('U'));
        assert_eq!(
            app.status, "nothing to undo",
            "a rescan offered an undo it cannot honour"
        );
    }

    /// `:` puts every command the program has in front of the reader, filtered as
    /// they type, and Enter does the one the browser knows how to do.
    #[test]
    fn the_palette_lists_every_command_and_runs_the_ones_the_browser_has() {
        let press = |app: &mut App, code: KeyCode| {
            app.on_key(event::KeyEvent::new(code, KeyModifiers::NONE))
                .unwrap();
        };
        let mut app = app();
        press(&mut app, KeyCode::Char(':'));
        let drawn = stable_frame(&mut app, 120, 36);
        assert!(drawn.contains("activate"), "{drawn}");
        assert!(drawn.contains("tag add"), "including the nested ones");
        assert!(
            drawn.contains("⏎ writes the command"),
            "and what each one will do"
        );

        // Typing is typing: `a` filters rather than activating anything.
        press(&mut app, KeyCode::Char('a'));
        press(&mut app, KeyCode::Char('c'));
        press(&mut app, KeyCode::Char('t'));
        assert_eq!(
            app.palette.as_ref().map(|p| p.query.as_str()),
            Some("act"),
            "a key while the palette is up went to the browser"
        );
        assert!(
            app.index.activation(1).unwrap().is_none(),
            "and it activated something"
        );

        // Enter on a command the browser implements presses its key.
        let ids = app.current_face_ids();
        press(&mut app, KeyCode::Enter);
        assert!(app.palette.is_none(), "the palette closed behind it");
        assert_eq!(
            app.index.activation(ids[0]).unwrap().map(|r| r.state),
            Some(ActivationState::User),
            "activate ran: {}",
            app.status
        );
    }

    /// A command that only prints is written out with the selection in it rather than
    /// run, because the browser is using the screen it would print to.
    #[test]
    fn a_command_that_prints_is_written_out_rather_than_run() {
        let mut app = app();
        app.on_key(event::KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE))
            .unwrap();
        for c in "info".chars() {
            app.on_key(event::KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
                .unwrap();
        }
        let ids = app.current_face_ids();
        app.on_key(event::KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap();
        assert!(
            app.status.starts_with(&format!("fontina info {}", ids[0])),
            "the command carries what is on the screen: {}",
            app.status
        );
        assert!(app.status.contains("prints"), "{}", app.status);
    }

    /// The listing commands describe the whole screen, so they carry the filter rather
    /// than a list of ids.
    #[test]
    fn a_listing_command_carries_the_filter_and_not_the_ids() {
        let mut app = app();
        app.selected.insert(Facet::Variable, "variable".into());
        app.reload().unwrap();
        let line = app.command_for("facets");
        assert!(line.starts_with("fontina facets"), "{line}");
        assert!(line.contains("--variable"), "{line}");
    }

    /// Anything that writes to the disk asks first, and anything but a yes is a no.
    #[test]
    fn a_command_that_writes_to_the_disk_asks_before_it_runs() {
        let press = |app: &mut App, code: KeyCode| {
            app.on_key(event::KeyEvent::new(code, KeyModifiers::NONE))
                .unwrap();
        };
        for (answer, want_installed) in [(KeyCode::Char('n'), false), (KeyCode::Char('y'), true)] {
            let mut app = app();
            let ids = app.current_face_ids();
            press(&mut app, KeyCode::Char(':'));
            for c in "install".chars() {
                press(&mut app, KeyCode::Char(c));
            }
            press(&mut app, KeyCode::Enter);
            assert_eq!(
                app.palette.as_ref().and_then(|p| p.confirming.as_deref()),
                Some("install"),
                "it went ahead without asking"
            );
            let asking = stable_frame(&mut app, 120, 36);
            assert!(asking.contains("writes to the disk"), "{asking}");

            press(&mut app, answer);
            assert!(app.palette.is_none());
            assert_eq!(
                app.index.activation(ids[0]).unwrap().is_some(),
                want_installed,
                "answering {answer:?} did the wrong thing: {}",
                app.status
            );
        }
    }

    /// Type into the filter bar, one key at a time, the way a reader does.
    fn type_filter(app: &mut App, line: &str) {
        press(app, KeyCode::Char('F'));
        // The bar opens pre-filled, so clear it first: this is what a reader who wants
        // a different filter does.
        while app.input.as_ref().is_some_and(|i| !i.buf.is_empty()) {
            press(app, KeyCode::Backspace);
        }
        for c in line.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    /// The whole of the item: a filter the facet pane cannot express, applied as it is
    /// typed, with the count in front of the reader while they compose it.
    #[test]
    fn the_filter_bar_applies_the_flags_as_they_are_typed() {
        let mut app = app();
        assert_eq!(app.facets.faces, 6);

        type_filter(&mut app, "--script Arab");
        assert_eq!(app.facets.faces, 1, "the panes moved as the line was typed");
        assert_eq!(app.count_line(), "1 face");
        assert_eq!(
            app.families
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            ["Amiri"]
        );

        // Enter keeps it, and the status line says the command that would repeat it.
        press(&mut app, KeyCode::Enter);
        assert!(app.input.is_none());
        assert_eq!(app.facets.faces, 1);
        assert!(
            app.command_line().contains("--script Arab"),
            "{}",
            app.command_line()
        );
    }

    /// A range and two scripts at once — the things the facet pane has no way to say.
    #[test]
    fn the_bar_says_what_the_facets_cannot() {
        let mut app = app();
        type_filter(&mut app, "--weight 300-500 --variable=false");
        press(&mut app, KeyCode::Enter);
        assert!(app.facets.faces < 6, "{} faces", app.facets.faces);
        assert!(
            app.families.iter().all(|f| !f.variable),
            "a variable family survived --variable=false"
        );
    }

    /// A half-typed flag is not a filter, so the panes stay on the last line that
    /// parsed and the prompt says what is wrong rather than blanking the list.
    #[test]
    fn a_line_that_does_not_parse_leaves_the_panes_alone_and_says_why() {
        let mut app = app();
        type_filter(&mut app, "--script Arab");
        assert_eq!(app.facets.faces, 1);

        for c in " --wieght".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        assert!(app.filter_error.is_some(), "the mistake went unnoticed");
        assert_eq!(app.facets.faces, 1, "the panes were blanked mid-word");
        let drawn = stable_frame(&mut app, 120, 36);
        assert!(
            drawn.contains("--wieght"),
            "the prompt says which word: {drawn}"
        );

        // And correcting it puts the count back.
        for _ in 0.." --wieght".len() {
            press(&mut app, KeyCode::Backspace);
        }
        assert!(app.filter_error.is_none());
    }

    /// Esc puts back what was there, panes and all.
    #[test]
    fn esc_restores_the_filter_the_bar_opened_over() {
        let mut app = app();
        type_filter(&mut app, "--script Arab");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.facets.faces, 1);

        // A filter with a different count, so "it changed" is visible: two of the
        // fixtures are variable and only one is Arabic.
        type_filter(&mut app, "--variable");
        assert_eq!(app.facets.faces, 2, "the new line took effect");
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.facets.faces, 1, "Esc did not put the old filter back");
        assert_eq!(app.filter_line.as_deref(), Some("--script Arab"));
    }

    /// The bar opens as an editable copy of the screen, not as an empty box.
    #[test]
    fn the_bar_opens_pre_filled_with_what_the_panes_are_showing() {
        let mut app = app();
        app.selected.insert(Facet::Variable, "variable".into());
        app.reload().unwrap();
        press(&mut app, KeyCode::Char('F'));
        assert_eq!(
            app.input.as_ref().map(|i| i.buf.as_str()),
            Some("--variable"),
            "the bar started empty over a filtered screen"
        );
    }

    /// One keystroke from a composed filter to a collection of what it matched.
    #[test]
    fn ctrl_s_saves_what_the_filter_matched_as_a_collection() {
        let mut app = app();
        type_filter(&mut app, "--script Arab");
        app.on_key(event::KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        ))
        .unwrap();
        assert_eq!(
            app.input.as_ref().map(|i| i.kind),
            Some(InputKind::Collection),
            "the next thing typed should be the name"
        );
        assert_eq!(app.marked.len(), 1, "everything it matched is marked");

        for c in "arabic".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.index
                .list(&FaceFilter {
                    collection: Some("arabic".into()),
                    ..Default::default()
                })
                .unwrap()
                .len(),
            1,
            "{}",
            app.status
        );
    }

    /// A typed filter and the facet pane are two ways of saying the same thing, and
    /// there is no sensible way to add one facet to `--variable=false`. Toggling one
    /// takes the facets back, and says so rather than half-keeping the line.
    #[test]
    fn toggling_a_facet_takes_the_facets_back_from_a_typed_filter() {
        let mut app = app();
        type_filter(&mut app, "--script Arab");
        press(&mut app, KeyCode::Enter);
        assert!(app.filter_override.is_some());

        app.focus = Focus::Facets;
        press(&mut app, KeyCode::Enter);
        assert!(app.filter_override.is_none(), "the line was half-kept");
        assert!(app.filter_line.is_none());
        assert!(
            app.status.contains("filter bar was cleared"),
            "{}",
            app.status
        );
    }

    #[test]
    fn the_browser_opens_on_the_family_list() {
        let mut app = app();
        let drawn = stable_frame(&mut app, 120, 36);
        assert!(drawn.contains("5 families"), "{drawn}");
        assert!(
            drawn.lines().next_back().unwrap().starts_with(" / search"),
            "the key line is the last row on the screen"
        );
        insta::assert_snapshot!(drawn);
    }

    #[test]
    fn opening_a_family_lists_its_faces_and_says_so_in_the_command() {
        let mut app = app();
        select_family(&mut app, "Inter");
        app.open_family().unwrap();
        assert_eq!(app.open_family.as_deref(), Some("Inter"));
        insta::assert_snapshot!(stable_frame(&mut app, 120, 36));
    }

    #[test]
    fn the_help_overlay_sits_over_the_browser() {
        let mut app = app();
        app.help = true;
        let drawn = stable_frame(&mut app, 120, 36);
        assert!(
            drawn.contains("any key to close"),
            "the overlay does not say how to leave it"
        );
        insta::assert_snapshot!(drawn);
    }

    #[test]
    fn the_status_line_says_what_the_screen_is() {
        let mut app = app();
        let mut rows = vec![status_line(&app, 100)];
        assert_eq!(rows[0].trim(), "$ fontina families");

        // A filter is a flag, and the line is a command that can be pasted.
        app.selected.insert(Facet::Variable, "variable".into());
        app.selected.insert(Facet::Vendor, "ATLR".into());
        app.reload().unwrap();
        rows.push(status_line(&app, 100));

        // While something is being typed, the line is the prompt instead.
        app.start_input(InputKind::Search, "grot".into());
        rows.push(status_line(&app, 100));

        // And after an action, what the action did, until the next reload.
        app.input = None;
        app.status = "tagged 1 face as favourite".into();
        rows.push(status_line(&app, 100));

        insta::assert_snapshot!(rows.join("\n"));
    }

    /// Every row a frame draws fits the terminal it was drawn into.
    ///
    /// Run at each breakpoint and at both sides of each one, because a layout bug
    /// shows up as a row one column too long and a terminal answers that by wrapping,
    /// which moves every row below it.
    fn no_row_overflows(app: &mut App, width: u16, height: u16) {
        let drawn = stable_frame(app, width, height);
        assert_eq!(drawn.lines().count(), height as usize, "one row per line");
        for (n, row) in drawn.lines().enumerate() {
            assert!(
                row.chars().count() <= width as usize,
                "row {n} is {} columns on a {width}-column terminal: {row}",
                row.chars().count()
            );
        }
    }

    #[test]
    fn no_width_a_terminal_can_be_makes_the_browser_overflow_it() {
        let mut app = app();
        for width in [40, 59, 60, 75, 76, 80, 99, 100, 120, 200] {
            no_row_overflows(&mut app, width, 24);
        }
    }

    /// Wide: the browser as designed, three panes side by side.
    #[test]
    fn the_third_pane_appears_at_the_width_that_can_afford_it() {
        let mut app = app();
        let drawn = stable_frame(&mut app, layout::THREE, 24);
        assert!(drawn.contains("faces"), "the facet pane is there: {drawn}");
        assert!(drawn.contains("families"), "and the list");
        assert!(drawn.contains("Details"), "and the face");
        insta::assert_snapshot!(drawn);
    }

    /// Medium: the pane the browser exists for gets the room, and the facets wait.
    ///
    /// This is the width the item was filed at. Three panes here left the face pane
    /// twenty-two columns and a path wrapped over four lines; two panes leave it
    /// fifty, which is a path on one line and a preview underneath it.
    #[test]
    fn an_eighty_column_terminal_gives_the_face_pane_room_to_be_read() {
        let mut app = app();
        select_family(&mut app, "Amiri");
        let drawn = stable_frame(&mut app, 80, 24);
        assert!(drawn.contains("Details"), "{drawn}");
        assert!(
            !drawn.contains(" faces "),
            "the facet pane is a Tab away, not on the screen"
        );
        let file = drawn
            .lines()
            .find(|l| l.contains("file "))
            .expect("the face names its file");
        assert!(
            file.contains("Amiri-Regular.ttf"),
            "the path fits on its own row rather than wrapping: {file}"
        );
        insta::assert_snapshot!(drawn);
    }

    #[test]
    fn the_facets_come_over_the_list_when_there_is_no_room_beside_it() {
        let mut app = app();
        app.focus = Focus::Facets;
        let drawn = stable_frame(&mut app, 80, 24);
        assert!(
            drawn.contains("⇥ closes"),
            "a pane over another says how to put it back: {drawn}"
        );
        assert!(
            drawn.contains("Details"),
            "and it only covers the list, not the face"
        );
        insta::assert_snapshot!(drawn);
    }

    /// Narrow: one pane, and the others a Tab away.
    #[test]
    fn a_sixty_column_terminal_shows_one_pane_at_a_time() {
        let mut app = app();
        let drawn = stable_frame(&mut app, 60, 24);
        assert!(
            drawn.contains("families"),
            "the list has the focus: {drawn}"
        );
        assert!(
            !drawn.contains("Details"),
            "and nothing else is competing for the width"
        );
        insta::assert_snapshot!(drawn);

        // list, face, facets, and round again.
        app.cycle_focus();
        app.cycle_focus();
        let facets = stable_frame(&mut app, 60, 24);
        assert!(
            facets.contains(" faces "),
            "Tab reaches the facets: {facets}"
        );
        assert!(
            !facets.contains("5 families"),
            "and the list is not under them: {facets}"
        );
    }

    /// The reason the face pane had to become a place the focus can rest: at sixty
    /// columns it is the only way to see a face at all, and a font browser that cannot
    /// show anyone a font has stopped being one.
    #[test]
    fn one_pane_at_a_time_can_still_reach_the_face() {
        let mut app = app();
        select_family(&mut app, "Amiri");
        // A face with nothing to adjust, which is the case the rule is about: with
        // controls the pane took focus already.
        app.controls = controls::Controls::default();

        // Drawn once so the browser knows how wide the terminal is. `frame` rather
        // than `stable_frame` throughout: the stable one blanks the sample text on the
        // app so that a snapshot cannot depend on the rasteriser, and the preview is
        // what this test is looking for. It takes no snapshot, so it can draw for real.
        frame(&mut app, 60, 24);
        app.cycle_focus();
        assert_eq!(app.focus, Focus::Detail, "Tab reaches it from the list");
        let drawn = frame(&mut app, 60, 24);
        assert!(
            drawn.contains("Amiri"),
            "the face is on the screen: {drawn}"
        );
        assert!(
            drawn.contains("upm"),
            "with the measurements under it: {drawn}"
        );

        // And beside the others it is a readout again: Tab skips a pane where nothing
        // the reader presses would do anything.
        frame(&mut app, 120, 36);
        app.focus = Focus::List;
        app.cycle_focus();
        assert_eq!(app.focus, Focus::Facets, "no stop on an inert face pane");
    }

    /// A focus can outlive its pane: a terminal dragged narrow takes the face pane's
    /// seat away while the cursor is in it.
    #[test]
    fn narrowing_the_terminal_moves_a_focus_that_has_nowhere_left_to_be() {
        let mut app = app();
        select_family(&mut app, "Amiri");
        app.controls = controls::Controls::default();
        stable_frame(&mut app, 60, 24);
        app.focus = Focus::Detail;
        stable_frame(&mut app, 120, 36);
        assert_eq!(
            app.focus,
            Focus::List,
            "a face with no controls cannot hold the focus beside the other panes"
        );
    }

    /// The third acceptance criterion: a value that did not fit says so.
    ///
    /// The lists cut with an ellipsis, and the face pane wraps rather than cutting, so
    /// the failure this guards against is a row that simply ends. Every family in the
    /// fixtures is short enough to fit, so the test makes one that is not.
    #[test]
    fn a_name_too_long_for_its_pane_is_cut_visibly() {
        let mut app = app();
        let long = "Bricolage Grotesque Extremely Wide Display Titling".to_string();
        app.families[0].name = long.clone();
        for width in [60, 80, 120] {
            let drawn = stable_frame(&mut app, width, 24);
            assert!(
                drawn.contains('…') || drawn.contains(&long),
                "{width}: the name was cut with nothing to show for it: {drawn}"
            );
        }
    }

    /// The key line used to be one long string, cut by the terminal wherever it ran
    /// out — which under 170 columns took `? help` with it.
    #[test]
    fn the_key_line_keeps_the_key_that_finds_the_others() {
        let mut app = app();
        for width in [40, 60, 80, 120, 200] {
            let drawn = stable_frame(&mut app, width, 24);
            let keys = drawn.lines().next_back().unwrap();
            assert!(
                keys.ends_with("? help"),
                "{width} columns: {keys:?} does not say where the rest are"
            );
        }
    }

    /// The help is the one screen a reader reaches when they are already lost, so it
    /// is the last place to end three lines early without saying so.
    #[test]
    fn the_help_says_when_it_does_not_fit_and_scrolls_when_it_does_not() {
        let mut app = app();
        app.help = true;
        let tall = stable_frame(&mut app, 100, 40);
        assert!(tall.contains("any key to close"), "all of it fits: {tall}");
        assert!(!tall.contains("j/k scrolls"), "so it says nothing about it");

        let short = stable_frame(&mut app, 100, 20);
        assert!(
            short.contains("j/k scrolls"),
            "a box that cannot hold its text says so: {short}"
        );
        assert!(!short.contains("any key to close"), "because it is cut off");

        app.on_key(event::KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE))
            .unwrap();
        assert!(app.help, "j scrolls the help rather than closing it");
        assert_ne!(
            stable_frame(&mut app, 100, 20),
            short,
            "and the box moved when it did"
        );
        app.on_key(event::KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
            .unwrap();
        assert!(!app.help, "anything else still closes it");
    }
}
