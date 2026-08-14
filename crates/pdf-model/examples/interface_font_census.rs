//! Which characters a program's *own* text needs, and which of them §9.6.2.2's fourteen state.
//!
//! `doc/todo/27` left one question open: an interface drawn in the compiled-in standard fonts
//! (ADR 0133) draws a box for a character it cannot set (ADR 0195), and what is owed is
//! **coverage**. The file priced three answers — a face from the machine, a face compiled in, or
//! the platform's own text stack in a native host — and every one of them costs something.
//!
//! None of the three can be chosen without knowing *which characters* real documents actually ask
//! an interface to set, and that had never been counted. This counts it, and it asks each
//! character two questions rather than one:
//!
//! - does a **character code** reach it — [`pdf_font::LoadedFont::code_for`], which is the route
//!   every panel used before the four-hundred-and-ninety-first session;
//! - does the **face** state a glyph for it at all — [`pdf_font::LoadedFont::character_glyph`],
//!   which is the same compiled-in bytes asked by character instead.
//!
//! The gap between the two is not a font question and not a licence question: a simple font's
//! encoding is 256 codes wide (§9.6.5), and an interface's text has no codes at all.
//!
//! **§12.7's field labels are deliberately not a population here.** A form's controls are drawn
//! by the host — `viewer-gtk` and `viewer-qt` put Table 226's `/TU` in a native widget with the
//! platform's own font stack — and the one host that would set a label in this face draws the
//! field from the document's own appearance stream instead. A label nothing sets in Helvetica is
//! not evidence about Helvetica.
//!
//! **Deliberately not asked through `viewer_ui::chrome`**, which is the code under test:
//! `viewer-ui --example chrome_coverage` measures what that host's own `Chrome` can set, and a
//! census whose predicate is the thing being checked measures the instrument (`doc/HANDOVER.md`
//! trap 8). Everything here goes to `pdf-font` and `pdf-model` directly.
//!
//! ```sh
//! cargo run --release -p pdf-model --example interface_font_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_font::LoadedFont;
use pdf_syntax::Document;

/// How many pages of one document are walked for popups, so a thousand-page file cannot dominate.
///
/// The same bound `markup_text_census` takes, for the same reason: a window is opened by a click
/// and a click can be on page 40, but a census is not a corpus gate.
const MAX_PAGES: usize = 200;

/// What one character costs an interface that draws it in one of §9.6.2.2's fourteen.
///
/// The four cases `viewer_ui::chrome::Chrome::set` decides between, named here so that the census
/// and the interface cannot drift apart in what they mean by "missing".
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Reach {
    /// A character code names it: the route a panel has always had.
    Coded,
    /// No code names it and the face's own `cmap` states a glyph anyway.
    Faced,
    /// Whitespace or a control character with neither: nothing is drawn, and nothing is missing.
    Blank,
    /// The face has no glyph for it. This is a box on the screen, and it is the population any
    /// answer to `doc/todo/27` has to cover.
    Absent,
}

/// Which of the two routes into the compiled-in face reaches a character, if either.
fn reach(face: &LoadedFont, character: char) -> Reach {
    if face.code_for(character).is_some() {
        Reach::Coded
    } else if face.character_glyph(character).is_some() {
        Reach::Faced
    } else if character.is_whitespace() || character.is_control() {
        Reach::Blank
    } else {
        Reach::Absent
    }
}

/// One population of strings an interface draws, and how much of it the binary can set.
#[derive(Default)]
struct Tally {
    /// Documents stating at least one string in this population.
    documents: usize,
    /// Documents that lost a character before the second route existed: the `short` column.
    documents_short_by_code: usize,
    /// Documents that still lose one after both routes are tried: the `boxed` column.
    documents_absent: usize,
    strings: usize,
    characters: usize,
    coded: usize,
    faced: usize,
    blank: usize,
    absent: usize,
}

impl Tally {
    /// Adds one string, and reports what it lost by each route.
    fn add(&mut self, face: &LoadedFont, text: &str) -> (bool, bool) {
        self.strings = self.strings.saturating_add(1);
        let (mut uncoded, mut absent) = (false, false);
        for character in text.chars() {
            self.characters = self.characters.saturating_add(1);
            match reach(face, character) {
                Reach::Coded => self.coded = self.coded.saturating_add(1),
                Reach::Faced => {
                    self.faced = self.faced.saturating_add(1);
                    uncoded = true;
                }
                Reach::Blank => self.blank = self.blank.saturating_add(1),
                Reach::Absent => {
                    self.absent = self.absent.saturating_add(1);
                    uncoded = true;
                    absent = true;
                }
            }
        }
        (uncoded, absent)
    }
}

/// The populations, in the order an interface meets them.
const POPULATIONS: [&str; 7] = [
    "§12.3.3 outline titles",
    "§8.11.4.3 layer names",
    "§7.11.4 attachment names",
    "§14.3.3 /Info values",
    "§14.3.2 XMP properties",
    "§12.4.2 page labels",
    "§12.5.6.14 popup text",
];

/// Everything one document asks an interface to set, in [`POPULATIONS`]' order.
fn strings_of(document: &Document) -> [Vec<String>; 7] {
    let mut strings: [Vec<String>; 7] = Default::default();
    let pages = pdf_model::Pages::new(document);

    let outline = pdf_model::outline::Outline::read(document, &pages);
    let mut stack: Vec<&pdf_model::outline::Item> = outline.items.iter().rev().collect();
    while let Some(item) = stack.pop() {
        strings[0].push(item.title.clone());
        stack.extend(item.children.iter().rev());
    }

    if let Some(content) = pdf_model::optional_content::OptionalContent::read(document) {
        let mut stack: Vec<&pdf_model::optional_content::Presented> =
            content.presentation().iter().rev().collect();
        while let Some(presented) = stack.pop() {
            match presented {
                pdf_model::optional_content::Presented::Group(group) => {
                    strings[1].extend(content.name(document, *group));
                }
                pdf_model::optional_content::Presented::Collection { label, children } => {
                    strings[1].extend(label.clone());
                    stack.extend(children.iter().rev());
                }
            }
        }
    }

    for file in pdf_model::attachment::attachments(document) {
        strings[2].push(file.name);
        strings[2].extend(file.file_name);
        strings[2].extend(file.description);
    }

    let information = pdf_model::metadata::Information::read(document);
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

    if let Some(Ok(xmp)) = pdf_model::xmp::Xmp::document(document) {
        strings[4].extend(xmp.title().map(str::to_owned));
        strings[4].extend(xmp.authors().map(|authors| authors.join(", ")));
        strings[4].extend(xmp.description().map(str::to_owned));
        strings[4].extend(xmp.producer().map(str::to_owned));
        strings[4].extend(xmp.creator_tool().map(str::to_owned));
    }

    let labels = pdf_model::page_label::PageLabels::read(document);
    for index in 0..pages.len().min(MAX_PAGES) {
        strings[5].extend(labels.label(index));
    }

    let view = pdf_model::view::ViewState::of(document);
    for index in 0..pages.len().min(MAX_PAGES) {
        let Some(page) = pages.get(index) else {
            continue;
        };
        for popup in pdf_model::popup::popups(document, &page, &view) {
            strings[6].extend(popup.title);
            strings[6].extend(popup.text);
        }
    }

    strings
}

/// A coarse name for where a character lives, so that a population reads as a language rather
/// than as a list of code points.
///
/// Ranges rather than Unicode's own block table: what a reader of this census needs is "Cyrillic"
/// and "CJK", and a full block table would be a data resource with a provenance to record for a
/// label on a report. Anything outside them prints as its code point.
const BLOCKS: [(u32, u32, &str); 24] = [
    (0x0000, 0x007F, "Basic Latin"),
    (0x0080, 0x00FF, "Latin-1 Supplement"),
    (0x0100, 0x017F, "Latin Extended-A"),
    (0x0180, 0x024F, "Latin Extended-B"),
    (0x0250, 0x02FF, "phonetic and modifier letters"),
    (0x0300, 0x036F, "combining marks"),
    (0x0370, 0x03FF, "Greek"),
    (0x0400, 0x04FF, "Cyrillic"),
    (0x0590, 0x05FF, "Hebrew"),
    (0x0600, 0x06FF, "Arabic"),
    (0x0900, 0x097F, "Devanagari"),
    (0x0E00, 0x0E7F, "Thai"),
    (0x1E00, 0x1EFF, "Latin Extended Additional"),
    (0x2000, 0x206F, "general punctuation"),
    (0x2070, 0x209F, "sub- and superscripts"),
    (0x20A0, 0x20CF, "currency"),
    (0x2100, 0x214F, "letterlike symbols"),
    (0x2190, 0x21FF, "arrows"),
    (0x2200, 0x22FF, "mathematical operators"),
    (0x2500, 0x25FF, "box drawing and geometric shapes"),
    (0x2600, 0x27BF, "miscellaneous symbols and dingbats"),
    (0x3000, 0x30FF, "Japanese kana and CJK punctuation"),
    (0x4E00, 0x9FFF, "CJK unified ideographs"),
    (0xAC00, 0xD7AF, "Hangul"),
];

/// Which of [`BLOCKS`] a character is in.
fn block_of(character: char) -> String {
    let point = u32::from(character);
    if character == char::REPLACEMENT_CHARACTER {
        // §7.9.2.2's undefined code point, which is a *report* about the file rather than a
        // script the interface is short of: `bug1146106.pdf` writes its text strings as UTF-16
        // little-endian and every second byte becomes one (ADR 0195).
        return "U+FFFD, Table D.3's undefined code point".to_owned();
    }
    BLOCKS
        .iter()
        .find(|(first, last, _)| (*first..=*last).contains(&point))
        .map_or_else(
            || format!("U+{point:04X}"),
            |(_, _, name)| (*name).to_owned(),
        )
}

/// Everything the walk accumulates, so that [`main`] walks and prints separately.
#[derive(Default)]
struct Census {
    opened: usize,
    tallies: [Tally; 7],
    /// Documents losing a character by the code route, which is what a panel used to lose.
    short_by_code: usize,
    /// Documents still losing one once the face itself is asked.
    absent: usize,
    /// Where the characters the face route recovered live.
    recovered: BTreeMap<String, usize>,
    /// Where the characters still drawn as boxes live.
    remaining: BTreeMap<String, usize>,
    /// The strings losing most, for the tail of the report.
    worst: Vec<(usize, String, String)>,
    /// How many characters each document still loses, so that the remaining demand is a list of
    /// files rather than a count — which is what a decision about compiling in another face needs.
    by_document: BTreeMap<String, usize>,
}

impl Census {
    /// Walks one document's seven populations into the counters.
    fn count(&mut self, face: &LoadedFont, path: &str, document: &Document) {
        self.opened = self.opened.saturating_add(1);
        let (mut any_uncoded, mut any_absent) = (false, false);

        for (tally, stated) in self.tallies.iter_mut().zip(strings_of(document)) {
            if stated.is_empty() {
                continue;
            }
            tally.documents = tally.documents.saturating_add(1);
            let (mut uncoded_here, mut absent_here) = (false, false);
            for text in stated {
                let (uncoded, absent) = tally.add(face, &text);
                uncoded_here |= uncoded;
                absent_here |= absent;
                for character in text.chars() {
                    let block = match reach(face, character) {
                        Reach::Faced => &mut self.recovered,
                        Reach::Absent => &mut self.remaining,
                        Reach::Coded | Reach::Blank => continue,
                    }
                    .entry(block_of(character))
                    .or_default();
                    *block = block.saturating_add(1);
                }
                if absent {
                    let missing = text
                        .chars()
                        .filter(|character| reach(face, *character) == Reach::Absent)
                        .count();
                    let stem = path.rsplit('/').next().unwrap_or(path).to_owned();
                    let document = self.by_document.entry(stem.clone()).or_default();
                    *document = document.saturating_add(missing);
                    self.worst.push((missing, stem, text));
                }
            }
            if uncoded_here {
                tally.documents_short_by_code = tally.documents_short_by_code.saturating_add(1);
            }
            if absent_here {
                tally.documents_absent = tally.documents_absent.saturating_add(1);
            }
            any_uncoded |= uncoded_here;
            any_absent |= absent_here;
        }
        if any_uncoded {
            self.short_by_code = self.short_by_code.saturating_add(1);
        }
        if any_absent {
            self.absent = self.absent.saturating_add(1);
        }
    }

    /// Prints one block table, commonest first.
    fn print_blocks(blocks: &BTreeMap<String, usize>, heading: &str) {
        let mut sorted: Vec<(&String, &usize)> = blocks.iter().collect();
        sorted.sort_by_key(|(name, count)| (std::cmp::Reverse(**count), (*name).clone()));
        println!("\n{heading}");
        for (block, count) in sorted {
            println!("  {count:>6}  {block}");
        }
    }
}

fn main() {
    let face = match LoadedFont::standard("Helvetica") {
        Ok(face) => face,
        Err(error) => {
            println!("the compiled-in Helvetica did not load: {error}");
            return;
        }
    };

    let mut census = Census::default();
    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        census.count(&face, &path, &document);
    }
    let Census {
        opened,
        tallies,
        short_by_code,
        absent: documents_absent,
        recovered,
        remaining,
        mut worst,
        by_document,
    } = census;

    // What the two routes reach at all, over every code point below the astral planes: the
    // denominator behind every row below, and the whole of the finding in two numbers.
    let addressable = |reachable: fn(&LoadedFont, char) -> bool| {
        (0..=0x2_FFFF_u32)
            .filter_map(char::from_u32)
            .filter(|character| reachable(&face, *character))
            .count()
    };
    println!(
        "the compiled-in Helvetica: {} characters by code, {} by character\n",
        addressable(|face, character| face.code_for(character).is_some()),
        addressable(|face, character| face.character_glyph(character).is_some()),
    );

    println!("{opened} document(s) opened\n");
    println!(
        "{:<34} {:>5} {:>6} {:>6} {:>7} {:>9} {:>7} {:>6} {:>6} {:>7}",
        "population",
        "docs",
        "short",
        "boxed",
        "strings",
        "chars",
        "coded",
        "faced",
        "blank",
        "absent"
    );
    for (population, tally) in POPULATIONS.into_iter().zip(&tallies) {
        println!(
            "{population:<34} {:>5} {:>6} {:>6} {:>7} {:>9} {:>7} {:>6} {:>6} {:>7}",
            tally.documents,
            tally.documents_short_by_code,
            tally.documents_absent,
            tally.strings,
            tally.characters,
            tally.coded,
            tally.faced,
            tally.blank,
            tally.absent
        );
    }
    println!(
        "\ndocuments losing a character by the code route: {short_by_code}\
         \ndocuments still losing one by the face:         {documents_absent}"
    );
    println!(
        "documents whose panels the face route completes: {}",
        short_by_code.saturating_sub(documents_absent)
    );

    Census::print_blocks(&by_document, "every document that still loses a character:");
    Census::print_blocks(&recovered, "what the face route recovered, by block:");
    Census::print_blocks(
        &remaining,
        "what is still a box, by block — the demand any further answer has to cover:",
    );

    worst.sort_by_key(|(missing, stem, _)| (std::cmp::Reverse(*missing), stem.clone()));
    println!("\nthe ten strings still losing most:");
    for (missing, stem, text) in worst.iter().take(10) {
        let shown: String = text.chars().take(36).collect();
        println!(
            "  {missing:>4} of {:<4} {stem:<40} {shown}",
            text.chars().count()
        );
    }
}
