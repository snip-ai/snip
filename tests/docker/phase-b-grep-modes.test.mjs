// Phase B (Grep modes) — the Grep tool's output_mode variants beyond `content`
// (covered in conformance). `files_with_matches` emits a "Found N files" header
// plus a bare path list in top-level `content`, which snip groups by directory
// (the `auto` group key folds by dir when a record has no `:line:` segment);
// `count` returns per-file counts, which must reach the model intact. This
// exercises the Grep tool's full surface, not just one mode.
//
// Run: `node --test tests/docker/phase-b-grep-modes.test.mjs` (needs `claude`).

import fs from "node:fs";
import path from "node:path";
import test, { after, describe } from "node:test";
import assert from "node:assert/strict";

import { claudeAvailable } from "./harness/lib.mjs";
import { cleanupTemp, makeWorkspace, runScenario, snipHooks } from "./harness/run.mjs";
import { resetIds, toolTurn } from "./harness/scenario.mjs";

const SKIP = claudeAvailable() ? false : "the `claude` CLI is not on PATH";

after(cleanupTemp);

/** A directory of `n` files under a shared deep prefix, each matching NEEDLE. */
function matchCorpus(cwd) {
  const base = path.join(cwd, "matchdir", "deep", "nested");
  fs.mkdirSync(base, { recursive: true });
  for (let i = 0; i < 50; i++) {
    fs.writeFileSync(path.join(base, `m${String(i).padStart(2, "0")}.txt`), "NEEDLE here\nNEEDLE again\n");
  }
  return path.join(cwd, "matchdir");
}

describe("Phase B — Grep output modes (real Grep tool)", () => {
  // Real Grep `files_with_matches` emits a `{content: "Found N files\\n<bare
  // paths>"}` string on the GREP surface. The bare path list has no
  // "path:line:match" segment, so the `grep`-surface spec groups it by directory
  // (via the `auto` group key, which falls back to dir grouping when a record
  // lacks a `:line:` segment): a long shared prefix collapses to one header. This
  // asserts both the grouped `[snip: search` view and that the matching files
  // still reach the model.
  test("files_with_matches: snip groups the path list by directory", { skip: SKIP }, async () => {
    // Arrange
    const ws = makeWorkspace();
    const dir = matchCorpus(ws.cwd);
    resetIds();
    const grep = toolTurn("Grep", { pattern: "NEEDLE", path: dir, output_mode: "files_with_matches" });

    // Act
    const { mock, res } = await runScenario(ws, {
      toolTurns: [grep],
      prompt: "List the files matching NEEDLE.",
      settings: snipHooks(),
      timeoutMs: 90000,
    });
    const seen = mock.toolResults()[grep.toolUse.id];

    // Assert: the path list is folded under one directory header, and the matching
    // files still reach the model uncorrupted.
    assert.equal(res.json?.is_error, false, "claude completed without error");
    assert.ok(seen, "model received the grep result");
    assert.match(seen, /\[snip: search/, "the path list was grouped, not passed through");
    assert.match(seen, /m\d+\.txt/, "the matching file paths reached the model");
  });

  test("count: per-file counts reach the model intact", { skip: SKIP }, async () => {
    // Arrange
    const ws = makeWorkspace();
    const dir = matchCorpus(ws.cwd);
    resetIds();
    const grep = toolTurn("Grep", { pattern: "NEEDLE", path: dir, output_mode: "count" });

    // Act
    const { mock, res } = await runScenario(ws, {
      toolTurns: [grep],
      prompt: "Count the matches of NEEDLE per file.",
      settings: snipHooks(),
      timeoutMs: 90000,
    });
    const seen = mock.toolResults()[grep.toolUse.id];

    // Assert: counts are numeric and the run is healthy (no corruption).
    assert.equal(res.json?.is_error, false, "claude completed without error");
    assert.ok(seen, "model received the count result");
    assert.match(seen, /\d/, "the count result carries numbers");
  });
});
