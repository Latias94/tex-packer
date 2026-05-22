# Architecture Deepening Refactor

Status: Complete
Last updated: 2026-05-22

## Why This Lane Exists

The core atlas algorithms have now been stabilized enough to reveal deeper architecture friction: geometry semantics, free-space mutation, runtime placement, configuration validation, and test invariants are still spread across multiple shallow Modules. This lane exists to complete the architecture-deepening candidates from the 2026-05-22 review without losing safety, evidence, or API compatibility.

## Relevant Authority

- ADRs: none found in `docs/adr/` as of 2026-05-22.
- Existing docs:
  - `README.md`
  - `crates/tex-packer-core/README.md`
  - architecture report: `%TEMP%/architecture-review-20260522-110031.html`
- Related workstreams:
  - `docs/workstreams/offline-pipeline-placement-geometry-v1/` — closed; introduced shared offline pipeline and initial placement geometry Module.

## Problem

Packing correctness currently depends on caller knowledge that leaks across seams: public `Rect` exposes inclusive `right()`/`bottom()` while algorithms mostly need exclusive edges; runtime packing duplicates offline placement logic; free-list split/prune/merge rules are repeated; `PackerConfig::validate()` proves little to downstream callers; CLI/YAML/GUI configuration adapters duplicate mapping rules; tests assert many cases but do not expose one reusable invariant Interface.

## Target State

- Pixel geometry is a deep crate-private Module with one Interface for exclusive edges, containment, intersection, area scoring, and placement geometry.
- Free-space operations are a deep crate-private Module reused by Guillotine, MaxRects, Skyline waste map, and runtime placement where applicable.
- Runtime placement is split out of the public session facade; strategy-specific placement is isolated behind a small internal Interface.
- Configuration validation produces or uses a validated packing context so derived geometry and page policies are not recomputed ad hoc.
- Offline packing remains one shared pipeline but has clearer internal seams for preparation, page packing, page sizing, rendering, and metadata.
- CLI/YAML/GUI configuration adapters converge through consistent parsing and validation instead of silent fallback.
- Algorithm/runtime invariants are reusable tests that cover all algorithms and future refactors.

## In Scope

- Crate-private refactors inside `crates/tex-packer-core/src/`.
- Test harness additions under `crates/tex-packer-core/tests/`.
- CLI configuration adapter cleanup in `crates/tex-packer-cli/src/` when it supports the validated context.
- GUI preset/config touch-ups only when they reduce duplicated config semantics.
- README/core README updates for shipped behavior if public behavior changes or a new public helper is added.
- Workstream evidence, task ledger, journal, and closeout docs.

## Out Of Scope

- New packing algorithms.
- Changing exported metadata schemas unless required for correctness.
- Changing the public `Rect` serialized shape.
- Breaking existing CLI flags or GUI presets without a separate ADR.
- Performance micro-optimization not tied to these architecture seams.

## Starting Assumptions

| Assumption | Confidence | Evidence | Consequence if wrong |
| --- | --- | --- | --- |
| Public `Rect` can remain compatible while internal code moves to exclusive-edge helpers. | High | Existing serializers/tests use `{x,y,w,h}` and not edge methods directly. | Need an ADR and possible semver note before changing public edge semantics. |
| Runtime and offline algorithms can share geometry/free-space helpers without sharing full algorithm state. | High | Recent Skyline/runtime bug was fixed with equivalent logic in both places. | Keep helpers narrower; do not force one large generic engine. |
| A reusable invariant test harness will reduce risk before larger refactors. | High | Bugs were caught by bespoke tests; helpers like `disjoint` are repeated. | Keep harness deterministic and avoid flaky property tests initially. |
| CLI/YAML silent fallback is undesirable for configuration correctness. | Medium | `YamlConfig::into_packer_config` currently uses `unwrap_or` for enum parsing. | If compatibility is required, surface warnings instead of hard errors. |

## Architecture Direction

Deepen Modules in this order:

1. Put invariant tests first so later refactors have a small, trustworthy Interface for correctness.
2. Deepen pixel geometry before touching free-space or runtime placement; this removes inclusive/exclusive leakage.
3. Deepen free-space operations where there are multiple real adapters: Guillotine, MaxRects, Skyline waste map, and runtime.
4. Split runtime placement behind an internal Module so `AtlasSession` becomes a facade instead of a strategy implementation file.
5. Introduce validated packing context after geometry/free-space names stabilize.
6. Revisit offline pipeline and configuration adapters once the core semantics stop moving.

## Closeout Condition

This lane can close when:

- all task ledger items are DONE or explicitly split/deferred,
- `cargo fmt --check` passes,
- `cargo check -p tex-packer-core` passes,
- `cargo nextest run -p tex-packer-core` passes,
- `cargo nextest run --workspace` passes or a narrower gate is justified,
- docs and evidence reflect shipped behavior,
- and `WORKSTREAM.json` status is updated to `complete`.

## Closeout Result

Closed on 2026-05-22. All executable tasks ADP-010 through ADP-090 are complete, the public API shape remained stable, and the final gates passed:

- `cargo fmt --check`
- `cargo check -p tex-packer-core`
- `cargo nextest run -p tex-packer-core` — 103/103 passed
- `cargo nextest run --workspace` — 103/103 passed
