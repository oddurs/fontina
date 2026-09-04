# 0002 — SQLite for the index

**Status:** accepted, 2026-09-03

## Context
The index must survive restarts, answer facet and full-text queries over tens of
thousands of faces in milliseconds, and be a single portable file.

## Decision
SQLite via `rusqlite` (bundled), WAL journal, FTS5 for names. The full `FaceMetadata`
JSON is stored per face beside indexed columns for the common filters. Migrations are
append-only SQL keyed on `PRAGMA user_version`.

## Consequences
Zero administration, one file to back up or delete. Schema changes need a migration.
Ad-hoc queries are possible with any SQLite client.
