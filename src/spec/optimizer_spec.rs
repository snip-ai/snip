//! A declarative optimizer: matching rules + an ordered transform pipeline.

use serde::{Deserialize, Serialize};

use super::{Bind, Transform};
use crate::domain::Surface;

/// A declarative optimizer: matching rules + an ordered transform pipeline.
#[derive(Clone, Serialize, Deserialize)]
pub struct OptimizerSpec {
    /// Stable identifier (and config key).
    pub name: String,
    /// The surface this spec attaches to.
    pub surface: Surface,
    /// How a tool event binds to this spec.
    #[serde(default)]
    pub bind: Bind,
    /// The ordered transform pipeline applied to the output.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transforms: Vec<Transform>,
    /// Flags injected before execution (Bash surface only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inject_flags: Vec<String>,
}

impl OptimizerSpec {
    /// Authoring-time check that this spec can actually fire on its surface.
    ///
    /// Catches specs that would be silently inert: Read/Edit/Write are served by
    /// the Rust `read` optimizer (not spec-extensible); a Bash spec needs
    /// `bind.cmd`; a Grep/Glob spec must not carry `bind.cmd`/`subcommands`
    /// (those are ignored off the Bash surface).
    ///
    /// # Errors
    /// Returns a human-readable reason when the spec could never run.
    pub fn validate(&self) -> Result<(), String> {
        match self.surface {
            Surface::Read | Surface::Edit | Surface::Write => Err(format!(
                "surface `{}` is served by the built-in Rust `read` optimizer and is \
                 not spec-extensible",
                self.surface.name()
            )),
            Surface::Bash if self.bind.cmd.is_none() => {
                Err("a Bash spec must set `bind.cmd` (e.g. \"git\")".to_owned())
            }
            Surface::Bash if !self.bind.path_globs.is_empty() => {
                Err("`bind.path_globs` applies to Grep/Glob specs, not Bash".to_owned())
            }
            Surface::Grep | Surface::Glob
                if self.bind.cmd.is_some() || !self.bind.subcommands.is_empty() =>
            {
                Err("`bind.cmd`/`bind.subcommands` apply to Bash specs, not Grep/Glob".to_owned())
            }
            Surface::Bash | Surface::Grep | Surface::Glob => Ok(()),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/spec/optimizer_spec.tests.rs"]
mod tests;
