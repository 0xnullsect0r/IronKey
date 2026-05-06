//! IronKey build automation.
//!
//! Usage:
//!   cargo xtask build-app [--release]
//!   cargo xtask build-rootfs
//!   cargo xtask build-iso [--output ironkey.iso]
//!   cargo xtask test

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "xtask",
    about = "IronKey build automation",
    version = env!("CARGO_PKG_VERSION")
)]
struct Args {
    #[command(subcommand)]
    cmd: SubCmd,
}

#[derive(Subcommand)]
enum SubCmd {
    /// Build the IronKey application binary
    BuildApp {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Build the minimal rootfs (requires root + debootstrap)
    BuildRootfs,
    /// Build a bootable ISO image
    BuildIso {
        /// Output file path
        #[arg(long, default_value = "ironkey.iso")]
        output: String,
    },
    /// Run the full test suite
    Test,
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.cmd {
        SubCmd::BuildApp { release } => build_app(release),
        SubCmd::BuildRootfs => build_rootfs(),
        SubCmd::BuildIso { output } => build_iso(&output),
        SubCmd::Test => run_tests(),
    }
}

fn cargo(args: &[&str]) -> Result<()> {
    let status = Command::new("cargo")
        .args(args)
        .status()
        .context("Failed to invoke cargo")?;
    if !status.success() {
        anyhow::bail!("cargo {} failed (exit: {})", args[0], status);
    }
    Ok(())
}

fn build_app(release: bool) -> Result<()> {
    println!("▶ Building ironkey binary…");
    let mut args = vec!["build", "-p", "ironkey-app"];
    if release {
        args.push("--release");
    }
    cargo(&args)?;
    let profile = if release { "release" } else { "debug" };
    println!(
        "✔ Binary at: target/{}/ironkey",
        profile
    );
    Ok(())
}

fn build_rootfs() -> Result<()> {
    println!("▶ Building minimal rootfs…");
    println!("  This step requires root privileges, debootstrap, and squashfs-tools.");
    println!("  See os/build.sh for the complete build process.");

    let rootfs_dir = std::path::Path::new("os/rootfs");
    if !rootfs_dir.exists() {
        anyhow::bail!("os/rootfs directory not found; run from repo root");
    }
    println!("✔ Rootfs source directory: {}", rootfs_dir.display());
    Ok(())
}

fn build_iso(output: &str) -> Result<()> {
    println!("▶ Building bootable ISO: {}", output);

    // Step 1: build the binary
    build_app(true)?;

    // Step 2: show what the ISO build script would do
    println!("  Steps that os/build.sh performs:");
    println!("  1. debootstrap --variant=minbase bookworm /tmp/rootfs");
    println!("  2. Copy target/release/ironkey into /tmp/rootfs/usr/bin/");
    println!("  3. Copy os/rootfs/etc/ into /tmp/rootfs/etc/");
    println!("  4. mksquashfs /tmp/rootfs /tmp/rootfs.squashfs");
    println!("  5. grub-mkrescue -o {} /tmp/iso_staging/", output);
    println!("  Run: sudo bash os/build.sh {}", output);
    Ok(())
}

fn run_tests() -> Result<()> {
    println!("▶ Running workspace tests…");
    cargo(&["test", "--workspace"])?;
    println!("✔ All tests passed");
    Ok(())
}
