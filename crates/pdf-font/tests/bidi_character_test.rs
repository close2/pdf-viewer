//! Unicode Standard Annex #9 against the Unicode Character Database's own conformance file.
//!
//! `pdf_font::shaping` takes its bidirectional algorithm from Servo's `unicode-bidi` rather than
//! writing one (ADR 1413), and a dependency taken for a specification is a claim about that
//! specification until it is measured. `data/unicode/BidiCharacterTest.txt` is the measurement:
//! every line states a paragraph, the direction it is resolved under, the level each character
//! resolves to and the order the line is displayed in, and the file's own header puts that at
//! rule L2 inclusively — rules L3 and L4 are outside it, which is why the mirroring of rule L4
//! has a test of its own in `shaping.rs`.
//!
//! The file is read here, at test time, and never at run time: nothing in the binary parses it.

use std::path::Path;

use pdf_font::shaping::{Paragraphs, visual_order};

/// One line of the file, parsed.
struct Case {
    text: String,
    direction: u8,
    paragraph_level: u8,
    levels: Vec<Option<u8>>,
    order: Vec<usize>,
}

fn parse(line: &str) -> Option<Case> {
    let fields: Vec<&str> = line.split(';').collect();
    let [points, direction, paragraph_level, levels, order] = fields.as_slice() else {
        return None;
    };
    let text = points
        .split_whitespace()
        .map(|point| u32::from_str_radix(point, 16).ok().and_then(char::from_u32))
        .collect::<Option<String>>()?;
    Some(Case {
        text,
        direction: direction.trim().parse().ok()?,
        paragraph_level: paragraph_level.trim().parse().ok()?,
        levels: levels
            .split_whitespace()
            .map(|level| level.parse().ok())
            .collect(),
        order: order
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?,
    })
}

/// What the algorithm resolves one case to: the paragraph level, a level per character and the
/// visual order of the characters rule X9 keeps.
///
/// Through `pdf_font::shaping` rather than straight into `unicode-bidi`, because rule L1 is
/// applied there, per line, and a line is what each case of the file is.
fn resolve(case: &Case) -> (u8, Vec<Option<u8>>, Vec<usize>) {
    let direction = match case.direction {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    };
    let text = case.text.as_str();
    let starts: Vec<usize> = text.char_indices().map(|(at, _)| at).collect();
    // Nothing resolving right to left leaves every character at level 0 in the stored order, and
    // the classes X9 reads are asked of `unicode-bidi` directly.
    let (paragraph, by_byte, removed) =
        if let Some(paragraphs) = Paragraphs::with_direction(text, direction) {
            let levels = paragraphs.line_levels(0..text.len());
            let removed: Vec<bool> = starts
                .iter()
                .map(|at| paragraphs.removed_by_x9(*at))
                .collect();
            (paragraphs.paragraph_level(0), levels, removed)
        } else {
            let info = unicode_bidi::BidiInfo::new(text, None);
            let removed = starts
                .iter()
                .map(|at| {
                    use unicode_bidi::BidiClass::{BN, LRE, LRO, PDF, RLE, RLO};
                    matches!(
                        info.original_classes.get(*at),
                        Some(RLE | LRE | RLO | LRO | PDF | BN)
                    )
                })
                .collect();
            (0, vec![0; text.len()], removed)
        };
    let per_character: Vec<u8> = starts.iter().map(|at| by_byte[*at]).collect();
    let resolved = per_character
        .iter()
        .zip(&removed)
        .map(|(level, removed)| (!removed).then_some(*level))
        .collect();
    let order = visual_order(&per_character)
        .into_iter()
        .filter(|index| !removed.get(*index).copied().unwrap_or(true))
        .collect();
    (paragraph, resolved, order)
}

#[test]
fn unicode_bidi_resolves_every_line_of_the_ucd_conformance_file() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/unicode/BidiCharacterTest.txt");
    let text =
        std::fs::read_to_string(&path).expect("data/unicode/BidiCharacterTest.txt is committed");
    let mut cases = 0_usize;
    let mut failures = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let case =
            parse(line).unwrap_or_else(|| panic!("line {} does not parse: {line}", number + 1));
        cases += 1;
        let (level, levels, order) = resolve(&case);
        if level != case.paragraph_level || levels != case.levels || order != case.order {
            failures.push((number + 1, level, levels, order));
        }
    }
    println!("{cases} cases, {} failures", failures.len());
    assert!(
        cases > 90_000,
        "the file holds the whole suite: {cases} cases"
    );
    assert!(
        failures.is_empty(),
        "{} of {cases} lines resolve otherwise than the file states, first at lines {:?}",
        failures.len(),
        failures.iter().take(5).collect::<Vec<_>>()
    );
}
