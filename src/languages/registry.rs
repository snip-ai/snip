//! The built-in language registry: every [`LanguageSpec`] plus extension lookup.
//!
//! Adding a language is one [`SPECS`] entry plus its grammar crate. A wrong
//! `comment_kinds` is safe (nothing matches → the file passes through unchanged),
//! never corrupting code. `string_kinds`/`block_kinds`/`is_single_line_safe`/
//! `indent_based` drive the medium/high compaction modes.

use super::LanguageSpec;

/// Grammars that name every comment `comment`.
const COMMENT: &[&str] = &["comment"];
/// Grammars that split line vs block comments (Rust, Java, …).
const LINE_BLOCK: &[&str] = &["line_comment", "block_comment"];
/// No brace blocks (indent-based or non-single-line-safe languages).
const NO_BLOCK: &[&str] = &[];
/// The generic brace block node kind (Rust, Java, C#, CSS).
const BLOCK: &[&str] = &["block"];
/// The C-family brace block node kind.
const COMPOUND: &[&str] = &["compound_statement"];

/// All built-in language specs. First extension match wins in [`detect`].
const SPECS: &[LanguageSpec] = &[
    LanguageSpec {
        name: "rust",
        extensions: &["rs"],
        grammar: || tree_sitter_rust::LANGUAGE.into(),
        comment_kinds: LINE_BLOCK,
        string_kinds: &["string_literal", "raw_string_literal", "char_literal"],
        block_kinds: BLOCK,
        is_single_line_safe: true,
        indent_based: false,
    },
    LanguageSpec {
        name: "python",
        extensions: &["py", "pyi"],
        grammar: || tree_sitter_python::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &["string", "concatenated_string"],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: true,
    },
    LanguageSpec {
        name: "javascript",
        extensions: &["js", "mjs", "cjs", "jsx"],
        grammar: || tree_sitter_javascript::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &["string", "template_string"],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: false,
    },
    LanguageSpec {
        name: "typescript",
        extensions: &["ts", "mts", "cts"],
        grammar: || tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        comment_kinds: COMMENT,
        string_kinds: &["string", "template_string"],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: false,
    },
    LanguageSpec {
        name: "tsx",
        extensions: &["tsx"],
        grammar: || tree_sitter_typescript::LANGUAGE_TSX.into(),
        comment_kinds: COMMENT,
        string_kinds: &["string", "template_string"],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: false,
    },
    LanguageSpec {
        name: "go",
        extensions: &["go"],
        grammar: || tree_sitter_go::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &[
            "interpreted_string_literal",
            "raw_string_literal",
            "rune_literal",
        ],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: false,
    },
    LanguageSpec {
        name: "c",
        extensions: &["c", "h"],
        grammar: || tree_sitter_c::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &[
            "string_literal",
            "char_literal",
            "system_lib_string",
            "concatenated_string",
        ],
        block_kinds: COMPOUND,
        is_single_line_safe: true,
        indent_based: false,
    },
    LanguageSpec {
        name: "cpp",
        extensions: &["cpp", "cc", "cxx", "hpp", "hh", "hxx"],
        grammar: || tree_sitter_cpp::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &[
            "string_literal",
            "char_literal",
            "raw_string_literal",
            "system_lib_string",
            "concatenated_string",
        ],
        block_kinds: COMPOUND,
        is_single_line_safe: true,
        indent_based: false,
    },
    LanguageSpec {
        name: "java",
        extensions: &["java"],
        grammar: || tree_sitter_java::LANGUAGE.into(),
        comment_kinds: LINE_BLOCK,
        string_kinds: &["string_literal", "text_block", "character_literal"],
        block_kinds: BLOCK,
        is_single_line_safe: true,
        indent_based: false,
    },
    LanguageSpec {
        name: "ruby",
        extensions: &["rb"],
        grammar: || tree_sitter_ruby::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &[
            "string",
            "string_array",
            "bare_string",
            "chained_string",
            "heredoc_body",
            "regex",
            "character",
        ],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: false,
    },
    LanguageSpec {
        name: "bash",
        extensions: &["sh", "bash"],
        grammar: || tree_sitter_bash::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &[
            "string",
            "raw_string",
            "ansi_c_string",
            "translated_string",
            "heredoc_body",
        ],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: false,
    },
    LanguageSpec {
        name: "csharp",
        extensions: &["cs"],
        grammar: || tree_sitter_c_sharp::LANGUAGE.into(),
        comment_kinds: &["comment", "multiline_comment"],
        string_kinds: &[
            "string_literal",
            "verbatim_string_literal",
            "interpolated_string_expression",
            "interpolated_verbatim_string_expression",
        ],
        block_kinds: BLOCK,
        is_single_line_safe: true,
        indent_based: false,
    },
    LanguageSpec {
        name: "php",
        extensions: &["php"],
        grammar: || tree_sitter_php::LANGUAGE_PHP.into(),
        comment_kinds: COMMENT,
        string_kinds: &[
            "string",
            "encapsed_string",
            "heredoc",
            "nowdoc",
            "shell_command_expression",
        ],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: false,
    },
    LanguageSpec {
        name: "css",
        extensions: &["css", "scss"],
        grammar: || tree_sitter_css::LANGUAGE.into(),
        comment_kinds: &["comment", "js_comment"],
        string_kinds: &["string_value"],
        block_kinds: BLOCK,
        is_single_line_safe: true,
        indent_based: false,
    },
    LanguageSpec {
        name: "lua",
        extensions: &["lua"],
        grammar: || tree_sitter_lua::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &["string"],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: false,
    },
    LanguageSpec {
        name: "elixir",
        extensions: &["ex", "exs"],
        grammar: || tree_sitter_elixir::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &["string", "charlist", "char"],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: false,
    },
    LanguageSpec {
        name: "kotlin",
        extensions: &["kt", "kts"],
        grammar: || tree_sitter_kotlin_ng::LANGUAGE.into(),
        comment_kinds: LINE_BLOCK,
        string_kinds: &["string_literal", "multiline_string_literal"],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: false,
    },
    LanguageSpec {
        name: "scala",
        extensions: &["scala", "sc"],
        grammar: || tree_sitter_scala::LANGUAGE.into(),
        comment_kinds: &["comment", "block_comment"],
        string_kinds: &["string", "interpolated_string"],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: false,
    },
    LanguageSpec {
        name: "yaml",
        extensions: &["yaml", "yml"],
        grammar: || tree_sitter_yaml::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &["string_scalar"],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: true,
    },
    LanguageSpec {
        name: "toml",
        extensions: &["toml"],
        grammar: || tree_sitter_toml_ng::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &["string"],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: false,
    },
    LanguageSpec {
        name: "sql",
        extensions: &["sql"],
        grammar: || tree_sitter_sequel::LANGUAGE.into(),
        comment_kinds: &["comment", "marginalia"],
        string_kinds: &["literal"],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: false,
    },
    LanguageSpec {
        name: "html",
        extensions: &["html", "htm"],
        grammar: || tree_sitter_html::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &["quoted_attribute_value", "attribute_value"],
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: false,
    },
    LanguageSpec {
        name: "swift",
        extensions: &["swift"],
        grammar: || tree_sitter_swift::LANGUAGE.into(),
        comment_kinds: &["comment", "multiline_comment"],
        string_kinds: &[
            "line_string_literal",
            "multi_line_string_literal",
            "raw_string_literal",
        ],
        block_kinds: &["function_body"],
        is_single_line_safe: true,
        indent_based: false,
    },
    LanguageSpec {
        name: "dart",
        extensions: &["dart"],
        grammar: || tree_sitter_dart::LANGUAGE.into(),
        comment_kinds: &["comment", "block_comment", "documentation_block_comment"],
        string_kinds: &["string_literal", "symbol_literal"],
        block_kinds: &[
            "block",
            "class_body",
            "enum_body",
            "extension_body",
            "function_body",
        ],
        is_single_line_safe: true,
        indent_based: false,
    },
    LanguageSpec {
        name: "r",
        extensions: &["r", "R"],
        grammar: || tree_sitter_r::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &["string"],
        block_kinds: &["braced_expression"],
        is_single_line_safe: true,
        indent_based: false,
    },
    LanguageSpec {
        name: "zig",
        extensions: &["zig", "zon"],
        grammar: || tree_sitter_zig::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &["string", "multiline_string", "character"],
        block_kinds: BLOCK,
        is_single_line_safe: true,
        indent_based: false,
    },
    LanguageSpec {
        name: "julia",
        extensions: &["jl"],
        grammar: || tree_sitter_julia::LANGUAGE.into(),
        comment_kinds: &["line_comment", "block_comment"],
        string_kinds: &[
            "string_literal",
            "character_literal",
            "command_literal",
            "prefixed_string_literal",
            "prefixed_command_literal",
        ],
        block_kinds: &["block", "compound_statement"],
        is_single_line_safe: true,
        indent_based: false,
    },
    LanguageSpec {
        name: "haskell",
        extensions: &["hs", "hs-boot"],
        grammar: || tree_sitter_haskell::LANGUAGE.into(),
        // `pragma`/`cpp` nodes are significant (they affect compilation), so they
        // are NOT comments — only `comment`/`haddock` are stripped.
        comment_kinds: &["comment", "haddock"],
        string_kinds: &["string", "char", "quasiquote"],
        // Layout/indent-based: never single-lined (joining would corrupt layout).
        block_kinds: NO_BLOCK,
        is_single_line_safe: false,
        indent_based: true,
    },
    LanguageSpec {
        name: "objc",
        // `.h` stays with C (first-match); `.mm` is Objective-C++.
        extensions: &["m", "mm"],
        grammar: || tree_sitter_objc::LANGUAGE.into(),
        comment_kinds: COMMENT,
        string_kinds: &["string_literal", "char_literal", "concatenated_string"],
        block_kinds: COMPOUND,
        is_single_line_safe: true,
        indent_based: false,
    },
];

/// Find the language spec for a file path by its extension, if supported.
#[must_use]
pub fn detect(path: &str) -> Option<&'static LanguageSpec> {
    let ext = std::path::Path::new(path).extension()?.to_str()?;
    SPECS.iter().find(|l| l.extensions.contains(&ext))
}

#[cfg(test)]
#[path = "../../tests/unit/languages/registry.tests.rs"]
mod tests;
