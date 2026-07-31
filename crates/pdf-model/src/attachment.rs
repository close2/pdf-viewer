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
//! This
//! module reads the second and provides the reader for the first, which is what §12.5.6.15's
//! file attachment annotations need.
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
    /// §7.11.4.1 says the tree "shall map name strings to file specifications", and that before
    /// PDF 1.6 "it was necessary to identify document-level embedded files by the name string
    /// provided in the name dictionary". So the tree's key is a name and not necessarily a file
    /// name, which is why [`Self::file_name`] is a separate answer.
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
    /// Kept as written. The clause makes it a *name* whose value "shall conform to the MIME
    /// media type names defined in Internet RFC 2046, with the provision that characters not
    /// permitted in names shall use the 2-character hexadecimal code format" — so `#2F` for the
    /// solidus is already undone by the name lexer, and what is left is the media type.
    pub media_type: Option<String>,
    /// Table 45's `/Size`: "the size of the uncompressed embedded file, in bytes".
    ///
    /// The document's claim rather than a measurement — nothing here inflates the stream to
    /// check it, and a caller that decodes the bytes learns the truth.
    pub size: Option<i64>,
    /// Table 45's `/CreationDate` and `/ModDate`, as the §7.9.4 date strings the file wrote.
    pub created: Option<String>,
    /// The modification date, likewise.
    pub modified: Option<String>,
    /// Table 45's `/CheckSum`: "a 16-byte string that is the checksum of the bytes of the
    /// uncompressed embedded file", by MD5.
    ///
    /// Read and not verified: this tree has no MD5 for it — §7.6's algorithms use one, but
    /// checking this would mean inflating every attachment — and the clause is explicit that
    /// "[t]his is strictly a checksum, and is not used for security purposes".
    pub checksum: Option<Vec<u8>>,
    /// Table 43's `/AFRelationship`, **default `Unspecified`**, which is §14.13's whole point.
    pub relationship: Relationship,
    /// The stream the bytes are in, for a caller that has decided to extract them.
    pub stream: Arc<Stream>,
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
    let array = document.get_key(dict, "AF");
    let Some(items) = array.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items.iter().take(MAX_ATTACHMENTS) {
        let resolved = document.resolve(item);
        let Some(specification) = resolved.as_dict() else {
            continue;
        };
        let name = match document.get_key(specification, "UF") {
            Object::String(bytes) => pdf_syntax::text_string(&bytes),
            _ => match document.get_key(specification, "F") {
                Object::String(bytes) => pdf_syntax::text_string(&bytes),
                _ => String::new(),
            },
        };
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
        file_name: text(specification, "UF").or_else(|| text(specification, "F")),
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
    use super::attachments;
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
}
