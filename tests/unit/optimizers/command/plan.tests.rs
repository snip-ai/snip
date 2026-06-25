//! Unit tests for the rewrite [`Plan`], in AAA form. Compiled into `snip_lib`
//! via a `#[path]` include in `src/commands/route/plan.rs`.

use assert2::check;

use super::Plan;
use crate::config::Config;
use crate::optimizers::command::CommandSpecs;

#[test]
fn recognized_command_is_wrapped_with_a_sentinel() {
    // Arrange
    let specs = CommandSpecs::load(&Config::default());

    // Act: `ps` is recognized and injects no flags — isolates the sentinel shape
    assert2::assert!(let Some(plan) = Plan::build("ps", &specs));

    // Assert
    check!(plan.has_recognized());
    check!(plan.recognized == vec![Some("ps".to_owned())]);
    check!(plan.wrapped.contains(r#"{ printf '%s' "$SNIP_M"; ps ; }"#));
    check!(plan.token.contains("SNIP"));
}

#[test]
fn recognized_command_gets_its_injected_flags_appended() {
    // Arrange: git-status injects porcelain v2 so the slice parses cleanly
    let specs = CommandSpecs::load(&Config::default());

    // Act
    assert2::assert!(let Some(plan) = Plan::build("git status", &specs));

    // Assert: flags land inside the wrapped unit, after the command
    check!(plan.recognized == vec![Some("git-status".to_owned())]);
    check!(
        plan.wrapped
            .contains("git status --porcelain=v2 --branch ; }")
    );
}

#[test]
fn injected_flags_precede_positional_args() {
    // Arrange: a sub-command WITH a positional pathspec — git rejects an option
    // placed after positional args (`git diff <pathspec> --no-color` is fatal),
    // so the injected flag must come before the pathspec.
    let specs = CommandSpecs::load(&Config::default());

    // Act
    assert2::assert!(let Some(plan) = Plan::build("git diff a.txt", &specs));

    // Assert: `--no-color` lands right after `git diff`, before `a.txt`
    check!(plan.recognized == vec![Some("git-diff".to_owned())]);
    check!(plan.wrapped.contains("git diff --no-color a.txt ; }"));
}

#[test]
fn injected_flags_precede_goals_for_a_no_subcommand_tool() {
    // Arrange: `mvn` binds no sub-command, so its flags must land right after argv0,
    // before the goals (`clean install`) — the placement fix holds for this shape too.
    let specs = CommandSpecs::load(&Config::default());

    // Act
    assert2::assert!(let Some(plan) = Plan::build("mvn clean install", &specs));

    // Assert
    check!(plan.recognized == vec![Some("maven".to_owned())]);
    check!(
        plan.wrapped
            .contains("mvn --no-transfer-progress --batch-mode clean install ; }")
    );
}

#[test]
fn mixes_recognized_and_verbatim_units_in_marker_order() {
    // Arrange
    let specs = CommandSpecs::load(&Config::default());

    // Act
    assert2::assert!(let Some(plan) = Plan::build("ls && frobnicate", &specs));

    // Assert: ls recognized, the unknown command left verbatim (still wrapped)
    check!(plan.recognized == vec![Some("ls".to_owned()), None]);
    check!(plan.has_recognized());
}

#[test]
fn unrecognized_only_line_wraps_nothing_worth_optimizing() {
    // Arrange
    let specs = CommandSpecs::load(&Config::default());

    // Act
    assert2::assert!(let Some(plan) = Plan::build("echo hello", &specs));

    // Assert: a plan exists but nothing is recognized → caller passes through
    check!(!plan.has_recognized());
}

#[test]
fn bails_on_blocking_command_or_unsafe_syntax() {
    // Arrange
    let specs = CommandSpecs::load(&Config::default());

    // Act + Assert: interactive command, background, heredoc → bail (None)
    check!(Plan::build("vim file.txt", &specs).is_none());
    check!(Plan::build("server &", &specs).is_none());
    check!(Plan::build("cat <<EOF", &specs).is_none());
}

#[test]
fn bails_for_capture_flags_msys_magic_device_redirects() {
    // Arrange + Act + Assert: /dev/std* and /dev/fd/N can't be reopened over the
    // capture pipe under MSYS, so they're unsafe to wrap on Windows only; /dev/null
    // and a plain command are always fine to wrap.
    check!(super::bails_for_capture("echo x >/dev/stderr") == cfg!(windows));
    check!(super::bails_for_capture("echo x 1>/dev/fd/2") == cfg!(windows));
    check!(!super::bails_for_capture("echo x >/dev/null"));
    check!(!super::bails_for_capture("echo hello"));
}

#[cfg(windows)]
#[test]
fn bails_on_a_magic_device_redirect_on_windows() {
    // Arrange: output written to /dev/stderr is lost over the capture pipe under
    // MSYS, so the whole line must run verbatim instead of being wrapped.
    let specs = CommandSpecs::load(&Config::default());

    // Act + Assert
    check!(Plan::build("echo progress >/dev/stderr; echo data", &specs).is_none());
    check!(!Plan::wrappable("echo progress >/dev/stderr | sort"));
}

#[test]
fn bails_when_a_blocking_command_is_upstream_in_a_pipe() {
    // Arrange: the interactive/streaming command is NOT the terminal stage — older
    // code only checked the terminal stage and would wrap (then hang under capture).
    let specs = CommandSpecs::load(&Config::default());

    // Act + Assert: every stage is checked now, so an upstream blocker bails
    check!(Plan::build("tail -f log | grep err", &specs).is_none());
    check!(Plan::build("vim file | cat", &specs).is_none());
    check!(Plan::build("watch ls | head", &specs).is_none());
    check!(!Plan::wrappable("tail -f log | grep err"));
    check!(!Plan::wrappable("less big.txt | head"));
}

#[test]
fn wraps_a_pipe_of_only_non_blocking_stages() {
    // Arrange: no stage is interactive/streaming, so the pipe is safe to wrap
    let specs = CommandSpecs::load(&Config::default());

    // Act + Assert
    check!(Plan::wrappable("cat file | grep x | sort"));
    assert2::assert!(let Some(_) = Plan::build("cat file | grep x", &specs));
}

#[test]
fn redirected_recognized_command_is_left_verbatim() {
    // Arrange: stdout goes to a file → nothing visible to optimize
    let specs = CommandSpecs::load(&Config::default());

    // Act
    assert2::assert!(let Some(plan) = Plan::build("ls > files.txt", &specs));

    // Assert
    check!(plan.recognized == vec![None]);
    check!(!plan.has_recognized());
}

#[test]
fn blank_unit_between_operators_is_skipped() {
    // Arrange: the empty stage between the two `;` is blank and must be skipped
    let specs = CommandSpecs::load(&Config::default());

    // Act
    assert2::assert!(let Some(plan) = Plan::build("echo a ;  ; echo b", &specs));

    // Assert: only the two real units are wrapped — the blank one yields no entry
    check!(plan.recognized.len() == 2);
    check!(plan.recognized == vec![None, None]);
}

#[test]
fn pure_assignment_unit_is_wrapped_but_unrecognized() {
    // Arrange: a bare `FOO=bar` parses to `None` (no command) → left verbatim
    let specs = CommandSpecs::load(&Config::default());

    // Act
    assert2::assert!(let Some(plan) = Plan::build("FOO=bar", &specs));

    // Assert: a single unit, recognized as nothing
    check!(plan.recognized == vec![None]);
    check!(!plan.has_recognized());
}

#[test]
fn wrappable_is_true_for_a_plain_command() {
    // Arrange + Act + Assert: a simple line segments cleanly and blocks nothing
    check!(Plan::wrappable("ls -la"));
}

#[test]
fn wrappable_is_false_when_the_segmenter_bails() {
    // Arrange + Act + Assert: an unterminated quote makes the segmenter bail
    check!(!Plan::wrappable("echo 'unterminated"));
}

#[test]
fn wrappable_is_false_for_an_all_blank_command() {
    // Arrange + Act + Assert: only whitespace → no non-blank unit to wrap
    check!(!Plan::wrappable("   "));
}
