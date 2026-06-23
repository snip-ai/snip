//! Grouping window for log fingerprinting.

use serde::{Deserialize, Serialize};

/// Over what span the `Fingerprint` transform groups masked-equal records.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FpWindow {
    /// Only collapse consecutive masked-equal lines (order-preserving).
    #[default]
    Consecutive,
    /// Collapse masked-equal lines across the whole output (first position kept).
    Whole,
}
