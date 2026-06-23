// Seed a run's working directory with the files the scripted tool calls touch.
// `commented.rs` is committed (the canonical Read compaction case); the grep and
// glob corpora are generated so we never commit dozens of throwaway files. Each
// corpus is shaped to produce REDUCIBLE output that snip's optimizers compact —
// repeated match lines for grep, a shared directory prefix for glob.

import fs from "node:fs";
import path from "node:path";

import { REPO_ROOT } from "./lib.mjs";

const FIXTURES = path.join(REPO_ROOT, "tests", "docker", "fixtures");

/**
 * Populate `cwd` and return the absolute paths + raw contents the assertions
 * compare against (e.g. the Read source the model must NOT see verbatim).
 */
export function seedWorkspace(cwd) {
  // Read: copy the committed commented source in.
  const readSrc = path.join(cwd, "commented.rs");
  fs.copyFileSync(path.join(FIXTURES, "commented.rs"), readSrc);
  const readRaw = fs.readFileSync(readSrc, "utf8");

  // Grep: one file with many identical matching lines → collapses to (×N).
  const grepDir = path.join(cwd, "grepcorpus");
  fs.mkdirSync(grepDir, { recursive: true });
  const grepFile = path.join(grepDir, "matches.txt");
  fs.writeFileSync(grepFile, "NEEDLE here\n".repeat(40));

  // Glob: several files under one deep shared directory → groups to one header.
  const globDir = path.join(cwd, "globcorpus");
  const globBase = path.join(globDir, "deep", "nested");
  fs.mkdirSync(globBase, { recursive: true });
  for (const name of ["a", "b", "c", "d", "e"]) {
    fs.writeFileSync(path.join(globBase, `${name}.rs`), "pub fn x() {}\n");
  }

  return {
    readPath: readSrc,
    readRaw,
    grepDir,
    globDir,
    grepPattern: "NEEDLE",
    globPattern: "globcorpus/**/*.rs",
    // The seeded source artifacts a tamper check must find byte-identical after a run.
    fixturePaths: [readSrc, grepDir, globDir],
  };
}
