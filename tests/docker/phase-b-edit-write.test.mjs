// Phase B (Edit/Write) — the input-rewriting surfaces, through the real tools.
// Covers: an Edit applying end-to-end with edit-fix present (verbatim passthrough),
// a Write creating a new file (write-guard passthrough), and the documented live
// recovery path `snip resolve` mapping a comment-stripped block back to the real
// bytes through the real Bash tool. (Claude Code >=2.1.x pre-validates old_string
// before PreToolUse, so `snip resolve` — not the edit-fix hook — is the live path.)
//
// Run: `node --test tests/docker/phase-b-edit-write.test.mjs` (needs `claude`).

import fs from "node:fs";
import path from "node:path";
import test, { after, describe } from "node:test";
import assert from "node:assert/strict";

import { claudeAvailable } from "./harness/lib.mjs";
import { cleanupTemp, makeWorkspace, runScenario, snipHooks } from "./harness/run.mjs";
import { resetIds, toolTurn } from "./harness/scenario.mjs";

const SKIP = claudeAvailable() ? false : "the `claude` CLI is not on PATH";

after(cleanupTemp);

describe("Phase B — Edit/Write surfaces (real tools)", () => {
  test("Edit: a verbatim edit applies through Claude Code (edit-fix passes through)", { skip: SKIP }, async () => {
    // Arrange
    const ws = makeWorkspace();
    const file = path.join(ws.cwd, "editme.txt");
    fs.writeFileSync(file, "line one\nTARGET line\nline three\n");
    resetIds();
    const read = toolTurn("Read", { file_path: file });
    const edit = toolTurn("Edit", { file_path: file, old_string: "TARGET line", new_string: "REPLACED line" });

    // Act
    const { res } = await runScenario(ws, {
      toolTurns: [read, edit],
      prompt: "Read the file, then replace TARGET line with REPLACED line.",
      settings: snipHooks(),
      timeoutMs: 90000,
    });
    const after = fs.readFileSync(file, "utf8");

    // Assert
    assert.equal(res.json?.is_error, false, "claude completed without error");
    assert.match(after, /REPLACED line/, "the edit was applied to the real file");
    assert.ok(!after.includes("TARGET line"), "the old text is gone");
  });

  test("Write: a new file is created through Claude Code (write-guard passes through)", { skip: SKIP }, async () => {
    // Arrange
    const ws = makeWorkspace();
    const file = path.join(ws.cwd, "created.txt");
    resetIds();
    const write = toolTurn("Write", { file_path: file, content: "hello\nworld\n" });

    // Act
    const { res } = await runScenario(ws, {
      toolTurns: [write],
      prompt: "Create created.txt with the given content.",
      settings: snipHooks(),
      timeoutMs: 90000,
    });

    // Assert
    assert.equal(res.json?.is_error, false, "claude completed without error");
    assert.ok(fs.existsSync(file), "the new file was created");
    assert.equal(fs.readFileSync(file, "utf8"), "hello\nworld\n", "with the exact content");
  });

  test("Edit recovery: `snip resolve` maps a comment-stripped block back to real bytes (via Bash)", { skip: SKIP }, async () => {
    // Arrange: soft compaction strips the inline comment, so a block copied from
    // the compacted view is no longer a verbatim substring of the real file.
    const ws = makeWorkspace();
    const file = path.join(ws.cwd, "resolveme.rs");
    fs.writeFileSync(file, "fn f() {\n    let x = 1; // alpha\n    let y = 2;\n}\n");
    resetIds();
    const read = toolTurn("Read", { file_path: file });
    // The model copies the comment-stripped two-line block and pipes it to resolve.
    const resolve = toolTurn("Bash", {
      command: `printf '    let x = 1;\\n    let y = 2;' | snip resolve '${file}'`,
    });

    // Act
    const { mock } = await runScenario(ws, {
      toolTurns: [read, resolve],
      prompt: "Read the file, then resolve the stripped block.",
      settings: snipHooks(),
      timeoutMs: 90000,
    });
    const resolved = mock.toolResults()[resolve.toolUse.id];

    // Assert: resolve restored the inline comment that the compacted view dropped.
    assert.ok(resolved, "model received the resolve output");
    assert.match(resolved, /alpha/, `resolve restored the real bytes — got: ${JSON.stringify(resolved?.slice(0, 100))}`);
  });
});
