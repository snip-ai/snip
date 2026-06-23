// Spawn the real `claude` CLI in headless mode, pinned at the fake model server
// (`baseUrl`). Every phase runs offline against the mock — no real API, no
// secrets. All flags/env are overridable so a local run can be tweaked without
// touching the tests; the defaults are chosen for a hermetic, non-interactive run
// that never prompts and never reaches an external service.

import { spawn } from "node:child_process";
import path from "node:path";

import { freshDir } from "./lib.mjs";

/** Best-effort parse of `--output-format json` stdout (tolerates stray lines). */
export function parseClaudeJson(stdout) {
  try {
    return JSON.parse(stdout);
  } catch {
    const i = stdout.indexOf("{");
    const j = stdout.lastIndexOf("}");
    if (i >= 0 && j > i) {
      try {
        return JSON.parse(stdout.slice(i, j + 1));
      } catch {
        /* fall through */
      }
    }
    return null;
  }
}

/**
 * Run one headless `claude -p` turn-loop.
 * @param {object} o
 * @param {string} o.prompt
 * @param {string} o.cwd            working dir (fixtures live here)
 * @param {string} o.baseUrl        ANTHROPIC_BASE_URL — the fake server (required)
 * @param {string} [o.snipHome]     $SNIP_HOME; its bin/ is prepended to PATH
 * @param {string} [o.pluginRoot]   CLAUDE_PLUGIN_ROOT for snip-run.sh
 * @param {string} [o.settingsPath] --settings file (hooks)
 * @param {string} [o.pluginDir]    --plugin-dir (real plugin lifecycle test)
 * @param {string} [o.apiKey]       ANTHROPIC_API_KEY (mock accepts anything)
 * @param {string} [o.model]
 * @param {boolean} [o.bare=true]   --bare (skip auto-discovery; hermetic)
 * @param {object} [o.extraEnv]
 * @param {number} [o.timeoutMs=60000]
 */
export async function runClaude(o) {
  const args = ["-p", o.prompt, "--output-format", "json"];
  // NOTE: `--bare` skips auto-discovery INCLUDING --settings hooks, so hooks
  // never fire under it. Hermeticity comes from an isolated CLAUDE_CONFIG_DIR +
  // a throwaway cwd instead. Opt in only when a test explicitly wants no hooks.
  if (o.bare === true) args.push("--bare");
  if (o.settingsPath) args.push("--settings", o.settingsPath);
  if (o.pluginDir) args.push("--plugin-dir", o.pluginDir);
  if (o.model) args.push("--model", o.model);
  args.push("--add-dir", o.cwd);
  // Headless sessions only expose non-core tools (Grep, Glob, …) when allowed;
  // Read/Bash are core. Pass the full surface set so every hook can fire.
  args.push("--allowedTools", o.allowedTools ?? "Read,Grep,Glob,Bash,Edit,Write");
  args.push(...(o.permissionArgs ?? ["--dangerously-skip-permissions"]));

  // Hard guarantee: every run is pinned to a fake server. No phase ever reaches
  // the real Anthropic API — the suite is offline, deterministic, and secretless.
  if (!o.baseUrl) throw new Error("runClaude requires baseUrl (the mock) — real API calls are not allowed");

  const binDir = o.snipHome ? path.join(o.snipHome, "bin") : null;
  const env = {
    ...process.env,
    // Always the mock URL + a placeholder key; the ambient real key is ignored.
    ANTHROPIC_BASE_URL: o.baseUrl,
    ANTHROPIC_API_KEY: o.apiKey ?? "sk-ant-mock-key",
    // Isolate Claude Code's own config so the host user's settings never leak in.
    CLAUDE_CONFIG_DIR: o.configDir ?? freshDir("cc-config-"),
    // Allow --dangerously-skip-permissions even when the test runs as root.
    IS_SANDBOX: "1",
    // Quiet the non-essential/external surfaces (best-effort; names may evolve).
    CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
    DISABLE_TELEMETRY: "1",
    DISABLE_ERROR_REPORTING: "1",
    DISABLE_AUTOUPDATER: "1",
    DISABLE_BUG_COMMAND: "1",
    ...o.extraEnv,
  };
  if (o.snipHome) env.SNIP_HOME = o.snipHome;
  if (o.pluginRoot) env.CLAUDE_PLUGIN_ROOT = o.pluginRoot;
  if (binDir) env.PATH = `${binDir}${path.delimiter}${process.env.PATH ?? ""}`;

  return new Promise((resolve) => {
    // stdin 'ignore' gives an immediate EOF — otherwise `claude -p` waits ~3s
    // for piped stdin before proceeding.
    const child = spawn("claude", args, { cwd: o.cwd, env, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      child.kill("SIGKILL");
    }, o.timeoutMs ?? 60000);

    child.stdout.on("data", (d) => (stdout += d));
    child.stderr.on("data", (d) => (stderr += d));
    child.on("error", (err) => {
      clearTimeout(timer);
      resolve({ code: null, stdout, stderr: `${stderr}\n${err.message}`, json: null, timedOut, args });
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      resolve({ code, stdout, stderr, json: parseClaudeJson(stdout), timedOut, args });
    });
  });
}
