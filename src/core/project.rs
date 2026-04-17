use std::path::Path;
use std::process::Command;

/// Project identity: either a user-given name from `.mengdie.toml` or a hash fallback.
///
/// Resolution order:
/// 1. `.mengdie.toml` in the directory (or any ancestor up to git root) → `project.name`
/// 2. Git remote URL hash (FNV-1a) → `proj_<16-hex>`
/// 3. Canonical path hash → `proj_<16-hex>`
pub fn infer_project_id(dir: &Path) -> String {
    if let Some(name) = read_project_name(dir) {
        return name;
    }
    let source = if let Some(remote) = git_remote_url(dir) {
        normalize_git_url(&remote)
    } else {
        let abs = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        abs.to_string_lossy().to_string()
    };
    format!("proj_{:016x}", simple_hash(source.as_bytes()))
}

/// Read project name from `.mengdie.toml` walking up to git root.
/// File format:
/// ```toml
/// [project]
/// name = "my-project"
/// ```
fn read_project_name(dir: &Path) -> Option<String> {
    let abs = dir.canonicalize().ok()?;
    let mut current = abs.as_path();
    loop {
        let toml_path = current.join(".mengdie.toml");
        if toml_path.is_file() {
            if let Ok(contents) = std::fs::read_to_string(&toml_path) {
                // Minimal TOML parsing — avoid adding a dependency for one field.
                // Looks for `name = "value"` under `[project]`.
                let mut in_project_section = false;
                for line in contents.lines() {
                    let trimmed = line.trim();
                    if trimmed == "[project]" {
                        in_project_section = true;
                        continue;
                    }
                    if trimmed.starts_with('[') {
                        in_project_section = false;
                        continue;
                    }
                    if in_project_section && trimmed.starts_with('#') {
                        continue; // skip comments
                    }
                    if in_project_section {
                        if let Some(rest) = trimmed.strip_prefix("name") {
                            let rest = rest.trim_start();
                            if let Some(rest) = rest.strip_prefix('=') {
                                let val = rest.trim();
                                // Extract quoted string properly (handle inline comments).
                                // NOTE: intentionally NOT escape-aware — `"foo\"bar"` terminates
                                // at the first unescaped `"` after the opening. See
                                // test_read_project_name_quote_in_value_terminates_early.
                                let val = if let Some(rest) = val.strip_prefix('"') {
                                    rest.find('"').map(|end| &rest[..end])
                                } else if let Some(rest) = val.strip_prefix('\'') {
                                    rest.find('\'').map(|end| &rest[..end])
                                } else {
                                    // Unquoted: take until whitespace or comment
                                    Some(val.split_once('#').map(|(v, _)| v.trim()).unwrap_or(val))
                                };
                                if let Some(val) = val {
                                    if !val.is_empty() {
                                        return Some(val.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Stop at git root or filesystem root
        if current.join(".git").exists() {
            break;
        }
        current = current.parent()?;
    }
    None
}

/// Compute the hash-based project_id for a given directory (used by migrate command).
pub fn hash_project_id(dir: &Path) -> String {
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

    // -- .mengdie.toml tests --

    #[test]
    fn test_read_project_name_found_in_current_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".mengdie.toml"),
            "[project]\nname = \"my-project\"\n",
        )
        .unwrap();
        assert_eq!(
            read_project_name(dir.path()),
            Some("my-project".to_string())
        );
    }

    #[test]
    fn test_read_project_name_found_in_ancestor() {
        let dir = tempfile::tempdir().unwrap();
        // Write .mengdie.toml in root, create a subdir, read from subdir
        std::fs::write(
            dir.path().join(".mengdie.toml"),
            "[project]\nname = \"parent-project\"\n",
        )
        .unwrap();
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        assert_eq!(
            read_project_name(&subdir),
            Some("parent-project".to_string())
        );
    }

    #[test]
    fn test_read_project_name_not_found_fallback_hash() {
        let dir = tempfile::tempdir().unwrap();
        // No .mengdie.toml → read_project_name returns None
        assert_eq!(read_project_name(dir.path()), None);
        // infer_project_id falls back to hash
        let id = infer_project_id(dir.path());
        assert!(id.starts_with("proj_"));
    }

    #[test]
    fn test_read_project_name_empty_name_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".mengdie.toml"), "[project]\nname = \"\"\n").unwrap();
        // Empty name should be treated as absent
        assert_eq!(read_project_name(dir.path()), None);
    }

    #[test]
    fn test_read_project_name_malformed_toml() {
        let dir = tempfile::tempdir().unwrap();
        // No [project] section
        std::fs::write(dir.path().join(".mengdie.toml"), "name = \"no-section\"\n").unwrap();
        assert_eq!(read_project_name(dir.path()), None);
    }

    #[test]
    fn test_read_project_name_wrong_section() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".mengdie.toml"),
            "[other]\nname = \"wrong\"\n[project]\nname = \"right\"\n",
        )
        .unwrap();
        assert_eq!(read_project_name(dir.path()), Some("right".to_string()));
    }

    #[test]
    fn test_read_project_name_single_quotes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".mengdie.toml"),
            "[project]\nname = 'single-quoted'\n",
        )
        .unwrap();
        assert_eq!(
            read_project_name(dir.path()),
            Some("single-quoted".to_string())
        );
    }

    #[test]
    fn test_infer_project_id_uses_toml_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".mengdie.toml"),
            "[project]\nname = \"test-project\"\n",
        )
        .unwrap();
        // Should return the name directly, not a hash
        assert_eq!(infer_project_id(dir.path()), "test-project");
    }

    #[test]
    fn test_read_project_name_inline_comment() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".mengdie.toml"),
            "[project]\nname = \"my-project\"  # set by CI\n",
        )
        .unwrap();
        assert_eq!(
            read_project_name(dir.path()),
            Some("my-project".to_string())
        );
    }

    #[test]
    fn test_read_project_name_comment_line_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".mengdie.toml"),
            "[project]\n# name = \"commented-out\"\nname = \"active\"\n",
        )
        .unwrap();
        assert_eq!(read_project_name(dir.path()), Some("active".to_string()));
    }

    #[test]
    fn test_read_project_name_stops_at_git_root() {
        let dir = tempfile::tempdir().unwrap();
        // Create a fake .git dir in root — prevents walking higher
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        // Put .mengdie.toml one level above (unreachable)
        // Since we can't go above tempdir easily, test that .git stops the walk
        let subdir = dir.path().join("sub");
        std::fs::create_dir(&subdir).unwrap();
        // No .mengdie.toml anywhere in the tree
        assert_eq!(read_project_name(&subdir), None);
    }

    /// Regression guard for plan 008 Step 1 `manual_strip` refactor in
    /// `read_project_name`. The refactor switches `val[1..].find('"')`-style
    /// indexing to `strip_prefix('"')` + `find`, which MUST preserve the
    /// exact current behavior including:
    ///   (a) our TOML-ish parser is NOT escape-aware — a `\"` inside a
    ///       quoted value terminates the string at the `"` following the
    ///       backslash; the `\` is NOT consumed. This test captures that
    ///       existing behavior so the clippy-style refactor cannot
    ///       silently "fix" it into real escape handling.
    ///   (b) multi-byte UTF-8 inside a quoted value is returned intact.
    /// If this test ever needs to change, the refactor has altered
    /// semantics — reject the diff.
    #[test]
    fn test_read_project_name_quote_in_value_terminates_early() {
        // Input line in the file: name = "foo\"bar"
        // Our parser stops at the first unescaped `"`, so the extracted
        // value is "foo\" (backslash preserved, `bar"` dropped).
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".mengdie.toml"),
            "[project]\nname = \"foo\\\"bar\"\n",
        )
        .unwrap();
        assert_eq!(
            read_project_name(dir.path()),
            Some("foo\\".to_string()),
            "parser is intentionally not escape-aware; refactor must preserve this"
        );
    }

    #[test]
    fn test_read_project_name_unicode_value_preserved() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".mengdie.toml"),
            "[project]\nname = \"项目名称\"\n",
        )
        .unwrap();
        assert_eq!(
            read_project_name(dir.path()),
            Some("项目名称".to_string()),
            "multi-byte UTF-8 content must pass through unchanged"
        );
    }
}
