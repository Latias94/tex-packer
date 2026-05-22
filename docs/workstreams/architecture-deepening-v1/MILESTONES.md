# Architecture Deepening Refactor — Milestones

Status: Complete
Last updated: 2026-05-22

## M0 — Scope And Evidence Freeze

Exit criteria:

- Workstream docs exist and agree.
- Architecture candidates are mapped to executable tasks.
- Gate set is explicit.
- First executable task is chosen.

Primary evidence:

- `docs/workstreams/architecture-deepening-v1/DESIGN.md`
- `docs/workstreams/architecture-deepening-v1/TODO.md`

## M1 — Safety Harness Before Refactor

Exit criteria:

- A reusable invariant test Module exists.
- Offline algorithms and runtime strategies are covered through the same assertion language.
- The harness is deterministic and not flaky.

Primary gate:

```bash
cargo nextest run -p tex-packer-core --test algorithm_invariants
```

Result: Complete. `algorithm_invariants.rs` now covers shared offline/runtime invariants.

## M2 — Core Geometry And Free-space Deepened

Exit criteria:

- Internal geometry helpers own edge semantics and area scoring.
- Free-space helpers own shared subtract/prune/merge behavior.
- Existing algorithm-specific choices remain explicit.

Primary gates:

```bash
cargo nextest run -p tex-packer-core --test algorithm_invariants --test boundary_conditions --test skyline_no_rotation --test runtime_skyline
cargo nextest run -p tex-packer-core --test maxrects_determinism --test skyline_waste_map --test runtime_session
```

Result: Complete. `geometry.rs` and `free_space.rs` now own shared edge, scoring, split, subtract, prune, and merge helpers.

## M3 — Runtime Placement Deepened

Exit criteria:

- Runtime placement strategy state is not embedded in the public session facade.
- Runtime placement uses shared geometry/free-space Modules where appropriate.
- Runtime atlas pixel management remains unchanged except for calling the stable session Interface.

Primary gate:

```bash
cargo nextest run -p tex-packer-core --test runtime_session --test runtime_skyline --test runtime_shelf --test runtime_api_improvements --test runtime_atlas_tests --test algorithm_invariants
```

Result: Complete. `runtime.rs` is now the public session facade over internal `runtime_placement.rs`.

## M4 — Validated Context And Offline Pipeline Deepened

Exit criteria:

- Internal callers use validated/derived packing context rather than recomputing core invariants ad hoc.
- Offline image/layout APIs still share the same placement pipeline.
- Page sizing and metadata behavior remain covered by tests.

Primary gate:

```bash
cargo nextest run -p tex-packer-core --test boundary_conditions --test force_max_and_border --test padding_extrude_offsets --test layout_vs_images --test pack_stats --test algorithm_invariants
```

Result: Complete. `PackingContext` centralizes derived geometry/page policy and `OfflinePipeline` owns offline packing phases.

## M5 — Configuration Adapter Convergence

Exit criteria:

- CLI/YAML/GUI config inputs converge through consistent parsing and validation.
- Existing CLI flags remain stable.
- Silent fallback is removed or intentionally documented.

Primary gate:

```bash
cargo check -p tex-packer-cli -p tex-packer-gui
```

Result: Complete. `config_adapter.rs` now centralizes strict CLI/YAML/bench parsing and validation; GUI typed presets required no semantic change.

## M6 — Closeout

Exit criteria:

- Full gate set is recorded.
- `WORKSTREAM.json` status is `complete`.
- Follow-on work is either completed, deferred, or split.

Primary gates:

```bash
cargo fmt --check
cargo check -p tex-packer-core
cargo nextest run -p tex-packer-core
cargo nextest run --workspace
```

Result: Complete. Final closeout gates passed on 2026-05-22.
