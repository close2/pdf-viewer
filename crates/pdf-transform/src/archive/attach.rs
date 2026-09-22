//! `preserve` by attachment — the producer's own bytes kept as an embedded file.
//!
//! # What this is, and where it is available
//!
//! `doc/rfc/0007` section 4.6.1's second mechanism, beside [`super::preserve`]'s appended page.
//! The six targets differ about what may be *attached* rather than about what may be a page: ISO
//! 19005-2 section 6.8 and ISO 19005-4 section 6.9 require an embedded file to conform to a part
//! of ISO 19005, and Annex A lifts that for PDF/A-4f and Annex B for PDF/A-4e. So bytes that are
//! not a PDF can stay inside the archive at those two targets and nowhere else, which is why
//! [`super::config`] refuses the answer at the other four rather than quietly giving a page.
//!
//! # What is attached, and what is claimed about it
//!
//! Two sites, and the bytes are the producer's at both: the XMP packet
//! [`super::rewrite::Rewrite::FreshMetadataPacket`] replaces, and the XFA resource
//! [`super::rewrite::Rewrite::XfaRemoved`] takes out of the interactive form dictionary. Neither
//! is re-encoded, re-indented or re-parsed on the way in — `CLAUDE.md`'s provenance fence is what
//! makes this remedy `preserve` rather than authoring.
//!
//! **The media type is the standard's own, and it is a `shall` rather than a choice.** §14.13.2
//! states both halves of it for an embedded file used as an associated file:
//!
//! > The embedded file stream dictionary shall include a valid MIME type value for the Subtype
//! > key. If the MIME type is not known, the value " application/octet-stream " shall be used.
//!
//! The bytes here arrive from a site whose whole subject is that this program could not read them
//! — a packet that broke ISO 16684-1's grammar, a resource this program has no engine for — so
//! the type is not known, and the clause says what to write when it is not. [`MEDIA_TYPE`] is
//! that value. `doc/adr/1270`.
//!
//! # What the file says about the attachment
//!
//! §7.11.4's embedded file stream and §7.11.3's file specification are written by
//! [`pdf_model::attachment::filing`], which is the one writer of both in this tree (ADR 0814) and
//! the one writer of §7.9.6's `/Names` node (ADR 1211). What this module adds beside them is
//! Table 43's `/AFRelationship` and §14.13.3's catalog `/AF` — see [`RELATIONSHIP`] and
//! [`Attached::associated`].

use std::collections::BTreeSet;

use pdf_model::attachment::filing;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::decision::Because;
use super::prepare::Spare;

/// The media type every file this module attaches states.
///
/// §14.13.2's own value for the case this module is always in:
///
/// > If the MIME type is not known, the value " application/octet-stream " shall be used.
///
/// The bytes come from a site whose subject is that this program could not read them, so the
/// type is not known and the clause states what stands in its place. Nothing here is a choice.
pub(super) const MEDIA_TYPE: &str = "application/octet-stream";

/// The relationship every file this module attaches states.
///
/// §7.11.3's Table 43 defines the value, and the definition is the reason it is the right one:
///
/// > Source shall be used if this file specification is the original source material for the
/// > associated content.
///
/// What is attached is in every case the producer's original of something this conversion
/// replaced — the packet a fresh one stands in for, the XFA resource the `AcroForm` now stands
/// for — so `Source` is what the relationship is. Table 43's NOTE 2 is why `Unspecified` would be
/// wrong: "Unspecified is to be used only when no other value correctly reflects the
/// relationship."
pub(super) const RELATIONSHIP: &str = "Source";

/// The sentence a person is owed when content is kept as a file rather than lost.
///
/// **Deliberately not an apology**, for [`super::preserve::PRESERVED_AS_A_PAGE`]'s reason:
/// nothing is lost and nothing is invented, and what changes is *where* the content is and what a
/// reader has to do to get at it (`doc/rfc/0007` section 2.1). A reader of the archive meets the
/// packet as a file to open rather than as metadata a tool reads.
pub const PRESERVED_AS_AN_ATTACHMENT: &str = "this content is the document's own, kept as a file \
     embedded in the archive because the target would not hold it where it was";

/// The `xmpMM:History` parameters recording every file attached, where any were.
///
/// `doc/questions/A55`'s rule read one mechanism over: what an archive carries that its producer
/// did not put there belongs in the *file* rather than only in a report somebody may not have
/// kept. The sentence is [`PRESERVED_AS_AN_ATTACHMENT`] verbatim, followed by what was kept and
/// the name it is filed under.
pub(super) fn attached_history(rows: &[AttachedFile]) -> Option<String> {
    use std::fmt::Write as _;
    if rows.is_empty() {
        return None;
    }
    let mut out = format!("{PRESERVED_AS_AN_ATTACHMENT}: ");
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push_str("; ");
        }
        let _ = write!(out, "{}, filed as {}", row.description, row.name);
    }
    Some(out)
}

/// Why an attachment cannot be filed.
pub(super) const NO_SPARE_OBJECT: &str = "no unused object number could be found for the embedded \
     file this remedy attaches, so the objects §7.11.4 needs for it cannot be numbered";

/// Why a catalog this conversion cannot read stops an attachment.
pub(super) const NO_CATALOG: &str = "the attachment is filed in the name tree §7.7.4 puts in the \
     document catalog, and this document states no catalog this conversion can read";

/// Why a file specification that is not a dictionary stops an attachment.
///
/// §7.11.3 makes a file specification a dictionary and [`filing::file_specification`] writes one,
/// so nothing reaches this. It is a refusal rather than a comment claiming the case away, which
/// is what the rest of this verb does with an impossible shape.
pub(super) const NOT_A_SPECIFICATION: &str = "the file specification this remedy wrote for the \
     attachment is not a dictionary, which §7.11.3 makes it, so the attachment is refused rather \
     than filed under something no reader can follow";

/// Why a name the document already files something under stops an attachment.
pub(super) const NAME_ALREADY_FILED: &str = "the §7.9.6 name this remedy would file the \
     producer's bytes under is already a key in this document's EmbeddedFiles tree, and a tree \
     stating one key twice is not a tree. The name carries the object number the bytes came from, \
     so a document reaching this has filed an attachment under a name of exactly that shape";

/// One thing to keep as an embedded file.
pub(super) struct Keeping<'a> {
    /// The requirement whose refusal this answers.
    pub(super) site: &'static str,
    /// The §7.9.6 key and §7.11.2 file name, which are the same string.
    pub(super) name: String,
    /// What the bytes are, in the words the report and Table 43's `/Desc` both use.
    pub(super) description: String,
    /// The bytes, the producer's own.
    pub(super) bytes: &'a [u8],
}

/// One file this conversion attached, for the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedFile {
    /// The requirement whose refusal it answers.
    pub site: &'static str,
    /// The name it is filed under, in the `/EmbeddedFiles` tree and in `/F` and `/UF`.
    pub name: String,
    /// What it holds, in the same words `/Desc` states.
    pub description: String,
    /// How many bytes, which is what Table 45's `/Size` states.
    pub size: usize,
    /// The media type its stream states.
    pub media_type: &'static str,
    /// The relationship its specification states.
    pub relationship: &'static str,
}

impl AttachedFile {
    /// One attachment as JSON.
    pub(super) fn to_json(&self) -> crate::json::Value {
        crate::json::Value::Object(vec![
            (
                "requirement".to_owned(),
                crate::json::Value::text(self.site.to_owned()),
            ),
            (
                "name".to_owned(),
                crate::json::Value::text(self.name.clone()),
            ),
            (
                "description".to_owned(),
                crate::json::Value::text(self.description.clone()),
            ),
            ("size".to_owned(), crate::json::Value::count(self.size)),
            (
                "media_type".to_owned(),
                crate::json::Value::text(self.media_type.to_owned()),
            ),
            (
                "relationship".to_owned(),
                crate::json::Value::text(self.relationship.to_owned()),
            ),
        ])
    }
}

/// What attaching decided: the objects to add, where the tree goes, and what to report.
#[derive(Debug)]
pub(super) struct Attached {
    /// The objects this conversion adds, in the *source's* numbering.
    ///
    /// Two per attachment — §7.11.4's stream and §7.11.3's specification — and one `/Names` node
    /// for the whole tree, ready for [`super::prepare::Prepared::added`] to hand to the walk.
    pub(super) written: Vec<(ObjectId, Object)>,
    /// The `/EmbeddedFiles` name tree's new root node.
    pub(super) tree: ObjectId,
    /// The specifications §14.13.3's catalog `/AF` array gains.
    ///
    /// > One or more files may be associated with the PDF document as a whole by including a file
    /// > specification dictionary (7.11.3, "File specification dictionaries") for each file as one
    /// > of the members of the array value of the AF key in the document catalog (7.7.2, "Document
    /// > catalog dictionary").
    ///
    /// The attachment is about the document rather than about any page of it — a packet is the
    /// document's metadata, an XFA resource is the document's form — so the catalog is where
    /// §14.13.3 puts the association.
    pub(super) associated: Vec<ObjectId>,
    /// What was attached, for the report.
    pub(super) rows: Vec<AttachedFile>,
}

/// Files each of `keeps` as an embedded file, or the reason none can be.
///
/// **Nothing is attached for a document that asks for none**: the caller reaches here only where
/// the operator's configuration named an attaching `preserve` whose requirement this document
/// failed, which is `doc/adr/0947`'s first rule applied to a remedy.
pub(super) fn attach(
    document: &Document,
    catalog: Option<&Dictionary>,
    keeps: &[Keeping<'_>],
    spare: &mut Spare,
) -> Result<Attached, Because> {
    let Some(catalog) = catalog else {
        return Err(Because::NotBuiltYet(NO_CATALOG));
    };
    // The tree as the document states it now: every key the producer filed, kept so that the one
    // `/Names` node this writes holds them all. §7.9.6 permits the single-node shape outright —
    // "[i]f the root node has a Names entry, it shall be the only node in the tree" — and ADR
    // 1211 is why this tree has exactly one writer of it.
    let existing = filing::Tree::read(document, catalog, &|id| document.get(id));
    let modified = modification_date(document);
    let mut entries = existing.entries.clone();
    let mut written = Vec::new();
    let mut associated = Vec::new();
    let mut rows = Vec::new();
    let mut taken: BTreeSet<Vec<u8>> = existing
        .entries
        .iter()
        .map(|(key, _)| key.clone())
        .collect();
    for keep in keeps {
        let key = keep.name.clone().into_bytes();
        if !taken.insert(key.clone()) {
            return Err(Because::NotBuiltYet(NAME_ALREADY_FILED));
        }
        let stream = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
        let specification = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
        // §14.13.2's `should`, answered from the file rather than from a clock: an associated
        // file's stream "should contain a Params key whose value shall be a dictionary containing
        // at least a ModDate key whose value shall be the latest modification date of the source
        // file", and the source of these bytes is this document, whose own latest modification
        // date §14.3.3's Table 349 states. Table 45's `/CreationDate` is left out: it is "[t]he
        // date and time when the embedded file was created" and nothing in the document says when
        // a packet it happened to hold came into being.
        written.push((
            stream,
            filing::embedded_file_stream_dated(keep.bytes, Some(MEDIA_TYPE), None, modified),
        ));
        let Object::Dictionary(mut dict) =
            filing::file_specification(&key, stream, Some(&keep.description))
        else {
            return Err(Because::NotBuiltYet(NOT_A_SPECIFICATION));
        };
        // Table 43's `/AFRelationship`, which ISO 19005-4 section 6.9 requires of every embedded
        // file's specification. `filing.rs` does not write it, and rightly: nothing it is told
        // about a caller's file says what the relationship is. Here the clause's own definition
        // does — see [`RELATIONSHIP`].
        dict.insert(
            Name::new(&b"AFRelationship"[..]),
            Object::Name(Name::new(RELATIONSHIP.as_bytes())),
        );
        written.push((specification, Object::Dictionary(dict)));
        entries.push((key, Object::Reference(specification)));
        associated.push(specification);
        rows.push(AttachedFile {
            site: keep.site,
            name: keep.name.clone(),
            description: keep.description.clone(),
            size: keep.bytes.len(),
            media_type: MEDIA_TYPE,
            relationship: RELATIONSHIP,
        });
    }
    let tree = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    written.push((tree, filing::tree_root(entries)));
    Ok(Attached {
        written,
        tree,
        associated,
        rows,
    })
}

/// The document's own latest modification date, where §14.3.3's Table 349 states one.
///
/// What §14.13.2 asks an associated file's `/ModDate` to carry is "the latest modification date
/// of the source file", and for bytes taken out of this document the source file is this
/// document. Table 349's `/ModDate` is where it says so, in §7.9.4's grammar, which is the
/// grammar Table 45's entry is written in too — so nothing is converted and nothing is guessed.
///
/// `None` where the document states none or states one §7.9.4 does not admit: the entry is then
/// omitted, and §14.13.2's sentence about it is a `should`.
fn modification_date(document: &Document) -> Option<pdf_syntax::Date> {
    let Object::Dictionary(info) = document.get_key(document.trailer(), "Info") else {
        return None;
    };
    let value = document.get_key(&info, "ModDate");
    let text = value.as_string()?;
    pdf_syntax::Date::parse(std::str::from_utf8(text).ok()?)
}

/// The name one attachment is filed under: what it is, the object it came from, and an extension.
///
/// **The object number is what makes it unique**, which a name a converter chooses otherwise is
/// not: two metadata streams replaced in one document would file two attachments, and §7.9.6
/// compares keys "on a simple byte-by-byte basis". The extension is a convention rather than a
/// declaration — the media type is where a claim about the bytes would be, and [`MEDIA_TYPE`] is
/// what this converter has to say there.
pub(super) fn filed_as(stem: &str, at: ObjectId, extension: &str) -> String {
    format!("{stem}-{}-{}.{extension}", at.number, at.generation)
}

/// The XFA resource's bytes, as the interactive form dictionary states them.
///
/// §12.7.3's Table 224 makes the entry "[a] stream or array containing an XFA resource", and
/// Annex K says what the array is: "[a] packet is a pair of a string and stream. The string
/// contains the name of the XML element and the stream contains the complete text of this XML
/// element. Each packet represents a complete XML element, with the exception of the first and
/// last packet, which specify begin and end tags for the xdp:xdp element". So the resource is the
/// streams in the order the array states them, end to end, and nothing between them: the first
/// opens the document element and the last closes it.
///
/// `None` where the entry is neither shape or where a stream will not decode — there is then no
/// resource to keep, and the caller refuses by name rather than attaching part of one.
pub(super) fn xfa_resource(document: &Document, form: &Dictionary) -> Option<Vec<u8>> {
    let entry = document.get_key(form, "XFA");
    if let Some(stream) = entry.as_stream() {
        return document
            .decoded_stream_data(stream)
            .map(|bytes| bytes.to_vec());
    }
    let packets = entry.as_array()?;
    let mut out = Vec::new();
    for item in packets {
        let resolved = document.resolve(item);
        let Some(stream) = resolved.as_stream() else {
            continue;
        };
        out.extend_from_slice(&document.decoded_stream_data(stream)?);
    }
    (!out.is_empty()).then_some(out)
}
