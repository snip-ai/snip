//! Per-language specs for the read optimizer (soft-mode comment stripping).
//!
//! [`LanguageSpec`] is the type; [`registry`] holds every built-in spec plus the
//! extension → spec lookup ([`detect`]). Adding a language is one registry entry
//! plus its grammar crate.

pub mod language_spec;
pub mod registry;

pub use language_spec::LanguageSpec;
pub use registry::detect;
