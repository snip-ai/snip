# syntax=docker/dockerfile:1
#
# Fully self-contained image for the snip Docker e2e tier: it builds snip from
# source, installs a pinned Claude Code, and runs the A/B/C phases entirely
# inside the container against a loopback fake model server. The host only does
# `docker build` + `docker run --network=none` — nothing touches the host, and
# with no network the suite physically cannot reach the real API.
#
# Build context = repo root:
#   docker build -f tests/docker/Dockerfile -t snip-docker-e2e .

# --- Stage 1: build snip from source (no host artifact needed) ---
# Pins the toolchain to the project MSRV (1.96, edition 2024 — see rust-toolchain.toml).
FROM rust:1.96-bookworm AS build
WORKDIR /src
COPY . .
# BuildKit cache mounts keep the registry + compiled grammars warm across builds;
# the binary is copied OUT of the (ephemeral) target cache into a real layer path.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release && cp target/release/snip /snip-bin

# --- Stage 2: runtime — Node + Claude Code + the harness ---
FROM node:22-bookworm-slim AS test
# curl + tar drive the plugin bootstrap (lifecycle phase); ripgrep backs Grep;
# git is exercised by the Bash phase (git status/log); findutils provides `find`.
RUN apt-get update \
    && apt-get install -y --no-install-recommends bash ca-certificates curl findutils git ripgrep tar \
    && rm -rf /var/lib/apt/lists/*
# Pin Claude Code. Bump via `--build-arg CLAUDE_VERSION=…` to test a new release.
ARG CLAUDE_VERSION=2.1.175
RUN npm install -g @anthropic-ai/claude-code@${CLAUDE_VERSION}

COPY --from=build /snip-bin /opt/snip/bin/snip
RUN chmod 0755 /opt/snip/bin/snip
ENV SNIP_TEST_BINARY=/opt/snip/bin/snip

WORKDIR /work
COPY --chown=node:node tests/docker ./tests/docker
COPY --chown=node:node plugins/snip ./plugins/snip

# Run as the unprivileged `node` user (Claude Code is happier non-root; the
# harness still sets IS_SANDBOX=1 for --dangerously-skip-permissions).
USER node
ENTRYPOINT ["bash", "tests/docker/run.sh"]
