use std::process::Command;

/// Detect current git branch and HEAD SHA. Returns (branch, sha) if in a git repo.
pub fn detect_git_ref() -> Option<String> {
    let branch = get_current_branch();
    let sha = get_head_sha();

    match (branch, sha) {
        (Some(b), Some(s)) => Some(format!("{} ({})", b, &s[..8.min(s.len())])),
        (Some(b), None) => Some(b),
        (None, Some(s)) => Some(s[..8.min(s.len())].to_string()),
        (None, None) => None,
    }
}

fn get_current_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch == "HEAD" {
            None // detached HEAD
        } else {
            Some(branch)
        }
    } else {
        None
    }
}

fn get_head_sha() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
