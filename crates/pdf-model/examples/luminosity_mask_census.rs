//! What a `/Luminosity` soft mask's group is painted with, and in which space (§11.5.3).
//!
//! §11.5.3 composites a luminosity mask's group in the space its `/CS` names and converts the
//! result to a luminosity there; this tree composited in device RGB and named the difference
//! by name, on the group's `/CS` alone. **What a group actually paints decides whether that
//! difference exists at all**, and this counts that rather than the declaration — which is how
//! the three-hundred-and-eightieth session found that every departure the corpus carries is in
//! the *backdrop* rather than in the artwork (ADR 0217).
//!
//! Walks every page's `/Resources /ExtGState`, and every form `XObject`'s, for a `/SMask`
//! whose `/S` is `/Luminosity`, then reads the group's own content stream for the colour
//! operators it uses and the ink §10.4.2.3 weighs in the colours it sets.
//!
//! ```sh
//! cargo run --release -p pdf-model --example luminosity_mask_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};

use pdf_model::colour::ColourSpace;
use pdf_syntax::{Dictionary, Document, Lexer, Object, Token};

/// How far a form `XObject`'s own resources are followed.
const MAX_DEPTH: usize = 8;

/// How many pages of one document are walked.
const MAX_PAGES: usize = 100;

/// What one mask group was found to hold.
#[derive(Default)]
struct Group {
    /// The group attributes dictionary's `/CS`, as written, with its component count.
    space: String,
    /// The ink §10.4.2.3 weighs in Table 142's `/BC`, resolved through its default.
    backdrop: f32,
    /// Every content-stream operator that sets a colour or paints something not a path.
    operators: BTreeSet<String>,
    /// The `/ColorSpace` of every `XObject` and shading the group's own resources hold.
    drawn: BTreeSet<String>,
    /// The largest ink any `k`/`K` operator in the group sets.
    ink: f32,
}

fn main() {
    let mut documents = 0_usize;
    let mut with_masks = 0_usize;
    let mut spaces: BTreeMap<String, usize> = BTreeMap::new();
    let mut over_inked = 0_usize;
    let mut lines: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        let pages = pdf_model::Pages::new(&document);
        let mut groups: Vec<Group> = Vec::new();
        for index in 0..pages.len().min(MAX_PAGES) {
            let Some(page) = pages.get(index) else {
                continue;
            };
            let mut seen = BTreeSet::new();
            walk(&document, &page.resources, 0, &mut seen, &mut groups);
        }
        if groups.is_empty() {
            continue;
        }
        with_masks = with_masks.saturating_add(1);
        for group in &groups {
            let counter = spaces.entry(group.space.clone()).or_default();
            *counter = counter.saturating_add(1);
            if group.backdrop > 1.0 || group.ink > 1.0 {
                over_inked = over_inked.saturating_add(1);
            }
            if group.space.contains("RGB") || group.space.starts_with("absent") {
                continue;
            }
            lines.push(format!(
                "  {name}: /CS {} /BC ink {:.2} artwork ink <= {:.2} [{}] {}",
                group.space,
                group.backdrop,
                group.ink,
                group.drawn.iter().cloned().collect::<Vec<_>>().join(", "),
                group
                    .operators
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
    }

    println!("{documents} document(s) opened, {with_masks} with a /Luminosity soft mask");
    println!("  group /CS: {spaces:?}");
    println!("  {over_inked} group(s) carry more than one unit of §10.4.2.3 ink");
    println!("  groups whose /CS is not an RGB one:");
    for line in &lines {
        println!("{line}");
    }
}

/// Every `/ExtGState` reachable from one resource dictionary, including through form `XObject`s.
fn walk(
    document: &Document,
    resources: &Dictionary,
    depth: usize,
    seen: &mut BTreeSet<pdf_syntax::ObjectId>,
    groups: &mut Vec<Group>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    if let Some(states) = document.get_key(resources, "ExtGState").as_dict() {
        for (_, value) in states.iter() {
            let resolved = document.resolve(value);
            let Some(state) = resolved.as_dict() else {
                continue;
            };
            let mask = document.get_key(state, "SMask");
            let Some(mask) = mask.as_dict() else {
                continue;
            };
            let luminosity = document
                .get_key(mask, "S")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Luminosity");
            if !luminosity {
                continue;
            }
            let group = document.get_key(mask, "G");
            let Some(stream) = group.as_stream() else {
                continue;
            };
            groups.push(read_group(document, mask, stream));
        }
    }
    let objects = document.get_key(resources, "XObject");
    let Some(dict) = objects.as_dict() else {
        return;
    };
    for (_, value) in dict.iter() {
        if let Some(id) = value.as_reference()
            && !seen.insert(id)
        {
            continue;
        }
        let resolved = document.resolve(value);
        let Object::Stream(stream) = &resolved else {
            continue;
        };
        let inner = document.get_key(&stream.dict, "Resources");
        if let Some(inner) = inner.as_dict() {
            walk(document, inner, depth.saturating_add(1), seen, groups);
        }
    }
}

/// Reads one mask group's declared space, its backdrop and what its content stream paints.
#[expect(
    clippy::too_many_lines,
    reason = "one census reading one group; splitting it would hide what is counted"
)]
fn read_group(document: &Document, mask: &Dictionary, stream: &pdf_syntax::Stream) -> Group {
    let attributes = document.get_key(&stream.dict, "Group");
    let entry = attributes.as_dict().map_or(Object::Null, |attributes| {
        document.get_key(attributes, "CS")
    });
    let parsed = ColourSpace::parse(document, &entry, &Dictionary::new());
    let mut group = Group {
        space: format!(
            "{} ({})",
            describe(document, &entry),
            parsed.as_ref().map_or_else(
                || "unparsed".to_owned(),
                |space| format!("{} components", space.components())
            )
        ),
        backdrop: parsed.as_ref().map_or(1.0, |space| {
            let values: Vec<f32> = document
                .get_key(mask, "BC")
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .map(|item| document.resolve(item))
                        .filter_map(|item| item.as_number())
                        .map(|value| {
                            #[expect(
                                clippy::cast_possible_truncation,
                                reason = "a colour component outside f32's range is not one"
                            )]
                            {
                                value as f32
                            }
                        })
                        .collect()
                })
                .filter(|values: &Vec<f32>| values.len() == space.components())
                .unwrap_or_else(|| space.initial_colour());
            space.ink(&values)
        }),
        ..Group::default()
    };

    if let Some(inner) = document.get_key(&stream.dict, "Resources").as_dict() {
        for category in ["Shading", "XObject"] {
            let entries = document.get_key(inner, category);
            let Some(entries) = entries.as_dict() else {
                continue;
            };
            for (_, value) in entries.iter() {
                let resolved = document.resolve(value);
                let dict = match &resolved {
                    Object::Dictionary(dict) => dict.clone(),
                    Object::Stream(stream) => stream.dict.clone(),
                    _ => continue,
                };
                let space = document.get_key(&dict, "ColorSpace");
                let subtype = document.get_key(&dict, "Subtype");
                let kind = subtype.as_name().map_or_else(
                    || category.to_owned(),
                    |name| String::from_utf8_lossy(name.as_bytes()).into_owned(),
                );
                group
                    .drawn
                    .insert(format!("{kind} {}", describe(document, &space)));
            }
        }
    }

    let Some(content) = document.decoded_stream_data(stream) else {
        group.operators.insert("<undecodable>".to_owned());
        return group;
    };
    let mut lexer = Lexer::new(&content);
    let mut operands: Vec<f32> = Vec::new();
    while let Some(token) = lexer.next_token() {
        match token {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a content-stream operand outside f32's range is not a colour"
            )]
            Token::Integer(value) => operands.push(value as f32),
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a content-stream operand outside f32's range is not a colour"
            )]
            Token::Real(value) => operands.push(value as f32),
            Token::Keyword(operator) => {
                let operator = String::from_utf8_lossy(operator).into_owned();
                if matches!(
                    operator.as_str(),
                    "k" | "K"
                        | "g"
                        | "G"
                        | "rg"
                        | "RG"
                        | "sc"
                        | "SC"
                        | "scn"
                        | "SCN"
                        | "cs"
                        | "CS"
                        | "sh"
                        | "Do"
                        | "BI"
                        | "gs"
                ) {
                    group.operators.insert(operator.clone());
                }
                if (operator == "k" || operator == "K") && operands.len() >= 4 {
                    group.ink = group.ink.max(ColourSpace::Cmyk.ink(&operands));
                }
                operands.clear();
            }
            _ => operands.clear(),
        }
    }
    group
}

/// A colour space object as a person would name it, without resolving it to components.
fn describe(document: &Document, entry: &Object) -> String {
    match document.resolve(entry) {
        Object::Null => "absent".to_owned(),
        Object::Name(name) => format!("/{}", String::from_utf8_lossy(name.as_bytes())),
        Object::Array(items) => items.first().map_or_else(
            || "an empty array".to_owned(),
            |first| match document.resolve(first) {
                Object::Name(name) => {
                    format!("[/{} …]", String::from_utf8_lossy(name.as_bytes()))
                }
                _ => "an array".to_owned(),
            },
        ),
        _ => "something else".to_owned(),
    }
}
