//! The built-in spec registry: declarative specs embedded as JSON assets.
//!
//! Specs live as per-family JSON arrays under `specs/` (search/base/git/lang),
//! embedded at compile time via `include_str!` — the single-static-binary property
//! holds (no runtime file load) — and parsed to [`OptimizerSpec`]. Same format as
//! the user's `config.specs[]`, so the override chain is one shape.
//!
//! Authoring notes (JSON carries no inline comments):
//! - **No flag injection for BSD coreutils** (`ls`, `grep`, `df`, `tree`): snip
//!   appends injected flags after operands and BSD/macOS getopt does not permute,
//!   so a trailing flag would be treated as a filename. Injection is reserved for
//!   permuting / clap tools (`rg`, `git`, `cargo`, `npm`, …).
//! - **Lang specs are scoped to terminating invocations** (no `npm` start/run,
//!   bare `vitest` watch, mutating writes) so a wrapped server can't block.

use crate::domain::Surface;
use crate::spec::OptimizerSpec;

const SEARCH: &str = include_str!("specs/search.json");
const BASE: &str = include_str!("specs/base.json");
const GIT: &str = include_str!("specs/git.json");
const LANG: &str = include_str!("specs/lang.json");

/// Parse the given embedded family JSON arrays into specs.
///
/// A malformed family is skipped rather than aborting the whole set; the test
/// suite asserts each family parses so a typo can't silently drop specs.
fn parse_families(families: &[&str]) -> Vec<OptimizerSpec> {
    families
        .iter()
        .filter_map(|json| serde_json::from_str::<Vec<OptimizerSpec>>(json).ok())
        .flatten()
        .collect()
}

/// The embedded families a `surface` can ever use, so the hot path parses only
/// what it needs.
///
/// Grep/Glob → search; Bash → base+git+lang; Read/Edit/Write → none (no built-in
/// spec targets them — `read` is the Rust optimizer).
const fn families_for(surface: Surface) -> &'static [&'static str] {
    match surface {
        Surface::Grep | Surface::Glob => &[SEARCH],
        Surface::Bash => &[BASE, GIT, LANG],
        Surface::Read | Surface::Edit | Surface::Write => &[],
    }
}

/// All built-in specs, parsed from every embedded family.
///
/// The full set; the hot path uses [`builtin_specs_for`] instead.
#[must_use]
pub fn builtin_specs() -> Vec<OptimizerSpec> {
    parse_families(&[SEARCH, BASE, GIT, LANG])
}

/// Built-in specs for `surface`, parsing only the families it can use.
///
/// Keeps only specs that actually target `surface`, so Grep doesn't pick up the
/// Glob `search` spec sharing the search family.
#[must_use]
pub fn builtin_specs_for(surface: Surface) -> Vec<OptimizerSpec> {
    parse_families(families_for(surface))
        .into_iter()
        .filter(|spec| spec.surface == surface)
        .collect()
}

/// Built-in specs for `surface` overlaid by the user `specs` targeting it.
///
/// Shadowed by `name`: a user spec replaces a built-in of the same name, a new
/// name extends coverage. The single layered merge both the registry (non-Bash)
/// and the command runtime (Bash) draw from.
#[must_use]
pub fn merged_specs_for(surface: Surface, user: &[OptimizerSpec]) -> Vec<OptimizerSpec> {
    let mut specs = builtin_specs_for(surface);
    for user_spec in user.iter().filter(|s| s.surface == surface) {
        if let Some(slot) = specs.iter_mut().find(|s| s.name == user_spec.name) {
            *slot = user_spec.clone();
        } else {
            specs.push(user_spec.clone());
        }
    }
    specs
}

#[cfg(test)]
#[path = "../../../tests/unit/spec/builtin.tests.rs"]
mod tests;
