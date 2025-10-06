# tex-packer-gui

Desktop GUI for tex-packer built with dear-imgui-rs + winit + wgpu.

- Load a folder of images, configure packing options, preview atlas pages, and export PNG + JSON.
- Uses tex-packer-core for algorithms and rendering.

Quickstart
- From repo: `cargo run -p tex-packer-gui`
- Controls:
  - Inputs: Pick input folder; optional output folder.
  - Config: Algorithm, dimensions, padding, rotation, pow2/square, auto settings.
  - Actions: Pack to preview; Export to save PNGs and JSON (hash format).

Notes
- dear-imgui-rs v0.3.0 backend (wgpu+winit). See repo-ref/dear-imgui-rs examples if needed.
- For large sets, Auto (quality) + time budget yields better single-page occupancy.
- Wasm: GUI is desktop-focused; core compiles to wasm32-unknown-unknown.
