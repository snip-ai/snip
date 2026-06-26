// Phase B (lifecycle banner) — the lifecycle `systemMessage` reaches the USER,
// never the MODEL, through the REAL `claude` CLI.
//
// snip's SessionStart hook (`snip-run.sh update-check`) runs the installed binary,
// which consumes a pending `.lifecycle` sentinel and prints a one-line
// `systemMessage` banner. Per the hook protocol, `systemMessage` is shown to the
// user but is NOT added to the model's context. This drives a real `claude -p`
// against a loopback mock Anthropic server and asserts the banner text never
// appears in any request sent to the model, and that the sentinel is consumed.
// Requires the `claude` CLI. Run: `node --test tests/docker/phase-b-lifecycle-banner.test.mjs`.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test, { after, describe } from "node:test";
import assert from "node:assert/strict";

import { PLUGIN_ROOT, SNIP_RUN, claudeAvailable, freshDir } from "./harness/lib.mjs";
import { snipBinary } from "./harness/binary.mjs";
import { installBinary } from "./harness/install.mjs";
import { MockAnthropic } from "./harness/mock-anthropic.mjs";
import { runClaude } from "./harness/claude-runner.mjs";
import { cleanupTemp } from "./harness/run.mjs";

const haveBash = spawnSync("bash", ["--version"], { encoding: "utf8" }).status === 0;
const SKIP = !claudeAvailable()
  ? "claude CLI not available"
  : !haveBash
    ? "bash not available"
    : false;

// Forward-slash the script/root paths so `${0%/*}` in snip-run.sh resolves on
// Windows too (real hooks pass a `/`-joined path). No-op on POSIX.
const RUN = SNIP_RUN.replace(/\\/g, "/");
const ROOT = PLUGIN_ROOT.replace(/\\/g, "/");
const DEAD = "http://127.0.0.1:1";

after(cleanupTemp);

describe("Phase B — lifecycle banner reaches the user, not the model", () => {
  test("a pending .lifecycle surfaces as a user-only systemMessage and is consumed", { skip: SKIP }, async () => {
    // Arrange: an installed binary with a pending lifecycle event, and the throttle
    // already satisfied so update-check does not spawn a background self-update.
    const home = installBinary(snipBinary());
    const lifecycle = path.join(home, ".lifecycle");
    fs.writeFileSync(lifecycle, "installed 9.9.9\n");
    const now = Math.floor(Date.now() / 1000);
    fs.writeFileSync(path.join(home, ".update-check"), String(now));

    const settings = {
      hooks: {
        SessionStart: [{ hooks: [{ type: "command", command: `bash "${RUN}" update-check`, timeout: 10 }] }],
      },
    };
    const cfgDir = freshDir("cc-cfg-");
    const settingsPath = path.join(cfgDir, "settings.json");
    fs.writeFileSync(settingsPath, JSON.stringify(settings));
    const cwd = freshDir("cwd-");

    const mock = new MockAnthropic({ finalText: "done" });
    const baseUrl = await mock.start();

    // Act: a real headless claude session; its SessionStart fires snip's banner.
    const res = await runClaude({
      prompt: "Reply with the single word ok.",
      baseUrl,
      cwd,
      settingsPath,
      configDir: freshDir("cc-config-"),
      snipHome: home,
      pluginRoot: ROOT,
      allowedTools: "Read",
      extraEnv: { SNIP_DOWNLOAD_BASE: DEAD, SNIP_RELEASES_API: DEAD },
      timeoutMs: 90000,
    });
    await mock.stop();

    // Assert: the banner text never reached the model, and the sentinel is consumed.
    const sentToModel = JSON.stringify(mock.requests);
    assert.equal(res.code, 0, "claude exits 0");
    assert.equal(
      sentToModel.includes("snip: installed v9.9.9"),
      false,
      "the lifecycle banner must NOT appear in any request sent to the model",
    );
    assert.equal(sentToModel.includes("9.9.9"), false, "no trace of the banner version in the model context");
    assert.equal(fs.existsSync(lifecycle), false, "update-check consumed the .lifecycle sentinel");
  });
});
