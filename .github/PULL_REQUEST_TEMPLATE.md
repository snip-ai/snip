## Summary

<!-- One or two sentences: what this PR does and why it matters. -->

## Motivation / context

<!-- Why is this change needed? Link the issue (e.g. Closes #123) or the design
note this implements. What problem does it solve? -->

## What changed

<!-- The concrete changes, as a short bulleted list. Call out new surfaces,
optimizers/specs, transforms, config keys, or behavioural changes. Note any
user-visible or breaking changes explicitly. -->

-

## How tested

The four required checks (all green):

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings` (all/pedantic/nursery denied)
- [ ] `cargo test`
- [ ] `cargo build --release`

Which tests cover this change:

<!-- Name the unit / integration tests added or updated (tests/{unit,integration}/...),
and any manual verification (e.g. driving a hook with JSON on stdin). -->

## Checklist

- [ ] PR title follows Conventional Commits (`feat`/`fix`/`perf`/`docs`/`refactor`/`test`/`build`/`ci`/`chore`; `!` or `BREAKING CHANGE` for a major).
- [ ] `fmt`, `clippy`, `test`, and `build --release` are all green.
- [ ] Tests added or updated for the change.
- [ ] Docs updated if needed (`CLAUDE.md`, `.claude/rules/`).
- [ ] No predecessor-tool names introduced.
- [ ] `CHANGELOG.md` not hand-edited (release-please owns it).
