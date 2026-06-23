//! Best-effort re-expansion of a single-line `new_string` back to multi-line.
//!
//! After a medium/high read, Claude may write an Edit whose `new_string` is also
//! single-line, which would flatten the edited region. This walks the parse and
//! re-emits with newlines/indentation at structural boundaries (`{`/`}`/`;`).
//! Safety: only a **clean** parse is touched; string/comment leaves are emitted
//! verbatim; the result must change nothing but whitespace and must re-parse
//! cleanly, else the input is returned unchanged (never corrupts content).

use tree_sitter::{Node, Parser, Tree};

use crate::languages::LanguageSpec;

/// Re-expand a single-line `code` fragment for `spec`'s language (best-effort).
#[must_use]
pub fn reexpand(spec: &LanguageSpec, code: &str) -> String {
    if !spec.is_single_line_safe || code.contains('\n') || !code.contains([';', '{']) {
        return code.to_owned();
    }
    let Some(tree) = parse(spec, code) else {
        return code.to_owned();
    };
    if tree.root_node().has_error() {
        return code.to_owned();
    }
    let expanded = emit(spec, code.as_bytes(), &tree);
    if non_whitespace(&expanded) != non_whitespace(code) {
        return code.to_owned();
    }
    match parse(spec, &expanded) {
        Some(t) if !t.root_node().has_error() => expanded,
        _ => code.to_owned(),
    }
}

fn parse(spec: &LanguageSpec, code: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(&spec.grammar()).ok()?;
    parser.parse(code, None)
}

/// The non-whitespace characters of `s` (used to assert only whitespace changed).
fn non_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Walk tokens, emitting them with structural newlines + indentation. String and
/// comment nodes are atomic (emitted verbatim, never descended).
fn emit(spec: &LanguageSpec, src: &[u8], tree: &Tree) -> String {
    let mut out = String::with_capacity(src.len() * 2);
    let mut depth: usize = 0;
    let mut cursor = tree.walk();
    let mut prev: Option<&str> = None;
    'outer: loop {
        let node = cursor.node();
        let kind = node.kind();
        let atomic = spec.is_string(kind) || spec.is_comment(kind);
        let is_leaf = node.child_count() == 0;
        if (atomic || is_leaf) && node.end_byte() > node.start_byte() {
            let text = std::str::from_utf8(&src[node.start_byte()..node.end_byte()]).unwrap_or("");
            emit_token(&mut out, &mut depth, prev, text, node);
            prev = Some(text);
        }
        if !atomic && cursor.goto_first_child() {
            continue;
        }
        if cursor.goto_next_sibling() {
            continue;
        }
        loop {
            if !cursor.goto_parent() {
                break 'outer;
            }
            if cursor.goto_next_sibling() {
                continue 'outer;
            }
        }
    }
    out.push('\n');
    out
}

/// A `;` separating array elements (`[x; n]`) must not break the line; a
/// statement `;` must.
fn is_statement_semicolon(node: Node<'_>) -> bool {
    node.parent().is_none_or(|p| p.kind() != "array_expression")
}

fn emit_token(out: &mut String, depth: &mut usize, prev: Option<&str>, text: &str, node: Node<'_>) {
    let newline = |out: &mut String, depth: usize| {
        while out.ends_with(' ') {
            out.pop();
        }
        out.push('\n');
        for _ in 0..depth {
            out.push_str("    ");
        }
    };
    match text {
        "}" => {
            *depth = depth.saturating_sub(1);
            if !out.is_empty() {
                newline(out, *depth);
            }
            out.push('}');
        }
        "{" => {
            if needs_space(prev, "{") {
                out.push(' ');
            }
            out.push('{');
            *depth += 1;
            newline(out, *depth);
        }
        ";" if is_statement_semicolon(node) => {
            while out.ends_with(' ') {
                out.pop();
            }
            out.push(';');
            newline(out, *depth);
        }
        _ => {
            if prev == Some("}") && !matches!(text, ";" | "," | "." | ")" | "]" | "?") {
                newline(out, *depth);
            } else if needs_space(prev, text) && !out.ends_with(char::is_whitespace) {
                out.push(' ');
            }
            out.push_str(text);
        }
    }
}

/// Whether a separating space is needed between single tokens `prev` and `cur`.
fn needs_space(prev: Option<&str>, cur: &str) -> bool {
    let Some(prev) = prev else { return false };
    if prev.ends_with('\n') {
        return false;
    }
    let no_space_after = matches!(prev, "(" | "[" | "." | "::" | "!" | "#" | "&");
    let no_space_before = matches!(
        cur,
        ")" | "]" | "," | ";" | ":" | "." | "::" | "?" | "(" | "!"
    );
    !(no_space_after || no_space_before)
}

#[cfg(test)]
#[path = "../../tests/unit/compaction/reexpand.tests.rs"]
mod tests;
