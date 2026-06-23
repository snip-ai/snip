//! Unit tests for the [`GroupKey`] grouping transform, in AAA form. Compiled
//! into `snip_lib` via a `#[path]` include in `src/spec/group_key.rs`.

use assert2::check;

use super::GroupKey;

fn lines(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn file_groups_consecutive_matches_under_one_path_header() {
    // Arrange: two matches in a.rs, one in b.rs
    let records = lines(&["a.rs:1:x", "a.rs:2:y", "b.rs:3:z"]);

    // Act
    let out = GroupKey::File.group(records);

    // Assert: a.rs collapses to a header + members; the lone b.rs stays verbatim
    check!(out == lines(&["a.rs:", "  1:x", "  2:y", "b.rs:3:z"]));
}

#[test]
fn file_split_survives_a_windows_drive_letter() {
    // Arrange: a `C:` drive letter must not be mistaken for the path/line split
    let records = lines(&[r"C:\a\b.rs:1:x", r"C:\a\b.rs:2:y"]);

    // Act
    let out = GroupKey::File.group(records);

    // Assert
    check!(out == lines(&[r"C:\a\b.rs:", "  1:x", "  2:y"]));
}

#[test]
fn non_matching_lines_pass_through() {
    // Arrange: a line with no `:<digits>:` cannot be a grep match
    let records = lines(&["a banner line", "a.rs:1:x", "a.rs:2:y"]);

    // Act
    let out = GroupKey::File.group(records);

    // Assert
    check!(out == lines(&["a banner line", "a.rs:", "  1:x", "  2:y"]));
}

#[test]
fn dir_groups_paths_under_their_directory() {
    // Arrange: glob paths; two in src/, one in lib/
    let records = lines(&["src/a.rs", "src/b.rs", "lib/c.rs"]);

    // Act
    let out = GroupKey::Dir.group(records);

    // Assert
    check!(out == lines(&["src:", "  a.rs", "  b.rs", "lib/c.rs"]));
}

#[test]
fn auto_folds_a_bare_path_list_by_directory() {
    // Arrange: a Grep `files_with_matches` payload — a "Found N files" header then
    // bare paths sharing a deep prefix, with no `:line:` segment to group by file
    let records = lines(&[
        "Found 3 files",
        "/repo/src/a.rs",
        "/repo/src/b.rs",
        "/repo/src/c.rs",
    ]);

    // Act
    let out = GroupKey::Auto.group(records);

    // Assert: the header passes through; the shared directory collapses to one header
    check!(out == lines(&["Found 3 files", "/repo/src:", "  a.rs", "  b.rs", "  c.rs"]));
}

#[test]
fn auto_still_groups_content_matches_by_file() {
    // Arrange: a Grep `content`-mode payload — `path:line:content` lines, which
    // must keep folding by file (no regression from the dir-aware fallback)
    let records = lines(&["a.rs:1:x", "a.rs:2:y", "b.rs:3:z"]);

    // Act
    let out = GroupKey::Auto.group(records);

    // Assert: identical to `File` grouping — the `:line:` segment wins
    check!(out == lines(&["a.rs:", "  1:x", "  2:y", "b.rs:3:z"]));
}

#[test]
fn a_lone_match_is_never_grouped() {
    // Arrange: every key is unique → grouping would inflate, so keep verbatim
    let records = lines(&["a.rs:1:x", "b.rs:2:y"]);

    // Act
    let out = GroupKey::File.group(records);

    // Assert
    check!(out == lines(&["a.rs:1:x", "b.rs:2:y"]));
}
