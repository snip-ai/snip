//! Unit tests for the built-in spec registry, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/spec/builtin/mod.rs`.

use assert2::check;

use super::{BASE, GIT, LANG, SEARCH, builtin_specs, builtin_specs_for, merged_specs_for};
use crate::domain::Surface;
use crate::spec::OptimizerSpec;

#[test]
fn every_family_file_parses_fully() {
    // A typo in a family JSON would otherwise be filter_map'd away silently.
    for (family, json) in [
        ("search", SEARCH),
        ("base", BASE),
        ("git", GIT),
        ("lang", LANG),
    ] {
        assert!(
            serde_json::from_str::<Vec<OptimizerSpec>>(json).is_ok(),
            "family {family} failed to parse"
        );
    }
}

#[test]
fn ships_search_on_grep_and_glob() {
    // Arrange + Act: the search surfaces carry one per-surface-named spec each.
    let specs = builtin_specs();
    let grep = specs
        .iter()
        .find(|s| s.name == "search-grep")
        .map(|s| s.surface);
    let glob = specs
        .iter()
        .find(|s| s.name == "search-glob")
        .map(|s| s.surface);

    // Assert
    check!(grep == Some(Surface::Grep));
    check!(glob == Some(Surface::Glob));
}

#[test]
fn ships_command_specs_on_the_bash_surface() {
    // Arrange + Act: command-family specs bind a command on the Bash surface
    let specs = builtin_specs();
    let bound: Vec<&str> = specs
        .iter()
        .filter(|s| s.surface == Surface::Bash)
        .filter_map(|s| s.bind.cmd.as_deref())
        .collect();

    // Assert
    check!(bound.contains(&"ls"));
    check!(bound.contains(&"git"));
    check!(bound.contains(&"cargo"));
    check!(bound.contains(&"go"));
    // P12 language toolchains
    check!(bound.contains(&"swift"));
    check!(bound.contains(&"dart"));
    check!(bound.contains(&"flutter"));
    check!(bound.contains(&"zig"));
    check!(bound.contains(&"cabal"));
    check!(bound.contains(&"xcodebuild"));
    // Tier 1/2 catalog expansion (JVM / .NET / Ruby / PHP / generic build)
    check!(bound.contains(&"mvn"));
    check!(bound.contains(&"mvnw"));
    check!(bound.contains(&"gradle"));
    check!(bound.contains(&"gradlew"));
    check!(bound.contains(&"dotnet"));
    check!(bound.contains(&"make"));
    check!(bound.contains(&"rspec"));
    check!(bound.contains(&"bundle"));
    check!(bound.contains(&"composer"));
    check!(bound.contains(&"golangci-lint"));
    check!(bound.contains(&"vitest"));
    check!(bound.contains(&"uv"));
}

#[test]
fn builtin_specs_for_parses_only_the_relevant_surface() {
    // Act
    let read = builtin_specs_for(Surface::Read);
    let grep = builtin_specs_for(Surface::Grep);
    let glob = builtin_specs_for(Surface::Glob);
    let bash = builtin_specs_for(Surface::Bash);

    // Assert: Read/Edit/Write parse no specs; Grep/Glob get only their own search
    // spec; Bash gets the command families.
    check!(read.is_empty());
    check!(grep.iter().all(|s| s.surface == Surface::Grep));
    check!(glob.iter().all(|s| s.surface == Surface::Glob));
    check!(grep.len() == 1 && glob.len() == 1);
    check!(!bash.is_empty() && bash.iter().all(|s| s.surface == Surface::Bash));
}

#[test]
fn merged_specs_for_overlays_user_over_builtin_by_name() {
    // Arrange: replace built-in "cargo" and add a new "make", both Bash
    let user: Vec<OptimizerSpec> = serde_json::from_str(
        r#"[
            {"name":"cargo","surface":"bash","bind":{"cmd":"cargo","subcommands":["doc"]}},
            {"name":"make","surface":"bash","bind":{"cmd":"make"}}
        ]"#,
    )
    .unwrap();

    // Act
    let specs = merged_specs_for(Surface::Bash, &user);

    // Assert: exactly one "cargo" (the user's) survives, plus the new "make"
    check!(specs.iter().filter(|s| s.name == "cargo").count() == 1);
    check!(specs.iter().any(|s| s.name == "make"));
}
