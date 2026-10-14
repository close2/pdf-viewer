//! §7.5.6's incremental save, and §7.11.4's embedded files extracted or opened.

use std::rc::Rc;

use viewer_core::{DocumentId, Extraction};
use viewer_host::native_window::NativeWindow;

use super::Host;

impl Host {
    /// §7.11.4's bytes came out: a document §O.2.1's `ef` asked to be opened, or a file to write.
    ///
    /// ISO 32000-2 §O.2.1, Table Annex O.3's `ef` row:
    ///
    /// > When used as part of a PDF open parameter, the PDF processor shall open the embedded file
    /// > contained within the EmbeddedFiles name tree identified by name .
    ///
    /// This window has a strip of tabs, so the embedded document opens in one of its own and the document holding it keeps its tab. What
    /// follows `ef` in the fragment goes with it as its own fragment, which is the row's next
    /// sentence. It waits in `viewer_host::Arrivals` like any document a reader named, so a
    /// password it asks for is asked about it, and it starts from the idle queue rather than
    /// inside the pump that extracted it (ADR 1275).
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
            viewer_host::Unpacking::Ask(question) => {
                let name = name.to_owned();
                let bytes = Rc::new(bytes);
                self.put_a_question(
                    viewer_host::Subject::Embedded.title(),
                    &question,
                    Rc::new(move |host: &mut Self, proceed| {
                        if proceed {
                            host.open_embedded(
                                document,
                                &name,
                                bytes.as_ref().clone(),
                                fragment.clone(),
                            );
                        } else {
                            host.say(&viewer_host::embedded_declined(&name));
                        }
                    }),
                );
            }
        }
    }
}
