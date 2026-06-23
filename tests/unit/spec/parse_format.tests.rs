//! Unit tests for [`ParseFormat`], in AAA form. Compiled into `snip_lib` via a
//! `#[path]` include in `src/spec/parse_format.rs`.

use assert2::check;

use super::ParseFormat;

fn lines(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn git_status_v2_compacts_to_the_short_form() {
    // Arrange: porcelain v2 --branch output (renamed entry uses a tab separator)
    let records = lines(&[
        "# branch.oid abc123",
        "# branch.head main",
        "# branch.ab +2 -1",
        "1 .M N... 100644 100644 100644 hH hI src/foo.rs",
        "1 A. N... 000000 100644 100644 hH hI src/new.rs",
        "2 R. N... 100644 100644 100644 hH hI R100 src/new_name.rs\tsrc/old_name.rs",
        "? untracked.txt",
        "! ignored.log",
    ]);

    // Act
    let out = ParseFormat::GitStatusV2.apply(&records);

    // Assert
    check!(
        out == lines(&[
            "on main +2 -1",
            ".M src/foo.rs",
            "A. src/new.rs",
            "R. src/old_name.rs -> src/new_name.rs",
            "?? untracked.txt",
            "!! ignored.log",
        ])
    );
}

#[test]
fn git_status_v2_renamed_entry_without_an_orig_path() {
    // Arrange: a `2 ` rename line whose path field carries no tab/orig-path —
    // the orig-is-empty arm emits just `XY path`
    let records = lines(&[
        "# branch.head main",
        "2 R. N... 100644 100644 100644 hH hI R100 src/file.rs",
    ]);

    // Act
    let out = ParseFormat::GitStatusV2.apply(&records);

    // Assert
    check!(out == lines(&["on main", "R. src/file.rs"]));
}

#[test]
fn clean_tree_collapses_to_just_the_branch() {
    // Arrange: no changes → only branch headers
    let records = lines(&["# branch.oid abc123", "# branch.head develop"]);

    // Act
    let out = ParseFormat::GitStatusV2.apply(&records);

    // Assert
    check!(out == lines(&["on develop"]));
}

#[test]
fn non_porcelain_lines_are_kept_verbatim() {
    // Arrange: if injection failed, never drop the real output
    let records = lines(&["fatal: not a git repository"]);

    // Act
    let out = ParseFormat::GitStatusV2.apply(&records);

    // Assert
    check!(out == lines(&["fatal: not a git repository"]));
}

#[test]
fn cargo_json_compacts_a_diagnostic_and_drops_noise() {
    // Arrange: one compiler-message plus artifact/finished noise to drop
    let records = lines(&[
        r#"{"reason":"compiler-artifact","target":{"name":"x"}}"#,
        r#"{"reason":"compiler-message","message":{"level":"error","message":"borrow of moved value","code":{"code":"E0382"},"spans":[{"file_name":"src/x.rs","line_start":12,"column_start":5,"is_primary":true}]}}"#,
        r#"{"reason":"build-finished","success":false}"#,
    ]);

    // Act
    let out = ParseFormat::CargoJson.apply(&records);

    // Assert: only the diagnostic survives, as one compact line
    check!(out == lines(&["error[E0382] src/x.rs:12:5: borrow of moved value"]));
}

#[test]
fn cargo_json_keeps_help_and_note_children() {
    // Arrange: a diagnostic whose actionable fix lives in its help/note children —
    // discarding them (as before) lost the suggestion the model needs.
    let records = lines(&[
        r#"{"reason":"compiler-message","message":{"level":"error","message":"mismatched types","code":{"code":"E0308"},"spans":[{"file_name":"src/x.rs","line_start":3,"column_start":9,"is_primary":true}],"children":[{"level":"note","message":"expected `&str`, found `String`"},{"level":"help","message":"consider borrowing here: `&x`"}]}}"#,
    ]);

    // Act
    let out = ParseFormat::CargoJson.apply(&records);

    // Assert: header line + one indented line per help/note child (the fix survives)
    check!(
        out == lines(&[
            "error[E0308] src/x.rs:3:9: mismatched types",
            "  note: expected `&str`, found `String`",
            "  help: consider borrowing here: `&x`",
        ])
    );
}

#[test]
fn cargo_json_keeps_non_json_lines_verbatim() {
    // Arrange: a stray non-JSON line must never be dropped
    let records = lines(&["error: could not compile `x`"]);

    // Act
    let out = ParseFormat::CargoJson.apply(&records);

    // Assert
    check!(out == lines(&["error: could not compile `x`"]));
}

#[test]
fn cargo_json_skips_blank_lines() {
    // Arrange: blank/whitespace-only lines are dropped; real output survives
    let records = lines(&["", "   ", "error: aborting due to 1 error"]);

    // Act
    let out = ParseFormat::CargoJson.apply(&records);

    // Assert: only the non-blank, non-JSON line is kept
    check!(out == lines(&["error: aborting due to 1 error"]));
}

#[test]
fn table_collapse_keeps_a_single_line_verbatim() {
    // Arrange: fewer than 2 records → nothing to compare down a column
    let records = lines(&["PID STAT CMD"]);

    // Act
    let out = ParseFormat::TableCollapse.apply(&records);

    // Assert
    check!(out == records);
}

#[test]
fn table_collapse_keeps_a_headerless_table_verbatim() {
    // Arrange: a blank header line splits into zero columns (ncol == 0)
    let records = lines(&["", "a b", "c d"]);

    // Act
    let out = ParseFormat::TableCollapse.apply(&records);

    // Assert
    check!(out == records);
}

#[test]
fn json_minify_strips_pretty_whitespace() {
    // Arrange
    let records = lines(&["{", "  \"a\": 1,", "  \"b\": [2, 3]", "}"]);

    // Act
    let out = ParseFormat::JsonMinify.apply(&records);

    // Assert
    check!(out == lines(&["{\"a\":1,\"b\":[2,3]}"]));
}

#[test]
fn json_minify_keeps_non_json_verbatim() {
    // Arrange
    let records = lines(&["not json", "at all"]);

    // Act
    let out = ParseFormat::JsonMinify.apply(&records);

    // Assert
    check!(out == records);
}

#[test]
fn json_array_table_encodes_uniform_objects() {
    // Arrange: a uniform array of objects (keys sorted — serde Map is a BTreeMap)
    let records = lines(&[
        "[",
        "  {\"id\": 1, \"ok\": true},",
        "  {\"id\": 2, \"ok\": false}",
        "]",
    ]);

    // Act
    let out = ParseFormat::JsonArrayTable.apply(&records);

    // Assert
    check!(out == lines(&["id,ok", "1,true", "2,false"]));
}

#[test]
fn json_array_table_renders_null_nested_and_quoted_cells() {
    // Arrange: keys are sorted (BTreeMap) — a=null, n=number, s=comma-string,
    // o=nested object. Exercises every cell arm plus CSV quoting.
    let records = lines(&[concat!(
        r#"[{"a":null,"n":7,"o":{"k":1},"s":"x,y"},"#,
        r#"{"a":null,"n":8,"o":{"k":2},"s":"plain"}]"#
    )]);

    // Act
    let out = ParseFormat::JsonArrayTable.apply(&records);

    // Assert: null→empty, number verbatim, nested object & comma-string quoted
    check!(
        out == lines(&[
            "a,n,o,s",
            r#",7,"{""k"":1}","x,y""#,
            r#",8,"{""k"":2}",plain"#,
        ])
    );
}

#[test]
fn json_array_table_bails_on_a_too_short_array() {
    // Arrange: a single-element array (< 2) → uniform_keys bails, kept verbatim
    let records = lines(&[r#"[{"a":1}]"#]);

    // Act
    let out = ParseFormat::JsonArrayTable.apply(&records);

    // Assert
    check!(out == records);
}

#[test]
fn json_array_table_bails_when_first_is_not_an_object() {
    // Arrange: a uniform array but the first element is a scalar, not an object
    let records = lines(&["[1, 2, 3]"]);

    // Act
    let out = ParseFormat::JsonArrayTable.apply(&records);

    // Assert: not a table of objects → verbatim
    check!(out == records);
}

#[test]
fn json_array_table_bails_on_ragged_records() {
    // Arrange: differing key sets → never guess
    let records = lines(&["[{\"a\":1},{\"a\":1,\"b\":2}]"]);

    // Act
    let out = ParseFormat::JsonArrayTable.apply(&records);

    // Assert
    check!(out == records);
}

#[test]
fn eslint_json_compacts_messages_with_severity_words() {
    // Arrange: one file, an error + a warning (eslint emits a single JSON array)
    let records = lines(&[concat!(
        r#"[{"filePath":"/p/a.js","messages":["#,
        r#"{"ruleId":"no-unused-vars","severity":2,"message":"x unused","line":3,"column":5},"#,
        r#"{"ruleId":"eqeqeq","severity":1,"message":"use ===","line":7,"column":9}"#,
        "]}]"
    )]);

    // Act
    let out = ParseFormat::EslintJson.apply(&records);

    // Assert: severity word (error/warning) is present so Rank can surface errors
    check!(
        out == lines(&[
            "/p/a.js:3:5: error no-unused-vars: x unused",
            "/p/a.js:7:9: warning eqeqeq: use ===",
        ])
    );
}

#[test]
fn eslint_json_clean_run_collapses_to_a_notice() {
    // Arrange: a file with no problems (and the empty-array case both apply)
    let records = lines(&[r#"[{"filePath":"/p/a.js","messages":[]}]"#]);

    // Act
    let out = ParseFormat::EslintJson.apply(&records);

    // Assert
    check!(out == lines(&["eslint: 0 problems"]));
}

#[test]
fn ruff_json_compacts_a_flat_diagnostic_array() {
    // Arrange: ruff emits a flat array of violations with a nested location
    let records = lines(&[concat!(
        r#"[{"code":"F401","message":"`os` imported but unused","#,
        r#""filename":"/p/x.py","location":{"row":1,"column":8}}]"#
    )]);

    // Act
    let out = ParseFormat::RuffJson.apply(&records);

    // Assert
    check!(out == lines(&["/p/x.py:1:8: F401 `os` imported but unused"]));
}

#[test]
fn ruff_json_clean_run_collapses_to_a_notice() {
    // Arrange: no violations
    let records = lines(&["[]"]);

    // Act
    let out = ParseFormat::RuffJson.apply(&records);

    // Assert
    check!(out == lines(&["ruff: 0 problems"]));
}

#[test]
fn ruff_json_keeps_non_array_output_verbatim() {
    // Arrange: injection failed / ruff errored → never drop the real output
    let records = lines(&["ruff failed: invalid config"]);

    // Act
    let out = ParseFormat::RuffJson.apply(&records);

    // Assert
    check!(out == records);
}

#[test]
fn eslint_json_skips_a_file_without_messages_and_notes_unknown_severity() {
    // Arrange: one file lacks a `messages` array (skipped); the other carries a
    // message with an unrecognized severity (0) → the `note` default
    let records = lines(&[concat!(
        r#"[{"filePath":"/p/a.js"},"#,
        r#"{"filePath":"/p/b.js","messages":[{"ruleId":"x","severity":0,"#,
        r#""message":"m","line":2,"column":3}]}]"#
    )]);

    // Act
    let out = ParseFormat::EslintJson.apply(&records);

    // Assert: only the second file emits, as a `note`
    check!(out == lines(&["/p/b.js:2:3: note x: m"]));
}

#[test]
fn eslint_json_keeps_non_array_output_verbatim() {
    // Arrange: injection failed / eslint crashed → never drop the real output
    let records = lines(&[
        "Oops! Something went wrong! :(",
        "ESLint couldn't find a config",
    ]);

    // Act
    let out = ParseFormat::EslintJson.apply(&records);

    // Assert
    check!(out == records);
}

#[test]
fn go_test_json_surfaces_a_failing_test_with_its_output() {
    // Arrange: run + captured output + fail for one (Package, Test)
    let records = lines(&[
        r#"{"Action":"run","Package":"p","Test":"TestX"}"#,
        r#"{"Action":"output","Package":"p","Test":"TestX","Output":"boom\n"}"#,
        r#"{"Action":"fail","Package":"p","Test":"TestX"}"#,
    ]);

    // Act
    let out = ParseFormat::GoTestJson.apply(&records);

    // Assert: failure block (header + indented output) precedes the tally
    check!(out == lines(&["FAIL p.TestX", "  boom", "p: 0 passed, 1 failed, 0 skipped"]));
}

#[test]
fn go_test_json_green_run_collapses_to_a_tally() {
    // Arrange: two passing tests + package-level banner noise (no failure)
    let records = lines(&[
        r#"{"Action":"output","Package":"p","Test":"TestA","Output":"=== RUN TestA\n"}"#,
        r#"{"Action":"pass","Package":"p","Test":"TestA"}"#,
        r#"{"Action":"pass","Package":"p","Test":"TestB"}"#,
        r#"{"Action":"output","Package":"p","Output":"ok  p  0.01s\n"}"#,
        r#"{"Action":"pass","Package":"p"}"#,
    ]);

    // Act
    let out = ParseFormat::GoTestJson.apply(&records);

    // Assert: all per-test/package output dropped; only the tally survives
    check!(out == lines(&["p: 2 passed, 0 failed, 0 skipped"]));
}

#[test]
fn go_test_json_surfaces_a_build_failure() {
    // Arrange: package-level output + a package-level fail (no per-test fail) —
    // a compile error must NOT be silently dropped
    let records = lines(&[
        r##"{"Action":"output","Package":"p","Output":"# p\n"}"##,
        r#"{"Action":"output","Package":"p","Output":"./x.go:3:2: undefined: y\n"}"#,
        r#"{"Action":"fail","Package":"p"}"#,
    ]);

    // Act
    let out = ParseFormat::GoTestJson.apply(&records);

    // Assert
    check!(
        out == lines(&[
            "FAIL p (build)",
            "  # p",
            "  ./x.go:3:2: undefined: y",
            "p: 0 passed, 0 failed, 0 skipped",
        ])
    );
}

#[test]
fn go_test_json_counts_skips_and_orders_packages() {
    // Arrange: a skip in one package, a pass in another (first-seen order)
    let records = lines(&[
        r#"{"Action":"skip","Package":"a","Test":"TestS"}"#,
        r#"{"Action":"pass","Package":"b","Test":"TestP"}"#,
    ]);

    // Act
    let out = ParseFormat::GoTestJson.apply(&records);

    // Assert: deterministic first-seen package order, skip counted
    check!(
        out == lines(&[
            "a: 0 passed, 0 failed, 1 skipped",
            "b: 1 passed, 0 failed, 0 skipped",
        ])
    );
}

#[test]
fn go_test_json_flushes_a_panicked_test_left_pending() {
    // Arrange: a blank line (skipped), an empty-Output event (no content to
    // buffer), then captured output for a test that never gets a terminal action
    let records = lines(&[
        "",
        r#"{"Action":"output","Package":"p","Test":"TestX","Output":"\n"}"#,
        r#"{"Action":"output","Package":"p","Test":"TestX","Output":"real log\n"}"#,
    ]);

    // Act
    let out = ParseFormat::GoTestJson.apply(&records);

    // Assert: the leftover pending test is surfaced as a FAIL, then the tally
    check!(
        out == lines(&[
            "FAIL p.TestX",
            "  real log",
            "p: 0 passed, 0 failed, 0 skipped",
        ])
    );
}

#[test]
fn go_test_json_keeps_non_json_verbatim() {
    // Arrange: a pre-JSON compile/toolchain error line → never dropped
    let records = lines(&["go: cannot find main module"]);

    // Act
    let out = ParseFormat::GoTestJson.apply(&records);

    // Assert
    check!(out == records);
}

#[test]
fn jest_json_surfaces_a_failed_assertion_after_the_tally() {
    // Arrange: one suite, one failed assertion (jest emits a single JSON object)
    let records = lines(&[concat!(
        r#"{"numFailedTests":1,"numPassedTests":0,"numTotalTests":1,"testResults":[{"#,
        r#""name":"/p/sum.test.js","status":"failed","assertionResults":[{"#,
        r#""status":"failed","fullName":"sum adds","#,
        r#""failureMessages":["Error: expect(received).toBe(expected)\n  Expected: 3"]}]}]}"#
    )]);

    // Act
    let out = ParseFormat::JestJson.apply(&records);

    // Assert: tally first, then the failure as one compact line
    check!(
        out == lines(&[
            "jest: 1 failed, 0 passed, 1 total",
            "/p/sum.test.js: sum adds — Error: expect(received).toBe(expected)",
        ])
    );
}

#[test]
fn jest_json_clean_run_collapses_to_the_tally() {
    // Arrange: all passing → only the tally survives
    let records = lines(&[concat!(
        r#"{"numFailedTests":0,"numPassedTests":2,"numTotalTests":2,"testResults":[{"#,
        r#""name":"/p/a.test.js","status":"passed","assertionResults":[{"#,
        r#""status":"passed","title":"a"},{"status":"passed","title":"b"}]}]}"#
    )]);

    // Act
    let out = ParseFormat::JestJson.apply(&records);

    // Assert
    check!(out == lines(&["jest: 0 failed, 2 passed, 2 total"]));
}

#[test]
fn jest_json_surfaces_a_suite_load_failure() {
    // Arrange: a suite that fails to import (status failed, no assertions ran)
    let records = lines(&[concat!(
        r#"{"numFailedTests":0,"numPassedTests":0,"numTotalTests":0,"testResults":[{"#,
        r#""name":"/p/broken.test.js","status":"failed","#,
        r#""message":"Cannot find module './x'\n  at Resolver","assertionResults":[]}]}"#
    )]);

    // Act
    let out = ParseFormat::JestJson.apply(&records);

    // Assert: the load failure is surfaced, not hidden behind a 0-failed tally
    check!(
        out == lines(&[
            "jest: 0 failed, 0 passed, 0 total",
            "/p/broken.test.js: <suite failed> — Cannot find module './x'",
        ])
    );
}

#[test]
fn table_collapse_drops_constant_columns_in_index_order() {
    // Arrange: STAT and TTY are constant down every row
    let records = lines(&[
        "PID STAT TTY CMD",
        "1 Ss ? init",
        "2 Ss ? bash",
        "3 Ss ? top",
    ]);

    // Act
    let out = ParseFormat::TableCollapse.apply(&records);

    // Assert: note lists dropped columns by ascending index; kept table follows
    check!(
        out == lines(&[
            "[const STAT=Ss TTY=?]",
            "PID CMD",
            "1 init",
            "2 bash",
            "3 top",
        ])
    );
}

#[test]
fn table_collapse_keeps_ragged_input_verbatim() {
    // Arrange: a row with a space-bearing cell over-splits → token count differs
    let records = lines(&["CONTAINER IMAGE STATUS", "a img up 2 hours", "b img exited"]);

    // Act
    let out = ParseFormat::TableCollapse.apply(&records);

    // Assert: never mis-align — kept verbatim
    check!(out == records);
}

#[test]
fn table_collapse_bails_when_nothing_is_constant() {
    // Arrange: every column varies
    let records = lines(&["NAME AGE", "a 1", "b 2"]);

    // Act
    let out = ParseFormat::TableCollapse.apply(&records);

    // Assert
    check!(out == records);
}

#[test]
fn table_collapse_refuses_to_drop_all_columns() {
    // Arrange: the only column is constant → would leave an empty table
    let records = lines(&["STATE", "up", "up"]);

    // Act
    let out = ParseFormat::TableCollapse.apply(&records);

    // Assert
    check!(out == records);
}

#[test]
fn table_collapse_keeps_a_blank_data_row_verbatim() {
    // Arrange: a blank line among the data → not a clean table
    let records = lines(&["NAME ST", "a up", "", "b up"]);

    // Act
    let out = ParseFormat::TableCollapse.apply(&records);

    // Assert
    check!(out == records);
}

#[test]
fn table_collapse_cannot_distinguish_prose_from_a_table() {
    // Arrange: a documented limitation — equal-word-count lines with a constant
    // 2nd column "collapse" like a table. The no-inflation guard + subcommand
    // scoping (kubectl get / docker ps) keep this from mattering in practice.
    let records = lines(&["hello world", "foo world", "bar world"]);

    // Act
    let out = ParseFormat::TableCollapse.apply(&records);

    // Assert
    check!(out == lines(&["[const world=world]", "hello", "foo", "bar"]));
}

#[test]
fn jest_json_reports_pending_and_a_messageless_failure() {
    // Arrange: a pending count (tally suffix) plus a failed assertion that
    // carries no failureMessages (join_msg's empty-message arm)
    let records = lines(&[concat!(
        r#"{"numFailedTests":1,"numPassedTests":0,"numTotalTests":2,"numPendingTests":1,"#,
        r#""testResults":[{"name":"/p/a.test.js","status":"failed","#,
        r#""assertionResults":[{"status":"failed","fullName":"a fails"}]}]}"#
    )]);

    // Act
    let out = ParseFormat::JestJson.apply(&records);

    // Assert: tally carries the pending suffix; the failure has no `— message`
    check!(
        out == lines(&[
            "jest: 1 failed, 0 passed, 2 total, 1 pending",
            "/p/a.test.js: a fails",
        ])
    );
}

#[test]
fn jest_json_keeps_non_object_output_verbatim() {
    // Arrange: a crash line (non-JSON) and a JSON array (wrong shape) both bail
    let non_json = lines(&["Cannot find module 'jest'"]);
    let array = lines(&["[1,2,3]"]);

    // Act
    let out_non_json = ParseFormat::JestJson.apply(&non_json);
    let out_array = ParseFormat::JestJson.apply(&array);

    // Assert
    check!(out_non_json == non_json);
    check!(out_array == array);
}
