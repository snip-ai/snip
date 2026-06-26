# Changelog

## [0.7.0](https://github.com/snip-ai/snip/compare/v0.6.0...v0.7.0) (2026-06-26)


### 🚀 Features

* **commands:** split /snip into per-purpose commands; make queries model-invocable ([#40](https://github.com/snip-ai/snip/issues/40)) ([04fd3ba](https://github.com/snip-ai/snip/commit/04fd3baac32a8f65816c3059982c6f16c3930705))
* **lifecycle:** surface install/update/uninstall state via a SessionStart banner ([#38](https://github.com/snip-ai/snip/issues/38)) ([8e6ca42](https://github.com/snip-ai/snip/commit/8e6ca426eea9df0c497c337b2303d2bc8606c8ea))
* **read:** always compact, even windowed reads — resolve is the sole Edit recovery ([#39](https://github.com/snip-ai/snip/issues/39)) ([8710ac6](https://github.com/snip-ai/snip/commit/8710ac67ecc6d3f183e5fa92978d97f62a47fca0))


### 🐛 Bug Fixes

* **lifecycle:** honor .uninstalled marker so `snip uninstall` is not auto-undone ([#35](https://github.com/snip-ai/snip/issues/35)) ([f52a810](https://github.com/snip-ai/snip/commit/f52a8104f0af10c4deaa5982a6c693d4ae030502))


### ♻️ Refactoring

* **install:** make PATH setup opt-in (no auto rc/PATH writes on install) ([#37](https://github.com/snip-ai/snip/issues/37)) ([b999346](https://github.com/snip-ai/snip/commit/b99934637657465ba3bc993ca9bfbce730c90053))

## [0.6.0](https://github.com/snip-ai/snip/compare/v0.5.2...v0.6.0) (2026-06-26)


### 🚀 Features

* context-optimization audit follow-ups (always-optimize large files/commands, Edit-after-Read recovery, glob fold) ([#33](https://github.com/snip-ai/snip/issues/33)) ([632dc47](https://github.com/snip-ai/snip/commit/632dc479e06be02d1d96851b15c6958d0f14111a))

## [0.5.2](https://github.com/snip-ai/snip/compare/v0.5.1...v0.5.2) (2026-06-25)


### 🐛 Bug Fixes

* **overflow:** trim spill breadcrumbs so marginal recoverable compactions still win ([#31](https://github.com/snip-ai/snip/issues/31)) ([28c8c36](https://github.com/snip-ai/snip/commit/28c8c36b388852cd82c932dca014503311259d5a))

## [0.5.1](https://github.com/snip-ai/snip/compare/v0.5.0...v0.5.1) (2026-06-25)


### 🐛 Bug Fixes

* **command:** don't wrap a command that redirects to a Git Bash magic device ([#27](https://github.com/snip-ai/snip/issues/27)) ([b5be1c6](https://github.com/snip-ai/snip/commit/b5be1c6d8da59b0c15877164d71caf70fc347510))
* **command:** never wrap a pipe with an interactive/streaming stage upstream ([#25](https://github.com/snip-ai/snip/issues/25)) ([c696703](https://github.com/snip-ai/snip/commit/c69670375ed1ed0066190ee8354b88a833dbb005))
* **command:** recognize git subcommands behind value-taking global options ([#29](https://github.com/snip-ai/snip/issues/29)) ([7df89f1](https://github.com/snip-ai/snip/commit/7df89f186fbb024977ddab489eb91e340835da47))
* **fold:** don't fold a pure-placeholder template (misrepresents distinct values) ([#26](https://github.com/snip-ai/snip/issues/26)) ([1559ffe](https://github.com/snip-ai/snip/commit/1559ffec50f0b8fb6316d4a956023a61a5382ad5))
* **overflow:** spill the middle a lossy Truncate elides (never discard output) ([#28](https://github.com/snip-ai/snip/issues/28)) ([078d4bd](https://github.com/snip-ai/snip/commit/078d4bda7214a032db4401ff5d419bbe37920ae3))
* **test:** align phase-b-shell-setup to the current script; skip on win32 ([#24](https://github.com/snip-ai/snip/issues/24)) ([7b7c9fc](https://github.com/snip-ai/snip/commit/7b7c9fc635c71532b356f02e8ee952e4274fdace))

## [0.5.0](https://github.com/snip-ai/snip/compare/v0.4.1...v0.5.0) (2026-06-25)


### 🚀 Features

* put snip on PATH everywhere on Windows (shell rc + USER PATH) ([#22](https://github.com/snip-ai/snip/issues/22)) ([5680dbf](https://github.com/snip-ai/snip/commit/5680dbf6e60619b92306b22c63b385560cb72b83))

## [0.4.1](https://github.com/snip-ai/snip/compare/v0.4.0...v0.4.1) (2026-06-25)


### 🐛 Bug Fixes

* target the right shell rc on Windows git bash; refresh command names ([#20](https://github.com/snip-ai/snip/issues/20)) ([77912bb](https://github.com/snip-ai/snip/commit/77912bb82b5ac82106bb3e608f1fac0044017e16))

## [0.4.0](https://github.com/snip-ai/snip/compare/v0.3.0...v0.4.0) (2026-06-25)


### 🚀 Features

* add `snip uninstall` to tear down state, binary, and PATH line ([#14](https://github.com/snip-ai/snip/issues/14)) ([dc30af6](https://github.com/snip-ai/snip/commit/dc30af69088facc0af92e0a295755017d574a79d))
* collapse to a single `/snip` command; rewrite the lifecycle doctrine ([#17](https://github.com/snip-ai/snip/issues/17)) ([75493fa](https://github.com/snip-ai/snip/commit/75493faad543111487f7a91d2407ff0f3e59a843))
* single `/snip` entry, `update` alias, auto-PATH, git-bash uninstall ([#16](https://github.com/snip-ai/snip/issues/16)) ([a04c736](https://github.com/snip-ai/snip/commit/a04c736134676626b2859298e55db1f97d1911e1))


### 🐛 Bug Fixes

* spawn the self-update bootstrap from git bash, not the native binary ([#18](https://github.com/snip-ai/snip/issues/18)) ([753f7bb](https://github.com/snip-ai/snip/commit/753f7bb5a9f6226314af16d9ff3473760b381857))


### 📚 Documentation

* lead with the shell commands in both READMEs ([#19](https://github.com/snip-ai/snip/issues/19)) ([0c06827](https://github.com/snip-ai/snip/commit/0c06827ed17e47f54ddc635eb171592d19f919d6))

## [0.3.0](https://github.com/snip-ai/snip/compare/v0.2.0...v0.3.0) (2026-06-24)


### 🚀 Features

* auto-update the binary to the latest release ([#12](https://github.com/snip-ai/snip/issues/12)) ([9772b47](https://github.com/snip-ai/snip/commit/9772b4730822aac1b5ca036910a4d13f3f500b60))

## [0.2.0](https://github.com/snip-ai/snip/compare/v0.1.1...v0.2.0) (2026-06-24)


### 🚀 Features

* **plugin:** opt-in /snip-shell-setup for running snip from a shell ([#10](https://github.com/snip-ai/snip/issues/10)) ([cf801b9](https://github.com/snip-ai/snip/commit/cf801b969edd9551d522d5c7a5ef968e57713ce7))

## [0.1.1](https://github.com/snip-ai/snip/compare/v0.1.0...v0.1.1) (2026-06-24)


### 🐛 Bug Fixes

* drop duplicate hooks declaration from the plugin manifest ([#5](https://github.com/snip-ai/snip/issues/5)) ([4bae211](https://github.com/snip-ai/snip/commit/4bae2115185c11b543ccd833800726bb364f4c03))


### ⚡ Performance

* run snip meta-commands on Haiku to cut their token cost ([#8](https://github.com/snip-ai/snip/issues/8)) ([04cb2c0](https://github.com/snip-ai/snip/commit/04cb2c0bf147726e40c49f251aa520aec10db21f))


### 📚 Documentation

* clarify install for integrated Claude Code; drop static badges ([#9](https://github.com/snip-ai/snip/issues/9)) ([c065d89](https://github.com/snip-ai/snip/commit/c065d89485b5f4f781d749c73536ec6f9b906f66))

## 0.1.0 (2026-06-23)


### 🚀 Features

* initial public release ([17a2545](https://github.com/snip-ai/snip/commit/17a25457134a2f1375617555638f2fb2cb213dd3))

## Changelog

All notable changes to snip are recorded here. This file is maintained
automatically by [release-please](https://github.com/googleapis/release-please)
from [Conventional Commits](https://www.conventionalcommits.org) — please don't
edit it by hand.
