//! How many documents state §10.4.2.4's black generation or undercolour removal, and how many
//! state a *function* rather than naming the device's own.
//!
//! ISO 32000-2 §11.7.5.3 applies Table 57's `/BG`, `/BG2`, `/UCR` and `/UCR2` "only during
//! conversion from DeviceRGB to DeviceCMYK colour spaces", and §10.4.2.4 is the conversion they
//! are parameters of. §10.4.2.1 ranks that conversion below §10.3's, which is the branch this
//! tree converts into a press on — so what the entries cost a page is a report and not a colour
//! (ADR 1069, `Unsupported::BlackGeneration`). This census is the population that report can fire
//! on, and it is two numbers rather than one because §8.4.5's Table 57 admits a second kind of
//! value:
//!
//! > Same as BG except that the value may also be the name Default, denoting the black-generation
//! > function that was in effect at the start of the page.
//!
//! That function is the device's, so a state naming it departs from nothing. §10.4.2.4 says what
//! the other kind is — "[t]he black-generation and undercolour-removal functions shall be defined
//! as PDF function dictionaries (see 7.10, \"Functions\")" — so a dictionary or a stream is a
//! stated function and a name is not. Table 57's own precedence decides which of each pair is in
//! force: "[i]f both BG and BG2 are present in the same graphics state parameter dictionary, BG2
//! shall take precedence."
//!
//! ```sh
//! cargo run --release -p pdf-model --example black_generation_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! **Every object the cross-reference table names is scanned, rather than the pages' resources**,
//! for `transfer_function_census`' reason: an `/ExtGState` reached only by a pattern, an
//! annotation appearance or an earlier revision states the same parameters, and a walk that starts
//! from the page tree is the shape of false zero this tree has produced twice.
//!
//! **And only inside an `/ExtGState` subdictionary**, which is not fussiness: §8.4.5 puts Table
//! 57's parameters in "the `ExtGState` subdictionary of the current resource dictionary", and
//! Table 189 gives an annotation's appearance characteristics dictionary a `/BG` of its own — "an
//! array of numbers ... specifying the colour of the widget annotation's background". Counting
//! every `/BG` key in the file finds hundreds of those and not one of them is this clause's.
#![expect(
    clippy::doc_markdown,
    reason = "the comment quotes §11.7.5.3 verbatim, and a quotation is not marked up"
)]
#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};

use pdf_syntax::{Document, Object};

/// How deep one object's own dictionaries and arrays are descended.
const MAX_DEPTH: usize = 8;

/// What one document said about Table 57's four entries.
#[derive(Default)]
struct Says {
    /// The document opened.
    opened: bool,
    /// Some object states one of the four at all.
    any: bool,
    /// Some object states one as a function dictionary or stream, which is a departure.
    stated: bool,
    /// Every entry shape seen, `"/BG2 /Default"` and the like, with its count.
    shapes: BTreeMap<String, usize>,
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let said: Vec<(String, Says)> = paths
        .par_iter()
        .map(|path| {
            let name = path.rsplit('/').next().unwrap_or(path).to_owned();
            (name, read(path))
        })
        .collect();

    let mut documents = 0_usize;
    let mut any = 0_usize;
    let mut stated = 0_usize;
    let mut shapes: BTreeMap<String, usize> = BTreeMap::new();
    let mut naming: Vec<String> = Vec::new();
    let mut stating: Vec<String> = Vec::new();

    for (name, says) in said {
        if !says.opened {
            continue;
        }
        documents = documents.saturating_add(1);
        for (shape, count) in says.shapes {
            let counter = shapes.entry(shape).or_default();
            *counter = counter.saturating_add(count);
        }
        if says.any {
            any = any.saturating_add(1);
            naming.push(name.clone());
        }
        if says.stated {
            stated = stated.saturating_add(1);
            stating.push(name);
        }
    }

    println!("{documents} document(s) opened");
    println!("  {any} state a Table 57 /BG, /BG2, /UCR or /UCR2 anywhere the xref table names");
    println!("  **{stated} state one as a function, rather than naming the device's own**");
    println!("  values: {shapes:?}");
    if naming.len() <= 40 {
        println!("  stating one: {}", naming.join(" "));
    }
    if stating.len() <= 40 {
        println!("  stating a function: {}", stating.join(" "));
    }
}

/// One document read: every object the cross-reference table names, descended.
fn read(path: &str) -> Says {
    let Ok(bytes) = std::fs::read(path) else {
        return Says::default();
    };
    let Ok(document) = Document::open(bytes) else {
        return Says::default();
    };
    let mut says = Says {
        opened: true,
        ..Says::default()
    };
    for number in document.xref().object_numbers() {
        let object = document.get(pdf_syntax::ObjectId {
            number,
            generation: 0,
        });
        descend(&document, &object, 0, &mut says);
    }
    says
}

/// One object's dictionaries and arrays, to [`MAX_DEPTH`], counting inside each `/ExtGState`.
///
/// References are not followed: every object the table names is scanned in its own right, so
/// following one would only count it twice.
fn descend(document: &Document, object: &Object, depth: usize, says: &mut Says) {
    if depth > MAX_DEPTH {
        return;
    }
    let dict = match object {
        Object::Dictionary(dict) => dict,
        Object::Stream(stream) => &stream.dict,
        Object::Array(items) => {
            for item in items {
                descend(document, item, depth.saturating_add(1), says);
            }
            return;
        }
        _ => return,
    };
    for (key, value) in dict.iter() {
        if key.as_bytes() == b"ExtGState"
            && let Some(states) = document.get_key(dict, "ExtGState").as_dict()
        {
            for (_, state) in states.iter() {
                if let Some(state) = document.resolve(state).as_dict() {
                    count(document, state, says);
                }
            }
        }
        descend(document, value, depth.saturating_add(1), says);
    }
}

/// Table 57's four entries in one graphics state parameter dictionary.
fn count(document: &Document, state: &pdf_syntax::Dictionary, says: &mut Says) {
    for key in ["BG", "BG2", "UCR", "UCR2"] {
        let resolved = document.get_key(state, key);
        if resolved.is_null() {
            continue;
        }
        says.any = true;
        let shape = match &resolved {
            Object::Name(name) => format!("/{key} /{}", String::from_utf8_lossy(name.as_bytes())),
            Object::Dictionary(_) | Object::Stream(_) => format!("/{key} function"),
            other => format!("/{key} {other:?}"),
        };
        if matches!(resolved, Object::Dictionary(_) | Object::Stream(_)) {
            says.stated = true;
        }
        let counter = says.shapes.entry(shape).or_default();
        *counter = counter.saturating_add(1);
    }
}
