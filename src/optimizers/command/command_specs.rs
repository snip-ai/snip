//! The command specs (declarative data) bound to the Bash surface, indexed for
//! `argv0`/sub-command lookup. Built-ins overlaid by user `specs[]` via
//! `spec::builtin::merged_specs_for(Bash, ..)`.

use crate::config::Config;
use crate::domain::Surface;
use crate::spec::OptimizerSpec;
use crate::spec::builtin::merged_specs_for;

/// All Bash-surface command specs, loaded once per `bash-route`/`exec` call.
pub struct CommandSpecs {
    specs: Vec<OptimizerSpec>,
}

impl CommandSpecs {
    /// Load the Bash command specs from `cfg` (built-ins overlaid by user specs,
    /// shadowed by `name`), keeping those that bind a command and aren't disabled
    /// by a `rules` toggle — by spec `name` (e.g. `git-diff`) or command family
    /// (e.g. `git`). A disabled spec is dropped, so its command passes through.
    #[must_use]
    pub fn load(cfg: &Config) -> Self {
        let specs = merged_specs_for(Surface::Bash, &cfg.specs)
            .into_iter()
            .filter(|spec| spec.bind.cmd.is_some())
            .filter(|spec| {
                cfg.rule_enabled("command", &spec.name)
                    && spec
                        .bind
                        .cmd
                        .as_deref()
                        .is_none_or(|family| cfg.rule_enabled("command", family))
            })
            .collect();
        Self { specs }
    }

    /// The spec matching `argv0` and (optionally) `sub`. A spec listing
    /// sub-commands matches only those; one with none is a catch-all fallback.
    #[must_use]
    pub fn find(&self, argv0: &str, sub: Option<&str>) -> Option<&OptimizerSpec> {
        let mut catch_all = None;
        for spec in &self.specs {
            if spec.bind.cmd.as_deref() != Some(argv0) {
                continue;
            }
            if spec.bind.subcommands.is_empty() {
                catch_all = catch_all.or(Some(spec));
            } else if sub.is_some_and(|s| spec.bind.subcommands.iter().any(|x| x == s)) {
                return Some(spec);
            }
        }
        catch_all
    }

    /// The spec with this exact `name` (used to re-resolve a planned unit).
    #[must_use]
    pub fn by_name(&self, name: &str) -> Option<&OptimizerSpec> {
        self.specs.iter().find(|spec| spec.name == name)
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/command/command_specs.tests.rs"]
mod tests;
