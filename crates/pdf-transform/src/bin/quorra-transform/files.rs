//! The outputs a pass writes to files: each staged beside its name and put in place only when the
//! pass succeeded (ADR 3812).
//!
//! The serializer writes a document as it walks it (`pdf_syntax::serialize`), so a verb that fails
//! after its first byte — a stream that does not encode, a budget passed, a full disk — has
//! already written part of a file. Written straight to its name, that part would be left there: a
//! truncated PDF under the name the person asked for, over whatever file stood there before. A
//! redaction is where that costs most, since the file it replaces may be the one the person meant
//! to keep and the part written is not a redaction of anything. So every output is written to a
//! hidden file in the same directory, and [`Sinks::commit`] renames each onto its name once the
//! verb has answered; sinks dropped uncommitted remove what they staged. The same directory
//! because a rename within one file system is the one step that replaces a file whole (POSIX
//! `rename(2)`): a reader of the name sees the old file or the new one, never a part.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pdf_transform::Sinks;

/// One output staged: the hidden file written, and the name it is renamed onto.
#[derive(Debug)]
struct Staged {
    /// The file being written.
    hidden: PathBuf,
    /// The name the plan expanded.
    name: PathBuf,
}

/// Sinks that write files at the names the plan expands, staged until the pass is committed.
///
/// The name is a path as given: relative to the working directory, its parent directory
/// existing. Nothing is created but the file (and, until the commit, its hidden sibling).
#[derive(Debug, Default)]
pub(super) struct FileSinks {
    /// Every output opened and not yet put in place.
    staged: Mutex<Vec<Staged>>,
}

/// The hidden sibling `name` is written as: in its directory, so the rename is within one file
/// system, and named by this process and the output's position so two runs never share one.
fn hidden_beside(name: &Path, position: usize) -> PathBuf {
    let file = name
        .file_name()
        .map_or_else(|| "output".into(), |file| file.to_string_lossy());
    let hidden = format!(".{file}.{}-{position}.partial", std::process::id());
    name.with_file_name(hidden)
}

impl Sinks for FileSinks {
    fn open(&self, name: &str) -> std::io::Result<Box<dyn Write + Send + '_>> {
        let name = PathBuf::from(name);
        let mut staged = self
            .staged
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let hidden = hidden_beside(&name, staged.len());
        // `create_new`: a file already at the hidden name is another writer's, never this one's
        // to truncate.
        let file = std::fs::File::options()
            .write(true)
            .create_new(true)
            .open(&hidden)?;
        staged.push(Staged { hidden, name });
        Ok(Box::new(std::io::BufWriter::new(file)))
    }

    fn commit(&self) -> std::io::Result<()> {
        let mut staged = self
            .staged
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while let Some(output) = staged.first() {
            std::fs::rename(&output.hidden, &output.name).map_err(|error| {
                std::io::Error::new(
                    error.kind(),
                    format!("{} cannot be put in place ({error})", output.name.display()),
                )
            })?;
            staged.remove(0);
        }
        Ok(())
    }
}

impl Drop for FileSinks {
    fn drop(&mut self) {
        let staged = self
            .staged
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for output in staged.drain(..) {
            // A pass that was not committed failed, and its report says how; a hidden file that
            // cannot be removed is left for the person to see beside the name, which is still
            // the file it was before the pass (or absent).
            let _ = std::fs::remove_file(&output.hidden);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh directory under the system's temporary one, removed by the caller.
    fn directory(label: &str) -> PathBuf {
        let at = std::env::temp_dir().join(format!(
            "quorra-transform-files-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&at);
        std::fs::create_dir_all(&at).expect("a temporary directory");
        at
    }

    /// Every file in `at`, by name.
    fn listed(at: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(at)
            .expect("the directory reads")
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_pass_that_fails_after_writing_leaves_the_file_that_was_there() {
        let at = directory("failed");
        let name = at.join("out.pdf");
        std::fs::write(&name, b"the file the person had").expect("the earlier file");
        {
            let sinks = FileSinks::default();
            let mut writer = sinks.open(&name.to_string_lossy()).expect("it opens");
            writer
                .write_all(b"%PDF-2.0\n1 0 obj")
                .expect("the first part");
            writer.flush().expect("the part reaches the disk");
            // The verb fails here, as a serializer error mid-body does: the writer and the sinks
            // are dropped and `commit` is never called.
        }
        assert_eq!(
            std::fs::read(&name).expect("the name still reads"),
            b"the file the person had",
            "a failed pass wrote over the file at its name"
        );
        assert_eq!(listed(&at), ["out.pdf"], "the staged part was left behind");
        let _ = std::fs::remove_dir_all(&at);
    }

    #[test]
    fn a_committed_pass_puts_every_output_in_place() {
        let at = directory("committed");
        let names = [at.join("a.pdf"), at.join("b.pdf")];
        let sinks = FileSinks::default();
        for (name, text) in names.iter().zip([b"first", b"other"]) {
            let mut writer = sinks.open(&name.to_string_lossy()).expect("it opens");
            writer.write_all(text).expect("written");
            writer.flush().expect("flushed");
        }
        assert_eq!(listed(&at).len(), 2, "both staged");
        assert!(!names[0].exists(), "nothing is at a name before the commit");
        sinks.commit().expect("the pass commits");
        drop(sinks);
        assert_eq!(listed(&at), ["a.pdf", "b.pdf"]);
        assert_eq!(std::fs::read(&names[1]).expect("it reads"), b"other");
        let _ = std::fs::remove_dir_all(&at);
    }
}
