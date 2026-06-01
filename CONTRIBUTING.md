# Contributing to Xelma-Blockchain

Thanks for improving Xelma. This document explains the expected workflow for contributors and maintainers.

## Before You Start

- Open or reference an issue describing the change.
- Keep changes focused and easy to review.
- For contract changes, include or update tests.

## Filing Issues

Blank issues are disabled. Pick the template that matches your work so every
backlog item ships with the technical detail maintainers need to triage it:

- **Bug report** — a reproducible defect in contract, bindings, or CI.
- **Feature request** — a general enhancement to logic, tooling, or docs.
- **Protocol improvement** — changes to contract logic, economics, or lifecycle.
- **Security hardening task** — public, non-sensitive defense-in-depth work.
  (Never disclose exploitable vulnerabilities publicly — use the private path
  in `SUPPORT.md`.)
- **Test task** — additional unit, property, chaos, or benchmark coverage.

The protocol, security, and test templates require **risk**, **scope**,
**acceptance criteria**, and a **test plan**. Issues missing these fields may be
sent back for detail before they are picked up.

## Development Setup

1. Fork and clone the repository.
2. Install Rust and Node.js dependencies.
3. Run local validation before opening a PR:
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo fmt --all -- --check`
   - `cd bindings && npm ci && npm run build`
   - `cd bindings && npm run test:parity` (ABI drift check; mirrors the CI `bindings-test` job)

## Canonical Contract Crate

The contract crate name is `xelma-contract`. Do not reintroduce legacy crate naming in build scripts, docs, or CI commands.

## Pull Request Expectations

- Link the issue(s) the PR closes.
- Describe behavioral impact and migration assumptions.
- Include test evidence for the changed behavior.
- Keep generated build artifacts out of commits (`target/`, `bindings/dist/`, etc.).

## Review and Merge Policy

- At least one maintainer approval is required for non-trivial changes.
- Contract, CI, or release workflow changes require review from a listed code owner.
- Maintainers may request follow-up hardening before merge when security or correctness risk exists.

## Security Reporting

Do not open public issues for vulnerabilities. Follow the process in `SUPPORT.md` and repository security policy/disclosure instructions.
