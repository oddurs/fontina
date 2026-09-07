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

//! An optional login agent that re-applies activations after a reboot.
//!
//! `fontina activate` records what it did, but a session-scoped activation does not
//! outlive the session. `fontina restore` puts them back; this makes the operating
//! system run it at login so nobody has to remember.
//!
//! Off unless asked for. Per-user everywhere: a user unit under `$XDG_CONFIG_HOME`, a
//! LaunchAgent in the user's own `~/Library`, a script in the user's Startup folder.
//! Nothing is written outside the home directory and nothing needs elevation, so
//! installing the agent can never affect anyone else on the machine.
//!
//! What gets written is decided by [`plan`], which touches no disk, so the exact file
//! for all three systems can be tested from any one of them — which matters, because
//! each CI runner would otherwise only ever exercise its own.

use crate::{PlatformError, Result};
use std::path::{Path, PathBuf};

/// The reverse-DNS name the agent is registered under. Stable: it is the key macOS and
/// systemd use to find an already-installed agent, so changing it would orphan one.
pub const LABEL: &str = "dev.fontina.restore";

/// What installing the agent would write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentPlan {
    /// Where the file goes.
    pub path: PathBuf,
    /// What it contains.
    pub contents: String,
    /// The mechanism, for a human: `systemd user unit`, `LaunchAgent`, `Startup folder`.
    pub kind: &'static str,
    /// What the reader still has to run for the agent to start working now, when the
    /// system needs a step beyond the file existing.
    pub activate_with: Option<String>,
    /// The counterpart, for removal. Deleting the file is not always enough: systemd
    /// keeps an enablement symlink, and launchd keeps a loaded job until logout.
    pub deactivate_with: Option<String>,
}

/// Whether the agent is installed, and whether the system will really run it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStatus {
    pub path: PathBuf,
    /// fontina's own agent is at that path — [`Ownership::Ours`] or [`Ownership::Edited`],
    /// both of which the system will run at the next login.
    ///
    /// False when something else occupies it: a directory, somebody's own file, a
    /// symbolic link. Those are not an installed agent and saying they are would tell a
    /// reader that fonts will come back at login when nothing will; [`AgentStatus::ownership`]
    /// is what says what is in the way, and it is why `install` refuses.
    pub installed: bool,
    /// False when the file is there but the system has not been told to run it — which
    /// on systemd is a separate step, and is the difference between an agent and a file.
    pub enabled: bool,
    /// Whose the file is, and whether it still says what fontina wrote.
    pub ownership: Ownership,
}

/// Whether the thing at the agent's path is fontina's to replace or to remove.
///
/// This is the whole of the ownership rule, and [`install`], [`uninstall`] and [`status`]
/// all read it from the same place: fontina writes over, and deletes, a file that fontina
/// wrote and nothing else. It is the same rule `copy_slot`/`is_copy_slot` give the font
/// backends in `lib.rs`, which stopped `install` adopting a font somebody had put in
/// their own font directory by hand; a font is recognised by the name fontina gave it,
/// and a text file by a marker line fontina writes into it.
///
/// The marker carries a checksum of the rest of the file, so a unit somebody added an
/// `Environment=` or an `ExecStartPre=` to comes back as [`Ownership::Edited`] rather
/// than [`Ownership::Ours`], and can be refused instead of thrown away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    /// Nothing at the path at all — not even a broken symbolic link.
    Absent,
    /// A regular file fontina wrote, byte for byte as it wrote it.
    Ours,
    /// A regular file fontina wrote, changed since. Removing it would take somebody's
    /// edit with it.
    Edited,
    /// Anything else: a file fontina never wrote, a directory, or a symbolic link —
    /// which is never followed, because the file at the far end is not the agent's.
    Foreign,
}

/// A systemd user service that runs `restore` at login.
///
/// `Type=oneshot` because the job exits and must not be restarted. `PartOf` and
/// `WantedBy` both name `graphical-session.target`: `After=` alone orders units within a
/// transaction that this one would never join, so it would have been inert and `restore`
/// could have run before the session existed.
pub fn systemd_unit(exe: &Path, args: &[String]) -> String {
    let mut command = quoted(&exe.display().to_string());
    for a in args {
        command.push(' ');
        command.push_str(&quoted(a));
    }
    systemd_unit_from(&command)
}

/// The unit around an `ExecStart` line that is already quoted word by word.
fn systemd_unit_from(command: &str) -> String {
    format!(
        "[Unit]\n\
         Description=Re-apply fontina font activations\n\
         Documentation=man:fontina(1)\n\
         PartOf=graphical-session.target\n\
         After=graphical-session.target\n\
         \n\
         [Service]\n\
         Type=oneshot\n\
         ExecStart={}\n\
         \n\
         [Install]\n\
         WantedBy=graphical-session.target\n",
        systemd_escape(command)
    )
}

/// `%` starts a systemd specifier, so a literal one has to be doubled. Quoting is done
/// per word by [`quoted`] before this is reached — the two together are what stop a path
/// containing a space from losing everything after it and failing 203/EXEC at every
/// login, silently.
fn systemd_escape(command: &str) -> String {
    command.replace('%', "%%")
}

/// A LaunchAgent that runs `restore` once at login.
///
/// `RunAtLoad` with `KeepAlive` false: the job exits, and launchd should not read that
/// as a crash and restart it for ever.
pub fn launch_agent(args: &[String]) -> String {
    let mut arguments = String::new();
    for a in args {
        arguments.push_str(&format!("\t\t<string>{}</string>\n", xml_escape(a)));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \t<key>Label</key>\n\t<string>{LABEL}</string>\n\
         \t<key>ProgramArguments</key>\n\t<array>\n{arguments}\t</array>\n\
         \t<key>RunAtLoad</key>\n\t<true/>\n\
         \t<key>KeepAlive</key>\n\t<false/>\n\
         </dict>\n\
         </plist>\n"
    )
}

/// A Startup-folder script that runs `restore` at login.
///
/// The Run registry key would do the same, but a file in the user's own Startup folder
/// needs no registry crate and no new dependency, and a reader can find and delete it
/// without a registry editor.
pub fn startup_script(exe: &Path, args: &[String]) -> String {
    // cmd.exe expands `%VAR%` when it reads the file, so a `%` in a path or in a `--db`
    // argument has to be doubled — the same hazard systemd has, and it was escaped there
    // and not here. Without this the agent would quietly restore from a different index.
    let cmd = |s: &str| s.replace('%', "%%");
    let extra: String = args.iter().map(|a| format!(" \"{}\"", cmd(a))).collect();
    format!(
        "@echo off\r\nstart \"\" /b \"{}\"{extra}\r\n",
        cmd(&exe.display().to_string())
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Quote one word of a systemd `ExecStart` line. Only systemd needs it — launchd takes
/// an argument array and the Startup script does its own quoting — but `systemd_unit` is
/// public and generates that file on any platform, so this is not gated.
fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

// ---------------------------------------------------------------------------------
// Ownership: the marker line, and what is at the agent's path
// ---------------------------------------------------------------------------------

/// The word that opens fontina's marker line, and the version of the line's format.
///
/// All three systems read a text file with a comment syntax, so one line at the end of
/// what fontina generated is enough to say "fontina wrote this", including on Windows:
/// the Startup entry is a `.cmd` script rather than a `.lnk` shortcut, so it takes a
/// `rem` line exactly as the unit takes a `#` one and the plist an XML comment. Nothing
/// here needs a shortcut parser or the registry.
///
/// The version is in the line so a later format can be recognised and refused rather
/// than silently mistaken for an edit.
const MARK: &str = "fontina-agent";
const MARK_VERSION: &str = "v1";

/// `body` with fontina's marker line appended, commented for the file it is going into:
/// `open` before it, `close` after it, `newline` at the end of it.
///
/// The line carries a checksum of everything else in the file, which is what lets
/// [`ownership_of`] separate a file fontina wrote from one somebody edited afterwards.
/// It goes last so that neither systemd, launchd nor cmd.exe reads a word of it before
/// the part that matters: a `#` comment closes a systemd unit, an XML comment after
/// `</plist>` is well-formed trailing Misc, and `rem` under `@echo off` prints nothing.
fn marked(body: &str, open: &str, close: &str, newline: &str) -> String {
    let sum = blake3::hash(body.as_bytes()).to_hex();
    format!("{body}{open}{MARK} {MARK_VERSION} {sum}{close}{newline}")
}

/// The checksum a marker line records, if `line` is one.
fn marker_sum(line: &str) -> Option<&str> {
    let mut words = line.split_whitespace().skip_while(|w| *w != MARK);
    if words.next() != Some(MARK) || words.next() != Some(MARK_VERSION) {
        return None;
    }
    words.next()
}

/// The byte range of the marker line in `contents`, and the checksum it records.
///
/// The last one wins: a file that somehow contains two is fontina's most recent write
/// plus whatever came before it, and that comparison will fail, which is the answer we
/// want anyway.
fn find_marker(contents: &str) -> Option<(usize, usize, &str)> {
    let mut start = 0;
    let mut found = None;
    while start < contents.len() {
        let end = contents[start..]
            .find('\n')
            .map_or(contents.len(), |i| start + i + 1);
        let line = contents[start..end].trim_end_matches(['\n', '\r']);
        if let Some(sum) = marker_sum(line) {
            found = Some((start, end, sum));
        }
        start = end;
    }
    found
}

/// Whether the text of a file is what fontina wrote.
///
/// The marker line is cut out and the rest checksummed, so an edit anywhere counts —
/// above the marker (a changed `ExecStart`) or below it (an appended `Nice=`) — and
/// neither can be passed off as fontina's own work.
fn ownership_of(contents: &str) -> Ownership {
    let Some((start, end, sum)) = find_marker(contents) else {
        return Ownership::Foreign;
    };
    let mut body = String::with_capacity(contents.len());
    body.push_str(&contents[..start]);
    body.push_str(&contents[end..]);
    if blake3::hash(body.as_bytes()).to_hex().as_str() == sum {
        Ownership::Ours
    } else {
        Ownership::Edited
    }
}

/// What is at `path`, without following a symbolic link and without trusting what the
/// file says about itself beyond the checksum.
///
/// `symlink_metadata` rather than `exists`: a link at the agent's path is a link,
/// dangling or not, and reading through it would be reading somebody else's file —
/// writing through it would be writing somebody else's file, which is how a unit ended
/// up in a `important.conf` two directories away.
fn inspect(path: &Path) -> Result<Ownership> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Ownership::Absent),
        Err(e) => return Err(PlatformError::Io(path.to_path_buf(), e)),
    };
    if !meta.is_file() {
        return Ok(Ownership::Foreign);
    }
    match std::fs::read_to_string(path) {
        Ok(c) => Ok(ownership_of(&c)),
        // Not text, so not a file fontina generated.
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => Ok(Ownership::Foreign),
        Err(e) => Err(PlatformError::Io(path.to_path_buf(), e)),
    }
}

/// Why fontina will not touch what is at `path`, said plainly enough that the reader can
/// go and look. `verb` is what was refused: `replaced`, or `removed`.
///
/// Refuse rather than act, in every case. Somebody else's file and somebody's edit are
/// both things fontina cannot put back, and a refusal costs one command to clear; the
/// crate takes the same line in `PlatformError::AlreadyPresent`, where a font already in
/// the user's font directory by hand stops an install instead of being adopted.
fn refusal(path: &Path, what: Ownership, verb: &str) -> PlatformError {
    let at = path.display();
    let what = match what {
        Ownership::Edited => format!("{at} was written by fontina and edited since"),
        _ => {
            let kind = match std::fs::symlink_metadata(path) {
                Ok(m) if m.file_type().is_symlink() => "a symbolic link",
                Ok(m) if m.is_dir() => "a directory",
                _ => "a file",
            };
            format!("{at} is {kind} fontina did not write")
        }
    };
    PlatformError::Os(format!(
        "{what}, so it will not be {verb}; move it aside or remove it yourself"
    ))
}

/// Where the agent lives on this system, and what it would contain.
///
/// `exe` is the binary to run and `args` everything after it — `restore`, plus `--db`
/// when the reader keeps their index somewhere other than the default, because an agent
/// that restores from an index nobody uses would report success and do nothing.
///
/// `None` when there is no home directory, which is the only way this fails before
/// anything is written.
pub fn plan(exe: &Path, args: &[String]) -> Option<AgentPlan> {
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let dir = directories::BaseDirs::new()?
            .config_dir()
            .join("systemd/user");
        Some(AgentPlan {
            path: dir.join(format!("{LABEL}.service")),
            contents: marked(&systemd_unit(exe, args), "# ", "", "\n"),
            kind: "systemd user unit",
            activate_with: Some(format!("systemctl --user enable --now {LABEL}.service")),
            deactivate_with: Some(format!("systemctl --user disable --now {LABEL}.service")),
        })
    }
    #[cfg(target_os = "macos")]
    {
        let dir = crate::home()?.join("Library/LaunchAgents");
        let mut argv = vec![exe.display().to_string()];
        argv.extend(args.iter().cloned());
        Some(AgentPlan {
            path: dir.join(format!("{LABEL}.plist")),
            contents: marked(&launch_agent(&argv), "<!-- ", " -->", "\n"),
            kind: "LaunchAgent",
            activate_with: Some(format!(
                "launchctl load ~/Library/LaunchAgents/{LABEL}.plist"
            )),
            deactivate_with: Some(format!(
                "launchctl unload ~/Library/LaunchAgents/{LABEL}.plist"
            )),
        })
    }
    #[cfg(target_os = "windows")]
    {
        let dir = directories::BaseDirs::new()?
            .data_dir()
            .join("Microsoft/Windows/Start Menu/Programs/Startup");
        Some(AgentPlan {
            path: dir.join("fontina-restore.cmd"),
            contents: marked(&startup_script(exe, args), "rem ", "", "\r\n"),
            kind: "Startup folder",
            // The folder is read at the next login; there is nothing to load or unload.
            activate_with: None,
            deactivate_with: None,
        })
    }
}

/// Where systemd records that a user unit is enabled. The unit file existing is not the
/// same as the system agreeing to run it.
#[cfg(all(unix, not(target_os = "macos")))]
fn enable_link() -> Option<PathBuf> {
    Some(
        directories::BaseDirs::new()?
            .config_dir()
            .join("systemd/user/graphical-session.target.wants")
            .join(format!("{LABEL}.service")),
    )
}

/// Write the agent. Returns what was written, so a caller can say where it went.
pub fn install(exe: &Path, args: &[String]) -> Result<AgentPlan> {
    if !exe.is_absolute() {
        // systemd rejects a relative ExecStart outright and launchd will not resolve
        // one either, so writing the file would only produce an agent that fails at
        // every login while reporting success here.
        return Err(PlatformError::Os(format!(
            "{} is not an absolute path, so no login agent can point at it",
            exe.display()
        )));
    }
    let plan = plan(exe, args).ok_or(PlatformError::NoUserDir)?;
    // Only ever replace a file fontina wrote and nobody has touched since. Anything else
    // at that path — somebody's own unit under the same name, a directory, a symbolic
    // link left by a dotfile manager — belongs to whoever put it there, and `fs::write`
    // would truncate it, or follow the link and truncate a file somewhere else entirely.
    match inspect(&plan.path)? {
        Ownership::Absent | Ownership::Ours => {}
        other => return Err(refusal(&plan.path, other, "replaced")),
    }
    if let Some(dir) = plan.path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| PlatformError::Io(dir.to_path_buf(), e))?;
    }
    std::fs::write(&plan.path, &plan.contents)
        .map_err(|e| PlatformError::Io(plan.path.clone(), e))?;
    Ok(plan)
}

/// Remove the agent's file. `Ok(false)` means there was none.
///
/// Only fontina's own file, unedited, is removed: [`Ownership`] says why, and anything
/// else is an error naming the path rather than a deletion nobody asked for. A unit
/// somebody has added a line to is refused too — the edit is theirs, `uninstall` cannot
/// put it back, and one `rm` clears the refusal if losing it is what they meant.
///
/// The OS registration is not removed here — see [`AgentPlan::deactivate_with`], which
/// the caller should show. Doing it silently would mean running `systemctl` or
/// `launchctl` on the reader's behalf, which this crate does not do anywhere else.
pub fn uninstall() -> Result<bool> {
    let plan = plan(Path::new("/fontina"), &[]).ok_or(PlatformError::NoUserDir)?;
    match inspect(&plan.path)? {
        Ownership::Absent => Ok(false),
        Ownership::Ours => {
            // `remove_file`, on a path just seen to be a regular file fontina wrote. It
            // does not follow symbolic links, which is why one is refused above rather
            // than quietly unlinked: the link is somebody's, and so is its target.
            std::fs::remove_file(&plan.path)
                .map_err(|e| PlatformError::Io(plan.path.clone(), e))?;
            Ok(true)
        }
        other => Err(refusal(&plan.path, other, "removed")),
    }
}

/// Whether an agent is installed here, and whether the system will run it.
pub fn status() -> Option<AgentStatus> {
    let plan = plan(Path::new("/fontina"), &[])?;
    // The same look `install` and `uninstall` take, so the three cannot disagree about
    // whether anything is there. `Path::exists` used to be asked instead, and it follows
    // symbolic links: a dangling one made `status` say nothing was installed while
    // `uninstall` went ahead and removed it.
    let ownership = inspect(&plan.path).unwrap_or(Ownership::Foreign);
    let installed = matches!(ownership, Ownership::Ours | Ownership::Edited);
    #[cfg(all(unix, not(target_os = "macos")))]
    let enabled = installed && enable_link().is_some_and(|l| l.exists());
    // launchd reads ~/Library/LaunchAgents at login and the Startup folder is scanned as
    // it is, so on those two the file being there is the whole story.
    #[cfg(not(all(unix, not(target_os = "macos"))))]
    let enabled = installed;
    Some(AgentStatus {
        path: plan.path,
        installed,
        enabled,
        ownership,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args() -> Vec<String> {
        vec!["restore".to_string()]
    }

    #[test]
    fn the_systemd_unit_runs_restore_once_at_login() {
        let unit = systemd_unit(Path::new("/usr/bin/fontina"), &[]);
        assert!(unit.contains("ExecStart=\"/usr/bin/fontina\""));
        assert!(
            unit.contains("Type=oneshot"),
            "the job exits; it is not a daemon"
        );
        assert!(
            !unit.contains("Restart="),
            "a oneshot that exits must not restart"
        );
        // `After=` alone orders units inside a transaction this one never joins, so the
        // unit has to be part of the session target as well as ordered after it.
        assert!(unit.contains("PartOf=graphical-session.target"));
        assert!(unit.contains("WantedBy=graphical-session.target"));
    }

    /// systemd splits `ExecStart` on whitespace, so an unquoted path with a space would
    /// lose everything after it and fail 203/EXEC at every login, silently.
    #[test]
    fn a_systemd_path_with_a_space_or_a_percent_survives() {
        let unit = systemd_unit(Path::new("/home/u/font tools/fontina"), &args());
        assert!(
            unit.contains("ExecStart=\"/home/u/font tools/fontina\" \"restore\""),
            "{unit}"
        );
        // `%` starts a systemd specifier and has to be doubled to mean itself.
        let unit = systemd_unit(Path::new("/opt/100%pure/fontina"), &[]);
        assert!(unit.contains("100%%pure"), "{unit}");
    }

    #[test]
    fn the_launch_agent_runs_once_and_is_not_kept_alive() {
        let plist = launch_agent(&["/usr/local/bin/fontina".into(), "restore".into()]);
        assert!(plist.contains("<string>dev.fontina.restore</string>"));
        assert!(plist.contains("<string>/usr/local/bin/fontina</string>"));
        assert!(plist.contains("<string>restore</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>\n\t<true/>"));
        assert!(
            plist.contains("<key>KeepAlive</key>\n\t<false/>"),
            "launchd would otherwise restart a job that correctly exits"
        );
    }

    #[test]
    fn a_path_with_xml_in_it_cannot_break_the_plist() {
        let plist = launch_agent(&["/Users/a&b/<fontina>".into(), "restore".into()]);
        assert!(plist.contains("/Users/a&amp;b/&lt;fontina&gt;"));
        assert!(
            !plist.contains("/Users/a&b/<fontina>"),
            "the raw path must not survive into the XML"
        );
    }

    /// cmd.exe expands `%VAR%` when it reads the file, exactly as systemd expands a
    /// specifier — the same hazard, and it used to be escaped on one side only.
    #[test]
    fn a_percent_in_a_windows_path_or_argument_is_not_expanded() {
        let cmd = startup_script(
            Path::new(r"C:\Users\me\%WORK%\fontina.exe"),
            &[
                "restore".into(),
                "--db".into(),
                r"C:\%WORK%\fonts.db".into(),
            ],
        );
        assert!(cmd.contains("%%WORK%%"), "{cmd}");
        assert!(
            !cmd.contains(r"\%WORK%\fontina"),
            "the raw form must not survive: {cmd}"
        );
    }

    #[test]
    fn the_startup_script_is_quoted_and_uses_crlf() {
        let cmd = startup_script(Path::new(r"C:\Program Files\fontina.exe"), &args());
        assert!(
            cmd.contains("\"C:\\Program Files\\fontina.exe\" \"restore\""),
            "a path with a space has to survive: {cmd}"
        );
        assert!(cmd.contains("\r\n"), "a .cmd file wants CRLF");
    }

    /// The index location has to travel with the agent, or it restores from an index
    /// nobody uses and reports success.
    #[test]
    fn a_custom_index_reaches_every_generator() {
        let extra = vec![
            "restore".to_string(),
            "--db".to_string(),
            "/srv/fonts.db".to_string(),
        ];
        let unit = systemd_unit(Path::new("/usr/bin/fontina"), &extra);
        assert!(unit.contains("\"--db\" \"/srv/fonts.db\""), "{unit}");

        let mut argv = vec!["/usr/bin/fontina".to_string()];
        argv.extend(extra.clone());
        let plist = launch_agent(&argv);
        assert!(plist.contains("<string>--db</string>"));
        assert!(plist.contains("<string>/srv/fonts.db</string>"));

        let cmd = startup_script(Path::new("C:\\fontina.exe"), &extra);
        assert!(cmd.contains("\"--db\" \"/srv/fonts.db\""), "{cmd}");
    }

    /// Whatever the system, the agent goes under the directory that system reserves for
    /// the user, and it runs `restore`.
    #[test]
    fn the_plan_stays_in_the_users_own_directory() {
        let Some(plan) = plan(Path::new("/opt/fontina"), &args()) else {
            return; // no home directory in this environment
        };
        assert!(plan.contents.contains("restore"));
        // Not the home directory: XDG_CONFIG_HOME may legitimately point elsewhere, and
        // asserting on home would fail in a sandbox that does exactly that.
        #[cfg(all(unix, not(target_os = "macos")))]
        let root = directories::BaseDirs::new().map(|b| b.config_dir().to_path_buf());
        #[cfg(not(all(unix, not(target_os = "macos"))))]
        let root = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf());
        if let Some(root) = root {
            assert!(
                plan.path.starts_with(&root),
                "{} is outside {}",
                plan.path.display(),
                root.display()
            );
        }
    }

    /// The marker line is added to the end of what was generated, in each of the three
    /// comment syntaxes, and an edit anywhere in the file is visible afterwards.
    ///
    /// Tested for all three from whichever platform is running, the same reason [`plan`]
    /// touches no disk: each CI runner would otherwise only ever exercise its own.
    #[test]
    fn the_marker_recognises_fontinas_own_file_in_every_comment_syntax() {
        let mut argv = vec!["/usr/bin/fontina".to_string()];
        argv.extend(args());
        for (open, close, newline, body) in [
            (
                "# ",
                "",
                "\n",
                systemd_unit(Path::new("/usr/bin/fontina"), &args()),
            ),
            ("<!-- ", " -->", "\n", launch_agent(&argv)),
            (
                "rem ",
                "",
                "\r\n",
                startup_script(Path::new("C:\\fontina.exe"), &args()),
            ),
        ] {
            let file = marked(&body, open, close, newline);
            assert!(
                file.starts_with(&body),
                "the marker is appended, never woven into the part the system reads: {file}"
            );
            assert!(file.contains("fontina-agent v1 "), "{file}");
            assert_eq!(ownership_of(&file), Ownership::Ours);

            // An edit above the marker (a changed ExecStart) and below it (an appended
            // line) are both edits, and neither can pass as fontina's own work.
            assert_eq!(
                ownership_of(&format!("{file}Nice=5{newline}")),
                Ownership::Edited
            );
            assert_eq!(
                ownership_of(&format!("Nice=5{newline}{file}")),
                Ownership::Edited
            );
            // And what fontina never marked is nobody's but its author's.
            assert_eq!(ownership_of(&body), Ownership::Foreign);
        }
    }

    /// The plist's marker has to come after the root element, where an XML comment is
    /// still well-formed, and not inside the dictionary launchd reads.
    #[test]
    fn the_plists_marker_sits_outside_the_property_list() {
        let plist = marked(
            &launch_agent(&["/usr/bin/fontina".into()]),
            "<!-- ",
            " -->",
            "\n",
        );
        let (before, after) = plist
            .split_once("</plist>")
            .expect("a closing root element");
        assert!(!before.contains(MARK), "not inside the plist: {plist}");
        assert!(after.contains(MARK), "after it: {plist}");
    }

    /// Whatever `install` writes, a later `uninstall` has to recognise as its own — the
    /// two meet only through the file, so the marker has to be in the plan itself.
    #[test]
    fn the_plan_carries_the_marker_that_makes_it_recognisable_later() {
        let Some(plan) = plan(Path::new("/opt/fontina"), &args()) else {
            return; // no home directory in this environment
        };
        assert_eq!(ownership_of(&plan.contents), Ownership::Ours);
        assert!(plan.contents.contains("restore"), "still the agent it was");
    }

    #[test]
    fn an_uninstall_names_the_step_the_install_asked_for() {
        let Some(plan) = plan(Path::new("/opt/fontina"), &args()) else {
            return;
        };
        // Either the system needs both a load and an unload, or neither.
        assert_eq!(
            plan.activate_with.is_some(),
            plan.deactivate_with.is_some(),
            "a system that must be told to start must be told to stop"
        );
    }
}
