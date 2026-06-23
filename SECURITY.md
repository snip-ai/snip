# Security Policy

Thanks for helping keep **snip** and its users safe. snip is a single static Rust
binary, shipped as a Claude Code plugin, that hooks into token-heavy tool surfaces
(Read, Bash, Grep, Glob). Because it runs inside your development environment, we
take security reports seriously and respond promptly.

## Supported versions

snip is pre-1.0 and ships from a single moving line. Only the **latest released
version** receives security fixes. Before reporting, update to the latest release
(via the plugin) and confirm the issue still reproduces.

| Version | Supported |
|---------|-----------|
| Latest release | ✅ |
| Any older version | ❌ |

## Reporting a vulnerability

**Please do not open a public issue, discussion, or pull request for a security
vulnerability.** Public disclosure before a fix is available puts users at risk.

Report privately through GitHub's private vulnerability reporting:

1. Go to the repository's **Security** tab:
   <https://github.com/snip-ai/snip/security>
2. Click **Report a vulnerability** to open a private security advisory.
3. Include as much detail as you can:
   - affected version (`snip --version`) and platform/OS,
   - a clear description of the issue and its impact,
   - reproduction steps or a proof of concept,
   - any relevant hook input, configuration, or logs (redact secrets).

This routes your report directly and privately to the maintainer
([Aymeric Pasco](https://github.com/snip-ai)). We will coordinate a fix and a
coordinated disclosure timeline with you, and credit you in the advisory unless you
prefer to remain anonymous.

## Response timeline

We aim to:

| Stage | Target |
|-------|--------|
| Acknowledge your report | within **3 business days** |
| Initial assessment & severity triage | within **7 business days** |
| Fix or mitigation for confirmed issues | as soon as practical, prioritized by severity |
| Public advisory & release | after a fix ships, coordinated with you |

These are good-faith targets for a small, volunteer-maintained project, not a
contractual SLA. We'll keep you updated throughout.

## Safety guarantees

snip is designed to **fail safe** and to minimize what it can affect. These
properties are core invariants, enforced in code and covered by tests:

- **Hooks always exit 0.** Every hook catches all errors *and panics* at the top
  level and returns success, so a snip bug can never block or break a Claude Code
  tool call.
- **Never writes to your source files.** snip only reads tool output to optimize
  it. Its own state (config, caches, stats) lives under the OS data directory —
  never in your repository or working tree.
- **Fails safe to the original output.** If optimization fails for any reason,
  snip returns the original, unmodified tool output. It never silently discards
  content; overflow is recoverable.
- **No regex, no scripting in specs.** Declarative optimizer specs use a closed,
  data-only transform vocabulary. There is no embedded scripting engine and no
  regex evaluation, eliminating that class of injection / ReDoS / RCE surface.
  User-overridable config selects and parameterizes built-in transforms only.
- **Checksum-verified binary download.** Binaries distributed through the plugin
  are verified against a published checksum before use, so a tampered or corrupted
  download is detected and rejected.

## Scope

In scope: the snip binary, its hooks, its declarative spec/config handling, and the
plugin's install/update path (including checksum verification).

Out of scope: vulnerabilities in Claude Code itself, in the Rust toolchain or
third-party crates (please report those upstream), and issues that require an
attacker who already has full control of the user's machine or data directory.

## Disclosure

We practice coordinated disclosure. Once a fix is released, we'll publish a GitHub
Security Advisory describing the issue, affected versions, the fix, and credit to
the reporter. snip is licensed under Apache-2.0.
