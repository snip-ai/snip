// Phase B — deterministic conformance. The real `claude` binary drives a real
// agentic loop against the fake model, with snip installed exactly as the plugin
// installs it. Because the mock records every request, the `tool_result` the
// model received back IS the post-hook view — so these assertions prove the full
// chain (CC dispatch → plugin wrapper → snip binary → updatedToolOutput → CC
// consumption → model context), not just snip's hook-contract implementation.
//
// Run: `node --test tests/docker/phase-b-conformance.test.mjs` (needs `claude`).

import assert from "node:assert/strict";
import test, { after, describe } from "node:test";

import { checksumPaths, claudeAvailable } from "./harness/lib.mjs";
import { cleanupTemp, makeWorkspace, runScenario, snipHooks } from "./harness/run.mjs";
import { resetIds, toolTurn } from "./harness/scenario.mjs";

const SKIP = claudeAvailable() ? false : "the `claude` CLI is not on PATH";

after(cleanupTemp);

/** The text the model received back for a given scripted tool turn. */
function modelSaw(mock, turn) {
  return mock.toolResults()[turn.toolUse.id];
}

describe("Phase B — conformance (deterministic, fake model)", () => {
  test("Read: model receives snip's compacted view, comment stripped", { skip: SKIP }, async () => {
    // Arrange
    const ws = makeWorkspace();
    resetIds();
    const read = toolTurn("Read", { file_path: ws.ws.readPath });

    // Act
    const { mock, res } = await runScenario(ws, {
      toolTurns: [read],
      prompt: "Read commented.rs.",
      settings: snipHooks(),
      timeoutMs: 90000,
    });
    const seen = modelSaw(mock, read);

    // Assert
    assert.equal(res.json?.is_error, false, "claude completed without error");
    assert.ok(seen, "model received a Read tool_result");
    assert.match(seen, /\[snip: read \| rust/, "model saw snip's compaction header");
    assert.ok(!seen.includes("SNIP_SECRET_MARKER"), "the comment was stripped before the model saw it");
  });

  test("Grep: model receives snip's compacted search view", { skip: SKIP }, async () => {
    // Arrange
    const ws = makeWorkspace();
    resetIds();
    const grep = toolTurn("Grep", {
      pattern: ws.ws.grepPattern,
      path: ws.ws.grepDir,
      output_mode: "content",
    });

    // Act
    const { mock, res } = await runScenario(ws, {
      toolTurns: [grep],
      prompt: "Grep for NEEDLE in grepcorpus.",
      settings: snipHooks(),
      timeoutMs: 90000,
    });
    const seen = modelSaw(mock, grep);

    // Assert
    assert.equal(res.json?.is_error, false, "claude completed without error");
    assert.ok(seen, "model received a Grep tool_result");
    assert.match(seen, /\[snip: search/, "model saw snip's search compaction header");
  });

  // Real Claude Code Glob returns `{ filenames: [...] }` (a path array, no
  // content string), which the model renders joined by newlines. snip now
  // extracts/rewrites that shape (see `ToolResponse`), so the model sees a
  // directory-grouped view. This case caught that snip previously no-op'd on the
  // real Glob shape (the Rust e2e fixture's `{file:{content}}` masked it).
  test("Glob: model receives snip's compacted (grouped) view", { skip: SKIP }, async () => {
    // Arrange
    const ws = makeWorkspace();
    resetIds();
    const glob = toolTurn("Glob", { pattern: ws.ws.globPattern });

    // Act
    const { mock } = await runScenario(ws, {
      toolTurns: [glob],
      prompt: "Glob the rs files.",
      settings: snipHooks(),
      timeoutMs: 90000,
    });
    const seen = modelSaw(mock, glob);

    // Assert
    assert.match(seen ?? "", /\[snip: search/, "model should see a grouped Glob view");
  });

  test("Bash: bash-route wraps transparently; output stays correct", { skip: SKIP }, async () => {
    // Arrange
    const ws = makeWorkspace();
    resetIds();
    const bash = toolTurn("Bash", { command: "for i in $(seq 1 30); do echo dup; done" });

    // Act
    const { mock, res } = await runScenario(ws, {
      toolTurns: [bash],
      prompt: "Run the loop.",
      settings: snipHooks(),
      timeoutMs: 90000,
    });
    const seen = modelSaw(mock, bash);

    // Assert: snip ran the command verbatim through `snip exec`, so the model
    // still gets the real output — never a corrupted or empty result.
    assert.equal(res.json?.is_error, false, "claude completed without error");
    assert.ok(seen, "model received a Bash tool_result");
    assert.match(seen, /dup/, "the command's real output reached the model");
    assert.ok(!/error|not found|no such/i.test(seen), "wrapping did not break the command");
  });

  test("safety: snip never writes to the user's source files", { skip: SKIP }, async () => {
    // Arrange
    const ws = makeWorkspace();
    const before = checksumPaths(ws.ws.fixturePaths);
    resetIds();
    const read = toolTurn("Read", { file_path: ws.ws.readPath });
    const grep = toolTurn("Grep", { pattern: ws.ws.grepPattern, path: ws.ws.grepDir, output_mode: "content" });

    // Act
    await runScenario(ws, {
      toolTurns: [read, grep],
      prompt: "Read then grep.",
      settings: snipHooks(),
      timeoutMs: 90000,
    });
    const afterRun = checksumPaths(ws.ws.fixturePaths);

    // Assert
    assert.deepEqual(afterRun, before, "every seeded source file is byte-identical after the run");
  });

  test("graceful degradation: a missing binary yields raw passthrough, never an error", { skip: SKIP }, async () => {
    // Arrange: a $SNIP_HOME with no binary — snip-run.sh must no-op cleanly.
    const ws = makeWorkspace({ withBinary: false });
    resetIds();
    const read = toolTurn("Read", { file_path: ws.ws.readPath });

    // Act
    const { mock, res } = await runScenario(ws, {
      toolTurns: [read],
      prompt: "Read commented.rs.",
      settings: snipHooks(),
      timeoutMs: 90000,
    });
    const seen = modelSaw(mock, read);

    // Assert: the hook fired, found no binary, exited 0 → the model gets the
    // original file unchanged and Claude Code reports no hook failure.
    assert.equal(res.json?.is_error, false, "claude completed without error");
    assert.ok(seen?.includes("SNIP_SECRET_MARKER"), "model saw the original (uncompacted) content");
    assert.ok(!seen.includes("[snip:"), "no snip rewrite when the binary is absent");
  });
});
