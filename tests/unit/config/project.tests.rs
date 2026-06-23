//! Unit tests for the project config layer merge, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/config/project.rs`.

use assert2::check;
use serde_json::{Value, json};

use super::{apply, merge, opted_in};

#[test]
fn opted_in_requires_the_explicit_flag() {
    // Act + Assert
    check!(opted_in(&json!({"allow_project_config": true})));
    check!(!opted_in(&json!({"allow_project_config": false})));
    check!(!opted_in(&json!({})));
}

#[test]
fn secret_safe_is_enable_only() {
    // Arrange + Act: a project layer can turn redaction ON...
    let mut on = json!({"secret_safe": false});
    merge(&mut on, &json!({"secret_safe": true}));
    // ...but never OFF
    let mut off = json!({"secret_safe": true});
    merge(&mut off, &json!({"secret_safe": false}));

    // Assert
    check!(on["secret_safe"] == Value::Bool(true));
    check!(off["secret_safe"] == Value::Bool(true));
}

#[test]
fn master_enabled_is_disable_only() {
    // Arrange + Act: a repo may opt out...
    let mut out = json!({"master_enabled": true});
    merge(&mut out, &json!({"master_enabled": false}));
    // ...but cannot force snip on against a global off
    let mut forced = json!({"master_enabled": false});
    merge(&mut forced, &json!({"master_enabled": true}));

    // Assert
    check!(out["master_enabled"] == Value::Bool(false));
    check!(forced["master_enabled"] == Value::Bool(false));
}

#[test]
fn allow_project_config_is_not_project_overridable() {
    // Arrange + Act: a project file must not be able to deepen its own loading
    let mut user = json!({"allow_project_config": false});
    merge(&mut user, &json!({"allow_project_config": true}));

    // Assert: the user's value wins
    check!(user["allow_project_config"] == Value::Bool(false));
}

#[test]
fn specs_append_and_optimizers_merge_per_name() {
    // Arrange
    let mut user = json!({
        "specs": [{"name": "a"}],
        "optimizers": {"read": {"enabled": true}}
    });

    // Act
    merge(
        &mut user,
        &json!({
            "specs": [{"name": "b"}],
            "optimizers": {"command": {"enabled": false}}
        }),
    );

    // Assert: specs accumulate; optimizers merge by name
    check!(user["specs"].as_array().map(Vec::len) == Some(2));
    check!(user["optimizers"]["read"]["enabled"] == Value::Bool(true));
    check!(user["optimizers"]["command"]["enabled"] == Value::Bool(false));
}

#[test]
fn tuning_keys_override() {
    // Arrange + Act: overflow/autodetect are plain tuning overrides
    let mut user = json!({"overflow": {"max_tokens": 8000}});
    merge(&mut user, &json!({"overflow": {"max_tokens": 2000}}));

    // Assert
    check!(user["overflow"]["max_tokens"] == json!(2000));
}

#[test]
fn merge_is_a_noop_when_either_side_is_not_an_object() {
    // Arrange: a non-object user value can't be merged into
    let mut user = json!("not-an-object");

    // Act
    merge(&mut user, &json!({"secret_safe": true}));

    // Assert: unchanged
    check!(user == json!("not-an-object"));
}

#[test]
fn merge_optimizers_starts_from_empty_when_user_has_none() {
    // Arrange: the user has no `optimizers` key at all
    let mut user = json!({});

    // Act: the project layer introduces one
    merge(
        &mut user,
        &json!({"optimizers": {"read": {"enabled": false}}}),
    );

    // Assert: an entry is created and populated
    check!(user["optimizers"]["read"]["enabled"] == Value::Bool(false));
}

#[test]
fn merge_specs_starts_from_empty_when_user_has_none() {
    // Arrange: the user has no `specs` key at all
    let mut user = json!({});

    // Act
    merge(&mut user, &json!({"specs": [{"name": "a"}]}));

    // Assert
    check!(user["specs"].as_array().map(Vec::len) == Some(1));
}

#[test]
fn merge_objects_ignores_a_non_object_optimizers_value() {
    // Arrange: a malformed project `optimizers` (an array, not an object)
    let mut user = json!({"optimizers": {"read": {"enabled": true}}});

    // Act: the inner merge is a no-op when the source isn't an object
    merge(&mut user, &json!({"optimizers": [1, 2, 3]}));

    // Assert: the user's optimizers survive untouched
    check!(user["optimizers"]["read"]["enabled"] == Value::Bool(true));
}

#[test]
fn append_array_ignores_a_non_array_specs_value() {
    // Arrange: a malformed project `specs` (an object, not an array)
    let mut user = json!({"specs": [{"name": "a"}]});

    // Act
    merge(&mut user, &json!({"specs": {"name": "b"}}));

    // Assert: the user's specs survive untouched
    check!(user["specs"].as_array().map(Vec::len) == Some(1));
}

#[test]
fn apply_is_a_noop_without_opt_in() {
    // Arrange: not opted in, so the project file is never even looked for
    let mut user = json!({"allow_project_config": false, "secret_safe": false});

    // Act
    apply(&mut user);

    // Assert: unchanged
    check!(user["secret_safe"] == Value::Bool(false));
}

#[test]
fn apply_overlays_the_project_file_when_opted_in() {
    // Arrange: a process-unique cwd holding a `.snip/config.json`, serialized on
    // the env lock because changing cwd is process-global.
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let original = std::env::current_dir().unwrap();
    let dir = std::env::temp_dir().join(format!("snip-projtest-{}", std::process::id()));
    std::fs::create_dir_all(dir.join(".snip")).unwrap();
    std::fs::write(
        dir.join(".snip").join("config.json"),
        r#"{"secret_safe": true, "overflow": {"max_tokens": 1234}}"#,
    )
    .unwrap();
    std::env::set_current_dir(&dir).unwrap();
    let mut user = json!({"allow_project_config": true, "secret_safe": false});

    // Act
    apply(&mut user);

    // Assert: the project layer's tuning is overlaid; secret_safe is OR'd on
    check!(user["secret_safe"] == Value::Bool(true));
    check!(user["overflow"]["max_tokens"] == json!(1234));

    // Cleanup
    std::env::set_current_dir(&original).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
