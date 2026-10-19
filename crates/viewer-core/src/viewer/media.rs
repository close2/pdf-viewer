//! A rich-media or sound annotation's activation, handed to the host as a clip.

use super::{DocumentId, Viewer};
#[cfg(doc)]
use crate::Command;
use crate::event::{Event, Extraction};

impl Viewer {
    /// §13.2.4.2's clip, asked of the reader's level for Table 286's `/TF` and handed over if it is
    /// let through — the four verdicts as [`Self::edit`] has them.
    ///
    /// **Only embedded data asks**: `/TF` governs "a temporary file in order to play a media clip",
    /// and a player handed a URL or a named file is given no file this program wrote. ADR 1961.
    pub(super) fn media(
        &mut self,
        id: DocumentId,
        clip: pdf_model::media::Clip,
        events: &mut Vec<Event>,
    ) {
        use crate::notes::{Standing as Worded, restricted};
        use pdf_model::restriction::{Operation, Verdict};
        let operation = Operation::HandOverMedia;
        let level = self.restrictions;
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };
        let level = level.under(open.restrictions).level(operation).level();
        let asserted = match clip.data {
            pdf_model::media::Data::Embedded(_) => {
                pdf_model::restriction::media(&open.document, clip.temporary_file)
            }
            _ => Vec::new(),
        };
        let verdict = level.verdict(asserted);
        if let Verdict::Refuse(restrictions) = verdict {
            events.push(Event::Refused {
                document: id,
                operation,
                notes: restricted(operation, &restrictions, Worded::Refused),
            });
            return;
        }
        let Some(media) = media_of(id, &open.document, &clip, events) else {
            return;
        };
        let content_type = clip.content_type;
        match verdict {
            // `Refuse` returned above, before anything was decoded.
            Verdict::Proceed | Verdict::Refuse(_) => events.push(Event::Media {
                document: id,
                content_type,
                media,
            }),
            Verdict::Warn(restrictions) => {
                events.push(Event::Media {
                    document: id,
                    content_type,
                    media,
                });
                events.push(Event::Warned {
                    document: id,
                    operation,
                    notes: restricted(operation, &restrictions, Worded::Warned),
                });
            }
            Verdict::Ask(restrictions) => {
                open.asking = Some(crate::open::Held::Media {
                    content_type,
                    media,
                });
                events.push(Event::Asking {
                    document: id,
                    operation,
                    notes: restricted(operation, &restrictions, Worded::Asked),
                });
            }
        }
    }
}

/// Hands one embedded file's decoded bytes to the host, saying what §7.11.4 lets a file get wrong.
///
/// Two callers and one set of sentences: [`Command::Extract`], which names a file §7.7.4's tree
/// filed, and a click on §12.5.6.15's file attachment annotation, which names none because an
/// annotation files its file under nothing. Both owe the same three statements, which is why
/// this is one function rather than two that drifted.
/// What [`Event::Media`] hands over for one clip, its embedded data decoded here for
/// [`hand_over`]'s reason: a host that had to undo §7.4's filters would be a second reader of the
/// document.
fn media_of(
    id: DocumentId,
    document: &pdf_syntax::Document,
    clip: &pdf_model::media::Clip,
    events: &mut Vec<Event>,
) -> Option<crate::Media> {
    use pdf_model::media::Data;
    let media = match &clip.data {
        Data::Embedded(file) => {
            let name = file.file_name.clone().unwrap_or_else(|| file.name.clone());
            // Trap 5, as `hand_over` has it: a stream that will not decode is said, never handed
            // over empty.
            let Some(bytes) = document.decoded_stream_data(&file.stream) else {
                events.push(Event::Reported {
                    document: id,
                    page: None,
                    notes: vec![format!(
                        "the media clip {name:?} states a filter or an encryption this program \
                         cannot undo, so nothing was handed over"
                    )],
                });
                return None;
            };
            crate::Media::Embedded {
                name,
                bytes: bytes.to_vec(),
            }
        }
        Data::Url(url) => crate::Media::Url(url.clone()),
        Data::File(name) => crate::Media::File(name.clone()),
        // `interact::rendered` keeps a form XObject back: its player is this processor.
        Data::Form => return None,
    };
    Some(media)
}

pub(super) fn hand_over(
    id: DocumentId,
    asked: Extraction,
    document: &pdf_syntax::Document,
    file: &pdf_model::attachment::Attachment,
    fragment: Option<String>,
    events: &mut Vec<Event>,
) {
    let name = file.file_name.clone().unwrap_or_else(|| file.name.clone());
    // Trap 5 on the one path where a *stream* can refuse: §7.6.6 makes a crypt filter's absent
    // key the stream's answer rather than the file's, and two of the corpus's twenty-three
    // attachments are exactly that. A silent empty file would be worse than none.
    let Some(bytes) = document.decoded_stream_data(&file.stream) else {
        events.push(Event::Reported {
            document: id,
            page: None,
            notes: vec![format!(
                "the embedded file {name:?} states a filter or an encryption this program cannot \
                 undo"
            )],
        });
        return;
    };
    // Table 45's `/CheckSum` is the one entry of §7.11.4 that can only be answered by somebody
    // holding the decoded bytes, and this is the only place in the program that does. Said and
    // not acted on: the clause calls it "strictly a checksum, and … not used for security
    // purposes", so a mismatch is the producer's mistake and withholding the file would tell a
    // person less than handing it over with a sentence.
    if file.checksum_matches(&bytes) == Some(false) {
        events.push(Event::Reported {
            document: id,
            page: None,
            notes: vec![format!(
                "the embedded file {name:?} does not match the MD5 checksum the document states \
                 for it (§7.11.4, Table 45)"
            )],
        });
    }
    events.push(Event::Extracted {
        document: id,
        asked,
        // Table 43's own name for the file where it states one, because that is what a person
        // would call it; the tree's key otherwise, which is all there is.
        name,
        bytes: bytes.to_vec(),
        // §O.2.1's remaining parameters, for the one caller that has a URI behind it.
        fragment,
    });
}
