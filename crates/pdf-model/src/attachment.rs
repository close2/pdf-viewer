//! ISO 32000-2 §7.11.4's embedded files, reached through §7.7.4's name dictionary.
//!
//! §7.11 is refused by architecture: a file specification "refers to a file external to the PDF
//! file" and this renderer has no filesystem to open one (principle 3, ADR 0014). §7.11.4 is the
//! one part of the family that asks for no filesystem at all — the bytes are *inside* the
//! document — and the clause says why they are there: embedding "makes the PDF file a
//! self-contained unit that can be stored or transmitted as a single entity".
//!
//! # What this reads and what it deliberately does not do
//!
//! It reads the *list*: each attachment's name, its description, its MIME subtype and Table 45's
//! parameters, and the stream object the bytes are in. It does **not** decode the bytes and it
//! does not write anything anywhere. Two reasons, and only the first is about principle 3:
//!
//! - Writing a file is the one thing the sandbox exists to prevent, and an attachment is
//!   arbitrary data a document controls. Extracting one is a person's decision, taken in a
//!   viewer that can ask.
//! - Decoding eagerly would inflate every attachment of every document that has one, on the
//!   path that opens a document. A 200 MB spreadsheet is a legal attachment.
//!
//! `Attachment::stream` is what a caller that *has* asked hands to `Document::decoded_stream_data`.
//!
//! # Where the two routes meet
//!
//! §7.11.4.1 gives an embedded file two homes — "[a]ny file specification dictionary in the
//! document may have an EF entry", and "[e]mbedded file streams may be associated with the
//! document as a whole through the `EmbeddedFiles` entry in the PDF file's name dictionary".
//! [`attachments`] walks the second, which is a *list* a panel shows; [`of_annotation`] reads
//! the first as §12.5.6.15's file attachment annotation states it, which is a file a person
//! reaches by clicking the paperclip on the page. Until the four-hundred-and-sixtieth session
//! nothing read the second route at all, so the corpus's one file attachment annotation — and
//! the six in ISO 32000-2's own PDF — embedded files no part of this program could reach
//! (ADR 0295).
//!
//! # §14.13's associated files are the same specifications, reached from elsewhere
//!
//! An associated file is a file specification carrying an `/AFRelationship`, named by an `/AF`
//! array on the object it belongs to — the catalog, a page, an annotation, an `XObject`, a
//! structure element, a `DPart`, or a marked-content sequence tagged `/AF`. [`associated`] reads
//! such an array from any of them, because the clause says the same sentence about every one:
//! "[t]he relationship that the associated files have to the … is supplied by the
//! `AFRelationship` key in each file specification dictionary". 7 corpus documents state one, 6
//! on the catalog and 30 on structure elements.
//!
//! 10 of the 974 corpus documents carry a `/Names /EmbeddedFiles` tree, holding 23 files
//! between them — mostly `application/mathml+xml` fragments from a LaTeX producer, one
//! `foo.txt`, and two attached PDFs. Two of the 23 refuse to decode, and correctly: they are in
//! documents whose `/Encrypt` this reader has no password for, and §7.6.6 puts the refusal on
//! the stream whose key is missing rather than on the file.

use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Object, Stream, tree};

/// Most attachments listed from one document.
///
/// A `/EmbeddedFiles` tree is a list a person scrolls; a document naming a million files is one
/// making a reader work.
const MAX_ATTACHMENTS: usize = 4096;

/// One embedded file: §7.11.3's file specification with §7.11.4's stream behind it.
#[derive(Debug, Clone)]
pub struct Attachment {
    /// The name the `/EmbeddedFiles` tree filed it under, or the file specification's own name.
    ///
    /// §7.11.4.1 makes the `/EmbeddedFiles` tree map name strings to file specifications, and
    /// says that before PDF 1.6 "it was necessary to identify document-level embedded files by
    /// the name string provided in the name dictionary". So the tree's key is a name and not
    /// necessarily a file name, which is why [`Self::file_name`] is a separate answer.
    ///
    /// The first half was a quotation — "shall map name strings to file specifications" — until
    /// the four-hundred-and-eighteenth session. Errata Collection 3 replaces the two bullets it
    /// came from outright (Issue #481, `/State` `Review` `Completed`), and the replacement says
    /// the same thing about this tree while adding §7.11.2's `/RF` beside `/EF` as the second
    /// place an embedded file stream may be specified. Nothing here changes: the tree is walked
    /// for its keys either way.
    pub name: String,
    /// Table 43's `/UF`, or `/F` where the file states no Unicode form.
    ///
    /// `/UF` first because Table 43 says so — it is "[a] Unicode text string that provides file
    /// specification", and the clause's own note is that `/F` is a byte string in a
    /// platform-dependent encoding. Neither is a path this program will open; what they are for
    /// here is telling a person which file this is.
    pub file_name: Option<String>,
    /// Table 43's `/Desc`, which since PDF 1.6 "should be used to provide a textual description
    /// of the embedded file, which can be displayed in the user interface".
    pub description: Option<String>,
    /// Table 44's `/Subtype`: the embedded file's MIME media type, as the file spells it.
    ///
    /// Kept as written. The clause makes it a *name* whose value is a MIME media type spelled so
    /// that a PDF name can carry it — `#2F` for the solidus, which the name lexer has already
    /// undone by the time this is read, leaving the media type.
    ///
    /// **Errata Collection 3 narrows what a producer may put here** (Issue #155, `/State`
    /// `Review` `Completed`), and this comment quoted the retired half until the
    /// four-hundred-and-eighteenth session: "the MIME media type names defined in Internet RFC
    /// 2046, with the provision that characters not permitted in names" becomes a subset of
    /// RFC 2046 section 2 — the top-level type and its description separated by a solidus, with no
    /// `;`, `=`, `#`, parameters or sub-parameters. Every requirement in it is on the writer,
    /// and a reader that answered only the types the amended clause permits would drop the
    /// value of every file written before it, so this still keeps what the document wrote.
    pub media_type: Option<String>,
    /// Table 45's `/Size`: "the size of the uncompressed embedded file, in bytes".
    ///
    /// The document's claim rather than a measurement — nothing here inflates the stream to
    /// check it, and a caller that decodes the bytes learns the truth.
    pub size: Option<i64>,
    /// Table 45's `/CreationDate` and `/ModDate`, as the §7.9.4 date strings the file wrote.
    ///
    /// The file's own bytes, with [`Attachment::created_date`] and [`Attachment::modified_date`]
    /// beside them for the parse — see [`crate::signature::Signature::signed_at`] for why both.
    pub created: Option<String>,
    /// The modification date, likewise.
    pub modified: Option<String>,
    /// Table 45's `/CheckSum`: "a 16-byte string that is the checksum of the bytes of the
    /// uncompressed embedded file", by MD5.
    ///
    /// Carried rather than checked *here*, because checking means inflating: the clause's
    /// subject is "the bytes of the uncompressed embedded file", so the question cannot be asked
    /// until somebody has decoded the stream. [`Self::checksum_matches`] is what asks it, and
    /// the one caller that has the bytes anyway is extraction.
    ///
    /// The clause is explicit about what an answer is worth: "[t]his is strictly a checksum, and
    /// is not used for security purposes."
    pub checksum: Option<Vec<u8>>,
    /// Table 43's `/AFRelationship`, **default `Unspecified`**, which is §14.13's whole point.
    pub relationship: Relationship,
    /// The stream the bytes are in, for a caller that has decided to extract them.
    pub stream: Arc<Stream>,
}

impl Attachment {
    /// Whether the decoded bytes are the ones Table 45's `/CheckSum` describes.
    ///
    /// ISO 32000-2 §7.11.4.1, Table 45:
    ///
    /// > A 16-byte string that is the checksum of the bytes of the uncompressed embedded file.
    /// > The checksum shall be calculated by applying the standard MD5 message-digest algorithm
    /// > (defined in Internet RFC 1321 ) to the bytes of the embedded file stream.
    ///
    /// `None` where the file states none, which is not a failure and is most of them: this is an
    /// *optional* entry, and a document that omits it has said nothing to disagree with.
    ///
    /// **A mismatch is worth reporting and is not worth refusing on.** The clause says in the
    /// same paragraph that it "is strictly a checksum, and is not used for security purposes",
    /// so a file whose digest differs is a file whose producer made a mistake, and handing over
    /// the bytes with a sentence beside them says more than withholding them.
    ///
    /// A `/CheckSum` that is not sixteen bytes is not what the clause describes, and answers
    /// `Some(false)` rather than `None`: the file stated a checksum and stated it wrongly, which
    /// is a different thing from stating none.
    #[must_use]
    pub fn checksum_matches(&self, bytes: &[u8]) -> Option<bool> {
        checksum_matches(self.checksum.as_deref(), bytes)
    }

    /// `/CreationDate` parsed as §7.9.4's date, where the producer wrote a conforming one.
    #[must_use]
    pub fn created_date(&self) -> Option<pdf_syntax::Date> {
        pdf_syntax::Date::parse(self.created.as_deref()?)
    }

    /// `/ModDate` parsed as §7.9.4's date, likewise.
    #[must_use]
    pub fn modified_date(&self) -> Option<pdf_syntax::Date> {
        pdf_syntax::Date::parse(self.modified.as_deref()?)
    }
}

/// Table 45's `/CheckSum` against bytes somebody has decoded, without an [`Attachment`] beside it.
///
/// ISO 32000-2 §7.11.4.1, Table 45:
///
/// > A 16-byte string that is the checksum of the bytes of the uncompressed embedded file. The
/// > checksum shall be calculated by applying the standard MD5 message-digest algorithm (defined
/// > in Internet RFC 1321 ) to the bytes of the embedded file stream.
///
/// [`Attachment::checksum_matches`] is this with the checksum taken from the file specification,
/// and it is the ordinary caller. **This spelling exists because a host may hold the two halves
/// apart**: `viewer_confined::Attachment` lists an embedded file without its stream, and the bytes
/// arrive later in `viewer_core::Event::Extracted` — so the question is asked with a checksum from
/// one message and a payload from another, and a rule with no way to ask it is `doc/todo/01`'s
/// fifth sweep in miniature.
///
/// `None` where the file states no checksum, which is not a failure and is most of them.
/// `Some(false)` for one that is not sixteen bytes: the file stated a checksum and stated it
/// wrongly, which is a different thing from stating none.
#[must_use]
pub fn checksum_matches(stated: Option<&[u8]>, bytes: &[u8]) -> Option<bool> {
    let stated = stated?;
    let digest = <md5::Md5 as md5::Digest>::digest(bytes);
    Some(stated == digest.as_slice())
}

/// Table 43's `/AFRelationship`: what an associated file is *to* the object that names it.
///
/// The clause is careful about what this is for, and it is not processing: "[t]he value of
/// `AFRelationship` does not explicitly provide any processing instructions for a PDF processor.
/// It is provided for information and semantic purposes for those processors that are able to
/// use such additional information."
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Relationship {
    /// `Source`: "the original source material for the associated content".
    Source,
    /// `Data`: "information used to derive a visual presentation — such as for a table or a
    /// graph".
    Data,
    /// `Alternative`: "an alternative representation of content, for example audio".
    Alternative,
    /// `Supplement`: "a supplemental representation of the original source or data that may be
    /// more easily consumable (e.g., A `MathML` version of an equation)". The corpus's commonest.
    Supplement,
    /// `EncryptedPayload`: an encrypted payload document, which §7.6.7's unencrypted wrapper
    /// names and which this reader has no cryptographic filter for.
    EncryptedPayload,
    /// `FormData`: "the data associated with the `AcroForm` … of this PDF".
    FormData,
    /// `Schema`: "a schema definition for the associated object".
    Schema,
    /// `Unspecified`, the default, "used when the relationship is not known or cannot be
    /// described using one of the other values".
    #[default]
    Unspecified,
    /// A second-class name a producer registered for a relationship the table does not define,
    /// which is what Annex E asks it to do rather than reusing `Unspecified`.
    Other(String),
}

impl Relationship {
    /// Reads Table 43's name, defaulting as the table states.
    fn read(document: &Document, specification: &Dictionary) -> Self {
        let stated = document.get_key(specification, "AFRelationship");
        let Some(name) = stated.as_name() else {
            return Self::Unspecified;
        };
        match name.as_bytes() {
            b"Source" => Self::Source,
            b"Data" => Self::Data,
            b"Alternative" => Self::Alternative,
            b"Supplement" => Self::Supplement,
            b"EncryptedPayload" => Self::EncryptedPayload,
            b"FormData" => Self::FormData,
            b"Schema" => Self::Schema,
            b"Unspecified" => Self::Unspecified,
            other => Self::Other(String::from_utf8_lossy(other).into_owned()),
        }
    }
}

/// §14.13's `/AF` array on any object that may carry one.
///
/// One function for all seven places the clause lists, because it states the same sentence about
/// each of them and the entry has the same shape in every one: an array of file specification
/// dictionaries. The name each attachment gets is its own `/UF` or `/F`, since an `/AF` array —
/// unlike §7.7.4's tree — files nothing under a key.
///
/// Empty where the object states no `/AF`, and a specification with no `/EF` is skipped: §14.13.2
/// permits an external associated file — "[b]oth types are allowed for associated files but the
/// embedded form is recommended" — and an external one is §7.11.1's refusal, which this program
/// has no filesystem to lift.
#[must_use]
pub fn associated(document: &Document, dict: &Dictionary) -> Vec<Attachment> {
    associated_under(document, dict, "AF")
}

/// §14.13.5's `/AF` on a *marked-content section*, whose property list names the array differently.
///
/// **The one `/AF` site whose key is not `AF`, and it took an erratum to say so.** §14.13.5's 2020
/// sentence never named the property list's key at all — it said only that the property list
/// "shall specify an array of file specification dictionaries", and §14.13.10's EXAMPLE 3 writes
/// `/AF /NamedAF BDC` without showing what `/NamedAF` resolves to. So `AF` was an inference from
/// the tag operand, and it is the inference this tree made.
///
/// Errata Collection 3 states the key: Issue #374, `/State` `Review` `Completed`, whose caret puts
/// "a dictionary with an MCAF entry defining" in front of that sentence and makes the following
/// one read "[t]he named resource in the Property List … shall specify this dictionary", against
/// "Table 409a - Property list entries for associated files". `doc/md/` has neither the caret nor
/// the table (ADR 0252, ADR 0253), so `MCAF` cannot be verified against this project's copy of the
/// standard and is taken from the annotation itself.
///
/// Both keys are read, `MCAF` first. That is not indecision: the erratum names `MCAF` and no text
/// available here says `AF` is now wrong there, so a file written to either reading is understood
/// and none is silently dropped — which is what the previous behaviour did to a conforming PDF 2.0
/// file. Table 409a is what would settle whether `AF` should be refused; when it can be read, this
/// is the function that narrows.
#[must_use]
pub fn associated_in_property_list(document: &Document, dict: &Dictionary) -> Vec<Attachment> {
    let mcaf = associated_under(document, dict, "MCAF");
    if mcaf.is_empty() {
        associated_under(document, dict, "AF")
    } else {
        mcaf
    }
}

/// The array under one key, read as §14.13's file specifications.
fn associated_under(document: &Document, dict: &Dictionary, key: &str) -> Vec<Attachment> {
    let array = document.get_key(dict, key);
    let Some(items) = array.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items.iter().take(MAX_ATTACHMENTS) {
        let resolved = document.resolve(item);
        let Some(specification) = resolved.as_dict() else {
            continue;
        };
        let name = crate::file_spec::FileSpec::from_dictionary(document, specification)
            .display_name()
            .unwrap_or_default();
        if let Some(attachment) = read(document, specification, name) {
            out.push(attachment);
        }
    }
    out
}

/// Every attachment §7.7.4's `/EmbeddedFiles` tree names, in the tree's own order.
///
/// Empty for a document with no name dictionary, no `/EmbeddedFiles` entry, or an entry that is
/// not a name tree — none of which is an error, and all of which is 964 of the 974 corpus
/// documents.
#[must_use]
pub fn attachments(document: &Document) -> Vec<Attachment> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    let names = document.get_key(&catalog, "Names");
    let Some(names) = names.as_dict() else {
        return Vec::new();
    };
    let embedded = document.get_key(names, "EmbeddedFiles");
    let Some(embedded) = embedded.as_dict() else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for (bytes, value) in tree::name_pairs(embedded, &|object| document.resolve(object)) {
        if out.len() >= MAX_ATTACHMENTS {
            break;
        }
        let resolved = document.resolve(&value);
        let Some(specification) = resolved.as_dict() else {
            continue;
        };
        if let Some(attachment) = read(document, specification, pdf_syntax::text_string(&bytes)) {
            out.push(attachment);
        }
    }
    out
}

/// The embedded file §12.5.6.15's file attachment annotation names, with its own description.
///
/// ISO 32000-2 §12.5.6.15:
///
/// > A file attachment annotation ( PDF 1.3 ) contains a reference to a file, which typically
/// > shall be embedded in the PDF file (see 7.11.4, "Embedded file streams").
///
/// Table 187 makes `/FS` **required**, and it is the second of the two homes §7.11.4.1 gives an
/// embedded file: "[a]ny file specification dictionary in the document may have an EF entry",
/// beside the document-wide `/EmbeddedFiles` tree [`attachments`] walks. A document may use
/// either, and the corpus's own file attachment annotation uses the one the tree cannot reach —
/// as do the six in ISO 32000-2's own PDF, counted by
/// `crates/pdf-model/examples/file_attachment_census.rs`.
///
/// `None` where the annotation states no `/FS`, or where the specification it names carries no
/// `/EF`: that is §7.11.1's file outside the document, which this program has no filesystem to
/// open (principle 3).
///
/// # The description is the annotation's, and that is the clause's one `shall`
///
/// ISO 32000-2 §12.5.6.15 again, and it is the only requirement the clause puts on a processor:
///
/// > The Contents entry of the annotation dictionary may specify descriptive text relating to
/// > the attached file. Interactive PDF processors shall use this entry rather than the optional
/// > Desc entry ( PDF 1.6 ) in the file specification dictionary (see "Table 43 -Entries in a
/// > file specification dictionary") identified by the annotation's FS entry.
///
/// So [`Attachment::description`] is Table 172's `/Contents` where the annotation states one.
/// Where it states none there is no "this entry" to use instead, and §7.11.4.1's `/Desc` keeps
/// its own clause's meaning — "a textual description of the embedded file, which can be
/// displayed in the user interface" — which is what [`read`] already put there.
///
/// The name is the specification's own — Table 43's `/UF` or `/F` — because an annotation files
/// its file under no key, where §7.7.4's tree does.
#[must_use]
pub fn of_annotation(document: &Document, annotation: &Dictionary) -> Option<Attachment> {
    let specification = document.get_key(annotation, "FS");
    let specification = specification.as_dict()?;
    let name = crate::file_spec::FileSpec::from_dictionary(document, specification)
        .display_name()
        .unwrap_or_default();
    let mut attachment = read(document, specification, name)?;
    if let Object::String(bytes) = document.get_key(annotation, "Contents") {
        let contents = pdf_syntax::text_string(&bytes);
        if !contents.is_empty() {
            attachment.description = Some(contents);
        }
    }
    Some(attachment)
}

/// §7.11.4.2's related files: the other files of a set the specification names one of.
///
/// > In some circumstances, a PDF file can refer to a group of related files, such as the set of
/// > five files that make up a DCS 1.0 colour-separated image. The file specification explicitly
/// > names only one of the files; the rest shall be identified by some systematic variation of
/// > that file name (such as by altering the extension).
///
/// `/RF` is a *dictionary keyed like `/EF`* — `/F`, `/UF` and the platform keys — whose values
/// are arrays of `2 × n` elements pairing a name with an embedded file stream. So this returns
/// the pairs behind whichever key the caller's own attachment came from, and it uses `/UF`
/// before `/F` for the reason [`read`] does.
///
/// Empty for a specification with no `/RF`, which is every one in the corpus: measured, and it
/// is why this is written from §7.11.4.2's own EXAMPLE.
#[must_use]
pub fn related(document: &Document, specification: &Dictionary) -> Vec<(String, Arc<Stream>)> {
    let related = document.get_key(specification, "RF");
    let Some(related) = related.as_dict() else {
        return Vec::new();
    };
    let array = ["UF", "F", "DOS", "Mac", "Unix"].iter().find_map(|key| {
        document
            .get_key(related, key)
            .as_array()
            .map(<[Object]>::to_vec)
    });
    let Some(array) = array else {
        return Vec::new();
    };
    array
        .chunks_exact(2)
        .take(MAX_ATTACHMENTS)
        .filter_map(|pair| {
            // "The first element of each pair shall be a string giving the name of one of the
            // related files; the second element shall be an embedded file stream holding the
            // file's contents." A pair whose halves are the wrong types is a file that has not
            // stated a related file, and is dropped rather than half-read.
            let Object::String(name) = document.resolve(pair.first()?) else {
                return None;
            };
            let stream = document.resolve(pair.get(1)?);
            let stream = stream.as_stream()?;
            Some((pdf_syntax::text_string(&name), Arc::clone(stream)))
        })
        .collect()
}

/// Reads one §7.11.3 file specification that carries an embedded file.
///
/// `None` where it carries none: `/EF` is what makes a file specification an *attachment*, and
/// a specification without one names a file outside the document, which §7.11.1 puts beyond
/// this program by architecture.
///
/// Table 43's `/EF` is a dictionary keyed the same way as `/F` and `/UF` — the clause says its
/// entries "shall be the same as those in the file specification dictionary" — so `/UF` is
/// preferred there too, for the same reason.
#[must_use]
pub fn read(document: &Document, specification: &Dictionary, name: String) -> Option<Attachment> {
    let files = document.get_key(specification, "EF");
    let files = files.as_dict()?;
    let stream = ["UF", "F", "DOS", "Mac", "Unix"]
        .into_iter()
        .find_map(|key| document.get_key(files, key).as_stream().cloned())?;

    let parameters = document.get_key(&stream.dict, "Params");
    let parameters = parameters.as_dict().cloned().unwrap_or_default();
    let text = |dict: &Dictionary, key: &str| match document.get_key(dict, key) {
        Object::String(bytes) => {
            let text = pdf_syntax::text_string(&bytes);
            (!text.is_empty()).then_some(text)
        }
        _ => None,
    };

    Some(Attachment {
        name,
        relationship: Relationship::read(document, specification),
        file_name: crate::file_spec::FileSpec::from_dictionary(document, specification)
            .display_name(),
        description: text(specification, "Desc"),
        media_type: document
            .get_key(&stream.dict, "Subtype")
            .as_name()
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()),
        size: document.get_key(&parameters, "Size").as_integer(),
        created: text(&parameters, "CreationDate"),
        modified: text(&parameters, "ModDate"),
        checksum: match document.get_key(&parameters, "CheckSum") {
            Object::String(bytes) => Some(bytes.to_vec()),
            _ => None,
        },
        stream,
    })
}

#[cfg(test)]
mod tests {
    use super::{attachments, of_annotation, related};
    use pdf_syntax::Document;

    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// §7.7.4's `/EmbeddedFiles` tree, with §7.11.4's stream and Table 45's parameters.
    ///
    /// The fixture is the shape §7.11.4.1 describes: a name tree mapping a name string to a file
    /// specification "that refer[s] to embedded file streams through their EF entries". The tree
    /// key and the file name differ on purpose, because the clause makes them different things —
    /// before PDF 1.6 the key was how a document-level attachment was identified at all.
    ///
    /// `/UF` is written as §7.9.2.2's UTF-16BE with its byte order marker, which is what a
    /// producer writes and what makes it "a Unicode text string"; the same characters written
    /// as literal UTF-8 bytes would be read as `PDFDocEncoding` and come back mojibake, which
    /// is the clause working rather than the reader failing.
    #[test]
    fn an_embedded_file_is_read_with_its_parameters() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles 4 0 R >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /Names [(the first one) 5 0 R] >>",
            "<< /Type /Filespec /F (data.csv) /UF <FEFF0064006F006E006E00E900650073002E006300730076> /Desc (last quarter) \
             /EF << /F 6 0 R >> >>",
            "<< /Type /EmbeddedFile /Subtype /text#2Fcsv /Length 5 \
             /Params << /Size 5 /ModDate (D:20260731120000Z) \
             /CheckSum <0123456789abcdef0123456789abcdef> >> >>\nstream\na,b,c\nendstream",
        ]);
        let attachments = attachments(&doc);
        let [attachment] = attachments.as_slice() else {
            panic!("one attachment, got {attachments:?}");
        };
        assert_eq!(attachment.name, "the first one", "the tree's key");
        assert_eq!(
            attachment.file_name.as_deref(),
            Some("données.csv"),
            "/UF wins over /F, which Table 43 makes the platform-encoded one"
        );
        assert_eq!(attachment.description.as_deref(), Some("last quarter"));
        assert_eq!(
            attachment.media_type.as_deref(),
            Some("text/csv"),
            "the name's #2F is the solidus §7.3.5 says it is"
        );
        assert_eq!(attachment.size, Some(5));
        assert_eq!(attachment.modified.as_deref(), Some("D:20260731120000Z"));
        assert_eq!(attachment.checksum.as_ref().map(Vec::len), Some(16));
        assert_eq!(
            doc.decoded_stream_data(&attachment.stream)
                .as_deref()
                .map(<[u8]>::to_vec),
            Some(b"a,b,c".to_vec()),
            "the bytes are reachable and were not decoded until now"
        );
    }

    /// Table 45's `/CheckSum` is checked against the bytes, and a file with none says nothing.
    ///
    /// The digest is the clause's: "the standard MD5 message-digest algorithm (described in
    /// Internet RFC 1321) … applied to the bytes of the embedded file stream". `a,b,c` hashes to
    /// `a44c56c8177e32d3613988f4dba7962e`, taken from `md5sum` rather than from this code, which
    /// is what makes it a check on the reader and not on itself.
    #[test]
    fn a_stated_checksum_is_answered_against_the_bytes() {
        let with = |checksum: &str| {
            document(&[
                "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles 4 0 R >> >>",
                "<< /Type /Pages /Count 0 /Kids [] >>",
                "<< /Unused true >>",
                "<< /Names [(one) 5 0 R] >>",
                "<< /Type /Filespec /F (data.csv) /EF << /F 6 0 R >> >>",
                &format!(
                    "<< /Type /EmbeddedFile /Length 5 /Params << {checksum} >> >>\n\
                     stream\na,b,c\nendstream"
                ),
            ])
        };
        let only = |doc: &Document| {
            let files = attachments(doc);
            let [file] = files.as_slice() else {
                panic!("one attachment, got {files:?}");
            };
            file.clone()
        };

        let right = with("/CheckSum <a44c56c8177e32d3613988f4dba7962e>");
        assert_eq!(only(&right).checksum_matches(b"a,b,c"), Some(true));
        let wrong = with("/CheckSum <0123456789abcdef0123456789abcdef>");
        assert_eq!(only(&wrong).checksum_matches(b"a,b,c"), Some(false));
        // An optional entry the file omits is not a disagreement.
        let silent = with("/Size 5");
        assert_eq!(only(&silent).checksum_matches(b"a,b,c"), None);
        // Stated, and not what the clause describes: sixteen bytes is part of the definition.
        let short = with("/CheckSum <0123>");
        assert_eq!(
            only(&short).checksum_matches(b"a,b,c"),
            Some(false),
            "a checksum that is not 16 bytes is a wrong statement, not an absent one"
        );
    }

    /// A file specification with no `/EF` names a file outside the document and is not listed.
    ///
    /// §7.11.1: the file "is considered external to the PDF file in either case", and without an
    /// `/EF` there is nothing inside the document to list — opening the named file is what this
    /// program has no filesystem for.
    #[test]
    fn a_specification_without_an_embedded_stream_is_not_an_attachment() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles 4 0 R >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /Names [(external) 5 0 R] >>",
            "<< /Type /Filespec /F (/etc/passwd) >>",
        ]);
        assert!(attachments(&doc).is_empty());
    }

    /// §14.13's `/AF`, read from the two places the corpus puts it.
    ///
    /// The clause lists seven objects that may carry the array and says the same sentence about
    /// each; the corpus states 6 on catalogs and 30 on structure elements, and the fixture is
    /// §14.13.10's own EXAMPLE 1 shape — a catalog `/AF` naming a file specification with an
    /// `/AFRelationship` and an `/EF`. The second half is what a structure element carries, which
    /// is the same function against a different dictionary.
    #[test]
    fn an_associated_file_carries_its_relationship() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AF [4 0 R] /StructTreeRoot 6 0 R >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /Type /Filespec /F (My Presentation.ppt) /AFRelationship /Source \
             /EF << /F 5 0 R >> >>",
            "<< /Type /EmbeddedFile /Subtype /application#2Fvnd.ms-powerpoint /Length 3 >>\nstream\nabc\nendstream",
            "<< /Type /StructTreeRoot /K 7 0 R >>",
            "<< /Type /StructElem /S /Formula /AF [8 0 R] >>",
            "<< /Type /Filespec /F (mathml-1.xml) /AFRelationship /Supplement \
             /EF << /F 9 0 R >> >>",
            "<< /Type /EmbeddedFile /Subtype /application#2Fmathml+xml /Length 3 >>\nstream\nxyz\nendstream",
        ]);
        let catalog = doc.catalog().expect("a catalog");
        let files = super::associated(&doc, &catalog);
        let [presentation] = files.as_slice() else {
            panic!("one associated file, got {files:?}");
        };
        assert_eq!(presentation.relationship, super::Relationship::Source);
        assert_eq!(presentation.name, "My Presentation.ppt");
        assert_eq!(
            presentation.media_type.as_deref(),
            Some("application/vnd.ms-powerpoint")
        );

        let element = doc.get(pdf_syntax::ObjectId {
            number: 7,
            generation: 0,
        });
        let element = element.as_dict().expect("the structure element");
        let supplements = super::associated(&doc, element);
        assert_eq!(
            supplements.first().map(|file| file.relationship.clone()),
            Some(super::Relationship::Supplement),
            "the commonest relationship in the corpus, and what a MathML equation is"
        );
    }

    /// An `/AFRelationship` outside Table 43's eight is kept, not flattened to `Unspecified`.
    ///
    /// The table's NOTE 2 is why: `Unspecified` "is to be used only when no other value correctly
    /// reflects the relationship", and "[s]econd-class names … should be used to represent other
    /// types of relationships" — so a producer that registered one has said something, and a
    /// reader that answered `Unspecified` would be throwing it away.
    #[test]
    fn a_second_class_relationship_keeps_its_name() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AF [4 0 R] >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /Type /Filespec /F (x.bin) /AFRelationship /ACME_Ledger /EF << /F 5 0 R >> >>",
            "<< /Type /EmbeddedFile /Length 1 >>\nstream\nx\nendstream",
        ]);
        let catalog = doc.catalog().expect("a catalog");
        assert_eq!(
            super::associated(&doc, &catalog)
                .first()
                .map(|file| file.relationship.clone()),
            Some(super::Relationship::Other("ACME_Ledger".to_owned()))
        );
    }

    /// §7.11.4.2's own EXAMPLE: a DCS 1.0 set of five files behind one specification.
    ///
    /// > 10 0 obj %File specification dictionary <</Type /Filespec /F (Sunset.eps) /UF
    /// > (Sunset.eps) /EF <</F 21 0 R /UF 41 0 R >> /RF <</UF 30 0 R>> %Related files array
    ///
    /// The shape worth checking is that `/RF` is keyed like `/EF` and its value is a flat array
    /// of alternating names and streams — the clause's `[ string 1 stream 1 string 2 stream 2 …
    /// string n stream n ]` — rather than an array of pairs.
    #[test]
    fn a_related_files_array_pairs_names_with_streams() {
        let stream = |name: &str| {
            format!(
                "<< /Type /EmbeddedFile /Length {} >>\nstream\n{name}\nendstream",
                name.len()
            )
        };
        let doc = document(&[
            "<< /Type /Catalog /Names << /EmbeddedFiles << /Names [(Sunset) 3 0 R] >> >> >>",
            "<< /Type /Pages /Kids [] /Count 0 >>",
            "<< /Type /Filespec /F (Sunset.eps) /UF (Sunset.eps) /EF << /UF 4 0 R >> \
             /RF << /UF [(Sunset.eps) 4 0 R (Sunset.C) 5 0 R (Sunset.M) 6 0 R \
             (Sunset.Y) 7 0 R (Sunset.K) 8 0 R] >> >>",
            &stream("eps"),
            &stream("cyan"),
            &stream("magenta"),
            &stream("yellow"),
            &stream("black"),
        ]);
        let files = attachments(&doc);
        let [file] = files.as_slice() else {
            panic!("one attachment, got {files:?}");
        };
        assert_eq!(file.file_name.as_deref(), Some("Sunset.eps"));

        let specification = doc
            .get(pdf_syntax::ObjectId {
                number: 3,
                generation: 0,
            })
            .as_dict()
            .cloned()
            .expect("a file specification");
        let set = related(&doc, &specification);
        assert_eq!(
            set.iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>(),
            ["Sunset.eps", "Sunset.C", "Sunset.M", "Sunset.Y", "Sunset.K"],
            "the whole DCS set, in the order the array pairs them"
        );
        assert_eq!(set.len(), 5);
    }

    /// One page carrying one file attachment annotation, and nothing in the name dictionary.
    ///
    /// `contents` is written into the annotation where it is `Some`, which is the only
    /// difference between the two documents §12.5.6.15's `shall` is read with.
    fn attached(contents: Option<&str>) -> Document {
        let annotation = format!(
            "<< /Type /Annot /Subtype /FileAttachment /Rect [10 10 30 34] /FS 5 0 R \
             /Name /Paperclip{} >>",
            contents.map_or_else(String::new, |text| format!(" /Contents ({text})"))
        );
        document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [4 0 R] >>",
            &annotation,
            "<< /Type /Filespec /UF (report.csv) /Desc (the specification's own words) \
             /EF << /F 6 0 R >> >>",
            "<< /Type /EmbeddedFile /Subtype /text#2Fcsv /Length 5 /Params << /Size 5 >> >>\n\
             stream\na,b,c\nendstream",
        ])
    }

    /// The annotation of the fixture above, which is object 4.
    fn annotation(doc: &Document) -> pdf_syntax::Dictionary {
        doc.get(pdf_syntax::ObjectId {
            number: 4,
            generation: 0,
        })
        .as_dict()
        .cloned()
        .expect("the annotation")
    }

    /// §12.5.6.15: the file an annotation names is read, though no name tree holds it.
    ///
    /// §7.11.4.1 gives an embedded file two homes and a document may use either. This fixture
    /// uses the one the `/EmbeddedFiles` tree does not reach, which is the corpus's own case:
    /// `annotation-fileattachment.pdf` states no name dictionary at all and attaches its file
    /// to a page (`crates/pdf-model/examples/file_attachment_census.rs`).
    #[test]
    fn a_file_an_annotation_names_is_read_though_the_name_tree_holds_none() {
        let doc = attached(Some("the sales figures"));
        assert!(
            attachments(&doc).is_empty(),
            "the document files nothing under /EmbeddedFiles"
        );
        let file = of_annotation(&doc, &annotation(&doc)).expect("the annotation's file");
        assert_eq!(
            file.name, "report.csv",
            "an annotation files its specification under no key, so the name is the file's own"
        );
        assert_eq!(file.media_type.as_deref(), Some("text/csv"));
        assert_eq!(
            doc.decoded_stream_data(&file.stream).as_deref(),
            Some(&b"a,b,c"[..]),
            "the bytes an extraction would hand over"
        );
    }

    /// §12.5.6.15's one `shall`, as a pair of documents differing only in `/Contents`.
    ///
    /// > The Contents entry of the annotation dictionary may specify descriptive text relating
    /// > to the attached file. Interactive PDF processors shall use this entry rather than the
    /// > optional Desc entry ( PDF 1.6 ) in the file specification dictionary … identified by
    /// > the annotation's FS entry.
    ///
    /// Both documents state the same `/Desc`, so the assertion is about which of two present
    /// texts is chosen rather than about a fallback firing. **No corpus document can rank this**:
    /// the one file attachment annotation in the 974 states a `/Contents` beside an *empty*
    /// `/Desc`, and so do all six in ISO 32000-2's own PDF, so a reader that preferred the wrong
    /// entry shows the same thing (trap 8).
    #[test]
    fn a_file_attachments_description_is_the_annotations_contents_and_not_the_specifications_desc()
    {
        let stated = attached(Some("the sales figures"));
        let with = of_annotation(&stated, &annotation(&stated)).expect("the annotation's file");
        assert_eq!(
            with.description.as_deref(),
            Some("the sales figures"),
            "Table 172's /Contents, which the clause puts ahead of Table 43's /Desc"
        );

        let silent = attached(None);
        let without = of_annotation(&silent, &annotation(&silent)).expect("the annotation's file");
        assert_eq!(
            without.description.as_deref(),
            Some("the specification's own words"),
            "with no /Contents there is no entry to use instead, so §7.11.4.1's /Desc keeps its \
             own clause's meaning"
        );
    }

    /// An annotation whose specification names a file *outside* the document attaches nothing.
    ///
    /// §7.11.1's external file is the refusal principle 3 makes architectural — this renderer
    /// has no filesystem — and `read` already draws that line at `/EF`. Asserted here because
    /// the caller is a click: a person who clicks a paperclip and gets a file that was never in
    /// the document would have been handed something invented.
    #[test]
    fn an_annotation_naming_an_external_file_attaches_nothing() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [4 0 R] >>",
            "<< /Type /Annot /Subtype /FileAttachment /Rect [10 10 30 34] /FS 5 0 R >>",
            "<< /Type /Filespec /F (elsewhere.csv) >>",
        ]);
        assert!(of_annotation(&doc, &annotation(&doc)).is_none());
    }
}
