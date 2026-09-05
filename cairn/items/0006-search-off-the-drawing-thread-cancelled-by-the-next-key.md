---
id: 6
title: Search off the drawing thread, cancelled by the next key
type: perf
status: backlog
milestone: tui-speed
created: 2026-09-05
updated: 2026-09-05
priority: p0
effort: m
crate: ui
---

## What is slow

Search runs the query against the index on the thread that draws. A query that
takes 80ms is 80ms in which the browser does not respond, and typing a six-letter
family name runs six of them, five of whose results nobody will ever see.

## Budget

No keystroke blocks a frame. The browser stays responsive while a query runs, and
a superseded query stops rather than finishing into a result nobody wants.

## Acceptance criteria

- [ ] queries run on a worker; the draw thread never waits on SQLite
- [ ] a new keystroke cancels the query in flight
- [ ] results arriving out of order are discarded rather than shown
- [ ] typing a ten-character query at 10,000 faces never drops a frame
