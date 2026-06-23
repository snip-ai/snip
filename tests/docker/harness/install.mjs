// Install snip the way the plugin does, but driven from the harness: drop the
// built binary into an isolated $SNIP_HOME/bin, and generate the settings.json
// `hooks` block that registers snip's eight hooks via the SAME `snip-run.sh`
// wrapper Claude Code runs in production. `${CLAUDE_PLUGIN_ROOT}` is NOT expanded
// outside a plugin, so settings.json hooks use the absolute wrapper path and the
// runner exports CLAUDE_PLUGIN_ROOT for `snip-run.sh`'s version probe.

import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import { SNIP_RUN, freshDir } from "./lib.mjs";

const BIN_NAME = os.platform() === "win32" ? "snip.exe" : "snip";

/** Copy `srcBinary` into a fresh isolated $SNIP_HOME/bin; return the home dir. */
export function installBinary(srcBinary) {
  const home = freshDir("snip-home-");
  const binDir = path.join(home, "bin");
  fs.mkdirSync(binDir, { recursive: true });
  const dest = path.join(binDir, BIN_NAME);
  fs.copyFileSync(srcBinary, dest);
  fs.chmodSync(dest, 0o755);
  return home;
}

/** Path to the installed binary inside a $SNIP_HOME (whether or not it exists). */
export function binaryPath(home) {
  return path.join(home, "bin", BIN_NAME);
}

// surface → (event, matcher, subcommand) for the eight production hooks.
const HOOKS = [
  ["PostToolUse", "Read", "read-hook"],
  ["PostToolUse", "Grep", "grep-hook"],
  ["PostToolUse", "Glob", "glob-hook"],
  ["PreToolUse", "Bash", "bash-route"],
  ["PreToolUse", "Edit", "edit-fix"],
  ["PreToolUse", "Write", "write-guard"],
  ["PreCompact", null, "session-reset"],
  ["SessionStart", null, "update-check"],
];

function entry(matcher, command, timeout) {
  const e = { hooks: [{ type: "command", command, timeout }] };
  if (matcher) e.matcher = matcher;
  return e;
}

/** settings.json `hooks` block wiring every surface to `snip-run.sh <sub>`. */
export function snipHooks() {
  const hooks = {};
  for (const [event, matcher, sub] of HOOKS) {
    const timeout = event === "PostToolUse" || event === "PreToolUse" ? 15 : 10;
    (hooks[event] ??= []).push(entry(matcher, `bash "${SNIP_RUN}" ${sub}`, timeout));
  }
  return { hooks };
}

/** settings.json `hooks` block routing the six tool surfaces to the spy script. */
export function spyHooks(spyScript) {
  const hooks = { PostToolUse: [], PreToolUse: [] };
  for (const [event, matcher] of HOOKS) {
    if (event !== "PostToolUse" && event !== "PreToolUse") continue;
    hooks[event].push(entry(matcher, `node "${spyScript}"`, 15));
  }
  return { hooks };
}

/** Write a settings object to `<dir>/settings.json`; return the file path. */
export function writeSettings(dir, settings) {
  fs.mkdirSync(dir, { recursive: true });
  const file = path.join(dir, "settings.json");
  fs.writeFileSync(file, JSON.stringify(settings, null, 2));
  return file;
}
