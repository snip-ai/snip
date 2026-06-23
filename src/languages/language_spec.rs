//! A language the read optimizer can parse and compact.

use tree_sitter::Language;

/// A language the read optimizer can parse and compact. Built by named-field
/// literal in [`crate::languages::registry`] (so the booleans can't be transposed).
pub struct LanguageSpec {
    /// Stable language name (also the Family-C command key — see `ARCHITECTURE.md`).
    pub name: &'static str,
    /// File extensions that map to this language (without the dot).
    pub extensions: &'static [&'static str],
    /// The tree-sitter grammar constructor — `pub(crate)` so the registry can set
    /// it by field while callers go through [`LanguageSpec::grammar`].
    pub(crate) grammar: fn() -> Language,
    /// tree-sitter node kinds removed in soft mode (comments and doc comments).
    pub comment_kinds: &'static [&'static str],
    /// String/char-literal node kinds — copied **verbatim** during a collapse so
    /// medium/high never mangles string contents.
    pub string_kinds: &'static [&'static str],
    /// Brace/block node kinds that high mode collapses onto their header line and
    /// medium collapses single-statement instances of (empty for non-brace langs).
    pub block_kinds: &'static [&'static str],
    /// Whether the language can be safely single-lined (medium/high build a
    /// collapsed view + origin map); false langs fall back to whitespace-only.
    pub is_single_line_safe: bool,
    /// Whether indentation is structural (Python) — drives the whitespace path.
    pub indent_based: bool,
}

impl LanguageSpec {
    /// The tree-sitter [`Language`] for this spec.
    #[must_use]
    pub fn grammar(&self) -> Language {
        (self.grammar)()
    }

    /// Whether `kind` is one of this language's comment node kinds.
    #[must_use]
    pub fn is_comment(&self, kind: &str) -> bool {
        self.comment_kinds.contains(&kind)
    }

    /// Whether `kind` is one of this language's string/char node kinds.
    #[must_use]
    pub fn is_string(&self, kind: &str) -> bool {
        self.string_kinds.contains(&kind)
    }
}
