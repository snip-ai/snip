//! Unit tests for [`file_units`] normalization, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/optimizers/read/unit.rs`.

use assert2::check;

use super::file_units;

#[test]
fn without_a_grammar_each_non_blank_line_is_a_trimmed_unit() {
    // Arrange: no LanguageSpec → the fallback treats every non-blank line as code
    let file = "  a\n\n  b\n";
    let lines: Vec<&str> = file.lines().collect();

    // Act
    let units = file_units(None, file, &lines, false);

    // Assert: the blank line is dropped; each code line is trimmed and spans itself
    check!(units.len() == 2);
    check!(units[0].text == "a" && units[0].first == 0 && units[0].last == 0);
    check!(units[1].text == "b" && units[1].first == 2 && units[1].last == 2);
}

#[test]
fn collapse_blocks_merges_a_single_statement_python_block() {
    // Arrange: medium/high on Python merges `def f():` + its one indented statement
    let file = "def f():\n    return 1\ng = 2\n";
    let lines: Vec<&str> = file.lines().collect();

    // Act
    let units = file_units(crate::languages::detect("a.py"), file, &lines, true);

    // Assert: the header+statement become one unit spanning both lines; `g = 2` stays
    check!(units.len() == 2);
    check!(units[0].text == "def f(): return 1" && units[0].first == 0 && units[0].last == 1);
    check!(units[1].text == "g = 2" && units[1].first == 2 && units[1].last == 2);
}
