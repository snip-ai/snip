//! `snip uninstall` — tear down snip's on-disk state and stop auto-reinstall.
//!
//! Removes snip's data dir (config, stats, session cache), the managed binary,
//! and the opt-in PATH line, then leaves a single marker so the next
//! `SessionStart` does not silently reinstall before the plugin is removed (the
//! plugin wiring is Claude Code's to remove: `/plugin uninstall snip@snip`).

use std::fs;
use std::path::Path;

use crate::commands::shell_path;

/// Marker left in the (otherwise emptied) data dir so `snip-run.sh` does not
/// auto-bootstrap a fresh binary before the user removes the plugin.
/// KEEP IN SYNC with `plugins/snip/scripts/snip-run.sh`.
const UNINSTALL_MARKER: &str = ".uninstalled";

/// Run `snip uninstall`: strip the PATH line, wipe snip's state and binary, and
/// print how to finish by removing the plugin.
///
/// # Errors
/// Never fails the command: every step is best-effort and reported; the result
/// type is kept for a uniform command signature.
#[allow(clippy::unnecessary_wraps)] // uniform command signature; best-effort steps never abort
pub fn run() -> anyhow::Result<()> {
    println!("snip uninstall:");
    let stripped = shell_path::strip_path_from_rcs(dirs::home_dir().as_deref());
    for rc in &stripped {
        println!("  removed PATH line from {}", rc.display());
    }
    if stripped.is_empty() {
        println!("  no PATH line found (nothing to remove)");
    }

    match crate::paths::data_dir() {
        Some(dir) if dir.is_dir() => {
            // Marker first, so a crash mid-purge still blocks an auto-reinstall.
            let _ = fs::write(dir.join(UNINSTALL_MARKER), b"");
            purge_state(&dir);
            remove_binary(&dir);
            println!("  removed snip state under {}", dir.display());
        }
        _ => println!("  no data dir found (nothing to remove)"),
    }

    println!("\nTo finish removing snip, remove the plugin in Claude Code:");
    println!("  /plugin uninstall snip@snip");
    println!("Then open a new shell. Thanks for trying snip.");
    Ok(())
}

/// Remove every entry under `data_dir` except the uninstall marker and `bin/`
/// (the running binary, handled separately by [`remove_binary`]).
fn purge_state(data_dir: &Path) {
    let Ok(entries) = fs::read_dir(data_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let keep = name
            .to_str()
            .is_some_and(|n| n == UNINSTALL_MARKER || n == "bin");
        if keep {
            continue;
        }
        let path = entry.path();
        let _ = if path.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
    }
}

#[cfg(not(windows))]
/// Unlink the running binary inline — Unix keeps the open inode alive.
fn remove_binary(data_dir: &Path) {
    let _ = fs::remove_dir_all(data_dir.join("bin"));
}

#[cfg(windows)]
#[allow(clippy::missing_const_for_fn)] // signature parity with the Unix arm (which can't be const)
/// No-op on Windows: a native `.exe` cannot delete its own running file, and a
/// detached shell it spawns does not survive this process exiting (verified). The
/// git-bash `snip-uninstall.sh` wrapper — which outlives the binary it runs —
/// removes `bin/` instead, keeping every command on git bash.
fn remove_binary(_data_dir: &Path) {}

#[cfg(test)]
#[path = "../../tests/unit/commands/uninstall.tests.rs"]
mod tests;
