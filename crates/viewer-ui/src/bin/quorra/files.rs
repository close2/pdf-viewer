//! What this host does with the filesystem, which `viewer-core`'s rule 2 says is a host's alone.
//!
//! Four decisions, each of them a document asking this machine for something: a file to import
//! (§12.7.6.4), an embedded file to write out (§7.11.4, §O.2.1), §7.5.6's update to save, and an
//! operation the document says its reader may not perform. Two of the four are the same decision
//! all three hosts make and are stated once in `viewer_host::policy`; what stays here is the
//! sentence a person in a terminal reads, because that part is this window's.

use std::collections::VecDeque;
use std::path::Path;

use viewer_core::{Command, Purpose};
use viewer_host::ImportRefusal;

use crate::app::App;

impl App {
    /// What this window does with §7.11.4's bytes when they arrive: opens them, or writes them.
    ///
    /// **Annex O's `ef` is a `shall` about *opening*, and this is where it is carried out.**
    /// ISO 32000-2 §O.2.1, Table Annex O.3:
    ///
    /// > When used as part of a PDF open parameter, the PDF processor shall open the embedded file
    /// > contained within the EmbeddedFiles name tree identified by name .
    ///
    /// > Any remaining parameters after this parameter apply to the selected embedded file.
    ///
    /// `viewer-core` has no `Command::Open` of its own to send (rule 2), so it hands the bytes and
    /// the rest of the fragment over and a host composes the two — which is the whole of the second
    /// sentence: `Command::Open` with those bytes and that fragment applies `page`, `search` or
    /// `highlight` to the file that came out rather than to the one it came out of.
    ///
    /// **One window, so the embedded document replaces the one that named it**, and that is this
    /// host's choice rather than the annex's requirement: everything after `ef` is a sentence about
    /// the embedded file, so the file the URI is *about* is the one this window shows. A host with
    /// tabs would open a second `DocumentId` beside the first and needs no other change.
    ///
    /// **Only a PDF**, because this window can show nothing else: an embedded spreadsheet is handed
    /// to [`Self::write_extracted`] and its policy, which is where a person can still get at it.
    /// The header is §7.5.2's, checked here rather than by trying an open and printing a failure a
    /// person did not ask for.
    ///
    /// It terminates without a counter: each open consumes at least one `ef=…` from the fragment,
    /// so a document embedding itself under a name its own fragment repeats still runs out of
    /// fragment. See `pdf_model::fragment::Fragment::after_embedded_file`.
    pub(crate) fn extracted(
        &mut self,
        asked: viewer_core::Extraction,
        name: &str,
        bytes: Vec<u8>,
        fragment: Option<String>,
        queue: &mut VecDeque<Command>,
    ) {
        if !matches!(asked, viewer_core::Extraction::Fragment) || !bytes.starts_with(b"%PDF-") {
            self.write_extracted(asked, name, &bytes);
            return;
        }
        // The other half of §O.2.1's row, asked once and in the place all three hosts ask it.
        if let Err(refusal) = viewer_host::may_open_extracted(asked) {
            println!("note: {refusal}");
            return;
        }
        match &fragment {
            Some(rest) => println!("opening the embedded file {name:?} at `{rest}` (§O.2.1)"),
            None => println!("opening the embedded file {name:?} (§O.2.1)"),
        }
        name.clone_into(&mut self.title);
        // §7.6.4.1's prompt re-opens the document, and an embedded one has no path to re-read.
        self.embedded = Some(bytes.clone());
        self.fragment.clone_from(&fragment);
        queue.push_back(Command::Open {
            id: crate::DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment,
        });
    }

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
    ///
    /// **The wording is `viewer_host::refused`'s since ADR 0604**, and this host wrote it for
    /// itself until then. The other two wrote it too, identically, naming a flag neither of their
    /// argument parsers took — so the third copy of a sentence turned out to be where two hosts
    /// stop agreeing not with each other but with *themselves*.
    pub(crate) fn say_refused(notes: &[String]) {
        println!("note: {}", viewer_host::refused(notes));
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
