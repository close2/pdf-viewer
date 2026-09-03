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
