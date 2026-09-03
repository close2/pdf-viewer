//! §7.11.4's embedded file *written*: the objects §7.5.6's update carries for one, and where in
//! the document they are filed.
//!
//! One writer for two consumers. `pdf-transform`'s `attachments --attach` and `--remove` built
//! these objects privately from the eight-hundred-and-seventieth session (ADRs 0802, 0803), and
//! the viewer's own edit log wanted the same three objects and the same tree rewrite in the
//! eight-hundred-and-eighty-fifth. The crate graph runs `pdf-transform → viewer-core` — that crate
//! takes `viewer_core::Secret` (ADR 0800) — so the shared writer cannot live in the transform, and
//! it lives here beside the reader it mirrors: [`super::read`] is what turns these objects back into
//! an [`super::Attachment`]. ADR 0814.
//!
//! # What is built, clause by clause
//!
//! - [`embedded_file_stream`] — §7.11.4's stream: Table 44's `/Type /EmbeddedFile` and `/Subtype`
//!   where the caller states a media type, Table 45's `/Params` with `/Size` and `/CheckSum` from
//!   the bytes, and the two dates only where the caller states one. **Unfiltered**: the bytes are
//!   the file's, and a compression this crate chose would be a second decision in a writer whose
//!   one job is to say what was attached.
//! - [`file_specification`] — §7.11.3's Table 43, indirect because the table requires it where
//!   `/EF` is present; `/F` and `/UF` both the filing name ("[t]he UF entry should be used in
//!   addition to the F entry"), `/EF` naming the one stream under both keys, `/Desc` where given.
//! - [`file_attachment_annotation`] — §12.5.6.15's Table 187: `/Subtype /FileAttachment`, the
//!   required `/FS`, and `/Name`, which "PDF writers should include" and which is therefore always
//!   written; Table 166's `/Rect` and `/P`; the description in `/Contents`, because the clause's
//!   one `shall` makes that the text a reader shows.
//! - [`Tree`] and [`point_holder_at_tree`] — §7.7.4's `/EmbeddedFiles` name tree, rewritten as one
//!   `/Names` node holding every entry in §7.9.6's order, and whichever object held the tree
//!   pointed at the new root. **The whole tree is rewritten as one node rather than one leaf
//!   edited in place, and that is a choice with a cost**: §7.9.6 permits it — "[i]f the root node
//!   has a Names entry, it shall be the only node in the tree" — and it makes the update the same
//!   objects whatever shape the producer chose; the cost is a document with thousands of embedded
//!   files paying for all of them in one array, which no corpus document has.
//! - [`other_homes`] — the condition on freeing what a removed entry reached: §7.11.4.1 gives an
//!   embedded file more than one home, and a stream the catalog's `/AF` or a page's annotation
//!   still reaches is not deleted by the tree letting go of it.
//!
//! Nothing here allocates an object number or writes a byte: every function takes the numbers
//! its caller chose and answers with objects for a replacement map, because the two consumers
//! allocate differently — the transform from the file's highest number, the viewer's
//! `ViewState` from a counter shared with the annotations it adds.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

use pdf_syntax::{Date, Dictionary, Document, Name, Object, ObjectId, Stream, tree};

/// The bytes of a file to attach, which print as their length rather than themselves.
///
/// Every message that carries a file derives `Debug` and two hosts trace a command by printing
/// one — the same reason `viewer_core::Secret` exists — so a payload's `Debug` is its size.
/// Shared rather than owned, because a viewer's log, its undo and its redo all hold one file.
#[derive(Clone, PartialEq, Eq)]
pub struct Payload(Arc<[u8]>);

impl Payload {
    /// The file.
    pub fn new(bytes: impl Into<Arc<[u8]>>) -> Self {
        Self(bytes.into())
    }

    /// Its bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Payload({} bytes)", self.0.len())
    }
}

impl From<Vec<u8>> for Payload {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes.into())
    }
}

impl std::ops::Deref for Payload {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

/// §12.5.6.15's four icon names, which are the only ones this tree draws (`crate::icon`).
pub const ICONS: [&str; 4] = ["Graph", "PushPin", "Paperclip", "Tag"];

/// Table 187's default: "Default value: `PushPin`".
pub const DEFAULT_ICON: &str = "PushPin";

/// The side of the square an annotation is given where nobody states a rectangle, in user-space
/// units — the side this tree draws a text annotation's icon at.
pub const DEFAULT_SIDE: f32 = 20.0;

/// Where a §12.5.6.15 annotation goes when nobody says: a [`DEFAULT_SIDE`] square,
/// [`DEFAULT_SIDE`] in from the crop box's upper-left corner.
///
/// A choice the standard leaves open, stated as one: §12.5.6.15 says where the *file* goes and
/// nothing about where the icon does. The upper-left is where a reader's eye starts on a page in
/// a left-to-right script; a page rotated by `/Rotate` keeps the same user-space corner, which a
/// caller who minds states a rectangle for.
#[must_use]
pub fn default_rect(crop_box: [f32; 4]) -> [f32; 4] {
    let [left, _, _, top] = crop_box;
    let inset = DEFAULT_SIDE;
    [
        left + inset,
        top - 2.0 * inset,
        left + 2.0 * inset,
        top - inset,
    ]
}

/// A [`DEFAULT_SIDE`] square centred on a point of the page, for a file a person put down *at*
/// a place rather than on a page.
///
/// Centred rather than cornered, and that too is a choice: a person who drops a file on a spot
/// means the spot, and an icon whose corner sits there reads as beside the place rather than on
/// it.
#[must_use]
pub fn rect_around(point: (f32, f32)) -> [f32; 4] {
    let half = DEFAULT_SIDE / 2.0;
    [
        point.0 - half,
        point.1 - half,
        point.0 + half,
        point.1 + half,
    ]
}

/// §7.11.4's embedded file stream, Tables 44 and 45.
///
/// `/CheckSum` is Table 45's — "[t]he checksum shall be calculated by applying the standard MD5
/// message-digest algorithm (defined in Internet RFC 1321) to the bytes of the embedded file
/// stream" — and `/Size` its "size of the uncompressed embedded file, in bytes". `/Subtype` is
/// Table 44's media type, written as the name §7.3.5 makes of it where the caller states one;
/// the two dates are written only where `date` is given, because neither consumer has a clock
/// of its own to state one with.
#[must_use]
pub fn embedded_file_stream(bytes: &[u8], media_type: Option<&str>, date: Option<Date>) -> Object {
    let mut params = Dictionary::new();
    params.insert(
        Name::new(&b"Size"[..]),
        Object::Integer(i64::try_from(bytes.len()).unwrap_or(i64::MAX)),
    );
    params.insert(
        Name::new(&b"CheckSum"[..]),
        Object::String(<md5::Md5 as md5::Digest>::digest(bytes).to_vec().into()),
    );
    if let Some(date) = date {
        let spelled = Object::String(pdf_date(date).into_bytes().into());
        params.insert(Name::new(&b"CreationDate"[..]), spelled.clone());
        params.insert(Name::new(&b"ModDate"[..]), spelled);
    }
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"EmbeddedFile"[..])),
    );
    if let Some(media_type) = media_type {
        dict.insert(
            Name::new(&b"Subtype"[..]),
            Object::Name(Name::new(media_type.as_bytes())),
        );
    }
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(bytes.len()).unwrap_or(i64::MAX)),
    );
    dict.insert(Name::new(&b"Params"[..]), Object::Dictionary(params));
    Object::Stream(Arc::new(Stream {
        dict,
        data: bytes.into(),
        decryption_failed: false,
    }))
}

/// §7.11.3's file specification dictionary, Table 43, for a file embedded under both of the
/// table's names.
///
/// `/F` and `/UF` are both written, as the table asks — "[t]he UF entry should be used in
/// addition to the F entry" — and both are the filing name; `/EF` names the one stream under
/// both keys, which is "a subset of the F and UF keys corresponding to the entries by those
/// names". `/Type` is required "if an EF, EP or RF entry is present".
#[must_use]
pub fn file_specification(name: &[u8], stream: ObjectId, description: Option<&str>) -> Object {
    let mut embedded = Dictionary::new();
    embedded.insert(Name::new(&b"F"[..]), Object::Reference(stream));
    embedded.insert(Name::new(&b"UF"[..]), Object::Reference(stream));
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Filespec"[..])),
    );
    dict.insert(Name::new(&b"F"[..]), Object::String(name.into()));
    dict.insert(Name::new(&b"UF"[..]), Object::String(name.into()));
    dict.insert(Name::new(&b"EF"[..]), Object::Dictionary(embedded));
    if let Some(description) = description {
        dict.insert(
            Name::new(&b"Desc"[..]),
            Object::String(pdf_syntax::text_string::encode_text_string(description).into()),
        );
    }
    Object::Dictionary(dict)
}

/// §12.5.6.15's annotation dictionary, Table 187 with Table 166's `/Rect`, `/P` and `/Contents`.
///
/// > A file attachment annotation ( PDF 1.3 ) contains a reference to a file, which typically
/// > shall be embedded in the PDF file (see 7.11.4, "Embedded file streams").
///
/// `/Name` is always written — Table 187: "PDF writers should include this entry" — and the
/// description goes in `/Contents` rather than beside the file, because the clause's one `shall`
/// makes that the entry a reader shows: "Interactive PDF processors shall use this entry rather
/// than the optional Desc entry". No `/AP` is written: this tree synthesises the four icons
/// (`crate::icon`), and a stream drawn here would be a second artwork for the same clause.
///
/// `icon` is one of [`ICONS`]; a caller that lets a person choose refuses any other name before
/// reaching here, because the viewer would report it by name (trap 5).
#[must_use]
pub fn file_attachment_annotation(
    page: ObjectId,
    rect: [f32; 4],
    specification: ObjectId,
    icon: &str,
    contents: Option<&str>,
) -> Dictionary {
    let mut annotation = Dictionary::new();
    annotation.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Annot"[..])),
    );
    annotation.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(&b"FileAttachment"[..])),
    );
    annotation.insert(
        Name::new(&b"Rect"[..]),
        Object::Array(
            rect.into_iter()
                .map(|value| Object::Real(f64::from(value)))
                .collect(),
        ),
    );
    annotation.insert(Name::new(&b"P"[..]), Object::Reference(page));
    annotation.insert(Name::new(&b"FS"[..]), Object::Reference(specification));
    annotation.insert(
        Name::new(&b"Name"[..]),
        Object::Name(Name::new(icon.as_bytes())),
    );
    if let Some(contents) = contents {
        annotation.insert(
            Name::new(&b"Contents"[..]),
            Object::String(pdf_syntax::text_string::encode_text_string(contents).into()),
        );
    }
    annotation
}

/// One `/Names` node holding every entry, in §7.9.6's order.
///
/// "[T]he keys shall be sorted in lexical order", and "[s]horter keys shall appear before longer
/// ones beginning with the same byte sequence" — which is what a byte vector's own ordering does.
#[must_use]
pub fn tree_root(mut entries: Vec<(Vec<u8>, Object)>) -> Object {
    entries.sort_by(|(a, _), (b, _)| a.cmp(b));
    let mut names_array = Vec::with_capacity(entries.len().saturating_mul(2));
    for (key, value) in entries {
        names_array.push(Object::String(key.into()));
        names_array.push(value);
    }
    let mut root = Dictionary::new();
    root.insert(Name::new(&b"Names"[..]), Object::Array(names_array));
    Object::Dictionary(root)
}

/// Where §7.7.4's `/EmbeddedFiles` tree is now: the name dictionary as the catalog states it,
/// the tree as the name dictionary states it, and the entries the tree's leaves hold, values as
/// stated.
///
/// Read once by a writer that is about to replace the tree, which is why the unresolved entries
/// are kept: [`point_holder_at_tree`] needs to know *which object* held the tree in order to
/// rewrite the nearest indirect one and say as little as possible twice.
#[derive(Debug, Clone, PartialEq)]
pub struct Tree {
    /// The catalog's `/Names` entry, unresolved.
    pub names_entry: Option<Object>,
    /// The name dictionary it names, where it names one.
    pub names_dict: Option<Dictionary>,
    /// The name dictionary's `/EmbeddedFiles` entry, unresolved.
    pub tree_entry: Option<Object>,
    /// Every key and value in the tree, values as the leaves state them.
    pub entries: Vec<(Vec<u8>, Object)>,
}

impl Tree {
    /// Reads the tree's state out of a catalog.
    ///
    /// `current` answers what an object says *now* — the document's copy, or a replacement a
    /// caller has already recorded for it — so that a writer composing several changes into one
    /// update reads the tree as its earlier changes left it.
    #[must_use]
    pub fn read(
        document: &Document,
        catalog: &Dictionary,
        current: &dyn Fn(ObjectId) -> Object,
    ) -> Self {
        let resolve = |object: &Object| match object {
            Object::Reference(id) => current(*id),
            other => other.clone(),
        };
        let names_entry = catalog.get("Names").cloned();
        let names_dict = names_entry
            .as_ref()
            .map(resolve)
            .and_then(|object| object.as_dict().cloned());
        let tree_entry = names_dict
            .as_ref()
            .and_then(|names| names.get("EmbeddedFiles").cloned());
        let entries = tree_entry
            .as_ref()
            .map(resolve)
            .and_then(|object| {
                object
                    .as_dict()
                    .map(|root| tree::name_entries(root, &|object| document.resolve(object)))
            })
            .unwrap_or_default();
        Self {
            names_entry,
            names_dict,
            tree_entry,
            entries,
        }
    }

    /// Whether the tree files something under this key, byte for byte — §7.9.6 compares keys
    /// "on a simple byte-by-byte basis".
    #[must_use]
    pub fn holds(&self, key: &[u8]) -> bool {
        self.entries.iter().any(|(existing, _)| existing == key)
    }
}

/// The objects a removed tree entry alone reached: the file specification where the leaf held a
/// reference, and the streams its `/EF` names — unless another home still reaches one of them.
///
/// **Alone** is the condition, and it is §7.11.4.1's: an embedded file has more than one home,
/// and a stream the catalog's `/AF` or a page's annotation still reaches is not deleted by the
/// tree letting go of it. The answer is the objects to mark free, or the sentence naming which
/// home keeps them in use.
///
/// # Errors
///
/// The sentence [`other_homes`] built, where a home other than the tree still reaches a stream.
pub fn freed_by_removing(document: &Document, value: &Object) -> Result<Vec<ObjectId>, String> {
    let mut freed = Vec::new();
    let specification = document.resolve(value);
    let Some(specification) = specification.as_dict() else {
        return Ok(freed);
    };
    let streams = streams_of(document, specification);
    let elsewhere = other_homes(document, &streams);
    if !elsewhere.is_empty() {
        return Err(elsewhere);
    }
    if let Object::Reference(id) = value {
        freed.push(*id);
    }
    freed.extend(streams);
    Ok(freed)
}

/// The embedded file streams a specification's `/EF` names, each once.
#[must_use]
pub fn streams_of(document: &Document, specification: &Dictionary) -> Vec<ObjectId> {
    let mut streams: Vec<ObjectId> = Vec::new();
    if let Some(embedded) = document.get_key(specification, "EF").as_dict() {
        for entry_key in ["F", "UF", "DOS", "Mac", "Unix"] {
            if let Some(Object::Reference(id)) = embedded.get(entry_key)
                && !streams.contains(id)
            {
                streams.push(*id);
            }
        }
    }
    streams
}

/// Which of the other two homes — the catalog's `/AF`, a page's annotation — still reach one of
/// these streams, as a sentence, or empty where none does.
///
/// By object identity rather than by pointer: the tree's own entry is being let go of, and what
/// matters is whether any *other* reference to the stream remains.
#[must_use]
pub fn other_homes(document: &Document, streams: &[ObjectId]) -> String {
    let mut homes: Vec<String> = Vec::new();
    if let Ok(catalog) = document.catalog()
        && let Some(associated) = document.get_key(&catalog, "AF").as_array()
        && associated.iter().any(|entry| {
            document
                .resolve(entry)
                .as_dict()
                .is_some_and(|spec| references_one_of(document, spec, streams))
        })
    {
        homes.push("the catalog's /AF".to_owned());
    }
    let pages = crate::Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        for annotation in crate::retrieval::annotations(document, &page) {
            if document
                .get_key(&annotation, "FS")
                .as_dict()
                .is_some_and(|spec| references_one_of(document, spec, streams))
            {
                homes.push(format!("page {}'s annotation", index.saturating_add(1)));
            }
        }
    }
    homes.join(", ")
}

/// Whether a file specification's `/EF` names any of these streams.
fn references_one_of(
    document: &Document,
    specification: &Dictionary,
    streams: &[ObjectId],
) -> bool {
    document
        .get_key(specification, "EF")
        .as_dict()
        .is_some_and(|embedded| {
            embedded
                .iter()
                .any(|(_, entry)| matches!(entry, Object::Reference(id) if streams.contains(id)))
        })
}

/// The objects that can hold the tree, from the outermost in.
#[derive(Debug, Clone, PartialEq)]
pub struct Holder {
    /// The catalog, as it reads now.
    pub catalog: Dictionary,
    /// Its object number, from the trailer's `/Root`.
    pub root_id: ObjectId,
    /// The catalog's `/Names` entry, unresolved.
    pub names_entry: Option<Object>,
    /// The name dictionary, where there is one.
    pub names_dict: Option<Dictionary>,
    /// The name dictionary's `/EmbeddedFiles` entry, unresolved.
    pub tree_entry: Option<Object>,
}

/// Points whichever object held the tree at the new root, rewriting the nearest indirect object
/// so that as little as possible is said twice: the old root's number where the tree was
/// indirect, the name dictionary's where that was, and the catalog's otherwise.
///
/// The new root is expected under `tree_id` in `replacements`; where the old root was indirect it
/// is moved to the old root's number, so nothing above it changes and `tree_id` is left unused.
pub fn point_holder_at_tree(
    replacements: &mut BTreeMap<ObjectId, Object>,
    tree_id: ObjectId,
    holder: Holder,
) {
    let Holder {
        mut catalog,
        root_id,
        names_entry,
        names_dict,
        tree_entry,
    } = holder;
    match (tree_entry, names_entry) {
        (Some(Object::Reference(old_root)), _) => {
            let root = replacements.remove(&tree_id).unwrap_or(Object::Null);
            replacements.insert(old_root, root);
        }
        (_, Some(Object::Reference(names_id))) => {
            let mut names = names_dict.unwrap_or_default();
            names.insert(Name::new(&b"EmbeddedFiles"[..]), Object::Reference(tree_id));
            replacements.insert(names_id, Object::Dictionary(names));
        }
        _ => {
            let mut names = names_dict.unwrap_or_default();
            names.insert(Name::new(&b"EmbeddedFiles"[..]), Object::Reference(tree_id));
            catalog.insert(Name::new(&b"Names"[..]), Object::Dictionary(names));
            replacements.insert(root_id, Object::Dictionary(catalog));
        }
    }
}

/// §7.9.4's date string, `D:YYYYMMDDHHmmSSOHH'mm'`, every field written.
///
/// The zone is written only where the date states one, and `Z` is written with the two zero
/// fields the clause's grammar places after it.
#[must_use]
pub fn pdf_date(date: Date) -> String {
    let mut text = format!(
        "D:{:04}{:02}{:02}{:02}{:02}{:02}",
        date.year, date.month, date.day, date.hour, date.minute, date.second
    );
    match date.offset {
        None => {}
        Some(0) => text.push_str("Z00'00'"),
        Some(minutes) => {
            let sign = if minutes < 0 { '-' } else { '+' };
            let absolute = minutes.unsigned_abs();
            let _ = write!(text, "{sign}{:02}'{:02}'", absolute / 60, absolute % 60);
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::{
        Tree, default_rect, embedded_file_stream, file_attachment_annotation, file_specification,
        rect_around, tree_root,
    };
    use pdf_syntax::{Object, ObjectId};

    /// Table 44 and Table 45, read back through the reader that lists them.
    #[test]
    fn the_stream_states_size_checksum_and_media_type_from_the_bytes() {
        let object = embedded_file_stream(b"a,b,c", Some("text/csv"), None);
        let Object::Stream(stream) = object else {
            panic!("a stream");
        };
        assert_eq!(&*stream.data, b"a,b,c");
        let params = stream.dict.get("Params").and_then(Object::as_dict).unwrap();
        assert_eq!(params.get("Size").and_then(Object::as_integer), Some(5));
        // `a,b,c` under RFC 1321, from `md5sum` rather than from this code.
        let Some(Object::String(digest)) = params.get("CheckSum") else {
            panic!("a checksum");
        };
        let mut hex = String::new();
        for byte in digest.iter() {
            let _ = write!(hex, "{byte:02x}");
        }
        assert_eq!(hex, "a44c56c8177e32d3613988f4dba7962e");
        assert!(params.get("ModDate").is_none(), "no clock, so no date");
        assert_eq!(
            stream
                .dict
                .get("Subtype")
                .and_then(Object::as_name)
                .map(pdf_syntax::Name::as_bytes),
            Some(&b"text/csv"[..])
        );
    }

    /// Table 43's shape, and Table 187's.
    #[test]
    fn the_specification_and_the_annotation_carry_the_tables_entries() {
        let stream = ObjectId::new(7, 0);
        let Object::Dictionary(spec) = file_specification(b"notes.txt", stream, Some("why")) else {
            panic!("a dictionary");
        };
        assert_eq!(
            spec.get("EF")
                .and_then(Object::as_dict)
                .and_then(|ef| ef.get("UF"))
                .and_then(Object::as_reference),
            Some(stream)
        );
        assert!(spec.get("Desc").is_some());

        let annotation = file_attachment_annotation(
            ObjectId::new(3, 0),
            [1.0, 2.0, 3.0, 4.0],
            ObjectId::new(8, 0),
            "Tag",
            None,
        );
        assert_eq!(
            annotation
                .get("Subtype")
                .and_then(Object::as_name)
                .map(pdf_syntax::Name::as_bytes),
            Some(&b"FileAttachment"[..])
        );
        assert_eq!(
            annotation.get("FS").and_then(Object::as_reference),
            Some(ObjectId::new(8, 0))
        );
        assert!(annotation.get("Contents").is_none());
        assert!(
            annotation.get("AP").is_none(),
            "the icon is this tree's own artwork"
        );
    }

    /// §7.9.6's order, and the two placements this module chooses.
    #[test]
    fn the_root_sorts_its_keys_and_a_placement_is_a_stated_square() {
        let Object::Dictionary(root) = tree_root(vec![
            (b"b".to_vec(), Object::Null),
            (b"ab".to_vec(), Object::Null),
            (b"a".to_vec(), Object::Null),
        ]) else {
            panic!("a dictionary");
        };
        let names = root.get("Names").and_then(Object::as_array).unwrap();
        let keys: Vec<&[u8]> = names
            .iter()
            .step_by(2)
            .filter_map(|key| match key {
                Object::String(bytes) => Some(&bytes[..]),
                _ => None,
            })
            .collect();
        assert_eq!(
            keys,
            [&b"a"[..], b"ab", b"b"],
            "shorter before longer with the same prefix"
        );

        // Compared bit for bit, because every operand is an integer a float represents exactly.
        let placed = default_rect([0.0, 0.0, 200.0, 300.0]).map(f32::to_bits);
        assert_eq!(placed, [20.0_f32, 260.0, 40.0, 280.0].map(f32::to_bits));
        let around = rect_around((50.0, 50.0)).map(f32::to_bits);
        assert_eq!(around, [40.0_f32, 40.0, 60.0, 60.0].map(f32::to_bits));
    }

    /// A tree read through `current` sees a replacement recorded for its root.
    #[test]
    fn the_tree_is_read_as_earlier_replacements_left_it() {
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles 3 0 R >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Names [(old) 4 0 R] >>",
            "<< /Type /Filespec >>",
        ];
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
        }
        let xref_at = out.len();
        let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len() + 1
        );
        let document = pdf_syntax::Document::open(out.into_bytes()).expect("a valid file");
        let catalog = document.catalog().unwrap();
        let replaced = tree_root(vec![(b"new".to_vec(), Object::Null)]);
        let current = |id: ObjectId| {
            if id == ObjectId::new(3, 0) {
                replaced.clone()
            } else {
                document.get(id)
            }
        };
        let tree = Tree::read(&document, &catalog, &current);
        assert!(tree.holds(b"new"));
        assert!(!tree.holds(b"old"));
        let plain = Tree::read(&document, &catalog, &|id| document.get(id));
        assert!(plain.holds(b"old"));
    }
}
