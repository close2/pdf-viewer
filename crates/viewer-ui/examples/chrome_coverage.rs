//! How much of what a document says about *itself* this program's own font can set.
//!
//! `doc/todo/27` named a silence: [`viewer_ui::chrome::Chrome::text`] used to draw nothing for a
//! character §9.6.2.2's Helvetica cannot set and [`viewer_ui::chrome::Chrome::width`] gave it no
//! advance, so a panel row whose text is Japanese was an empty row. It is a box now (ADR 0195),
//! and this counts them — over §12.3.3's outline titles, §8.11.4.3's layer names, §7.11.4's file
//! names, §14.3.3's `/Info` values and §14.3.2's XMP properties, all of which were unmeasured
//! before the three-hundred-and-sixteenth session.
//!
//! This is that measurement, over whatever documents are named on the command line (the corpus,
//! usually). It opens each one through `viewer-core` and asks the four queries the sidebar asks,
//! so what it counts is the strings this host actually draws rather than every string in the
//! file.
//!
//! **It asks `Chrome`, which is what makes it this host's question rather than the binary's.**
//! `pdf-model --example interface_font_census` is the other one: what *the compiled-in faces*
//! state, asked of `pdf-font` directly, so that a change to how `Chrome` reaches a glyph is
//! measured by something outside it (ADR 0326).
//!
//! ```sh
//! cargo run --release -p viewer-ui --example chrome_coverage -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use viewer_core::{Answer, Command, DocumentId, Layer, Query, Viewer};
use viewer_ui::chrome::{Chrome, Style};

/// One population of strings the sidebar draws, and what the interface's face can set of it.
#[derive(Default)]
struct Tally {
    /// How many documents state at least one string in this population.
    documents: usize,
    /// How many of those state a string with a character the face has no code for.
    documents_short: usize,
    /// How many strings in total.
    strings: usize,
    /// How many strings lose at least one character.
    strings_short: usize,
    /// How many strings lose *every* non-blank character, which is a row drawn as nothing.
    strings_empty: usize,
    /// How many characters are lost altogether.
    characters: usize,
}

impl Tally {
    /// Adds one string, and says whether it lost anything.
    fn add(&mut self, chrome: &Chrome, text: &str) -> bool {
        self.strings = self.strings.saturating_add(1);
        let missing = chrome.without_a_code(text, Style::default());
        if missing == 0 {
            return false;
        }
        self.strings_short = self.strings_short.saturating_add(1);
        self.characters = self.characters.saturating_add(missing);
        let set = text
            .chars()
            .filter(|character| !character.is_whitespace())
            .count()
            .saturating_sub(missing);
        if set == 0 {
            self.strings_empty = self.strings_empty.saturating_add(1);
        }
        true
    }
}

/// The four tabs' populations, in the order the sidebar shows them.
const POPULATIONS: [&str; 5] = [
    "§12.3.3 outline titles",
    "§8.11.4.3 layer names",
    "§7.11.4 file names and descriptions",
    "§14.3.3 /Info values",
    "§14.3.2 XMP properties",
];

/// The five populations of one document, in [`POPULATIONS`]' order.
///
/// Everything the sidebar draws from the document itself, taken through the same queries the
/// window asks — so a string that is not here is a string this host does not show.
fn strings_of(viewer: &Viewer) -> [Vec<String>; 5] {
    let mut strings: [Vec<String>; 5] = Default::default();
    if let Answer::Outline(outline) = viewer.query(Query::Outline) {
        let mut stack: Vec<&pdf_model::outline::Item> = outline.items.iter().rev().collect();
        while let Some(item) = stack.pop() {
            strings[0].push(item.title.clone());
            stack.extend(item.children.iter().rev());
        }
    }
    if let Answer::Layers(layers) = viewer.query(Query::Layers) {
        let mut stack: Vec<Layer> = layers.into_iter().rev().collect();
        while let Some(layer) = stack.pop() {
            match layer {
                Layer::Group { name, .. } => strings[1].extend(name),
                Layer::Collection { label, children } => {
                    strings[1].extend(label);
                    stack.extend(children.into_iter().rev());
                }
            }
        }
    }
    if let Answer::Attachments(files) = viewer.query(Query::Attachments) {
        for file in files {
            strings[2].push(file.name);
            strings[2].extend(file.file_name);
            strings[2].extend(file.description);
        }
    }
    if let Answer::Properties {
        information,
        metadata,
    } = viewer.query(Query::Properties)
    {
        strings[3].extend(
            [
                information.title,
                information.author,
                information.subject,
                information.keywords,
                information.creator,
                information.producer,
            ]
            .into_iter()
            .flatten(),
        );
        // The seven the properties panel labels, and not every property in the packet: what is
        // being measured is the text this host draws.
        if let Some(Ok(xmp)) = metadata {
            strings[4].extend(xmp.title().map(str::to_owned));
            strings[4].extend(xmp.authors().map(|authors| authors.join(", ")));
            strings[4].extend(xmp.description().map(str::to_owned));
            strings[4].extend(xmp.producer().map(str::to_owned));
            strings[4].extend(xmp.creator_tool().map(str::to_owned));
            strings[4].extend(xmp.created().map(str::to_owned));
            strings[4].extend(xmp.modified().map(str::to_owned));
        }
    }
    strings
}

/// Opens one document with no viewport at all, so that nothing is interpreted or rasterised.
fn open(bytes: Vec<u8>) -> Option<Viewer> {
    let mut viewer = Viewer::new(0, 0, 1.0);
    let opened = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes,
            password: None,
            fragment: None,
        })
        .any(|event| matches!(event, viewer_core::Event::Opened { .. }));
    opened.then_some(viewer)
}

fn main() {
    let chrome = match Chrome::new() {
        Ok(chrome) => chrome,
        Err(error) => {
            println!("the interface's own faces did not load: {error}");
            return;
        }
    };
    let mut tallies: BTreeMap<&str, Tally> = BTreeMap::new();
    let mut opened = 0_usize;
    let mut worst: Vec<(usize, String, String)> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Some(viewer) = open(bytes) else { continue };
        opened = opened.saturating_add(1);

        for (population, stated) in POPULATIONS.into_iter().zip(strings_of(&viewer)) {
            if stated.is_empty() {
                continue;
            }
            let tally = tallies.entry(population).or_default();
            tally.documents = tally.documents.saturating_add(1);
            let mut short = false;
            for text in stated {
                if tally.add(&chrome, &text) {
                    short = true;
                    let missing = chrome.without_a_code(&text, Style::default());
                    worst.push((missing, path.clone(), text));
                }
            }
            if short {
                tally.documents_short = tally.documents_short.saturating_add(1);
            }
        }
    }

    println!("{opened} document(s) opened\n");
    println!(
        "{:<38} {:>5} {:>7} {:>7} {:>7} {:>6} {:>7}",
        "population", "docs", "short", "strings", "short", "empty", "chars"
    );
    for (population, tally) in &tallies {
        println!(
            "{population:<38} {:>5} {:>7} {:>7} {:>7} {:>6} {:>7}",
            tally.documents,
            tally.documents_short,
            tally.strings,
            tally.strings_short,
            tally.strings_empty,
            tally.characters
        );
    }

    worst.sort_by_key(|(missing, _, _)| std::cmp::Reverse(*missing));
    println!("\nthe ten strings losing most:");
    for (missing, path, text) in worst.iter().take(10) {
        let stem = path.rsplit('/').next().unwrap_or(path);
        let shown: String = text.chars().take(40).collect();
        println!(
            "  {missing:>4} of {:<4} {stem:<40} {shown}",
            text.chars().count()
        );
    }
}
