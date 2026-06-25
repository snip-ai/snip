//! Reassemble a command's captured stdout from its wrapped run.
//!
//! Slice it on the plan's markers, optimize each recognized unit's slice (the
//! same `SpecOptimizer` pipeline + overflow as every other surface), and leave
//! the rest verbatim. Pure — no process I/O.

use super::{CommandSpecs, Plan};
use crate::config::Config;
use crate::domain::Outcome;
use crate::optimizers::SpecOptimizer;
use crate::overflow::Spill;
use crate::spec::OptimizerSpec;
use crate::tokens::estimate_tokens;

/// Slice `captured` on `plan.token` and optimize each recognized unit's slice.
///
/// On a marker-count mismatch (collision or early `exit`), strips the markers and
/// returns the output verbatim — never corrupts. `session` scopes any spill file.
#[must_use]
pub fn assemble(
    captured: &str,
    plan: &Plan,
    specs: &CommandSpecs,
    cfg: &Config,
    session: Option<&str>,
) -> String {
    let parts: Vec<&str> = captured.split(&plan.token).collect();
    if parts.len() != plan.recognized.len() + 1 {
        return captured.replace(&plan.token, "");
    }
    let mut out = String::with_capacity(captured.len());
    out.push_str(parts[0]);
    for (slice, name) in parts[1..].iter().zip(&plan.recognized) {
        match name.as_deref().and_then(|n| specs.by_name(n)) {
            Some(spec) => out.push_str(&optimize(slice, spec, cfg, session)),
            None => out.push_str(slice),
        }
    }
    out
}

/// Optimize one slice through its spec, capping with the shared overflow service.
fn optimize(slice: &str, spec: &OptimizerSpec, cfg: &Config, session: Option<&str>) -> String {
    match SpecOptimizer::new(spec.clone()).apply_to(slice, cfg.secret_safe) {
        Outcome::Rewrite {
            header,
            body,
            lossy,
            ..
        } => {
            let ov = cfg.overflow_for_command(&spec.name);
            if lossy {
                // A lossy `Truncate` dropped the middle records; spill the full slice
                // so they stay recoverable, then breadcrumb the view. The breadcrumb
                // costs a line, so a near-threshold short output can end up no smaller
                // than the original — re-apply the no-inflation guard and fall back to
                // the verbatim slice (nothing dropped, no spill needed) when it does.
                let recoverable = Spill::keep_recoverable(&body, slice, session, &spec.name);
                let capped = Spill::apply(recoverable, session, &spec.name, &ov);
                let view = format!("{header}{capped}");
                if estimate_tokens(&view) < estimate_tokens(slice) {
                    view
                } else {
                    slice.to_string()
                }
            } else {
                let capped = Spill::apply(body, session, &spec.name, &ov);
                format!("{header}{capped}")
            }
        }
        _ => slice.to_string(),
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/command/assemble.tests.rs"]
mod tests;
