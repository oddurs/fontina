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

| `woff1-origlength-allocation.woff` | WOFF 1.0 table entry with `compLength = 0` and `origLength = 0xFFFFFFFF`. The decoder passed that straight to `Vec::with_capacity`, so 64 bytes of input asked for 4 GB of memory before reading a single compressed byte. Not a panic and not a hang, so `catch_unwind` never saw it. Fixed in #51: an implausible length is refused, the buffer grows with the bytes that arrive, and the decompressed length must match what was declared. 64 bytes. |
| `gpos-scriptlist-quadratic.ttf.gz` | `GPOS` declaring 12194 script records. `features` looked up each script with a linear `find` and each language with `contains`, so importing one 241 KB file took 37 seconds with the shipped binary, and minutes under a sanitizer. Both counts are `u16` off the wire. Fixed in #51: an ordered map instead of two linear scans, plus a cap on how many records are read, because records may all point at one language list and so the work is not bounded by the file's size. 65 KB raw, 1.8 KB gzipped. |

## `open/` — found, not yet fixed

`open/` holds inputs for defects the fuzzer has found that are still open. They are
neither replayed by the regression test nor seeded into the corpus, for one reason each:
an unfixed input would fail the required CI test job on every pull request, and libFuzzer
loads its whole corpus before it fuzzes anything, so a known-bad seed aborts the run at
startup and nothing gets fuzzed at all. The test prints them so they cannot be forgotten.

It is empty. Both of its inputs were fixed in #51 and moved up into the table above.
