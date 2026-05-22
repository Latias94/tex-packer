# tex-packer-gui

[![Crates.io](https://img.shields.io/crates/v/tex-packer-gui.svg)](https://crates.io/crates/tex-packer-gui)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/Latias94/tex-packer)

![GUI Overview](https://raw.githubusercontent.com/Latias94/tex-packer/main/screenshots/gui-overview.png)

Desktop GUI for tex-packer built with egui/eframe (wgpu).

- Load a folder of images, configure packing options, preview atlas pages, and export PNG + JSON.
- Uses tex-packer-core for algorithms and rendering.

## Quickstart

- From repo: `cargo run -p tex-packer-gui`
- Release build from repo: `cargo run -p tex-packer-gui --release`
- From crates.io: `cargo install tex-packer-gui`
- Controls:
  - Inputs: Pick input folder; optional output folder.
  - Config: Algorithm, dimensions, padding, rotation, pow2/square, auto settings.
  - Actions: Pack to preview; Export to save PNGs and JSON (hash format).

## Basic workflow

1. Click the input folder picker and select a directory containing images.
2. Pick an output folder.
3. Choose atlas dimensions and packing settings.
4. Click **Pack** to preview the generated atlas pages.
5. Click **Export** to write atlas PNG pages and JSON metadata.

Output naming matches the CLI:

- Single page: `atlas.png` and `atlas.json`
- Multiple pages: `atlas_0.png`, `atlas_1.png`, ... and `atlas.json`

Use the GUI when you want to tune settings visually. Use `tex-packer-cli` when you need repeatable build scripts or CI integration.

## Notes
- For large sets, Auto (quality) + time budget yields better single-page occupancy.
- Wasm: GUI is desktop-focused; core compiles to wasm32-unknown-unknown.
