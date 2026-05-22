use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Context;
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};

use super::parser::is_ingestable;

/// File event from the watcher (after debounce).
#[derive(Debug)]
pub struct FileEvent {
    pub path: PathBuf,
}

/// Start watching directories for AE output files.
/// Returns a receiver that yields debounced file events for ingestable files.
/// The debouncer handle must be kept alive (dropping it stops watching).
pub fn start_watcher(
    dirs: &[PathBuf],
) -> anyhow::Result<(
    mpsc::Receiver<FileEvent>,
    Debouncer<notify::RecommendedWatcher>,
)> {
    let (raw_tx, raw_rx) = mpsc::channel::<DebounceEventResult>();
    let (tx, rx) = mpsc::channel();

    // Forward debounced events, filtering to ingestable files
    std::thread::spawn(move || {
        for result in raw_rx {
            match result {
                Ok(events) => {
                    for event in events {
                        if event.path.exists() && is_ingestable(&event.path) {
                            let _ = tx.send(FileEvent { path: event.path });
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!(error = %err, "watcher error");
                }
            }
        }
    });

    let mut debouncer =
        new_debouncer(Duration::from_millis(500), raw_tx).context("failed to create debouncer")?;

    for dir in dirs {
        if dir.exists() {
            debouncer
                .watcher()
                .watch(dir, RecursiveMode::Recursive)
                .with_context(|| format!("failed to watch {}", dir.display()))?;
            tracing::info!(dir = %dir.display(), "watching directory");
        } else {
            tracing::warn!(dir = %dir.display(), "watch directory does not exist, skipping");
        }
    }

    Ok((rx, debouncer))
}

/// Process file events from the watcher in a loop.
/// Calls `on_file` for each ingestable file event.
/// Blocks the calling thread. Use in a dedicated thread or spawn_blocking.
pub fn watch_loop<F>(rx: mpsc::Receiver<FileEvent>, mut on_file: F)
where
    F: FnMut(&Path),
{
    for event in rx {
        on_file(&event.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_watcher_detects_file() {
        let dir = tempfile::tempdir().unwrap();
        let (rx, _debouncer) = start_watcher(&[dir.path().to_path_buf()]).unwrap();

        // Small delay for watcher to register
        std::thread::sleep(Duration::from_millis(100));

        // Create an ingestable file
        let path = dir.path().join("conclusion.md");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "# Test").unwrap();
        drop(f);

        // Wait for event (up to 3 seconds — debouncer adds 500ms delay)
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
        let (rx, _debouncer) = start_watcher(&[dir.path().to_path_buf()]).unwrap();

        // Create a non-ingestable file
        let path = dir.path().join("round-01.md");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "# Round 1").unwrap();
        drop(f);

        // Should NOT get an event (wait longer than debounce window)
        let event = rx.recv_timeout(Duration::from_secs(2));
        assert!(
            event.is_err(),
            "should not receive event for non-ingestable file"
        );
    }

    #[test]
    fn test_watcher_skips_nonexistent_dir() {
        let result = start_watcher(&[PathBuf::from("/nonexistent/dir/12345")]);
        // Should succeed (skips nonexistent with warning), not error
        assert!(result.is_ok());
    }
}
