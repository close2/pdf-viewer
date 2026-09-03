//! Walks written independently of the crate, so that a gate's expected value is derived from
//! the document rather than read off what the tool printed.

#![expect(
    clippy::expect_used,
    dead_code,
    reason = "a support module is compiled into each test binary and each uses part of it"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// A committed document, which every checkout has once `doc/specifications.zip` is unpacked.
pub(crate) fn committed(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(name)
}

/// A corpus document's path, or `None` when the submodule is not checked out.
pub(crate) fn corpus(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    path.exists().then_some(path)
}

/// Every page's dictionary, in page order: §7.7.3.2's tree walked from `/Root /Pages`, `/Kids`
/// first to last, leaves being the dictionaries whose `/Type` is `/Page`.
pub(crate) fn page_dictionaries(document: &Document) -> Vec<Dictionary> {
    let mut out = Vec::new();
    let Ok(catalog) = document.catalog() else {
        return out;
    };
    let root = document.get_key(&catalog, "Pages");
    let mut visited = BTreeSet::new();
    if let Some(root) = root.as_dict() {
        descend(document, root, &mut visited, &mut out, 0);
    }
    out
}

/// One node of the page tree.
fn descend(
    document: &Document,
    node: &Dictionary,
    visited: &mut BTreeSet<ObjectId>,
    out: &mut Vec<Dictionary>,
    depth: usize,
) {
    if depth > 64 {
        return;
    }
    let kids = document.get_key(node, "Kids");
    let Some(kids) = kids.as_array() else {
        let kind = document.get_key(node, "Type");
        if kind.as_name().and_then(|name| name.as_str()) == Some("Page") {
            out.push(node.clone());
        }
        return;
    };
    for kid in kids {
        if let Object::Reference(id) = kid
            && !visited.insert(*id)
        {
            continue;
        }
        let kid = document.resolve(kid);
        if let Some(kid) = kid.as_dict() {
            descend(document, kid, visited, out, depth.saturating_add(1));
        }
    }
}

/// The distinct image `XObject`s a document's pages reach through their resources, forms
/// descended (§7.8.3's `/XObject` sub-dictionary, §8.10.2's form resources), by object id.
///
/// Counts what the standard's structure makes reachable, which is the population `images`
/// claims to list. A direct-object image (no reference) would be uncountable here and is not
/// in any committed document.
pub(crate) fn reachable_image_objects(document: &Document) -> BTreeSet<ObjectId> {
    let mut images = BTreeSet::new();
    let mut forms = BTreeSet::new();
    for page in page_dictionaries(document) {
        let resources = document.get_key(&page, "Resources");
        if let Some(resources) = resources.as_dict() {
            xobjects_of(document, resources, &mut images, &mut forms, 0);
        }
    }
    images
}

/// One resource dictionary's `/XObject`s.
fn xobjects_of(
    document: &Document,
    resources: &Dictionary,
    images: &mut BTreeSet<ObjectId>,
    forms: &mut BTreeSet<ObjectId>,
    depth: usize,
) {
    if depth > 16 {
        return;
    }
    let xobjects = document.get_key(resources, "XObject");
    let Some(xobjects) = xobjects.as_dict() else {
        return;
    };
    for (_, entry) in xobjects.iter() {
        let Object::Reference(id) = entry else {
            continue;
        };
        let resolved = document.resolve(entry);
        let Some(stream) = resolved.as_stream() else {
            continue;
        };
        let subtype = document.get_key(&stream.dict, "Subtype");
        match subtype.as_name().and_then(|name| name.as_str()) {
            Some("Image") => {
                images.insert(*id);
            }
            Some("Form") => {
                if !forms.insert(*id) {
                    continue;
                }
                let inner = document.get_key(&stream.dict, "Resources");
                if let Some(inner) = inner.as_dict() {
                    xobjects_of(document, inner, images, forms, depth.saturating_add(1));
                }
            }
            _ => {}
        }
    }
}

/// The file names §12.5.6.15's annotations carry, page by page in `/Annots` order: each
/// `/Subtype /FileAttachment` annotation's `/FS`, and Table 43's `/UF` else `/F` of it, as a
/// text string. Paired with the page number, counted from 1.
pub(crate) fn annotation_file_names(document: &Document) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (index, page) in page_dictionaries(document).iter().enumerate() {
        let annots = document.get_key(page, "Annots");
        let Some(annots) = annots.as_array() else {
            continue;
        };
        for annot in annots {
            let annot = document.resolve(annot);
            let Some(annot) = annot.as_dict() else {
                continue;
            };
            let subtype = document.get_key(annot, "Subtype");
            if subtype.as_name().and_then(|name| name.as_str()) != Some("FileAttachment") {
                continue;
            }
            let specification = document.get_key(annot, "FS");
            let Some(specification) = specification.as_dict() else {
                continue;
            };
            let name =
                ["UF", "F"]
                    .iter()
                    .find_map(|key| match document.get_key(specification, key) {
                        Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
                        _ => None,
                    });
            if let Some(name) = name {
                out.push((index.saturating_add(1), name));
            }
        }
    }
    out
}

/// The oracle backend's raster of one page at 150 dpi — the pipeline
/// `crates/pdf-model/examples/render_at.rs` runs, stated here independently of the crate.
pub(crate) fn oracle(bytes: &[u8], index: usize) -> pdf_render::Raster {
    use pdf_render::Rasterizer as _;
    let document = Document::open(bytes.to_vec()).expect("a PDF");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(index).expect("that page");
    let view = pdf_model::view::ViewState::of(&document);
    let interpretation = pdf_model::content::interpret_with_fonts(
        &document,
        &page,
        &view,
        &pdf_model::content::FontCache::new(),
    );
    let list = interpretation.display_list;
    // ISO 32000-2 §8.3.2.3: 72 user-space units to the inch, so 150 dpi is 150/72.
    let target = pdf_render::TargetSpec::for_page(&list, 150.0 / 72.0, 1 << 28).expect("a target");
    render_cpu::CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("drawn")
}

/// Decodes a PNG to its RGBA8 samples.
pub(crate) fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("a PNG");
    let mut data = vec![0; reader.output_buffer_size().expect("a bounded size")];
    let info = reader.next_frame(&mut data).expect("a frame");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    data.truncate(info.buffer_size());
    (info.width, info.height, data)
}

/// What §14.7's carry looks like in a derived document, judged against the clauses.
///
/// Every check here is derivable from ISO 32000-2 rather than from what the writer does, which
/// is the whole point of this module (trap 8): the walk asks the *output* whether it is a
/// conforming tagged document, not whether it matches what `structure.rs` intended.
pub(crate) struct StructureCheck {
    /// Whether the output states a `/StructTreeRoot` at all.
    pub(crate) carried: bool,
    /// Structure elements reached from the root.
    pub(crate) elements: usize,
    /// Pages whose `/StructParents` resolves in the output's own parent tree.
    pub(crate) resolved_pages: usize,
    /// Objects whose `/StructParent` resolves in it.
    pub(crate) resolved_objects: usize,
    /// Everything the output states that a clause forbids, each with the clause.
    pub(crate) faults: Vec<String>,
}

/// Reads a derived document's §14.7 structure and checks the four properties a carry owes.
///
/// 1. Every page stating Table 359's `/StructParents` has an entry in §14.7.5.4's parent tree,
///    and it is "an array of indirect references to the sequences' parent structure elements".
/// 2. Every object stating Table 359's `/StructParent` has one too, and it "shall be an indirect
///    reference to the parent structure element".
/// 3. Every structure element's Table 355 `/Pg` "shall be an indirect reference" to "[a] page
///    object", and the page has to be one this document holds — the property that separates a
///    pruned tree from a half-carried one.
/// 4. §14.7.5.4's `/ParentTreeNextKey` "shall hold an integer value greater than any that is
///    currently in use as a key in the structural parent tree".
pub(crate) fn check_structure(read: &Document) -> StructureCheck {
    let mut out = StructureCheck {
        carried: false,
        elements: 0,
        resolved_pages: 0,
        resolved_objects: 0,
        faults: Vec::new(),
    };
    let Ok(catalog) = read.catalog() else {
        return out;
    };
    let root = read.get_key(&catalog, "StructTreeRoot");
    let Some(root) = root.as_dict() else {
        return out;
    };
    out.carried = true;
    if read
        .get_key(root, "Type")
        .as_name()
        .is_none_or(|name| name.as_bytes() != b"StructTreeRoot")
    {
        out.faults
            .push("Table 354 makes /Type required and \"shall be StructTreeRoot\"".to_owned());
    }
    let tree = read.get_key(root, "ParentTree");
    let tree = tree.as_dict().cloned();
    let entry = |key: i64| {
        tree.as_ref().and_then(|tree| {
            pdf_syntax::tree::lookup_unresolved(tree, &pdf_syntax::tree::TreeKey::Number(key), &{
                |value: &Object| read.resolve(value)
            })
        })
    };

    let held: BTreeSet<ObjectId> = page_ids(read);
    let mut largest: Option<i64> = None;
    let note = |key: i64, largest: &mut Option<i64>| {
        *largest = Some(largest.map_or(key, |seen: i64| seen.max(key)));
    };

    for (index, page) in page_dictionaries(read).iter().enumerate() {
        let Some(key) = read.get_key(page, "StructParents").as_integer() else {
            continue;
        };
        note(key, &mut largest);
        match entry(key) {
            Some(Object::Array(_)) => out.resolved_pages = out.resolved_pages.saturating_add(1),
            Some(other) => out.faults.push(format!(
                "§14.7.5.4: page {}'s /StructParents {key} names {other:?} and the clause makes a \
                 content stream's value \"an array of indirect references\"",
                index.saturating_add(1)
            )),
            None => out.faults.push(format!(
                "§14.7.5.4: page {}'s /StructParents {key} has no entry in the parent tree",
                index.saturating_add(1)
            )),
        }
    }

    for number in read.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        let value = read.get(id);
        let dict = match &value {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            _ => continue,
        };
        if let Some(key) = dict.get("StructParent").and_then(Object::as_integer) {
            note(key, &mut largest);
            check_object_key(&entry, number, key, &mut out);
        }
        // A structure element is a dictionary with Table 355's two required entries; asked of
        // the file rather than of a `/Type`, which the table makes optional.
        if dict.get("S").is_none() || dict.get("P").is_none() {
            continue;
        }
        out.elements = out.elements.saturating_add(1);
        let Some(page) = dict.get("Pg").and_then(Object::as_reference) else {
            continue;
        };
        if !held.contains(&page) {
            out.faults.push(format!(
                "Table 355: structure element {number}'s /Pg names object {} and this document \
                 holds no such page",
                page.number
            ));
        }
    }

    if let Some(next) = read.get_key(root, "ParentTreeNextKey").as_integer()
        && let Some(largest) = largest
        && next <= largest
    {
        out.faults.push(format!(
            "§14.7.5.4: /ParentTreeNextKey is {next} and \"shall hold an integer value greater \
             than any that is currently in use as a key\", which is {largest}"
        ));
    }
    out
}

/// Every page object's number, for the `/Pg` check above.
fn page_ids(document: &Document) -> BTreeSet<ObjectId> {
    pdf_model::Pages::new(document)
        .indices()
        .into_keys()
        .collect()
}

/// §14.7.5.4's second bullet, checked for one object: "[f]or an object identified as a content
/// item by means of an object reference … the value shall be an indirect reference to the parent
/// structure element."
fn check_object_key(
    entry: &dyn Fn(i64) -> Option<Object>,
    number: u32,
    key: i64,
    out: &mut StructureCheck,
) {
    match entry(key) {
        Some(Object::Reference(_)) => {
            out.resolved_objects = out.resolved_objects.saturating_add(1);
        }
        Some(other) => out.faults.push(format!(
            "§14.7.5.4: object {number}'s /StructParent {key} names {other:?} and the clause \
             makes an object's value \"an indirect reference to the parent structure element\""
        )),
        None => out.faults.push(format!(
            "§14.7.5.4: object {number}'s /StructParent {key} has no entry in the parent tree"
        )),
    }
}

/// What §7.5.7 and §7.5.5 say about a document `optimize` wrote, judged against the clauses.
///
/// The same discipline as [`check_structure`] and for the same reason (trap 8): every property
/// below is a sentence of ISO 32000-2 asked of the *output*, never a comparison with what
/// `optimize` meant to do. A file the writer built consistently wrong would satisfy the second
/// kind of check and fail these.
pub(crate) struct OptimizedCheck {
    /// How many objects the trailer's `/Size` declares, less §7.5.4's free head.
    pub(crate) declared: usize,
    /// How many §7.5.7 object streams the file holds.
    pub(crate) object_streams: usize,
    /// How many objects those carry, summed from each carrier's own `/N`.
    pub(crate) compressed: usize,
    /// Objects no path from `/Root` or `/Info` reaches, and which no clause excuses.
    pub(crate) unreachable: Vec<u32>,
    /// Everything the output states that a clause forbids, each with the clause.
    pub(crate) faults: Vec<String>,
}

/// Reads a rewritten document and checks what §7.5.5 and §7.5.7 require of it.
///
/// 1. **Nothing is written that nothing reaches.** §7.5.5's Table 15 makes `/Root` "[t]he
///    catalog dictionary for the PDF file" and §7.7.2's Table 29 the root of its object
///    hierarchy, so after a pruning rewrite every object a reader can ask for should be one
///    some path from `/Root` — or from the trailer's other root, §14.3.3's `/Info` — arrives
///    at. §7.5.7 states the one exception in as many words: an object stream is an indirect
///    object "although there might not be any references to it (of the form 243 0 R)", and
///    §7.5.8's cross-reference stream is the same. Both are recognised by their own `/Type`.
/// 2. **Table 16's entries.** `/Type` is `ObjStm`, `/N` is "[t]he number of indirect objects
///    stored in the stream" and `/First` "[t]he byte offset in the decoded stream of the first
///    compressed object" — so the header holds exactly `/N` pairs, and `/First` is where they
///    end.
/// 3. **"The byte offsets shall be in increasing order."**
/// 4. **`/Extends` "[a] reference to another object stream"**, whose links "form a directed
///    acyclic graph" — so a carrier that states one names an object whose `/Type` is `ObjStm`,
///    and following the chain terminates.
pub(crate) fn check_optimized(read: &Document) -> OptimizedCheck {
    let declared = read
        .trailer()
        .get("Size")
        .and_then(Object::as_integer)
        .and_then(|size| usize::try_from(size).ok())
        .map_or(0, |size| size.saturating_sub(1));
    let mut out = OptimizedCheck {
        declared,
        object_streams: 0,
        compressed: 0,
        unreachable: Vec::new(),
        faults: Vec::new(),
    };

    let mut reached: BTreeSet<ObjectId> = BTreeSet::new();
    for key in ["Root", "Info"] {
        if let Some(id) = read.trailer().get(key).and_then(Object::as_reference) {
            walk_closure(read, id, &mut reached);
        }
    }

    for number in 1..=u32::try_from(declared).unwrap_or(0) {
        let id = ObjectId::new(number, 0);
        let value = read.get(id);
        if value == Object::Null {
            continue;
        }
        let kind = value
            .as_stream()
            .and_then(|stream| stream.dict.get("Type"))
            .and_then(Object::as_name)
            .map(|name| name.as_bytes().to_vec());
        match kind.as_deref() {
            Some(b"ObjStm") => {
                out.object_streams = out.object_streams.saturating_add(1);
                check_object_stream(read, id, &mut out);
                continue;
            }
            // §7.5.8: "[l]ike any stream, a cross-reference stream shall be an indirect
            // object", and nothing refers to it either.
            Some(b"XRef") => continue,
            _ => {}
        }
        if !reached.contains(&id) {
            out.unreachable.push(number);
        }
    }
    out
}

/// One §7.5.7 carrier read back against Table 16 and the clause's own sentences.
fn check_object_stream(read: &Document, id: ObjectId, out: &mut OptimizedCheck) {
    let value = read.get(id);
    let Some(stream) = value.as_stream() else {
        return;
    };
    let Some(count) = read
        .get_key(&stream.dict, "N")
        .as_integer()
        .and_then(|value| usize::try_from(value).ok())
    else {
        out.faults.push(format!(
            "§7.5.7 Table 16: object stream {} states no /N, which is \"( Required ) The number \
             of indirect objects stored in the stream\"",
            id.number
        ));
        return;
    };
    let Some(first) = read
        .get_key(&stream.dict, "First")
        .as_integer()
        .and_then(|value| usize::try_from(value).ok())
    else {
        out.faults.push(format!(
            "§7.5.7 Table 16: object stream {} states no /First",
            id.number
        ));
        return;
    };
    out.compressed = out.compressed.saturating_add(count);

    let Some(data) = read.decoded_stream_data(stream) else {
        out.faults.push(format!(
            "§7.5.7: object stream {}'s data does not decode",
            id.number
        ));
        return;
    };
    let Some(head) = data.get(..first) else {
        out.faults.push(format!(
            "§7.5.7: object stream {}'s /First {first} is past its {} decoded bytes",
            id.number,
            data.len()
        ));
        return;
    };
    let numbers: Vec<i64> = String::from_utf8_lossy(head)
        .split_ascii_whitespace()
        .filter_map(|word| word.parse::<i64>().ok())
        .collect();
    if numbers.len() != count.saturating_mul(2) {
        out.faults.push(format!(
            "§7.5.7: object stream {} states /N {count} and its header holds {} integer(s), and \
             the clause makes the header \"N pairs of integers\"",
            id.number,
            numbers.len()
        ));
        return;
    }
    let mut previous: Option<i64> = None;
    for pair in numbers.chunks_exact(2) {
        let (member, offset) = (pair[0], pair[1]);
        if previous.is_some_and(|last| offset <= last) {
            out.faults.push(format!(
                "§7.5.7: object stream {}'s offsets are not increasing, and \"[t]he byte offsets \
                 shall be in increasing order\"",
                id.number
            ));
        }
        previous = Some(offset);
        if first.saturating_add(usize::try_from(offset).unwrap_or(usize::MAX)) > data.len() {
            out.faults.push(format!(
                "§7.5.7: object stream {}'s member {member} starts past the decoded stream",
                id.number
            ));
        }
        // "The following objects shall not be stored in an object stream:", whose first
        // bullet is "Stream objects".
        let Ok(number) = u32::try_from(member) else {
            out.faults.push(format!(
                "§7.5.7: object stream {} names member {member}",
                id.number
            ));
            continue;
        };
        if read.get(ObjectId::new(number, 0)).as_stream().is_some() {
            out.faults.push(format!(
                "§7.5.7: object {number} is a stream and is stored in object stream {}, and \
                 \"[t]he following objects shall not be stored in an object stream:\", whose \
                 first bullet is \"Stream objects\"",
                id.number
            ));
        }
    }

    check_extends(read, id, out);
}

/// Table 16's `/Extends`, and the sentence that bounds it: the links "form a directed acyclic
/// graph", so following them from one carrier has to terminate at another.
fn check_extends(read: &Document, id: ObjectId, out: &mut OptimizedCheck) {
    let mut at = id;
    let mut seen = BTreeSet::new();
    while let Some(next) = read
        .get(at)
        .as_stream()
        .and_then(|stream| stream.dict.get("Extends"))
        .and_then(Object::as_reference)
    {
        if !seen.insert(next) {
            out.faults.push(format!(
                "§7.5.7: /Extends from object stream {} is a cycle, and the clause makes the \
                 links \"a directed acyclic graph\"",
                id.number
            ));
            break;
        }
        let extended = read.get(next);
        let is_carrier = extended
            .as_stream()
            .and_then(|stream| stream.dict.get("Type"))
            .and_then(Object::as_name)
            .is_some_and(|name| name.as_bytes() == b"ObjStm");
        if !is_carrier {
            out.faults.push(format!(
                "§7.5.7: object stream {}'s /Extends names object {}, which is not an object \
                 stream, and the entry is \"[a] reference to another object stream\"",
                id.number, next.number
            ));
            break;
        }
        at = next;
    }
}

/// Every object one root reaches, `/Length` excepted for the reason `optimize` gives.
fn walk_closure(read: &Document, start: ObjectId, reached: &mut BTreeSet<ObjectId>) {
    let mut queue = vec![start];
    reached.insert(start);
    while let Some(id) = queue.pop() {
        let value = read.get(id);
        collect(&value, 0, reached, &mut queue);
    }
}

/// Every reference in one value, queued once.
fn collect(
    value: &Object,
    depth: usize,
    reached: &mut BTreeSet<ObjectId>,
    queue: &mut Vec<ObjectId>,
) {
    if depth >= 256 {
        return;
    }
    match value {
        Object::Reference(id) => {
            if reached.insert(*id) {
                queue.push(*id);
            }
        }
        Object::Array(items) => {
            for item in items {
                collect(item, depth.saturating_add(1), reached, queue);
            }
        }
        Object::Dictionary(dict) => {
            for (_, item) in dict.iter() {
                collect(item, depth.saturating_add(1), reached, queue);
            }
        }
        Object::Stream(stream) => {
            for (_, item) in stream.dict.iter() {
                collect(item, depth.saturating_add(1), reached, queue);
            }
        }
        _ => {}
    }
}
