// Phase B — real plugin install/discovery (the production load path).
//
// Every other Phase B test wires snip's hooks through `--settings` (the
// `snipHooks()` helper), which fabricates a settings.json `hooks` block and never
// touches the plugin manifest. That bypass is exactly why the v0.1.0 regression
// slipped through: the manifest declared `"hooks": "./hooks/hooks.json"`, but
// Claude Code already auto-loads that standard path — so a real install hit
// "Duplicate hooks file detected" -> hook-load-failed and EVERY hook silently
// unregistered (only slash-commands worked). The `--settings` path could never
// reproduce it.
//
// This test instead loads snip the way a marketplace install does: `claude
// --plugin-dir <plugin root>`, so Claude Code discovers the plugin from
// `.claude-plugin/plugin.json` and auto-loads `hooks/hooks.json` — manifest
// duplicate-check included. We then prove the hooks actually registered by
// observing that the model received snip's compacted Read view, and assert Claude
// Code reported no plugin/hook load failure.
//
// Run: `node --test tests/docker/phase-b-plugin-install.test.mjs` (needs `claude`).

import assert from "node:assert/strict";
import test, { after, describe } from "node:test";

import { PLUGIN_ROOT, claudeAvailable } from "./harness/lib.mjs";
import { cleanupTemp, makeWorkspace, runScenario } from "./harness/run.mjs";
import { resetIds, toolTurn } from "./harness/scenario.mjs";

const SKIP = claudeAvailable() ? false : "the `claude` CLI is not on PATH";

after(cleanupTemp);

/** The text the model received back for a given scripted tool turn. */
function modelSaw(mock, turn) {
  return mock.toolResults()[turn.toolUse.id];
}

describe("Phase B — real plugin discovery (manifest, not --settings)", () => {
  test("plugin loaded from its manifest registers hooks; the Read hook fires", { skip: SKIP }, async () => {
    // Arrange: install the binary + seed fixtures, but DO NOT inject hooks via
    // settings. An empty settings block means the ONLY hooks in play are the ones
    // Claude Code discovers from the plugin manifest below.
    const ws = makeWorkspace();
    resetIds();
    const read = toolTurn("Read", { file_path: ws.ws.readPath });

    // Act: `--plugin-dir <plugins/snip>` is the dev-mode equivalent of a real
    // install — Claude Code loads `.claude-plugin/plugin.json` and auto-loads
    // `hooks/hooks.json`, running the same manifest-duplicate check production does.
    const { mock, res } = await runScenario(ws, {
      toolTurns: [read],
      prompt: "Read commented.rs.",
      settings: {},
      pluginDir: PLUGIN_ROOT,
      timeoutMs: 90000,
    });
    const seen = modelSaw(mock, read);
    const streams = `${res.stderr ?? ""}\n${res.stdout ?? ""}`;

    // Assert: the Read PostToolUse hook fired (so it registered from the manifest)
    // and Claude Code surfaced no duplicate-hooks / hook-load-failed error. If the
    // manifest re-declared the auto-loaded hooks file, NO hook would register and
    // the model would receive the raw, uncompacted file (SNIP_SECRET_MARKER intact).
    assert.equal(res.json?.is_error, false, "claude completed without error");
    assert.ok(seen, "model received a Read tool_result");
    assert.match(seen, /\[snip: read \| rust/, "the plugin's Read hook fired — manifest discovery registered it");
    assert.ok(!seen.includes("SNIP_SECRET_MARKER"), "the comment was stripped — the hook really ran");
    assert.doesNotMatch(streams, /duplicate hooks file detected/i, "no duplicate-hooks manifest error");
    assert.doesNotMatch(streams, /hook-load-failed/i, "Claude Code reported no hook-load failure for the plugin");
  });
});
