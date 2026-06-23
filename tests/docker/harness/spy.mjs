#!/usr/bin/env node
// A pass-through "spy" hook for Phase A. It captures the verbatim hook JSON
// Claude Code sends on stdin (whose `tool_response`/`tool_input` is the REAL
// shape the real tool surface produced), writes it under $SNIP_SPY_DIR, then
// exits 0 with empty stdout — i.e. it changes nothing the model sees. Diffing
// those captures against the shapes the Rust e2e fixtures hard-code is the
// contract-drift canary.

import fs from "node:fs";
import path from "node:path";

const dir = process.env.SNIP_SPY_DIR;
if (!dir) process.exit(0); // misconfigured → behave like any hook: safe no-op

const chunks = [];
process.stdin.on("data", (c) => chunks.push(c));
process.stdin.on("end", () => {
  try {
    fs.mkdirSync(dir, { recursive: true });
    const name = `cap-${process.hrtime.bigint()}-${process.pid}.json`;
    fs.writeFileSync(path.join(dir, name), Buffer.concat(chunks));
  } catch {
    /* a capture failure must never break the run */
  }
  process.exit(0);
});
process.stdin.on("error", () => process.exit(0));
