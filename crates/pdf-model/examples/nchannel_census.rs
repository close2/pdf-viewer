//! How many documents state an `NChannel` colour space, and how many of those name a spot
//! colourant.
//!
//! ISO 32000-2 §8.6.6.5 gives a `DeviceN` space an optional attributes dictionary (Table 70)
//! whose `/Subtype` of `NChannel` turns two of its entries into requirements:
//!
//! > Colorants … ( Required if Subtype is NChannel and the colour space includes spot
//! > colourants; otherwise optional; PDF 1.6 )
//!
//! > Process … ( Required if Subtype is NChannel and the colour space includes components of a
//! > process colour space, otherwise optional; PDF 1.6 )
//!
//! and whose per-component rule — "[f]or NChannel colour spaces, the components shall be
//! evaluated individually; that is, only the ones not present on the output device shall use
//! the alternate colour space of that component" — divides on exactly that line. On a display
//! no colourant is present, so a process component takes the process dictionary's own
//! `/ColorSpace` and a spot component takes its `/Colorants` `Separation`; what the clause
//! never states is how the two are then combined, which NOTE 3 hands to the processor. So the
//! population this census separates is the one the implementation divides on: a space with no
//! spot colourant needs no combination and is complete on the clause's own terms.
//!
//! Which components are process ones is Table 71's `/Components` plus one sentence of the
//! clause, and both are applied here:
//!
//! > The reserved names Cyan , Magenta , Yellow , and Black shall always be considered to be
//! > process colours, which do not necessarily correspond to the colourants of a specific
//! > device; they need not have entries in the process dictionary.
//!
//! > Any component not specified in the process dictionary shall be considered to be a spot
//! > colourant.
//!
//! ```sh
//! cargo run --release -p pdf-model --example nchannel_census -- @paths.txt
//! ```
//!
//! **Every object the cross-reference table names is scanned**, rather than the pages'
//! resources, for `black_generation_census`' reason: a colour space reached only by a pattern,
//! an annotation appearance or an earlier revision is stated just as much as one a page names,
//! and a walk that starts from the page tree is the shape of false zero this tree has produced
//! twice.
#![expect(
    clippy::doc_markdown,
    reason = "the comment quotes §8.6.6.5 and Table 70 verbatim, and a quotation is not marked up"
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

/// The four names §8.6.6.5 reserves to the subtractive process colourants of a CMYK device.
const RESERVED: [&[u8]; 4] = [b"Cyan", b"Magenta", b"Yellow", b"Black"];

/// What one document said about §8.6.6.5's attributes dictionary.
#[derive(Default)]
struct Says {
    /// The document opened.
    opened: bool,
    /// Every question, with how many *spaces* in this document answered it yes.
    counts: BTreeMap<&'static str, usize>,
    /// Every process `/ColorSpace` family seen, with its count.
    families: BTreeMap<String, usize>,
}

impl Says {
    /// Records one space answering `question`.
    fn saw(&mut self, question: &'static str) {
        let counter = self.counts.entry(question).or_default();
        *counter = counter.saturating_add(1);
    }
}

/// The questions, in the order they are printed.
const QUESTIONS: [&str; 11] = [
    "a /DeviceN space at all",
    "with an attributes dictionary",
    "whose /Subtype is /NChannel",
    "  with a /Process dictionary",
    "  with a /Colorants dictionary",
    "  with a /MixingHints dictionary",
    "  **every component a process one**",
    "    every /Components name present in the names array",
    "    omitting a component, the process space named /DeviceCMYK",
    "    omitting a component, the process space named otherwise",
    "  at least one spot component",
];

fn main() {
    let paths = paths();
    let said: Vec<Says> = paths.par_iter().map(|path| read(path)).collect();

    let mut documents = 0_usize;
    let mut spaces: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut holders: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut families: BTreeMap<String, usize> = BTreeMap::new();

    for says in said {
        if !says.opened {
            continue;
        }
        documents = documents.saturating_add(1);
        for (question, count) in says.counts {
            let total = spaces.entry(question).or_default();
            *total = total.saturating_add(count);
            let holder = holders.entry(question).or_default();
            *holder = holder.saturating_add(1);
        }
        for (family, count) in says.families {
            let total = families.entry(family).or_default();
            *total = total.saturating_add(count);
        }
    }

    println!("{documents} document(s) opened");
    for question in QUESTIONS {
        let count = spaces.get(question).copied().unwrap_or_default();
        let holder = holders.get(question).copied().unwrap_or_default();
        println!("  {holder} document(s), {count} space(s): {question}");
    }
    println!("  process /ColorSpace families: {families:?}");
}

/// The paths to walk: the arguments, or the lines of the file an `@name` argument names.
fn paths() -> Vec<String> {
    let mut paths = Vec::new();
    for argument in std::env::args().skip(1) {
        if let Some(name) = argument.strip_prefix('@') {
            match std::fs::read_to_string(name) {
                Ok(listing) => paths.extend(
                    listing
                        .lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .map(ToOwned::to_owned),
                ),
                Err(error) => println!("  {name}: {error}"),
            }
        } else {
            paths.push(argument);
        }
    }
    paths
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

/// One object's dictionaries and arrays, to [`MAX_DEPTH`], counting each `DeviceN` array.
///
/// References are not followed: every object the table names is scanned in its own right, so
/// following one would only count it twice.
fn descend(document: &Document, object: &Object, depth: usize, says: &mut Says) {
    if depth > MAX_DEPTH {
        return;
    }
    match object {
        Object::Dictionary(dict) => {
            for (_, value) in dict.iter() {
                descend(document, value, depth.saturating_add(1), says);
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter() {
                descend(document, value, depth.saturating_add(1), says);
            }
        }
        Object::Array(items) => {
            if items
                .first()
                .and_then(Object::as_name)
                .is_some_and(|name| name.as_bytes() == b"DeviceN")
            {
                count(document, items, says);
            }
            for item in items {
                descend(document, item, depth.saturating_add(1), says);
            }
        }
        _ => {}
    }
}

/// One `DeviceN` array: §8.6.6.5's four or five elements.
fn count(document: &Document, items: &[Object], says: &mut Says) {
    says.saw(QUESTIONS[0]);
    let names: Vec<Vec<u8>> = items
        .get(1)
        .map(|object| document.resolve(object))
        .and_then(|object| {
            object.as_array().map(|entries| {
                entries
                    .iter()
                    .map(|entry| document.resolve(entry))
                    .filter_map(|entry| entry.as_name().map(|name| name.as_bytes().to_vec()))
                    .collect()
            })
        })
        .unwrap_or_default();
    let attributes = items.get(4).map(|object| document.resolve(object));
    let Some(attributes) = attributes.as_ref().and_then(Object::as_dict) else {
        return;
    };
    says.saw(QUESTIONS[1]);
    if document
        .get_key(attributes, "Subtype")
        .as_name()
        .is_none_or(|subtype| subtype.as_bytes() != b"NChannel")
    {
        return;
    }
    says.saw(QUESTIONS[2]);

    let process = document.get_key(attributes, "Process");
    let process = process.as_dict();
    if process.is_some() {
        says.saw(QUESTIONS[3]);
    }
    if document
        .get_key(attributes, "Colorants")
        .as_dict()
        .is_some()
    {
        says.saw(QUESTIONS[4]);
    }
    if document
        .get_key(attributes, "MixingHints")
        .as_dict()
        .is_some()
    {
        says.saw(QUESTIONS[5]);
    }

    let components: Vec<Vec<u8>> = process
        .map(|process| document.get_key(process, "Components"))
        .and_then(|object| {
            object.as_array().map(|entries| {
                entries
                    .iter()
                    .map(|entry| document.resolve(entry))
                    .filter_map(|entry| entry.as_name().map(|name| name.as_bytes().to_vec()))
                    .collect()
            })
        })
        .unwrap_or_default();
    let space = process.map(|process| document.get_key(process, "ColorSpace"));
    let family = space.as_ref().map_or_else(
        || "(none)".to_owned(),
        |space| match space {
            Object::Name(name) => format!("/{}", String::from_utf8_lossy(name.as_bytes())),
            Object::Array(items) => items.first().and_then(Object::as_name).map_or_else(
                || "(array)".to_owned(),
                |name| format!("[/{}]", String::from_utf8_lossy(name.as_bytes())),
            ),
            _ => "(other)".to_owned(),
        },
    );
    let counter = says.families.entry(family).or_default();
    *counter = counter.saturating_add(1);

    // §8.6.6.5's reserved four are process colours wherever the process space is a CMYK one,
    // which is the only space they can be the colourants of: "[t]he reserved names Cyan ,
    // Magenta , Yellow , and Black shall always be considered to be process colours … they
    // need not have entries in the process dictionary."
    let cmyk = space.as_ref().is_some_and(|space| {
        space
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"DeviceCMYK")
            || components.len() == 4
    });
    let is_process =
        |name: &Vec<u8>| components.contains(name) || (cmyk && RESERVED.contains(&name.as_slice()));
    if names.is_empty() {
        return;
    }
    if names.iter().all(is_process) {
        says.saw(QUESTIONS[6]);
        if components.iter().all(|component| names.contains(component)) {
            says.saw(QUESTIONS[7]);
        } else if space
            .as_ref()
            .and_then(Object::as_name)
            .is_some_and(|name| name.as_bytes() == b"DeviceCMYK")
        {
            // §8.6.6.5 admits a subset for a CMYK colour space alone, and §8.6.4.4 is what
            // then says what an omitted colourant is worth.
            says.saw(QUESTIONS[8]);
        } else {
            says.saw(QUESTIONS[9]);
        }
    } else {
        says.saw(QUESTIONS[10]);
    }
}
