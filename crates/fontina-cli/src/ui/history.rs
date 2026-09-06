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

//! What the browser did, and how to take it back.
//!
//! With a selection, one keystroke changes a hundred rows. A tool that makes a hundred
//! changes at once and offers no way back is one people use carefully rather than
//! fluently, which is the opposite of the point.
//!
//! Two rules keep this honest, and both are tests below.
//!
//! **A batch is one change.** Marking two hundred faces and pressing `a` is one entry,
//! so undoing it is one keystroke and not two hundred. The whole reason to have this is
//! the thing that made it necessary.
//!
//! **What cannot be reversed exactly is not offered.** Every change here records the
//! state it replaced rather than assuming one: tagging faces that were already tagged
//! records that they were, so undo removes the tag from the ones that gained it and
//! leaves the rest alone. A rescan changes what the index knows about the disk and
//! there is no state to put back, so nothing is recorded and `U` says so rather than
//! doing something approximate.
//!
//! In memory, for the session. An undo history that outlived the process would be a
//! claim about a filesystem several other programs can also write to.

use fontina_core::ActivationState;

/// The activation a face had before something changed it.
///
/// `None` is "not activated at all", which is a state like any other and the one an
/// undo most often has to restore.
pub type Prior = (i64, Option<ActivationState>);

/// One thing the browser did, described by how to undo it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A tag was added to, or taken from, exactly these faces.
    ///
    /// Only the ones it actually moved: tagging a hundred faces of which forty already
    /// carried the tag records the sixty, so undo does not take a tag off a face that
    /// had it before anybody pressed anything.
    Tag {
        ids: Vec<i64>,
        name: String,
        added: bool,
    },
    /// The same, for a collection.
    Collection {
        ids: Vec<i64>,
        name: String,
        added: bool,
    },
    /// Activation state, per face, as it was.
    ///
    /// Per face rather than one state for the batch, because a selection can hold
    /// faces in three different states and putting them all back to one of them is not
    /// an undo.
    Activation { prior: Vec<Prior> },
}

impl Change {
    /// Whether it moved anything. A change that touched nothing is not worth a place
    /// in the history, and offering to undo it would be a lie about what happened.
    pub fn is_empty(&self) -> bool {
        match self {
            Change::Tag { ids, .. } | Change::Collection { ids, .. } => ids.is_empty(),
            Change::Activation { prior } => prior.is_empty(),
        }
    }

    /// What to say when this is undone, in the browser's own words.
    pub fn describe(&self) -> String {
        match self {
            Change::Tag { ids, name, added } => format!(
                "{} {:?} {} {} face(s)",
                if *added { "removed" } else { "put back" },
                name,
                if *added { "from" } else { "on" },
                ids.len()
            ),
            Change::Collection { ids, name, added } => format!(
                "{} {} face(s) {} {:?}",
                if *added { "took" } else { "put back" },
                ids.len(),
                if *added { "out of" } else { "into" },
                name
            ),
            Change::Activation { prior } => {
                format!("put {} face(s) back as they were", prior.len())
            }
        }
    }
}

/// What has been done, and what has been taken back.
///
/// Bounded: a session that runs for a week should not hold every keystroke of it. The
/// oldest entries fall off the bottom, which is what every editor does and what nobody
/// notices.
#[derive(Default)]
pub struct History {
    done: Vec<Change>,
    undone: Vec<Change>,
}

/// Deep enough that no ordinary session reaches the end of it, and shallow enough that
/// the ids it holds are a rounding error against the index they name.
const DEPTH: usize = 64;

impl History {
    /// Record something that happened. Anything undone is no longer reachable, which
    /// is the rule every editor has: a new action is a new branch of the past.
    pub fn record(&mut self, change: Change) {
        if change.is_empty() {
            return;
        }
        self.done.push(change);
        if self.done.len() > DEPTH {
            self.done.remove(0);
        }
        self.undone.clear();
    }

    /// The next thing to undo, taken off the stack.
    pub fn undo(&mut self) -> Option<Change> {
        self.done.pop()
    }

    /// The next thing to redo.
    pub fn redo(&mut self) -> Option<Change> {
        self.undone.pop()
    }

    /// Put an undone change on the redo stack. Separate from `record`, because
    /// recording is what clears the redo stack and an undo must not clear it.
    pub fn undone(&mut self, change: Change) {
        self.undone.push(change);
    }

    /// Put a redone change back on the undo stack, for the same reason.
    pub fn redone(&mut self, change: Change) {
        self.done.push(change);
    }

    #[cfg(test)]
    pub fn depth(&self) -> (usize, usize) {
        (self.done.len(), self.undone.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(n: usize) -> Change {
        Change::Tag {
            ids: (0..n as i64).collect(),
            name: "display".into(),
            added: true,
        }
    }

    /// A batch is one entry, which is the whole reason this exists: with a selection,
    /// one keystroke changes a hundred rows.
    #[test]
    fn two_hundred_faces_in_one_action_are_one_undo() {
        let mut h = History::default();
        h.record(tag(200));
        assert_eq!(h.depth(), (1, 0));
        let change = h.undo().expect("something to undo");
        assert!(matches!(&change, Change::Tag { ids, .. } if ids.len() == 200));
        assert_eq!(h.depth(), (0, 0), "and it came off in one piece");
    }

    /// A change that moved nothing is not a change.
    #[test]
    fn an_action_that_touched_nothing_is_not_recorded() {
        let mut h = History::default();
        h.record(tag(0));
        h.record(Change::Activation { prior: Vec::new() });
        assert_eq!(h.depth(), (0, 0));
        assert!(h.undo().is_none(), "there was something to take back");
    }

    /// Undo, redo, and the rule that a new action ends the redo branch.
    #[test]
    fn a_new_action_puts_the_redo_branch_out_of_reach() {
        let mut h = History::default();
        h.record(tag(3));
        let change = h.undo().unwrap();
        h.undone(change);
        assert_eq!(h.depth(), (0, 1), "there is something to redo");

        let change = h.redo().unwrap();
        h.redone(change);
        assert_eq!(h.depth(), (1, 0), "and redoing puts it back to undo");

        // Undo it again, then do something new: the redo is gone.
        let change = h.undo().unwrap();
        h.undone(change);
        h.record(tag(9));
        assert_eq!(h.depth(), (1, 0), "a new action kept a stale redo alive");
    }

    /// A week-long session does not hold a week of keystrokes.
    #[test]
    fn the_history_has_a_bottom_and_the_oldest_falls_out_of_it() {
        let mut h = History::default();
        for i in 1..=DEPTH + 10 {
            h.record(tag(i));
        }
        assert_eq!(h.depth(), (DEPTH, 0));
        // The newest is still the next one back.
        assert!(matches!(h.undo(), Some(Change::Tag { ids, .. }) if ids.len() == DEPTH + 10));
    }

    /// What the status line says is what happened, not "undone".
    #[test]
    fn every_change_says_what_it_was_in_words() {
        assert_eq!(
            Change::Tag {
                ids: vec![1, 2],
                name: "display".into(),
                added: true
            }
            .describe(),
            "removed \"display\" from 2 face(s)"
        );
        assert_eq!(
            Change::Collection {
                ids: vec![1],
                name: "print".into(),
                added: false
            }
            .describe(),
            "put back 1 face(s) into \"print\""
        );
        assert_eq!(
            Change::Activation {
                prior: vec![(1, None), (2, Some(ActivationState::User))]
            }
            .describe(),
            "put 2 face(s) back as they were"
        );
    }
}
