use std::path::PathBuf;
use std::process::Command;
use anyhow::{bail, Context, Result};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("build-ebpf") => {
            let release = args.any(|a| a == "--release");
            build_ebpf(release)?;
            Ok(())
        }
        unk => {
            bail!("Unknown command: {:?}. Usage: cargo xtask build-ebpf [--release]", unk)
        }
    }
}

fn find_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn build_ebpf(release: bool) -> Result<()> {
    let root = find_root();
    let mut cmd = Command::new("cargo");
    cmd.current_dir(root.clone()).args([
        "+nightly",
        "build",
        "--package", "wirelux-ebpf",
        "--target", "bpfel-unknown-none",
        "-Z", "build-std=core"
    ]);
    if release {
        cmd.arg("--release");
    }
    let status = cmd.status().context("failed to run cargo")?;
    if !status.success() {
        bail!("cargo build failed with status: {}", status);
    }
    Ok(())
}