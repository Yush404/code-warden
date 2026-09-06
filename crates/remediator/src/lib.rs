pub mod git_ops;

use std::path::Path;
use std::process::Command;

pub struct Remediator;

impl Remediator {
    pub fn run_deterministic_fixes(target: &Path) {
        if target.join("Cargo.toml").exists() {
            let _ = Command::new("cargo")
                .args(["fix", "--allow-no-vcs", "--broken-code"])
                .current_dir(target)
                .output();
        }
        let _ = Command::new("ruff")
            .args(["check", "--fix", "."])
            .current_dir(target)
            .output();
        let _ = Command::new("biome")
            .args(["check", "--write", "."])
            .current_dir(target)
            .output();
    }
}
