// Phase B — the opt-in /snip-shell-setup script (snip-shell-setup.sh).
//
// Exercises the shell script the /snip-shell-setup slash-command wraps, the same
// way the lifecycle test exercises snip-run.sh / snip-bootstrap.sh: invoke it
// directly via bash against a throwaway HOME, no `claude` and no network. It must
// add ONE clearly-marked PATH block to the shell rc, be idempotent, remove it
// cleanly, and reject an unknown action. SHELL is pinned to zsh so the target rc
// (~/.zshrc) is the same on every OS (the script's zsh arm has no per-OS branch).

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test, { after, describe } from "node:test";
import assert from "node:assert/strict";

import { SNIP_SHELL_SETUP, freshDir } from "./harness/lib.mjs";
import { cleanupTemp } from "./harness/run.mjs";

const haveBash = spawnSync("bash", ["--version"], { encoding: "utf8" }).status === 0;
const SKIP = haveBash ? false : "bash not available (the setup script needs it)";

const MARK_BEGIN = "# >>> snip shell setup >>>";

after(cleanupTemp);

// Run the setup script with an isolated HOME and a pinned zsh rc path. SNIP_HOME
// is stripped so the data-dir resolution takes the OS default, matching a normal
// user shell.
function runSetup(home, action) {
  const env = { ...process.env, HOME: home, SHELL: "/bin/zsh" };
  delete env.SNIP_HOME;
  const r = spawnSync("bash", [SNIP_SHELL_SETUP, ...(action ? [action] : [])], {
    encoding: "utf8",
    env,
  });
  return { status: r.status, stdout: `${r.stdout ?? ""}${r.stderr ?? ""}` };
}

function rc(home) {
  return path.join(home, ".zshrc");
}

function countMarkers(file) {
  const text = fs.existsSync(file) ? fs.readFileSync(file, "utf8") : "";
  return text.split(MARK_BEGIN).length - 1;
}

describe("Phase B — /snip-shell-setup (opt-in shell PATH)", () => {
  test("setup writes one marked PATH block; idempotent; remove takes it back out", { skip: SKIP }, () => {
    const home = freshDir("snip-shellsetup-");

    // setup: writes the marked block
    const a = runSetup(home, "setup");
    assert.equal(a.status, 0, "setup exits 0");
    assert.match(a.stdout, /added the binary dir to your PATH/, "setup reports success");
    const after1 = fs.readFileSync(rc(home), "utf8");
    assert.match(after1, /# >>> snip shell setup >>>/, "begin marker present");
    assert.match(after1, /# <<< snip shell setup <<</, "end marker present");
    assert.match(after1, /export PATH=.*snip\/bin/, "the PATH export points at snip/bin");

    // setup again: no-op, still exactly one block
    const b = runSetup(home, "setup");
    assert.equal(b.status, 0, "second setup exits 0");
    assert.match(b.stdout, /already configured/, "second setup is a reported no-op");
    assert.equal(countMarkers(rc(home)), 1, "still exactly one block (idempotent)");

    // remove: the block is gone
    const c = runSetup(home, "remove");
    assert.equal(c.status, 0, "remove exits 0");
    assert.match(c.stdout, /removed the snip PATH line/, "remove reports success");
    assert.equal(countMarkers(rc(home)), 0, "the block is gone after remove");

    // remove again: nothing to do
    const d = runSetup(home, "remove");
    assert.equal(d.status, 0, "second remove exits 0");
    assert.match(d.stdout, /nothing to remove/, "second remove is a reported no-op");
  });

  test("default action is setup", { skip: SKIP }, () => {
    const home = freshDir("snip-shellsetup-");

    const r = runSetup(home, null);

    assert.equal(r.status, 0, "no-arg invocation exits 0");
    assert.equal(countMarkers(rc(home)), 1, "no-arg invocation defaults to setup");
  });

  test("an unknown action is rejected with a non-zero exit", { skip: SKIP }, () => {
    const home = freshDir("snip-shellsetup-");

    const r = runSetup(home, "bogus");

    assert.equal(r.status, 2, "unknown action exits 2");
    assert.match(r.stdout, /unknown action/, "unknown action is reported");
    assert.ok(!fs.existsSync(rc(home)), "an unknown action writes no rc file");
  });
});
