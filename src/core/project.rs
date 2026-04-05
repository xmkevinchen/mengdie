use std::path::Path;
use std::process::Command;

/// Infer a project_id from the git remote URL of the given directory.
/// Falls back to a hash of the directory path if no git remote is found.
pub fn infer_project_id(dir: &Path) -> String {
    if let Some(remote) = git_remote_url(dir) {
        normalize_git_url(&remote)
    } else {
        // Fallback: hash of absolute path
        let abs = dir
            .canonicalize()
            .unwrap_or_else(|_| dir.to_path_buf());
        format!("local:{:x}", simple_hash(abs.to_string_lossy().as_bytes()))
    }
}

/// Normalize git URLs so SSH and HTTPS for the same repo produce the same ID.
/// `git@github.com:user/repo.git` → `github.com/user/repo`
/// `https://github.com/user/repo.git` → `github.com/user/repo`
fn normalize_git_url(url: &str) -> String {
    let url = url.trim().trim_end_matches(".git").to_lowercase();
    // SSH: git@host:path → host/path
    if let Some(rest) = url.strip_prefix("git@") {
        return rest.replacen(':', "/", 1);
    }
    // HTTPS: https://host/path → host/path
    if let Some(rest) = url.strip_prefix("https://") {
        return rest.to_string();
    }
    if let Some(rest) = url.strip_prefix("http://") {
        return rest.to_string();
    }
    // ssh://git@host/path → host/path
    if let Some(rest) = url.strip_prefix("ssh://") {
        let rest = rest.strip_prefix("git@").unwrap_or(rest);
        return rest.to_string();
    }
    url
}

fn git_remote_url(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(dir)
        .output()
        .ok()?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if url.is_empty() {
            None
        } else {
            Some(url)
        }
    } else {
        None
    }
}

fn simple_hash(data: &[u8]) -> u64 {
    // FNV-1a hash
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_infer_project_id_fallback() {
        // A non-git directory should produce a local: hash
        let tmp = env::temp_dir();
        let id = infer_project_id(&tmp);
        assert!(id.starts_with("local:"));
        assert!(id.len() > 8);
    }

    #[test]
    fn test_infer_project_id_deterministic() {
        let tmp = env::temp_dir();
        let id1 = infer_project_id(&tmp);
        let id2 = infer_project_id(&tmp);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_simple_hash_different_inputs() {
        let h1 = simple_hash(b"hello");
        let h2 = simple_hash(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_normalize_ssh_and_https_match() {
        let ssh = normalize_git_url("git@github.com:user/repo.git");
        let https = normalize_git_url("https://github.com/user/repo.git");
        assert_eq!(ssh, https);
        assert_eq!(ssh, "github.com/user/repo");
    }

    #[test]
    fn test_normalize_strips_git_suffix() {
        assert_eq!(
            normalize_git_url("https://github.com/user/repo.git"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn test_normalize_ssh_protocol() {
        assert_eq!(
            normalize_git_url("ssh://git@github.com/user/repo.git"),
            "github.com/user/repo"
        );
    }
}
