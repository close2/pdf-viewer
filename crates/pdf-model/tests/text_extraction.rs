//! Comparing the text we draw against an independent extractor.
//!
//! This is the check that a character code reaches the *right* glyph, across the whole
//! corpus at once. Every other check on text is structural — that a font loaded, that a
//! code resolved, that widths agree — and all of them pass while a font draws confident,
//! wrong letters. Reading the page back and comparing it against `pdftotext` is the only
//! test here that would notice.
//!
//! It works because the text comes from the *drawing* pass: `Interpretation::text` is
//! accumulated by the same loop that places the glyphs, from the same code-to-glyph
//! decisions. It is a readback of what was rendered, not a second pipeline.
//!
//! # What is compared, and what is deliberately not
//!
//! Each word `pdftotext` found must appear in our text *with all whitespace removed*.
//!
//! Word boundaries are deliberately not compared, because a content stream does not
//! record them. It records positions, and a producer is free to place every glyph
//! individually — several corpus documents do, with gaps inside a word wider than the
//! font's own space. Recovering words from that is layout analysis, which `pdftotext`
//! performs and this crate does not; scoring it here would measure a heuristic rather
//! than the renderer.
//!
//! What a content stream *does* determine, exactly, is which glyph each code selects. So
//! the comparison strips whitespace from both sides and asks whether the reference's words
//! still appear. A wrong mapping breaks those substrings immediately; a debatable space
//! does not.

#![expect(
    clippy::print_stdout,
    reason = "test code: the measurements are the point of the exercise"
)]
#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    reason = "test code: counters are bounded by one page's words, and a missing corpus \
              file should stop the run loudly"
)]

use std::collections::BTreeMap;

use pdf_syntax::Document;

/// How the comparison scores one page.
struct Score {
    /// Words `pdftotext` found that we also produced.
    matched: usize,
    /// Words `pdftotext` found in total.
    total: usize,
    /// A few words we failed to produce, for diagnosis.
    missing: Vec<String>,
}

impl Score {
    fn ratio(&self) -> f64 {
        if self.total == 0 {
            return 1.0;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "word counts on one page are far below f64's exact integer limit"
        )]
        {
            self.matched as f64 / self.total as f64
        }
    }
}

/// Folds the characters extractors legitimately disagree about.
///
/// Case is kept — a mapping that swaps case is a real defect — but the dashes, quotes and
/// spaces that producers spell inconsistently in `/ToUnicode` are normalised, because
/// which codepoint was meant by them is a question about extraction conventions rather
/// than about which glyph was drawn.
fn fold(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\u{2018}' | '\u{2019}' | '\u{201B}' => '\'',
            '\u{201C}' | '\u{201D}' => '"',
            '\u{2010}'..='\u{2015}' | '\u{2212}' => '-',
            '\u{00A0}' | '\u{2007}' | '\u{202F}' => ' ',
            other => other,
        })
        .collect()
}

/// The words a reference extraction contains, as units to look for.
///
/// Very short words are dropped: they occur so often that finding them in a
/// whitespace-stripped page proves nothing.
fn reference_words(text: &str) -> Vec<String> {
    fold(text)
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()).to_owned())
        .filter(|word| word.chars().count() >= 3)
        .collect()
}

/// Our text with every space removed, which is the form word boundaries cannot affect.
fn without_spaces(text: &str) -> String {
    fold(text).chars().filter(|c| !c.is_whitespace()).collect()
}

/// Runs `pdftotext` over one page.
fn reference_text(path: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("pdftotext")
        .args(["-f", "1", "-l", "1"])
        .arg(path)
        .arg("-")
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Scores one document's first page against `pdftotext`.
fn score(path: &std::path::Path) -> Option<Score> {
    let reference = reference_text(path)?;
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).ok()?;
    let page = pdf_model::Pages::new(&document).get(0)?;
    let ours = pdf_model::interpret(&document, &page).text;

    let haystack = without_spaces(&ours);
    let expected = reference_words(&reference);

    // Each *distinct* word must occur at least as often as the reference has it, so that
    // dropping every second glyph cannot score well by leaving one copy behind.
    let mut wanted: BTreeMap<&str, usize> = BTreeMap::new();
    for word in &expected {
        *wanted.entry(word.as_str()).or_default() += 1;
    }

    let mut matched = 0usize;
    let mut missing = Vec::new();
    for (word, wanted) in wanted {
        let found = haystack.matches(word).count();
        matched += found.min(wanted);
        if found < wanted && missing.len() < 8 {
            missing.push(format!("{word} ({found}/{wanted})"));
        }
    }

    Some(Score {
        matched,
        total: expected.len(),
        missing,
    })
}

fn corpus() -> Vec<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .expect("the corpus directory is readable")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    files.sort();
    files
}

/// The text we draw must be the text the page says.
///
/// The gate is a floor on a real measurement rather than a target. Every document in the
/// corpus currently scores 100%, and the figures are printed on every run, so a drop is a
/// regression to investigate even while it stays above the floor.
///
/// The gate is known to bite. Reverting the operand cap that this test originally found
/// takes the corpus to 93.2%; shifting every `/ToUnicode` entry by one code — a wrong
/// glyph rather than a missing one — takes it to 58.7%.
#[test]
fn the_text_we_draw_agrees_with_an_independent_extractor() {
    assert!(
        reference_text(&corpus()[0]).is_some(),
        "pdftotext is required for this test; it comes with poppler"
    );

    let mut worst = 1.0f64;
    let mut worst_file = String::new();
    let mut total_matched = 0usize;
    let mut total_words = 0usize;

    for path in corpus() {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let Some(score) = score(&path) else {
            panic!("{name}: could not be scored");
        };

        println!(
            "{name}: {:.1}% ({}/{} words){}",
            score.ratio() * 100.0,
            score.matched,
            score.total,
            if score.missing.is_empty() {
                String::new()
            } else {
                format!("  missing e.g. {:?}", score.missing)
            }
        );

        total_matched += score.matched;
        total_words += score.total;
        if score.ratio() < worst {
            worst = score.ratio();
            worst_file = name.into_owned();
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "corpus word counts are far below f64's exact integer limit"
    )]
    let overall = total_matched as f64 / total_words as f64;
    println!(
        "overall: {:.1}% ({total_matched}/{total_words} words)",
        overall * 100.0
    );

    assert!(
        worst > 0.99,
        "{worst_file} matched only {:.1}% of the words pdftotext found",
        worst * 100.0
    );
}
