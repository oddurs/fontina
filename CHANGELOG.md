# Changelog

## [0.1.1](https://github.com/oddurs/fontina/compare/v0.1.0...v0.1.1) (2026-09-04)


### Features

* **cli:** a glyph map in the browser, and stop corrupting the terminal with it ([#48](https://github.com/oddurs/fontina/issues/48)) ([0c21e49](https://github.com/oddurs/fontina/commit/0c21e49f1f8ce5a02e0c183503d1f65d4a3c8f47))
* **cli:** move the axes and toggle the features from inside the browser ([#44](https://github.com/oddurs/fontina/issues/44)) ([1fce658](https://github.com/oddurs/fontina/commit/1fce658cb6f4348b42e9688859fc0147bfb4a3c9))
* **cli:** waterfall and compare, as one sheet rendered once ([#49](https://github.com/oddurs/fontina/issues/49)) ([9b716ab](https://github.com/oddurs/fontina/commit/9b716ab532473fe7acecbeeda36a753454c6229f))


### Bug Fixes

* **cli:** teach the test activator that deactivate now reports a bool ([#39](https://github.com/oddurs/fontina/issues/39)) ([1773689](https://github.com/oddurs/fontina/commit/1773689de2428a0ff8aa0e328032f9d0e9741561))
* **core:** bound what a hostile font can make the importer allocate and do ([#52](https://github.com/oddurs/fontina/issues/52)) ([5beaf8d](https://github.com/oddurs/fontina/commit/5beaf8d5e7caffc32d973c5625ffb0f0faaf1a95))
* **core:** let concurrent fontina processes share one index ([#47](https://github.com/oddurs/fontina/issues/47)) ([67be3c0](https://github.com/oddurs/fontina/commit/67be3c0824c186e754d07b5470e5fbdfd9f59ea4))
* **core:** never let a scan destroy the user's curation ([#34](https://github.com/oddurs/fontina/issues/34)) ([9c08fdf](https://github.com/oddurs/fontina/commit/9c08fdf3773e6d0de9681ddd38d9c17cedf793fb))
* **core:** stop three panics and a hang on hostile font input ([#33](https://github.com/oddurs/fontina/issues/33)) ([8916e09](https://github.com/oddurs/fontina/commit/8916e09d8aee0926beab82dee481564f7a7ddd38))
* **platform:** make activation reversible, and never touch a font we did not create ([#37](https://github.com/oddurs/fontina/issues/37)) ([fb4978f](https://github.com/oddurs/fontina/commit/fb4978fe8413089415a096411e033186da0e2ce8))
* **release:** ship static GNU/Linux binaries, so they run on every distribution ([#45](https://github.com/oddurs/fontina/issues/45)) ([74749e6](https://github.com/oddurs/fontina/commit/74749e60a1078fe1515623fdf2d1d0a7a7180278))
* vanished directories, per-frame detail queries and double-counted restores ([#36](https://github.com/oddurs/fontina/issues/36)) ([a6d1b81](https://github.com/oddurs/fontina/commit/a6d1b817bc4dbf8e9f3e79a5371d361664ac9366))


### Performance Improvements

* measure the budgets instead of claiming them ([#43](https://github.com/oddurs/fontina/issues/43)) ([d23008b](https://github.com/oddurs/fontina/commit/d23008b5b5dd9a50f30bd41262af00c07d9a7dc8))
* measure the list budget at the scale the budget states ([#53](https://github.com/oddurs/fontina/issues/53)) ([9f62e34](https://github.com/oddurs/fontina/commit/9f62e3471374b8eadd42b925614b8a2a63b9e31e))

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
