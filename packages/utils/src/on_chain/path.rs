use std::{path::PathBuf, process::Command};

pub fn repo_root() -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;

    if !output.status.success() {
        None
    } else {
        let git_root_bytes = String::from_utf8_lossy(&output.stdout);
        let git_root = git_root_bytes.trim();
        Some(PathBuf::from(git_root))
    }
}

pub fn repo_wavs_home() -> Option<PathBuf> {
    repo_root().map(|root| root.join("backend").join("wavs-home"))
}
