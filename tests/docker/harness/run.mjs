// The glue every phase test shares: stand up an isolated $SNIP_HOME with the
// real binary, seed a working dir, register hooks via a settings.json, point a
// fresh mock server at one scripted scenario, and drive `claude` through it.
// Temp dirs are tracked so a test file can drop them all in an `after` hook.

import path from "node:path";

import { PLUGIN_ROOT, REPO_ROOT, freshDir, rmrf } from "./lib.mjs";
import { installBinary, snipHooks, spyHooks, writeSettings } from "./install.mjs";
import { snipBinary } from "./binary.mjs";
import { seedWorkspace } from "./fixtures.mjs";
import { MockAnthropic } from "./mock-anthropic.mjs";
import { runClaude } from "./claude-runner.mjs";

export { snipHooks, spyHooks };
export const SPY_SCRIPT = path.join(REPO_ROOT, "tests", "docker", "harness", "spy.mjs");

const TEMP = [];

/** Remove every temp dir created by `makeWorkspace` (call in `after`). */
export function cleanupTemp() {
  for (const d of TEMP) rmrf(d);
  TEMP.length = 0;
}

/**
 * Create an isolated workspace: $SNIP_HOME (binary installed unless
 * `withBinary:false`), a seeded working dir, and a Claude config dir.
 */
export function makeWorkspace({ withBinary = true } = {}) {
  const home = withBinary ? installBinary(snipBinary()) : freshDir("snip-home-");
  const cwd = freshDir("work-");
  const cfgDir = freshDir("cc-config-");
  const ws = seedWorkspace(cwd);
  TEMP.push(home, cwd, cfgDir);
  return { home, cwd, cfgDir, ws };
}

/**
 * Run one scripted scenario through `claude` against a fresh mock.
 * Returns `{ mock, res, settingsPath }` — the mock holds the recorded requests
 * (assert on `mock.toolResults()`), `res` the parsed `claude` JSON output.
 */
export async function runScenario(wsObj, opts) {
  const settingsPath = writeSettings(path.join(wsObj.cwd, ".snip-settings"), opts.settings);
  const mock = new MockAnthropic({ toolTurns: opts.toolTurns ?? [], finalText: opts.finalText ?? "ok" });
  await mock.start();
  let res;
  try {
    res = await runClaude({
      prompt: opts.prompt,
      cwd: wsObj.cwd,
      baseUrl: mock.baseUrl,
      snipHome: wsObj.home,
      pluginRoot: PLUGIN_ROOT,
      settingsPath,
      configDir: wsObj.cfgDir,
      pluginDir: opts.pluginDir,
      bare: opts.bare,
      model: opts.model,
      extraEnv: opts.extraEnv,
      timeoutMs: opts.timeoutMs,
      permissionArgs: opts.permissionArgs,
    });
  } finally {
    await mock.stop();
  }
  return { mock, res, settingsPath };
}
