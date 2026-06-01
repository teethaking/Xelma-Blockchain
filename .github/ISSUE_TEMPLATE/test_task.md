---
name: Test task
about: Request additional test coverage (unit, property, chaos, benchmark)
title: "[Test] "
labels: testing
assignees: ""
---

## Summary

One-line description of the coverage gap.

## What needs coverage

Describe the behavior, path, or invariant that is currently under-tested.

## Where to work

- [ ] `contracts/src/tests/` (unit / integration)
- [ ] `contracts/src/tests/property_invariants.rs` (property)
- [ ] `contracts/src/tests/chaos_recovery.rs` (chaos / recovery)
- [ ] `contracts/src/tests/storage_benchmarks.rs` / benchmarks (performance)

## Risk

What can break silently today because this is not tested?

## Scope

- [ ] Happy path
- [ ] Failure / revert paths
- [ ] Boundary values
- [ ] Multi-round / lifecycle interactions

## Acceptance criteria

- [ ] Tests fail before the fix / cover the gap
- [ ] Tests pass on `cargo test --workspace`
- [ ]

## Test plan

List the specific cases to add.

-
-

## Difficulty

beginner | intermediate | advanced
