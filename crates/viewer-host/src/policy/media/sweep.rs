//! This program's old clip directories, removed at the next hand-off (ADR 3218).
//!
//! [`super::hand_media`] writes an embedded clip into a fresh `quorra-media-*` directory and
//! leaves it there, because the player opens it at a moment this program is not told (ADR 3202).
//! What is left behind is swept the next time a clip is written: a directory of this program's
//! naming, owned by this user, last changed more than [`SWEPT_AFTER`] ago.

use std::path::Path;
use std::time::{Duration, SystemTime};

/// The prefix of every directory [`super::hand_media`] writes a clip into.
pub const MEDIA_DIRECTORY_PREFIX: &str = "quorra-media-";

/// How long a written clip is left for its player before the next hand-off may remove it.
///
/// **A day, and the argument is what a removal can break** (ADR 3218). A player that holds the
/// file open is not disturbed on Unix, where removing a name leaves an open file readable, and on
/// Windows the removal of an open file fails and is tried again at a later hand-off. What a
/// removal breaks is a player that opens the file *again by its name* — a replay, a recent-files
/// list — and that happens within the sitting in which the clip was played. A day outlasts a
/// sitting, a window left open overnight included, and bounds what is kept to one day's clips on a
/// directory that `$XDG_RUNTIME_DIR` already empties at logout.
pub const SWEPT_AFTER: Duration = Duration::from_hours(24);

/// Removes every directory under `directory` that this program wrote a clip into, that belongs to
/// the user who owns `ours`, and that was last changed more than [`SWEPT_AFTER`] before `now`;
/// `ours`, the directory just written, is never one of them.
///
/// The owner is read from `ours` rather than asked of the system: the directory this call just
/// created is this user's by construction, and under a shared temporary directory a name of this
/// program's form made by another user is left alone. Windows' temporary directory is the user's
/// own, so there the name and the age decide. Nothing is followed through a symbolic link, and
/// [`std::fs::remove_dir_all`] does not follow one inside.
///
/// Answers a sentence for each directory that could not be read or removed; one another window
/// removed first is not a failure.
#[must_use]
pub fn sweep(directory: &Path, ours: &Path, now: SystemTime) -> Vec<String> {
    let Some(cutoff) = now.checked_sub(SWEPT_AFTER) else {
        return Vec::new();
    };
    let owner = match std::fs::symlink_metadata(ours) {
        Ok(metadata) => metadata,
        Err(error) => return vec![format!("cannot read {}: {error}", ours.display())],
    };
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => return vec![format!("cannot read {}: {error}", directory.display())],
    };
    let mut failed = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                failed.push(format!("cannot read {}: {error}", directory.display()));
                continue;
            }
        };
        let path = entry.path();
        let ours_by_name = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(MEDIA_DIRECTORY_PREFIX));
        if !ours_by_name || path == ours {
            continue;
        }
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                failed.push(format!("cannot read {}: {error}", path.display()));
                continue;
            }
        };
        let old = metadata.modified().is_ok_and(|changed| changed < cutoff);
        if !metadata.file_type().is_dir() || !same_owner(&metadata, &owner) || !old {
            continue;
        }
        match std::fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => failed.push(format!("cannot remove {}: {error}", path.display())),
        }
    }
    failed
}

#[cfg(unix)]
fn same_owner(found: &std::fs::Metadata, ours: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    found.uid() == ours.uid()
}

#[cfg(not(unix))]
fn same_owner(_found: &std::fs::Metadata, _ours: &std::fs::Metadata) -> bool {
    true
}
