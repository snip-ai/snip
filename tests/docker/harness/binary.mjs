// Locate the snip binary to install for a run. In Docker the entrypoint exports
// SNIP_TEST_BINARY (the musl build); locally we fall back to the release build
// under target/. Failing to find it is a hard error — the phase tests must run
// the REAL binary, never silently skip it.

import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import { REPO_ROOT } from "./lib.mjs";

const BIN = os.platform() === "win32" ? "snip.exe" : "snip";

const CANDIDATES = [
  () => process.env.SNIP_TEST_BINARY,
  () => path.join(REPO_ROOT, "target", "release", BIN),
  () => path.join(REPO_ROOT, "target", "x86_64-unknown-linux-musl", "release", BIN),
  () => path.join(REPO_ROOT, "target", "aarch64-unknown-linux-musl", "release", BIN),
];

/** Absolute path to a built snip binary, or throw with a build hint. */
export function snipBinary() {
  for (const c of CANDIDATES) {
    const p = c();
    if (p && fs.existsSync(p)) return p;
  }
  throw new Error(
    "no snip binary found — set SNIP_TEST_BINARY or run `cargo build --release` " +
      `(looked under ${REPO_ROOT}/target/…)`,
  );
}
