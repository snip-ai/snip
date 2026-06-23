//! The sentinel rewrite plan for one command line.
//!
//! Which units to wrap, the wrapped command, and the per-unit recognized spec
//! (in marker order). Built identically by `bash-route` (to decide whether to
//! rewrite) and `exec` (to run + slice). `None` bails — the line runs verbatim.

use super::{CommandSpecs, Segmenter, Unit, recognition};
use crate::spec::OptimizerSpec;

/// A sentinel-wrapped execution plan for one command line.
pub struct Plan {
    /// High-entropy marker passed to the shell via `$SNIP_M` (never inlined).
    pub token: String,
    /// The command with each non-blank unit wrapped `{ printf "$SNIP_M"; …; }`.
    pub wrapped: String,
    /// The recognized spec name per wrapped unit, in marker order (`None` = leave
    /// the slice verbatim).
    pub recognized: Vec<Option<String>>,
}

impl Plan {
    /// Build the plan, or `None` to bail (segmenter bailed, or a unit is
    /// interactive/streaming/non-POSIX and must never be wrapped).
    #[must_use]
    pub fn build(cmd: &str, specs: &CommandSpecs) -> Option<Self> {
        let segments = Segmenter::split(cmd)?;
        let units = Unit::build(&segments);
        let mut wrapped = String::new();
        let mut recognized = Vec::new();
        let mut cursor = 0;
        for unit in &units {
            if unit.is_blank(cmd) {
                continue;
            }
            let matched = match recognition::parse(unit.last_text(cmd)) {
                Some((argv0, sub)) => {
                    if recognition::is_blocking(&argv0) {
                        return None; // never wrap interactive/streaming/non-POSIX
                    }
                    if unit.redirects_stdout(cmd) {
                        None
                    } else {
                        specs.find(&argv0, sub.as_deref())
                    }
                }
                None => None, // pure assignment / empty stage → verbatim
            };
            wrapped.push_str(&cmd[cursor..unit.start]);
            wrapped.push_str("{ printf '%s' \"$SNIP_M\"; ");
            // Inject structured flags (e.g. `--porcelain=v2`) right AFTER the
            // recognized command + sub-command prefix — tools reject options placed
            // after positional args (`git diff <pathspec> --no-color` fails), so the
            // flags must precede them. Splices into the matched (last pipe) stage;
            // falls back to the verbatim unit when there's nothing to inject or the
            // prefix can't be located (never risk breaking the command).
            match inject_at(cmd, unit, matched) {
                Some((pos, flags)) => {
                    wrapped.push_str(&cmd[unit.start..pos]);
                    for flag in flags {
                        wrapped.push(' ');
                        wrapped.push_str(flag);
                    }
                    wrapped.push_str(&cmd[pos..unit.end]);
                }
                None => wrapped.push_str(unit.text(cmd)),
            }
            wrapped.push_str(" ; }");
            cursor = unit.end;
            recognized.push(matched.map(|s| s.name.clone()));
        }
        wrapped.push_str(&cmd[cursor..]);
        Some(Self {
            token: make_token(),
            wrapped,
            recognized,
        })
    }

    /// Whether any wrapped unit resolved to a spec worth optimizing.
    #[must_use]
    pub fn has_recognized(&self) -> bool {
        self.recognized.iter().any(Option::is_some)
    }

    /// Cheap "is this line safe to wrap?" check that needs **no spec catalog** —
    /// it segments the line and rejects only what [`Plan::build`] would also
    /// reject (a segmenter bail, or an interactive/streaming/non-POSIX command).
    ///
    /// `bash-route` uses this on its default (auto-detect-on) path so the spec
    /// catalog is parsed once (in `exec`), not twice.
    #[must_use]
    pub fn wrappable(cmd: &str) -> bool {
        let Some(segments) = Segmenter::split(cmd) else {
            return false;
        };
        let units = Unit::build(&segments);
        let non_blank: Vec<&Unit> = units.iter().filter(|u| !u.is_blank(cmd)).collect();
        if non_blank.is_empty() {
            return false;
        }
        !non_blank.iter().any(|u| {
            recognition::parse(u.last_text(cmd))
                .is_some_and(|(argv0, _)| recognition::is_blocking(&argv0))
        })
    }
}

/// The absolute byte offset at which to splice a recognized unit's `inject_flags`,
/// paired with the flags — or `None` when the unit isn't recognized, carries no
/// flags, or the command prefix can't be located (the unit then runs verbatim).
fn inject_at<'a>(
    cmd: &str,
    unit: &Unit,
    matched: Option<&'a OptimizerSpec>,
) -> Option<(usize, &'a [String])> {
    let spec = matched?;
    if spec.inject_flags.is_empty() {
        return None;
    }
    let with_sub = !spec.bind.subcommands.is_empty();
    let offset = recognition::inject_offset(unit.last_text(cmd), with_sub)?;
    Some((unit.last.0 + offset, &spec.inject_flags))
}

/// A high-entropy, per-invocation marker (pid + nanos), bracketed by SOH bytes so
/// a collision with real command output is astronomically improbable. On the rare
/// collision, slicing detects the marker-count mismatch and falls back to verbatim.
fn make_token() -> String {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!("\u{1}SNIP{pid:x}-{nanos:x}\u{1}")
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/command/plan.tests.rs"]
mod tests;
