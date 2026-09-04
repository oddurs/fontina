---
title: JSON output and schemas
description: "machine-readable output, the three JSON Schemas, and stability guarantees."
order: 4
---

Every reporting command takes `--json` and then prints exactly one JSON document to
standard output. Human-readable output is not stable and not meant to be parsed;
JSON output is.

## The three schemas

The schemas are JSON Schema draft 2020-12, generated from the Rust types and checked
in to `schemas/`. Continuous integration fails if they drift from the code.

<dl>
<dt><code>face.json</code></dt>
<dd>The metadata of one face: the document that <code>info --json</code> prints and
that the index stores. Its top-level type is <code>FaceMetadata</code> and it carries
a <code>schema_version</code>.</dd>
<dt><code>collection.json</code></dt>
<dd>The file that <code>collection export</code> writes and <code>collection import</code>
reads: a name, an ordered list of faces identified by identity hash, PostScript name
and path, so that it survives a move to another machine.</dd>
<dt><code>cli-output.json</code></dt>
<dd>One definition per command's <code>--json</code> output type, so a consumer can
validate <code>list</code>, <code>facets</code>, <code>check</code>, <code>dupes</code>,
<code>stats</code> and the rest.</dd>
</dl>

Print any of them from the binary you have, which is always the right version:

```
unifont schema face
unifont schema collection
unifont schema cli-output
```

## Stability

- Fields are added, not removed or renamed, within a schema version.
- A change that is not backwards compatible bumps `schema_version` in `face.json`
  and is called out in the changelog.
- Health check identifiers (`area/check`) are never renamed. New ones may appear.
- Face ids are stable for the life of a file in the index. They are not stable
  across indexes; use the identity hash or the PostScript name for that.

## Examples

Ids of every variable font, for a shell loop:

```
unifont list --variable --json | jq -r '.[].id'
```

Families that cover Devanagari, with the number of faces in each:

```
unifont families --script Deva --json | jq -r '.[] | "\(.family)\t\(.faces | length)"'
```

Fail a script if any font forbids embedding:

```
unifont license --json | jq -e 'all(.[]; .embedding.level != "restricted_license")'
```

Validate a collection file before importing it:

```
unifont schema collection > collection.schema.json
check-jsonschema --schemafile collection.schema.json editorial.json
```

## The stored metadata

The index stores the complete `FaceMetadata` document for each face, as JSON, beside
the columns it indexes for filtering. `info --json` returns that document unchanged.
Anything a future filter needs is therefore already on disk; a migration backfills
the column from the JSON rather than rescanning.
