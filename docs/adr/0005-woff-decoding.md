# 0005 — WOFF and WOFF2 decoding

**Status:** accepted, 2026-09-03

## Context
fontations reads sfnt data only. WOFF 1.0 is per-table zlib; WOFF 2.0 is Brotli with
glyf/loca transforms.

## Decision
WOFF 1.0: a small hand-written unwrapper in `unifont-core::container` (flate2).
WOFF 2.0: the pure-Rust `woff2-patched` crate. Both produce sfnt bytes that then go
through the normal fontations path.

## Consequences
`woff2-patched` does not support the `hmtx` transform; such files fail with a clear error
and are counted in scan failures. If that becomes common, an FFI fallback to google/woff2
behind a feature flag is the planned mitigation.
