use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Context;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};

use super::parser::is_ingestable;

/// File event from the watcher (after debounce).
#[derive(Debug)]
pub struct FileEvent {
    pub path: PathBuf,
}

/// Start watching directories for AE output files.
/// Returns a receiver that yields debounced file events for ingestable files.
/// The watcher handle must be kept alive (dropping it stops watching).
pub fn start_watcher(
    dirs: &[PathBuf],
) -> anyhow::Result<(mpsc::Receiver<FileEvent>, RecommendedWatcher)> {
    let (tx, rx) = mpsc::channel();

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) => {
                    for path in event.paths {
                        if is_ingestable(&path) {
                            let _ = tx.send(FileEvent { path });
                        }
                    }
                }
                _ => {}
            }
        }
    })
    .context("failed to create file watcher")?;

    for dir in dirs {
        if dir.exists() {
            watcher
                .watch(dir, RecursiveMode::Recursive)
                .with_context(|| format!("failed to watch {}", dir.display()))?;
            tracing::info!(dir = %dir.display(), "watching directory");
        } else {
            tracing::warn!(dir = %dir.display(), "watch directory does not exist, skipping");
        }
    }

    Ok((rx, watcher))
}

/// Process file events from the watcher in a loop.
/// Calls `on_file` for each ingestable file event.
/// Blocks the calling thread. Use in a dedicated thread or spawn_blocking.
pub fn watch_loop<F>(rx: mpsc::Receiver<FileEvent>, mut on_file: F)
where
    F: FnMut(&Path),
{
    // Simple debounce: collect events for 500ms, deduplicate by path
    loop {
        // Wait for first event
        let first = match rx.recv() {
            Ok(e) => e,
            Err(_) => break, // Channel closed
        };

        // Collect more events for 500ms
        let mut paths = std::collections::HashSet::new();
        paths.insert(first.path);

        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match rx.recv_timeout(remaining) {
                Ok(e) => {
                    paths.insert(e.path);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }

        // Process deduplicated paths
        for path in paths {
            on_file(&path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_watcher_detects_file() {
        let dir = tempfile::tempdir().unwrap();
        let (rx, _watcher) = start_watcher(&[dir.path().to_path_buf()]).unwrap();

        // Small delay for watcher to register
        std::thread::sleep(Duration::from_millis(100));

        // Create an ingestable file
        let path = dir.path().join("conclusion.md");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "# Test").unwrap();
        drop(f);

        // Wait for event (up to 3 seconds)
        let event = rx.recv_timeout(Duration::from_secs(3));
        assert!(event.is_ok(), "expected file event within 3 seconds");
        let event_path = event.unwrap().path;
        assert_eq!(
            event_path.file_name().unwrap().to_string_lossy(),
            "conclusion.md"
        );
    }

    #[test]
    fn test_watcher_ignores_non_ingestable() {
        let dir = tempfile::tempdir().unwrap();
        let (rx, _watcher) = start_watcher(&[dir.path().to_path_buf()]).unwrap();

        // Create a non-ingestable file
        let path = dir.path().join("round-01.md");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "# Round 1").unwrap();
        drop(f);

        // Should NOT get an event
        let event = rx.recv_timeout(Duration::from_secs(1));
        assert!(event.is_err(), "should not receive event for non-ingestable file");
    }

    #[test]
    fn test_watcher_skips_nonexistent_dir() {
        let result = start_watcher(&[PathBuf::from("/nonexistent/dir/12345")]);
        // Should succeed (skips nonexistent with warning), not error
        assert!(result.is_ok());
    }
}
