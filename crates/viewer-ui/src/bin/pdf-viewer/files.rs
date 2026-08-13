//! What this host does with the filesystem, which `viewer-core`'s rule 2 says is a host's alone.
//!
//! Four decisions, each of them a document asking this machine for something: a file to import
//! (§12.7.6.4), an embedded file to write out (§7.11.4, §O.2.1), §7.5.6's update to save, and an
//! operation the document says its reader may not perform. Two of the four are the same decision
//! all three hosts make and are stated once in `viewer_host::policy`; what stays here is the
//! sentence a person in a terminal reads, because that part is this window's.

use std::path::Path;

use viewer_core::Purpose;
use viewer_host::ImportRefusal;

use crate::app::App;

impl App {
    /// Writes an extracted embedded file beside the document.
    ///
    /// **Rule 2 in the other direction**: the core produced the bytes and the host decides where
    /// they go. Beside the open document and nowhere else, which is the mirror of the policy
    /// §12.7.6.4's import takes — and the file's own name is a string *the document wrote*, so
    /// only its last component is used and a name that is a path, is empty, or is `..` is
    /// refused rather than followed. §7.11.4 states no policy at all, because a policy is a
    /// property of the processor.
    pub(crate) fn write_extracted(&self, asked: viewer_core::Extraction, name: &str, bytes: &[u8]) {
        // §O.2.1's own sentence, decided once for all three hosts in `viewer_host::policy`: a URI
        // that named a file is not a person who asked for one.
        if let Err(refusal) = viewer_host::may_write_extracted(asked) {
            println!("note: {refusal}");
            return;
        }
        let stem = Path::new(name).file_name();
        let Some(stem) = stem.filter(|stem| !stem.is_empty()) else {
            println!("note: the embedded file's name {name:?} is not a file name");
            return;
        };
        let path = self.directory.clone().unwrap_or_default().join(stem);
        match std::fs::write(&path, bytes) {
            Ok(()) => println!("extracted {} bytes to {}", bytes.len(), path.display()),
            Err(error) => println!("note: cannot write {}: {error}", path.display()),
        }
    }

    /// §7.5.6's update, written beside the document rather than over it.
    ///
    /// Rule 2 in one method: the core produced the bytes and the host owns the filesystem.
    /// `.edited.pdf` appended rather than the file replaced, because overwriting somebody's
    /// document is a decision this program has not been given.
    pub(crate) fn write_saved(&self, bytes: &[u8]) {
        let path = self.path.with_extension("edited.pdf");
        match std::fs::write(&path, bytes) {
            Ok(()) => println!("saved {} bytes to {}", bytes.len(), path.display()),
            Err(error) => println!("note: cannot write {}: {error}", path.display()),
        }
    }

    /// What this window does about an operation the document restricted.
    ///
    /// Said rather than swallowed, and said as the *reader's* doing rather than as the
    /// document's: this host obeys what a file asserts unless it was started with
    /// `--ignore-restrictions`, and a person whose keystroke did nothing is owed both the clause
    /// and the way out. Trap 5 on the one path where this program declines on somebody else's
    /// instructions rather than on its own.
    ///
    /// **This is not a user interface for the levels**, which `CLAUDE.md` says is not to be built
    /// yet. It is the sentence a person needs, in the terminal this program already prints to.
    pub(crate) fn say_refused(notes: &[String]) {
        for note in notes {
            println!("note: {note}");
        }
        println!(
            "note: this reader is obeying that; --ignore-restrictions turns it off \
             (CLAUDE.md: a document's restrictions are the reader's to set)"
        );
    }

    /// §12.7.6.4's file, under the narrowest policy that still performs the action.
    ///
    /// The clause says a processor "shall import data … from a specified file" and specifies
    /// nothing about *which* files a document may name, because that is a property of the
    /// processor. So this states the policy, and it is a host's to state:
    ///
    /// - the name must be a single path component, so `../…` and any absolute path are refused;
    /// - it is resolved against the directory the open document is in, and nowhere else.
    ///
    /// Both rules are `viewer_host::resolve_import`, which is where the three hosts keep the one
    /// decision rather than three copies of it; what is this window's is the sentence below,
    /// because a terminal is the only place this host has to say anything.
    ///
    /// Every refusal is printed, which is trap 5 on the one path where a click can decline.
    pub(crate) fn supply(&self, purpose: Purpose, name: &str) -> Option<Vec<u8>> {
        let Purpose::ImportData = purpose;
        let path = match viewer_host::resolve_import(self.directory.as_deref(), name) {
            Ok(path) => path,
            Err(ImportRefusal::NoDirectory) => {
                println!("import-data: declined — the document is not in a known directory");
                return None;
            }
            Err(ImportRefusal::NotAPlainName { name }) => {
                println!(
                    "import-data: declined — {name} is not a plain file name beside the document"
                );
                return None;
            }
        };
        match std::fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                println!("import-data: cannot read {}: {error}", path.display());
                None
            }
        }
    }
}
