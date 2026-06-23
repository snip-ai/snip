//! Unit tests for [`compact_collapse`] (collapse + origin map), in AAA form.
//! Compiled into `snip_lib` via a `#[path]` include in
//! `src/compaction/single_line.rs`.

use assert2::check;

use super::compact_collapse;

#[test]
fn collapses_a_block_range_and_maps_origin_to_source() {
    // Arrange: "fn f() {\n  a;\n}" — collapse the block range, no comments/strings
    let src = b"fn f() {\n  a;\n}";

    // Act
    let (out, origin) = compact_collapse(src, &[], &[], &[(7, src.len())]);
    let view = String::from_utf8(out).unwrap();

    // Assert: interior whitespace collapses to single spaces, NO_SPACE rules apply
    check!(view == "fn f() { a;}");
    // The `a` in the view maps back to its original source byte (index 11).
    let a_pos = view.find('a').expect("a in view");
    check!(origin[a_pos] == 11);
}

#[test]
fn copies_a_string_range_verbatim() {
    // Arrange: a string literal spanning the collapse range must not be touched
    let src = b"x = { \"a  b\" }";
    // the quoted string "a  b" occupies bytes [6, 12) (quotes included)

    // Act: collapse everything, but mark [6,12) as a verbatim string
    let (out, _origin) = compact_collapse(src, &[], &[(6, 12)], &[(0, src.len())]);
    let view = String::from_utf8(out).unwrap();

    // Assert: the double-spaced string content survives verbatim
    check!(view.contains("\"a  b"));
}
