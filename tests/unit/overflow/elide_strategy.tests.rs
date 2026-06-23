//! Unit tests for [`ElideStrategy`] record-level elision, in AAA form. Compiled
//! into `snip_lib` via a `#[path]` include in `src/overflow/elide_strategy.rs`.

use assert2::check;

use super::ElideStrategy;

fn lines(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn head_keeps_the_start_and_marks_the_drop() {
    // Arrange: six 2-token records ("1".."6"); budget 4 ⇒ first two fit
    let records = lines(&["1", "2", "3", "4", "5", "6"]);

    // Act
    let out = ElideStrategy::Head.elide(&records, 4, 0.6);

    // Assert
    check!(out == lines(&["1", "2", "… (4 lines elided)"]));
}

#[test]
fn tail_keeps_the_end_and_marks_the_drop() {
    // Arrange
    let records = lines(&["1", "2", "3", "4", "5", "6"]);

    // Act
    let out = ElideStrategy::Tail.elide(&records, 4, 0.6);

    // Assert
    check!(out == lines(&["… (4 lines elided)", "5", "6"]));
}

#[test]
fn middle_keeps_both_ends_and_elides_the_gap() {
    // Arrange: budget 8, head_frac 0.5 ⇒ 4 tok each end ⇒ two records each side
    let records = lines(&["1", "2", "3", "4", "5", "6"]);

    // Act
    let out = ElideStrategy::Middle.elide(&records, 8, 0.5);

    // Assert
    check!(out == lines(&["1", "2", "… (2 lines elided)", "5", "6"]));
}

#[test]
fn at_least_one_record_survives_a_tiny_budget() {
    // Arrange: a budget smaller than any single record
    let records = lines(&["aaaaaaaa", "bbbbbbbb", "cccccccc"]);

    // Act
    let out = ElideStrategy::Head.elide(&records, 1, 0.6);

    // Assert: never elide everything — keep the first, mark the rest
    check!(out == lines(&["aaaaaaaa", "… (2 lines elided)"]));
}

#[test]
fn relevance_first_keeps_error_lines_over_budget() {
    // Arrange: only one line looks like an error; budget fits one 4-tok line
    let records = lines(&["noise one", "noise two", "error boom", "noise three"]);

    // Act
    let out = ElideStrategy::RelevanceFirst.elide(&records, 4, 0.6);

    // Assert: the error survives; everything else is elided
    check!(out == lines(&["error boom", "… (3 lines elided)"]));
}

#[test]
fn relevance_first_falls_back_to_middle_without_errors() {
    // Arrange: nothing relevant → behaves like Middle
    let records = lines(&["1", "2", "3", "4", "5", "6"]);

    // Act
    let out = ElideStrategy::RelevanceFirst.elide(&records, 8, 0.5);

    // Assert
    check!(out == lines(&["1", "2", "… (2 lines elided)", "5", "6"]));
}

#[test]
fn head_keeps_everything_when_it_fits() {
    // Arrange: a budget large enough for every record ⇒ no marker added
    let records = lines(&["1", "2", "3"]);

    // Act
    let out = ElideStrategy::Head.elide(&records, 1000, 0.6);

    // Assert: returned verbatim (take >= len branch)
    check!(out == records);
}

#[test]
fn tail_keeps_everything_when_it_fits() {
    // Arrange
    let records = lines(&["1", "2", "3"]);

    // Act
    let out = ElideStrategy::Tail.elide(&records, 1000, 0.6);

    // Assert
    check!(out == records);
}

#[test]
fn middle_keeps_everything_when_it_fits() {
    // Arrange: head + tail budgets together cover the whole input ⇒ no gap
    let records = lines(&["1", "2", "3"]);

    // Act
    let out = ElideStrategy::Middle.elide(&records, 1000, 0.5);

    // Assert: tail_start <= head_end branch returns the records unchanged
    check!(out == records);
}

#[test]
fn relevance_first_caps_the_kept_errors_to_budget() {
    // Arrange: two error lines but a budget that fits only the first
    let records = lines(&["error one", "noise", "error two", "noise"]);

    // Act
    let out = ElideStrategy::RelevanceFirst.elide(&records, 4, 0.6);

    // Assert: the loop breaks after the first error; the rest is one marker
    check!(out == lines(&["error one", "… (3 lines elided)"]));
}
