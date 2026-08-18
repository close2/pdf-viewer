//! Quotations of the standard in this project's own Markdown documents.
//!
//! # The population nothing read
//!
//! `CLAUDE.md` principle 5 binds quotation marks rather than doc comments — "[a]nything less
//! than verbatim is prose *without* quotation marks" — and the gate reads one population:
//! rustdoc blockquotes under `crates/`, `tools/` and `fuzz/`. `doc/todo/48` names the rest, and
//! the largest of them is this one: every quotation of the standard in `doc/*.md`, in
//! `doc/todo/`, in `doc/history/` and in the ADRs. Hundreds of rounds wrote it, each quoting
//! from memory or from a conversion, and until this module existed **not one word of it had ever
//! been compared with anything**.
//!
//! # Why it is a sweep and not a gate
//!
//! For ADR 0249's reason, one population over: a rustdoc `> ` under a clause number is
//! unambiguous, and a pair of `"` in a paragraph means nothing at all. These documents quote
//! `CLAUDE.md`, the project owner, another renderer's output, a report this program prints, a
//! test's name and their own retired wording — and no syntax says which is which. A checker that
//! failed the build on all of it would be switched off in a round.
//!
//! **What makes the output readable is the discriminator rather than the match**, and it is
//! ADR 0249's: a claim this project invented shares almost no words with the standard, and a
//! misquotation shares most of them. So [`Conversion::judge`] reports only a quotation that
//! matches the standard for at least [`MIN_MATCH`] words *and* at least half its length, and
//! then diverges — with the standard's own continuation printed beside it, which is the part a
//! person needs in order to write the correction.
//!
//! # Three properties of `doc/md/` that shape the comparison
//!
//! Every one of them was met the hard way by an earlier round, and each is a reason a miss here
//! is a question rather than a verdict:
//!
//! - **It is a conversion.** Words break across lines — `hierarch y`, `text-tospeech` — so a
//!   quotation that is exactly right cannot be found by a comparison that keeps spaces. This one
//!   takes them out ([`folded`]), which is ADR 0253's repair for the same problem.
//! - **It drops content.** A silence in `doc/md/` is not a silence in the standard, so nothing
//!   here is corrected without `pdftotext -layout` over the PDF in `doc/` first.
//! - **It writes `Table 87 -Additional entries` where the standard prints an em dash.** Dash
//!   shapes are folded together for that reason, and for the plain one that a writer typing a
//!   quotation into a document types the dash on the keyboard.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::quote;

/// How many words of the standard a quotation must match before a divergence is worth printing.
///
/// ADR 0249's number, and its argument: below five words two different sentences agree by
/// accident all the time — the standard says "shall be the same as" on many pages — so a lower
/// threshold reports the noise and buries the finding.
pub const MIN_MATCH: usize = 5;

/// The shortest span of prose that is treated as a quotation at all.
///
/// [`quote::MIN_WORDS`], which is where the rule lives now that three populations share it.
pub const MIN_WORDS: usize = quote::MIN_WORDS;

/// The directories under `doc/` this sweep does not read.
///
/// Stated rather than quietly skipped, because an instrument that does not look somewhere should
/// say where. `md` is **the standard itself**, which is the other side of every comparison here;
/// the rest are third-party checkouts this project did not write and does not correct — the four
/// test corpora, the `pdf.js` source tree, the Arlington model, and a renderer kept beside the
/// tree for comparison.
pub const NOT_READ: [&str; 5] = [
    "md",
    "corpora",
    "pdf.js",
    "arlington-pdf-model",
    "rasterrocket",
];

/// The one file under `doc/` this sweep does not read.
///
/// `doc/errata.md` is what `tools/spec-errata` emits from the specifications' own annotations
/// (ADR 0252). Every quotation in it is the standard's by construction, because a program lifted
/// it out of the PDF, so checking it would measure the extractor rather than a claim.
pub const NOT_A_DOCUMENT: &str = "errata.md";

/// How a document marks a quotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A `>` blockquote — the shape `CLAUDE.md` asks a rustdoc quotation to take, and the one
    /// these documents use for a passage set apart from the prose around it.
    Blockquote,
    /// A pair of `"` inside ordinary prose, which is the commoner shape by an order of
    /// magnitude and carries exactly the same claim.
    Quoted,
    /// A pair of `'`, which carries the same claim again and which nothing read until
    /// [`quote::quoted_spans`] learned the rule that tells one from an apostrophe.
    Apostrophed,
}

impl Shape {
    /// The shape a [`quote::Mark`] makes.
    #[must_use]
    pub fn of(mark: quote::Mark) -> Self {
        match mark {
            quote::Mark::Double => Self::Quoted,
            quote::Mark::Single => Self::Apostrophed,
        }
    }
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Blockquote => "blockquote",
            Self::Quoted => "quoted",
            Self::Apostrophed => "single-quoted",
        })
    }
}

/// One quotation, as a document writes it.
#[derive(Debug, Clone)]
pub struct Written {
    /// The document it is written in.
    pub file: PathBuf,
    /// The 1-based line the quotation starts on.
    pub line: usize,
    /// Which of the two shapes marks it.
    pub shape: Shape,
    /// The quoted text, with the `>` markers removed and the lines joined.
    pub text: String,
}

/// What the sweep concludes about one quotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Found in one of the specifications, by [`folded`]'s comparison.
    Verbatim,
    /// Matches the standard, then stops matching — the shape a misquotation has.
    Diverges {
        /// How many words matched.
        matched: usize,
        /// How many words the longest segment of the quotation has.
        total: usize,
        /// The standard's own words from where the match began, for the correction to be
        /// written from.
        standard: String,
    },
    /// Shares too little with the specifications to be a quotation of one.
    ///
    /// The overwhelming majority, and the reason this is a sweep: these documents quote
    /// `CLAUDE.md`, the project owner, another implementation and themselves.
    Unrelated,
}

/// Why the sweep could not read what it needs.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file or directory could not be read.
    #[error("cannot read {path}: {source}")]
    Unreadable {
        /// The path that was tried.
        path: PathBuf,
        /// The underlying failure.
        source: std::io::Error,
    },
    /// The conversion directory held no specification at all.
    #[error("{path} holds no Markdown conversion; the specifications are not unpacked")]
    NoConversion {
        /// The directory that was read.
        path: PathBuf,
    },
}

/// The words of a passage, reduced to what two extractions of the same sentence agree on.
///
/// [`quote::normalise`] first, which is the gate's own comparison — layout, Markdown markup and
/// typography — and then four further foldings, each of which can only make the comparison
/// *coarser* and so can only ever hide a finding, never invent one:
///
/// - **Spaces come out.** ADR 0253's repair: `doc/md/` breaks words across lines, and a
///   comparison that keeps whitespace calls a passage absent that is there in full.
/// - **Case is folded.** A writer quoting the middle of a sentence lowers its first letter,
///   because a quotation starting mid-sentence reads wrongly with a capital, and
///   `CLAUDE.md`'s rule is about the words.
/// - **Square brackets come out.** The same practice written the other way — `"[t]he
///   implementation of such a schema driven page generation"` is `CLAUDE.md`'s own spelling of
///   an altered first letter, and it is exact quotation with a mark round the change.
/// - **Dashes come out, having first been folded together.** The conversion prints `Table 87
///   -Additional entries` where the standard sets an em dash, so a person typing a quotation
///   into a document types whichever dash the keyboard offers — and the conversion **loses the
///   hyphen of a word it breaks across a line**, which the spaces coming out cannot repair.
///   `pdftotext -layout` over the PDF finds `text-to-speech`, `implementation-dependent` and
///   `marked-content` where `doc/md/` has `text-tospeech`, `implementationdependent` and
///   `markedcontent`; those three were three of the nine divergences the ledger's own notes
///   reported on the first run of the sweep that reads them.
/// - **A fraction slash is a slash.** Table 145's `/PrintScaling` neighbourhood sets `1 ⁄ 72`
///   with U+2044 and spaces round it, and a note quoting it types `1/72`.
///
/// The quotation marks used to be folded together here as a fifth. They are not, because
/// [`quote::normalise`] now drops them outright one step earlier — a quotation delimited by
/// `"` cannot contain a `"`, and the conversion does not spell the standard's own mark
/// consistently anyway; §14.8.6 is the witness and that function carries it.
#[must_use]
pub fn folded(text: &str) -> String {
    quote::normalise(text)
        .chars()
        .map(fold)
        .filter(|character| kept(*character))
        .flat_map(char::to_lowercase)
        .collect()
}

/// The one-character foldings [`folded`] and [`Conversion::of`] must agree on.
fn fold(character: char) -> char {
    match character {
        '\u{2010}'..='\u{2015}' | '\u{2212}' => '-',
        '\u{2044}' => '/',
        character => plain(character),
    }
}

/// Whether a character survives the fold, which [`folded`] and [`Conversion::of`] must also
/// agree on — and which is applied **after** [`fold`], so that every dash shape is dropped
/// rather than only the one on the keyboard.
fn kept(character: char) -> bool {
    !character.is_whitespace() && !matches!(character, '[' | ']' | '-')
}

/// A Mathematical Alphanumeric Symbol, folded back to the letter or digit it styles.
///
/// ISO sets every variable in a formula in mathematical italic — Table 191's `𝑓 (𝑥) = 1 – 𝑥` is
/// U+1D453 and U+1D465 rather than `f` and `x` — and a document quoting a formula types the
/// letters that are on the keyboard. The same letters in another font is exactly what
/// [`quote::normalise`] already folds a curly quotation mark for, and the standard's formulae are
/// a population rather than a case: clause 11's blend functions, §7.10's shadings and §10.5's
/// transfer functions are all set this way.
///
/// The block is thirteen Latin styles of 52 characters — A to Z, then a to z — followed by the
/// Greek styles, which are left alone because folding a Greek letter onto a Greek letter buys
/// nothing, and five runs of ten digits. The holes in the script, fraktur and double-struck runs
/// are code points Unicode left unassigned because the letter was already in Letterlike Symbols,
/// so the arithmetic is exact over every code point that exists.
fn plain(character: char) -> char {
    /// U+1D400, MATHEMATICAL BOLD CAPITAL A: the first of the thirteen Latin styles.
    const LATIN: u32 = 0x0001_d400;
    /// The last of them, U+1D6A3 — thirteen styles of fifty-two characters.
    const LATIN_LAST: u32 = 0x0001_d6a3;
    /// U+1D7CE, MATHEMATICAL BOLD DIGIT ZERO, and the four styles of digit after it.
    const DIGITS: u32 = 0x0001_d7ce;
    /// The last of them, U+1D7FF.
    const DIGITS_LAST: u32 = 0x0001_d7ff;

    let code = character as u32;
    if (LATIN..=LATIN_LAST).contains(&code) {
        let letter = (code.saturating_sub(LATIN) % 52) as u8;
        let base = if letter < 26 {
            b'A'
        } else {
            b'a'.wrapping_sub(26)
        };
        return char::from(base.wrapping_add(letter));
    }
    if (DIGITS..=DIGITS_LAST).contains(&code) {
        let digit = (code.saturating_sub(DIGITS) % 10) as u8;
        return char::from(b'0'.wrapping_add(digit));
    }
    character
}

/// The specifications under `doc/md/`, as one body of text a quotation can be looked for in.
///
/// All fourteen rather than ISO 32000-2 alone, for ADR 0249's reason: these documents quote the
/// technical specifications, the two accessibility standards and the three PDF Association
/// application notes, and a sweep that knew only the first would report every quotation of the
/// others as a divergence.
#[derive(Debug)]
pub struct Conversion {
    /// Every conversion's normalised text, joined, with a separator between documents.
    spaced: String,
    /// The same text under [`folded`].
    folded: String,
    /// For each byte of [`Self::folded`], the byte of [`Self::spaced`] it was folded from.
    ///
    /// What turns a match into something a person can read: the sweep searches the folded text,
    /// where `theregiontowhich` is one word, and prints the spaced text, where it is four.
    origin: Vec<u32>,
}

/// What separates one specification from the next, so that a quotation cannot match across the
/// join between two documents.
///
/// A control character survives [`folded`] — it is neither whitespace nor a bracket — and no
/// quotation anybody writes contains one.
const BETWEEN_DOCUMENTS: char = '\u{1}';

impl Conversion {
    /// Reads every specification directly under `directory`.
    ///
    /// # Errors
    ///
    /// [`Error::Unreadable`] where the directory or one of its files cannot be read, and
    /// [`Error::NoConversion`] where it holds no `.md` file at all — which means the
    /// specifications are not unpacked, and every verdict below would otherwise be `Unrelated`
    /// for the wrong reason.
    pub fn read(directory: &Path) -> Result<Self, Error> {
        let entries = std::fs::read_dir(directory).map_err(|source| Error::Unreadable {
            path: directory.to_owned(),
            source,
        })?;
        let mut paths = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|source| Error::Unreadable {
                    path: directory.to_owned(),
                    source,
                })?
                .path();
            if path.extension().is_some_and(|extension| extension == "md") {
                paths.push(path);
            }
        }
        paths.sort();
        if paths.is_empty() {
            return Err(Error::NoConversion {
                path: directory.to_owned(),
            });
        }

        let mut spaced = String::new();
        for path in paths {
            let text = std::fs::read_to_string(&path).map_err(|source| Error::Unreadable {
                path: path.clone(),
                source,
            })?;
            spaced.push(BETWEEN_DOCUMENTS);
            spaced.push_str(&quote::normalise(&text));
        }

        Ok(Self::of(spaced))
    }

    /// One already-[`quote::normalise`]d body of text, folded and indexed.
    ///
    /// Folded a character at a time rather than through [`folded`], because the map from a folded
    /// byte back to the byte it came from is what makes the report readable, and that map cannot
    /// be recovered afterwards. The two must agree, which is why the per-character part of both
    /// is [`fold`].
    fn of(spaced: String) -> Self {
        let mut text = String::with_capacity(spaced.len());
        let mut origin = Vec::with_capacity(spaced.len());
        for (at, character) in spaced.char_indices() {
            let character = fold(character);
            if !kept(character) {
                continue;
            }
            for lowered in character.to_lowercase() {
                text.push(lowered);
                let at = u32::try_from(at).unwrap_or(u32::MAX);
                origin.resize(text.len(), at);
            }
        }
        Self {
            spaced,
            folded: text,
            origin,
        }
    }

    /// Whether the specifications hold `quotation` as written.
    ///
    /// Split at the ellipses that stand for what a quotation left out, and each segment required
    /// **in order** — [`quote::occurs_in`]'s rule, so that an elision cannot join two clauses'
    /// worth of text into a sentence the standard does not contain.
    #[must_use]
    pub fn holds(&self, quotation: &str) -> bool {
        let mut rest = self.folded.as_str();
        for segment in segments(quotation) {
            let needle = folded(&segment);
            if needle.is_empty() {
                continue;
            }
            let Some(at) = rest.find(&needle) else {
                return false;
            };
            rest = rest
                .get(at.saturating_add(needle.len())..)
                .unwrap_or_default();
        }
        true
    }

    /// What the sweep concludes about one quotation.
    ///
    /// The divergence is measured on the **longest segment the conversion does not hold**,
    /// rather than on the quotation as a whole. Each segment of an elided quotation claims on
    /// its own to be the standard's words, so a segment that matches for [`MIN_MATCH`] words and
    /// half its length and then stops is the misquotation shape whatever the segments either
    /// side of it do; measuring against the whole would let an ellipsis dilute a wrong sentence
    /// below the threshold.
    ///
    /// A quotation whose every segment *is* the standard's and whose **order** is not passes as
    /// `Unrelated` here, and deliberately: [`Self::holds`] is what reports that, the words are
    /// not misquoted, and a single-segment quotation — which is all but a handful of this
    /// population — cannot reach the case at all.
    #[must_use]
    pub fn judge(&self, quotation: &str) -> Verdict {
        if self.holds(quotation) {
            return Verdict::Verbatim;
        }
        let Some(segment) = segments(quotation)
            .into_iter()
            .filter(|segment| !self.folded.contains(&folded(segment)))
            .max_by_key(|segment| segment.split_whitespace().count())
        else {
            return Verdict::Unrelated;
        };
        let words: Vec<&str> = segment.split_whitespace().collect();
        let total = words.len();
        if total < MIN_MATCH {
            return Verdict::Unrelated;
        }

        // The longest prefix that occurs at all, by bisection. Sound because folding a prefix of
        // the words gives a prefix of the folded string, so a longer prefix that occurs implies
        // every shorter one does — the predicate is monotone and the search cannot step over the
        // boundary. Five probes rather than thirty, over four megabytes each.
        let mut low = MIN_MATCH;
        let mut high = total;
        let Some(mut at) = self.position(&words, low) else {
            return Verdict::Unrelated;
        };
        while low < high {
            let middle = low.saturating_add(high.saturating_sub(low).saturating_add(1) / 2);
            match self.position(&words, middle) {
                Some(found) => {
                    at = found;
                    low = middle;
                }
                None => high = middle.saturating_sub(1),
            }
        }
        if low.saturating_mul(2) < total {
            return Verdict::Unrelated;
        }
        Verdict::Diverges {
            matched: low,
            total,
            standard: self.reading_from(at, &words, low),
        }
    }

    /// Where the first `count` words of `words` occur in the folded conversion, if they do.
    fn position(&self, words: &[&str], count: usize) -> Option<usize> {
        let needle = folded(&words.get(..count)?.join(" "));
        self.folded.find(&needle)
    }

    /// The standard's own words from `at`, far enough past the match to show the divergence.
    ///
    /// Read out of the *spaced* text through [`Self::origin`], because the folded text is what a
    /// search is fast in and not what a person can read.
    fn reading_from(&self, at: usize, words: &[&str], matched: usize) -> String {
        let length = folded(&words.get(..matched).unwrap_or_default().join(" ")).len();
        // Twice the match again, and never less than 120 folded bytes: enough of the standard's
        // continuation to see where the two sentences part company, and bounded so that a long
        // quotation does not print a page.
        let past = length.saturating_mul(2).clamp(120, 400);
        let start = self.origin.get(at).map_or(0, |byte| *byte as usize);
        let end = self
            .origin
            .get(at.saturating_add(length).saturating_add(past))
            .map_or(self.spaced.len(), |byte| *byte as usize);
        let mut text = self
            .spaced
            .get(start..end.max(start))
            .unwrap_or_default()
            .replace(BETWEEN_DOCUMENTS, " ")
            .trim()
            .to_owned();
        text.push('\u{2026}');
        text
    }
}

/// A quotation's segments, split at the ellipses that stand for what it left out.
///
/// Both spellings this project writes: the character `…` and three full stops. A quotation with
/// no ellipsis is one segment, which is what makes every caller here able to ignore the
/// distinction.
fn segments(quotation: &str) -> Vec<String> {
    quotation
        .replace("...", "\u{2026}")
        .split('\u{2026}')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(str::to_owned)
        .collect()
}

/// One block of a Markdown document, joined into one text.
///
/// A paragraph, a blockquote, a table row or a list item — whatever [`blocks`]'s rules make a
/// unit. It is the unit *quotations* are collected from, and it is also the unit a claim
/// belongs to: a sweep asking whether the prose beside a quotation says something about it
/// needs the prose as well as the span.
#[derive(Debug, Clone)]
pub struct Block {
    /// The 1-based line the block starts on.
    pub line: usize,
    /// Whether the block is a blockquote or ordinary prose.
    pub shape: Shape,
    /// Its lines, trimmed of their `>` markers and joined with one space.
    pub text: String,
}

/// Every block one Markdown document is made of, in the order it writes them.
///
/// The rules that keep each of them honest:
///
/// - **A fenced block is not prose.** These documents are full of shell invocations, Rust
///   snippets and sample output, every string literal in which would otherwise be read as a
///   claim about the standard. The Rust scanner skips a fenced example inside a doc comment for
///   exactly this reason, and one of this project's own ADRs quotes a deliberately wrong
///   sentence inside a fence in order to describe it.
/// - **A paragraph is joined before the marks are counted.** A quotation worth checking is
///   longer than the column these documents wrap at, so its opening `"` and its closing one are
///   on different lines, and a line-at-a-time reader sees two unmatched marks and reports
///   nothing. This is `spec-errata`'s lesson about doc comments, and Markdown wraps harder.
#[must_use]
pub fn blocks(text: &str) -> Vec<Block> {
    let mut found: Vec<Block> = Vec::new();
    let mut fenced = false;
    let mut block: Vec<String> = Vec::new();
    let mut shape = Shape::Quoted;
    let mut start = 0usize;

    let flush = |block: &mut Vec<String>, shape: Shape, start: usize, found: &mut Vec<Block>| {
        if block.is_empty() {
            return;
        }
        let joined = block.join(" ");
        block.clear();
        found.push(Block {
            line: start,
            shape,
            text: joined,
        });
    };

    for (index, line) in text.lines().enumerate() {
        let number = index.saturating_add(1);
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            flush(&mut block, shape, start, &mut found);
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let quoted = trimmed.starts_with('>');
        let body = trimmed.trim_start_matches('>').trim();
        // A blank line ends a paragraph, and a change of shape ends one too: a blockquote under
        // a paragraph is two quotations, and joining them would invent a span no file contains.
        // So does the opening of a table row or a list item, and that one is about the *report*
        // rather than about the span: `doc/history.md`'s table runs for hundreds of unbroken
        // lines, and a finding in it was answered with the line number of the header row.
        let ends = body.is_empty()
            || opens_an_item(body)
            || (quoted && shape == Shape::Quoted)
            || (!quoted && shape == Shape::Blockquote);
        if ends && !block.is_empty() {
            flush(&mut block, shape, start, &mut found);
        }
        if body.is_empty() {
            continue;
        }
        if block.is_empty() {
            start = number;
            shape = if quoted {
                Shape::Blockquote
            } else {
                Shape::Quoted
            };
        }
        block.push(body.to_owned());
    }
    flush(&mut block, shape, start, &mut found);
    found
}

/// Every quotation one Markdown document makes, in the order it makes them.
///
/// Three shapes — a blockquote, and the two pairs of marks [`quote::quoted_spans`] finds in a
/// paragraph. [`blocks`] is where the rules that decide what a paragraph *is* live; this is the
/// one step after them, and the reason the two are separate functions is that a sweep asking
/// what the prose around a quotation says needs the block and not only the span.
#[must_use]
pub fn quotations(text: &str) -> Vec<(usize, Shape, String)> {
    let mut found: Vec<(usize, Shape, String)> = Vec::new();
    for block in blocks(text) {
        match block.shape {
            Shape::Blockquote => found.push((block.line, block.shape, block.text)),
            // A paragraph carries whichever marks it carries, and the span itself says which
            // pair delimited it: the block only ever knew that it was not a blockquote.
            _ => {
                for (marked, span) in quoted_spans(&block.text) {
                    found.push((block.line, marked, span));
                }
            }
        }
    }
    found
}

/// Whether a line opens a table row or a list item rather than continuing the line above it.
///
/// A cell and a bullet wrap, so their *continuation* lines are ordinary prose and stay joined —
/// what this ends is the block, at the line that starts the next one. Two things follow: a
/// quotation is never joined across two bullets into a span no document contains, and a finding
/// carries the line of the row it is in rather than of the table it is in.
fn opens_an_item(body: &str) -> bool {
    body.starts_with('|')
        || body.starts_with("- ")
        || body.starts_with("* ")
        || body.split_once(". ").is_some_and(|(number, _)| {
            !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())
        })
}

/// The quoted spans of a piece of prose, in the order they appear.
///
/// [`quote::quoted_spans`], which is where the rule lives so that this population, the
/// ledger's notes and `spec-errata`'s doc comments share one answer to what a quotation is.
#[must_use]
pub fn quoted_spans(text: &str) -> Vec<(Shape, String)> {
    quote::quoted_spans(text)
        .into_iter()
        .map(|(mark, span)| (Shape::of(mark), span))
        .collect()
}

/// Every Markdown document under `directory` that this project wrote, in a stable order.
///
/// # Errors
///
/// [`Error::Unreadable`] where a directory cannot be walked. A sweep that skipped what it could
/// not open would report a clean tree for a tree it had not looked at.
pub fn documents(directory: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut found = Vec::new();
    walk(directory, &mut found)?;
    found.sort();
    Ok(found)
}

fn walk(directory: &Path, found: &mut Vec<PathBuf>) -> Result<(), Error> {
    let entries = std::fs::read_dir(directory).map_err(|source| Error::Unreadable {
        path: directory.to_owned(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Unreadable {
            path: directory.to_owned(),
            source,
        })?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            // A symbolic link to a directory is how a worktree gets at the checkouts that live
            // beside it, and following one would read a third-party tree twice.
            if NOT_READ.contains(&name.as_ref()) || path.is_symlink() {
                continue;
            }
            walk(&path, found)?;
        } else if path.extension().is_some_and(|extension| extension == "md")
            && name != NOT_A_DOCUMENT
        {
            found.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The conversion this project reads is fourteen files; these tests need one sentence.
    fn conversion(text: &str) -> Conversion {
        Conversion::of(format!("{BETWEEN_DOCUMENTS}{}", quote::normalise(text)))
    }

    #[test]
    fn a_verbatim_quotation_is_found() {
        let standard = conversion(
            "The pattern matrix maps pattern space to the default (initial) coordinate space \
             of the page.",
        );
        assert_eq!(
            standard.judge("maps pattern space to the default (initial) coordinate space"),
            Verdict::Verbatim
        );
    }

    /// The finding this sweep exists for, in the shape ADR 0249 found it in the ledger: a
    /// quotation that is the standard's sentence up to a point and then somebody's memory of it.
    #[test]
    fn a_misquotation_diverges_and_the_standards_own_words_are_printed() {
        let standard = conversion(
            "PDF processors may choose to ignore any flatness tolerance specified within a PDF \
             file.",
        );
        let Verdict::Diverges {
            matched,
            total,
            standard,
        } = standard.judge("PDF processors may choose to ignore this parameter entirely")
        else {
            panic!("a sentence that starts as the standard's and then is not");
        };
        assert_eq!((matched, total), (6, 9));
        assert!(
            standard.contains("any flatness tolerance"),
            "the correction has to be writable from the report: {standard}"
        );
    }

    /// Almost every quotation in these documents is of something else, and reporting them is
    /// what would make the sweep unreadable.
    #[test]
    fn a_quotation_of_something_that_is_not_the_standard_is_unrelated() {
        let standard = conversion("The pattern matrix maps pattern space to the default page.");
        assert_eq!(
            standard.judge(
                "noticeably fastest PDF viewer available, and clean enough to be \
                            taught from"
            ),
            Verdict::Unrelated
        );
    }

    /// Half the quotation, as well as five words. A long sentence whose opening five words are
    /// the standard's is a coincidence the standard's own repetitiveness produces.
    #[test]
    fn five_words_of_a_long_quotation_is_not_enough() {
        let standard = conversion("shall be the same as the value of the BaseFont entry.");
        assert_eq!(
            standard.judge(
                "shall be the same as everything this project has ever decided about fonts \
                 and their widths"
            ),
            Verdict::Unrelated
        );
    }

    /// The three foldings, each of which an earlier round paid for.
    #[test]
    fn a_broken_word_a_lowered_capital_and_a_typed_dash_are_all_noise() {
        let standard = conversion(
            "The implementation of such a schema driven page generation involves \
             considerable effort - see Table 87 -Additional entries.",
        );
        assert_eq!(
            standard.judge("[t]he implementation of such a schema driven page generation"),
            Verdict::Verbatim,
            "a bracketed lowered capital is exact quotation with the change marked"
        );
        assert_eq!(
            standard.judge("considerable effort \u{2014} see Table 87 \u{2014} Additional entries"),
            Verdict::Verbatim,
            "the conversion's dash and the one on the keyboard are the same dash"
        );
    }

    /// The standard sets a formula's variables in mathematical italic and a document types the
    /// letters on the keyboard; Table 191's inversion is the case that found this.
    #[test]
    fn a_formulas_variables_are_the_letters_they_style() {
        let standard = conversion(
            "colour values shall be transformed by the function \u{1d453} (\u{1d465}) = 1 \
             \u{2013} \u{1d465} for display.",
        );
        assert_eq!(
            standard.judge("colour values shall be transformed by the function f(x) = 1 - x"),
            Verdict::Verbatim
        );
    }

    #[test]
    fn an_elision_requires_its_segments_in_order() {
        let standard = conversion("aligning the darkest colour of the source with the display.");
        assert!(standard.holds("aligning the darkest \u{2026} with the display"));
        assert!(!standard.holds("with the display \u{2026} aligning the darkest"));
    }

    #[test]
    fn a_fenced_block_is_not_prose() {
        let read =
            quotations("before\n\n```sh\ncargo run -- \"a flag with four words\"\n```\n\nafter\n");
        assert!(read.is_empty(), "{read:?}");
    }

    #[test]
    fn a_quotation_wrapped_across_two_lines_is_one_span() {
        let read =
            quotations("the row says \"a sentence that is long\nenough to wrap\" and so on\n");
        assert_eq!(read.len(), 1);
        assert_eq!(
            read.first()
                .map(|(line, shape, text)| (*line, *shape, text.as_str())),
            Some((1, Shape::Quoted, "a sentence that is long enough to wrap"))
        );
    }

    #[test]
    fn a_blockquote_is_one_quotation_and_does_not_join_the_paragraph_above_it() {
        let read = quotations(
            "a paragraph with \"four words in it\"\n> the quoted sentence\n> continued\n",
        );
        assert_eq!(read.len(), 2);
        assert_eq!(
            read.get(1)
                .map(|(line, shape, text)| (*line, *shape, text.as_str())),
            Some((2, Shape::Blockquote, "the quoted sentence continued"))
        );
    }

    /// The population `doc/todo/48` carried as owed: a paragraph's single-quoted span is a
    /// quotation and reaches the same judge as every other.
    #[test]
    fn a_single_quoted_span_in_a_paragraph_is_a_quotation() {
        let read = quotations("the row says 'a sentence that is long enough' and so on\n");
        assert_eq!(
            read.first()
                .map(|(line, shape, text)| (*line, *shape, text.as_str())),
            Some((1, Shape::Apostrophed, "a sentence that is long enough"))
        );
    }
}
