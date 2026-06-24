// Phase B (Bash) — the command optimizer through the real Bash tool. Claude Code
// fires bash-route (PreToolUse), which rewrites to `snip exec`; snip runs the
// command and the model sees the optimized stdout. We cover: recognized base-shell
// families that compact on overflow (ls/find/grep), the git family (git_status_v2),
// and transparency — small output and shell operators/pipes must reach the model
// byte-for-byte (the wrapper never corrupts a command). Language-framework families
// (cargo/eslint/ruff/go-test/jest) are unit-tested in Rust against captured output;
// here we exercise everything that needs only base tools + git.
//
// Run: `node --test tests/docker/phase-b-bash.test.mjs` (needs `claude`, `git`).

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test, { after, before, describe } from "node:test";
import assert from "node:assert/strict";

import { claudeAvailable, freshDir } from "./harness/lib.mjs";
import { cleanupTemp, makeWorkspace, runScenario, snipHooks } from "./harness/run.mjs";
import { resetIds, toolTurn } from "./harness/scenario.mjs";

const SKIP = claudeAvailable() ? false : "the `claude` CLI is not on PATH";
const haveGit = spawnSync("git", ["--version"], { encoding: "utf8" }).status === 0;

after(cleanupTemp);

/** Create `n` empty files in `dir` so a listing overflows the truncate cap. */
function manyFiles(dir, n) {
  fs.mkdirSync(dir, { recursive: true });
  for (let i = 0; i < n; i++) fs.writeFileSync(path.join(dir, `f${String(i).padStart(3, "0")}.rs`), "");
}

/** A git repo with `commits` commits and `untracked` untracked files. */
function gitRepo(dir, { commits, untracked, modified = 0 }) {
  fs.mkdirSync(dir, { recursive: true });
  const env = { ...process.env, GIT_AUTHOR_NAME: "t", GIT_AUTHOR_EMAIL: "t@t", GIT_COMMITTER_NAME: "t", GIT_COMMITTER_EMAIL: "t@t" };
  const git = (...a) => spawnSync("git", ["-C", dir, ...a], { env, encoding: "utf8" });
  git("init", "-q");
  git("config", "user.email", "t@t");
  git("config", "user.name", "t");
  for (let i = 0; i < commits; i++) {
    fs.writeFileSync(path.join(dir, `c${i}.txt`), `content ${i}\n`);
    git("add", "-A");
    git("commit", "-q", "-m", `commit number ${i} with a reasonably long descriptive subject line`);
  }
  // Dirty tracked files so `git status --porcelain=v2` emits long type-1 records
  // (`1 <xy> <sub> <3 modes> <2 oids> <path>`) that git_status_v2 collapses to
  // `<xy> path` — a real saving that clears the no-inflation guard. Untracked-only
  // (`? path` → `?? path`) would INFLATE, so these modified entries are what makes
  // the compaction header appear; half staged (M.), half unstaged (.M).
  for (let i = 0; i < modified && i < commits; i++) {
    fs.appendFileSync(path.join(dir, `c${i}.txt`), `dirty change ${i}\n`);
    if (i % 2 === 0) git("add", `c${i}.txt`);
  }
  for (let i = 0; i < untracked; i++) fs.writeFileSync(path.join(dir, `u${String(i).padStart(3, "0")}.txt`), `u${i}\n`);
}

describe("Phase B — Bash command optimizer (real Bash tool)", () => {
  const seen = {};

  before(async () => {
    if (SKIP) return;

    // Scenario 1: base-shell families + transparency.
    {
      const ws = makeWorkspace();
      const bigdir = path.join(ws.cwd, "bigdir");
      manyFiles(bigdir, 150);
      resetIds();
      const turns = {
        ls: toolTurn("Bash", { command: `ls '${bigdir}'` }),
        find: toolTurn("Bash", { command: `find '${bigdir}' -type f` }),
        grep: toolTurn("Bash", { command: `grep -rn NEEDLE '${ws.ws.grepDir}'` }),
        transparency: toolTurn("Bash", { command: "printf 'alpha\\nbeta\\ngamma\\n'" }),
        operators: toolTurn("Bash", { command: "echo one && echo two | tr a-z A-Z" }),
      };
      const { mock } = await runScenario(ws, {
        toolTurns: Object.values(turns),
        prompt: "Run each shell command in turn.",
        settings: snipHooks(),
        timeoutMs: 150000,
      });
      const r = mock.toolResults();
      for (const [k, t] of Object.entries(turns)) seen[k] = r[t.toolUse.id];
    }

    // Scenario 2: git family (needs git).
    if (haveGit) {
      const ws = makeWorkspace();
      const repo = freshDir("gitrepo-");
      // 60 commits so `git log` (parseFormat=none) overflows the cap and the spec
      // produces a rewrite; 60 untracked so `git status` has plenty to fold.
      gitRepo(repo, { commits: 60, untracked: 60, modified: 60 });
      resetIds();
      const turns = {
        gitStatus: toolTurn("Bash", { command: `cd '${repo}' && git status` }),
        gitLog: toolTurn("Bash", { command: `cd '${repo}' && git log` }),
      };
      const { mock } = await runScenario(ws, {
        toolTurns: Object.values(turns),
        prompt: "Inspect the git repository.",
        settings: snipHooks(),
        timeoutMs: 150000,
      });
      const r = mock.toolResults();
      for (const [k, t] of Object.entries(turns)) seen[k] = r[t.toolUse.id];
    }
  });

  test("ls of a large directory is compacted ([snip: ls |])", { skip: SKIP }, () => {
    assert.ok(seen.ls, "model received the ls result");
    assert.match(seen.ls, /\[snip: ls \|/, "ls output was compacted");
  });

  test("find over many files is compacted ([snip: find |])", { skip: SKIP }, () => {
    assert.ok(seen.find, "model received the find result");
    assert.match(seen.find, /\[snip: find \|/, "find output was compacted");
  });

  test("grep over many matches is compacted ([snip: grep |])", { skip: SKIP }, () => {
    assert.ok(seen.grep, "model received the grep result");
    assert.match(seen.grep, /\[snip: grep \|/, "grep output was compacted");
  });

  test("git status is compacted ([snip: git-status |])", { skip: SKIP || (haveGit ? false : "git not available") }, () => {
    assert.ok(seen.gitStatus, "model received the git status result");
    assert.match(seen.gitStatus, /\[snip: git-status \|/, "git status was compacted via git_status_v2");
  });

  // snip's git-log family compacts by injecting flags so git emits the compact
  // one-line-per-commit form (hash + subject) instead of the full Author:/Date:/
  // body blocks — the savings come from the reshaped command, not a snip header.
  test("git log is compacted to one-line-per-commit (no Author:/Date: blocks)", { skip: SKIP || (haveGit ? false : "git not available") }, () => {
    assert.ok(seen.gitLog, "model received the git log result");
    assert.ok(!seen.gitLog.includes("Author:"), `git log compacted to oneline — got: ${JSON.stringify(seen.gitLog?.slice(0, 90))}`);
    assert.match(seen.gitLog, /^[0-9a-f]{7,}\s+\S/m, "one-line hash + subject format");
    assert.match(seen.gitLog, /commit number/, "commit subjects preserved");
  });

  test("transparency: small output reaches the model byte-for-byte (passthrough)", { skip: SKIP }, () => {
    assert.ok(seen.transparency, "model received the printf result");
    assert.match(seen.transparency, /alpha\nbeta\ngamma/, "exact output preserved");
    assert.ok(!seen.transparency.includes("[snip:"), "small output is passed through, not rewritten");
  });

  test("transparency: shell operators and pipes run correctly through the wrapper", { skip: SKIP }, () => {
    assert.ok(seen.operators, "model received the operators result");
    assert.match(seen.operators, /one/, "first command ran");
    assert.match(seen.operators, /TWO/, "piped tr ran on the second command");
  });
});
