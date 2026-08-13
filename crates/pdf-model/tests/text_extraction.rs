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
/// and `/Adobe-Korea1-UCS2 usecmap` for the rest, which §9.10.3 permits — "UseCMap , which may be
/// used if the CMap is based on another ToUnicode CMap" — and which nothing here followed, so the
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
/// - `issue1350.pdf` (1): the reference reads `beginnerÕs`, which is MacRomanEncoding 0xD5 taken
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
