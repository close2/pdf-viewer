//! How many substituted composite fonts state a `/ToUnicode` that omits codes their character
//! collection names.
//!
//! The instrument behind ADR 1002. ISO 32000-2 §9.10.2 gives a processor three methods "in the
//! priority given" to map a character code to a Unicode value, and §9.7.4.2 leaves a composite
//! font whose program is absent reachable only through such a value — so for a substituted
//! composite font §9.10.2 is the glyph selection algorithm. The clause's first method is the
//! producer's `/ToUnicode` and its third is the collection's own `registry-ordering-UCS2` table,
//! and the methods are ranked *per code*: a `/ToUnicode` that omits a code has "fail[ed] to
//! produce a Unicode value" for that code and the third method is next. Until session 981 the
//! drawing route took the `/ToUnicode` for every code the moment it was non-empty and never asked
//! the collection about the ones it left out, while the readback route asked both — two routes
//! over one clause, disagreeing about what the file said.
//!
//! This counts the population that could show the difference, which is a property of the files
//! and the same on every machine: a `Type0` font whose descendant embeds no program, whose
//! `/CIDSystemInfo` names a collection this binary carries a table for, and whose `/ToUnicode`
//! states at least one mapping. For each such font it then counts the codes the font's `CMap`
//! makes addressable that the `/ToUnicode` says nothing about and the collection's table does —
//! the *capacity* for a mark lost, not its incidence, since which codes a page shows is a
//! content stream's business and this crate reads none.
//!
//! ```sh
//! cargo run --release -p pdf-font --example partial_to_unicode_census -- doc/pdf.js doc/corpora
//! ```
//!
//! **Every object the table names and every dictionary nested inside one** is walked, for trap
//! 25's reason: a font dictionary need not be an indirect object, and `issue11555.pdf` writes its
//! whole `Type0` inline in a page's `/Resources`.
//!
//! The `/ToUnicode` is read as one file; a `/UseCMap` chain beneath it is not followed here,
//! which under-reads a producer that states its table in two files and over-counts nothing.

#![expect(
    clippy::print_stdout,
    reason = "an example whose whole product is a census"
)]

use std::collections::BTreeMap;

use pdf_font::cmap::CMap;
use pdf_font::tounicode::ToUnicode;
use pdf_syntax::{Dictionary, Document, FileBytes, Object, ObjectId};

/// How many codes one font's `CMap` is walked for before it is declined, which is the loader's
/// own bound doubled so that no carried `CMap` is refused here.
const MAX_ADDRESSABLE_CODES: u64 = 1 << 18;

fn main() {
    let roots: Vec<String> = std::env::args().skip(1).collect();
    let roots = if roots.is_empty() {
        vec!["doc".to_owned()]
    } else {
        roots
    };
    let mut files = Vec::new();
    for root in &roots {
        collect(std::path::Path::new(root), &mut files);
    }
    files.sort();

    let mut documents = 0usize;
    let mut composite = 0usize;
    let mut substituted = 0usize;
    let mut registered = 0usize;
    let mut with_table = 0usize;
    let mut partial: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut omitted_codes = 0usize;

    for path in &files {
        let Ok(bytes) = FileBytes::on_disk(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let mut fonts = Vec::new();
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId::new(number, 0));
            collect_type0(&document, &object, &mut fonts);
        }
        for dict in fonts {
            composite = composite.saturating_add(1);
            let Some(descendant) = descendant_of(&document, &dict) else {
                continue;
            };
            if embeds_a_program(&document, &descendant) {
                continue;
            }
            substituted = substituted.saturating_add(1);
            let Some(collection) = collection_table(&document, &descendant) else {
                continue;
            };
            registered = registered.saturating_add(1);
            let Some(to_unicode) = to_unicode_of(&document, &dict) else {
                continue;
            };
            with_table = with_table.saturating_add(1);
            let Some(cmap) = encoding_cmap(&document, &dict) else {
                continue;
            };
            let mut omitted = 0usize;
            let mut scratch = String::new();
            let walked = cmap.each_addressable_code(MAX_ADDRESSABLE_CODES, |code| {
                scratch.clear();
                if to_unicode.append(code.value(), &mut scratch) {
                    return;
                }
                if cmap
                    .cid(code)
                    .and_then(|cid| collection.char_for(cid))
                    .is_some()
                {
                    omitted = omitted.saturating_add(1);
                }
            });
            if !walked || omitted == 0 {
                continue;
            }
            omitted_codes = omitted_codes.saturating_add(omitted);
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let entry = partial.entry(name).or_insert((0, 0));
            entry.0 = entry.0.saturating_add(1);
            entry.1 = entry.1.saturating_add(omitted);
        }
    }

    println!("{} files, {documents} opened", files.len());
    println!(
        "{composite} Type0 fonts: {substituted} embed no program, {registered} of those name a \
         collection this binary carries a table for, {with_table} of those state a /ToUnicode"
    );
    println!(
        "{} documents hold a substituted font whose /ToUnicode omits a code the collection \
         names, {omitted_codes} such codes in all (capacity, not incidence)",
        partial.len()
    );
    for (name, (found, codes)) in &partial {
        println!("  {name}: {found} font(s), {codes} codes");
    }
}

/// Every `Type0` dictionary inside one object, itself included.
fn collect_type0(document: &Document, object: &Object, into: &mut Vec<Dictionary>) {
    match object {
        Object::Dictionary(dict) => {
            if document
                .get_key(dict, "Subtype")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Type0")
            {
                into.push(dict.clone());
            }
            for (_, value) in dict.iter() {
                collect_type0(document, value, into);
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter() {
                collect_type0(document, value, into);
            }
        }
        Object::Array(items) => {
            for item in items {
                collect_type0(document, item, into);
            }
        }
        _ => {}
    }
}

/// The first descendant, which is the only one Table 119 lets a `Type0` font have.
fn descendant_of(document: &Document, dict: &Dictionary) -> Option<Dictionary> {
    let descendants = document.get_key(dict, "DescendantFonts");
    descendants
        .as_array()
        .and_then(<[Object]>::first)
        .map(|item| document.resolve(item))
        .and_then(|item| item.as_dict().cloned())
}

/// Whether the descendant's descriptor states a program with any bytes in it.
///
/// Table 120's three keys each hold "[a] stream containing a … font program", and a stream that
/// decodes to nothing contains none (ADR 0940), so it is counted as absent here as the loader
/// counts it.
fn embeds_a_program(document: &Document, descendant: &Dictionary) -> bool {
    let descriptor = document.get_key(descendant, "FontDescriptor");
    let Some(descriptor) = descriptor.as_dict() else {
        return false;
    };
    ["FontFile", "FontFile2", "FontFile3"].iter().any(|key| {
        document
            .get_key(descriptor, key)
            .as_stream()
            .and_then(|stream| document.decoded_stream_data(stream))
            .is_some_and(|data| !data.is_empty())
    })
}

/// §9.10.2's third method's table, where the descendant names a collection this binary carries.
fn collection_table(document: &Document, descendant: &Dictionary) -> Option<ToUnicode> {
    let info = document.get_key(descendant, "CIDSystemInfo");
    let info = info.as_dict()?;
    let text = |key: &str| {
        document
            .get_key(info, key)
            .as_string()
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    };
    pdf_font::predefined::cid_to_unicode(&text("Registry")?, &text("Ordering")?)
}

/// The font's own `/ToUnicode`, where it states at least one mapping.
fn to_unicode_of(document: &Document, dict: &Dictionary) -> Option<ToUnicode> {
    let object = document.get_key(dict, "ToUnicode");
    let stream = object.as_stream()?;
    let bytes = document.decoded_stream_data(stream)?;
    let table = ToUnicode::parse(&bytes);
    (!table.is_empty()).then_some(table)
}

/// The `CMap` the font's `/Encoding` names or embeds (§9.7.5).
fn encoding_cmap(document: &Document, dict: &Dictionary) -> Option<CMap> {
    let encoding = document.get_key(dict, "Encoding");
    if let Some(name) = encoding.as_name() {
        let name = String::from_utf8_lossy(name.as_bytes());
        return match name.as_ref() {
            "Identity-H" => Some(CMap::identity()),
            "Identity-V" => Some(CMap::identity_vertical()),
            other => pdf_font::predefined::cmap(other),
        };
    }
    let stream = encoding.as_stream()?;
    let bytes = document.decoded_stream_data(stream)?;
    let used = document
        .get_key(&stream.dict, "UseCMap")
        .as_name()
        .and_then(|name| pdf_font::predefined::cmap(&String::from_utf8_lossy(name.as_bytes())));
    Some(CMap::parse(&bytes, used.as_ref()))
}

/// Every `.pdf` under `dir`, recursively.
fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "pdf") {
            out.push(path);
        }
    }
}
