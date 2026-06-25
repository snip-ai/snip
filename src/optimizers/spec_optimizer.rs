//! Adapter that runs an [`OptimizerSpec`] as an [`Optimizer`] on an output surface.
//!
//! Serves the Grep/Glob surfaces (via the registry) and the Bash command runtime
//! (which reuses [`SpecOptimizer::apply_to`] on each captured slice). Read/Edit/Write
//! are owned by the Rust `read` optimizer and are not spec-extensible — see
//! [`OptimizerSpec::validate`].

use crate::domain::{HEADER_PREFIX, HookCtx, Optimizer, Outcome, Surface};
use crate::spec::{OptimizerSpec, Transform};
use crate::tokens::estimate_tokens;

/// Adapter that runs an [`OptimizerSpec`] as an [`Optimizer`] on an output surface.
pub struct SpecOptimizer {
    spec: OptimizerSpec,
    surfaces: [Surface; 1],
}

impl SpecOptimizer {
    /// Wrap a spec into a dispatchable optimizer.
    #[must_use]
    pub const fn new(spec: OptimizerSpec) -> Self {
        let surfaces = [spec.surface];
        Self { spec, surfaces }
    }
}

impl Optimizer for SpecOptimizer {
    fn name(&self) -> &str {
        &self.spec.name
    }

    fn surfaces(&self) -> &[Surface] {
        &self.surfaces
    }

    fn matches(&self, ctx: &HookCtx) -> bool {
        // Surface, plus the optional `bind.path_globs` scope against the Grep/Glob
        // search `path`. `bind.cmd`/`subcommands` are evaluated by the command
        // runtime (`CommandSpecs`), never here — see ARCHITECTURE.md §2.3.
        ctx.surface == self.spec.surface
            && self
                .spec
                .bind
                .path_matches(ctx.input.get("path").and_then(serde_json::Value::as_str))
    }

    fn apply(&self, ctx: &HookCtx) -> anyhow::Result<Outcome> {
        Ok(ctx.output.map_or(Outcome::PassThrough, |o| {
            self.apply_to(o, ctx.cfg.secret_safe)
        }))
    }
}

impl SpecOptimizer {
    /// Run the transform pipeline over `output`, applying the no-inflation guard
    /// and tagging a header. Pure (no `HookCtx`), so the command runtime can
    /// reuse the exact same pipeline on a captured output slice. With
    /// `secret_safe`, secret-bearing lines are masked before any transform.
    #[must_use]
    pub fn apply_to(&self, output: &str, secret_safe: bool) -> Outcome {
        let original_tokens = estimate_tokens(output);
        let original_lines = output.lines().count();
        let mut records: Vec<String> = output.lines().map(str::to_owned).collect();
        let masked_any = secret_safe && crate::optimizers::redact::mask_records(&mut records);
        let mut lossy = false;
        for transform in &self.spec.transforms {
            // A `Truncate` that fires drops the middle records irrecoverably (the
            // marker only counts them). Flag it — using the same threshold as
            // `truncate` itself — so the caller spills the full original.
            if let Transform::Truncate { head, tail } = transform
                && records.len() > head + tail + 1
            {
                lossy = true;
            }
            records = transform.apply(records);
        }
        let trailing = if output.ends_with('\n') { "\n" } else { "" };
        let body = format!("{}{trailing}", records.join("\n"));
        let body_tokens = estimate_tokens(&body);
        let new_lines = records.len();
        let name = &self.spec.name;
        let header = format!(
            "{HEADER_PREFIX} {name} | {original_lines}→{new_lines} lines, {original_tokens}→{body_tokens} tok]\n"
        );
        // No-inflation guard, header-inclusive: the model pays for the injected
        // header *plus* the body, so the combined view — not the body alone — must
        // be strictly smaller than the original. Applies to every optimizer (and to
        // the command runtime, which reuses this path via `assemble`). Skipped when a
        // line was masked: passing through would un-redact the secret, and masking
        // never lengthens the output.
        let new_tokens = estimate_tokens(&header) + body_tokens;
        if new_tokens >= original_tokens && !masked_any {
            return Outcome::PassThrough;
        }
        Outcome::Rewrite {
            header,
            body,
            original_tokens,
            new_tokens,
            lossy,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/optimizers/spec_optimizer.tests.rs"]
mod tests;
