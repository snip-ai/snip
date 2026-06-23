//! The `read` optimizer's Edit and Write surface handlers.
//!
//! Split out of [`super::read_optimizer`] (which keeps the trait impl + the Read
//! surface) to stay within the file-size limit. `Edit` maps a compacted
//! `old_string` back to real bytes (re-expanding a collapsed `new_string`);
//! `Write` asks before reproducing a file's own stripped view.

use serde_json::Value;

use super::read_optimizer::file_path;
use super::{correct, write_guard};
use crate::compaction::reexpand::reexpand;
use crate::config::CompactMode;
use crate::domain::{HookCtx, Outcome};
use crate::languages;

/// `Edit`: map a compacted `old_string` back to real bytes; re-expand a collapsed
/// `new_string` for medium/high single-line-safe languages.
pub(super) fn apply_edit(ctx: &HookCtx<'_>) -> Outcome {
    let (Some(path), Some(old)) = (file_path(ctx), str_field(ctx, "old_string")) else {
        return Outcome::PassThrough;
    };
    let Ok(file) = std::fs::read_to_string(path) else {
        return Outcome::PassThrough;
    };
    if file.contains(old) {
        return Outcome::PassThrough; // already matches verbatim
    }
    let mode = ctx.cfg.mode_for("read");
    let Some(corrected) = correct::correct_old_string(path, &file, old, mode) else {
        return Outcome::PassThrough;
    };
    let mut updated = ctx.input.clone();
    if let Some(obj) = updated.as_object_mut() {
        obj.insert("old_string".to_owned(), Value::String(corrected));
        if mode != CompactMode::Soft
            && let (Some(spec), Some(new)) = (languages::detect(path), str_field(ctx, "new_string"))
            && spec.is_single_line_safe
        {
            obj.insert("new_string".to_owned(), Value::String(reexpand(spec, new)));
        }
    }
    Outcome::FixInput(updated)
}

/// `Write`: ask before overwriting a file with its own compacted (stripped) view.
pub(super) fn apply_write(ctx: &HookCtx<'_>) -> Outcome {
    let (Some(path), Some(content)) = (file_path(ctx), str_field(ctx, "content")) else {
        return Outcome::PassThrough;
    };
    let Some(spec) = languages::detect(path) else {
        return Outcome::PassThrough;
    };
    let Ok(existing) = std::fs::read_to_string(path) else {
        return Outcome::PassThrough; // new file → nothing to lose
    };
    write_guard::should_ask(spec, &existing, content)
        .map_or(Outcome::PassThrough, |reason| Outcome::Ask { reason })
}

/// A named string field of the tool input.
fn str_field<'a>(ctx: &HookCtx<'a>, key: &str) -> Option<&'a str> {
    ctx.input.get(key).and_then(Value::as_str)
}
