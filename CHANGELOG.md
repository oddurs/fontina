# Changelog

## [0.1.2](https://github.com/oddurs/fontina/compare/v0.1.1...v0.1.2) (2026-09-13)


### Features

* adopt cairn as the tracker, and plan the browser as four sprints ([#126](https://github.com/oddurs/fontina/issues/126)) ([38b96ad](https://github.com/oddurs/fontina/commit/38b96ad9b5b4bd497938bf7a55321335deb12d18))
* **cli:** --lang, a language facet, and the two claims kept apart in info ([#102](https://github.com/oddurs/fontina/issues/102)) ([178057d](https://github.com/oddurs/fontina/commit/178057d4327f81183f67722ab2dbf280b1230bd1))
* **cli:** --mono and --proportional, and a spacing facet ([#103](https://github.com/oddurs/fontina/issues/103)) ([c7411ea](https://github.com/oddurs/fontina/commit/c7411ea0352217e3b32184fb7cf8e27383de3f6e))
* **cli:** --script repeats, --script-min asks how much, and the facet leads with depth ([#100](https://github.com/oddurs/fontina/issues/100)) ([d366f1e](https://github.com/oddurs/fontina/commit/d366f1efd637bd5b38f0933edbe9c2b1e26cc820))
* **cli:** a configuration file that can only change what you leave out ([#93](https://github.com/oddurs/fontina/issues/93)) ([c1a2ae7](https://github.com/oddurs/fontina/commit/c1a2ae79cfef68943819f7cceefe82836273609c))
* **cli:** a specimen says how many licensed fonts it is carrying ([#139](https://github.com/oddurs/fontina/issues/139)) ([f147de7](https://github.com/oddurs/fontina/commit/f147de73f35326ddf933652a736bb852f8134ec3))
* **cli:** accept the address a listing prints, and test a collection at last ([#132](https://github.com/oddurs/fontina/issues/132)) ([c9f7f8d](https://github.com/oddurs/fontina/commit/c9f7f8d92e1a3fb286536d7c8e6c7a943b3d4e9c))
* **cli:** collection export --bundle, a collection you can hand over ([#85](https://github.com/oddurs/fontina/issues/85)) ([bd35bb2](https://github.com/oddurs/fontina/commit/bd35bb285bd4a307302406823b5ca10f9151c942))
* **cli:** fontina tag sync, in one direction at a time ([#89](https://github.com/oddurs/fontina/issues/89)) ([e0f7c94](https://github.com/oddurs/fontina/commit/e0f7c943d5164c889bfb6283cece29d43896ea33))
* **cli:** fontina variants, the score and the metrics side by side ([#105](https://github.com/oddurs/fontina/issues/105)) ([af1dd4d](https://github.com/oddurs/fontina/commit/af1dd4d9541ed89e5c7ccdc1a6e55fb8b3bd3273))
* **cli:** read targets from standard input, so programs can pipe in ([#91](https://github.com/oddurs/fontina/issues/91)) ([8d806d0](https://github.com/oddurs/fontina/commit/8d806d0816f12d4e1d479075affae0715bc91357))
* **cli:** s writes a specimen for the selection and opens it ([#90](https://github.com/oddurs/fontina/issues/90)) ([002e039](https://github.com/oddurs/fontina/commit/002e039aa9e5a930d50abca3ba24b5d26e9be210))
* **cli:** say how many builds hide under one PostScript name ([#188](https://github.com/oddurs/fontina/issues/188)) ([df4da37](https://github.com/oddurs/fontina/commit/df4da371781f768ba20e0054e89b22d2a8dca7d4))
* **cli:** say the licence and the warranty disclaimer in --version ([#63](https://github.com/oddurs/fontina/issues/63)) ([5abc416](https://github.com/oddurs/fontina/commit/5abc416e3b6916b19dfd4b04bfe390234f00c0b3))
* **cli:** say what a variable font can be set to, in the table and in the facets ([#95](https://github.com/oddurs/fontina/issues/95)) ([f187ee1](https://github.com/oddurs/fontina/commit/f187ee1883c3538e9b86703431b021946c2ce7ac))
* **cli:** say whether a font is free, and let the browser filter on it ([#55](https://github.com/oddurs/fontina/issues/55)) ([57de821](https://github.com/oddurs/fontina/commit/57de8211c40e3cc765042fa49a1b53e0bce3d67d))
* **cli:** the command line prints in one colour and one width ([#175](https://github.com/oddurs/fontina/issues/175)) ([516859e](https://github.com/oddurs/fontina/commit/516859e5ae6e1447ac2b7fc501b4c8920d9d2542))
* **core:** a table for script coverage, with the depth it always counted ([#99](https://github.com/oddurs/fontina/issues/99)) ([74362b5](https://github.com/oddurs/fontina/commit/74362b54f67c76c6c4b40153a59abb0356639572))
* **core:** a table for the languages a face claims, and which claim each one is ([#101](https://github.com/oddurs/fontina/issues/101)) ([405c743](https://github.com/oddurs/fontina/commit/405c74393263c066f5255e370e5599fd380d957b))
* **core:** five more health checks, and stop the documented list drifting ([#60](https://github.com/oddurs/fontina/issues/60)) ([8fa215a](https://github.com/oddurs/fontina/commit/8fa215af6ccad1fc09f624d214ba73b7f08d3385))
* **core:** Index::related, a question about one face rather than a stored grouping ([#104](https://github.com/oddurs/fontina/issues/104)) ([a8a3b8d](https://github.com/oddurs/fontina/commit/a8a3b8daec76c0a98dc5ffcaa6453b1666dceff7))
* **core:** lay the specimen out so the type can actually be judged ([#195](https://github.com/oddurs/fontina/issues/195)) ([1ddb3a4](https://github.com/oddurs/fontina/commit/1ddb3a4bad94f688c62636ef5a508704b7e7cb1c))
* **core:** let a collection export travel with its fonts ([#83](https://github.com/oddurs/fontina/issues/83)) ([ec352ae](https://github.com/oddurs/fontina/commit/ec352ae03bd3b46f0894b3669ef4ffe7ec1ea579))
* **core:** read what a font says it is, and rank pairings on it ([#180](https://github.com/oddurs/fontina/issues/180)) ([4b1f35e](https://github.com/oddurs/fontina/commit/4b1f35e8e89fa77bf5aecca44b1f8e139c77fbf6))
* **core:** report when a font's advances disagree with its own isFixedPitch ([#174](https://github.com/oddurs/fontina/issues/174)) ([5f92dfd](https://github.com/oddurs/fontina/commit/5f92dfd711258239df5193536d91de1fe3b5c978))
* **dist:** manifests for Homebrew, Scoop, winget and the AUR ([#116](https://github.com/oddurs/fontina/issues/116)) ([e8f33e4](https://github.com/oddurs/fontina/commit/e8f33e4ba705b582e1a1966fd7a96914568faa3b))
* **platform:** an optional login agent that puts activations back ([#64](https://github.com/oddurs/fontina/issues/64)) ([f795ae7](https://github.com/oddurs/fontina/commit/f795ae7ce9145d6425a83f7095d05b40e333e8ec))
* **platform:** read and write the operating system's own file tags ([#88](https://github.com/oddurs/fontina/issues/88)) ([dae8a98](https://github.com/oddurs/fontina/commit/dae8a9841c610146e8413f6e7df69d72a74ee08c))
* **site:** a design system in system sans, with the tool shown running ([#74](https://github.com/oddurs/fontina/issues/74)) ([28282c8](https://github.com/oddurs/fontina/commit/28282c89222d7ff3087946b41bf4ebf358d025ea))
* **site:** a landing page, a manual, and navigation in three tiers ([#113](https://github.com/oddurs/fontina/issues/113)) ([a02043b](https://github.com/oddurs/fontina/commit/a02043b35968504482c0bd5ca7a7760916d53e71))
* **site:** a session that ends in proof, and a browser that narrates itself ([#145](https://github.com/oddurs/fontina/issues/145)) ([e390992](https://github.com/oddurs/fontina/commit/e3909923469512caeb767a7f6ea061560c6315aa))
* **ui:** a filter bar that takes the command line's own flags ([#166](https://github.com/oddurs/fontina/issues/166)) ([2158c27](https://github.com/oddurs/fontina/commit/2158c27141998d811a790378210584367b3469e1))
* **ui:** a filter line, and the facets as a panel with a name ([#168](https://github.com/oddurs/fontina/issues/168)) ([9030d0e](https://github.com/oddurs/fontina/commit/9030d0efbdfb17085e774ac6a97c93612bb102d2))
* **ui:** as many panes as the terminal can carry, and no more ([#147](https://github.com/oddurs/fontina/issues/147)) ([4bd41ff](https://github.com/oddurs/fontina/commit/4bd41ffb3acc5ab205dca9ee85d5dd8babf6c7be))
* **ui:** every command the program has, listed where the reader is ([#155](https://github.com/oddurs/fontina/issues/155)) ([2a04e12](https://github.com/oddurs/fontina/commit/2a04e12032d5e89ba3ed7d33c3c672eefa89cf59))
* **ui:** faces ranked against this one, with the numbers behind it ([#173](https://github.com/oddurs/fontina/issues/173)) ([3432046](https://github.com/oddurs/fontina/commit/343204674cae1ac097f8d736a25cbc8f0b1a08f3))
* **ui:** mark many faces, act on them once ([#153](https://github.com/oddurs/fontina/issues/153)) ([3abc2b4](https://github.com/oddurs/fontina/commit/3abc2b40ea45d6fa97740d19cedb09b94e7cc730))
* **ui:** one box, one bar, one left edge ([#179](https://github.com/oddurs/fontina/issues/179)) ([984dbcf](https://github.com/oddurs/fontina/commit/984dbcf5f9c93c819814fe003e136294323b25e4))
* **ui:** one palette, resolved against what the terminal can show ([#146](https://github.com/oddurs/fontina/issues/146)) ([89f32f6](https://github.com/oddurs/fontina/commit/89f32f65cb20d6353e6f8d8c5ccec73a5e09b504))
* **ui:** take back the last thing that changed the index ([#154](https://github.com/oddurs/fontina/issues/154)) ([4579667](https://github.com/oddurs/fontina/commit/457966740f15e4f916fd7d52e4edfaf0a17fb5d3))
* **ui:** what else in the library is nearly this font ([#171](https://github.com/oddurs/fontina/issues/171)) ([a96ff0f](https://github.com/oddurs/fontina/commit/a96ff0f601e93d784271ebe4bdd5304d1ccf348c))
* **ui:** who can set this text ([#172](https://github.com/oddurs/fontina/issues/172)) ([2ddc2a6](https://github.com/oddurs/fontina/commit/2ddc2a64951fda4199a4bb09cdffde833e048770))


### Bug Fixes

* **acceptance:** ask fontina where the copy went rather than guessing ([#133](https://github.com/oddurs/fontina/issues/133)) ([3bdaaff](https://github.com/oddurs/fontina/commit/3bdaaffb0471f3442ff7cec971227ef73037cc50))
* **ci:** keep-current blocked the merges it exists to enable ([#182](https://github.com/oddurs/fontina/issues/182)) ([228f629](https://github.com/oddurs/fontina/commit/228f629bc178b1d738d315b8352f5e1dd1546584))
* **cli:** "no conflicts" has to mean somebody looked ([#136](https://github.com/oddurs/fontina/issues/136)) ([f1b5418](https://github.com/oddurs/fontina/commit/f1b5418e12ca43409d3d36656b40a49040c54253))
* **cli:** a path is the answer, so print all of it ([#129](https://github.com/oddurs/fontina/issues/129)) ([54b3b87](https://github.com/oddurs/fontina/commit/54b3b87ce4477c2a0a4e044f8c40daa6a5901b47))
* **cli:** a transition gives up the state it leaves ([#121](https://github.com/oddurs/fontina/issues/121)) ([5495617](https://github.com/oddurs/fontina/commit/54956178e0a14d321abe0f32f6d0be4b1f521e60))
* **cli:** describe every --json output in schemas/cli-output.json ([#119](https://github.com/oddurs/fontina/issues/119)) ([aaf7d9b](https://github.com/oddurs/fontina/commit/aaf7d9b4a3fb6caa72efda6c792c59cab2f46203))
* **cli:** draw the preview, keep the file name, and reject nonsense colours ([#66](https://github.com/oddurs/fontina/issues/66)) ([87e045b](https://github.com/oddurs/fontina/commit/87e045be41a8c1f90f40f56f45378b807af239f2))
* **cli:** end quietly when a pipe closes, like every other Unix program ([#58](https://github.com/oddurs/fontina/issues/58)) ([854fdfc](https://github.com/oddurs/fontina/commit/854fdfcce8f0356d19a484a458097fcd30b78cf6))
* **cli:** keep the searched-for character on the glyph map, and stop an overflow ([#86](https://github.com/oddurs/fontina/issues/86)) ([5ead289](https://github.com/oddurs/fontina/commit/5ead289a1a34805e9631290e628ec712a2967f89))
* **cli:** name a maintainer in the .deb, as Debian Policy requires ([#72](https://github.com/oddurs/fontina/issues/72)) ([9bcb4b3](https://github.com/oddurs/fontina/commit/9bcb4b33e15c1df32b2e6acd99aa9a2be9ba3d36))
* **cli:** pad table columns by terminal columns, not by characters ([#123](https://github.com/oddurs/fontina/issues/123)) ([056ac89](https://github.com/oddurs/fontina/commit/056ac89c4dc3bb33d1bb589409b4474caf4f1f2e))
* **cli:** say what the index cannot see once, not on every activation ([#144](https://github.com/oddurs/fontina/issues/144)) ([2f3ea34](https://github.com/oddurs/fontina/commit/2f3ea34ea76f3c23537073803b655eddc7802400))
* **cli:** the licence column carries the licence, not SPDX's namespace ([#138](https://github.com/oddurs/fontina/issues/138)) ([b44c370](https://github.com/oddurs/fontina/commit/b44c37076cacc7d9d870afe7224d08607079131f))
* **cli:** the two latent traps the browser tests wrote down ([#131](https://github.com/oddurs/fontina/issues/131)) ([c991cd4](https://github.com/oddurs/fontina/commit/c991cd4717e8a7c47871e0e3aa0b4e9c0e3a6582))
* **cli:** two filters that silently answered a different question ([#106](https://github.com/oddurs/fontina/issues/106)) ([9b57f60](https://github.com/oddurs/fontina/commit/9b57f6010dd8f5254d9477d39f0a26b9b4cc05ed))
* **cli:** what a review of tag sync found, before it can lose anyone's tags ([#97](https://github.com/oddurs/fontina/issues/97)) ([e8d5ad0](https://github.com/oddurs/fontina/commit/e8d5ad08712906f20698f480a37060f9b4fff4ab))
* **core:** a vendor id padded with NUL is text a person can type ([#137](https://github.com/oddurs/fontina/issues/137)) ([1d3064b](https://github.com/oddurs/fontina/commit/1d3064b5bf1870b53ea8e5b5ba57aa8b9f453472))
* **core:** an empty cmap no longer hides the rest of a font's report ([#57](https://github.com/oddurs/fontina/issues/57)) ([dee1c5c](https://github.com/oddurs/fontina/commit/dee1c5ce564f6ec2fde0696304b8231c48801219))
* **core:** clip a preview instead of smearing it, and stop the rasteriser overflowing ([#82](https://github.com/oddurs/fontina/issues/82)) ([882fe20](https://github.com/oddurs/fontina/commit/882fe20fb103e92cb141a23280d038cb881c9c01))
* **core:** contain the WOFF2 decoder, which panics on a file it is given ([#114](https://github.com/oddurs/fontina/issues/114)) ([3893cef](https://github.com/oddurs/fontina/commit/3893cef1ffade38d966641aa0a392829d6102887))
* **core:** count the characters a font has no glyph for ([#125](https://github.com/oddurs/fontina/issues/125)) ([eaea3b8](https://github.com/oddurs/fontina/commit/eaea3b806720b8d4bc625c0cac6185dbda2137d8))
* **core:** find a variable font at every weight it spans, not only its default ([#94](https://github.com/oddurs/fontina/issues/94)) ([b73413c](https://github.com/oddurs/fontina/commit/b73413cb8c8f5d09bc8452f141bb14d0aace93ee))
* **core:** let the script filter use its index ([#107](https://github.com/oddurs/fontina/issues/107)) ([cc0e993](https://github.com/oddurs/fontina/commit/cc0e993f6116e10b419a205ccced7db147f0b14f))
* **core:** quote and encode what goes into a stylesheet ([#120](https://github.com/oddurs/fontina/issues/120)) ([0e52ea9](https://github.com/oddurs/fontina/commit/0e52ea96e75a8ed75bcb8845008d28a209ee1716))
* **core:** say which fonts a scan walked past ([#141](https://github.com/oddurs/fontina/issues/141)) ([07d2445](https://github.com/oddurs/fontina/commit/07d2445e28c2fb1696c6ff6f434661ff0fe5a711))
* **core:** stop retrying an index that will never open ([#127](https://github.com/oddurs/fontina/issues/127)) ([2a67a10](https://github.com/oddurs/fontina/commit/2a67a10fabf5357f6b6c28160fbae3251acc307a))
* **core:** the M4 review's four smaller findings ([#111](https://github.com/oddurs/fontina/issues/111)) ([c14a51b](https://github.com/oddurs/fontina/commit/c14a51b0b77c84e5266ef4a5e405ca6132fe913b))
* **core:** the watcher keeps up with what people do to a font directory ([#115](https://github.com/oddurs/fontina/issues/115)) ([e077868](https://github.com/oddurs/fontina/commit/e07786812a2359f31b28546e079e1f60fc2b67a0))
* **core:** two processes can create the same index at the same instant ([#108](https://github.com/oddurs/fontina/issues/108)) ([f97fc98](https://github.com/oddurs/fontina/commit/f97fc9809049c5a3de509f3fbf784a6b912b6dce))
* **platform:** one lock for every test that touches the environment ([#185](https://github.com/oddurs/fontina/issues/185)) ([69e10d3](https://github.com/oddurs/fontina/commit/69e10d3946c5d97eefd462e608f759eb91507f77))
* **platform:** the login agent touches only files fontina wrote ([#122](https://github.com/oddurs/fontina/issues/122)) ([ba75807](https://github.com/oddurs/fontina/commit/ba75807f97916a5a1bcbf9251c3fba48c03084c3))
* **site:** the navbar never said where you were ([#220](https://github.com/oddurs/fontina/issues/220)) ([cdf974e](https://github.com/oddurs/fontina/commit/cdf974ed900a6ecc4918353ffad50ff1819c1cd5))
* the deadline I set fired on honest work within the hour ([#215](https://github.com/oddurs/fontina/issues/215)) ([ac53c9e](https://github.com/oddurs/fontina/commit/ac53c9e8aec83849bb95ebccdfb3960f568eaa11))
* **ui:** a filter that matches nothing keeps the row that undoes it ([#163](https://github.com/oddurs/fontina/issues/163)) ([d8ca2ac](https://github.com/oddurs/fontina/commit/d8ca2acf87b385e8551610aa3339f9a74017d282))
* **ui:** say what a pseudo-script is, rather than spelling its code ([#169](https://github.com/oddurs/fontina/issues/169)) ([43340bd](https://github.com/oddurs/fontina/commit/43340bd34a7bef74e0d4f1a1ff7a380084511823))
* **ui:** the search worker took a request off its channel and dropped it ([#167](https://github.com/oddurs/fontina/issues/167)) ([74c7a6d](https://github.com/oddurs/fontina/commit/74c7a6dbd118756b24affd06b4d7a1556d8d8f60))
* what a review of M2 as a whole found ([#76](https://github.com/oddurs/fontina/issues/76)) ([ac8bce1](https://github.com/oddurs/fontina/commit/ac8bce1b0682bc062b2af211ad55c4139ed61335))
* **wt:** catch a branch whose pull request was closed without merging ([#205](https://github.com/oddurs/fontina/issues/205)) ([955dbd1](https://github.com/oddurs/fontina/commit/955dbd1133892f968d97251860fcd8860e63492a))


### Performance Improvements

* **cli:** buffer the face table, and state the list budget for the machine that checks it ([#61](https://github.com/oddurs/fontina/issues/61)) ([a69731d](https://github.com/oddurs/fontina/commit/a69731df95e7fcb5c805a7d79b4b3e2ad826c7ab))
* **ui:** a pane builds the rows it draws, not the rows it holds ([#149](https://github.com/oddurs/fontina/issues/149)) ([a57a30f](https://github.com/oddurs/fontina/commit/a57a30f084dde9229803f9e212e4786a20eaf86b))
* **ui:** a repaint budget, and the property behind it, in scripts/bench ([#151](https://github.com/oddurs/fontina/issues/151)) ([4048313](https://github.com/oddurs/fontina/commit/4048313fe5b236096963050abb2fbeb3ec89ec00))
* **ui:** say what the preview cache costs, and prove what it saves ([#150](https://github.com/oddurs/fontina/issues/150)) ([40340ba](https://github.com/oddurs/fontina/commit/40340baa38411887b0e7c376bbaaeee12ace5e88))
* **ui:** the listing queries leave the thread that draws ([#152](https://github.com/oddurs/fontina/issues/152)) ([95ce83d](https://github.com/oddurs/fontina/commit/95ce83dbda8d8cf0751997e1d1150bf6f1a08661))

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
