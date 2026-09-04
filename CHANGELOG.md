# Changelog

## [0.1.0](https://github.com/oddurs/unifont/compare/v0.0.1...v0.1.0) (2026-09-04)


### ⚠ BREAKING CHANGES

* the crates are fontina-core, fontina-platform and fontina-cli, the binary is `fontina`, the environment variable is FONTINA_DB, and the license is GPL-3.0-or-later rather than MIT OR Apache-2.0.

### Features

* **cli:** `unifont ui`, a ratatui browser over the index ([#22](https://github.com/oddurs/unifont/issues/22)) ([ae4ad69](https://github.com/oddurs/unifont/commit/ae4ad69e3c1a25a861c0009607a24c705a9c6cb9))
* **core:** shaped, rasterised previews and `unifont preview` ([#21](https://github.com/oddurs/unifont/issues/21)) ([dcedf67](https://github.com/oddurs/unifont/commit/dcedf67772a74d34c199ac237c082f7c92b88fde))
* **core:** tags, collections, sources, activation state, facets and families ([#14](https://github.com/oddurs/unifont/issues/14)) ([cc38f6c](https://github.com/oddurs/unifont/commit/cc38f6ce0b9e2e5b29ecbfd681dbbd6107545703))
* **core:** watched folders and `unifont watch` ([#19](https://github.com/oddurs/unifont/issues/19)) ([bf383f5](https://github.com/oddurs/unifont/commit/bf383f562c3409cf7db5364d35a96b7d00c23c8f))
* M0 core parser, SQLite index and CLI ([#1](https://github.com/oddurs/unifont/issues/1)) ([2840788](https://github.com/oddurs/unifont/commit/2840788beddcff5cb049a2ecf546479bef5c30b9))
* M2 typography tools in core and CLI ([#4](https://github.com/oddurs/unifont/issues/4)) ([3d190ba](https://github.com/oddurs/unifont/commit/3d190bae86f6eb9126464f9db62b02cc9787e077))
* **platform:** native activation on Linux, macOS and Windows ([#16](https://github.com/oddurs/unifont/issues/16)) ([bf5f76b](https://github.com/oddurs/unifont/commit/bf5f76baf00dbe960950fba61a7c141399cf57da))
* relicense under GPLv3, rename to fontina, report each font's freedom ([#25](https://github.com/oddurs/unifont/issues/25)) ([496a6d1](https://github.com/oddurs/unifont/commit/496a6d1424185d0bee0a1aa388d2a3e9b3567cc7))


### Bug Fixes

* remove site/ that [#14](https://github.com/oddurs/unifont/issues/14) committed by accident ([#15](https://github.com/oddurs/unifont/issues/15)) ([ca064d3](https://github.com/oddurs/unifont/commit/ca064d34ae976cbfd1d9f409f69c1bfaee3e8752))
* **site:** keep .well-known in the Pages artifact ([#20](https://github.com/oddurs/unifont/issues/20)) ([07e89ab](https://github.com/oddurs/unifont/commit/07e89ab2988030bbf4860c3716eb4dad93884e58))
