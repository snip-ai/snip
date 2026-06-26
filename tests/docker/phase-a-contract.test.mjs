// Phase A — contract-drift canary. snip's optimizers read specific fields out of
// each tool's `tool_response` / `tool_input`. If a Claude Code release changes
// those shapes, snip silently no-ops (deserialization finds nothing) while every
// unit/e2e test — which feeds hand-written shapes — stays green. This phase runs
// the REAL `claude` over fixtures with a pass-through spy hook that captures the
// verbatim payloads, and asserts the fields snip depends on are still present.
//
// (This is the phase that found the Glob `filenames` divergence; the assertions
// below now encode the corrected, real contract.)
//
// Run: `node --test tests/docker/phase-a-contract.test.mjs` (needs `claude`).

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test, { after, before, describe } from "node:test";

import { claudeAvailable, freshDir, writeArtifact } from "./harness/lib.mjs";
import { cleanupTemp, makeWorkspace, runScenario, spyHooks, SPY_SCRIPT } from "./harness/run.mjs";
import { resetIds, toolTurn } from "./harness/scenario.mjs";

const SKIP = claudeAvailable() ? false : "the `claude` CLI is not on PATH";

after(cleanupTemp);

describe("Phase A — contract drift canary", () => {
  /** tool_name → the verbatim hook payload Claude Code sent. */
  const captured = {};

  before(async () => {
    if (SKIP) return;
    const ws = makeWorkspace();
    const spyDir = freshDir("spy-");
    resetIds();
    const turns = [
      toolTurn("Read", { file_path: ws.ws.readPath }),
      toolTurn("Grep", { pattern: ws.ws.grepPattern, path: ws.ws.grepDir, output_mode: "content" }),
      toolTurn("Glob", { pattern: ws.ws.globPattern }),
      toolTurn("Bash", { command: "echo contract-probe" }),
    ];
    await runScenario(ws, {
      toolTurns: turns,
      prompt: "Inspect the workspace with Read, Grep, Glob, and a Bash echo.",
      settings: spyHooks(SPY_SCRIPT),
      extraEnv: { SNIP_SPY_DIR: spyDir },
      timeoutMs: 120000,
    });
    for (const f of fs.readdirSync(spyDir)) {
      const j = JSON.parse(fs.readFileSync(path.join(spyDir, f), "utf8"));
      captured[j.tool_name] = j;
    }
    // The observed shapes, printed as the canary's current "contract of record".
    for (const [tool, p] of Object.entries(captured)) {
      const tr = p.tool_response;
      const shape = tr && typeof tr === "object" ? Object.keys(tr) : typeof tr;
      const fileKeys = tr?.file && typeof tr.file === "object" ? Object.keys(tr.file) : null;
      console.log(`[contract] ${tool}: tool_response=${JSON.stringify(shape)}${fileKeys ? ` file=${JSON.stringify(fileKeys)}` : ""}`);
    }
    // Optional machine-readable artifact for tracking drift over time.
    writeArtifact(
      "observed-shapes.json",
      Object.fromEntries(Object.entries(captured).map(([t, p]) => [t, { tool_input: p.tool_input, tool_response: p.tool_response }])),
    );
  });

  test("Read: tool_response.file.content is still a string (read optimizer)", { skip: SKIP }, () => {
    const tr = captured.Read?.tool_response;
    assert.ok(tr, "the Read PostToolUse hook fired with a tool_response");
    assert.equal(tr.type, "text", "Read tool_response.type");
    assert.equal(typeof tr.file?.content, "string", "snip reads tool_response.file.content");
    assert.equal(typeof tr.file?.filePath, "string", "snip's header uses tool_response.file.filePath");
    assert.equal(typeof tr.file?.numLines, "number", "snip rewrites tool_response.file.numLines");
    // Claude Code >= 2.1.x adds `startLine`/`totalLines` to the windowed-read file
    // object; snip does not read them but MUST preserve them through its rewrite
    // (covered version-independently by the ToolResponse round-trip unit test).
    // Assert their type when present without requiring them, so the canary tolerates
    // the pinned older client yet flags a type change on newer ones.
    for (const k of ["startLine", "totalLines"]) {
      if (k in tr.file) assert.equal(typeof tr.file[k], "number", `Read file.${k} (when present) is numeric`);
    }
  });

  test("Grep: tool_response.content is still a top-level string (search optimizer)", { skip: SKIP }, () => {
    const tr = captured.Grep?.tool_response;
    assert.ok(tr, "the Grep PostToolUse hook fired with a tool_response");
    assert.equal(typeof tr.content, "string", "snip reads top-level tool_response.content for Grep");
  });

  test("Glob: tool_response.filenames is still an array (search optimizer)", { skip: SKIP }, () => {
    const tr = captured.Glob?.tool_response;
    assert.ok(tr, "the Glob PostToolUse hook fired with a tool_response");
    assert.ok(Array.isArray(tr.filenames), "snip reads tool_response.filenames for Glob");
    assert.equal(tr.content, undefined, "Glob still has no content field (the reason snip must read filenames)");
  });

  test("Bash: tool_input.command is still a string on PreToolUse (bash-route)", { skip: SKIP }, () => {
    const p = captured.Bash;
    assert.ok(p, "the Bash PreToolUse hook fired");
    assert.equal(p.hook_event_name, "PreToolUse", "bash-route is a PreToolUse hook");
    assert.equal(typeof p.tool_input?.command, "string", "bash-route reads tool_input.command");
  });

  test("hook payload still carries the documented envelope fields", { skip: SKIP }, () => {
    const p = captured.Read;
    for (const k of ["session_id", "hook_event_name", "tool_name", "tool_input", "tool_response"]) {
      assert.ok(k in p, `hook payload exposes ${k}`);
    }
    assert.equal(p.hook_event_name, "PostToolUse");
    assert.equal(p.tool_name, "Read");
  });
});
