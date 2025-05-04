use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

use anyhow::Context as _;

const REPO: &str = "https://github.com/averms/glvs";
const TARBALL_NAME: &str = "single_step.tar.zst";

fn main() -> Result<(), anyhow::Error> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir
        .parent()
        .expect("xtask dir should be inside a workspace");
    let resources_dir = workspace_dir.join("resources");
    let single_step_dir = resources_dir.join("single_step");
    let tarball_path = resources_dir.join(TARBALL_NAME);

    fs::create_dir_all(&resources_dir)?;
    env::set_current_dir(&resources_dir).context("couldn't chdir to resources")?;
    if tarball_path.try_exists()? {
        println!("Tarball already downloaded");
    } else {
        let link = format!("{REPO}/releases/download/test-tarball/{TARBALL_NAME}");
        cmd(&["curl", "--fail", "--location", "--remote-name", &link])?;
    }
    if single_step_dir.try_exists()? {
        println!("Test cases already extracted");
    } else {
        cmd(&["tar", "-xf", TARBALL_NAME])?;
    }

    Ok(())
}

fn cmd(argv: &[&str]) -> Result<(), anyhow::Error> {
    let s = Command::new(argv[0])
        .args(&argv[1..])
        .status()
        .with_context(|| format!("couldn't exec {}", argv[0]))?;
    if !s.success() {
        anyhow::bail!("{argv:?} exited with non-zero");
    }

    Ok(())
}
