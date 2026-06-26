#!/usr/bin/env bash
# Host launcher: build the image and run the full e2e tier in a network-isolated
# container. This is the canonical, fully-isolated entrypoint — nothing runs on
# the host, and `--network=none` makes reaching the real API impossible (the mock
# is loopback-only). Dependency-free: just the `docker` CLI.
#
#   tests/docker/run-docker.sh                       # all phases
#   tests/docker/run-docker.sh tests/docker/phase-a-contract.test.mjs   # a subset
#
# Env: SNIP_DOCKER_IMAGE (tag), CLAUDE_VERSION (build arg to pin Claude Code).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
image="${SNIP_DOCKER_IMAGE:-snip-docker-e2e}"
build_args=()
[ -n "${CLAUDE_VERSION:-}" ] && build_args+=(--build-arg "CLAUDE_VERSION=${CLAUDE_VERSION}")

echo ">> building ${image}"
# `${arr[@]+"${arr[@]}"}` so an empty array doesn't trip `set -u` on bash 3.2 (macOS).
docker build ${build_args[@]+"${build_args[@]}"} -f "${root}/tests/docker/Dockerfile" -t "${image}" "${root}"

echo ">> running (network-isolated)"
exec docker run --rm --network=none "${image}" "$@"
