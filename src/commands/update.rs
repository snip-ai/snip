//! `snip update` — force a re-check against the latest release.
//!
//! A user-facing alias over the `update-check` hook with `force` set, so a person
//! can pull a fresh binary on demand instead of waiting for the once-a-day
//! `SessionStart` check. A newer binary (if any) lands in the background and is
//! active next session. The fetch needs the plugin's installer, so it only runs
//! inside a Claude Code session (where `CLAUDE_PLUGIN_ROOT` is set).

/// Force an update check and report what will happen.
///
/// # Errors
/// Propagates a strict-mode failure from the underlying hook; in production the
/// hook is infallible (exit-0 invariant).
pub fn run() -> anyhow::Result<()> {
    crate::hooks::update_check::run(true)?;
    if std::env::var("CLAUDE_PLUGIN_ROOT").is_ok_and(|v| !v.is_empty()) {
        println!(
            "snip: checked the latest release. A newer binary, if any, is fetched in the \
             background and is active next session."
        );
    } else {
        println!(
            "snip: updates flow through the plugin — run /snip update inside Claude Code to fetch one."
        );
    }
    Ok(())
}
