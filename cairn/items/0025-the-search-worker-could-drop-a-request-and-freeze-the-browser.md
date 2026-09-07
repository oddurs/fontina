---
id: 25
title: The search worker could drop a request and freeze the browser
type: fix
status: done
milestone: tui-speed
created: 2026-09-06
updated: 2026-09-06
priority: p0
effort: s
crate: ui
---

## Problem

The browser could freeze on a keystroke, with no error and nothing on the screen to say
why.

The search worker coalesces: it takes a request, drains anything newer off its channel,
and answers only the newest, replying with that generation. That is safe — a waiter
holding an older generation accepts a newer answer. What was not safe was the retry:

    if listing.is_err() && matches!(requests.try_recv(), Err(TryRecvError::Empty)) {
        listing = worker.answer(&request.ask);
    }

`try_recv` *takes the request out of the channel*. It was read only to decide whether
to retry, and then dropped. The worker then replied with the older generation and went
back to waiting on an empty channel. A `reload` sitting in `settle` on the newer
generation waited for an answer that no longer existed anywhere.

Reaching it takes three things at once, which is why it survived a year: a request in
flight, a second asked while the first is still running (typing does this on every
keystroke), and the interrupt that the second one fires landing on the first, so its
answer comes back an error. A soak test pressing arbitrary keys found it.

## Proposal

Hold the request instead of dropping it. The worker keeps a `held: Option<Request>` and
takes from it before reading the channel, so everything it takes out is answered or
superseded by an answer to something newer.

## Acceptance criteria

- [x] a test stands in the window — a query running, a second asked, the first ending in
      an error — and asserts the second is answered
- [x] the test hangs on the old worker and passes on the new one
- [x] `settle` is never reachable with the worker parked on an empty channel
