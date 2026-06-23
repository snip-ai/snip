//! Unit tests for [`fold_frames`] stacktrace pruning, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/spec/stacktrace.rs`.

use assert2::check;

use super::{StacktraceCfg, fold_frames};

fn lines(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn folds_framework_runs_keeping_app_frames() {
    // Arrange: a Python traceback — app frames surround a deep site-packages run
    let records = lines(&[
        "Traceback (most recent call last):",
        "  File \"app/main.py\", line 10, in run",
        "  File \"x/site-packages/flask/app.py\", line 1, in dispatch",
        "  File \"x/site-packages/flask/app.py\", line 2, in full",
        "  File \"x/site-packages/werkzeug/wsgi.py\", line 3, in call",
        "  File \"app/handlers.py\", line 42, in handle",
        "ValueError: boom",
    ]);

    // Act: default cfg (enabled, keep_top = 1)
    let out = fold_frames(records, &StacktraceCfg::default());

    // Assert: header + app frames + error kept verbatim; the framework run keeps
    // its top frame then folds the remaining 2
    check!(
        out == lines(&[
            "Traceback (most recent call last):",
            "  File \"app/main.py\", line 10, in run",
            "  File \"x/site-packages/flask/app.py\", line 1, in dispatch",
            "… (2 framework frames)",
            "  File \"app/handlers.py\", line 42, in handle",
            "ValueError: boom",
        ])
    );
}

#[test]
fn disabled_returns_input_unchanged() {
    // Arrange
    let records = lines(&[
        "  File \"app/main.py\", line 10, in run",
        "  File \"x/site-packages/lib.py\", line 1, in a",
        "  File \"x/site-packages/lib.py\", line 2, in b",
    ]);
    let cfg = StacktraceCfg {
        enabled: false,
        ..StacktraceCfg::default()
    };

    // Act
    let out = fold_frames(records.clone(), &cfg);

    // Assert
    check!(out == records);
}

#[test]
fn keep_top_zero_folds_the_whole_run() {
    // Arrange: keep_top = 0 → no framework frame is kept before folding
    let records = lines(&[
        "  at app.Main.run(Main.java:10)",
        "  at java.base/java.lang.Thread.run(Thread.java:1)",
        "  at jdk.internal.X.y(X.java:2)",
    ]);
    let cfg = StacktraceCfg {
        keep_top: 0,
        ..StacktraceCfg::default()
    };

    // Act
    let out = fold_frames(records, &cfg);

    // Assert: app frame kept; both runtime frames fold together
    check!(out == lines(&["  at app.Main.run(Main.java:10)", "… (2 framework frames)",]));
}
