//! `go test -json` → failing tests + a per-package pass/fail/skip tally.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

/// Compact `go test -json` (the `test2json` NDJSON stream) to failures + a tally.
///
/// Each line is one event `{Action, Package, Test, Output}`. Captured output is
/// buffered per test and surfaced only for failures; passing/skipped tests
/// collapse into per-package counts. A build/compile error (package-level output
/// plus a package `fail` with no per-test failure) is surfaced too, so nothing is
/// dropped. Any non-JSON line is kept verbatim. No regex.
#[must_use]
pub fn go_test_json(records: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut order: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut tally: HashMap<String, [usize; 3]> = HashMap::new();
    let mut pending: HashMap<(String, String), Vec<String>> = HashMap::new();
    let mut pkg_output: HashMap<String, Vec<String>> = HashMap::new();
    let mut build_fail: HashSet<String> = HashSet::new();
    let mut test_fail: HashSet<String> = HashSet::new();

    for line in records {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some((event, action)) = parse_event(trimmed) else {
            out.push(line.clone());
            continue;
        };
        let pkg = event.get("Package").and_then(Value::as_str).unwrap_or("?");
        let test = event
            .get("Test")
            .and_then(Value::as_str)
            .filter(|t| !t.is_empty());
        if !seen.contains(pkg) {
            seen.insert(pkg.to_owned());
            order.push(pkg.to_owned());
        }
        match action.as_str() {
            "output" => buffer_output(&event, pkg, test, &mut pending, &mut pkg_output),
            "pass" => {
                if let Some(t) = test {
                    tally.entry(pkg.to_owned()).or_insert([0; 3])[0] += 1;
                    pending.remove(&(pkg.to_owned(), t.to_owned()));
                }
            }
            "skip" => {
                if let Some(t) = test {
                    tally.entry(pkg.to_owned()).or_insert([0; 3])[2] += 1;
                    pending.remove(&(pkg.to_owned(), t.to_owned()));
                }
            }
            "fail" => match test {
                Some(t) => {
                    tally.entry(pkg.to_owned()).or_insert([0; 3])[1] += 1;
                    test_fail.insert(pkg.to_owned());
                    let body = pending.remove(&(pkg.to_owned(), t.to_owned()));
                    flush(&mut out, &format!("FAIL {pkg}.{t}"), body);
                }
                None => {
                    build_fail.insert(pkg.to_owned());
                }
            },
            _ => {}
        }
    }
    finish(
        out,
        &order,
        &tally,
        &mut pending,
        &pkg_output,
        &build_fail,
        &test_fail,
    )
}

/// Parse a test2json line into `(event, action)`; `None` ⇒ treat as non-JSON.
fn parse_event(line: &str) -> Option<(Value, String)> {
    let value = serde_json::from_str::<Value>(line).ok()?;
    let action = value.get("Action").and_then(Value::as_str)?.to_owned();
    Some((value, action))
}

/// Buffer an `output` event under its test (or its package for build errors).
fn buffer_output(
    event: &Value,
    pkg: &str,
    test: Option<&str>,
    pending: &mut HashMap<(String, String), Vec<String>>,
    pkg_output: &mut HashMap<String, Vec<String>>,
) {
    let content = event
        .get("Output")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim_end_matches('\n');
    if content.is_empty() {
        return;
    }
    match test {
        Some(t) => pending
            .entry((pkg.to_owned(), t.to_owned()))
            .or_default()
            .push(content.to_owned()),
        None => pkg_output
            .entry(pkg.to_owned())
            .or_default()
            .push(content.to_owned()),
    }
}

/// Push a `FAIL …` header then its captured output lines (2-space indented).
fn flush(out: &mut Vec<String>, header: &str, body: Option<Vec<String>>) {
    out.push(header.to_owned());
    for line in body.into_iter().flatten() {
        out.push(format!("  {line}"));
    }
}

/// Emit build-failures, any leftover (panicked) tests, then per-package tallies.
fn finish(
    mut out: Vec<String>,
    order: &[String],
    tally: &HashMap<String, [usize; 3]>,
    pending: &mut HashMap<(String, String), Vec<String>>,
    pkg_output: &HashMap<String, Vec<String>>,
    build_fail: &HashSet<String>,
    test_fail: &HashSet<String>,
) -> Vec<String> {
    for pkg in order {
        if build_fail.contains(pkg) && !test_fail.contains(pkg) {
            flush(
                &mut out,
                &format!("FAIL {pkg} (build)"),
                pkg_output.get(pkg).cloned(),
            );
        }
    }
    let mut leftover: Vec<(String, String)> = pending.keys().cloned().collect();
    leftover.sort();
    for key in leftover {
        let body = pending.remove(&key);
        flush(&mut out, &format!("FAIL {}.{}", key.0, key.1), body);
    }
    for pkg in order {
        let [p, f, s] = tally.get(pkg).copied().unwrap_or([0; 3]);
        out.push(format!("{pkg}: {p} passed, {f} failed, {s} skipped"));
    }
    out
}
