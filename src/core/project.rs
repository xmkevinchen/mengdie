use std::path::Path;
use std::process::Command;

/// Infer a project_id from the git remote URL of the given directory.
/// Falls back to a hash of the directory path if no git remote is found.
pub fn infer_project_id(dir: &Path) -> String {
    if let Some(remote) = git_remote_url(dir) {
        // Normalize: strip .git suffix, lowercase
        let normalized = remote
            .trim()
            .trim_end_matches(".git")
            .to_lowercase();
        normalized
    } else {
        // Fallback: hash of absolute path
        let abs = dir
            .canonicalize()
            .unwrap_or_else(|_| dir.to_path_buf());
        format!("local:{:x}", simple_hash(abs.to_string_lossy().as_bytes()))
    }
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
}
