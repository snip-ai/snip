// Shared leaf utilities for the Docker e2e harness: paths, temp dirs, checksums,
// the chars/4 token heuristic (mirrors `crate::tokens::estimate_tokens`), and a
// poll helper. Dependency-free (Node stdlib only), so the runtime image needs
// nothing beyond the Node that Claude Code already requires.

import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));

/** Repository root (…/tests/docker/harness → three levels up). */
export const REPO_ROOT = path.resolve(HERE, "..", "..", "..");

/** The snip plugin directory — the same scripts Claude Code runs in production. */
export const PLUGIN_ROOT = path.join(REPO_ROOT, "plugins", "snip");

/** The plugin's hook wrapper — `snip-run.sh <subcommand>`. */
export const SNIP_RUN = path.join(PLUGIN_ROOT, "scripts", "snip-run.sh");

/** The plugin's self-install script — `snip-bootstrap.sh <version> <home>`. */
export const SNIP_BOOTSTRAP = path.join(PLUGIN_ROOT, "scripts", "snip-bootstrap.sh");

export const SNIP_SHELL_SETUP = path.join(PLUGIN_ROOT, "scripts", "snip-shell-setup.sh");

/** A fresh auto-namespaced temp directory under the OS tmp root. */
export function freshDir(prefix = "snip-docker-") {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

/** Recursively remove a directory, ignoring errors (best-effort cleanup). */
export function rmrf(dir) {
  try {
    fs.rmSync(dir, { recursive: true, force: true });
  } catch {
    /* best-effort */
  }
}

/** SHA-256 of a file's bytes, or `null` if it does not exist. */
export function sha256File(file) {
  try {
    return createHash("sha256").update(fs.readFileSync(file)).digest("hex");
  } catch {
    return null;
  }
}

/** Map of every file under `dir` (relative path → SHA-256) for tamper checks. */
export function checksumTree(dir) {
  const out = {};
  const walk = (d) => {
    for (const entry of fs.readdirSync(d, { withFileTypes: true })) {
      const abs = path.join(d, entry.name);
      if (entry.isDirectory()) walk(abs);
      else if (entry.isFile()) out[path.relative(dir, abs)] = sha256File(abs);
    }
  };
  walk(dir);
  return out;
}

/** Checksum a mix of file and directory paths into one flat relative→hash map. */
export function checksumPaths(paths) {
  const out = {};
  for (const p of paths) {
    if (!fs.existsSync(p)) continue;
    if (fs.statSync(p).isDirectory()) {
      for (const [rel, hash] of Object.entries(checksumTree(p))) {
        out[`${path.basename(p)}/${rel}`] = hash;
      }
    } else {
      out[path.basename(p)] = sha256File(p);
    }
  }
  return out;
}

/** The same chars/4 token estimate snip uses, so Phase C figures line up. */
export function estimateTokens(text) {
  return Math.ceil((text ?? "").length / 4);
}

/**
 * Best-effort write of an optional artifact under $SNIP_DOCKER_ARTIFACTS. Pure
 * telemetry — a non-writable dir must NEVER fail a test, so all errors are
 * swallowed. No-op when the env var is unset.
 */
export function writeArtifact(name, obj) {
  const dir = process.env.SNIP_DOCKER_ARTIFACTS;
  if (!dir) return;
  try {
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(path.join(dir, name), JSON.stringify(obj, null, 2));
  } catch {
    /* artifacts are optional; never break a run over them */
  }
}

/** Whether the `claude` CLI is on PATH and answers `--version`. */
export function claudeAvailable() {
  const r = spawnSync("claude", ["--version"], { encoding: "utf8" });
  return r.status === 0;
}

/** The `claude` CLI version string (e.g. "2.1.175"), or `null`. */
export function claudeVersion() {
  const r = spawnSync("claude", ["--version"], { encoding: "utf8" });
  if (r.status !== 0) return null;
  return (r.stdout || "").trim();
}

/**
 * Spawn a process and resolve `{ status, stdout, stderr }` — the async sibling of
 * `spawnSync`. Use this (never `spawnSync`) when an in-process HTTP server must
 * stay responsive during the child: `spawnSync` blocks the event loop, so a
 * child that calls back into a loopback server would deadlock.
 */
export function spawnAsync(cmd, args, opts = {}) {
  return new Promise((resolve) => {
    const child = spawn(cmd, args, opts);
    let stdout = "";
    let stderr = "";
    child.stdout?.setEncoding("utf8");
    child.stderr?.setEncoding("utf8");
    child.stdout?.on("data", (d) => (stdout += d));
    child.stderr?.on("data", (d) => (stderr += d));
    child.on("error", (e) => resolve({ status: null, stdout, stderr: stderr + e.message }));
    child.on("close", (status) => resolve({ status, stdout, stderr }));
  });
}

/** Poll `predicate` every `stepMs` until truthy or `timeoutMs` elapses. */
export async function waitUntil(predicate, { timeoutMs = 5000, stepMs = 50 } = {}) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    if (predicate()) return true;
    if (Date.now() > deadline) return false;
    await new Promise((r) => setTimeout(r, stepMs));
  }
}
