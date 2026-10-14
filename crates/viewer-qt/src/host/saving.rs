//! What this window writes: §7.5.6's update beside the document, §7.11.4's extracted files, and
//! §O.2.1's embedded document opened in a tab (filed here by ADR 1908).

use std::path::Path;

use viewer_core::{DocumentId, Extraction};

use super::Host;
use super::questions::Pending;

impl Host {
    /// §7.5.6's update, written beside the document rather than over it.
    pub(super) fn write_saved(&mut self, bytes: &[u8]) {
        let path = self.showing.path.with_extension("edited.pdf");
        match std::fs::write(&path, bytes) {
            Ok(()) => self.say(&format!(
                "saved {} bytes to {}",
                bytes.len(),
                path.display()
            )),
            Err(error) => self.say(&format!("cannot write {}: {error}", path.display())),
        }
    }

    /// §7.11.4's bytes came out: a document §O.2.1's `ef` asked to be opened, or a file to write.
    ///
    /// ISO 32000-2 §O.2.1, Table Annex O.3's `ef` row:
    ///
    /// > When used as part of a PDF open parameter, the PDF processor shall open the embedded file
    /// > contained within the EmbeddedFiles name tree identified by name .
    ///
    /// This window has a strip of tabs, so the embedded document opens in one of its own and the document holding it keeps its tab. What
    /// follows `ef` in the fragment goes with it as its own fragment. It waits in
    /// `viewer_host::Arrivals` like any document a reader named, and starts from the zero-length
    /// timer rather than inside the pump that extracted it (ADR 1275).
    pub(super) fn extracted(
        &mut self,
        document: DocumentId,
        asked: Extraction,
        name: &str,
        bytes: Vec<u8>,
        fragment: Option<String>,
    ) {
        if !viewer_host::opens_as_document(asked, &bytes) {
            self.write_extracted(asked, name, &bytes);
            return;
        }
        // §O.2.1's caution, under the level the menu holds: `viewer_host::may_open_extracted`
        // decides, and this window carries out the answer and puts the question (ADR 1331).
        match viewer_host::may_open_extracted(asked, name, self.restrictions.embedded_documents()) {
            viewer_host::Unpacking::Open => self.open_embedded(document, name, bytes, fragment),
            viewer_host::Unpacking::Warn(note) => {
                self.say(&note);
                self.open_embedded(document, name, bytes, fragment);
            }
            viewer_host::Unpacking::Refuse(why) => self.say(&why),
            viewer_host::Unpacking::Ask(question) => self.put_the_question(
                Pending::Embedded {
                    document,
                    name: name.to_owned(),
                    bytes,
                    fragment,
                },
                &question,
            ),
        }
    }

    /// §O.2.1's embedded document, on its way to a tab of its own beside the one holding it.
    pub(super) fn open_embedded(
        &mut self,
        document: DocumentId,
        name: &str,
        bytes: Vec<u8>,
        fragment: Option<String>,
    ) {
        self.say(&viewer_host::opening_embedded(name, fragment.as_deref()));
        let named = viewer_host::Named::embedded(self.showing.directory.as_deref(), name, fragment);
        let behind = document != self.documents.focused();
        self.arrivals.wait_held(named, bytes.into(), behind);
        self.later_open_the_next();
    }

    /// §7.11.4's file, written beside the document.
    ///
    /// The name is the document's own words and not a path — §7.11.4's note is that `/F` is "a
    /// platform-dependent encoding" — so only its last component is used and only beside the
    /// document, which is the same policy §12.7.6.4's import takes in the other direction.
    fn write_extracted(&mut self, asked: Extraction, name: &str, bytes: &[u8]) {
        // §O.2.1's own sentence, decided once for all three hosts in `viewer_host::policy`: a URI
        // that named a file is not a person who asked for one.
        if let Err(refusal) = viewer_host::may_write_extracted(asked) {
            self.say(&refusal);
            return;
        }
        let Some(directory) = self.showing.directory.clone() else {
            self.say("cannot write the attachment: the document is not in a known directory");
            return;
        };
        let single = Path::new(name).file_name().unwrap_or(name.as_ref());
        let path = directory.join(single);
        match std::fs::write(&path, bytes) {
            Ok(()) => self.say(&format!(
                "wrote {} bytes to {}",
                bytes.len(),
                path.display()
            )),
            Err(error) => self.say(&format!("cannot write {}: {error}", path.display())),
        }
    }
}
