use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use image::{Rgba, RgbaImage};

static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    root: PathBuf,
    input: PathBuf,
    output: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "tex-packer-cli-{label}-{}-{sequence}",
            std::process::id()
        ));
        let input = root.join("input");
        let output = root.join("output");
        fs::create_dir_all(&input).expect("create CLI progress fixture directory");
        RgbaImage::from_pixel(2, 2, Rgba([32, 160, 224, 255]))
            .save(input.join("sprite.png"))
            .expect("write CLI progress fixture image");
        Self {
            root,
            input,
            output,
        }
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let temp_root = std::env::temp_dir();
        if self.root.starts_with(&temp_root)
            && self
                .root
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("tex-packer-cli-"))
        {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn run_dry_pack(fixture: &TestDirectory, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tex-packer"))
        .arg("pack")
        .arg(&fixture.input)
        .arg("--out-dir")
        .arg(&fixture.output)
        .arg("--dry-run")
        .args(args)
        .output()
        .expect("run tex-packer CLI")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "CLI failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_no_progress_control_sequences(output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains('\r'),
        "redirected stderr contains CR: {stderr:?}"
    );
    assert!(
        !stderr.contains('\u{1b}'),
        "redirected stderr contains ANSI escapes: {stderr:?}"
    );
    assert!(
        !stderr.contains(" loading "),
        "redirected stderr contains a rendered progress bar: {stderr:?}"
    );
}

#[test]
fn explicit_no_progress_and_quiet_emit_no_progress_output() {
    let fixture = TestDirectory::new("progress-disabled");

    for args in [
        ["--progress", "false"].as_slice(),
        ["--progress", "true", "--quiet"].as_slice(),
    ] {
        let output = run_dry_pack(&fixture, args);
        assert_success(&output);
        assert_no_progress_control_sequences(&output);
    }
}

#[test]
fn redirected_progress_is_hidden_for_normal_and_parallel_packs() {
    let fixture = TestDirectory::new("progress-redirected");
    for args in [
        ["--progress", "true"].as_slice(),
        [
            "--progress",
            "true",
            "--algorithm",
            "auto",
            "--parallel",
            "--time-budget",
            "20",
        ]
        .as_slice(),
    ] {
        let output = run_dry_pack(&fixture, args);
        assert_success(&output);
        assert_no_progress_control_sequences(&output);
    }
}
