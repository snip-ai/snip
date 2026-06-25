//! Unit tests for command [`recognition`], in AAA form. Compiled into `snip_lib`
//! via a `#[path]` include in `src/commands/route/recognition.rs`.

use assert2::check;

use super::{inject_offset, is_blocking, parse};

#[test]
fn parses_argv0_and_subcommand() {
    // Arrange + Act + Assert
    check!(parse("git status") == Some(("git".to_owned(), Some("status".to_owned()))));
    check!(parse("ls") == Some(("ls".to_owned(), None)));
}

#[test]
fn skips_env_assignments_and_strips_path_and_exe() {
    // Arrange + Act
    let envd = parse("FOO=bar BAZ=1 git commit");
    let pathed = parse("/usr/bin/git.exe push");

    // Assert
    check!(envd == Some(("git".to_owned(), Some("commit".to_owned()))));
    check!(pathed == Some(("git".to_owned(), Some("push".to_owned()))));
}

#[test]
fn subcommand_skips_leading_flags() {
    // Arrange + Act + Assert: flags before the sub-command are ignored
    check!(parse("cargo --quiet build") == Some(("cargo".to_owned(), Some("build".to_owned()))));
}

#[test]
fn subcommand_skips_git_value_taking_global_options() {
    // Arrange + Act + Assert: a value-taking git global option's value (`/path`,
    // `x=y`) must not be mistaken for the sub-command — it resolves to diff/status.
    check!(
        parse("git -C /some/path diff HEAD") == Some(("git".to_owned(), Some("diff".to_owned())))
    );
    check!(
        parse("git -c user.name=x status") == Some(("git".to_owned(), Some("status".to_owned())))
    );
    check!(
        parse("git --git-dir /repo/.git log") == Some(("git".to_owned(), Some("log".to_owned())))
    );
}

#[test]
fn git_value_option_skip_is_scoped_to_git() {
    // Arrange + Act + Assert: `-C` is only a value-taking option for git; for
    // another command its following word is still the first eligible sub-command.
    check!(parse("docker -C build run") == Some(("docker".to_owned(), Some("build".to_owned()))));
}

#[test]
fn inject_offset_lands_after_subcommand_past_git_global_value() {
    // Arrange: a value-taking global option puts a path before the sub-command
    let text = "git -C /some/path diff a.txt";

    // Act
    let off = inject_offset(text, true).expect("argv0 located");

    // Assert: the splice point falls after the real `diff`, not after `/some/path`
    check!(&text[..off] == "git -C /some/path diff");
    check!(&text[off..] == " a.txt");
}

#[test]
fn all_assignment_or_empty_has_no_command() {
    // Arrange + Act + Assert
    check!(parse("FOO=bar").is_none());
    check!(parse("   ").is_none());
}

#[test]
fn blocking_set_covers_interactive_streaming_and_nonposix() {
    // Arrange + Act + Assert
    for argv0 in [
        "vim",
        "less",
        "top",
        "tail",
        "watch",
        "powershell",
        "pwsh",
        "cmd",
    ] {
        assert!(is_blocking(argv0), "{argv0} should block");
    }
    for argv0 in ["git", "ls", "cargo", "echo"] {
        assert!(!is_blocking(argv0), "{argv0} should not block");
    }
}

#[test]
fn backslash_escaped_space_keeps_argv0_as_one_word() {
    // Arrange + Act: the escaped space is part of argv0, not a separator
    let parsed = parse(r"my\ tool run");

    // Assert: argv0 is the single escaped word, `run` is its sub-command
    check!(parsed == Some(("my tool".to_owned(), Some("run".to_owned()))));
}

#[test]
fn trailing_backslash_does_not_panic_and_yields_argv0() {
    // Arrange + Act: a dangling backslash with nothing to escape is dropped
    let parsed = parse(r"ls\");

    // Assert
    check!(parsed == Some(("ls".to_owned(), None)));
}

#[test]
fn inject_offset_with_subcommand_splits_before_positional_args() {
    // Arrange: a sub-command followed by a positional pathspec
    let text = "git diff a.txt";

    // Act
    let off = inject_offset(text, true).expect("argv0 located");

    // Assert: the splice point falls right after `git diff`, before the pathspec
    check!(&text[..off] == "git diff");
    check!(&text[off..] == " a.txt");
}

#[test]
fn inject_offset_without_subcommand_follows_argv0() {
    // Arrange: a command that takes flags directly (no sub-command, e.g. rg)
    let text = "rg pattern src";

    // Act
    let off = inject_offset(text, false).expect("argv0 located");

    // Assert: flags splice in right after argv0
    check!(&text[..off] == "rg");
}

#[test]
fn inject_offset_skips_leading_env_assignments() {
    // Arrange: leading `NAME=val` assignments precede the real command
    let text = "FOO=bar git log --oneline";

    // Act
    let off = inject_offset(text, true).expect("argv0 located");

    // Assert: the splice point is past the assignment AND the sub-command
    check!(&text[..off] == "FOO=bar git log");
}
