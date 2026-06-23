// Phase B (languages) — every language snip supports, through the real Read tool.
// For each of the 29 languages, the model must receive snip's compacted view
// ("[snip: read | <name>") with the buried marker comment stripped. The reads are
// batched into a few real `claude` sessions (one tool call per file) to keep the
// claude spawn count low. A failure here means a language is mis-detected or its
// comments aren't stripped through the real pipeline — exactly what the existing
// unit/e2e tiers (which feed synthetic shapes) can't catch.
//
// Run: `node --test tests/docker/phase-b-languages.test.mjs` (needs `claude`).

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test, { after, before, describe } from "node:test";

import { claudeAvailable } from "./harness/lib.mjs";
import { LANGS, SECRET_MARKER } from "./harness/languages.mjs";
import { cleanupTemp, makeWorkspace, runScenario, snipHooks } from "./harness/run.mjs";
import { resetIds, toolTurn } from "./harness/scenario.mjs";

const SKIP = claudeAvailable() ? false : "the `claude` CLI is not on PATH";
const BATCH = 10;

after(cleanupTemp);

function chunk(arr, n) {
  const out = [];
  for (let i = 0; i < arr.length; i += n) out.push(arr.slice(i, i + n));
  return out;
}

describe("Phase B — all languages (Read compaction)", () => {
  /** language name → the text the model received for its Read. */
  const seen = {};

  before(async () => {
    if (SKIP) return;
    for (const batch of chunk(LANGS, BATCH)) {
      const ws = makeWorkspace();
      resetIds();
      const turns = batch.map((lang) => {
        const file = path.join(ws.cwd, `sample.${lang.ext}`);
        fs.writeFileSync(file, lang.src);
        const turn = toolTurn("Read", { file_path: file });
        turn._lang = lang.name;
        return turn;
      });
      const { mock } = await runScenario(ws, {
        toolTurns: turns,
        prompt: "Read each source file in this directory, one at a time.",
        settings: snipHooks(),
        timeoutMs: 180000,
      });
      const results = mock.toolResults();
      for (const turn of turns) seen[turn._lang] = results[turn.toolUse.id];
    }
  });

  for (const lang of LANGS) {
    test(`${lang.name}: model receives the compacted view, comment stripped`, { skip: SKIP }, () => {
      // Assert
      const view = seen[lang.name];
      assert.ok(view, `the model received a Read result for ${lang.name}`);
      assert.ok(
        view.includes(`[snip: read | ${lang.name}`),
        `header should name language "${lang.name}" — got: ${JSON.stringify(view.slice(0, 70))}`,
      );
      assert.ok(!view.includes(SECRET_MARKER), `the comment must be stripped for ${lang.name}`);
    });
  }
});
