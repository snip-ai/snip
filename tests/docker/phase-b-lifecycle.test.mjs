// Phase B (lifecycle) — the plugin self-install path, deterministic and offline.
// The SessionStart hook's `snip-run.sh update-check` spawns `snip-bootstrap.sh`
// when the binary is missing; bootstrap downloads + verifies + installs the
// release tarball. We point it at a local fake release server (loopback only) so
// the real install logic runs end-to-end without touching the network. No claude
// needed. Run: `node --test tests/docker/phase-b-lifecycle.test.mjs`.

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import test, { after, describe } from "node:test";
import assert from "node:assert/strict";

import { PLUGIN_ROOT, SNIP_BOOTSTRAP, SNIP_RUN, freshDir, spawnAsync, waitUntil } from "./harness/lib.mjs";
import { snipBinary } from "./harness/binary.mjs";
import { cleanupTemp } from "./harness/run.mjs";

const haveCurl = spawnSync("curl", ["--version"], { encoding: "utf8" }).status === 0;
const haveTar = spawnSync("tar", ["--version"], { encoding: "utf8" }).status === 0;
const SKIP = haveCurl && haveTar ? false : "curl/tar not available (bootstrap needs them)";

after(cleanupTemp);

/** Build a `snip` tarball + its sha256 from the built binary. */
function makeRelease() {
  const dir = freshDir("release-");
  fs.copyFileSync(snipBinary(), path.join(dir, "snip"));
  fs.chmodSync(path.join(dir, "snip"), 0o755);
  const tgz = path.join(dir, "snip.tar.gz");
  const r = spawnSync("tar", ["-czf", tgz, "-C", dir, "snip"]);
  assert.equal(r.status, 0, "tar built the release archive");
  const bytes = fs.readFileSync(tgz);
  return { bytes, sha: createHash("sha256").update(bytes).digest("hex") };
}

/** A loopback release server: any `*.tar.gz` → the archive, any `*.sha256` → its hash. */
async function releaseServer(rel) {
  const server = http.createServer((req, res) => {
    if (req.url.endsWith(".sha256")) {
      res.writeHead(200, { "Content-Type": "text/plain" });
      return res.end(`${rel.sha}  snip.tar.gz\n`);
    }
    if (req.url.endsWith(".tar.gz")) {
      res.writeHead(200, { "Content-Type": "application/gzip" });
      return res.end(rel.bytes);
    }
    res.writeHead(404);
    return res.end();
  });
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  return server;
}

describe("Phase B — plugin lifecycle (offline)", () => {
  test("snip-bootstrap.sh downloads, verifies, and installs the binary", { skip: SKIP }, async () => {
    // Arrange
    const home = freshDir("snip-home-");
    const rel = makeRelease();
    const server = await releaseServer(rel);
    const base = `http://127.0.0.1:${server.address().port}`;

    // Act: invoke bootstrap directly (deterministic), pointed at the fake
    // release. MUST be async — bootstrap curls back into the in-process server,
    // which `spawnSync` would deadlock by blocking the event loop.
    const r = await spawnAsync("bash", [SNIP_BOOTSTRAP, "0.1.0", home], {
      env: { ...process.env, SNIP_DOWNLOAD_BASE: base, SNIP_RELEASES_API: base },
    });
    server.close();
    const bin = path.join(home, "bin", "snip");
    const installed = fs.existsSync(bin);

    // Assert
    assert.equal(r.status, 0, "bootstrap always exits 0");
    if (process.platform === "linux" && process.arch === "x64") {
      assert.ok(installed, "amd64 Linux is in snip's release matrix → must install");
    }
    if (installed) {
      const v = spawnSync(bin, ["--version"], { encoding: "utf8" });
      assert.equal(v.status, 0, "the installed binary runs");
    } else {
      // Documented graceful no-op for an arch outside snip's release matrix
      // (e.g. linux/arm64): bootstrap exits 0 and installs nothing.
      console.log(`[lifecycle] bootstrap no-op: ${process.platform}/${process.arch} not in snip's release matrix`);
    }
  });

  test("snip-run.sh self-installs via update-check when the binary is missing", { skip: SKIP }, async () => {
    // Arrange: an empty $SNIP_HOME — the wrapper must trigger bootstrap.
    const home = freshDir("snip-home-");
    const rel = makeRelease();
    const server = await releaseServer(rel);
    const base = `http://127.0.0.1:${server.address().port}`;

    // Act: the wrapper spawns bootstrap detached, then exits 0 immediately. Async
    // spawn keeps the event loop free so the loopback release server can answer.
    const r = await spawnAsync("bash", [SNIP_RUN, "update-check"], {
      env: {
        ...process.env,
        SNIP_HOME: home,
        CLAUDE_PLUGIN_ROOT: PLUGIN_ROOT,
        SNIP_DOWNLOAD_BASE: base,
        SNIP_RELEASES_API: base,
      },
    });
    const bin = path.join(home, "bin", "snip");
    // amd64 Linux installs in ~1s; on an arch outside snip's release matrix the
    // wrapper correctly no-ops, so a short poll avoids burning the full timeout.
    const supported = process.platform === "linux" && process.arch === "x64";
    const appeared = await waitUntil(() => fs.existsSync(bin), { timeoutMs: supported ? 15000 : 3000, stepMs: 150 });
    server.close();

    // Assert
    assert.equal(r.status, 0, "the wrapper exits 0 (never blocks Claude Code)");
    if (process.platform === "linux" && process.arch === "x64") {
      assert.ok(appeared, "amd64 Linux: the wrapper self-installed the binary");
    }
  });
});
