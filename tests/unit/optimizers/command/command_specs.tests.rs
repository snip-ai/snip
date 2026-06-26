//! Unit tests for [`CommandSpecs`] lookup, in AAA form. Compiled into `snip_lib`
//! via a `#[path]` include in `src/commands/route/command_specs.rs`.

use assert2::check;

use super::CommandSpecs;
use crate::config::Config;

#[test]
fn catch_all_spec_matches_any_subcommand() {
    // Arrange: `ls` binds no sub-commands → catch-all
    let specs = CommandSpecs::load(&Config::default());

    // Act
    let hit = specs.find("ls", Some("-la"));
    let bare = specs.find("ls", None);

    // Assert
    check!(hit.map(|s| s.name.as_str()) == Some("ls"));
    check!(bare.map(|s| s.name.as_str()) == Some("ls"));
}

#[test]
fn subcommand_specific_match_requires_a_listed_subcommand() {
    // Arrange: `git` lists sub-commands → no catch-all
    let specs = CommandSpecs::load(&Config::default());

    // Act
    let log = specs.find("git", Some("log"));
    let status = specs.find("git", Some("status"));
    let unknown = specs.find("git", Some("frobnicate"));
    let bare = specs.find("git", None);

    // Assert: sub-commands resolve to their dedicated specs
    check!(log.map(|s| s.name.as_str()) == Some("git-log"));
    check!(status.map(|s| s.name.as_str()) == Some("git-status"));
    check!(unknown.is_none());
    check!(bare.is_none());
}

#[test]
fn unknown_command_and_by_name_lookup() {
    // Arrange
    let specs = CommandSpecs::load(&Config::default());

    // Act + Assert
    check!(specs.find("definitely-not-a-command", None).is_none());
    check!(specs.by_name("cargo").map(|s| s.name.as_str()) == Some("cargo"));
    check!(specs.by_name("search").is_none()); // search is not a Bash command spec
}

#[test]
fn base_and_git_family_specs_resolve() {
    // Arrange
    let specs = CommandSpecs::load(&Config::default());

    // Act + Assert: the base + git catalog is present
    check!(specs.find("grep", None).map(|s| s.name.as_str()) == Some("grep"));
    check!(specs.find("rg", None).map(|s| s.name.as_str()) == Some("rg"));
    check!(specs.find("find", None).map(|s| s.name.as_str()) == Some("find"));
    check!(specs.find("git", Some("show")).map(|s| s.name.as_str()) == Some("git-show"));
    check!(specs.find("git", Some("branch")).map(|s| s.name.as_str()) == Some("git-branch"));
}

#[test]
fn a_rule_disables_a_whole_command_family() {
    // Arrange: disable the git family
    let cfg: Config =
        serde_json::from_str(r#"{"optimizers":{"command":{"rules":{"git":false}}}}"#).unwrap();
    let specs = CommandSpecs::load(&cfg);

    // Act + Assert: no git spec resolves; other families unaffected
    check!(specs.find("git", Some("status")).is_none());
    check!(specs.find("ls", None).map(|s| s.name.as_str()) == Some("ls"));
}

#[test]
fn a_rule_disables_a_single_spec_by_name() {
    // Arrange: disable just git-diff
    let cfg: Config =
        serde_json::from_str(r#"{"optimizers":{"command":{"rules":{"git-diff":false}}}}"#).unwrap();
    let specs = CommandSpecs::load(&cfg);

    // Act + Assert
    check!(specs.find("git", Some("diff")).is_none());
    check!(specs.find("git", Some("status")).map(|s| s.name.as_str()) == Some("git-status"));
}

#[test]
fn lang_family_specs_resolve_and_skip_servers() {
    // Arrange
    let specs = CommandSpecs::load(&Config::default());

    // Act + Assert: terminating lang commands resolve
    check!(specs.find("npm", Some("install")).map(|s| s.name.as_str()) == Some("npm-install"));
    check!(specs.find("pytest", None).map(|s| s.name.as_str()) == Some("pytest"));
    check!(specs.find("go", Some("test")).map(|s| s.name.as_str()) == Some("go-test"));
    check!(specs.find("go", Some("build")).map(|s| s.name.as_str()) == Some("go-build-vet"));
    check!(specs.find("go", Some("vet")).map(|s| s.name.as_str()) == Some("go-build-vet"));
    check!(specs.find("eslint", None).map(|s| s.name.as_str()) == Some("eslint"));
    check!(specs.find("jest", None).map(|s| s.name.as_str()) == Some("jest"));
    // table-collapse stays scoped to the tabular subcommands (ps/images); `docker
    // build` logs now have their own log-oriented spec (strip_ansi/dedupe/rank), not
    // table_collapse.
    check!(specs.find("kubectl", Some("get")).map(|s| s.name.as_str()) == Some("kubectl-get"));
    check!(
        specs
            .find("kubectl", Some("describe"))
            .map(|s| s.name.as_str())
            == Some("kubectl-describe")
    );
    check!(specs.find("docker", Some("ps")).map(|s| s.name.as_str()) == Some("docker-ps"));
    check!(specs.find("docker", Some("build")).map(|s| s.name.as_str()) == Some("docker-build"));
    // `npm start` / `npm run` are not matched (would block a dev server)
    check!(specs.find("npm", Some("start")).is_none());
    check!(specs.find("npm", Some("run")).is_none());
    // Tier 1/2 expansion: JVM/.NET/git resolve; server subcommands stay excluded
    check!(specs.find("mvn", None).map(|s| s.name.as_str()) == Some("maven"));
    check!(specs.find("gradle", None).map(|s| s.name.as_str()) == Some("gradle"));
    check!(specs.find("dotnet", Some("build")).map(|s| s.name.as_str()) == Some("dotnet"));
    check!(specs.find("dotnet", Some("run")).is_none());
    check!(specs.find("vitest", Some("run")).map(|s| s.name.as_str()) == Some("vitest"));
    check!(specs.find("vitest", None).is_none());
    check!(specs.find("git", Some("ls-files")).map(|s| s.name.as_str()) == Some("git-ls-files"));
    check!(specs.find("git", Some("grep")).map(|s| s.name.as_str()) == Some("git-grep"));
}

#[test]
fn user_spec_extends_command_lookup() {
    // Arrange: a user spec adds a brand-new command with no built-in counterpart
    let cfg: Config = serde_json::from_str(
        r#"{"specs":[{"name":"frobnicate","surface":"bash","bind":{"cmd":"frobnicate"},
            "transforms":[{"op":"dedupe"}]}]}"#,
    )
    .unwrap();
    let specs = CommandSpecs::load(&cfg);

    // Act
    let frob = specs.find("frobnicate", None);

    // Assert
    check!(frob.map(|s| s.name.as_str()) == Some("frobnicate"));
}

#[test]
fn build_and_compiler_catch_alls_resolve_for_any_subcommand() {
    // Arrange: the new non-streaming build/compiler families (no dev-server
    // subcommand, so safe as catch-alls — unlike npm/yarn/pnpm).
    let specs = CommandSpecs::load(&Config::default());

    // Act + Assert: each resolves for an arbitrary invocation …
    check!(specs.find("make", None).map(|s| s.name.as_str()) == Some("make"));
    check!(specs.find("make", Some("build")).map(|s| s.name.as_str()) == Some("make"));
    check!(
        specs
            .find("cmake", Some("--build"))
            .map(|s| s.name.as_str())
            == Some("cmake")
    );
    check!(specs.find("gcc", None).map(|s| s.name.as_str()) == Some("gcc"));
    check!(specs.find("g++", None).map(|s| s.name.as_str()) == Some("gpp"));
    check!(specs.find("clang", None).map(|s| s.name.as_str()) == Some("clang"));
    // … pip queries resolve to the catch-all, while `pip install` keeps its spec.
    check!(specs.find("pip", Some("list")).map(|s| s.name.as_str()) == Some("pip-query"));
    check!(specs.find("pip", Some("install")).map(|s| s.name.as_str()) == Some("pip-install"));
}

#[test]
fn user_spec_shadows_a_builtin_by_name() {
    // Arrange: a user spec named "cargo" replaces the built-in cargo entirely
    let cfg: Config = serde_json::from_str(
        r#"{"specs":[{"name":"cargo","surface":"bash",
            "bind":{"cmd":"cargo","subcommands":["doc"]}}]}"#,
    )
    .unwrap();
    let specs = CommandSpecs::load(&cfg);

    // Act
    let doc = specs.find("cargo", Some("doc"));
    let test = specs.find("cargo", Some("test"));

    // Assert: the user's cargo (doc) replaced the built-in `cargo` (test/run/bench)
    check!(doc.map(|s| s.name.as_str()) == Some("cargo"));
    check!(test.is_none());
}
