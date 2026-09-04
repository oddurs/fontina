# Security policy

fontina parses untrusted font files. Parser bugs that lead to crashes, memory
unsafety, or out-of-bounds reads are security bugs and we treat them as such.

## Supported versions

Only the latest release and `main` receive fixes.

## Reporting a vulnerability

Please do not open a public issue. Use GitHub's private vulnerability reporting:
**Security → Report a vulnerability** on the repository page, or email
oddurs@gmail.com with `[fontina security]` in the subject.

Include the font file that triggers the problem if you can share it, or the
`fontina info --json` output and a description of how it was produced.

You will get an acknowledgement within 7 days and a fix or a mitigation plan
within 90 days. We credit reporters in the release notes unless asked not to.

## Scope

- `fontina-core` parsing, decoding (WOFF/WOFF2) and the SQLite index
- the `fontina` CLI
- release artifacts and the build pipeline

Out of scope: vulnerabilities in the fonts themselves, or in operating-system
font rasterisers that fontina does not ship.

## Hardening in place

- All OpenType parsing goes through [fontations](https://github.com/googlefonts/fontations),
  which is fuzzed continuously in OSS-Fuzz.
- Parsing runs inside a panic boundary; a malformed file is reported, never fatal.
- No network access, no telemetry, no elevation, no writes to system font directories.
- Releases carry SLSA build provenance attestations and an SPDX SBOM.
