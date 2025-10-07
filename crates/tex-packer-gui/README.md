# tex-packer-gui

[![Crates.io](https://img.shields.io/crates/v/tex-packer-gui.svg)](https://crates.io/crates/tex-packer-gui)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/Latias94/tex-packer)

![GUI Overview](https://raw.githubusercontent.com/Latias94/tex-packer/main/screenshots/gui-overview.png)

Desktop GUI for tex-packer built with egui/eframe (wgpu).

- Load a folder of images, configure packing options, preview atlas pages, and export PNG + JSON.
- Uses tex-packer-core for algorithms and rendering.


Quickstart
- From repo: `cargo run -p tex-packer-gui`
- Controls:
  - Inputs: Pick input folder; optional output folder.
  - Config: Algorithm, dimensions, padding, rotation, pow2/square, auto settings.
  - Actions: Pack to preview; Export to save PNGs and JSON (hash format).

Notes
- egui/eframe (wgpu). See repo-ref/egui for API references.
- For large sets, Auto (quality) + time budget yields better single-page occupancy.
- Wasm: GUI is desktop-focused; core compiles to wasm32-unknown-unknown.
