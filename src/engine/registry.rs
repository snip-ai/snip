//! Optimizer registry: the optimizers bound to one surface, first-match-wins.
//!
//! Each hook process serves a single surface, so the registry holds exactly that
//! surface's optimizers — only the spec families that surface can use are parsed
//! (a Read hook parses no specs at all).

use crate::config::Config;
use crate::domain::{HookCtx, Optimizer, Surface};
use crate::optimizers::SpecOptimizer;
use crate::optimizers::read::ReadOptimizer;
use crate::spec::builtin::merged_specs_for;

/// The optimizers bound to one surface, in first-match-wins order.
pub struct Registry {
    optimizers: Vec<Box<dyn Optimizer>>,
}

impl Registry {
    /// Build the registry for `surface`: the Rust `read` optimizer on
    /// Read/Edit/Write, plus every declarative spec bound to `surface` (built-ins
    /// overlaid by user specs, by `name`). Bash command specs are consumed by the
    /// command runtime (`bash-route`/`exec`), so the registry binds specs only for
    /// non-Bash surfaces. Built once per hook process — no global cache, so a
    /// different `cfg`/`surface` always yields the matching registry.
    #[must_use]
    pub fn build(cfg: &Config, surface: Surface) -> Self {
        let mut optimizers: Vec<Box<dyn Optimizer>> = Vec::new();
        // Exhaustive on purpose (no wildcard): a new `Surface` variant fails to
        // compile until it is bound here, instead of silently yielding an empty
        // registry that passes through forever — the worst failure for a tool
        // whose whole value is the rewrite.
        match surface {
            // Code surfaces are owned by the Rust `read` optimizer (not
            // spec-extensible — see `OptimizerSpec::validate`).
            Surface::Read | Surface::Edit | Surface::Write => {
                optimizers.push(Box::new(ReadOptimizer));
            }
            // Output search surfaces: declarative specs (built-ins overlaid by
            // user specs, by name). An invalid user spec is dropped with a note
            // rather than instantiated as a silently-inert optimizer. The `search`
            // family switch gates the whole surface (mirrors how `command` gates its
            // runtime) — the per-spec `search-grep`/`search-glob` switches still apply
            // via `first_match`'s `optimizer_enabled(name)` check.
            Surface::Grep | Surface::Glob => {
                if cfg.optimizer_enabled("search") {
                    for spec in merged_specs_for(surface, &cfg.specs) {
                        if let Err(reason) = spec.validate() {
                            eprintln!("[snip] ignoring spec `{}`: {reason}", spec.name);
                            continue;
                        }
                        optimizers.push(Box::new(SpecOptimizer::new(spec)));
                    }
                }
            }
            // Bash specs are consumed by the command runtime (`bash-route`/`exec`),
            // never the registry.
            Surface::Bash => {}
        }
        Self { optimizers }
    }

    /// The first enabled optimizer that matches `ctx`, if any.
    #[must_use]
    pub fn first_match(&self, ctx: &HookCtx) -> Option<&dyn Optimizer> {
        self.optimizers
            .iter()
            .map(Box::as_ref)
            .find(|o| ctx.cfg.optimizer_enabled(o.name()) && o.matches(ctx))
    }
}

#[cfg(test)]
#[path = "../../tests/unit/engine/registry.tests.rs"]
mod tests;
