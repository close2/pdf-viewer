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
//!
//! # Two references, and one of them is frozen
//!
//! `pdftotext` runs at gate time and answers whatever the poppler on this machine answers
//! today. `doc/corpora/pdfbox` carries the other kind: `*.pdf.txt` and `*.pdf-sorted.txt`
//! checked in beside 40 of its PDFs, which is Apache `PDFBox`'s own `PDFTextStripper` output
//! frozen at the commit the submodule pins. A frozen opinion from a second implementation is
//! a different instrument from a live one — it cannot drift under this tree, and it was
//! written by people who read §9.10.2 independently.
//!
//! `CLAUDE.md`'s principle 5 governs both of them identically: agreement raises confidence
//! that this tree read the clause correctly, and disagreement is a question to take back to
//! the standard. Neither is a target.

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
///
/// **U+00AD goes with it, and there the file has said so rather than left it to be guessed.**
/// §14.8.2.3 names the soft hyphen as the representation of exactly this character — the one a
/// line break introduced, which "may be represented as a soft hyphen, mapped to the Unicode
/// value U+00AD" — and requires a writer to "distinguish explicitly between soft and hard
/// hyphens". So a producer that writes one has stated the very fact the paragraph above has to
/// infer for a hyphen-minus, and dropping it is the same rule with a stronger warrant.
/// `bug1997343.pdf` is the witness: a tagged LaTeX document whose six line-broken words read
/// back as `in\u{ad}cluding`, `fol\u{ad}low`, `mathemat\u{ad}ics` and three more, against a
/// reference that rejoins them.
fn without_hyphens(text: &str) -> String {
    text.chars()
        .filter(|c| *c != '-' && *c != '\u{00AD}')
        .collect()
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
/// archetype and was the largest entry on the list — 1327 glyphs, 193 reference words, and a
/// readback of nothing but the spaces the placement pass inferred. Its Type 3 fonts name their
/// glyphs `/a45`, `/a66`, `/a97` …, which is the character code in decimal and is not a name
/// §9.10.2 can resolve; the file states no `/ToUnicode` at all. `simpletype3font.pdf`,
/// `complex_ttf_font.pdf`, `issue1350.pdf` and `issue19802.pdf` are the same shape.
///
/// **`issue918.pdf` reads back 186 of those 193 words now, and the sentence above went on
/// saying otherwise long after it stopped being true** — which is worth leaving visible rather
/// than editing away: §9.10.2's closing permission, the code itself where it is a printable
/// ASCII byte, answers `/a65` for `A` and every other Latin letter dvips numbered that way, and
/// it landed in the three-hundred-and-twenty-eighth session. What it cannot answer is a code
/// *outside* 0x21–0x7E, which is where the seven that are left live: `Václav`'s `á` is one
/// glyph at an OT1 code, and `signifier`'s `fi` is another. `pdftotext` answers those with
/// U+001C and U+001E, which are not characters either, so the two readers fail the same
/// question differently rather than one of them being right.
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
/// **`issue8697.pdf` was described here as "a question about a clause" and it was a defect.**
/// The entry said the file draws `Ωηατ Οπερατινγ Σψστεµσ ∆ο` where `pdftotext` reads
/// `What Operating Systems Do`, that this is a Symbol font whose glyphs are Greek and whose
/// codes are Latin, and that "both readbacks are defensible" because §9.10.2 names what was
/// drawn. Every sentence of that is true about the readback and none of it asked why the Greek
/// was on the page: the font is `/SegoeUISymbol`, a sans-serif face whose *name* ends in the
/// word, and both `/Encoding /WinAnsiEncoding` and Table 121's Nonsymbolic flag say so.
/// §9.6.5.4 makes that a `shall` — the code-to-glyph-name table is the Latin one — and the
/// standard-14 `Symbol` was being substituted off the name alone. ADR 0158; the readback is
/// 100% now. **A gate entry that reasons about its own half of the pipeline can be right in
/// every sentence and still be describing a defect one stage upstream.**
///
/// The remaining three are partial for reasons nobody has diagnosed further: `issue13211.pdf`,
/// `issue16553.pdf` and `bug1392647.pdf`. **All of them agree with the reference consensus on
/// pixels**, measured in the sixty-sixth session with one filtered oracle run, so every one of
/// them draws a picture two independent renderers accept and fails only at naming what it drew.
/// (`issue16538.pdf` was a fourth until §9.7.5.2's `CMap`s landed; see below.)
///
/// **`issue16553.pdf` was one of that three for three hundred and fifty-seven sessions, and it
/// was diagnosed by a corpus this tree had never seen.** It and `javauninstall-7r.pdf` left this
/// list in the four-hundred-and-twenty-third session, and neither was worked on: the gate below
/// — `doc/corpora/pdfbox`'s frozen extraction — found `PDFBOX-5838-0024320-reduced.pdf` reading
/// `H Reeach Pec` for `Honors Research Project`, and the clause that fixed that one fixed these
/// two the same afternoon. All three are `Identity-H` composite fonts whose `/ToUnicode` answers
/// for some codes or none, and §9.10.2 excludes exactly that shape from its third method by
/// name, so every method had failed and the permission the clause grants was being declined.
/// `pdf_font::LoadedFont::text_from_program` carries the reading. **An entry parked as
/// undiagnosed is not the same as an entry that cannot be diagnosed**, and what moved this one
/// was a second population rather than a second look.
///
/// **Two left this list in the hundred-and-twenty-seventh session and neither was diagnosed
/// here**: `issue19182.pdf` and `issue19971.pdf` were reading a font the font *cache* had
/// handed them, keyed by the resource name `/C2_0` or `/F1` rather than by the font's identity,
/// so a form `XObject`'s font was answering the page's question. The first now reports the
/// predefined `CMap` it actually names and leaves this gate's population; the second rose above
/// the floor. **A text-extraction shortfall nobody could diagnose was a font nobody had
/// looked up** — worth remembering before spending a session on §9.10.2 for the three left. The whole of what is left
/// on this list is §9.10.2, and none of it is a drawing defect — which is worth knowing before
/// spending a session on any of them, and is the cheapest thing to check about an entry here.
///
/// The method that found the two real defects is still the one to use: print our readback beside
/// the reference's and read the file where they part. It found `operator-in-TJ-array.pdf` and
/// `issue15910.pdf` in the two sessions after this gate landed.
///
/// # Three left in the sixty-fourth session, and the route is §9.10.2's own permission
///
/// `bug894572.pdf`, `issue1350.pdf` and `issue15910.pdf` are gone because a simple font's
/// glyph is now named by the *program* where the clause's three methods all fail — the `post`
/// table's name through the Adobe Glyph List, or the Unicode `cmap` subtable inverted. See
/// `pdf_font::LoadedFont::text_from_program`. The corpus went 96.5% to **97.8%** and no
/// document moved the other way, which is the measurement that says this is not the
/// fallback-that-fills-the-page.
///
/// # One joined in the seventy-second session, and it is the denominator moving
///
/// `issue17069.pdf` reads back 10 of the 12 words `pdftotext` finds, missing `rmX` and `teO`
/// — and it is *new to this list without its readback changing at all*. It reported §9.3.8's
/// text knockout until that clause was implemented, and only pages we draw completely are
/// gated. A gate's numerator moves when its denominator does, and only one of those is news.
/// # Six left in the hundred-and-fifty-sixth session, and the list heard in the hundred-and-sixty-sixth
///
/// `arial_unicode_en_cidfont.pdf`, `issue13147.pdf`, `issue16538.pdf`, `issue2884_reduced.pdf`,
/// `issue7696.pdf` and `vertical.pdf` all read back **100%** of `pdftotext`'s words once
/// §9.7.5.2's predefined `CMap`s and §9.10.2's third method arrived (ADR 0140). Six is the
/// number that session's own handover entry records — and the constant below still had all six
/// in it, so **this gate has been failing since, and two sessions of "everything re-verified"
/// did not notice**. The lesson is the ratchet's, not the clause's: an entry that becomes a
/// success fails the build in a message that reads like a regression, and a session that reads
/// only the summary line will believe the summary. `doc/HANDOVER.md`'s own "36 below the floor"
/// was right about the *measurement* and wrong about the list the whole time.
/// # One joined in the two-hundred-and-eighty-fourth, and **the reference is the one that is wrong**
///
/// `bug1865341.pdf` is a free text annotation whose value is *Załącznik* and whose `/DA` names a
/// font `/DR` does not define. `pdftotext` reads back **`Zacznik`** — poppler draws it that way
/// too, with both diacritics silently dropped, which the side-by-side in ADR 0184 shows. This
/// tree draws and reads back all nine characters, so the comparison scores **0 of 1 words** and
/// the ratchet fires on an improvement for the second time in this file's history.
///
/// The clause settles it rather than the vote: §9.6.5.1's `/Differences` is how a glyph the base
/// encoding has no code for is named, the value's `ą` is `aogonek` in the Adobe Glyph List, and
/// every Helvetica has that glyph. Held by name because a reference that drops characters cannot
/// be the numerator, and this is the one entry on this list whose readback is *better* than
/// `pdftotext`'s rather than worse.
///
/// # One left in the four-hundred-and-sixty-third, and three of the four *below* this list were read
///
/// `issue5010.pdf` is gone: its `/ToUnicode` states five mappings for codes its page never shows
/// and `/Adobe-Korea1-UCS2 usecmap` for the rest, which §9.10.3 permits — "`UseCMap` , which may be
/// used if the `CMap` is based on another `ToUnicode` `CMap`" — and which nothing here followed, so the
/// page read back the empty string. ADR 0298.
///
/// That round also listed, for the first time, every document scoring under 100% rather than
/// under the floor, because the band between them had never been named. It is **four documents
/// and seventeen words**, and none of the four is a wrong glyph:
///
/// - `bug1997343.pdf` (8): four are §14.9.4's `/ActualText` — a structure element saying `LaTeX`
///   where the glyphs spell `LATEX`, which this tree reads and the reference does not — and four
///   are the soft hyphens [`without_hyphens`] now folds.
/// - `issue918.pdf` (7): the Type 3 codes outside printable ASCII described above.
/// - `issue20489.pdf` (1): `Date>SCALE` is two labels forty lines apart that `pdftotext`'s column
///   analysis ran into one word.
/// - `issue1350.pdf` (1): the reference reads `beginnerÕs`, which is `MacRomanEncoding` 0xD5 taken
///   as Latin-1; this tree reads `beginner’s`.
const TEXT_BELOW_FLOOR: [&str; 22] = [
    "ArabicCIDTrueType.pdf",
    "bug1865341.pdf",
    "PDFJS-7562-reduced.pdf",
    "Type3WordSpacing.pdf",
    "bug1027533.pdf",
    "bug1392647.pdf",
    "bug1650302_reduced.pdf",
    "complex_ttf_font.pdf",
    "french_diacritics.pdf",
    "issue10301.pdf",
    "issue11131_reduced.pdf",
    "issue11656.pdf",
    "issue12705.pdf",
    "issue13211.pdf",
    "issue14046.pdf",
    "issue17069.pdf",
    "issue19802.pdf",
    "issue2017r.pdf",
    "issue2537r.pdf",
    "issue5677.pdf",
    "issue5874.pdf",
    "issue9915_reduced.pdf",
];

/// The floor a complete page's extraction must clear.
///
/// Not 0.99, which is what the 14 specification PDFs are held to. Those are consistently
/// typeset files from three producers; the pdf.js corpus is 974 files chosen for having broken
/// a reader, and the disagreements that remain at this level are about what a *word* is rather
/// than about which glyph was drawn — see the module comment on why boundaries are not compared
/// and why a stripped-whitespace search still cannot be perfect.
const PDFJS_FLOOR: f64 = 0.90;

/// Fails the gate if this build cannot reach the sandboxed image decoder.
///
/// `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by a separate program, and Cargo
/// does not build another package's binaries when it tests this one (trap 10). A build without
/// it draws every other image and none of those three, so what follows would be a measurement of
/// the build rather than of the tree — which is exactly what moved the accessibility census's
/// ratchet by nine elements while four rounds read the difference as something else
/// (ADR 0557, trap 16).
#[expect(
    clippy::panic,
    reason = "a gate that cannot decode the images it is measuring must stop rather than \
              print a number about a different program"
)]
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so the counts below would be \
             wrong: {error}"
        );
    }
}

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
    require_the_sandbox();
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

/// Where Apache `PDFBox` keeps its test documents and the extraction it expects from them.
///
/// A **partial, sparse** submodule — `doc/oracle-and-corpus.md` §2 carries the checkout recipe
/// because `.gitmodules` cannot express one.
const PDFBOX_INPUT: &str = "../../doc/corpora/pdfbox/pdfbox/src/test/resources/input";

/// One `PDFBox` document and the text that repository froze beside it.
struct Frozen {
    /// The document.
    path: PathBuf,
    /// `<name>.pdf.txt`: `PDFTextStripper` with its default ordering, which is the order the
    /// content stream shows the glyphs in.
    stripped: String,
    /// `<name>.pdf-sorted.txt`: the same stripper with `setSortByPosition(true)`, which is
    /// `PDFBox` performing the layout analysis this crate deliberately does not.
    ///
    /// Present for 40 of the 40, and read as a *diagnosis* rather than as a second gate: where
    /// the two disagree the question is about reading order, which §14.8.2.5.1 puts in the
    /// structure tree rather than in the content stream.
    sorted: Option<String>,
}

/// Reads one of `PDFBox`'s expected-text files, dropping the byte-order mark it begins with.
///
/// Every one of the 81 is UTF-8 with a BOM; U+FEFF would otherwise become a word character
/// on the front of the first word and cost it its match.
fn frozen_text(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(text.strip_prefix('\u{FEFF}').unwrap_or(&text).to_owned())
}

/// Every `PDFBox` document that has a frozen extraction beside it.
///
/// Returns an empty list where the submodule is not checked out, which is the same one skip
/// the pdf.js gate above allows. 64 PDFs are checked out and 40 of them carry a `.pdf.txt`;
/// the other 24 are there for rendering, merging and compression tests, and a document with
/// no expected text is not this instrument's business.
fn pdfbox_corpus() -> Vec<Frozen> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(PDFBOX_INPUT);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut documents: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    documents.sort();
    documents
        .into_iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_string_lossy().into_owned();
            let beside = |suffix: &str| path.with_file_name(format!("{name}{suffix}"));
            Some(Frozen {
                stripped: frozen_text(&beside(".txt"))?,
                sorted: frozen_text(&beside("-sorted.txt")),
                path,
            })
        })
        .collect()
}

/// This tree's readback of a whole document, and whether every page of it drew completely.
///
/// Whole document rather than page one, because `PDFBox`'s fixture is a whole document:
/// `PDFTextStripper` walks every page unless told otherwise, and `cweb.pdf` has 28. Comparing
/// 28 pages of expectation against one page of readback would score the page count.
fn whole_document_text(path: &Path) -> Option<(String, bool)> {
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).ok()?;
    let pages = pdf_model::Pages::new(&document);
    let mut text = String::new();
    let mut complete = true;
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            complete = false;
            continue;
        };
        let interpretation = pdf_model::interpret(&document, &page);
        complete &= interpretation.is_complete();
        text.push_str(&interpretation.text);
        text.push('\n');
    }
    Some((text, complete))
}

/// Scores one readback against one frozen reference, by [`score_and_completeness`]'s rule.
fn score_against(ours: &str, reference: &str) -> Score {
    let haystack = without_spaces(ours);
    let expected = reference_words(reference);

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
    Score {
        matched,
        total: expected.len(),
        missing,
    }
}

/// Documents whose every page draws completely and whose readback is still below the floor
/// against `PDFBox`'s frozen extraction.
///
/// Named rather than counted and checked in both directions, exactly as [`TEXT_BELOW_FLOOR`] is:
/// a document that starts failing fails the gate even if another was fixed the same day.
///
/// # What the four are, read in the four-hundred-and-twenty-third session
///
/// The first run named five. **One of them was a defect and it is fixed** —
/// `PDFBOX-5838-0024320-reduced.pdf` read back `H Reeach Pec` where `PDFBox` reads
/// `Honors Research Project`, because §9.10.2 excludes an `Identity-H` composite font from its
/// third method *by name* and the permission it grants where every method fails was being
/// declined; see `pdf_font::LoadedFont::text_from_program`. The four below are differences
/// rather than defects, and the reading is recorded here because a difference from another
/// implementation is a question and never a target (`CLAUDE.md`, principle 5).
///
/// **`hello3.pdf` and `FC60_Times.pdf` are right-to-left text read back in the order the content
/// stream shows it, in the forms the file's own `/ToUnicode` names.** `hello3.pdf` draws
/// `Hello محمد World.`; this tree returns U+FEE3 U+FEA4 U+FEE4 U+FEAA — the same four letters in
/// their Arabic Presentation Forms-B contextual shapes, in painting order — where `PDFBox` returns
/// U+0645 U+062D U+0645 U+062F in logical order. `FC60_Times.pdf` is the same twice over, plus
/// U+FC60, the shadda-with-fatha ligature, where `PDFBox` writes the two marks separately.
///
/// Three conventions differ and none of them is about which glyph was drawn:
///
/// - **Order.** §14.8.2.5.1 is decisive and this tree is on its own side of it: "[p]age content
///   order shall be defined by the sequencing of graphics objects within a page's content
///   stream", while logical content order "shall be defined by a depth-first traversal of the
///   document's logical structure hierarchy". `Interpretation::text` is the first of those by
///   construction. `FC60_Times.pdf` has no structure tree at all and `hello3.pdf` has one, and
///   neither settles this: a traversal of the structure hierarchy orders *content items*, and the
///   characters here are reversed inside one show string. What `PDFBox` returns is the Unicode
///   bidirectional algorithm applied to a run it identified as Arabic, which is layout analysis
///   and not extraction.
/// - **§14.8.2.5.3 is the tag that would settle it and neither file writes one.** `/ReversedChars`
///   is how a file says a show string holds its characters backwards, and this tree has obeyed it
///   since the eighty-third session — grepped for in both documents' bytes and absent from both.
///   A file that has not used the mechanism the standard provides has not stated the order.
/// - **Presentation form against base letter** is `fold`'s Latin argument in another script: a
///   `/ToUnicode` naming U+FEE3 says what the glyph *is* and one naming U+0645 says what it
///   *reads as*, Unicode records the pair as a compatibility equivalence, and §9.10.2 says how to
///   learn what a code means and nothing about normalising the answer. It is deliberately **not**
///   folded here: Arabic Presentation Forms-B is 141 code points against `fold`'s nine Latin
///   ones, and folding a block this instrument has two witnesses for would be fitting the
///   instrument to the population.
///
/// **`PDFBOX-4322-Empty-ToUnicode-reduced.pdf` and `sample_fonts_solidconvertor.pdf` are the one
/// place this tree and `PDFBox` make different *choices* under the same permission**, and the
/// choice is this tree's and is deliberate. Both are `Identity-H` composite fonts whose
/// `/ToUnicode` is an Identity CID `CMap` rather than a mapping to Unicode — a stream declaring
/// `/CMapType 1` and one `begincidrange` in the first, the bare name `/Identity-H` in the second
/// — so §9.10.3 is not satisfied ("[i]t shall use the beginbfchar, endbfchar, beginbfrange, and
/// endbfrange operators to define the mapping from character codes to Unicode character
/// sequences expressed in UTF-16BE encoding") and no code maps to anything. Their embedded
/// programs then say nothing either: neither subset carries a `cmap` **or** a `post` table, which
/// was checked rather than assumed, so §9.10.2's last resort correctly declines. `PDFBox` reads the
/// code itself as the Unicode value; its own source calls that "the undocumented case", it is
/// right on these two files because their producers numbered the CIDs by code point, and it is
/// mojibake on any file that did not. `text_from_the_code` takes that step for a **one-byte**
/// code, where §9.6.5's encodings make a byte and a code point the same character, and refuses it
/// for a two-byte one, where nothing does. A silence this tree can defend is preferred to a guess
/// it cannot — and the choice is written here rather than left in the shape of a passing gate.
const PDFBOX_BELOW_FLOOR: [&str; 4] = [
    "FC60_Times.pdf",
    "PDFBOX-4322-Empty-ToUnicode-reduced.pdf",
    "hello3.pdf",
    "sample_fonts_solidconvertor.pdf",
];

/// The floor a completely drawn document's extraction must clear against `PDFBox`'s own.
///
/// The same 0.90 the pdf.js gate uses and for the same reason: these are 40 documents attached
/// to a decade of bug reports, and what remains at this level is about what a *word* is rather
/// than about which glyph was drawn.
const PDFBOX_FLOOR: f64 = 0.90;

/// One document's two scores, and whether every page of it drew completely.
struct Scored {
    /// Against `<name>.pdf.txt`, which is the score the floor applies to.
    stripped: Score,
    /// Against `<name>.pdf-sorted.txt`, printed beside it for diagnosis.
    sorted: Option<Score>,
    /// Whether every page reported nothing; only those are gated.
    complete: bool,
}

/// Reads one document back and scores it against both of `PDFBox`'s frozen texts.
fn score_frozen(frozen: &Frozen) -> Option<Scored> {
    let (ours, complete) = whole_document_text(&frozen.path)?;
    Some(Scored {
        stripped: score_against(&ours, &frozen.stripped),
        sorted: frozen
            .sorted
            .as_deref()
            .map(|sorted| score_against(&ours, sorted)),
        complete,
    })
}

/// The text we draw agrees with an extraction a different implementation froze on disk.
///
/// The instrument is `text_extraction.rs`'s and the reference is not: `pdftotext` above runs
/// now, and this runs nothing at all — Apache `PDFBox`'s `PDFTextStripper` output is checked into
/// the submodule beside the documents it was taken from. It costs no external process, which is
/// why it is cheap enough to be worth having and is still **not** in `doc/todo/02` §2's default
/// sequence until it has earned a place there.
///
/// `#[ignore]`d because it needs a submodule, like every other corpus test here.
///
/// **Only documents every page of which we claim to draw completely are gated**, which is the
/// denominator rule the pdf.js gate and the oracle both use: a page whose font this tree
/// refuses draws no glyphs and reads back nothing, and failing it here would score the report
/// rather than the extraction.
#[test]
#[ignore = "needs the doc/corpora/pdfbox submodule"]
fn the_text_we_draw_agrees_with_pdfboxs_frozen_extraction() {
    require_the_sandbox();
    let corpus = pdfbox_corpus();
    if corpus.is_empty() {
        println!("doc/corpora/pdfbox is not checked out: skipped");
        return;
    }

    let started = Instant::now();
    let scored: Vec<(String, Option<Scored>)> = corpus
        .par_iter()
        .map(|frozen| {
            let name = frozen
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (name, score_frozen(frozen))
        })
        .collect();

    let (mut skipped, mut incomplete) = (0usize, 0usize);
    let (mut total_matched, mut total_words) = (0usize, 0usize);
    let (mut sorted_matched, mut sorted_words) = (0usize, 0usize);
    let mut below: Vec<String> = Vec::new();
    let mut ranked: Vec<(f64, String, String)> = Vec::new();

    for (name, result) in &scored {
        let Some(result) = result else {
            skipped += 1;
            continue;
        };
        if !result.complete {
            incomplete += 1;
            continue;
        }
        let score = &result.stripped;
        total_matched += score.matched;
        total_words += score.total;
        if let Some(sorted) = result.sorted.as_ref() {
            sorted_matched += sorted.matched;
            sorted_words += sorted.total;
        }
        // A document with nothing to find proves nothing either way; the ratio is 1.0 by
        // definition and listing it would drown the ones that mean something.
        if score.total == 0 || score.ratio() >= PDFBOX_FLOOR {
            continue;
        }
        below.push(name.clone());
        ranked.push((score.ratio(), name.clone(), detail(result)));
    }

    ranked.sort_by(|left, right| left.0.total_cmp(&right.0));
    for (ratio, name, detail) in &ranked {
        println!("  {name}: {:.1}% — {detail}", ratio * 100.0);
    }
    println!(
        "{} documents in {:.1}s: {skipped} skipped (unopenable or no pages), {incomplete} \
         incomplete and not gated; overall {:.1}% ({total_matched}/{total_words} words) against \
         PDFBox's stream order and {:.1}% ({sorted_matched}/{sorted_words}) against its \
         position-sorted output, {} below {:.0}%",
        scored.len(),
        started.elapsed().as_secs_f64(),
        percentage(total_matched, total_words),
        percentage(sorted_matched, sorted_words),
        below.len(),
        PDFBOX_FLOOR * 100.0
    );

    below.sort();
    let expected: Vec<String> = PDFBOX_BELOW_FLOOR.iter().map(|s| (*s).to_owned()).collect();
    let newly: Vec<&String> = below.iter().filter(|n| !expected.contains(n)).collect();
    let fixed: Vec<&String> = expected.iter().filter(|n| !below.contains(n)).collect();
    assert!(
        newly.is_empty(),
        "{} document(s) newly below {:.0}% of the words PDFBox froze: {newly:?}",
        newly.len(),
        PDFBOX_FLOOR * 100.0
    );
    assert!(
        fixed.is_empty(),
        "{} document(s) no longer below the floor: {fixed:?}. Delete them from \
         PDFBOX_BELOW_FLOOR: a fixed document must not be able to come back.",
        fixed.len()
    );
}

/// A shortfall's line: the words, what the position-sorted reference would have scored, and a
/// few of the words we did not produce.
///
/// The sorted figure is here because it separates the two questions this comparison confuses.
/// `PDFBox`'s `-sorted.txt` is the same extraction with `setSortByPosition(true)`, so a document
/// on which the two references disagree is one where *reading order* is at stake and a document
/// on which they agree is one where it is not — which is worth knowing before reading a
/// shortfall as a wrong glyph.
fn detail(scored: &Scored) -> String {
    format!(
        "{}/{} words, {} sorted, missing e.g. {:?}",
        scored.stripped.matched,
        scored.stripped.total,
        scored.sorted.as_ref().map_or_else(
            || "no".to_owned(),
            |sorted| format!("{:.1}%", sorted.ratio() * 100.0)
        ),
        scored.stripped.missing
    )
}

/// A matched-over-total as a percentage, with an empty population counting as complete.
fn percentage(matched: usize, words: usize) -> f64 {
    if words == 0 {
        return 100.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "corpus word counts are far below f64's exact integer limit"
    )]
    {
        matched as f64 / words as f64 * 100.0
    }
}

// ---------------------------------------------------------------------------------------------
// ADR 0323's instrument 1: selection and extraction geometry.
//
// Everything above compares *which characters* a page reads back as; nothing anywhere compared
// *where they are* — and the quadrilateral a drag is turned into is built from exactly that
// geometry (`Interpretation::text_layer`). The instrument below judges those word boxes against
// `pdftotext -bbox -cropbox`, matched by unique text, with the frame audited before any box is
// compared and every exclusion printed by reason, because a document the instrument cannot
// judge is a document off the judged set (trap 11's arithmetic).
//
// The verdict is bounded and split by axis, on ADR 0323's measurement: horizontal edges are a
// specification quantity (§9.4.4's glyph positioning arithmetic — the references agree to a
// ten-thousandth of a point); the vertical *extent* of a word box is each extractor's own
// ascent/descent convention and is deliberately unjudged; the vertical *centre* is judged
// relative to the word's own height. Reading order is deliberately not part of any of it
// (ADR 0323 Finding 4). The bounds are re-derived by this binary itself —
// `the_selection_bounds_against_the_references_own_spread` below — so the derivation lives
// next to the bound.
// ---------------------------------------------------------------------------------------------

use pdfref::{ExtractionCache, ExtractionError, Extractor};

/// The horizontal bound, in points, on each of a matched word's two vertical edges.
///
/// From ADR 0323 Finding 2: over 13 130 matched word pairs the two references' own horizontal
/// spread is p99 0.14 pt, and 0.5 pt admits it with the same headroom `Tolerance`'s raster
/// bounds carry. Re-derive it rather than trusting this comment:
///
/// ```sh
/// PDFVIEWER_SELECTION_SPREAD=1 cargo test --profile gates -p pdf-model \
///     --test text_extraction -- --ignored --nocapture the_selection_bounds
/// ```
const HORIZONTAL_BOUND: f64 = 0.5;

/// The vertical-centre bound, as a fraction of the matched word's own height.
///
/// From ADR 0323 Finding 3: vertical *edges* are convention against convention (the two
/// references' box-height ratio is median 1.29 on identical glyphs) and are excluded from the
/// verdict; centres relative to the word's height agree to p99 0.455, which half the word's
/// height admits. The height a delta is relative to is the mean of the two boxes' heights,
/// stated here because the two conventions differ by a third and the choice moves the number.
/// The same derivation run as [`HORIZONTAL_BOUND`] reprints it.
const VERTICAL_CENTRE_BOUND: f64 = 0.5;

/// The judged documents with a word out of bounds, and the ratchet's whole population.
///
/// **This is the instrument's first ratchet, and ADR 0323's rule is what let it be one**: the
/// figures held from the four-hundred-and-ninety-eighth session to the five-hundred-and-eighty-
/// sixth — 98.26% of matched words in bounds both times, 486 of 508 documents fully in bounds
/// against 485 of 507, the one document's difference being a document that entered the judged
/// set rather than a word that moved. A number that survived eighty-eight rounds of unrelated
/// work is a number a round can be held to; ADR 0421.
///
/// Checked in **both** directions, exactly as [`TEXT_BELOW_FLOOR`] is: a document arriving here
/// fails the gate, and a document that leaves must be deleted from this list so that it cannot
/// come back. What each one is is not written down per line — the gate prints every name with
/// its fraction and its worst horizontal delta above, which is where a round reads them — and
/// three classes account for most of it: a box convention on Type 3 and vertically-set text
/// (`issue1350.pdf`'s hundred words agree horizontally to 0.00 pt and fail only the vertical
/// centre; `vertical.pdf` and `issue11555.pdf` fail on the cross-axis), a substituted face whose
/// advances are not the document's, and a genuine placement difference of a point or more.
const SELECTION_BELOW_FLOOR: [&str; 22] = [
    "TrueType_without_cmap.pdf",
    "annotation-tx.pdf",
    "bug1771477.pdf",
    "bug1796741.pdf",
    "bug1844576.pdf",
    "bug1844583.pdf",
    "bug868745.pdf",
    "hello_world_rotated.pdf",
    "issue11555.pdf",
    "issue12750.pdf",
    "issue1350.pdf",
    "issue14497.pdf",
    "issue16021.pdf",
    "issue18099_reduced.pdf",
    "issue1905.pdf",
    "issue19389.pdf",
    "issue20232.pdf",
    "issue2391-2.pdf",
    "issue4665.pdf",
    "issue6127.pdf",
    "issue6605.pdf",
    "vertical.pdf",
];

/// The smallest judged set this instrument may have.
///
/// Trap 11's arithmetic as a ratchet rather than as a printed line: every refusal above takes a
/// document off the judged set, so a change that made poppler refuse more documents — or made
/// this tree read fewer words — would shrink the denominator and leave the verdict above looking
/// unmoved. 508 in the five-hundred-and-eighty-sixth session, 507 in the four-hundred-and-ninety-
/// eighth; it may rise, and a rise is written down here.
const JUDGED_FLOOR: usize = 508;

/// One point per axis before two statements of the page's frame count as the same frame.
///
/// The harness's one-pixel-per-axis rule at 72 dpi, applied to the page's *size*: ADR 0323's
/// Finding 1 found three mechanisms by which a positional extractor answers in a different
/// frame — `MediaBox` against `CropBox`, Table 31's `/UserUnit`, `/Rotate` — and a document
/// whose frames cannot be reconciled is refused by name rather than judged.
const FRAME_TOLERANCE: f64 = 1.0;

/// A word and its box in the reference frame: **the displayed page in unscaled points** —
/// origin at the displayed page's top-left corner, y growing down, `/Rotate` applied,
/// `/UserUnit` *not* applied.
///
/// The frame is `pdftotext -bbox -cropbox`'s own — established by measurement, and not the
/// frame its `<page>` element states; see [`Frame::reference_point`] for the witnesses.
/// Everything else — `mutool`'s `/UserUnit`-scaled frame and this tree's display-list space —
/// is normalised *into* it explicitly.
struct WordBox {
    /// The word, folded by [`fold`].
    text: String,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl WordBox {
    fn height(&self) -> f64 {
        self.y1 - self.y0
    }

    fn centre_y(&self) -> f64 {
        f64::midpoint(self.y0, self.y1)
    }
}

/// One page of one extractor's answer: the page's stated size, and its words.
struct ExtractedPage {
    width: f64,
    height: f64,
    words: Vec<WordBox>,
}

/// Undoes the XML escaping both extractors' output formats apply.
fn unescaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let Some(end) = tail.find(';') else {
            out.push_str(tail);
            return out;
        };
        match &tail[1..end] {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            code => {
                let parsed = code.strip_prefix("#x").map_or_else(
                    || code.strip_prefix('#').and_then(|d| d.parse::<u32>().ok()),
                    |hex| u32::from_str_radix(hex, 16).ok(),
                );
                match parsed.and_then(char::from_u32) {
                    Some(c) => out.push(c),
                    // Not an entity this format writes; kept as it came.
                    None => out.push_str(&tail[..=end]),
                }
            }
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

/// One `name="value"` attribute of an XML tag, unparsed.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let start = tag.find(&format!("{name}=\""))? + name.len() + 2;
    let rest = tag.get(start..)?;
    rest.get(..rest.find('"')?)
}

/// A `name="number"` attribute of an XML tag.
fn number(tag: &str, name: &str) -> Option<f64> {
    attribute(tag, name)?.parse().ok()
}

/// Parses `pdftotext -bbox`'s XHTML: one `<word>` element per word, already in the reference
/// frame.
fn poppler_page(html: &str) -> Option<ExtractedPage> {
    let mut page: Option<ExtractedPage> = None;
    for line in html.lines() {
        let line = line.trim_start();
        if line.starts_with("<page ") {
            if page.is_some() {
                // -f 1 -l 1 asks for one page; a second one means the question changed.
                return None;
            }
            page = Some(ExtractedPage {
                width: number(line, "width")?,
                height: number(line, "height")?,
                words: Vec::new(),
            });
        } else if let Some(open) = line.strip_prefix("<word ")
            && let Some(page) = page.as_mut()
        {
            let text = open.get(open.find('>')? + 1..)?.strip_suffix("</word>")?;
            page.words.push(WordBox {
                text: fold(&unescaped(text)),
                x0: number(open, "xMin")?,
                y0: number(open, "yMin")?,
                x1: number(open, "xMax")?,
                y1: number(open, "yMax")?,
            });
        }
    }
    page
}

/// Parses `mutool draw -F stext`'s XML into words, in **`mutool`'s own frame**: points from the
/// top-left of the page as displayed — rotated by `/Rotate` and scaled by `/UserUnit`, which is
/// what [`normalised_into_the_reference_frame`] undoes.
///
/// `stext` states characters, not words, so words are built here: characters of one `<line>`
/// accumulate and whitespace closes a word, which is the same convention `pdftotext` applies
/// on its side of the match.
fn mupdf_page(xml: &str) -> Option<ExtractedPage> {
    let mut page: Option<ExtractedPage> = None;
    // The word being accumulated: text, and the running box.
    let mut word: Option<WordBox> = None;
    for line in xml.lines() {
        let line = line.trim_start();
        if line.starts_with("<page ") {
            if page.is_some() {
                return None;
            }
            page = Some(ExtractedPage {
                width: number(line, "width")?,
                height: number(line, "height")?,
                words: Vec::new(),
            });
        } else if line.starts_with("</line>") {
            if let (Some(page), Some(done)) = (page.as_mut(), word.take()) {
                page.words.push(done);
            }
        } else if line.starts_with("<char ")
            && let Some(page) = page.as_mut()
        {
            let text = unescaped(attribute(line, "c")?);
            if text.chars().all(char::is_whitespace) {
                if let Some(done) = word.take() {
                    page.words.push(done);
                }
                continue;
            }
            let quad = attribute(line, "quad")?;
            let mut corners = quad.split_ascii_whitespace().filter_map(|n| n.parse().ok());
            let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
            let (mut x1, mut y1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
            for _ in 0..4 {
                let (x, y): (f64, f64) = (corners.next()?, corners.next()?);
                (x0, y0) = (x0.min(x), y0.min(y));
                (x1, y1) = (x1.max(x), y1.max(y));
            }
            match word.as_mut() {
                Some(word) => {
                    word.text.push_str(&fold(&text));
                    word.x0 = word.x0.min(x0);
                    word.y0 = word.y0.min(y0);
                    word.x1 = word.x1.max(x1);
                    word.y1 = word.y1.max(y1);
                }
                None => {
                    word = Some(WordBox {
                        text: fold(&text),
                        x0,
                        y0,
                        x1,
                        y1,
                    });
                }
            }
        }
    }
    page
}

/// The page's frame as this tree reads it: the crop box in points, `/Rotate`, `/UserUnit`.
///
/// ISO 32000-2 §7.7.3.3 Table 31 is the source of all three, and each is one of the three
/// mechanisms ADR 0323's Finding 1 measured. What the audit and the normalisation need is
/// exactly this triple and nothing else about the page.
struct Frame {
    /// Crop-box width and height, unrotated, in points.
    width: f64,
    height: f64,
    rotate: u16,
    user_unit: f64,
}

impl Frame {
    fn of(page: &pdf_model::Page) -> Option<Self> {
        // The display list's own space is built from `/ViewArea`'s box (§12.2), which Table
        // 147 defaults to the crop box and 0 of the 974 corpus documents move. `pdftotext`
        // knows nothing of viewer preferences, so the one document that did state one would
        // be normalised into a frame the reference is not answering in — refused instead.
        if page
            .display_box
            .iter()
            .zip(&page.crop_box)
            .any(|(view, crop)| (view - crop).abs() > 0.01)
        {
            return None;
        }
        Some(Self {
            width: f64::from(page.crop_box[2] - page.crop_box[0]),
            height: f64::from(page.crop_box[3] - page.crop_box[1]),
            rotate: page.rotate,
            user_unit: f64::from(page.user_unit),
        })
    }

    /// The page's size as displayed — rotated and `/UserUnit`-scaled — which is the frame
    /// `mutool` states its coordinates in.
    fn displayed(&self) -> (f64, f64) {
        if self.rotate == 90 || self.rotate == 270 {
            (self.height * self.user_unit, self.width * self.user_unit)
        } else {
            (self.width * self.user_unit, self.height * self.user_unit)
        }
    }

    /// The displayed height in unscaled points, which the y flip below is about.
    fn displayed_height_in_points(&self) -> f64 {
        if self.rotate == 90 || self.rotate == 270 {
            self.width
        } else {
            self.height
        }
    }

    /// Maps a display-space point (y up, rotated, `/UserUnit`-scaled) into the reference
    /// frame: **the displayed page in unscaled points**, y down from its top-left corner —
    /// rotated by `/Rotate`, *not* scaled by `/UserUnit`.
    ///
    /// That frame is `pdftotext -bbox`'s own, and it was established by measurement rather
    /// than read anywhere, because the reference's two statements disagree: its `<page>`
    /// element states the **unrotated** crop box size while its `<word>` coordinates are in
    /// the **rotated** frame. `hello_world_rotated.pdf` (`/Rotate 90`) is the witness — page
    /// stated 612x792, the word boxes matching `mutool`'s 792x612 frame to the point — with
    /// `issue14415.pdf` (180) and `bug1947248_text.pdf` (`/UserUnit 3`, boxes unscaled)
    /// pinning the other two mechanisms. This instrument's own first run caught it, on
    /// exactly trap 3's rule: check what question a reference answers before reading its
    /// answer — and a reference's stated frame and its coordinates can answer different ones.
    /// ADR 0323's Finding 1 read the *size* lines and was right about them; the coordinates
    /// rotate where the size does not.
    ///
    /// So the map from our display space is the `/UserUnit` division and the y flip about the
    /// displayed height — trap 12a's flip, made explicit where it happens — and no rotation
    /// at all, because the display list's space is already the displayed page's.
    fn reference_point(&self, x: f64, y: f64) -> (f64, f64) {
        let (x, y) = (x / self.user_unit, y / self.user_unit);
        (x, self.displayed_height_in_points() - y)
    }

    /// Maps a point in `mutool`'s frame into the reference frame.
    ///
    /// `mutool` answers in the same displayed frame with the same y direction and differs by
    /// exactly one mechanism: it multiplies Table 31's `/UserUnit` in and `pdftotext` does
    /// not.
    fn reference_point_from_mupdf(&self, x: f64, y: f64) -> (f64, f64) {
        (x / self.user_unit, y / self.user_unit)
    }
}

/// This tree's words on a page, in the reference frame.
///
/// A word is a maximal whitespace-free run of [`pdf_model::Interpretation::text`], and its box
/// is the union of the [`pdf_model::content::Placed`] quads whose readback spans intersect it —
/// the same geometry a drag's selection shapes are built from, which is what makes this the
/// thing worth judging. A word none of whose codes carries geometry (an invisible mode places
/// its glyphs, so that is not the common case) is skipped.
fn our_words(interpretation: &pdf_model::Interpretation, frame: &Frame) -> Vec<WordBox> {
    let text = &interpretation.text;
    let mut words = Vec::new();
    let mut start: Option<usize> = None;
    let bytes: Vec<(usize, char)> = text.char_indices().collect();
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for &(at, c) in &bytes {
        if c.is_whitespace() {
            if let Some(from) = start.take() {
                runs.push((from, at));
            }
        } else if start.is_none() {
            start = Some(at);
        }
    }
    if let Some(from) = start {
        runs.push((from, text.len()));
    }

    for (from, to) in runs {
        let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
        let (mut x1, mut y1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut placed_any = false;
        for placed in &interpretation.text_layer {
            if placed.span.start >= to || placed.span.end <= from || placed.span.is_empty() {
                continue;
            }
            placed_any = true;
            for corner in placed.quad.chunks_exact(2) {
                let (x, y) = frame.reference_point(f64::from(corner[0]), f64::from(corner[1]));
                (x0, y0) = (x0.min(x), y0.min(y));
                (x1, y1) = (x1.max(x), y1.max(y));
            }
        }
        if placed_any {
            words.push(WordBox {
                text: fold(&text[from..to]),
                x0,
                y0,
                x1,
                y1,
            });
        }
    }
    words
}

/// Pairs the words whose folded text occurs exactly once on each side.
///
/// Matching by unique text is what makes the comparison independent of both sides'
/// segmentation and order: a word an extractor split differently simply fails to match, which
/// Finding 5 prices as a printed matched fraction rather than as a verdict, and a word that
/// occurs twice proves nothing about *which* occurrence is which. Words under three characters
/// are dropped for the same reason [`reference_words`] drops them.
fn unique_matches<'a>(
    ours: &'a [WordBox],
    reference: &'a [WordBox],
) -> Vec<(&'a WordBox, &'a WordBox)> {
    let once = |words: &'a [WordBox]| -> BTreeMap<&'a str, &'a WordBox> {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for word in words {
            if word.text.chars().count() >= 3 {
                *counts.entry(word.text.as_str()).or_default() += 1;
            }
        }
        words
            .iter()
            .filter(|word| counts.get(word.text.as_str()) == Some(&1))
            .map(|word| (word.text.as_str(), word))
            .collect()
    };
    let ours = once(ours);
    let reference = once(reference);
    ours.into_iter()
        .filter_map(|(text, word)| reference.get(text).map(|matched| (word, *matched)))
        .collect()
}

/// One matched pair's measurements, in the vocabulary the bounds are stated in.
struct PairDelta {
    /// Left edge against left edge, in points.
    dx0: f64,
    /// Right edge against right edge, in points.
    dx1: f64,
    /// Vertical centre against vertical centre, in points.
    dcy: f64,
    /// [`Self::dcy`] relative to the mean of the two boxes' heights.
    relative_centre: f64,
}

impl PairDelta {
    fn of(left: &WordBox, right: &WordBox) -> Option<Self> {
        let height = f64::midpoint(left.height(), right.height());
        if height <= 0.0 {
            // A degenerate box measures nothing; `stext` writes zero-height quads for some
            // combining marks and this instrument declines to divide by them.
            return None;
        }
        let dcy = (left.centre_y() - right.centre_y()).abs();
        Some(Self {
            dx0: (left.x0 - right.x0).abs(),
            dx1: (left.x1 - right.x1).abs(),
            dcy,
            relative_centre: dcy / height,
        })
    }

    /// Whether the pair is inside both bounds: horizontal edges tight, vertical centre
    /// relative to the word's height, vertical extent deliberately unjudged.
    fn in_bounds(&self) -> bool {
        self.dx0 <= HORIZONTAL_BOUND
            && self.dx1 <= HORIZONTAL_BOUND
            && self.relative_centre <= VERTICAL_CENTRE_BOUND
    }
}

/// The `p`-th percentile of an unsorted sample, by nearest rank.
fn percentile(sample: &mut [f64], p: f64) -> f64 {
    if sample.is_empty() {
        return f64::NAN;
    }
    sample.sort_by(f64::total_cmp);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "a rank in a sample whose length fits in memory"
    )]
    let rank = ((p / 100.0 * sample.len() as f64).ceil() as usize).clamp(1, sample.len()) - 1;
    sample[rank]
}

/// Prints one measure's spread in the derivation's own format.
fn print_spread(name: &str, sample: &mut [f64]) {
    println!(
        "  {name}: median {:.4}  p90 {:.4}  p99 {:.4}  max {:.4}",
        percentile(sample, 50.0),
        percentile(sample, 90.0),
        percentile(sample, 99.0),
        percentile(sample, 100.0),
    );
}

/// Where an extraction is remembered: the reference cache's directory, one level down.
///
/// The same variable and default as the oracle's render cache, because it is the same kind of
/// memory under the same rules (trap 10a) — `PDFREF_CACHE` names a directory, and only the
/// literal `off` disables it.
fn extraction_cache() -> ExtractionCache {
    let default = Path::new(env!("CARGO_TARGET_TMPDIR")).join("pdfref-cache");
    let root = match std::env::var("PDFREF_CACHE") {
        Ok(value) if value.eq_ignore_ascii_case("off") => return ExtractionCache::disabled(),
        Ok(value) if !value.trim().is_empty() => PathBuf::from(value),
        _ => default,
    };
    ExtractionCache::at(root.join("extraction"))
}

/// Why a document is off the judged set. Every one of these is printed with its count,
/// because a refusal costs the instrument a document and that arithmetic is trap 11's.
type Refusal = &'static str;

/// Runs one extractor over one page through the cache, folding failure kinds into refusals.
fn extracted(
    cache: &ExtractionCache,
    extractor: Extractor,
    path: &Path,
    work_dir: &Path,
) -> Result<ExtractedPage, Refusal> {
    let text = cache
        .extract(extractor, path, 1, work_dir)
        .map_err(|error| match (extractor, error) {
            (Extractor::PopplerBoxes, ExtractionError::TimedOut { .. }) => {
                "pdftotext exceeded its budget"
            }
            (Extractor::PopplerBoxes, _) => "pdftotext refused the document",
            (Extractor::MuPdfText, ExtractionError::TimedOut { .. }) => {
                "mutool exceeded its budget"
            }
            (Extractor::MuPdfText, _) => "mutool refused the document",
        })?;
    let page = match extractor {
        Extractor::PopplerBoxes => poppler_page(&text),
        Extractor::MuPdfText => mupdf_page(&text),
    };
    match extractor {
        Extractor::PopplerBoxes => page.ok_or("pdftotext produced no page"),
        Extractor::MuPdfText => page.ok_or("mutool produced no page"),
    }
}

/// This tree's frame for a document's page one, or the refusal naming why not.
fn our_frame(path: &Path) -> Result<(Document, pdf_model::Page), Refusal> {
    let bytes = std::fs::read(path).map_err(|_| "unreadable file")?;
    let document = Document::open(bytes).map_err(|_| "unopenable to this tree")?;
    let page = {
        let pages = pdf_model::Pages::new(&document);
        pages.get(0).ok_or("no page one")?
    };
    Ok((document, page))
}

/// What one document contributed: a refusal by reason, or its matched pairs.
enum Contribution {
    Refused(Refusal),
    Judged {
        name: String,
        pairs: Vec<PairDelta>,
        /// How many of the reference's words found a unique match — Finding 5's printed
        /// fraction, kept as the two counts so the caller can aggregate honestly.
        matched: usize,
        reference_words: usize,
    },
}

/// Judges one corpus document's page one against `pdftotext -bbox -cropbox`.
fn judge_against_poppler(path: &Path, cache: &ExtractionCache, work_dir: &Path) -> Contribution {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let (document, page) = match our_frame(path) {
        Ok(opened) => opened,
        Err(reason) => return Contribution::Refused(reason),
    };
    let Some(frame) = Frame::of(&page) else {
        return Contribution::Refused("a /ViewArea departs from the crop box");
    };
    let reference = match extracted(cache, Extractor::PopplerBoxes, path, work_dir) {
        Ok(page) => page,
        Err(reason) => return Contribution::Refused(reason),
    };
    // The frame audit. `pdftotext -bbox -cropbox` *states* the unrotated, unscaled crop box
    // as its page size — even though its word boxes are in the rotated frame, which is the
    // measured quirk `Frame::reference_point` records — so the unrotated crop is what the
    // stated size is audited against. A mismatch means the reference read a different box
    // than §7.7.3.3's crop for this page, and judging across it would manufacture spread
    // (ADR 0323 Finding 1).
    if (reference.width - frame.width).abs() > FRAME_TOLERANCE
        || (reference.height - frame.height).abs() > FRAME_TOLERANCE
    {
        return Contribution::Refused("frame mismatch against pdftotext");
    }
    if reference.words.is_empty() {
        return Contribution::Refused("no words in the reference");
    }
    let interpretation = pdf_model::interpret(&document, &page);
    let ours = our_words(&interpretation, &frame);
    if ours.is_empty() {
        return Contribution::Refused("no words in our readback");
    }
    let paired = unique_matches(&ours, &reference.words);
    if paired.is_empty() {
        return Contribution::Refused("no unique matches");
    }
    let matched = paired.len();
    let reference_words = reference.words.len();
    Contribution::Judged {
        name,
        pairs: paired
            .iter()
            .filter_map(|(ours, reference)| PairDelta::of(ours, reference))
            .collect(),
        matched,
        reference_words,
    }
}

/// The word boxes this tree's text layer places agree with `pdftotext`'s, where the two frames
/// agree and the words match uniquely — ADR 0323's instrument 1, geometry half.
///
/// **Prints, and does not yet ratchet.** ADR 0323's rule (and `doc/todo/05`'s): an
/// instrument's numbers enter `doc/todo/02` §2 only once they have held across rounds. What is
/// asserted today is that the instrument has a population — a judged set that silently went
/// empty would otherwise print a perfect verdict over nothing.
///
/// The verdict per matched word, from the measured reference-vs-reference spread this binary
/// re-derives (see [`HORIZONTAL_BOUND`] and [`VERTICAL_CENTRE_BOUND`]): horizontal edges
/// within 0.5 pt, vertical centres within half the word's height, vertical extent and reading
/// order deliberately unjudged (ADR 0323 Findings 3 and 4).
#[test]
#[ignore = "needs the pdf.js submodule and runs pdftotext per document"]
#[expect(
    clippy::too_many_lines,
    reason = "one pass and its report; splitting the printing from the counting would separate \
              the numbers from the arithmetic that justifies them"
)]
fn the_word_boxes_we_place_agree_with_the_references() {
    let corpus = pdfjs_corpus();
    if corpus.is_empty() {
        println!("doc/pdf.js is not checked out: skipped");
        return;
    }
    assert!(
        Extractor::PopplerBoxes.is_available(),
        "pdftotext is required for this test; it comes with poppler"
    );
    let cache = extraction_cache();
    let work_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("extract");

    let started = Instant::now();
    let contributions: Vec<Contribution> = corpus
        .par_iter()
        .map(|path| judge_against_poppler(path, &cache, &work_dir))
        .collect();

    let mut refusals: BTreeMap<Refusal, usize> = BTreeMap::new();
    let mut judged = 0usize;
    let (mut pairs_total, mut pairs_in_bounds) = (0usize, 0usize);
    let mut horizontal: Vec<f64> = Vec::new();
    let mut relative_centres: Vec<f64> = Vec::new();
    let mut matched_fractions: Vec<f64> = Vec::new();
    // (fraction in bounds, name, matched pairs, worst horizontal delta) per judged document.
    let mut ranked: Vec<(f64, String, usize, f64)> = Vec::new();

    for contribution in contributions {
        match contribution {
            Contribution::Refused(reason) => *refusals.entry(reason).or_default() += 1,
            Contribution::Judged {
                name,
                pairs,
                matched,
                reference_words,
            } => {
                judged += 1;
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "word counts on one page are far below f64's exact integer limit"
                )]
                matched_fractions.push(matched as f64 / reference_words.max(1) as f64);
                let in_bounds = pairs.iter().filter(|pair| pair.in_bounds()).count();
                pairs_total += pairs.len();
                pairs_in_bounds += in_bounds;
                let worst_dx = pairs
                    .iter()
                    .map(|pair| pair.dx0.max(pair.dx1))
                    .fold(0.0f64, f64::max);
                for pair in &pairs {
                    horizontal.push(pair.dx0);
                    horizontal.push(pair.dx1);
                    relative_centres.push(pair.relative_centre);
                }
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "word counts on one page are far below f64's exact integer limit"
                )]
                ranked.push((
                    in_bounds as f64 / pairs.len().max(1) as f64,
                    name,
                    pairs.len(),
                    worst_dx,
                ));
            }
        }
    }

    ranked.sort_by(|left, right| left.0.total_cmp(&right.0));
    // Every document with a word out of bounds, not the worst ten: this list *is* the ratchet
    // below, and a ratchet whose population is printed truncated cannot be maintained from the
    // gate's own output.
    println!("documents not fully in bounds:");
    for (fraction, name, pairs, worst_dx) in ranked.iter().filter(|(f, ..)| *f < 1.0) {
        println!(
            "  {name}: {:.1}% of {pairs} matched words in bounds, worst horizontal delta \
             {worst_dx:.2} pt",
            fraction * 100.0
        );
    }
    println!("\nspread of our boxes against pdftotext's, over every matched pair:");
    print_spread("horizontal edge delta (pt)  ", &mut horizontal);
    print_spread("vertical centre / word height", &mut relative_centres);
    println!(
        "matched-unique fraction per judged document: median {:.2}, p10 {:.2} (Finding 5: \
         segmentation is each extractor's own, so only unique matches are compared)",
        percentile(&mut matched_fractions.clone(), 50.0),
        percentile(&mut matched_fractions, 10.0),
    );

    // Trap 11's arithmetic, stated in the gate's own output: every refusal is a document the
    // instrument does not judge, by name and reason, so the judged set cannot shrink silently.
    let refused: usize = refusals.values().sum();
    println!(
        "\n{judged} of {} documents judged in {:.1}s; {refused} refused, each a document off \
         the judged set:",
        corpus.len(),
        started.elapsed().as_secs_f64(),
    );
    for (reason, count) in &refusals {
        println!("  {count:4}  {reason}");
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "corpus word counts are far below f64's exact integer limit"
    )]
    let fraction = pairs_in_bounds as f64 / pairs_total.max(1) as f64;
    println!(
        "verdict: {pairs_in_bounds}/{pairs_total} matched words in bounds ({:.2}%), horizontal \
         edges within {HORIZONTAL_BOUND} pt and vertical centres within {VERTICAL_CENTRE_BOUND} \
         of the word's height; {} of {judged} documents fully in bounds",
        fraction * 100.0,
        ranked.iter().filter(|(f, ..)| *f >= 1.0).count(),
    );

    let stats = cache.statistics();
    println!(
        "extraction cache: {} hits, {} misses, {} remembered timeouts",
        stats.hits, stats.misses, stats.remembered_timeouts
    );

    assert!(pairs_total > 0, "no word was matched anywhere");
    assert!(
        judged >= JUDGED_FLOOR,
        "the judged set fell to {judged} from {JUDGED_FLOOR}: {refused} documents are refused \
         above, and a document off the judged set is a document this instrument stopped judging"
    );
    let mut below: Vec<&str> = ranked
        .iter()
        .filter(|(fraction, ..)| *fraction < 1.0)
        .map(|(_, name, ..)| name.as_str())
        .collect();
    below.sort_unstable();
    let newly: Vec<&&str> = below
        .iter()
        .filter(|name| !SELECTION_BELOW_FLOOR.contains(name))
        .collect();
    let fixed: Vec<&&str> = SELECTION_BELOW_FLOOR
        .iter()
        .filter(|name| !below.contains(name))
        .collect();
    assert!(
        newly.is_empty(),
        "{} document(s) newly carrying a word out of bounds: {newly:?}",
        newly.len()
    );
    assert!(
        fixed.is_empty(),
        "{} document(s) no longer out of bounds: {fixed:?}. Delete them from \
         SELECTION_BELOW_FLOOR: a fixed page must not be able to come back.",
        fixed.len()
    );
}

/// The environment variable that asks for the derivation below; `--ignored` alone must not.
const SELECTION_SPREAD_IS_ASKED_FOR: &str = "PDFVIEWER_SELECTION_SPREAD";

/// One document's contribution to the derivation: a refusal, or the matched pairs plus the
/// height ratios Finding 3 reports.
enum Spread {
    Refused(Refusal),
    Measured(Vec<PairDelta>, Vec<f64>),
}

/// Where [`HORIZONTAL_BOUND`] and [`VERTICAL_CENTRE_BOUND`] come from, re-derived from the
/// corpus rather than remembered — `pdftotext -bbox -cropbox` against `mutool draw -F stext`,
/// two positional extractors sharing no code, over the same matcher and the same frame audit
/// the verdict uses.
///
/// It prints, and asserts only that it had a population: a number here is evidence for moving
/// a bound, never itself a gate (the same rule as the raster derivation's, and `oracle.rs`
/// states it at length). The guard is an environment variable for ADR 0282's reason —
/// `-- --ignored` runs every ignored test in the binary, and a derivation that walked the
/// corpus beside the gate under `rayon` would double the gate line's wall clock. The guard is
/// here rather than in an invocation because an invocation can be copied without its guard and
/// a test cannot be run without itself.
#[test]
#[ignore = "runs pdftotext and mutool per corpus document; run explicitly, in release"]
#[expect(
    clippy::too_many_lines,
    reason = "one pass and its report; splitting the printing from the counting would separate \
              the numbers from the arithmetic that justifies them"
)]
fn the_selection_bounds_against_the_references_own_spread() {
    if std::env::var_os(SELECTION_SPREAD_IS_ASKED_FOR).is_none() {
        println!(
            "skipped: this is a derivation rather than a gate — set \
             {SELECTION_SPREAD_IS_ASKED_FOR}=1 to run it."
        );
        return;
    }
    let corpus = pdfjs_corpus();
    if corpus.is_empty() {
        println!("doc/pdf.js is not checked out: skipped");
        return;
    }
    for extractor in [Extractor::PopplerBoxes, Extractor::MuPdfText] {
        assert!(
            extractor.is_available(),
            "{} is required for this derivation; it comes with {}",
            extractor.program(),
            extractor.package_hint()
        );
    }
    let cache = extraction_cache();
    let work_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("extract");

    let spread_of = |path: &Path| -> Spread {
        let (_, page) = match our_frame(path) {
            Ok(opened) => opened,
            Err(reason) => return Spread::Refused(reason),
        };
        let Some(frame) = Frame::of(&page) else {
            return Spread::Refused("a /ViewArea departs from the crop box");
        };
        let poppler = match extracted(&cache, Extractor::PopplerBoxes, path, &work_dir) {
            Ok(page) => page,
            Err(reason) => return Spread::Refused(reason),
        };
        let mupdf = match extracted(&cache, Extractor::MuPdfText, path, &work_dir) {
            Ok(page) => page,
            Err(reason) => return Spread::Refused(reason),
        };
        // The audit, against each extractor's own *stated* size: `pdftotext` states the
        // unrotated, unscaled crop box (while its boxes are in the rotated frame — the quirk
        // `Frame::reference_point` records), and `mutool` states the page as displayed. The
        // two statements differ by exactly the mechanisms Finding 1 names, and each is
        // checked against what this tree reads out of §7.7.3.3's entries rather than against
        // the other extractor.
        if (poppler.width - frame.width).abs() > FRAME_TOLERANCE
            || (poppler.height - frame.height).abs() > FRAME_TOLERANCE
        {
            return Spread::Refused("frame mismatch against pdftotext");
        }
        let (displayed_width, displayed_height) = frame.displayed();
        if (mupdf.width - displayed_width).abs() > FRAME_TOLERANCE
            || (mupdf.height - displayed_height).abs() > FRAME_TOLERANCE
        {
            return Spread::Refused("frame mismatch against mutool");
        }
        if poppler.words.is_empty() || mupdf.words.is_empty() {
            return Spread::Refused("no words in one of the references");
        }
        let normalised: Vec<WordBox> = mupdf
            .words
            .iter()
            .map(|word| {
                let corners = [
                    frame.reference_point_from_mupdf(word.x0, word.y0),
                    frame.reference_point_from_mupdf(word.x1, word.y1),
                ];
                WordBox {
                    text: word.text.clone(),
                    x0: corners[0].0.min(corners[1].0),
                    y0: corners[0].1.min(corners[1].1),
                    x1: corners[0].0.max(corners[1].0),
                    y1: corners[0].1.max(corners[1].1),
                }
            })
            .collect();
        let matches = unique_matches(&poppler.words, &normalised);
        if matches.is_empty() {
            return Spread::Refused("no unique matches");
        }
        let ratios = matches
            .iter()
            .filter(|(poppler, mupdf)| poppler.height() > 0.0 && mupdf.height() > 0.0)
            .map(|(poppler, mupdf)| poppler.height() / mupdf.height())
            .collect();
        Spread::Measured(
            matches
                .iter()
                .filter_map(|(poppler, mupdf)| PairDelta::of(poppler, mupdf))
                .collect(),
            ratios,
        )
    };

    let started = Instant::now();
    let spreads: Vec<Spread> = corpus.par_iter().map(|path| spread_of(path)).collect();

    let mut refusals: BTreeMap<Refusal, usize> = BTreeMap::new();
    let mut judged = 0usize;
    let mut horizontal: Vec<f64> = Vec::new();
    let mut centres: Vec<f64> = Vec::new();
    let mut relative_centres: Vec<f64> = Vec::new();
    let mut height_ratios: Vec<f64> = Vec::new();
    for spread in spreads {
        match spread {
            Spread::Refused(reason) => *refusals.entry(reason).or_default() += 1,
            Spread::Measured(pairs, ratios) => {
                judged += 1;
                for pair in pairs {
                    horizontal.push(pair.dx0);
                    horizontal.push(pair.dx1);
                    centres.push(pair.dcy);
                    relative_centres.push(pair.relative_centre);
                }
                height_ratios.extend(ratios);
            }
        }
    }

    println!(
        "pdftotext -bbox -cropbox against mutool -F stext: {} matched word pairs over {judged} \
         documents in {:.1}s",
        relative_centres.len(),
        started.elapsed().as_secs_f64()
    );
    let refused: usize = refusals.values().sum();
    println!("{refused} documents refused, each off the derivation's population:");
    for (reason, count) in &refusals {
        println!("  {count:4}  {reason}");
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "pair counts are far below f64's exact integer limit"
    )]
    let rejected = |sample: &[f64], bound: f64| {
        100.0 * sample.iter().filter(|d| **d > bound).count() as f64 / sample.len().max(1) as f64
    };
    let horizontal_rejects = rejected(&horizontal, HORIZONTAL_BOUND);
    let vertical_rejects = rejected(&relative_centres, VERTICAL_CENTRE_BOUND);
    println!("\nthe references' own spread, which the bounds must admit:");
    print_spread("horizontal edge delta (pt)  ", &mut horizontal);
    print_spread("vertical centre delta (pt)  ", &mut centres);
    print_spread("vertical centre / word height", &mut relative_centres);
    print_spread("word-box height ratio        ", &mut height_ratios);
    println!(
        "\nshare of reference pairs outside each bound: horizontal {HORIZONTAL_BOUND} pt \
         rejects {horizontal_rejects:.2}%, vertical centre {VERTICAL_CENTRE_BOUND} of the \
         word's height rejects {vertical_rejects:.2}% \
         (the vertical *extent* is excluded from any verdict on this table's own evidence: \
         the height ratio row is convention against convention, ADR 0323 Finding 3)"
    );

    assert!(judged > 0, "the derivation has no population");
    assert!(!horizontal.is_empty(), "no word was matched anywhere");
}
