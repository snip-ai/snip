//! Unit tests for stdout [`assemble`]-ation, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/commands/route/assemble.rs`.

use assert2::check;

use super::assemble;
use crate::config::Config;
use crate::optimizers::command::{CommandSpecs, Plan};

const TOKEN: &str = "\u{1}MARK\u{1}";

fn plan(recognized: Vec<Option<String>>) -> Plan {
    Plan {
        token: TOKEN.to_owned(),
        wrapped: String::new(),
        recognized,
    }
}

fn many_lines(n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        s.push_str("file-");
        s.push_str(&i.to_string());
        s.push_str(".rs\n");
    }
    s
}

#[test]
fn recognized_slice_is_optimized_and_tagged() {
    // Arrange: one recognized `ls` unit whose long slice the spec compacts
    let specs = CommandSpecs::load(&Config::default());
    let cfg = Config::default();
    let body = many_lines(200);
    let captured = format!("{TOKEN}{body}");

    // Act
    let out = assemble(
        &captured,
        &plan(vec![Some("ls".to_owned())]),
        &specs,
        &cfg,
        None,
    );

    // Assert
    check!(out.contains("[snip: ls |"));
    check!(out.len() < captured.len());
}

#[test]
fn unrecognized_slice_is_kept_verbatim() {
    // Arrange
    let specs = CommandSpecs::load(&Config::default());
    let cfg = Config::default();
    let captured = format!("{TOKEN}raw output stays\n");

    // Act
    let out = assemble(&captured, &plan(vec![None]), &specs, &cfg, None);

    // Assert: marker stripped, content untouched
    check!(out == "raw output stays\n");
}

#[test]
fn marker_count_mismatch_falls_back_to_verbatim() {
    // Arrange: two markers but the plan expects one unit (a collision/early exit)
    let specs = CommandSpecs::load(&Config::default());
    let cfg = Config::default();
    let captured = format!("{TOKEN}a{TOKEN}b");

    // Act
    let out = assemble(
        &captured,
        &plan(vec![Some("ls".to_owned())]),
        &specs,
        &cfg,
        None,
    );

    // Assert: never corrupts — strips markers and returns the raw output
    check!(out == "ab");
}
