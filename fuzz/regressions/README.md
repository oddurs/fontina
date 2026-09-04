# Fuzzing regressions

One file per input that once broke the import path. `crates/fontina-core/tests/fuzz_regressions.rs`
replays every one of them on **stable** Rust, through both `fontina_core::load_bytes` and
`fontina_core::parse::parse_sfnt`, and fails if any of them panics or takes longer than a
few seconds. The fuzzers themselves need a nightly toolchain and so do not run in the
normal CI matrix; this directory is how their findings stay fixed anyway. `scripts/fuzz
seed` also copies these inputs into the corpora, so a past finding keeps steering the
mutator instead of having to be rediscovered.

When `scripts/fuzz` reports a crash it writes the input to `fuzz/artifacts/<target>/`.
Minimise it (`cargo fuzz tmin <target> <file>`, or by hand — the bug is usually one field),
put it here under a name that says what it does, add a row to the table, and fix it.

Inputs are kept small. One is not: the WOFF table-count overflow needs a 32768-entry
directory before the arithmetic is reachable at all, which is 640 KB of mostly zeroes, so
it is stored gzipped. Any file here ending in `.gz` is decompressed before it is replayed.

| Input | What it broke |
|---|---|
| `woff1-zero-tables.woff` | WOFF 1.0 header with `numTables == 0`. `rangeShift` is defined as `numTables * 16 - searchRange`, so the reconstruction computed `0 * 16 - 16`: a subtraction overflow panic in debug, a wrapped 65520 in release. Fixed in #33; a zero-table directory is now an ordinary parse error. 44 bytes. |
| `woff1-table-count-overflow.woff.gz` | WOFF 1.0 header with `numTables == 32768`. `entrySelector` was found by shifting a `u16` left until it passed `numTables`, which at 32768 shifts by 16: a panic in debug, and in release a masked shift amount that keeps the loop condition true **forever**, wedging a scan worker for the life of the process. No `catch_unwind` can recover from that, which is the reason this directory exists. Fixed in #33. 640 KB raw, 681 bytes gzipped. |

## `open/` — found, not yet fixed

`open/` holds inputs for defects the fuzzer has found that are still open. They are
neither replayed by the regression test nor seeded into the corpus, for one reason each:
an unfixed input would fail the required CI test job on every pull request, and libFuzzer
loads its whole corpus before it fuzzes anything, so a known-bad seed aborts the run at
startup and nothing gets fuzzed at all. The test prints them so they cannot be forgotten.

When one is fixed, move it up a directory and add a row to the table above.

| Input | What it breaks |
|---|---|
| `woff1-origlength-allocation.woff` | A 64-byte WOFF 1.0 file whose one directory entry declares `compLength = 0` and `origLength = 0xFFFFFFFF`. `container::decode_woff1` reserves the declared length with `Vec::with_capacity(orig_len)` before decompressing a single byte, so 64 bytes of input ask for 4 GiB. Found by the `parse` target in 58 executions; `-rss_limit_mb` is what turns it into a report, since it is neither a panic nor a hang and `catch_unwind` never sees it. |
| `gpos-scriptlist-quadratic.ttf.gz` | A 65 KB font with a GPOS `ScriptList` of 10000 records that all carry the tag `latn` and all point at one `Script` table with 800 `LangSysRecord`s. `parse::features` merges scripts by scanning the accumulated `Vec` (`scripts.iter_mut().find(...)`, then `existing.languages.contains(&l)` per language), which is O(*scripts* x *languages*²) with a `String` allocated per record. 3.8 s in a plain release build, 12 s under the sanitizer, against roughly 10 ms for a real font of that size; both counts are `u16`, so a larger table gets to minutes. Found by the `sfnt` target, which turned up a 241 KB mutation of `SourceSerif4-Regular.otf`: 262 s under the sanitizer, and 37.65 s for `fontina scan` of that one file with the shipped release binary. This is the hand-minimised equivalent, because the artifact itself does not compress. The `parse` target finds it too, from an ordinary TTF: it is on the everyday import path, not only behind the `sfnt` shortcut. |
