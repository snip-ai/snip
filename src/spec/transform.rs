//! The closed transform vocabulary applied to a spec's output.

use serde::{Deserialize, Serialize};

use super::{DiffFoldCfg, FingerprintCfg, GroupKey, ParseFormat, RankKey, StacktraceCfg, StrPred};

/// The closed transform vocabulary. Each variant is `O(n)` over records (lines
/// by default). No regex, no scripting — see the module docs.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Transform {
    /// Interpret a known machine format and re-emit compact lines.
    Parse {
        /// The input format to parse (e.g. git porcelain v2).
        format: ParseFormat,
    },
    /// Collapse consecutive identical records to `"<record> (×N)"`.
    Dedupe,
    /// Collapse a repeated path/dir prefix to one header + indented members.
    Group {
        /// What to group consecutive records by.
        by: GroupKey,
    },
    /// Keep only records satisfying `pred` (a regex-free predicate).
    Keep {
        /// The predicate a record must satisfy to be kept.
        pred: StrPred,
    },
    /// Drop records satisfying `pred`.
    Drop {
        /// The predicate that marks a record for removal.
        pred: StrPred,
    },
    /// Stably reorder records (e.g. errors first) so the most useful survive a
    /// later overflow cap.
    Rank {
        /// What to order records by.
        by: RankKey,
    },
    /// Re-emit each record through a compact template (the first `{}` is replaced
    /// by the record text).
    Template {
        /// The per-record template; its first `{}` becomes the record.
        each: String,
    },
    /// Strip ANSI escapes and `\r` overwrites from each record (lossless).
    StripAnsi,
    /// Collapse blank-line runs to one and trim trailing whitespace per record.
    Squeeze,
    /// Collapse near-identical log lines (masked-equal) to `<sample> (×N)`.
    Fingerprint(FingerprintCfg),
    /// Fold framework/runtime stack frames, keeping the app frames (stacktrace prune).
    FoldFrames(StacktraceCfg),
    /// Fold long runs of unchanged diff context to `… (N unchanged)`.
    FoldDiff(DiffFoldCfg),
    /// Collapse each consecutive run of passing-test lines (matching `pred`) to
    /// `… (N passed)`; failures and the summary pass through verbatim.
    TestRollup {
        /// The predicate marking a passing-test line to roll up.
        pred: StrPred,
    },
    /// Keep the first `head` and last `tail` records, eliding the middle.
    Truncate {
        /// Records kept from the start.
        head: usize,
        /// Records kept from the end.
        tail: usize,
    },
    /// Reproject each record to a chosen subset of its whitespace-split fields,
    /// joined by `sep` (e.g. keep columns 0 and 4 of `ps` output). Regex-free; a
    /// record with none of the requested columns is kept verbatim.
    Project {
        /// 0-based field indices to keep, in output order.
        cols: Vec<usize>,
        /// Separator joining the kept fields.
        #[serde(default = "default_sep")]
        sep: String,
    },
}

/// The default field separator for [`Transform::Project`] (a single space).
fn default_sep() -> String {
    " ".to_owned()
}

impl Transform {
    /// Apply this transform to an ordered list of records, returning the result.
    ///
    /// Each arm dispatches to the transform's own module (`super::<name>`) or, for
    /// the variants parameterized by a vocabulary type, to a method on that type.
    #[must_use]
    pub fn apply(&self, records: Vec<String>) -> Vec<String> {
        match self {
            Self::Parse { format } => format.apply(&records),
            Self::Dedupe => super::dedupe::dedupe(records),
            Self::Group { by } => by.group(records),
            Self::Keep { pred } => records.into_iter().filter(|r| pred.matches(r)).collect(),
            Self::Drop { pred } => records.into_iter().filter(|r| !pred.matches(r)).collect(),
            Self::Rank { by } => by.rank(records),
            Self::Template { each } => super::template::template(records, each),
            Self::StripAnsi => super::ansi::strip_ansi(records),
            Self::Squeeze => super::squeeze::squeeze(records),
            Self::Fingerprint(cfg) => super::log_fold::fingerprint(records, cfg),
            Self::FoldFrames(cfg) => super::stacktrace::fold_frames(records, cfg),
            Self::FoldDiff(cfg) => super::diff_fold::fold_diff(records, cfg),
            Self::TestRollup { pred } => super::test_rollup::test_rollup(records, pred),
            Self::Truncate { head, tail } => super::truncate::truncate(records, *head, *tail),
            Self::Project { cols, sep } => super::project::project(records, cols, sep),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/spec/transform.tests.rs"]
mod tests;
