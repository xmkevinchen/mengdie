use std::path::Path;
use std::process::Command;

/// Infer a stable, opaque project_id from the directory.
/// Uses git remote URL if available (normalized so SSH/HTTPS produce the same hash),
/// otherwise falls back to the canonical directory path.
/// Format: `proj_<16-hex-chars>` — platform-agnostic, deterministic.
pub fn infer_project_id(dir: &Path) -> String {
    let source = if let Some(remote) = git_remote_url(dir) {
        normalize_git_url(&remote)
    } else {
        let abs = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        abs.to_string_lossy().to_string()
    };
    format!("proj_{:016x}", simple_hash(source.as_bytes()))
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
    fn test_infer_project_id_format() {
        // Non-git directory produces proj_<hex>
        let tmp = env::temp_dir();
        let id = infer_project_id(&tmp);
        assert!(id.starts_with("proj_"));
        assert_eq!(id.len(), 5 + 16); // "proj_" + 16 hex chars
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
    fn test_normalize_ssh_and_https_same_hash() {
        let ssh = normalize_git_url("git@github.com:user/repo.git");
        let https = normalize_git_url("https://github.com/user/repo.git");
        // Same normalized form → same project_id
        assert_eq!(ssh, https);
    }

    #[test]
    fn test_normalize_self_hosted() {
        let ssh = normalize_git_url("git@gitlab.mycompany.com:team/project.git");
        let https = normalize_git_url("https://gitlab.mycompany.com/team/project.git");
        assert_eq!(ssh, https);
    }

    #[test]
    fn test_normalize_ssh_protocol() {
        let ssh1 = normalize_git_url("ssh://git@github.com/user/repo.git");
        let ssh2 = normalize_git_url("git@github.com:user/repo.git");
        assert_eq!(ssh1, ssh2);
    }
}
