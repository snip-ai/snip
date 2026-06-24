#!/usr/bin/env bash
# In-container entrypoint: self-test the mock (no claude), then run the A/B/C
# phases via Node's test runner. Pass phase file paths as args to run a subset;
# with none, all phases run. Exit code propagates from `node --test`.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1
# A writable, world-safe default (the container runs as the unprivileged `node`
# user and the repo root is root-owned). Artifacts are optional telemetry.
export SNIP_DOCKER_ARTIFACTS="${SNIP_DOCKER_ARTIFACTS:-${TMPDIR:-/tmp}/snip-docker-artifacts}"
mkdir -p "$SNIP_DOCKER_ARTIFACTS" 2>/dev/null || true

echo "== environment =="
echo "node:   $(node --version)"
echo "claude: $(claude --version 2>&1)"
echo "snip:   ${SNIP_TEST_BINARY:-?} -> $("${SNIP_TEST_BINARY:-snip}" --version 2>&1)"
echo

echo "== harness self-test (no claude) =="
node tests/docker/harness/mock.selftest.mjs || exit 1
echo

phases=("$@")
if [ ${#phases[@]} -eq 0 ]; then
  phases=(
    tests/docker/phase-a-contract.test.mjs
    tests/docker/phase-b-conformance.test.mjs
    tests/docker/phase-b-languages.test.mjs
    tests/docker/phase-b-bash.test.mjs
    tests/docker/phase-b-edit-write.test.mjs
    tests/docker/phase-b-grep-modes.test.mjs
    tests/docker/phase-b-plugin-install.test.mjs
    tests/docker/phase-b-lifecycle.test.mjs
    tests/docker/phase-b-shell-setup.test.mjs
    tests/docker/phase-c-efficacy.test.mjs
  )
fi

echo "== phases: ${phases[*]} =="
node --test "${phases[@]}"
