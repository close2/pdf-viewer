//! The vfs worker's vocabulary, as bytes.
//!
//! [`crate::worker::Query`] and [`crate::worker::Answer`] made of fixed-width, big-endian fields,
//! length-checked before anything is allocated from them. The *transport* under this — the frame
//! header, the greeting, the socket that carries a descriptor — is `confined_transport`'s and is
//! shared with `viewer-confined`'s wire (ADR 0846); what is here is the half no two protocols
//! could share.
//!
//! # Two properties this module exists to hold
//!
//! **Nothing is dropped in silence.** Every `match` over a [`crate::worker`] enum here names every
//! variant, so a question added to that module fails to compile in this one rather than falling
//! into a catch-all arm.
//!
//! **A message that cannot be decoded is a refusal that says so.** [`WireError`] names what was
//! truncated or unrecognised; nothing here defaults, clamps or guesses. The confined side is the
//! untrusted side of this boundary, so every length it states is a claim: a count is checked
//! against what remains before a single element is reserved.

use std::collections::BTreeMap;

use pdf_transform::attachments::AttachmentEntry;

use crate::worker::{Answer, Query, WorkerError};

/// Greeting bytes, changed whenever this format changes incompatibly.
///
/// A host and a worker from different builds must not talk to each other, and the cheapest place
/// to find that out is the first thing either says. It is also what keeps two protocols on one
/// transport apart: `pdf-view-worker` greets with `PDFVCF05`, and a host that got the wrong
/// program back refuses at nine bytes rather than at the first answer it misreads.
pub(crate) const MAGIC: &[u8; 8] = b"PDFVFS01";

/// Frame kind: the document and the ceilings, from the host. Sent once, at spawn.
pub(crate) const FRAME_OPEN: u8 = 1;
/// Frame kind: a question, from the host.
pub(crate) const FRAME_QUERY: u8 = 2;
/// Frame kind: the worker has the document open and is ready to be asked.
pub(crate) const FRAME_READY: u8 = 3;
/// Frame kind: the answer to a question, from the worker.
pub(crate) const FRAME_ANSWER: u8 = 4;
/// Frame kind: the worker refused, and this says why.
///
/// A refusal is a *response*, not a transport failure: a page this document does not have keeps
/// the worker, exactly as a malformed image keeps `pdf-sandbox`'s.
pub(crate) const FRAME_REFUSAL: u8 = 5;

/// How a document crosses.
mod document_kind {
    /// The bytes follow, whole.
    pub(super) const BYTES: u8 = 0;
    /// The file's length follows, and its descriptor rides beside the frame (ADR 0812).
    pub(super) const ON_DISK: u8 = 1;
}

/// Query discriminants. One per variant of [`Query`].
mod query_kind {
    pub(super) const PAGE_COUNT: u8 = 1;
    pub(super) const EXTRACT_PAGE: u8 = 2;
    pub(super) const RENDER_PAGE: u8 = 3;
    pub(super) const EXTRACT_IMAGES: u8 = 4;
    pub(super) const PAGE_TEXT: u8 = 5;
    pub(super) const ATTACHMENT_INVENTORY: u8 = 6;
    pub(super) const EXTRACT_ATTACHMENT: u8 = 7;
    pub(super) const INFORMATION: u8 = 8;
    pub(super) const METADATA_STREAM: u8 = 9;
    pub(super) const OUTLINE: u8 = 10;
}

/// Answer discriminants. One per variant of [`Answer`].
mod answer_kind {
    pub(super) const COUNT: u8 = 1;
    pub(super) const BYTES: u8 = 2;
    pub(super) const FILES: u8 = 3;
    pub(super) const ATTACHMENTS: u8 = 4;
    pub(super) const ABSENT: u8 = 5;
}

/// Refusal discriminants: the four things a face has to be able to tell apart.
mod refusal_kind {
    pub(super) const PASSWORD_REQUIRED: u8 = 1;
    pub(super) const REFUSED: u8 = 2;
    pub(super) const DECLINED: u8 = 3;
    pub(super) const NOT_PRESENT: u8 = 4;
}

/// Why a message could not be read.
#[derive(Debug, thiserror::Error)]
pub(crate) enum WireError {
    /// The message ended before the field being read did.
    #[error("the message ended while reading {what}")]
    Truncated {
        /// Which field.
        what: &'static str,
    },
    /// A discriminant this build does not define.
    #[error("{what}: {value} is not a kind this build defines")]
    Unrecognised {
        /// Which enum.
        what: &'static str,
        /// The byte on the wire.
        value: u8,
    },
    /// A length or count larger than what is left to read it from.
    ///
    /// Checked *before* anything is reserved: the other side is untrusted, and a count is a claim.
    #[error("{what}: {stated} is more than the {remaining} bytes that remain")]
    Overlong {
        /// Which field.
        what: &'static str,
        /// What was claimed.
        stated: u64,
        /// What is left.
        remaining: usize,
    },
    /// A text field that is not UTF-8.
    #[error("{what}: not UTF-8")]
    NotText {
        /// Which field.
        what: &'static str,
    },
}

/// Writes fixed-width fields into one buffer.
#[derive(Debug, Default)]
pub(crate) struct Writer {
    /// What has been written.
    out: Vec<u8>,
}

impl Writer {
    /// An empty message.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// One byte.
    pub(crate) fn u8(&mut self, value: u8) -> &mut Self {
        self.out.push(value);
        self
    }

    /// Eight bytes, big-endian.
    pub(crate) fn u64(&mut self, value: u64) -> &mut Self {
        self.out.extend_from_slice(&value.to_be_bytes());
        self
    }

    /// A count as the fixed-width number this format carries.
    ///
    /// `try_from` rather than `as`: on every platform this compiles for `usize` is at most 64
    /// bits, so the conversion cannot fail — and on a hypothetical wider one the fallback is a
    /// length the reader refuses rather than a number that is quietly wrong.
    pub(crate) fn count(&mut self, value: usize) -> &mut Self {
        self.u64(u64::try_from(value).unwrap_or(u64::MAX))
    }

    /// Eight bytes of a signed number, big-endian.
    pub(crate) fn i64(&mut self, value: i64) -> &mut Self {
        self.out.extend_from_slice(&value.to_be_bytes());
        self
    }

    /// A length and then the bytes.
    pub(crate) fn bytes(&mut self, value: &[u8]) -> &mut Self {
        self.count(value.len());
        self.out.extend_from_slice(value);
        self
    }

    /// A length and then the text.
    pub(crate) fn text(&mut self, value: &str) -> &mut Self {
        self.bytes(value.as_bytes())
    }

    /// A presence byte and, where there is one, the text.
    pub(crate) fn optional(&mut self, value: Option<&str>) -> &mut Self {
        match value {
            Some(text) => {
                self.u8(1);
                self.text(text)
            }
            None => self.u8(0),
        }
    }

    /// What was written.
    pub(crate) fn finish(self) -> Vec<u8> {
        self.out
    }
}

/// Reads fixed-width fields out of one buffer, refusing anything the buffer cannot hold.
#[derive(Debug)]
pub(crate) struct Reader<'a> {
    /// What is left to read.
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    /// A reader over a whole message.
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { rest: bytes }
    }

    /// One byte.
    pub(crate) fn u8(&mut self, what: &'static str) -> Result<u8, WireError> {
        let (first, rest) = self
            .rest
            .split_first()
            .ok_or(WireError::Truncated { what })?;
        self.rest = rest;
        Ok(*first)
    }

    /// Eight bytes, big-endian.
    pub(crate) fn u64(&mut self, what: &'static str) -> Result<u64, WireError> {
        let (head, rest) = self
            .rest
            .split_at_checked(8)
            .ok_or(WireError::Truncated { what })?;
        let bytes: [u8; 8] = head.try_into().map_err(|_| WireError::Truncated { what })?;
        self.rest = rest;
        Ok(u64::from_be_bytes(bytes))
    }

    /// Eight bytes of a signed number, big-endian.
    pub(crate) fn i64(&mut self, what: &'static str) -> Result<i64, WireError> {
        #[expect(
            clippy::cast_possible_wrap,
            reason = "the same eight bytes read as the signed number they were written from; \
                      `i64::from_be_bytes` of what `i64::to_be_bytes` wrote"
        )]
        let signed = self.u64(what)? as i64;
        Ok(signed)
    }

    /// A count, checked against what remains before anything is reserved from it.
    pub(crate) fn count(&mut self, what: &'static str) -> Result<usize, WireError> {
        let stated = self.u64(what)?;
        let count = usize::try_from(stated).map_err(|_| WireError::Overlong {
            what,
            stated,
            remaining: self.rest.len(),
        })?;
        Ok(count)
    }

    /// A length-prefixed run of bytes.
    pub(crate) fn bytes(&mut self, what: &'static str) -> Result<&'a [u8], WireError> {
        let stated = self.u64(what)?;
        let length = usize::try_from(stated)
            .ok()
            .filter(|length| *length <= self.rest.len());
        let Some(length) = length else {
            return Err(WireError::Overlong {
                what,
                stated,
                remaining: self.rest.len(),
            });
        };
        let (head, rest) = self
            .rest
            .split_at_checked(length)
            .ok_or(WireError::Truncated { what })?;
        self.rest = rest;
        Ok(head)
    }

    /// A length-prefixed run of text.
    pub(crate) fn text(&mut self, what: &'static str) -> Result<String, WireError> {
        let bytes = self.bytes(what)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| WireError::NotText { what })
    }

    /// A presence byte and, where there is one, the text.
    pub(crate) fn optional(&mut self, what: &'static str) -> Result<Option<String>, WireError> {
        match self.u8(what)? {
            0 => Ok(None),
            1 => Ok(Some(self.text(what)?)),
            value => Err(WireError::Unrecognised { what, value }),
        }
    }

    /// How much is left, so that a count can be sanity-checked against it.
    pub(crate) fn remaining(&self) -> usize {
        self.rest.len()
    }
}

/// Encodes one question.
pub(crate) fn encode_query(query: &Query) -> Vec<u8> {
    use query_kind as k;

    let mut writer = Writer::new();
    match query {
        Query::PageCount => {
            writer.u8(k::PAGE_COUNT);
        }
        Query::ExtractPage { page } => {
            writer.u8(k::EXTRACT_PAGE).count(*page);
        }
        Query::RenderPage { page, dpi } => {
            writer.u8(k::RENDER_PAGE).count(*page).u64(u64::from(*dpi));
        }
        Query::ExtractImages { page } => {
            writer.u8(k::EXTRACT_IMAGES).count(*page);
        }
        Query::PageText { page } => {
            writer.u8(k::PAGE_TEXT).count(*page);
        }
        Query::AttachmentInventory => {
            writer.u8(k::ATTACHMENT_INVENTORY);
        }
        Query::ExtractAttachment { name } => {
            writer.u8(k::EXTRACT_ATTACHMENT).text(name);
        }
        Query::Information => {
            writer.u8(k::INFORMATION);
        }
        Query::MetadataStream => {
            writer.u8(k::METADATA_STREAM);
        }
        Query::Outline => {
            writer.u8(k::OUTLINE);
        }
    }
    writer.finish()
}

/// Decodes one question.
pub(crate) fn decode_query(bytes: &[u8]) -> Result<Query, WireError> {
    use query_kind as k;

    let mut reader = Reader::new(bytes);
    let kind = reader.u8("a query's kind")?;
    let query = match kind {
        k::PAGE_COUNT => Query::PageCount,
        k::EXTRACT_PAGE => Query::ExtractPage {
            page: reader.count("a page ordinal")?,
        },
        k::RENDER_PAGE => Query::RenderPage {
            page: reader.count("a page ordinal")?,
            dpi: u32::try_from(reader.u64("a resolution")?).map_err(|_| WireError::Overlong {
                what: "a resolution",
                stated: 0,
                remaining: reader.remaining(),
            })?,
        },
        k::EXTRACT_IMAGES => Query::ExtractImages {
            page: reader.count("a page ordinal")?,
        },
        k::PAGE_TEXT => Query::PageText {
            page: reader.count("a page ordinal")?,
        },
        k::ATTACHMENT_INVENTORY => Query::AttachmentInventory,
        k::EXTRACT_ATTACHMENT => Query::ExtractAttachment {
            name: reader.text("an attachment's name")?,
        },
        k::INFORMATION => Query::Information,
        k::METADATA_STREAM => Query::MetadataStream,
        k::OUTLINE => Query::Outline,
        value => {
            return Err(WireError::Unrecognised {
                what: "a query's kind",
                value,
            });
        }
    };
    Ok(query)
}

/// Encodes one answer.
pub(crate) fn encode_answer(answer: &Answer) -> Vec<u8> {
    use answer_kind as k;

    let mut writer = Writer::new();
    match answer {
        Answer::Count(count) => {
            writer.u8(k::COUNT).count(*count);
        }
        Answer::Bytes(bytes) => {
            writer.u8(k::BYTES).bytes(bytes);
        }
        Answer::Files(files) => {
            writer.u8(k::FILES).count(files.len());
            for (name, bytes) in files {
                writer.text(name).bytes(bytes);
            }
        }
        Answer::Attachments(entries) => {
            writer.u8(k::ATTACHMENTS).count(entries.len());
            for entry in entries {
                encode_attachment(&mut writer, entry);
            }
        }
        Answer::Absent => {
            writer.u8(k::ABSENT);
        }
    }
    writer.finish()
}

/// Decodes one answer.
pub(crate) fn decode_answer(bytes: &[u8]) -> Result<Answer, WireError> {
    use answer_kind as k;

    let mut reader = Reader::new(bytes);
    let kind = reader.u8("an answer's kind")?;
    let answer = match kind {
        k::COUNT => Answer::Count(reader.count("a count")?),
        k::BYTES => Answer::Bytes(reader.bytes("a file's bytes")?.to_vec()),
        k::FILES => {
            let count = reader.count("a file count")?;
            // A count on the wire is a claim, and the claim's *reservation* is a separate cost
            // from its length: the smallest entry this loop reads is seventeen bytes, so a count
            // past what remains is refused before a map is grown for it.
            if count > reader.remaining() {
                return Err(WireError::Overlong {
                    what: "a file count",
                    stated: u64::try_from(count).unwrap_or(u64::MAX),
                    remaining: reader.remaining(),
                });
            }
            let mut files = BTreeMap::new();
            for _ in 0..count {
                let name = reader.text("a file's name")?;
                let bytes = reader.bytes("a file's bytes")?.to_vec();
                files.insert(name, bytes);
            }
            Answer::Files(files)
        }
        k::ATTACHMENTS => {
            let count = reader.count("an attachment count")?;
            if count > reader.remaining() {
                return Err(WireError::Overlong {
                    what: "an attachment count",
                    stated: u64::try_from(count).unwrap_or(u64::MAX),
                    remaining: reader.remaining(),
                });
            }
            let mut entries = Vec::new();
            for _ in 0..count {
                entries.push(decode_attachment(&mut reader)?);
            }
            Answer::Attachments(entries)
        }
        k::ABSENT => Answer::Absent,
        value => {
            return Err(WireError::Unrecognised {
                what: "an answer's kind",
                value,
            });
        }
    };
    Ok(answer)
}

/// One §7.11.4 inventory entry, every field of Tables 43, 45 and 46 the reader records.
///
/// Every field is named, so a field added to [`AttachmentEntry`] fails to compile here rather than
/// crossing as nothing — the property this module's header states, for a type that has no arms.
fn encode_attachment(writer: &mut Writer, entry: &AttachmentEntry) {
    let AttachmentEntry {
        source,
        name,
        file_name,
        description,
        media_type,
        size,
        created,
        modified,
        relationship,
        page,
    } = entry;
    writer.count(*source).text(name);
    writer.optional(file_name.as_deref());
    writer.optional(description.as_deref());
    writer.optional(media_type.as_deref());
    match size {
        Some(size) => writer.u8(1).i64(*size),
        None => writer.u8(0),
    };
    writer.optional(created.as_deref());
    writer.optional(modified.as_deref());
    writer.text(relationship);
    match page {
        Some(page) => writer.u8(1).count(*page),
        None => writer.u8(0),
    };
}

/// One inventory entry, read back.
fn decode_attachment(reader: &mut Reader<'_>) -> Result<AttachmentEntry, WireError> {
    let source = reader.count("an attachment's source")?;
    let name = reader.text("an attachment's name")?;
    let file_name = reader.optional("an attachment's file name")?;
    let description = reader.optional("an attachment's description")?;
    let media_type = reader.optional("an attachment's media type")?;
    let size = match reader.u8("an attachment's stated size")? {
        0 => None,
        1 => Some(reader.i64("an attachment's stated size")?),
        value => {
            return Err(WireError::Unrecognised {
                what: "an attachment's stated size",
                value,
            });
        }
    };
    let created = reader.optional("an attachment's creation date")?;
    let modified = reader.optional("an attachment's modification date")?;
    let relationship = reader.text("an attachment's relationship")?;
    let page = match reader.u8("an attachment's page")? {
        0 => None,
        1 => Some(reader.count("an attachment's page")?),
        value => {
            return Err(WireError::Unrecognised {
                what: "an attachment's page",
                value,
            });
        }
    };
    Ok(AttachmentEntry {
        source,
        name,
        file_name,
        description,
        media_type,
        size,
        created,
        modified,
        relationship,
        page,
    })
}

/// Encodes a refusal: which of the four it is, and its sentence.
pub(crate) fn encode_refusal(error: &WorkerError) -> Vec<u8> {
    use refusal_kind as k;

    let mut writer = Writer::new();
    match error {
        WorkerError::PasswordRequired(detail) => {
            writer.u8(k::PASSWORD_REQUIRED).text(detail);
        }
        WorkerError::Declined { subject, detail } => {
            writer.u8(k::DECLINED).text(subject).text(detail);
        }
        WorkerError::NotPresent(detail) => {
            writer.u8(k::NOT_PRESENT).text(detail);
        }
        // A worker cannot produce either of the last two — a mismatch is what the *broker* says
        // about an answer's shape, and a transport failure is what the broker says about the
        // worker being gone — so both cross as what they are from the worker's side: a refusal
        // in its own words.
        WorkerError::Refused(detail) => {
            writer.u8(k::REFUSED).text(detail);
        }
        other @ (WorkerError::Mismatched { .. } | WorkerError::Transport(_)) => {
            writer.u8(k::REFUSED).text(&other.to_string());
        }
    }
    writer.finish()
}

/// Decodes a refusal.
pub(crate) fn decode_refusal(bytes: &[u8]) -> Result<WorkerError, WireError> {
    use refusal_kind as k;

    let mut reader = Reader::new(bytes);
    let kind = reader.u8("a refusal's kind")?;
    let error = match kind {
        k::PASSWORD_REQUIRED => WorkerError::PasswordRequired(reader.text("a refusal")?),
        k::REFUSED => WorkerError::Refused(reader.text("a refusal")?),
        k::DECLINED => WorkerError::Declined {
            subject: reader.text("what was declined")?,
            detail: reader.text("why it was declined")?,
        },
        k::NOT_PRESENT => WorkerError::NotPresent(reader.text("a refusal")?),
        value => {
            return Err(WireError::Unrecognised {
                what: "a refusal's kind",
                value,
            });
        }
    };
    Ok(error)
}

/// Encodes the one frame that opens a document: the ceilings, the policy, the password, the bytes.
///
/// **The document is the one thing that may not cross as bytes**, where the platform can pass a
/// descriptor: `on_disk` says its length and the descriptor rides beside the frame's header
/// (ADR 0812), so the broker holds the file and the worker holds a descriptor it cannot have
/// asked for.
pub(crate) fn encode_open(
    policy: pdf_transform::Policy,
    budget: &pdf_transform::Budget,
    password: Option<&str>,
    document: Document<'_>,
) -> Vec<u8> {
    let mut writer = Writer::new();
    writer.u8(level_code(policy.restrictions));
    writer.count(budget.limits.max_depth);
    writer.count(budget.limits.max_array_len);
    writer.count(budget.limits.max_dict_len);
    writer.count(budget.limits.max_string_len);
    writer.count(budget.limits.max_stream_len);
    writer.u64(budget.max_pixels);
    writer.optional(password);
    match document {
        Document::Bytes(bytes) => {
            writer.u8(document_kind::BYTES).bytes(bytes);
        }
        Document::OnDisk { length } => {
            writer.u8(document_kind::ON_DISK).u64(length);
        }
    }
    writer.finish()
}

/// How the document crosses in an open frame.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Document<'a> {
    /// Whole, in the frame.
    Bytes(&'a [u8]),
    /// As an open file beside the frame, at this length.
    OnDisk {
        /// What the host says the file is, since the worker may not ask (`statx` is not on the
        /// allow-list, ADR 0812).
        length: u64,
    },
}

/// What an open frame said.
#[derive(Debug)]
pub(crate) struct Opening {
    /// The host's decision about the document's assertions over its reader.
    pub(crate) policy: pdf_transform::Policy,
    /// The ceilings.
    pub(crate) budget: pdf_transform::Budget,
    /// §7.6.4.1's password, where the host supplied one.
    pub(crate) password: Option<String>,
    /// The document's bytes, or the length of the file whose descriptor arrived beside the frame.
    pub(crate) document: Held,
}

/// The document as the worker received it.
#[derive(Debug)]
pub(crate) enum Held {
    /// Whole, out of the frame.
    Bytes(Vec<u8>),
    /// A descriptor arrived, and this is what the host said its file is.
    OnDisk {
        /// The stated length.
        length: u64,
    },
}

/// Decodes the open frame.
pub(crate) fn decode_open(bytes: &[u8]) -> Result<Opening, WireError> {
    let mut reader = Reader::new(bytes);
    let restrictions = level_of(reader.u8("a restriction level")?)?;
    let budget = pdf_transform::Budget {
        limits: pdf_syntax::Limits {
            max_depth: reader.count("a depth ceiling")?,
            max_array_len: reader.count("an array ceiling")?,
            max_dict_len: reader.count("a dictionary ceiling")?,
            max_string_len: reader.count("a string ceiling")?,
            max_stream_len: reader.count("a stream ceiling")?,
        },
        max_pixels: reader.u64("a pixel ceiling")?,
    };
    let password = reader.optional("a password")?;
    let document = match reader.u8("how the document crosses")? {
        document_kind::BYTES => Held::Bytes(reader.bytes("the document")?.to_vec()),
        document_kind::ON_DISK => Held::OnDisk {
            length: reader.u64("the document's length")?,
        },
        value => {
            return Err(WireError::Unrecognised {
                what: "how the document crosses",
                value,
            });
        }
    };
    Ok(Opening {
        policy: pdf_transform::Policy { restrictions },
        budget,
        password,
        document,
    })
}

/// `CLAUDE.md` principle 3's four levels, as one byte.
///
/// Every arm named, so a fifth level added to `pdf_model::restriction::Level` fails to compile
/// here rather than crossing as one of the four.
fn level_code(level: pdf_model::restriction::Level) -> u8 {
    use pdf_model::restriction::Level;

    match level {
        Level::Off => 0,
        Level::On => 1,
        Level::Ask => 2,
        Level::Warn => 3,
    }
}

/// And back.
fn level_of(code: u8) -> Result<pdf_model::restriction::Level, WireError> {
    use pdf_model::restriction::Level;

    match code {
        0 => Ok(Level::Off),
        1 => Ok(Level::On),
        2 => Ok(Level::Ask),
        3 => Ok(Level::Warn),
        value => Err(WireError::Unrecognised {
            what: "a restriction level",
            value,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use pdf_transform::attachments::AttachmentEntry;

    use super::{
        Answer, Held, Query, WorkerError, decode_answer, decode_open, decode_query, decode_refusal,
        encode_answer, encode_open, encode_query, encode_refusal,
    };

    /// Every question this build defines, out and back.
    ///
    /// The population is written out rather than derived, and the assertion at the end is what
    /// keeps it honest: a variant added to `Query` and not added here is a question the confined
    /// transport would carry untested.
    #[test]
    fn every_query_round_trips() {
        let queries = [
            Query::PageCount,
            Query::ExtractPage { page: 7 },
            Query::RenderPage { page: 3, dpi: 300 },
            Query::ExtractImages { page: 35 },
            Query::PageText { page: 1 },
            Query::AttachmentInventory,
            Query::ExtractAttachment {
                name: "ISO_32000-2:2020".to_owned(),
            },
            Query::Information,
            Query::MetadataStream,
            Query::Outline,
        ];
        for query in &queries {
            let back = decode_query(&encode_query(query)).expect("a query this build wrote");
            assert_eq!(&back, query);
        }
    }

    #[test]
    fn every_answer_round_trips() {
        let mut files = BTreeMap::new();
        files.insert("01.png".to_owned(), vec![1, 2, 3]);
        files.insert("02.jpg".to_owned(), vec![4, 5]);
        let answers = [
            Answer::Count(72),
            Answer::Bytes(b"%PDF-2.0".to_vec()),
            Answer::Files(files),
            Answer::Attachments(vec![AttachmentEntry {
                source: 0,
                name: "a:b".to_owned(),
                file_name: Some("a:b.txt".to_owned()),
                description: None,
                media_type: Some("text/plain".to_owned()),
                size: Some(-1),
                created: Some("D:20260903000000Z".to_owned()),
                modified: None,
                relationship: "Unspecified".to_owned(),
                page: Some(4),
            }]),
            Answer::Absent,
        ];
        for answer in &answers {
            let back = decode_answer(&encode_answer(answer)).expect("an answer this build wrote");
            assert_eq!(&back, answer);
        }
    }

    /// The four refusals a face has to tell apart, each arriving as itself.
    #[test]
    fn every_refusal_keeps_its_kind_and_its_sentence() {
        let errors = [
            WorkerError::PasswordRequired("source 0: a password is required".to_owned()),
            WorkerError::Refused("source 0: does not open as a PDF".to_owned()),
            WorkerError::Declined {
                subject: "page 4".to_owned(),
                detail: "the rasteriser would not draw it".to_owned(),
            },
            WorkerError::NotPresent("page 900: the document has 72".to_owned()),
        ];
        for error in &errors {
            let back = decode_refusal(&encode_refusal(error)).expect("a refusal this build wrote");
            assert_eq!(back.to_string(), error.to_string());
            assert_eq!(
                std::mem::discriminant(&back),
                std::mem::discriminant(error),
                "{error} changed kind on the way across"
            );
        }
    }

    /// The ceilings and the policy cross whole, because a worker that guessed one would be
    /// working to a budget nobody set.
    #[test]
    fn an_open_frame_carries_every_ceiling_and_the_policy() {
        let policy = pdf_transform::Policy {
            restrictions: pdf_model::restriction::Level::Warn,
        };
        let budget = pdf_transform::Budget::default();
        let encoded = encode_open(
            policy,
            &budget,
            Some("open sesame"),
            super::Document::OnDisk { length: 19_200_000 },
        );
        let back = decode_open(&encoded).expect("an open frame this build wrote");
        assert_eq!(back.policy, policy);
        assert_eq!(back.budget, budget);
        assert_eq!(back.password.as_deref(), Some("open sesame"));
        assert!(matches!(back.document, Held::OnDisk { length: 19_200_000 }));
    }

    /// **No truncation and no flipped byte is ever a panic.** The confined side is the untrusted
    /// side of this boundary, so every message this build can write is cut at every length and
    /// altered at every position, and the decoder answers or refuses — it never unwinds.
    #[test]
    fn no_truncation_or_flip_of_a_message_makes_a_decoder_panic() {
        let mut files = BTreeMap::new();
        files.insert("01.png".to_owned(), vec![9; 40]);
        let messages: Vec<Vec<u8>> = vec![
            encode_query(&Query::ExtractAttachment {
                name: "a:b".to_owned(),
            }),
            encode_answer(&Answer::Files(files)),
            encode_answer(&Answer::Attachments(vec![AttachmentEntry {
                source: 0,
                name: "n".to_owned(),
                file_name: None,
                description: None,
                media_type: None,
                size: None,
                created: None,
                modified: None,
                relationship: "Data".to_owned(),
                page: None,
            }])),
            encode_refusal(&WorkerError::NotPresent("no".to_owned())),
            encode_open(
                pdf_transform::Policy::default(),
                &pdf_transform::Budget::default(),
                None,
                super::Document::Bytes(b"%PDF-2.0\n"),
            ),
        ];
        for message in &messages {
            for cut in 0..=message.len() {
                let short = &message[..cut];
                drop(decode_query(short));
                drop(decode_answer(short));
                drop(decode_refusal(short));
                drop(decode_open(short));
            }
            for at in 0..message.len() {
                let mut altered = message.clone();
                altered[at] ^= 0xff;
                drop(decode_query(&altered));
                drop(decode_answer(&altered));
                drop(decode_refusal(&altered));
                drop(decode_open(&altered));
            }
        }
    }
}
