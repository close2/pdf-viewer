//! How many corpus documents state a mesh shading, and how many of them state a `/Function`.
//!
//! ISO 32000-2 §8.7.4.5.5 gives a type 4 to 7 shading two ways of carrying colour: a set of
//! components per vertex, or a single parametric value per vertex with a `/Function` that turns
//! it into a colour. The second one carries a `shall` about the *order* of two operations —
//! "[a]ll linear interpolation within the triangle mesh shall be done using the t values. After
//! interpolation, the results shall be passed to the function(s)" — and this counts the
//! population that clause can reach before anything is decided about it.
//!
//! Counted per document and per shading, with the colour space family beside it, because
//! §8.7.4.4's own rule about *where* interpolation happens turns on that family.
//!
//! ```sh
//! cargo run --release -p pdf-model --example mesh_census
//! ```
#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    reason = "a measurement whose output is its purpose: it stops loudly where the corpus is \
              missing, and its counters over the corpus's shadings are four orders of \
              magnitude below what a usize counts"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use pdf_syntax::{Document, Object, ObjectId};

fn corpus() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("the submodule is checked out")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|e| e == "pdf"))
        .collect();
    files.sort();
    files
}

/// The family of a `/ColorSpace` entry, named as the standard names it.
fn family(document: &Document, space: &Object) -> String {
    match document.resolve(space) {
        Object::Name(name) => String::from_utf8_lossy(&name.0).into_owned(),
        Object::Array(items) => items.first().map_or_else(
            || "array".to_owned(),
            |head| match document.resolve(head) {
                Object::Name(name) => String::from_utf8_lossy(&name.0).into_owned(),
                _ => "array".to_owned(),
            },
        ),
        Object::Stream(_) => "stream".to_owned(),
        _ => "absent".to_owned(),
    }
}

fn main() {
    // type -> (shadings, of which with a /Function)
    let mut by_type: BTreeMap<i64, (usize, usize)> = BTreeMap::new();
    // colour space family -> (shadings, of which with a /Function)
    let mut by_space: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    // colour space family -> the documents that state a mesh in it, which is what §8.7.4.4's
    // rule about where interpolation happens turns on.
    let mut spaces_of: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut documents: Vec<String> = Vec::new();
    let mut with_function: Vec<String> = Vec::new();

    for path in corpus() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let (mut meshes, mut parametric) = (0usize, 0usize);
        // This document's own meshes by colour space family, so that the population a
        // §8.7.4.4 departure reaches can be attributed to a file rather than to a total.
        let mut mine: BTreeMap<String, usize> = BTreeMap::new();
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            let Some(stream) = object.as_stream() else {
                continue;
            };
            let dict = &stream.dict;
            let Some(kind) = document.get_key(dict, "ShadingType").as_integer() else {
                continue;
            };
            if !(4..=7).contains(&kind) {
                continue;
            }
            let function = !matches!(document.get_key(dict, "Function"), Object::Null);
            meshes += 1;
            parametric += usize::from(function);
            let entry = by_type.entry(kind).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += usize::from(function);
            let space = family(&document, &document.get_key(dict, "ColorSpace"));
            *mine.entry(space.clone()).or_insert(0) += 1;
            spaces_of
                .entry(space.clone())
                .or_default()
                .insert(name.clone());
            let entry = by_space.entry(space).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += usize::from(function);
        }
        if meshes > 0 {
            let spaces: Vec<String> = mine
                .iter()
                .map(|(space, count)| format!("{count} {space}"))
                .collect();
            documents.push(format!("{name} ({})", spaces.join(" + ")));
        }
        if parametric > 0 {
            with_function.push(format!("{name} ({parametric})"));
        }
    }

    println!(
        "{} documents state a mesh shading, {} of them one with a /Function",
        documents.len(),
        with_function.len()
    );
    for (kind, (total, parametric)) in &by_type {
        println!("  type {kind}: {total} shadings, {parametric} with a /Function");
    }
    for (space, (total, parametric)) in &by_space {
        let documents = spaces_of
            .get(space)
            .map(|names| names.iter().cloned().collect::<Vec<_>>().join(", "))
            .unwrap_or_default();
        println!("  {space}: {total} shadings, {parametric} with a /Function — {documents}");
    }
    println!();
    println!("mesh: {}", documents.join(", "));
    println!();
    println!("with a /Function: {}", with_function.join(", "));
}
