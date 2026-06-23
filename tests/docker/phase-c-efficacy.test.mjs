// Phase C — efficacy, measured deterministically and offline. Instead of a real
// model run (no real API, no secrets, no flakiness), we replay the SAME scripted
// tool calls twice — snip OFF then ON — and diff the tokens of the tool_results
// the model actually received (the mock records them verbatim). Because the tool
// calls are identical across both runs, the difference is purely snip's effect: a
// precise, reproducible savings figure on the real bytes Claude Code would feed
// the model. Token counts use snip's own chars/4 heuristic so figures line up.
//
// Run: `node --test tests/docker/phase-c-efficacy.test.mjs` (needs `claude`).

import assert from "node:assert/strict";
import test, { after, describe } from "node:test";

import { claudeAvailable, estimateTokens, writeArtifact } from "./harness/lib.mjs";
import { cleanupTemp, makeWorkspace, runScenario, snipHooks } from "./harness/run.mjs";
import { resetIds, toolTurn } from "./harness/scenario.mjs";

const SKIP = claudeAvailable() ? false : "the `claude` CLI is not on PATH";

after(cleanupTemp);

const SURFACES = ["Read", "Grep", "Glob"];

/** Replay the fixed Read/Grep/Glob scenario under `settings`, returning the
 *  per-surface token count of what the model received. */
async function measure(settings) {
  const ws = makeWorkspace();
  resetIds();
  const turns = {
    Read: toolTurn("Read", { file_path: ws.ws.readPath }),
    Grep: toolTurn("Grep", { pattern: ws.ws.grepPattern, path: ws.ws.grepDir, output_mode: "content" }),
    Glob: toolTurn("Glob", { pattern: ws.ws.globPattern }),
  };
  const { mock, res } = await runScenario(ws, {
    toolTurns: [turns.Read, turns.Grep, turns.Glob],
    prompt: "Inspect the workspace with Read, Grep, and Glob.",
    settings,
    timeoutMs: 120000,
  });
  const results = mock.toolResults();
  const perSurface = {};
  for (const s of SURFACES) perSurface[s] = estimateTokens(results[turns[s].toolUse.id] ?? "");
  return { res, perSurface };
}

describe("Phase C — efficacy (deterministic, offline)", () => {
  test("snip reduces the model-visible tokens per surface and overall, never inflating", { skip: SKIP }, async () => {
    // Arrange + Act: identical tool calls, snip off then on.
    const off = await measure({ hooks: {} });
    const on = await measure(snipHooks());

    // Assert: both runs were healthy.
    assert.equal(off.res.json?.is_error, false, "baseline run completed without error");
    assert.equal(on.res.json?.is_error, false, "snip run completed without error");

    // Assert + report: per surface, snip never inflates; report the delta.
    const rows = [];
    let totalOff = 0;
    let totalOn = 0;
    for (const s of SURFACES) {
      const a = off.perSurface[s];
      const b = on.perSurface[s];
      totalOff += a;
      totalOn += b;
      const pct = a ? Math.round((1 - b / a) * 100) : 0;
      rows.push({ surface: s, off: a, on: b, savedPct: pct });
      console.log(`[efficacy] ${s}: ${a} → ${b} tok (-${pct}%)`);
      assert.ok(b <= a, `${s}: snip must never inflate the model-visible view (${b} > ${a})`);
    }
    const totalPct = totalOff ? Math.round((1 - totalOn / totalOff) * 100) : 0;
    console.log(`[efficacy] TOTAL: ${totalOff} → ${totalOn} tok (-${totalPct}%)`);

    // Assert: on this realistic mix, snip delivers a net reduction.
    assert.ok(totalOn < totalOff, "snip reduces total model-visible tokens on the Read/Grep/Glob mix");

    writeArtifact("efficacy.json", { rows, totalOff, totalOn, totalPct, estimator: "chars/4 (snip heuristic)" });
  });
});
