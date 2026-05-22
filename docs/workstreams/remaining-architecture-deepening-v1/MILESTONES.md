# Remaining Architecture Deepening — Milestones

Status: Complete
Last updated: 2026-05-22

## M0 — Workstream Ready

Exit criteria:

- Durable docs exist and agree.
- Tasks are independently verifiable.
- First executable task is RAD-020.

## M1 — Export Projection Deepened

Exit criteria:

- One internal export manifest projection exists.
- JSON, plist, and template Adapters share projection semantics where practical.
- Export smoke tests pass.

Primary gate:

```bash
cargo nextest run -p tex-packer-core --test export_smoke
cargo check -p tex-packer-cli
```

## M2 — Runtime Strategy Locality Improved

Exit criteria:

- Runtime Guillotine, Shelf, and Skyline strategy Implementations are separated.
- `AtlasSession` public Interface remains unchanged.
- Runtime and shared invariant tests pass.

Primary gate:

```bash
cargo nextest run -p tex-packer-core --test runtime_session --test runtime_shelf --test runtime_skyline --test runtime_api_improvements --test algorithm_invariants
```

## M3 — Preparation Deepened

Exit criteria:

- Preparation owns trim, transparent-policy, source rect, layout-only, and sorting semantics.
- Offline image and layout flows remain aligned.

Primary gate:

```bash
cargo nextest run -p tex-packer-core --test transparent_policy --test layout_vs_images --test padding_extrude_offsets --test algorithm_invariants
```

## M4 — CLI Pipeline Deepened

Exit criteria:

- [x] CLI command orchestration has deeper Modules for input, packing, output, and reporting.
- [x] Existing CLI flags and output behavior are preserved.
- [x] Workspace tests and targeted CLI smoke checks pass.

Primary gate:

```bash
cargo check -p tex-packer-cli
cargo nextest run --workspace
```

## M5 — Closeout

Exit criteria:

- [x] All task statuses and evidence are current.
- [x] Full closeout gates pass.
- [x] `WORKSTREAM.json` status is `complete`.
