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

//! The listing queries, off the thread that draws.
//!
//! Every filter change — a keystroke in the search box, a facet toggled, a family
//! opened — asks the index three questions: the facet counts, and either the families
//! or the faces. On the fixtures that is instant. On a real library it is tens of
//! milliseconds, and it used to be spent on the drawing thread, so typing a six-letter
//! family name was six of them in a row: the browser stopped answering for as long as
//! it took, and five of the six answers were for a query nobody would ever read.
//!
//! So the queries move to a worker, and the drawing thread only ever sends and
//! receives. Three things make that safe, and each of them is a test below:
//!
//! 1. **Every request carries a generation**, which only goes up. A reply whose
//!    generation is not the newest one asked for is dropped on arrival, so an answer
//!    that overtakes a newer one on the way back cannot put a stale list on the screen.
//! 2. **Queued requests are coalesced.** The worker takes the newest waiting request
//!    and throws the rest away without running them, because their answers are already
//!    known to be unwanted.
//! 3. **The request already running is interrupted.** Coalescing cannot help the one
//!    inside SQLite; `Interrupt` is SQLite's own way to stop it. An interrupted query
//!    comes back as an error, which is correct and is why an error on a superseded
//!    generation is silence rather than a message.
//!
//! The protocol is the part that is easy to get wrong, so it is written against a
//! `Worker` trait rather than against the index, and the tests below drive it with a
//! worker that sleeps and counts instead of one that opens a database.

use fontina_core::{FaceFilter, FaceSummary, Facets, Family, Index, Interrupt};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::thread;

/// What one filter looks like once answered.
pub struct Listing {
    pub facets: Facets,
    /// Families, when no family is open.
    pub families: Vec<Family>,
    /// Faces, when one is.
    pub faces: Vec<FaceSummary>,
}

/// A filter, and where a family is open.
#[derive(Clone)]
pub struct Ask {
    pub filter: FaceFilter,
    pub open_family: bool,
}

/// Something that can answer an [`Ask`]. The index in production, a stub in the tests.
pub trait Worker: Send + 'static {
    fn answer(&mut self, ask: &Ask) -> fontina_core::Result<Listing>;
}

/// The index, on the other end of a channel.
struct IndexWorker(Index);

impl Worker for IndexWorker {
    fn answer(&mut self, ask: &Ask) -> fontina_core::Result<Listing> {
        IndexWorker::answer_with(&self.0, ask)
    }
}

/// Answer an [`Ask`] on the caller's own thread.
///
/// For an index with no file behind it, which cannot be reopened and so has no worker.
/// Nothing in the browser is written twice for the two cases: the questions are the
/// same and only the waiting differs.
pub fn answer_here(index: &Index, ask: &Ask) -> fontina_core::Result<Listing> {
    IndexWorker::answer_with(index, ask)
}

impl IndexWorker {
    fn answer_with(index: &Index, ask: &Ask) -> fontina_core::Result<Listing> {
        // Facets are counted over the whole filter except the family, so that opening a
        // family does not collapse every count to that family's own.
        let facets = index.facets(&FaceFilter {
            family: None,
            ..ask.filter.clone()
        })?;
        let (families, faces) = if ask.open_family {
            (Vec::new(), index.list(&ask.filter)?)
        } else {
            (index.families(&ask.filter)?, Vec::new())
        };
        Ok(Listing {
            facets,
            families,
            faces,
        })
    }
}

struct Request {
    generation: u64,
    ask: Ask,
}

struct Reply {
    generation: u64,
    listing: fontina_core::Result<Listing>,
}

/// The browser's end of the conversation.
pub struct Search {
    tx: Sender<Request>,
    rx: Receiver<Reply>,
    /// Stops the query the worker is inside right now.
    interrupt: Option<Interrupt>,
    /// The newest generation asked for.
    sent: u64,
    /// The newest generation whose answer has been put on the screen.
    applied: u64,
}

impl Search {
    /// A worker with its own connection to the same index.
    ///
    /// Its own, rather than a share of the browser's: SQLite in WAL mode reads
    /// concurrently across connections, and a shared one would need a lock held for the
    /// length of a query — which is the thing being moved off the drawing thread in the
    /// first place.
    ///
    /// An index with no file behind it cannot be reopened, so there is nothing to
    /// spawn; the caller answers its own questions in that case.
    ///
    /// The check is for a file that is already there, not merely for a name that is
    /// not `:memory:`. SQLite reports an in-memory connection as the empty path, and
    /// `Connection::open("")` does not fail on it — it quietly makes a fresh temporary
    /// database. A worker answering out of that would return no families and no faces
    /// for every filter, which on the screen is indistinguishable from an index that
    /// has lost every font in it.
    pub fn open(index: &Index) -> Option<Search> {
        let path = index.path();
        if path.is_empty() || path == ":memory:" || !std::path::Path::new(&path).is_file() {
            return None;
        }
        let worker = Index::open(std::path::Path::new(&path)).ok()?;
        let interrupt = worker.interrupt_handle();
        Some(Search::with_worker(IndexWorker(worker), Some(interrupt)))
    }

    /// The same conversation, over any worker. The tests use this.
    pub fn with_worker<W: Worker>(mut worker: W, interrupt: Option<Interrupt>) -> Search {
        let (tx, requests) = channel::<Request>();
        let (replies, rx) = channel::<Reply>();
        thread::spawn(move || {
            // A request taken off the channel and not yet answered.
            //
            // Everything the worker takes out has to be answered or superseded by an
            // answer to something newer, because `settle` waits for a generation and
            // has nothing else to wait on. This is where the one request that used to
            // be dropped on the floor is kept instead.
            let mut held: Option<Request> = None;
            loop {
                let mut request = match held.take() {
                    Some(request) => request,
                    None => match requests.recv() {
                        Ok(request) => request,
                        Err(_) => return,
                    },
                };
                // Coalesce: whatever else is already waiting supersedes this one, and
                // running a superseded query is work whose answer is known to be
                // unwanted before it starts. Superseding is safe where dropping is
                // not — the reply carries the newer generation, which is what the
                // waiter was going to accept anyway.
                while let Ok(newer) = requests.try_recv() {
                    request = newer;
                }
                let mut listing = worker.answer(&request.ask);
                // An interrupt is aimed at whatever was running when it was fired, and
                // between coalescing and firing there is a window where it can land on
                // the request that superseded the one it was meant for. So: if this
                // failed and nothing newer is waiting, it was either that race or a
                // real fault, and one more attempt tells them apart. A real fault fails
                // again and is reported; the race costs one query nobody notices.
                //
                // And if something newer *is* waiting, it is held rather than looked at
                // and forgotten. `try_recv` takes the request out of the channel: the
                // old code read it only to decide whether to retry, dropped it, and
                // replied with the older generation — so a `settle` waiting on the
                // newer one waited for an answer that no longer existed anywhere, with
                // the worker parked on an empty channel. That is a browser frozen on a
                // keystroke, and it took a filter change landing in the window where a
                // search was still running to do it.
                if listing.is_err() {
                    match requests.try_recv() {
                        Ok(newer) => held = Some(newer),
                        Err(TryRecvError::Empty) => listing = worker.answer(&request.ask),
                        Err(TryRecvError::Disconnected) => {}
                    }
                }
                if replies
                    .send(Reply {
                        generation: request.generation,
                        listing,
                    })
                    .is_err()
                {
                    return;
                }
            }
        });
        Search {
            tx,
            rx,
            interrupt,
            sent: 0,
            applied: 0,
        }
    }

    /// Ask for a listing, and stop caring about every answer still owed.
    ///
    /// Returns the generation, which is what a caller waiting for this particular
    /// answer holds on to.
    pub fn ask(&mut self, ask: Ask) -> u64 {
        self.sent += 1;
        // Before sending, not after: the worker may be inside a query whose answer is
        // now known to be stale, and the sooner it stops the sooner it starts on this.
        if let Some(i) = &self.interrupt {
            i.cancel();
        }
        // A worker that has gone is a browser that will simply stop updating rather
        // than one that panics at a reader. `settle` is where that becomes visible.
        let _ = self.tx.send(Request {
            generation: self.sent,
            ask,
        });
        self.sent
    }

    /// The newest answer that has arrived and is still wanted, if any.
    ///
    /// Everything staler is dropped here rather than at the sender, because an answer
    /// can overtake a newer one on the way back: two queries in flight, the second
    /// quick and the first slow, and the first arrives last carrying an older list.
    pub fn take(&mut self) -> Option<fontina_core::Result<Listing>> {
        let mut newest = None;
        while let Ok(reply) = self.rx.try_recv() {
            if reply.generation <= self.applied || reply.generation > self.sent {
                continue;
            }
            // An error on a superseded generation is an interrupted query saying so,
            // which is not something to tell anybody about.
            if reply.listing.is_err() && reply.generation < self.sent {
                self.applied = self.applied.max(reply.generation);
                continue;
            }
            self.applied = reply.generation;
            newest = Some(reply.listing);
        }
        newest
    }

    /// Wait for the answer to `generation`, discarding everything staler.
    ///
    /// For the places that cannot carry on without the answer: the first frame, and the
    /// reload after a tag or an activation, where the next thing the reader does is
    /// look at what changed. Typing does not use this, which is the whole point.
    pub fn settle(&mut self, generation: u64) -> Option<fontina_core::Result<Listing>> {
        while self.applied < generation {
            let Ok(reply) = self.rx.recv() else {
                // The worker is gone. Nothing will ever arrive, so say so once rather
                // than block a browser forever.
                return None;
            };
            if reply.generation < generation {
                continue;
            }
            self.applied = reply.generation;
            return Some(reply.listing);
        }
        None
    }

    /// How many have been asked for and how many answered, for a test that wants to
    /// say which of the two went wrong rather than only that something did.
    #[cfg(test)]
    pub fn progress(&self) -> (u64, u64) {
        (self.sent, self.applied)
    }

    /// Whether an answer is still owed. The event loop polls faster while it is.
    pub fn waiting(&self) -> bool {
        self.applied < self.sent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn ask() -> Ask {
        Ask {
            filter: FaceFilter::default(),
            open_family: false,
        }
    }

    fn empty() -> Listing {
        Listing {
            facets: Facets::default(),
            families: Vec::new(),
            faces: Vec::new(),
        }
    }

    /// A worker that takes `delay` to answer and counts how often it was asked.
    struct Slow {
        delay: Duration,
        answered: Arc<AtomicUsize>,
    }

    impl Worker for Slow {
        fn answer(&mut self, _: &Ask) -> fontina_core::Result<Listing> {
            thread::sleep(self.delay);
            self.answered.fetch_add(1, Ordering::SeqCst);
            Ok(empty())
        }
    }

    /// A worker that reports how many families it was asked for, so a reply can be
    /// told apart from every other reply.
    struct Counting(usize);

    impl Worker for Counting {
        fn answer(&mut self, _: &Ask) -> fontina_core::Result<Listing> {
            self.0 += 1;
            Ok(Listing {
                families: Vec::with_capacity(self.0),
                ..empty()
            })
        }
    }

    /// A worker that says when it has started an answer and waits to be told how it
    /// ends, so a test can stand exactly in the window where the browser interrupts a
    /// query that is already running.
    struct Gated {
        started: Sender<()>,
        outcome: Receiver<bool>,
    }

    impl Worker for Gated {
        fn answer(&mut self, _: &Ask) -> fontina_core::Result<Listing> {
            let _ = self.started.send(());
            match self.outcome.recv() {
                Ok(true) => Ok(empty()),
                _ => Err(fontina_core::Error::Other("interrupted".into())),
            }
        }
    }

    /// Every generation the worker takes off the channel gets an answer, or is
    /// superseded by an answer to a newer one. Nothing is taken out and dropped.
    ///
    /// The hang this pins: the browser asks while a query is running (typing does this
    /// on every keystroke), the interrupt lands on that running query and it comes back
    /// an error, and the worker looked at the newer request only to decide whether to
    /// retry — taking it out of the channel and throwing it away. It then replied with
    /// the *older* generation. `settle` was waiting for the newer one, which no longer
    /// existed anywhere, and the worker was parked on an empty channel. The browser
    /// froze on the keystroke, with no error and nothing to see.
    #[test]
    fn a_request_the_worker_takes_out_is_never_dropped() {
        let (started_tx, started) = channel::<()>();
        let (outcome, outcome_rx) = channel::<bool>();
        let mut search = Search::with_worker(
            Gated {
                started: started_tx,
                outcome: outcome_rx,
            },
            None,
        );

        // One query, running. The worker is inside `answer` and cannot see anything
        // else on its channel yet.
        let first = search.ask(ask());
        started.recv_timeout(Duration::from_secs(5)).unwrap();

        // A second, asked while the first is still running: the keystroke that used to
        // freeze the browser.
        let second = search.ask(ask());
        assert!(second > first);

        // The first ends as an interrupt does, in an error.
        outcome.send(false).unwrap();
        // The second is asked for real and answers.
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        outcome.send(true).unwrap();

        // Settling has to be able to finish, so it is done where a deadlock is a failed
        // assertion rather than a test runner that never comes back.
        let (done, waited) = channel();
        thread::spawn(move || {
            let settled = search.settle(second);
            let _ = done.send(settled.is_some());
        });
        assert_eq!(
            waited.recv_timeout(Duration::from_secs(10)),
            Ok(true),
            "the answer to generation {second} never arrived: the worker took the \
             request off its channel and dropped it"
        );
    }

    /// An index nothing can reopen gets no worker, and the browser answers its own
    /// questions instead. The case that made this a test rather than an `if`: SQLite
    /// calls an in-memory connection the empty path, and opening that succeeds, on a
    /// brand new empty database.
    #[test]
    fn an_index_with_no_file_behind_it_gets_no_worker() {
        let index = Index::open_in_memory().unwrap();
        assert!(
            Search::open(&index).is_none(),
            "a worker was spawned against {:?}, which is not the index",
            index.path()
        );
    }

    /// The thing the item is about: typing does not wait.
    #[test]
    fn asking_returns_immediately_however_slow_the_worker_is() {
        let mut search = Search::with_worker(
            Slow {
                delay: Duration::from_millis(80),
                answered: Arc::new(AtomicUsize::new(0)),
            },
            None,
        );
        let start = std::time::Instant::now();
        for _ in 0..10 {
            search.ask(ask());
        }
        assert!(
            start.elapsed() < Duration::from_millis(80),
            "ten keystrokes waited on the worker: {:?}",
            start.elapsed()
        );
        assert!(search.waiting(), "and an answer is owed");
    }

    /// Queued requests are thrown away rather than run: ten keystrokes are not ten
    /// queries, they are the one in flight and the last one typed.
    #[test]
    fn a_burst_of_keystrokes_costs_the_worker_at_most_two_queries() {
        let answered = Arc::new(AtomicUsize::new(0));
        let mut search = Search::with_worker(
            Slow {
                delay: Duration::from_millis(40),
                answered: Arc::clone(&answered),
            },
            None,
        );
        // Collected, not lazily mapped: a `Map` that is only asked for its last item
        // only calls the closure once, which is nine keystrokes that never happened.
        let sent: Vec<u64> = (0..10).map(|_| search.ask(ask())).collect();
        assert_eq!(
            sent.last(),
            Some(&10),
            "every keystroke is its own generation"
        );
        let answer = search.settle(10).expect("the newest one is answered");
        assert!(answer.is_ok());
        assert!(
            answered.load(Ordering::SeqCst) <= 2,
            "the worker ran {} of the ten queries",
            answered.load(Ordering::SeqCst)
        );
    }

    /// The answer that reaches the screen is the answer to the last thing typed, and
    /// nothing older can overwrite it afterwards.
    #[test]
    fn only_the_newest_answer_is_taken_and_the_rest_are_dropped() {
        let mut search = Search::with_worker(Counting(0), None);
        let sent: Vec<u64> = (0..5).map(|_| search.ask(ask())).collect();
        let last = *sent.last().unwrap();
        search.settle(last);

        // Everything the worker sent while catching up has been consumed or dropped,
        // and the generation applied is the last one asked for.
        assert_eq!(search.applied, last);
        assert!(!search.waiting(), "nothing is owed once the newest arrives");
        assert!(
            search.take().is_none(),
            "an older answer was still waiting to overwrite a newer one"
        );
    }

    /// A reply that arrives after a newer one has already been shown is dropped, which
    /// is the case a generation number exists for.
    #[test]
    fn an_answer_that_overtakes_a_newer_one_is_discarded() {
        let (tx, requests) = channel::<Request>();
        let (replies, rx) = channel::<Reply>();
        drop(requests);
        let mut search = Search {
            tx,
            rx,
            interrupt: None,
            sent: 3,
            applied: 0,
        };

        // Out of order, as two queries in flight can be: the newest first.
        replies
            .send(Reply {
                generation: 3,
                listing: Ok(Listing {
                    families: Vec::with_capacity(3),
                    ..empty()
                }),
            })
            .unwrap();
        replies
            .send(Reply {
                generation: 2,
                listing: Ok(Listing {
                    families: Vec::with_capacity(2),
                    ..empty()
                }),
            })
            .unwrap();

        let taken = search.take().expect("something arrived").unwrap();
        assert_eq!(
            taken.families.capacity(),
            3,
            "the older answer overwrote the newer one"
        );
        assert_eq!(search.applied, 3);
        assert!(search.take().is_none(), "and nothing is left to apply");
    }

    /// An interrupted query fails, and a failure nobody asked about is not a message to
    /// put in front of a reader.
    #[test]
    fn a_failure_on_a_superseded_generation_is_silence() {
        let (tx, requests) = channel::<Request>();
        let (replies, rx) = channel::<Reply>();
        drop(requests);
        let mut search = Search {
            tx,
            rx,
            interrupt: None,
            sent: 2,
            applied: 0,
        };
        replies
            .send(Reply {
                generation: 1,
                listing: Err(fontina_core::Error::Other("interrupted".into())),
            })
            .unwrap();
        assert!(
            search.take().is_none(),
            "a cancelled query complained to the reader"
        );

        // The newest one failing is a different matter: that one is worth saying.
        replies
            .send(Reply {
                generation: 2,
                listing: Err(fontina_core::Error::Other("the index is gone".into())),
            })
            .unwrap();
        assert!(
            search.take().is_some_and(|r| r.is_err()),
            "a failure the reader is waiting on was swallowed"
        );
    }

    /// A worker that has died leaves a browser that stops updating, not one that hangs.
    #[test]
    fn a_worker_that_is_gone_does_not_block_the_browser() {
        struct Dying;
        impl Worker for Dying {
            fn answer(&mut self, _: &Ask) -> fontina_core::Result<Listing> {
                panic!("the worker fell over")
            }
        }
        let mut search = Search::with_worker(Dying, None);
        let g = search.ask(ask());
        assert!(search.settle(g).is_none(), "settle waited forever");
    }
}
