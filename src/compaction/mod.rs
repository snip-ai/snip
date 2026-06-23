//! The `read` optimizer's compaction engine.
//!
//! [`Compactor`] strips comments (soft) and collapses code (medium/high) with an
//! origin map; [`code_lines`] gives each line's comment residue. Token sizing is
//! the cross-cutting [`crate::tokens`] leaf, not part of this read engine.

pub mod code_lines;
pub mod collapse;
pub mod compactor;
mod parse;
pub mod reexpand;
pub mod single_line;
pub mod splice;
pub mod whitespace;

pub use code_lines::code_lines;
pub use collapse::collapse_ranges_for;
pub use compactor::Compactor;
pub use reexpand::reexpand;
pub use single_line::compact_collapse;
pub use splice::{full_line_expanded, splice_out};
pub use whitespace::compact_whitespace;
