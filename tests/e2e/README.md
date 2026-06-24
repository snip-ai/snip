# End-to-end tests

E2E tests drive the **built `snip` binary** the way Claude Code does: feed a
hook's JSON on **stdin**, assert the JSON on **stdout** and the exit code
(hooks always `0`). They mirror `src/` like the other tiers
(`<area>.tests.rs`) and are wired as the dedicated `e2e` `[[test]]` target in
`Cargo.toml` (`autotests = false`, so nothing is auto-discovered).

The binary is located via `assert_cmd` (`Command::cargo_bin("snip")`, which reads
the `CARGO_BIN_EXE_snip` cargo provides to test targets). Each test runs under an
isolated, auto-cleaned `SNIP_HOME` (a `tempfile::TempDir`), so on-disk config,
the stats DB, and the session cache never touch the real data dir or another
test — the suite is parallel-safe and leaves nothing behind. The shared harness
lives in [`support.rs`](support.rs) (`Snip::fresh()` / `.command()` / `.run()`).

## Coverage

| File | Surface(s) exercised end-to-end |
|---|---|
| `read_hook.tests.rs` | `read-hook`: commented code → compacted nested `tool_response`; comment-free → pass-through; `SNIP_ENABLED=0` → pass-through |
| `search_hooks.tests.rs` | `grep-hook` / `glob-hook`: reducible output → rewrite; tiny output → pass-through; glob directory grouping |
| `command.tests.rs` | `bash-route`: wrappable → `snip exec` rewrite; already-wrapped / disabled → pass-through. `exec`: verbatim run + exit-code preservation |
| `edit_write.tests.rs` | `edit-fix`: verbatim / missing-file pass-through. `write-guard`: stripped-view reproduction → `ask`; new file / genuine write → pass-through |
| `maintenance.tests.rs` | `session-reset`: drops only the named session cache. `update-check`: no-plugin-root no-op; with a plugin root but no bootstrap script, writes the throttle, no respawn |
| `cli.tests.rs` | `config` get/set/list, `enable`/`disable`, `status`, `gain`, `resolve` (match + no-match), `--version`, unknown subcommand (clap exit 2) |
| `robustness.tests.rs` | the always-exit-0 invariant: every hook × {empty, whitespace, malformed, wrong-shape} stdin → exit 0, empty stdout |

The command-runtime cases (`exec`) no-op where a POSIX `sh` is absent
(`support::sh_available`), so the suite stays CI-safe on a bare Windows runner.
