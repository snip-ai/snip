//! Identify a segment's effective command and flag the ones that are never safe
//! to wrap (interactive, streaming, or a non-POSIX shell → whole-line bail).

/// Parse a segment into `(argv0 basename, first sub-command word)`.
///
/// Skips leading `NAME=val` environment assignments and is quote-aware enough to
/// find argv0; `None` for an empty/all-assignment segment. Sub-command detection
/// (incl. `git`'s value-taking global options) lives in [`locate`].
#[must_use]
pub fn parse(text: &str) -> Option<(String, Option<String>)> {
    let owned = words(text);
    let words: Vec<&str> = owned.iter().map(String::as_str).collect();
    let (argv0, sub) = locate(&words)?;
    Some((basename(words[argv0]), sub.map(|i| words[i].to_owned())))
}

/// `git` global options that consume the **following** token as their value, so the
/// real sub-command is the word after it. Only these two-token forms mask the
/// sub-command; single-token `--flag=value` is one flag word and needs no entry.
const GIT_VALUE_OPTS: &[&str] = &[
    "-C",
    "-c",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--exec-path",
];

/// Wrapper commands that prefix the real tool (`sudo cargo build`, `npx eslint`,
/// `env FOO=1 pytest`). Skipping past them lets the real tool's spec match — and,
/// since `is_blocking` then sees the real tool, also stops a wrapped interactive
/// command (`sudo vim`) from being wrapped into a hang.
const WRAPPERS: &[&str] = &[
    "sudo", "doas", "env", "nice", "ionice", "nohup", "stdbuf", "setarch", "time", "npx",
];

/// Wrapper options that consume the following word as their value. Conservative:
/// a wrong guess only costs a missed spec (the landing word isn't a known command,
/// so `find` returns nothing) — never a misfire that breaks a command.
const WRAPPER_VALUE_OPTS: &[&str] = &[
    "-u",
    "-g",
    "-U",
    "-C",
    "-p",
    "-n",
    "-c",
    "-r",
    "-t",
    "--package",
];

/// Advance `argv0` past any leading wrapper commands to the real tool. Bounded so
/// a pathological chain can't loop.
fn unwrap_wrappers(words: &[&str], mut argv0: usize) -> usize {
    for _ in 0..8 {
        let base = basename(words[argv0]);
        if !WRAPPERS.contains(&base.as_str()) {
            return argv0;
        }
        let mut i = argv0 + 1;
        while let Some(&w) = words.get(i) {
            if w == "--" {
                i += 1;
                break;
            }
            if is_assignment(w) {
                i += 1;
                continue;
            }
            if !w.starts_with('-') {
                break;
            }
            if WRAPPER_VALUE_OPTS.contains(&w) {
                i += 1;
            }
            i += 1;
        }
        if i < words.len() {
            argv0 = i;
        } else {
            return argv0;
        }
    }
    argv0
}

/// True if `text`'s words already contain the option `key`.
///
/// Matches both the `--flag value` form (`key` as its own word) and the
/// `--flag=value` form (`key=…`). Lets the planner skip injecting a
/// format-changing flag the user set explicitly, so snip never overrides — and
/// breaks — a user-requested output format.
#[must_use]
pub fn has_option(text: &str, key: &str) -> bool {
    words(text).iter().any(|w| {
        w == key
            || w.strip_prefix(key)
                .is_some_and(|rest| rest.starts_with('='))
    })
}

/// `(argv0 index, sub-command index)` into `words`: the first non-assignment word
/// (argv0, after skipping any [`WRAPPERS`]), then the first non-flag word after it
/// — skipping, for `git`, a [`GIT_VALUE_OPTS`] value (`git -C /path diff` ⇒ `diff`).
fn locate(words: &[&str]) -> Option<(usize, Option<usize>)> {
    let argv0 = words.iter().position(|w| !is_assignment(w))?;
    let argv0 = unwrap_wrappers(words, argv0);
    let is_git = basename(words[argv0]) == "git";
    let mut i = argv0 + 1;
    while let Some(&w) = words.get(i) {
        if !w.starts_with('-') {
            return Some((argv0, Some(i)));
        }
        if is_git && GIT_VALUE_OPTS.contains(&w) {
            i += 1; // also skip the option's value token
        }
        i += 1;
    }
    Some((argv0, None))
}

/// Whether `argv0` is interactive, streaming, or a non-POSIX shell — any of
/// which makes wrapping unsafe (hangs) or pointless, so the whole line bails.
#[must_use]
pub fn is_blocking(argv0: &str) -> bool {
    const BLOCKING: &[&str] = &[
        // interactive editors / pagers / REPLs
        "vim",
        "vi",
        "nano",
        "emacs",
        "less",
        "more",
        "top",
        "htop",
        "man",
        "ssh",
        "python",
        "python3",
        "node",
        "irb",
        "psql",
        "mysql",
        "ftp",
        "telnet",
        // streaming / long-lived
        "watch",
        "tail", // `tail -f` — conservatively bail on all tail
        // non-POSIX shells (PowerShell is never optimized; only Git Bash/WSL)
        "powershell",
        "pwsh",
        "cmd",
        "nu",
        "fish",
        "wsl",
    ];
    BLOCKING.contains(&argv0)
}

/// Split `text` into words on unquoted whitespace, stripping quotes and honoring
/// backslash escapes. Enough to recover argv0/sub-command (not a full shell lexer).
fn words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut started = false;
    let mut chars = text.chars().peekable();
    let (mut single, mut double) = (false, false);
    while let Some(c) = chars.next() {
        match c {
            '\'' if !double => single = !single,
            '"' if !single => double = !double,
            '\\' if !single => {
                if let Some(&n) = chars.peek() {
                    cur.push(n);
                    chars.next();
                    started = true;
                }
            }
            c if c.is_whitespace() && !single && !double => {
                if started {
                    words.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            c => {
                cur.push(c);
                started = true;
            }
        }
    }
    if started {
        words.push(cur);
    }
    words
}

/// Whether `word` is a `NAME=value` environment assignment prefix.
fn is_assignment(word: &str) -> bool {
    let Some(eq) = word.find('=') else {
        return false;
    };
    let name = &word[..eq];
    !name.is_empty()
        && name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The basename of `argv0` (after the last `/` or `\`), with any `.exe` dropped.
fn basename(argv0: &str) -> String {
    let base = argv0.rsplit(['/', '\\']).next().unwrap_or(argv0);
    base.strip_suffix(".exe").unwrap_or(base).to_string()
}

/// Byte offset in `text` just past the command prefix that `inject_flags` follow.
///
/// Past argv0 (skipping leading `NAME=val` assignments), and past the first
/// sub-command word too when `with_sub` is set. Tools require their options
/// *before* positional arguments (e.g. `git diff --no-color <pathspec>`,
/// `cargo test --message-format=json -- <args>`), so flags appended at the line's
/// end break the command. `None` when no argv0 can be located (the caller then
/// injects nothing rather than risk a broken command). Shares [`locate`] with
/// [`parse`], so `git -C /path diff` splices after `diff`, not `/path`.
#[must_use]
pub fn inject_offset(text: &str, with_sub: bool) -> Option<usize> {
    let ranges = word_ranges(text);
    let words: Vec<&str> = ranges.iter().map(|&(s, e)| &text[s..e]).collect();
    let (argv0, sub) = locate(&words)?;
    if !with_sub {
        return Some(ranges[argv0].1);
    }
    // Past the sub-command (per `locate`); fall back to just-past-argv0 if none.
    Some(sub.map_or(ranges[argv0].1, |i| ranges[i].1))
}

/// Byte ranges `[start, end)` of each whitespace-separated word in `text`, with
/// the same quote/escape rules as [`words`] but tracking offsets (not content).
fn word_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let (mut single, mut double) = (false, false);
    let mut start: Option<usize> = None;
    let mut chars = text.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '\'' if !double => {
                single = !single;
                start.get_or_insert(i);
            }
            '"' if !single => {
                double = !double;
                start.get_or_insert(i);
            }
            '\\' if !single => {
                start.get_or_insert(i);
                chars.next(); // the escaped char stays part of this word
            }
            c if c.is_whitespace() && !single && !double => {
                if let Some(s) = start.take() {
                    ranges.push((s, i));
                }
            }
            _ => {
                start.get_or_insert(i);
            }
        }
    }
    if let Some(s) = start.take() {
        ranges.push((s, text.len()));
    }
    ranges
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/command/recognition.tests.rs"]
mod tests;
