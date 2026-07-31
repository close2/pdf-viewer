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
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rayon::prelude::*;

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
///
/// Two of the foldings are **compatibility decompositions**, and they are here for the same
/// reason as the dashes rather than for convenience. A `/ToUnicode` that maps a ligature glyph
/// to U+FB01 is *correct* — it says what the glyph is — and one that maps it to `f` followed by
/// `i` is correct too, and says what the glyph reads as. Unicode itself records the pair as a
/// compatibility equivalence. `openoffice.pdf` says `Proﬁle` where `pdftotext` says `Profile`,
/// and neither is a wrong glyph. §14.9.4's `/ActualText` is what a producer writes when it
/// wants to settle it, and 5 corpus documents write one.
fn fold(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            match c {
                '\u{2018}' | '\u{2019}' | '\u{201B}' => "'",
                '\u{201C}' | '\u{201D}' => "\"",
                '\u{2010}'..='\u{2015}' | '\u{2212}' => "-",
                '\u{00A0}' | '\u{2007}' | '\u{202F}' => " ",
                // The Alphabetic Presentation Forms a Latin text font puts its ligatures in.
                '\u{FB00}' => "ff",
                '\u{FB01}' => "fi",
                '\u{FB02}' => "fl",
                '\u{FB03}' => "ffi",
                '\u{FB04}' => "ffl",
                '\u{FB05}' | '\u{FB06}' => "st",
                '\u{0132}' => "IJ",
                '\u{0133}' => "ij",
                '\u{0152}' => "OE",
                '\u{0153}' => "oe",
                _ => return std::iter::once(c).collect::<Vec<char>>(),
            }
            .chars()
            .collect::<Vec<char>>()
        })
        .collect()
}

/// Removes the hyphens a line break introduces, from both sides of the comparison.
///
/// A word broken across a line is written with a hyphen the word does not have —
/// `issue19120.pdf` draws `Trace-` and `Monkey` and means `TraceMonkey` — and **nothing in the
/// content stream says which hyphens those are**. §14.8.2.3 gives a *tagged* producer a way to
/// say so and this one has not used it. That is the same argument the module comment makes
/// about spaces, so it gets the same treatment: the character is removed from our text and from
/// the reference's alike, which leaves a genuinely hyphenated word matching itself and costs
/// only the ability to notice a hyphen that should not be there.
fn without_hyphens(text: &str) -> String {
    text.chars().filter(|c| *c != '-').collect()
}

/// The words a reference extraction contains, as units to look for.
///
/// Very short words are dropped: they occur so often that finding them in a
/// whitespace-stripped page proves nothing.
fn reference_words(text: &str) -> Vec<String> {
    fold(text)
        .split_whitespace()
        .map(|word| without_hyphens(word.trim_matches(|c: char| !c.is_alphanumeric())))
        .filter(|word| word.chars().count() >= 3)
        .collect()
}

/// Our text with every space removed, which is the form word boundaries cannot affect.
fn without_spaces(text: &str) -> String {
    without_hyphens(&fold(text))
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// How long `pdftotext` is given for one page before it is killed.
///
/// The pdf.js corpus holds files written to make a reader loop, and `Command::output` waits
/// forever — the same reason `pdfref::Reference::render_within` exists and the same budget it
/// uses. A document that times out is skipped and counted rather than hanging the suite.
const EXTRACTION_TIMEOUT: Duration = Duration::from_secs(30);

/// Runs `pdftotext` over one page, killing it if it outlives [`EXTRACTION_TIMEOUT`].
///
/// The output goes to a file rather than to a pipe: a poll loop over a child writing to a
/// pipe deadlocks the moment the pipe fills, which on a page of dense text it does.
fn reference_text(path: &Path) -> Option<String> {
    /// Short enough that a killed extractor does not hold up the run, long enough that
    /// polling costs nothing measurable.
    const POLL: Duration = Duration::from_millis(10);

    let out = std::env::temp_dir().join(format!(
        "pdfviewer-extract-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut child = std::process::Command::new("pdftotext")
        .args(["-f", "1", "-l", "1"])
        .arg(path)
        .arg(&out)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(_) => return None,
        }
        if started.elapsed() > EXTRACTION_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&out);
            return None;
        }
        std::thread::sleep(POLL);
    };
    let text = status
        .success()
        .then(|| std::fs::read_to_string(&out).ok())
        .flatten();
    let _ = std::fs::remove_file(&out);
    text
}

/// Scores one document's first page against `pdftotext`.
fn score(path: &Path) -> Option<Score> {
    let (score, _) = score_and_completeness(path)?;
    Some(score)
}

/// [`score`], and whether we drew the page completely.
///
/// The second answer is what the pdf.js gate needs and the specification gate does not: a page
/// whose font this tree refuses draws no glyphs and reads back nothing, and scoring it as a
/// failure would measure the report rather than the extraction.
fn score_and_completeness(path: &Path) -> Option<(Score, bool)> {
    let reference = reference_text(path)?;
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).ok()?;
    let page = pdf_model::Pages::new(&document).get(0)?;
    let interpretation = pdf_model::interpret(&document, &page);
    let complete = interpretation.is_complete();
    let ours = interpretation.text;

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

    Some((
        Score {
            matched,
            total: expected.len(),
            missing,
        },
        complete,
    ))
}

fn corpus() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc");
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

/// The pdf.js corpus, page one of every document it holds.
///
/// Returns an empty list where the submodule is not checked out, which is the one skip this
/// file allows — every other absence is a failure. See `corpus.rs` for the same rule.
fn pdfjs_corpus() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    files.sort();
    files
}

/// Documents whose page one we draw completely and whose text we still do not read back.
///
/// Named rather than counted, and checked for **equality** in both directions, exactly as the
/// oracle's lists are: a document that starts failing fails the gate even if another was fixed
/// the same day, and one that is fixed must be deleted rather than left to rot.
///
/// # What the 46 are, classified in the sixty-third session
///
/// The instrument that sorts them is one number per document — how many of the glyphs that
/// marked the page produced a character in the readback — and it separates two quite different
/// failures that both show up here as a low score.
///
/// **Thirty-one draw glyphs and name none of them**, and that is the limit this project has
/// recorded since the eighth session rather than a defect: `Interpretation::glyphs` exists
/// precisely because a font with no `/ToUnicode`, no AGL-known glyph names and no `cmap` a
/// reader can invert draws perfectly good letters that nothing can name. `issue918.pdf` is the
/// archetype and the largest entry on the list — 1327 glyphs, 193 reference words, and a
/// readback of nothing but the spaces the placement pass inferred. Its Type 3 fonts name their
/// glyphs `/a45`, `/a66`, `/a97` …, which is the character code in decimal and is not a name
/// §9.10.2 can resolve; the file states no `/ToUnicode` at all. `simpletype3font.pdf`,
/// `complex_ttf_font.pdf`, `issue1350.pdf` and `issue19802.pdf` are the same shape.
///
/// **Seven are right-to-left scripts read back in the order the content stream draws them.**
/// `issue10301.pdf` draws Hebrew that reads `אבג` and we return `גבא`; `ArabicCIDTrueType.pdf`,
/// `issue11656.pdf`, `issue12705.pdf`, `issue14046.pdf`, `issue5677.pdf` and `issue5874.pdf`
/// are the rest. **This is the module comment's own rule, one level up**: a content stream
/// records positions and not words, and it records glyphs in painting order and not in reading
/// order. Turning the second into the first is the Unicode bidirectional algorithm over a
/// layout this crate deliberately does not analyse, and `pdftotext` runs it. Ours is not wrong
/// about which glyph was drawn — which is what this test is for — and a fix belongs to whoever
/// builds selection.
///
/// **`issue8697.pdf` is the one that is a question about a clause.** It draws
/// `Ωηατ Οπερατινγ Σψστεµσ ∆ο` where `pdftotext` reads `What Operating Systems Do`: a Symbol
/// font whose glyphs are Greek and whose codes are Latin, so §9.10.2's second method takes each
/// code to the *glyph's* name and each glyph is a Greek letter. Both readbacks are defensible
/// and the clause is about naming what was drawn, which is what ours does.
///
/// The remaining six are partial for reasons nobody has looked into: `issue13211.pdf`,
/// `issue16538.pdf`, `issue16553.pdf`, `issue19182.pdf`, `issue19971.pdf` and
/// `bug1392647.pdf`. **They are the list worth working**, and the method is the one that found
/// `operator-in-TJ-array.pdf` and `issue15910.pdf` in the two sessions after this gate landed:
/// print our readback beside the reference's and read the file where they part.
///
/// # Three left in the sixty-fourth session, and the route is §9.10.2's own permission
///
/// `bug894572.pdf`, `issue1350.pdf` and `issue15910.pdf` are gone because a simple font's
/// glyph is now named by the *program* where the clause's three methods all fail — the `post`
/// table's name through the Adobe Glyph List, or the Unicode `cmap` subtable inverted. See
/// `pdf_font::LoadedFont::text_from_program`. The corpus went 96.5% to **97.8%** and no
/// document moved the other way, which is the measurement that says this is not the
/// fallback-that-fills-the-page.
const TEXT_BELOW_FLOOR: [&str; 43] = [
    "ArabicCIDTrueType.pdf",
    "PDFJS-7562-reduced.pdf",
    "Type3WordSpacing.pdf",
    "arial_unicode_en_cidfont.pdf",
    "bug1001080.pdf",
    "bug1027533.pdf",
    "bug1392647.pdf",
    "bug1650302_reduced.pdf",
    "complex_ttf_font.pdf",
    "french_diacritics.pdf",
    "issue10301.pdf",
    "issue11016_reduced.pdf",
    "issue11131_reduced.pdf",
    "issue11656.pdf",
    "issue12705.pdf",
    "issue13147.pdf",
    "issue13211.pdf",
    "issue14046.pdf",
    "issue14999_reduced.pdf",
    "issue15516_reduced.pdf",
    "issue16538.pdf",
    "issue16553.pdf",
    "issue19182.pdf",
    "issue19802.pdf",
    "issue19971.pdf",
    "issue2017r.pdf",
    "issue2537r.pdf",
    "issue2884_reduced.pdf",
    "issue3188.pdf",
    "issue5010.pdf",
    "issue5501.pdf",
    "issue5677.pdf",
    "issue5874.pdf",
    "issue5896.pdf",
    "issue7696.pdf",
    "issue8187.pdf",
    "issue8697.pdf",
    "issue918.pdf",
    "issue9655_reduced.pdf",
    "issue9915_reduced.pdf",
    "javauninstall-7r.pdf",
    "simpletype3font.pdf",
    "vertical.pdf",
];

/// The floor a complete page's extraction must clear.
///
/// Not 0.99, which is what the 14 specification PDFs are held to. Those are consistently
/// typeset files from three producers; the pdf.js corpus is 974 files chosen for having broken
/// a reader, and the disagreements that remain at this level are about what a *word* is rather
/// than about which glyph was drawn — see the module comment on why boundaries are not compared
/// and why a stripped-whitespace search still cannot be perfect.
const PDFJS_FLOOR: f64 = 0.90;

/// The text we draw agrees with an independent extractor, over the whole pdf.js corpus.
///
/// 974 documents against the 14 the test above uses, which is the extension the handover has
/// named as an opportunity since the thirty-first session. It is `#[ignore]`d for the same
/// reason `corpus.rs` and `oracle.rs` are: it needs a submodule and it runs an external program
/// per document.
///
/// **Only pages we claim to draw completely are gated.** A page whose font this tree refuses
/// draws no glyphs and reads back nothing, and failing it here would score the report rather
/// than the extraction — the same denominator rule the oracle uses, and the same warning: the
/// gated set grows when reports stop firing and shrinks when a silence ends.
#[test]
#[ignore = "needs the pdf.js submodule and runs pdftotext per document"]
fn the_text_we_draw_agrees_with_an_independent_extractor_across_the_pdfjs_corpus() {
    let corpus = pdfjs_corpus();
    if corpus.is_empty() {
        println!("doc/pdf.js is not checked out: skipped");
        return;
    }
    assert!(
        corpus
            .first()
            .is_some_and(|first| reference_text(first).is_some()),
        "pdftotext is required for this test; it comes with poppler"
    );

    let started = Instant::now();
    let scored: Vec<(String, Option<(Score, bool)>)> = corpus
        .par_iter()
        .map(|path| {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (name, score_and_completeness(path))
        })
        .collect();

    let (mut skipped, mut incomplete) = (0usize, 0usize);
    let (mut total_matched, mut total_words) = (0usize, 0usize);
    let mut below: Vec<String> = Vec::new();
    let mut ranked: Vec<(f64, String, String)> = Vec::new();

    for (name, result) in &scored {
        let Some((score, complete)) = result else {
            skipped += 1;
            continue;
        };
        if !complete {
            incomplete += 1;
            continue;
        }
        total_matched += score.matched;
        total_words += score.total;
        // A page with nothing to find proves nothing either way; the ratio is 1.0 by
        // definition and listing it would drown the ones that mean something.
        if score.total == 0 {
            continue;
        }
        if score.ratio() < PDFJS_FLOOR {
            below.push(name.clone());
            ranked.push((
                score.ratio(),
                name.clone(),
                format!(
                    "{}/{} words, missing e.g. {:?}",
                    score.matched, score.total, score.missing
                ),
            ));
        }
    }

    ranked.sort_by(|left, right| left.0.total_cmp(&right.0));
    for (ratio, name, detail) in &ranked {
        println!("  {name}: {:.1}% — {detail}", ratio * 100.0);
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "corpus word counts are far below f64's exact integer limit"
    )]
    let overall = if total_words == 0 {
        1.0
    } else {
        total_matched as f64 / total_words as f64
    };
    println!(
        "{} documents in {:.1}s: {} skipped (unopenable, no page one, or pdftotext refused), \
         {incomplete} incomplete and not gated; overall {:.1}% ({total_matched}/{total_words} \
         words), {} below {:.0}%",
        scored.len(),
        started.elapsed().as_secs_f64(),
        skipped,
        overall * 100.0,
        below.len(),
        PDFJS_FLOOR * 100.0
    );

    below.sort();
    let expected: Vec<String> = TEXT_BELOW_FLOOR.iter().map(|s| (*s).to_owned()).collect();
    let newly: Vec<&String> = below.iter().filter(|n| !expected.contains(n)).collect();
    let fixed: Vec<&String> = expected.iter().filter(|n| !below.contains(n)).collect();
    assert!(
        newly.is_empty(),
        "{} document(s) newly below {:.0}% of the words pdftotext finds: {newly:?}",
        newly.len(),
        PDFJS_FLOOR * 100.0
    );
    assert!(
        fixed.is_empty(),
        "{} document(s) no longer below the floor: {fixed:?}. Delete them from \
         TEXT_BELOW_FLOOR: a fixed page must not be able to come back.",
        fixed.len()
    );
}
