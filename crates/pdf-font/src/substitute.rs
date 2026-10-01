//! Choosing a font to stand in for one the document did not embed.
//!
//! A PDF may name a font without carrying it. The specification's standard 14 fonts are
//! the common case — a reader is required to have them — but any font may be referenced
//! without a `/FontFile`, and a viewer that draws nothing for those is not a viewer.
//!
//! # What is derived from the document, and what from the machine
//!
//! These are kept strictly apart, because confusing them makes rendering depend on what
//! happens to be installed:
//!
//! - [`Request`] is derived from the document alone — the `/BaseFont` name and the
//!   `/FontDescriptor`. The same PDF produces the same request on every machine.
//! - [`find`] then resolves that request against this machine's fonts, which obviously
//!   cannot be machine-independent, and reports failure rather than inventing something.
//!
//! **Advances do not come from here when the document states them.** `/Widths` and `/W`
//! are honoured whatever substitute is chosen, so lines break and glyphs land where the
//! producer intended even when the shapes differ. This is the property that matters: a
//! substituted page with correct metrics is readable and correctly laid out, whereas one
//! with the substitute's own metrics drifts out of alignment with the document's own
//! positioning.
//!
//! # This module used to describe itself as the only machine-dependent code in the tree
//!
//! It no longer is, for the fourteen faces where it matters. [`crate::standard`] compiles
//! §9.6.2.2's fourteen font programs into the binary, and [`find`] consults them **first**
//! for a request whose `/BaseFont` names one of them — see [`Request::standard`] — so those
//! pages render identically on every machine. The machine's own fonts still serve every
//! other non-embedded font, where their broader coverage is worth more than reproducibility,
//! and the compiled-in set is the fallback there rather than the first choice.
//!
//! Metrics are the other half of the same problem: [`crate::standard_metrics`] carries the standard
//! 14's advances, so a page whose font states no `/Widths` is laid out by the document rather than
//! by the substitute. The two halves come from the same faces. ADRs 0007, 0133.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

use pdf_syntax::{Dictionary, Document};

/// The generic family a substitute has to belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Family {
    /// A serif face, such as Times.
    Serif,
    /// A sans-serif face, such as Helvetica.
    SansSerif,
    /// A fixed-pitch face, such as Courier.
    Monospace,
    /// The standard-14 `Symbol` font.
    Symbol,
    /// The standard-14 `ZapfDingbats` font.
    ZapfDingbats,
}

impl Family {
    /// Whether the family has its own character set rather than a Latin one.
    #[must_use]
    pub fn is_symbolic(self) -> bool {
        matches!(self, Self::Symbol | Self::ZapfDingbats)
    }
}

/// Which reader parses a substitute's bytes.
///
/// A compiled-in face may be either. The Liberation faces are `sfnt` containers; the Foxit ones
/// are **bare CFF programs**, whatever their `.pfb` extension says — `PDFium`'s files begin
/// `01 00 04 02`, which is a CFF header and not PostScript, and the extension is inherited from
/// whatever they were converted from. §9.6.2.1's NOTE 1 is why that costs nothing here: a CFF is
/// "an alternative, more compact but functionally equivalent representation of a Type 1 font
/// program", and [`crate::cff`] already reads one because §9.9's `/FontFile3` embeds them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Format {
    /// `TrueType` or `OpenType`, read through `skrifa`'s `FontRef`.
    Sfnt,
    /// A bare CFF program, read through `read-fonts`' CFF reader.
    BareCff,
}

/// What the document asked for, derived from the document alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    /// The generic family.
    pub family: Family,
    /// Whether a bold weight was asked for.
    pub bold: bool,
    /// Whether an italic or oblique face was asked for.
    pub italic: bool,
    /// Whether the `/BaseFont` names one of §9.6.2.2's fourteen, or a metric-compatible clone.
    ///
    /// **This is what decides whether the compiled-in face or the machine's is tried first**,
    /// and the reason is that the two cases are different questions. A document naming
    /// `/Helvetica` is asking for something the standard says a processor *has*, so answering
    /// it the same way on every machine is the whole point. A document naming `/Garamond`
    /// without embedding it is asking for something no processor is required to have, and the
    /// machine's catalogue — which may hold a face with a far wider character set — is the
    /// better first answer there, with the compiled-in face behind it so that a machine with
    /// no fonts at all still draws the text.
    pub standard: bool,
}

impl Request {
    /// Derives a request from a font dictionary and its descriptor, if it has one.
    ///
    /// The descriptor is optional because the standard 14 are allowed to omit it, which is
    /// exactly the case that needs substituting most often.
    #[must_use]
    pub fn derive(document: &Document, dict: &Dictionary, descriptor: Option<&Dictionary>) -> Self {
        let base = document
            .get_key(dict, "BaseFont")
            .as_name()
            .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
            .unwrap_or_default();
        let base = strip_subset_prefix(&base);

        // The name is the strongest signal and the only one the standard 14 reliably
        // carry, so it is consulted first; the descriptor fills in what it does not say.
        let folded: String = base
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .map(|c| c.to_ascii_lowercase())
            .collect();

        // §9.8.3.2's classification, where a CIDFont's descriptor states one. It is consulted
        // *after* the name and *before* the flags, and the order is the argument: the name is
        // what the producer called the font, PANOSE is what a classification says about it, and
        // `/Flags` is a bitfield producers set carelessly (see `family_of`).
        let panose = descriptor.and_then(|d| panose(document, d));

        // Bold and italic are [`Style::derive`]'s, so that the compiled-in face a machine with no
        // fonts falls back to is the same weight and slope as the machine face ranked by the
        // whole style (ADR 1441).
        let style = Style::derive(document, dict, descriptor);

        Self {
            family: family_of(&folded, document, dict, descriptor, panose),
            bold: style.is_bold(),
            italic: style.italic,
            standard: names_a_standard_font(&folded),
        }
    }
}

/// The style a substitute is chosen by: Table 120's weight, slope and width, derived from the
/// document alone as [`Request`] is.
///
/// ISO 32000-2 §9.8.1 is the clause that gives a descriptor this job:
///
/// > These font metrics provide information that enables a PDF processor to synthesise a
/// > substitute font or select a similar font when the font program is unavailable.
///
/// [`Request`] carries the generic family (Table 121's `FixedPitch` and `Serif`, after the name and
/// PANOSE) and a bold/italic pair, which is all §9.6.2.2's compiled-in faces can be chosen by. A
/// machine's catalogue offers more — a light, a semibold, a black, a narrow face — and this is
/// what [`find_styled`] and [`installed_covering_styled`] rank its faces against (ADR 1441).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Style {
    /// Table 120's `/FontWeight` scale: 400 normal, 700 bold, each hundred "at least as dark as
    /// its predecessor" — the scale an `OS/2` table's `usWeightClass` states a face's weight on.
    pub weight: u16,
    /// Whether the glyphs slope: `/ItalicAngle` other than zero, or Table 121's Italic flag.
    pub italic: bool,
    /// Table 120's nine `/FontStretch` names numbered 1 (`UltraCondensed`) to 9
    /// (`UltraExpanded`), 5 being `Normal` — the numbering an `OS/2` table's `usWidthClass` uses.
    pub width: u16,
}

impl Style {
    /// The normal width.
    pub const NORMAL_WIDTH: u16 = 5;

    /// The style a request's two booleans imply, for a caller that knows no more than them — an
    /// interface's label, or one of §9.6.2.2's fourteen.
    #[must_use]
    pub const fn of(request: Request) -> Self {
        Self {
            weight: if request.bold { 700 } else { 400 },
            italic: request.italic,
            width: Self::NORMAL_WIDTH,
        }
    }

    /// Derives the style from a font dictionary and its descriptor, if it has one.
    ///
    /// **The weight** is the first of these that speaks, and the order is a documented choice
    /// (ADR 1441):
    ///
    /// 1. A weight word in the `/BaseFont` name (`Light`, `Medium`, `Semibold`, `Bold`, `Black`,
    ///    …). Table 120 calls `/FontWeight` "[t]he weight (thickness) component of the fully-
    ///    qualified font name", so where both are stated they state one thing, and the name is
    ///    what [`Request::derive`] has always read first. One of §9.6.2.2's fourteen states its
    ///    weight by its name alone — `Helvetica` is regular because it is not `Helvetica-Bold` —
    ///    so for those the name is the whole answer.
    /// 2. `/FontWeight`.
    /// 3. §9.8.3.2's PANOSE classification, where it says bold or not.
    /// 4. `/StemV`, by [`weight_from_stem`] — which is **not** the standard's: Table 120 defines
    ///    the entry as "[t]he thickness measured horizontally, of the dominant vertical stems of
    ///    glyphs in the font" and says nothing of how a thickness becomes a weight.
    ///
    /// and Table 121's `ForceBold`, whose clause speaks of "bold glyphs", makes it at least bold.
    ///
    /// **The slope** is [`Request::derive`]'s: the name, `/ItalicAngle` other than zero, or the
    /// Italic flag. **The width** is `/FontStretch`, and then a width word in the name.
    #[must_use]
    pub fn derive(document: &Document, dict: &Dictionary, descriptor: Option<&Dictionary>) -> Self {
        let folded = folded_base_font(document, dict);
        let number = |key: &str| descriptor.and_then(|d| document.get_key(d, key).as_number());
        let panose = descriptor.and_then(|d| panose(document, d));

        let named = weight_named(&folded).or_else(|| names_a_standard_font(&folded).then_some(400));
        let mut weight = named
            .or_else(|| number("FontWeight").and_then(weight_stated))
            .or_else(|| {
                panose
                    .and_then(crate::panose::Panose::is_bold)
                    .map(|bold| if bold { 700 } else { 400 })
            })
            .or_else(|| number("StemV").and_then(weight_from_stem))
            .unwrap_or(400);
        if descriptor.is_some_and(|d| flag(document, d, Flags::FORCE_BOLD)) {
            weight = weight.max(700);
        }

        let italic = folded.contains("italic")
            || folded.contains("oblique")
            || number("ItalicAngle").is_some_and(|angle| angle != 0.0)
            || descriptor.is_some_and(|d| flag(document, d, Flags::ITALIC));

        let stretch = descriptor.and_then(|d| {
            document
                .get_key(d, "FontStretch")
                .as_name()
                .and_then(|name| width_stated(name.as_bytes()))
        });
        let width = stretch
            .or_else(|| width_named(&folded))
            .unwrap_or(Self::NORMAL_WIDTH);

        Self {
            weight,
            italic,
            width,
        }
    }

    /// Whether this weight is bold, which is what §9.6.2.2's compiled-in faces are chosen by.
    ///
    /// Six hundred, `Demi` — the line PANOSE draws, and the one an integer `/FontWeight` of
    /// Errata Collection 3's 1..=1000 is read against as well as the published nine hundreds.
    #[must_use]
    pub const fn is_bold(self) -> bool {
        self.weight >= 600
    }
}

/// A `/BaseFont` name without its subset tag, lowercased and without punctuation.
fn folded_base_font(document: &Document, dict: &Dictionary) -> String {
    let base = document
        .get_key(dict, "BaseFont")
        .as_name()
        .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
        .unwrap_or_default();
    strip_subset_prefix(&base)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// A weight word a folded name carries, on Table 120's scale.
///
/// Longest first, so that `extralight` is not read as `light` nor `semibold` as `bold`. The words
/// and their numbers are the names `OS/2`'s `usWeightClass` gives its nine classes, which is the
/// scale Table 120's hundreds share — a documented choice, since the standard names no words.
fn weight_named(folded: &str) -> Option<u16> {
    const WORDS: &[(&str, u16)] = &[
        ("extralight", 200),
        ("ultralight", 200),
        ("extrabold", 800),
        ("ultrabold", 800),
        ("semibold", 600),
        ("demibold", 600),
        ("hairline", 100),
        ("medium", 500),
        ("black", 900),
        ("heavy", 900),
        ("light", 300),
        ("thin", 100),
        ("demi", 600),
        ("bold", 700),
    ];
    WORDS
        .iter()
        .find(|(word, _)| folded.contains(word))
        .map(|(_, weight)| *weight)
}

/// Table 120's `/FontWeight`, as one of the nine classes faces are ranked against.
///
/// Errata Collection 3 makes the entry an integer from 1 to 1000; a value outside that, or not a
/// finite number, states nothing this can rank by. A value between the published hundreds is read
/// as the nearest of them, 100 to 900, because a face states its weight on those nine classes and
/// because a description that crosses to a broker (`crate::provider`) is then one of nine weights
/// rather than one of a thousand.
fn weight_stated(value: f64) -> Option<u16> {
    if !(1.0..=1000.0).contains(&value) {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "checked to lie in 1..=1000 on the line above, so the hundreds are 0..=10"
    )]
    let hundreds = (value / 100.0).round() as u16;
    Some(hundreds.clamp(1, 9).saturating_mul(100))
}

/// A weight from Table 120's `/StemV`, where nothing else states one — **a documented choice, not
/// the standard's** (ADR 1441).
///
/// The rule: a stem of 120 thousandths of an em or more is bold (700), and a stem above zero and
/// below it is normal (400). Zero is Table 120's "unknown stem thickness" and states nothing. The
/// line sits between the stems of the regular and the bold of each of §9.6.2.2's Latin families
/// as their own font metrics state them — the Helvetica and Times regulars near 85 and their
/// bolds near 140 — so that a descriptor carrying the stems of one of those designs is read as
/// the weight it names. It ranks nothing finer than bold or not, because a stem measures one
/// design's strokes and the weight classes between are a designer's names, not a thickness.
fn weight_from_stem(stem: f64) -> Option<u16> {
    /// The stem width, in thousandths of an em, from which a face is read as bold.
    const BOLD_STEM: f64 = 120.0;
    if !stem.is_finite() || stem <= 0.0 {
        return None;
    }
    Some(if stem >= BOLD_STEM { 700 } else { 400 })
}

/// Table 120's nine `/FontStretch` names, numbered as [`Style::width`] numbers them.
fn width_stated(name: &[u8]) -> Option<u16> {
    Some(match name {
        b"UltraCondensed" => 1,
        b"ExtraCondensed" => 2,
        b"Condensed" => 3,
        b"SemiCondensed" => 4,
        b"Normal" => 5,
        b"SemiExpanded" => 6,
        b"Expanded" => 7,
        b"ExtraExpanded" => 8,
        b"UltraExpanded" => 9,
        _ => return None,
    })
}

/// A width word a folded name carries, longest first; `narrow` is how a producer names a
/// condensed design (`ArialNarrow`), and the standard names no words.
fn width_named(folded: &str) -> Option<u16> {
    const WORDS: &[(&str, u16)] = &[
        ("ultracondensed", 1),
        ("extracondensed", 2),
        ("semicondensed", 4),
        ("ultraexpanded", 9),
        ("extraexpanded", 8),
        ("semiexpanded", 6),
        ("condensed", 3),
        ("narrow", 3),
        ("expanded", 7),
    ];
    WORDS
        .iter()
        .find(|(word, _)| folded.contains(word))
        .map(|(_, width)| *width)
}

/// Whether a folded `/BaseFont` names one of §9.6.2.2's fourteen.
///
/// > The PostScript language names of 14 Type 1 fonts, known as the standard 14 fonts, are as
/// > follows: Times-Roman, Helvetica, Courier, Symbol, Times-Bold, Helvetica-Bold, Courier-Bold,
/// > `ZapfDingbats`, Times-Italic, Helvetica-Oblique, Courier-Oblique, Times-BoldItalic,
/// > Helvetica-BoldOblique, CourierBoldOblique.
///
/// Matched on the family part only, because the weight and slope are already in [`Request`] and
/// because the clause's own list spells one of them without a hyphen (`CourierBoldOblique`) —
/// a name-by-name table would have to reproduce that, and a reader that only accepted the
/// fourteen exact strings would refuse `Courier-BoldOblique`, which is what producers write.
///
/// The three clones are here on the same argument [`crate::standard_metrics::StandardFont`]
/// already makes for the metrics: Arial was drawn metric-compatible with Helvetica, and a
/// document naming it without embedding it means Helvetica in practice.
fn names_a_standard_font(folded: &str) -> bool {
    const FAMILIES: &[&str] = &[
        "times",
        "timesnewroman",
        "helvetica",
        "arial",
        "courier",
        "couriernew",
        "symbol",
        "zapfdingbats",
        "dingbats",
    ];
    FAMILIES.iter().any(|family| {
        folded.strip_prefix(family).is_some_and(|rest| {
            rest.is_empty()
                || matches!(
                    rest,
                    "roman"
                        | "bold"
                        | "italic"
                        | "oblique"
                        | "bolditalic"
                        | "boldoblique"
                        | "psmt"
                        | "ps"
                        | "mt"
                        | "boldmt"
                        | "italicmt"
                        | "bolditalicmt"
                )
        })
    })
}

/// The `/Flags` bits this module reads, numbered as the specification numbers them.
struct Flags;

impl Flags {
    /// Bit 1: all glyphs have the same width.
    const FIXED_PITCH: u32 = 1 << 0;
    /// Bit 2: glyphs have serifs.
    const SERIF: u32 = 1 << 1;
    /// Bit 6: the font uses the Standard Latin character set, or a subset of it.
    const NONSYMBOLIC: u32 = 1 << 5;
    /// Bit 7: the font slopes to the right.
    const ITALIC: u32 = 1 << 6;
    /// Bit 19: bold glyphs are painted with extra weight.
    const FORCE_BOLD: u32 = 1 << 18;
}

/// [`crate::metrics::flag`], which is the one reader of `/Flags` this crate has.
fn flag(document: &Document, descriptor: &Dictionary, bit: u32) -> bool {
    crate::metrics::flag(document, descriptor, bit)
}

/// §9.8.3.3's `/FD`: per-glyph-class metric overrides, listed and not applied.
///
/// A `CIDFont` "may be made up of different classes of glyphs, each class requiring different sets
/// of the font-wide attributes that appear in font descriptors" — Latin glyphs and kanji, in the
/// clause's own example — and `/FD` maps a class name to a descriptor overriding the font-wide
/// one for that class alone.
///
/// # Why this returns names rather than metrics for a CID
///
/// The names are not free text: "[t]he names of the glyph classes depend on the character
/// collection, as identified by the Registry , Ordering , and Supplement entries in the
/// `CIDSystemInfo` dictionary", and Table 123 lists them per collection — `Proportional`, `Kanji`,
/// `HRoman` and the rest. Knowing which *CIDs* a class holds means having the character
/// collection itself, which is registered data published outside this standard. That is the same
/// boundary Table 116's predefined `CMap`s sit behind, and the same decision: vendoring it is a
/// licensing question, and guessing at it would assign a kanji's metrics to a Latin glyph.
///
/// So a caller gets the classes the file states and may use them to *build* a substitute, which
/// is what the clause says they are for — "[w]ith the information for these glyphs, a more
/// accurate substitution font can be created". This crate selects an installed face instead
/// (ADR 0007), so nothing here consumes them yet.
///
/// # The sentence that forbids what the clause recommends
///
/// §9.8.3.3 says such a descriptor "shall contain entries for metric information only" and shall
/// not include the three `/FontFile` entries "or any of the entries listed in" Table 120. Every
/// metric a font descriptor can state — the
/// ascent, the descent, the stem widths, the missing width — **is** in Table 120, so read
/// literally the two halves of that sentence cannot both be satisfied by a descriptor that
/// states anything at all. The corpus's one witness resolves it the only way a producer can:
/// `issue13147.pdf`'s `/FD << /Proportional … >>` holds `/Ascent`, `/Descent`, `/CapHeight`,
/// `/XHeight`, `/StemV`, `/StemH`, `/Flags`, `/FontBBox`, `/ItalicAngle` and `/FontName`, all of
/// them Table 120's. Nothing here enforces the restriction, and this comment is the record of
/// why: it is the standard disagreeing with itself, not a file being wrong.
#[must_use]
pub fn glyph_classes(document: &Document, descriptor: &Dictionary) -> Vec<(String, Dictionary)> {
    let classes = document.get_key(descriptor, "FD");
    let Some(classes) = classes.as_dict() else {
        return Vec::new();
    };
    classes
        .iter()
        .filter_map(|(name, value)| {
            Some((
                String::from_utf8_lossy(name.as_bytes()).into_owned(),
                document.resolve(value).as_dict()?.clone(),
            ))
        })
        .collect()
}

/// One glyph class's descriptor laid over the main one, which is what §9.8.3.3 asks for.
///
/// Table 122 on `/FD`: "Each value shall be a dictionary containing entries that shall override
/// the corresponding values in the main font descriptor dictionary for that class of glyphs",
/// and §9.8.3.3 again from the other side: "[t]he entry's value shall be a font descriptor whose
/// contents shall override the font-wide attributes for that class only."
///
/// So the result is the main descriptor with the class's entries written over it, and every
/// reader of a descriptor in this crate works on it unchanged.
///
/// # What the clause keeps out of it
///
/// §9.8.3.3 bounds what such a descriptor may hold: it "shall contain entries for metric
/// information only; it shall not include `FontFile` , `FontFile2` , `FontFile3` , or any of the
/// entries listed in" — and the 2020 printing names Table 120 here, which is every entry a
/// descriptor can state, so read literally the sentence forbids a descriptor from stating
/// anything. Errata Collection 3's Issue #5 repairs it, repointing the prohibition at Table 122's
/// additional font descriptor entries for `CIDFonts` and making Table 120 the set the keys are
/// drawn *from*; §9.8.3.3's ledger row carries the three amendments word for word.
///
/// Both printings forbid the three font-program streams by name, and the amended one forbids
/// Table 122's four, so seven keys are dropped rather than carried over. That is not a validator
/// enforcing a `shall` — a file writing one is read exactly as before — but a reader declining to
/// let a class's descriptor answer a question the clause says it is not there to answer.
/// `/FD` itself is among the seven, so nothing here recurses.
#[must_use]
pub fn overridden(document: &Document, main: &Dictionary, over: &Dictionary) -> Dictionary {
    /// The seven keys §9.8.3.3 says a class's descriptor shall not hold.
    const FORBIDDEN: &[&[u8]] = &[
        b"FontFile",
        b"FontFile2",
        b"FontFile3",
        b"Style",
        b"Lang",
        b"FD",
        b"CIDSet",
    ];
    let mut merged = main.clone();
    for (key, value) in over.iter() {
        if FORBIDDEN.contains(&key.as_bytes()) {
            continue;
        }
        merged.insert(key.clone(), document.resolve(value).clone());
    }
    merged
}

/// Table 122's `/Lang`, as far as it can decide a face.
///
/// ISO 32000-2 §9.8.3.1, Table 122, on the entry:
///
/// > A name specifying the language of the font, which may be used for encodings where the
/// > language is not implied by the encoding itself. The value shall be a Language-Tag as
/// > defined in BCP 47.
///
/// The second half of the first sentence is what makes this worth reading and also what bounds
/// it: a `CIDFont` whose `/CIDSystemInfo` names `Adobe-Japan1` has already said Japanese, and
/// `/Lang` adds nothing there. An `Identity` ordering says nothing about a script at all, and
/// that is the case the entry is for — see `substituted::script_sample`, which is where
/// this is consumed.
///
/// # Only the three the substitution table already distinguishes
///
/// A `Language-Tag` names any of BCP 47's languages, and this enum names three. That is not a
/// claim about BCP 47 but about what a face can be chosen by: the one thing this crate does with
/// a script is ask whether a face can draw a character of it, and
/// `substituted::script_sample`'s table has one character each for Japanese, Chinese and
/// Korean and nothing for any other script. A tag outside them is read, found to name no entry of
/// that table, and answered `None` — which leaves the font with the family match it had, exactly
/// as an unregistered character collection does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Language {
    /// BCP 47's `ja`.
    Japanese,
    /// BCP 47's `zh`, without regard to script or region.
    ///
    /// Simplified and traditional are one variant here for the same reason
    /// `substituted::script_sample` gives `Adobe-GB1` and `Adobe-CNS1` the same
    /// character: the two collections disagree about glyph *forms*, and a face that has neither
    /// form has neither.
    Chinese,
    /// BCP 47's `ko`.
    Korean,
}

impl Language {
    /// Reads a `Language-Tag`, or `None` where it names no language this can choose a face by.
    ///
    /// Two rules of ISO 32000-2 §14.9.2.2 decide the reading, and both are that clause's:
    ///
    /// > A language identifier shall either be the empty text string, to indicate that the
    /// > language is unknown, or a Language-Tag as defined in BCP 47.
    ///
    /// > Although language codes are commonly represented using lowercase letters and country
    /// > codes are commonly represented using uppercase letters, all language tags shall be
    /// > treated as case-insensitive.
    ///
    /// So an empty value is unknown rather than malformed, and `JA`, `ja` and `Ja-JP` are one
    /// tag. What is compared is the **primary language subtag**, which a tag states first and
    /// separates with a hyphen: everything from the first hyphen on is a script, a region or a
    /// variant, none of which changes which of the three scripts above is wanted. `/zh-TW` is
    /// the witness that the comparison has to stop there.
    ///
    /// The three subtags are matched in their two-letter forms alone, and that is a statement
    /// about this reader rather than about BCP 47: that document is not in `doc/md/`, so nothing
    /// here can check a claim about what it requires, and the four witnesses on this disk write
    /// `ja`, `zh-TW` and `EN`. A three-letter spelling of the same language is therefore answered
    /// exactly as a tag for a script this table has no character for — it names no entry, and the
    /// font keeps the family match it had.
    #[must_use]
    pub fn read(tag: &[u8]) -> Option<Self> {
        let primary = tag.split(|byte| *byte == b'-').next()?;
        match primary.to_ascii_lowercase().as_slice() {
            b"ja" => Some(Self::Japanese),
            b"zh" => Some(Self::Chinese),
            b"ko" => Some(Self::Korean),
            _ => None,
        }
    }
}

/// Table 122's `/Lang`, where the descriptor states one.
///
/// # Both types, because the standard states two
///
/// Table 122's own type column says **name**, and §14.9.2.2 lists "Font Descriptors for `CIDFonts`
/// [which] can have a Lang key" among the places where language identifiers are **text strings**.
/// The two clauses disagree about the object type and agree about the value, so both are read:
/// which of the two a producer wrote cannot change which language it named.
#[must_use]
pub fn language(document: &Document, descriptor: &Dictionary) -> Option<Language> {
    let value = document.get_key(descriptor, "Lang");
    let tag = value
        .as_name()
        .map(pdf_syntax::Name::as_bytes)
        .or_else(|| value.as_string())?;
    Language::read(tag)
}

/// Table 122's `/Style` `/Panose`, where the descriptor states one.
///
/// §9.8.3.2 makes the value a *string*, so a file writing a name or an array has not stated a
/// classification — and [`crate::panose::Panose::read`] then requires the twelve bytes the
/// clause states.
fn panose(document: &Document, descriptor: &Dictionary) -> Option<crate::panose::Panose> {
    let style = document.get_key(descriptor, "Style");
    let style = style.as_dict()?;
    let value = document.get_key(style, "Panose");
    crate::panose::Panose::read(value.as_string()?)
}

/// Whether the document states that this font's codes are Latin glyph names.
///
/// ISO 32000-2 §9.6.5.4 states two conditions disjunctively and gives them one effect:
///
/// > If the font has a named Encoding entry of either MacRomanEncoding or WinAnsiEncoding , or
/// > if the font descriptor's Nonsymbolic flag (see "Table 121 -Font flags") is set, the PDF
/// > processor shall create a table that maps from character codes to glyph names
///
/// The clause is written for TrueType, but the statement is about the *codes* rather than about
/// one font type: §9.6.5.2 says of a Type 1 program that "An Encoding entry in the PDF font
/// dictionary, if present, shall override a Type 1 font's mapping from character codes to
/// character names", and Table 121's own prose says the Nonsymbolic flag means "the font's
/// character set is the Standard Latin character set (or a subset of it) and that it uses the
/// standard names for those glyphs".
///
/// Neither symbolic standard-14 font has a glyph under a Latin name, so a font described this
/// way cannot be stood in for by one — whatever its `/BaseFont` is spelled. §9.8.2 is the clause
/// that permits the flag to decide a substitute at all: "This influences the font's default base
/// encoding and may affect a PDF processor's font substitution strategies."
///
/// The `/Encoding` half is read here rather than through the font module's own reader because
/// this is a question about what the document *said*, not about what the encoding resolves to:
/// a `/BaseEncoding` this crate does not implement still states that the codes are Latin.
fn states_latin_codes(
    document: &Document,
    dict: &Dictionary,
    descriptor: Option<&Dictionary>,
) -> bool {
    if descriptor.is_some_and(|d| flag(document, d, Flags::NONSYMBOLIC)) {
        return true;
    }
    // §9.6.5.4 names the *Encoding entry*; Table 112 makes `/BaseEncoding` the same statement
    // one level in, and §9.6.5.4's second bullet reads a dictionary's entry exactly that way.
    let encoding = document.get_key(dict, "Encoding");
    let named = encoding
        .as_name()
        .map(|value| value.as_bytes().to_vec())
        .or_else(|| {
            encoding
                .as_dict()
                .map(|d| document.get_key(d, "BaseEncoding"))
                .and_then(|value| value.as_name().map(|n| n.as_bytes().to_vec()))
        });
    matches!(
        named.as_deref(),
        Some(b"MacRomanEncoding" | b"WinAnsiEncoding")
    )
}

/// Chooses a family from the font name, then §9.8.3.2's classification, then the flags.
fn family_of(
    folded: &str,
    document: &Document,
    dict: &Dictionary,
    descriptor: Option<&Dictionary>,
    panose: Option<crate::panose::Panose>,
) -> Family {
    // The two symbolic standard-14 fonts are matched first: their names are unambiguous
    // and getting them wrong substitutes Latin letters for symbols, which is unreadable
    // rather than merely imperfect.
    //
    // **Unless the document has said the codes are Latin**, which it may do twice over and
    // which outranks a substring of a name: `SegoeUISymbol` is a sans-serif face whose name
    // ends in the word, and `issue8697.pdf` draws "What Operating Systems Do" in it under
    // `/Encoding /WinAnsiEncoding` with Table 121's Nonsymbolic flag set. See
    // [`states_latin_codes`] for the clauses.
    if !states_latin_codes(document, dict, descriptor) {
        if folded.contains("zapfdingbat") || folded.contains("dingbat") {
            return Family::ZapfDingbats;
        }
        if folded.contains("symbol") {
            return Family::Symbol;
        }
    }
    if folded.contains("courier") || folded.contains("mono") || folded.contains("consol") {
        return Family::Monospace;
    }
    if folded.contains("times")
        || folded.contains("georgia")
        || folded.contains("garamond")
        || folded.contains("palatino")
        || folded.contains("century")
        || folded.contains("cambria")
        || folded.contains("book")
        || folded.contains("serif") && !folded.contains("sansserif")
    {
        return Family::Serif;
    }
    if folded.contains("helvetica")
        || folded.contains("arial")
        || folded.contains("verdana")
        || folded.contains("tahoma")
        || folded.contains("calibri")
        || folded.contains("segoe")
    {
        return Family::SansSerif;
    }

    // The name said nothing recognisable. §9.8.3.2's PANOSE number is next, because it is a
    // *classification of the face* rather than a bit somebody set: a document that carries one
    // has said whether the glyphs have serifs and whether they are monospaced, on a scale
    // defined outside this standard and cited by it.
    if let Some(panose) = panose {
        // Serifs decide before proportion, and the ordering is a **documented choice** rather
        // than the clause's — §9.8.3.2 states no rule for choosing a substitute at all. Two
        // reasons, and the second is this crate's own architecture: a monospaced face standing
        // in for a serifed design changes the shape of every glyph, which is the more
        // conspicuous error; and the proportion matters least here, because advances come from
        // `/Widths` or `/W` whenever the document states them (see this module's comment). The
        // corpus's own case is the argument in miniature — `vertical.pdf` embeds a Japanese
        // Mincho classified as *both* Cove-serifed and monospaced, and its Latin glyphs are
        // serifed.
        if let Some(serif) = panose.is_serif() {
            return if serif {
                Family::Serif
            } else {
                Family::SansSerif
            };
        }
        if panose.is_monospaced() == Some(true) {
            return Family::Monospace;
        }
        // A `LatinSymbol` face is deliberately *not* `Family::Symbol`: that arm means the
        // standard-14 `Symbol` font, whose character set is a specific one. All PANOSE states
        // here is that the glyphs are not letters, which no installed Latin family draws either.
    }

    // Last, the descriptor's flags, which are a weaker signal than either — many producers
    // set them carelessly.
    match descriptor {
        Some(d) if flag(document, d, Flags::FIXED_PITCH) => Family::Monospace,
        Some(d) if flag(document, d, Flags::SERIF) => Family::Serif,
        // Sans-serif is the safer default: a serif face standing in for a sans one is more
        // conspicuous than the reverse at reading sizes.
        _ => Family::SansSerif,
    }
}

/// Removes the `ABCDEF+` prefix a subset font's name carries.
fn strip_subset_prefix(name: &str) -> &str {
    match name.split_once('+') {
        Some((prefix, rest))
            if prefix.len() == 6 && prefix.bytes().all(|b| b.is_ascii_uppercase()) =>
        {
            rest
        }
        _ => name,
    }
}

/// Substitute families in the order they should be tried, most metric-compatible first.
///
/// The leading entries of the Latin families are the URW and Liberation metric clones of
/// the standard 14: `NimbusSans` reproduces Helvetica's advances, `NimbusRoman` Times',
/// `NimbusMonoPS` Courier's, and the Liberation and Croscore families reproduce the
/// Arial, Times New Roman and Courier New advances that most documents actually mean.
/// Choosing one of those makes the metrics right even when the document states none.
///
/// The later entries are ordinary faces with no such guarantee. They are still worth
/// trying, because text in approximately the right shape is far more useful than a blank
/// page — but only after every metric-compatible option has been ruled out.
static PREFERENCES: &[(Family, &[&str])] = &[
    (
        Family::SansSerif,
        &[
            "NimbusSans",
            "LiberationSans",
            "Arimo",
            "Helvetica",
            "Arial",
            "DejaVuSans",
            "NotoSans",
            "FreeSans",
        ],
    ),
    (
        Family::Serif,
        &[
            "NimbusRoman",
            "LiberationSerif",
            "Tinos",
            "Times",
            "DejaVuSerif",
            "NotoSerif",
            "FreeSerif",
        ],
    ),
    (
        Family::Monospace,
        &[
            "NimbusMonoPS",
            "LiberationMono",
            "Cousine",
            "Courier",
            "DejaVuSansMono",
            "NotoSansMono",
            "FreeMono",
        ],
    ),
    (
        Family::Symbol,
        &["StandardSymbolsPS", "OpenSymbol", "Symbola", "DejaVuSans"],
    ),
    (
        Family::ZapfDingbats,
        &["D050000L", "Dingbats", "OpenSymbol", "Symbola"],
    ),
];

/// Whether what follows a family's name in a file name is a style and nothing else.
///
/// `NimbusSans` is a family and `NimbusSans-BoldItalic`, `NimbusSansNarrow-Regular` and
/// `NimbusSans` are three of its members; `NotoSansMono-Regular` is not a member of `NotoSans`,
/// because `mono` names another design rather than a weight, a slope or a width. A file whose
/// name carries nothing after the family is a member — how most families name their upright
/// regular. What a member *is* comes from its own tables ([`FaceStyle::read`]); the words decide
/// only which files are the family.
fn names_a_style(mut rest: &str) -> bool {
    /// The weight, slope and width words a file name spells a style in, longest first so that
    /// `semibold` is not read as `semi` followed by something that is not a word.
    const WORDS: &[&str] = &[
        "ultracondensed",
        "extracondensed",
        "semicondensed",
        "ultraexpanded",
        "extraexpanded",
        "semiexpanded",
        "extralight",
        "ultralight",
        "extrabold",
        "ultrabold",
        "condensed",
        "semibold",
        "demibold",
        "expanded",
        "hairline",
        "regular",
        "oblique",
        "italic",
        "medium",
        "narrow",
        "normal",
        "black",
        "heavy",
        "light",
        "roman",
        "bold",
        "book",
        "demi",
        "thin",
    ];
    while !rest.is_empty() {
        let Some(word) = WORDS.iter().find(|word| rest.starts_with(**word)) else {
            return false;
        };
        rest = &rest[word.len()..];
    }
    true
}

/// What a machine face states about its own style, read from its `OS/2` and `post` tables.
///
/// The face side of [`Style`]: `usWeightClass` is on Table 120's `/FontWeight` scale and
/// `usWidthClass` numbers Table 120's nine `/FontStretch` names in their order, so a request and a
/// face are compared on one scale without a conversion of this crate's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FaceStyle {
    weight: u16,
    italic: bool,
    width: u16,
    fixed_pitch: bool,
    /// The face's own serif classification — `OS/2`'s `sFamilyClass`, or its PANOSE numbers where
    /// the class says nothing — or `None` where it states neither.
    serif: Option<bool>,
}

impl FaceStyle {
    /// Reads a face's statement of its style, or `None` for bytes that are not an `sfnt`.
    ///
    /// A face without an `OS/2` table (an old Macintosh `TrueType`) is read through `skrifa`'s
    /// attributes, which fall back to the `head` table's style bits.
    fn read(bytes: &[u8]) -> Option<Self> {
        use read_fonts::TableProvider;
        let font = skrifa::FontRef::new(bytes).ok()?;
        let attributes = skrifa::MetadataProvider::attributes(&font);
        let fixed_pitch = font.post().is_ok_and(|post| post.is_fixed_pitch() != 0);
        let os2 = font.os2().ok();
        let serif = os2.as_ref().and_then(|os2| {
            let class = os2.s_family_class().to_be_bytes();
            match class[0] {
                // The IBM font classes `OS/2` borrows: 1 to 5 and 7 are serifed designs, 8 the
                // sans serifs.
                1..=5 | 7 => Some(true),
                8 => Some(false),
                _ => {
                    // §9.8.3.2's `/Panose` is these same twelve bytes — the class and the ten
                    // PANOSE digits — so the one reader of them answers for a face as well.
                    let mut numbers = [0_u8; 12];
                    numbers[..2].copy_from_slice(&class);
                    numbers[2..].copy_from_slice(os2.panose_10());
                    crate::panose::Panose::read(&numbers).and_then(crate::panose::Panose::is_serif)
                }
            }
        });
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "skrifa's weight is a usWeightClass, 1..=1000"
        )]
        let weight = os2.as_ref().map_or_else(
            || attributes.weight.value().clamp(1.0, 1000.0) as u16,
            read_fonts::tables::os2::Os2::us_weight_class,
        );
        let width = os2
            .as_ref()
            .map_or(Style::NORMAL_WIDTH, |os2| os2.us_width_class().clamp(1, 9));
        Some(Self {
            weight,
            italic: !matches!(attributes.style, skrifa::attribute::Style::Normal),
            width,
            fixed_pitch,
            serif,
        })
    }

    /// How far this face is from `style`, as a key that sorts the nearest first: the weight, then
    /// the slope, then the width (ADR 1441).
    ///
    /// **Weight before slope** because a family missing the bold italic answers a bold italic
    /// request better with its bold than with its italic — a line of text is read by its weight
    /// before its slope. **Slope before width** because the width is the one of the three a page
    /// partly repairs: a substitute is drawn to the advances the file states
    /// ([`crate::metrics::substitute_stretch`]), and a slant has no such repair. Between two faces
    /// equally far in weight, the lighter is taken for a weight of 500 or less and the heavier
    /// above it — a documented choice, the one CSS's font matching makes.
    fn distance(self, style: Style) -> Distance {
        let wrong_side = if style.weight <= 500 {
            self.weight > style.weight
        } else {
            self.weight < style.weight
        };
        (
            self.weight.abs_diff(style.weight),
            wrong_side,
            self.italic != style.italic,
            self.width.abs_diff(style.width),
        )
    }

    /// Whether this face is of another generic family than the request's: spaced when Table 121's
    /// `FixedPitch` was asked for or proportional when it was not, serifed or not against the Serif
    /// flag. The two symbolic families ask neither.
    fn other_family(self, family: Family) -> bool {
        match family {
            Family::Monospace => !self.fixed_pitch,
            Family::Serif => self.fixed_pitch || self.serif == Some(false),
            Family::SansSerif => self.fixed_pitch || self.serif == Some(true),
            Family::Symbol | Family::ZapfDingbats => false,
        }
    }
}

/// [`FaceStyle::distance`]'s key: the weight's distance and whether it lies on the side not
/// preferred, whether the slope differs, and the width's distance.
type Distance = (u16, bool, bool, u16);

/// The style each file states, read once per file and kept as the style alone.
///
/// Ranking a family's members reads every member, and holding their programs to answer one
/// question would keep faces no page draws; only the winner is read again through
/// [`read_cached`], where the pages that use it find it.
static STYLES: OnceLock<RwLock<Vec<StyleRead>>> = OnceLock::new();

/// One file's style, or `None` where it is not a face this crate reads.
type StyleRead = (PathBuf, Option<FaceStyle>);

/// The style a file states, read and remembered.
fn style_of(path: &Path) -> Option<FaceStyle> {
    let memo = STYLES.get_or_init(|| RwLock::new(Vec::new()));
    if let Ok(held) = memo.read()
        && let Some((_, style)) = held.iter().find(|(cached, _)| cached == path)
    {
        return *style;
    }
    if !machine_fonts() {
        return None;
    }
    let style = std::fs::read(path)
        .ok()
        .and_then(|bytes| FaceStyle::read(&bytes));
    if let Ok(mut held) = memo.write() {
        held.push((path.to_path_buf(), style));
    }
    style
}

/// A font file this machine offers, with its name already in the form matching needs.
///
/// The normalised stem is computed once here rather than at each comparison. Doing it
/// inside the matching loops instead meant one string allocation per catalogue entry per
/// family per suffix. A lookup that had to try several families cost 1.37 ms; hoisting the
/// allocation here brought that to 18 µs, and a first-choice match from 35 µs to under a
/// microsecond. That is time-to-first-page for any document with a font it did not embed,
/// which is why it is worth the extra field.
#[derive(Debug)]
struct Candidate {
    path: PathBuf,
    /// The file stem, lowercased with punctuation removed.
    stem: String,
}

/// Every font file this machine offers, discovered once and kept.
///
/// Discovery is deferred behind a `OnceLock` because it walks the filesystem, which is
/// exactly the kind of work `CLAUDE.md` forbids on the launch path. A document with no
/// missing fonts — the common case — never pays for it at all.
static CATALOGUE: OnceLock<Vec<Candidate>> = OnceLock::new();

/// A font file's bytes, kept so two fonts resolving to one file read it once.
type Loaded = (PathBuf, Arc<[u8]>);

/// Font programs already read.
static LOADED: OnceLock<RwLock<Vec<Loaded>>> = OnceLock::new();

/// Whether this process may read the machine's own font files.
///
/// True until [`no_machine_fonts`] says otherwise, which is what a *confined* process does
/// before it loses the filesystem.
static MACHINE_FONTS: AtomicBool = AtomicBool::new(true);

/// States that this process cannot reach the machine's fonts, and must not try.
///
/// # Why a switch rather than an error path
///
/// [`catalogue`] walks the font directories and [`read_cached`] reads a file out of them, and
/// both are written to shrug off an `Err` — a machine with no fonts installed is a supported
/// deployment, and [`find`] answers from [`crate::standard`] there. **A confined process does not
/// get an `Err`.** `pdf-sandbox`'s seccomp filter is an allow-list whose action is
/// `SECCOMP_RET_KILL_PROCESS`, so `openat` is not refused, it is fatal: `read_dir` on
/// `/usr/share/fonts` ends the worker with `SIGSYS` before any `else` branch runs. ADR 0870's
/// corpus walk found four such documents in its first sixty — each names a CJK or Arabic face
/// without embedding it, and each kills the generator outright, taking the whole generation with
/// it.
///
/// So the reachability of the filesystem has to be *stated*, by the one part of the program that
/// knows: the caller that is about to confine itself. It is the same shape, in the same place, as
/// `pdf_sandbox::set_isolation` — asked before the confinement because after it there is
/// nothing to ask.
///
/// # What it costs, which is a real cost and is written down
///
/// A confined worker then behaves exactly like a machine with no fonts installed: [`find`] never
/// fails, so the text is still drawn, from the compiled-in faces. For a Latin document that is
/// the same page; for a document naming an uninstalled CJK face it is a page whose glyphs are
/// missing, and `pdf_model::interpret` reports that by name (§9.10.2's coverage note) rather than
/// drawing it silently.
///
/// **The fidelity is given back by [`crate::provider`], and this switch is what turns that path
/// on.** A process that has stated this and has been armed by its host asks the *broker* — which
/// has the filesystem — for a face **by description**, and gets a font program back; the matcher
/// the broker runs is [`machine_face`], which is this module's own walk. What does not change is
/// anything about what this process may do: it still cannot name a path, and its system-call set
/// is the one it had. ADRs 0870 and 0880, `doc/todo/59`.
///
/// There is no way to undo this, deliberately: it is called beside a confinement that cannot be
/// undone either.
pub fn no_machine_fonts() {
    MACHINE_FONTS.store(false, Ordering::Relaxed);
}

/// Whether the machine's own font files may be read.
#[must_use]
pub fn machine_fonts() -> bool {
    MACHINE_FONTS.load(Ordering::Relaxed)
}

/// The directories fonts are conventionally installed in.
fn font_directories() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    // Unix-like systems, including the paths a user-local install uses.
    for path in [
        "/usr/share/fonts",
        "/usr/local/share/fonts",
        "/usr/share/X11/fonts",
        // macOS.
        "/System/Library/Fonts",
        "/Library/Fonts",
        // Windows, for when this is built there.
        "C:\\Windows\\Fonts",
    ] {
        dirs.push(PathBuf::from(path));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        dirs.push(home.join(".local/share/fonts"));
        dirs.push(home.join(".fonts"));
        dirs.push(home.join("Library/Fonts"));
    }
    dirs
}

/// Walks the font directories, collecting files a font reader can open.
fn catalogue() -> &'static [Candidate] {
    // Checked here as well as inside the initialiser, so that a process which states it late
    // still stops reading — and so that one which states it first never walks at all.
    if !machine_fonts() {
        return &[];
    }
    CATALOGUE.get_or_init(|| {
        /// Bounds the walk so a pathological directory tree cannot stall a page.
        const MAX_DEPTH: u32 = 8;
        /// Bounds the catalogue so a directory with a million files cannot exhaust memory.
        const MAX_FILES: usize = 8192;

        fn walk(dir: &Path, depth: u32, found: &mut Vec<Candidate>) {
            if depth == 0 || found.len() >= MAX_FILES {
                return;
            }
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                if found.len() >= MAX_FILES {
                    return;
                }
                let path = entry.path();
                match entry.file_type() {
                    // Symlinks are not followed: font directories contain plenty, and a
                    // cycle would otherwise be a hang rather than a missing glyph.
                    Ok(kind) if kind.is_dir() => walk(&path, depth.saturating_sub(1), found),
                    Ok(kind) if kind.is_file() => {
                        // Only containers a font reader understands. Bare Type1 (`.pfb`)
                        // is excluded because nothing here reads it yet.
                        let usable = path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                            matches!(
                                e.to_ascii_lowercase().as_str(),
                                "ttf" | "otf" | "ttc" | "otc"
                            )
                        });
                        if usable {
                            let stem = path
                                .file_stem()
                                .and_then(|stem| stem.to_str())
                                .map(normalise)
                                .unwrap_or_default();
                            found.push(Candidate { path, stem });
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut found = Vec::new();
        // Asked again inside the initialiser, and after the items so that the lint is happy: a
        // process that states it *between* the guard above and this closure must still not walk.
        if !machine_fonts() {
            return found;
        }
        for dir in font_directories() {
            walk(&dir, MAX_DEPTH, &mut found);
        }
        found.sort_by(|a, b| a.path.cmp(&b.path));
        found
    })
}

/// Lowercases a name and drops the punctuation font file names vary in.
fn normalise(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Finds a font program to stand in for the requested one, in the style its two booleans imply.
///
/// [`find_styled`] with [`Style::of`], for a caller that knows no more than the request — the
/// compiled-in faces are chosen by nothing finer.
#[must_use]
pub fn find(request: Request) -> (Arc<[u8]>, Format) {
    find_styled(request, Style::of(request))
}

/// Finds a font program to stand in for the requested one, the machine's faces ranked by `style`.
///
/// **This never fails** (ADR 0133): [`crate::standard`] has a
/// face for every [`Family`], so a machine with no fonts installed at all draws the text. What
/// the order decides is *which* answer comes first, and [`Request::standard`] is what decides
/// the order — the fourteen the standard says a processor has are answered from the binary, and
/// everything else is answered from the machine with the binary behind it.
#[must_use]
pub fn find_styled(request: Request, style: Style) -> (Arc<[u8]>, Format) {
    if request.standard {
        let (bytes, format) = crate::standard::face(request);
        return (Arc::from(bytes), format);
    }
    if let Some(bytes) = installed_styled(request, style) {
        return (bytes, Format::Sfnt);
    }
    let (bytes, format) = crate::standard::face(request);
    (Arc::from(bytes), format)
}

/// The best face this machine offers for a request, or `None` if it offers none.
///
/// [`installed_styled`] with [`Style::of`].
#[must_use]
pub fn installed(request: Request) -> Option<Arc<[u8]>> {
    installed_styled(request, Style::of(request))
}

/// The best face this machine offers for a request in a style, or `None` if it offers none.
///
/// Every candidate is an `sfnt` container: [`catalogue`] admits no other extension, because a
/// bare Type 1 program on disk carries no name this could match against without opening it.
///
/// **Public because a composite font needs exactly this and not [`find`].** §9.7.4.2 leaves a
/// substituted composite font reachable only through `/ToUnicode`, so its face has to answer *by
/// character* — which an `sfnt`'s `cmap` does and a name-keyed CFF cannot. Handing the compiled-in
/// Foxit faces to that path would refuse five corpus documents that a machine font draws.
#[must_use]
pub fn installed_styled(request: Request, style: Style) -> Option<Arc<[u8]>> {
    installed_accepted(request, style, |_| true)
}

/// The best face of the request's family that answers more of a document's codes than the one
/// in hand.
///
/// **Public because a *simple* substituted font needs exactly this**, and it is the mirror of
/// [`installed_covering`] one clause over: a composite font's substitute is judged by whether it
/// can draw a script (§9.10.2 gives it characters and nothing else), and a simple font's by
/// whether it answers the codes §9.6.5's encoding names. `accept` is handed each candidate's
/// bytes and answers whether its code table is a strict improvement; `pdf_font::substitute_face`
/// is where that comparison lives, because building the table is the caller's business.
///
/// The list is walked in its own order and stops at the first face that improves, so a Times
/// document with Cyrillic in its `/Differences` gets `LiberationSerif` — the second name on this
/// machine's `Serif` list, `NimbusRoman` having no Cyrillic — rather than whatever face on the
/// machine happens to have the widest `cmap`.
#[must_use]
pub fn installed_wider(
    request: Request,
    style: Style,
    accept: impl Fn(&Arc<[u8]>) -> bool,
) -> Option<Arc<[u8]>> {
    installed_accepted(request, style, accept)
}

/// The best face on this machine that matches the request's family *and* satisfies `accept`.
///
/// The preference list is walked in its own order and each match is offered to `accept`, so a
/// caller that needs more than a family match — [`installed_covering`] needs a repertoire — gets
/// the *next* face of the same family rather than nothing. That distinction is the difference
/// between a Cyrillic document drawn in a serif face and one drawn in whatever face on the
/// machine happens to have the widest `cmap`: this machine's preference list for `Serif` begins
/// with `NimbusRoman`, which has no Cyrillic, and continues with `LiberationSerif`, which has.
fn installed_accepted(
    request: Request,
    style: Style,
    accept: impl Fn(&Arc<[u8]>) -> bool,
) -> Option<Arc<[u8]>> {
    // A process that has stated it cannot read the machine's fonts has no catalogue to walk, and
    // [`crate::provider`] is the one way a face can still reach it: the broker walks the list
    // below in the broker's own process and hands one candidate over at a time, so `accept` —
    // which judges a face by its *contents* and cannot cross a wire — still sees the same
    // preference list in the same order. Nothing is armed unless a host armed it, so this is
    // `None` in every process that has not asked for the port.
    if !machine_fonts() {
        for skip in 0..MAX_OFFERS {
            let (bytes, _name) = crate::provider::offered(request, style, &[], skip)?;
            if accept(&bytes) {
                return Some(bytes);
            }
        }
        return None;
    }

    // Not this one is not the end of it: the same family's next name may still answer, which is
    // why the walk is a list and the accept is a filter over it.
    for path in preferred_paths(request, style) {
        if let Some(bytes) = read_cached(path)
            && accept(&bytes)
        {
            return Some(bytes);
        }
    }
    None
}

/// How many candidates a confined caller may ask its broker for before it gives up.
///
/// [`preferred_paths`] is [`PREFERENCES`]'s families and each family's members, so its length is
/// bounded by that table and the machine's catalogue and is well under this for every family
/// installed here. The constant is a bound on *round trips* rather than a policy about faces: a
/// broker that answered every `skip` would otherwise be able to keep a worker asking.
pub(crate) const MAX_OFFERS: u32 = 64;

/// Every face this machine offers for a request, in the order [`PREFERENCES`] puts the families
/// and `style` puts each family's members.
///
/// **Family is the outer loop**: a Helvetica-metric face in the wrong style beats a
/// correctly-styled face with unrelated metrics, because the metrics move every glyph on the line
/// and the style changes its shape. **Within a family the members are ranked by the style each
/// states of itself** ([`FaceStyle::distance`]): a request for 300 gets `DejaVuSans-ExtraLight`
/// before `DejaVuSans`, and one for a condensed width gets `NimbusSansNarrow` before `NimbusSans`
/// (ADR 1441). One entry per file name, because a second file with the same stem is the same face
/// under another directory.
///
/// Lazy by family, because ranking a family reads its members and a lookup that the first family
/// answers should read no other.
///
/// Public in this module rather than a loop inside one caller because two callers need the same
/// order and a third needs it **as paths**: [`machine_face`] is what a broker answers a confined
/// worker's description with, and a broker matching in some other order would be a second matcher
/// for the confined side to disagree with the unconfined one about.
fn preferred_paths(request: Request, style: Style) -> impl Iterator<Item = &'static Path> {
    let families = PREFERENCES
        .iter()
        .find(|(family, _)| *family == request.family)
        .map_or(&[][..], |(_, names)| *names);
    families
        .iter()
        .flat_map(move |family| family_members(family, style))
}

/// One family's members on this machine, nearest `style` first.
fn family_members(family: &str, style: Style) -> Vec<&'static Path> {
    let family = normalise(family);
    let mut stems: Vec<&str> = Vec::new();
    let mut members: Vec<(Distance, &'static Path)> = Vec::new();
    for candidate in catalogue() {
        let Some(rest) = candidate.stem.strip_prefix(&family) else {
            continue;
        };
        if !names_a_style(rest) || stems.contains(&candidate.stem.as_str()) {
            continue;
        }
        stems.push(&candidate.stem);
        if let Some(face) = style_of(&candidate.path) {
            members.push((face.distance(style), candidate.path.as_path()));
        }
    }
    // A stable sort over the catalogue's path order, so that two members equally near are taken in
    // one order on every run.
    members.sort_by_key(|(distance, _)| *distance);
    members.into_iter().map(|(_, path)| path).collect()
}

/// The face this machine offers for a description, as a path a broker can open.
///
/// **This is the resource port's matcher, and it is the same walk everything else here uses.**
/// `crate::provider::open_a_face` is a decode, this call, and a `File::open`; there is no second
/// implementation of "which face answers this description" for a confined worker to disagree with
/// an unconfined one about (`doc/todo/59`).
///
/// `skip` passes over that many of the answers, so a caller judging faces by their *contents* can
/// walk the list rather than being handed one candidate — which is what [`installed_wider`] needs,
/// because §9.6.5.4's code table is built from the face's own program and cannot cross a wire. It
/// is meaningful only where `wanted` is empty: a covering search is a search for *the* face that
/// draws a script best (§9.10.2 gives a composite font's substitute characters and nothing else),
/// and there is no second answer to it — a `skip` past zero with characters asked for is `None`.
#[must_use]
pub fn machine_face(request: Request, style: Style, wanted: &[char], skip: u32) -> Option<PathBuf> {
    if wanted.is_empty() {
        let index = usize::try_from(skip).ok()?;
        return preferred_paths(request, style)
            .nth(index)
            .map(Path::to_path_buf);
    }
    if skip != 0 {
        return None;
    }
    covering_path(request, style, wanted)
}

/// The best face this machine offers that can draw `wanted`, in the style its request's two
/// booleans imply — [`installed_covering_styled`] with [`Style::of`].
#[must_use]
pub fn installed_covering(request: Request, wanted: &[char]) -> Option<Arc<[u8]>> {
    installed_covering_styled(request, Style::of(request), wanted)
}

/// The best face this machine offers that can draw `wanted`, or `None` — the one covering search,
/// which a substituted composite font on the page and a word of this program's own interface both
/// ask (ADRs 0152, 1430, 1441).
///
/// # Why a composite font needs this and [`installed`] is not enough
///
/// [`installed`] ranks candidates by the *generic family* a descriptor implies — serif, sans
/// serif, monospace — which is the right question for a Latin face and cannot express "this
/// one has to be able to draw Chinese". A non-embedded `Adobe-GB1` font therefore resolved to
/// a Latin face with no glyph for any character §9.10.2 gave it, and the page came out blank:
/// `issue8372.pdf`, and seven more like it (ADR 0152).
///
/// So the family's preference list is tried first, in `style`'s order, and *kept only if it
/// covers*; the whole catalogue is searched otherwise. The answer is deterministic on one machine
/// and says nothing about which machine — which is inherent: §9.10.2 leaves the choice of
/// substitute open, and ADR 0133 is why only §9.6.2.2's fourteen are compiled in.
///
/// **Coverage means every character in `wanted`, not some.** A face with one of the
/// collection's characters and not the rest is worse than the family match, because it draws
/// part of a line and leaves the rest blank at a different metric.
///
/// # How the catalogue's qualifying faces are ranked
///
/// **Repertoire first, and then the style** — the weight, the slope, the generic family (Table
/// 121's `FixedPitch` and `Serif` against the face's `post` and `OS/2`) and the width, in that order.
/// The widest `cmap` is the proxy for the characters the sample does not name, and the sample is
/// one character: this machine's widest faces for 的 are `DroidSansFallbackFull`'s thousands, and
/// among the faces stating it is `NotoTraditionalNushu-Bold`, a Nüshu face with a handful of Han
/// characters, which a style-first ranking hands every bold Chinese font. So style never buys a
/// face with fewer characters; it decides among the faces of the widest repertoire, which is where
/// a family's weights sit — every weight and width of `NotoSansArabic` states the same 1 250
/// characters, and the style is then what picks `NotoSansArabic-Regular` from its thirty-six
/// (ADR 1441). An interface's label gets its regular weight the same way a page's composite font
/// gets its bold.
///
/// Costs one `cmap` lookup per candidate per character, over faces already read for the
/// catalogue, the first time a set of characters is asked for; after that the qualifying faces
/// are remembered and each style is a ranking over them.
#[must_use]
pub fn installed_covering_styled(
    request: Request,
    style: Style,
    wanted: &[char],
) -> Option<Arc<[u8]>> {
    if wanted.is_empty() {
        return installed_styled(request, style);
    }
    // The port, for a process that cannot read a font file: the description that crosses carries
    // the characters and the style, so the broker runs `covering_path` — this function's own
    // search — in the process that has the filesystem. The answer is checked here anyway, because
    // a broker is a peer and a face that does not cover is worse than the compiled-in one.
    if !machine_fonts() {
        let (bytes, _name) = crate::provider::offered(request, style, wanted, 0)?;
        return covers(&bytes, wanted).then_some(bytes);
    }
    covering_path(request, style, wanted).and_then(|path| read_cached(&path))
}

/// Whether a face found earlier answers every one of `wanted`: the test [`installed_covering`] keeps
/// a face by, for a caller holding faces it already has.
///
/// A caller that asked once and was given a face asks this before asking the machine again, so a
/// line of Japanese walks the catalogue for its first character and finds the rest in the face
/// that answered it (ADR 1406).
#[must_use]
pub fn face_covers(bytes: &Arc<[u8]>, wanted: &[char]) -> bool {
    covers(bytes, wanted)
}

/// Whether a face answers every one of the characters asked for.
fn covers(bytes: &Arc<[u8]>, wanted: &[char]) -> bool {
    let Ok(font) = skrifa::FontRef::new(bytes) else {
        return false;
    };
    let charmap = skrifa::MetadataProvider::charmap(&font);
    wanted.iter().all(|c| charmap.map(*c).is_some())
}

/// [`installed_covering_styled`]'s search, answering the path rather than the bytes.
///
/// Split out because [`machine_face`] answers a broker with a path to open, and a broker that
/// searched differently would be a second matcher.
fn covering_path(request: Request, style: Style, wanted: &[char]) -> Option<PathBuf> {
    for path in preferred_paths(request, style) {
        if let Some(bytes) = read_cached(path)
            && covers(&bytes, wanted)
        {
            return Some(path.to_path_buf());
        }
    }
    best_covering(&qualifying(wanted), request, style).map(Path::to_path_buf)
}

/// The catalogue face [`installed_covering_styled`] takes among those stating every character:
/// the widest repertoire, and among the widest the nearest style, then the request's generic
/// family. The first in the catalogue's path order wins a tie, so one machine answers alike on
/// every run.
fn best_covering(faces: &[Qualifying], request: Request, style: Style) -> Option<&Path> {
    faces
        .iter()
        .min_by_key(|(_, mappings, face)| {
            (
                std::cmp::Reverse(*mappings),
                face.distance(style),
                face.other_family(request.family),
            )
        })
        .map(|(path, _, _)| path.as_path())
}

/// One face that states every character of a set: its file, its `cmap`'s size, and its style.
type Qualifying = (PathBuf, usize, FaceStyle);

/// The catalogue's faces that state every one of `wanted`, remembered by the characters.
///
/// Memoised on the characters alone, because the search is the expensive part — it reads font
/// files until it has read them all, 215 ms the first time on this machine's 1 400 faces — and
/// what it finds does not depend on the style: a document with three Japanese fonts in three
/// weights walks the catalogue once and ranks its answer three times.
///
/// Read straight from the filesystem rather than through `read_cached`: the search touches most of
/// the catalogue, and caching every face it rejects would hold the machine's entire font collection
/// in memory to answer one question. Only the winner is read again through the cache, where the
/// pages that use it will find it.
fn qualifying(wanted: &[char]) -> Vec<Qualifying> {
    let key: Vec<char> = wanted.to_vec();
    let memo = COVERING.get_or_init(|| RwLock::new(Vec::new()));
    if let Ok(held) = memo.read()
        && let Some((_, found)) = held.iter().find(|(cached, _)| *cached == key)
    {
        return found.clone();
    }
    let found: Vec<Qualifying> = catalogue()
        .iter()
        .filter_map(|candidate| {
            let bytes: Arc<[u8]> = std::fs::read(&candidate.path).ok()?.into();
            if !covers(&bytes, wanted) {
                return None;
            }
            let font = skrifa::FontRef::new(&bytes).ok()?;
            let mappings = skrifa::MetadataProvider::charmap(&font).mappings().count();
            Some((candidate.path.clone(), mappings, FaceStyle::read(&bytes)?))
        })
        .collect();
    if let Ok(mut held) = memo.write() {
        held.push((key, found.clone()));
    }
    found
}

/// One remembered answer to [`qualifying`]'s catalogue search.
type Covering = (Vec<char>, Vec<Qualifying>);

/// Answers to [`qualifying`]'s catalogue search, by the characters asked for.
static COVERING: OnceLock<RwLock<Vec<Covering>>> = OnceLock::new();

/// Reads a font file, reusing the bytes if they have been read already.
fn read_cached(path: &Path) -> Option<Arc<[u8]>> {
    let cache = LOADED.get_or_init(|| RwLock::new(Vec::new()));

    // A path already read is answered from memory whatever the switch says; a path that is not
    // is a file, and [`no_machine_fonts`] is the statement that this process may not open one.

    if let Ok(loaded) = cache.read()
        && let Some((_, bytes)) = loaded.iter().find(|(cached, _)| cached == path)
    {
        return Some(Arc::clone(bytes));
    }

    if !machine_fonts() {
        return None;
    }
    let bytes: Arc<[u8]> = std::fs::read(path).ok()?.into();
    if let Ok(mut loaded) = cache.write() {
        loaded.push((path.to_path_buf(), Arc::clone(&bytes)));
    }
    Some(bytes)
}

/// ISO 32000-2 §9.9.2's subset tag, which is a rule about six letters and a plus sign.
#[cfg(test)]
mod tests {
    use pdf_syntax::{Dictionary, Document, Name, Object};

    use std::path::PathBuf;

    use super::{
        FaceStyle, Family, Language, Request, Style, best_covering, catalogue, language,
        names_a_style, strip_subset_prefix, weight_from_stem,
    };

    /// A font dictionary carrying the entries a case needs and nothing else.
    fn font(entries: &[(&str, &str)]) -> Dictionary {
        let mut dict = Dictionary::new();
        for (key, value) in entries {
            dict.insert(
                Name::new(key.as_bytes().to_vec()),
                Object::Name(Name::new(value.as_bytes().to_vec())),
            );
        }
        dict
    }

    /// A descriptor whose `/Flags` are the integer given.
    fn descriptor(flags: i64) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert(Name::new(b"Flags".to_vec()), Object::Integer(flags));
        dict
    }

    /// ISO 32000-2 §9.6.5.4:
    ///
    /// > If the font has a named Encoding entry of either MacRomanEncoding or WinAnsiEncoding ,
    /// > or if the font descriptor's Nonsymbolic flag (see "Table 121 -Font flags") is set, the
    /// > PDF processor shall create a table that maps from character codes to glyph names
    ///
    /// So a name that merely *contains* "symbol" cannot select the standard-14 `Symbol`, whose
    /// glyphs carry no Latin name: `issue8697.pdf` draws "What Operating Systems Do" in
    /// `/SegoeUISymbol` and states both of the clause's two conditions. ADR 0158.
    #[test]
    fn a_document_that_states_latin_codes_is_not_given_a_symbolic_substitute() {
        let document = Document::empty();
        let nonsymbolic = descriptor(32);

        let segoe = font(&[
            ("BaseFont", "SegoeUISymbol"),
            ("Encoding", "WinAnsiEncoding"),
        ]);
        let request = Request::derive(&document, &segoe, Some(&nonsymbolic));
        assert_eq!(request.family, Family::SansSerif);

        // Either condition alone is enough — the clause states them disjunctively.
        let flag_only = font(&[("BaseFont", "SegoeUISymbol")]);
        assert_eq!(
            Request::derive(&document, &flag_only, Some(&nonsymbolic)).family,
            Family::SansSerif
        );
        let encoding_only = font(&[
            ("BaseFont", "SegoeUISymbol"),
            ("Encoding", "MacRomanEncoding"),
        ]);
        assert_eq!(
            Request::derive(&document, &encoding_only, None).family,
            Family::SansSerif
        );

        // And a document that states neither still gets the symbolic face, which is the case
        // the name check was written for.
        let plain = font(&[("BaseFont", "Symbol")]);
        assert_eq!(
            Request::derive(&document, &plain, None).family,
            Family::Symbol
        );
        let dingbats = font(&[("BaseFont", "ZapfDingbats")]);
        assert_eq!(
            Request::derive(&document, &dingbats, Some(&descriptor(4))).family,
            Family::ZapfDingbats
        );
    }

    /// ISO 32000-2 §14.9.2.2:
    ///
    /// > Although language codes are commonly represented using lowercase letters and country
    /// > codes are commonly represented using uppercase letters, all language tags shall be
    /// > treated as case-insensitive.
    ///
    /// The corpus's own witness is why the folding is asserted rather than assumed:
    /// `PDFJS-9279-reduced.pdf` states `/Lang /EN`, in capitals, where BCP 47's own spelling is
    /// `en`. And the tag that carries more than a primary subtag is
    /// `hayro-tests/pdfs/custom/pdftc_900k_0319_page_1.pdf`'s `/zh-TW`, which is the case that
    /// says the comparison has to stop at the first hyphen.
    #[test]
    fn a_language_tag_is_read_case_insensitively_and_only_as_far_as_its_primary_subtag() {
        assert_eq!(Language::read(b"ja"), Some(Language::Japanese));
        assert_eq!(Language::read(b"JA"), Some(Language::Japanese));
        assert_eq!(Language::read(b"Ja-JP"), Some(Language::Japanese));
        assert_eq!(Language::read(b"zh-TW"), Some(Language::Chinese));
        assert_eq!(Language::read(b"ZH-Hans-CN"), Some(Language::Chinese));
        assert_eq!(Language::read(b"ko-KR"), Some(Language::Korean));

        // "A language identifier shall either be the empty text string, to indicate that the
        // language is unknown, or a Language-Tag as defined in BCP 47" — so an empty value is
        // the file saying it does not know, and it names no script to choose a face by.
        assert_eq!(Language::read(b""), None);
        // A tag this table has no character for, which `PDFJS-9279-reduced.pdf` is: the font
        // keeps the family match it had. And a tag that merely *begins* with one of the three
        // is a different language — `jam` is Jamaican Creole, not Japanese.
        assert_eq!(Language::read(b"EN"), None);
        assert_eq!(Language::read(b"jam"), None);
        assert_eq!(Language::read(b"kok"), None);
    }

    /// Table 122 types `/Lang` as a name and §14.9.2.2 lists a `CIDFont`'s descriptor among the
    /// places a language identifier is a text string, so both are read. All four witnesses on
    /// this disk write a name; the string arm is the other clause's.
    #[test]
    fn a_lang_is_read_whether_the_producer_wrote_a_name_or_a_string() {
        let document = Document::empty();
        let mut named = Dictionary::new();
        named.insert(
            Name::new(b"Lang".to_vec()),
            Object::Name(Name::new(b"ja".to_vec())),
        );
        assert_eq!(language(&document, &named), Some(Language::Japanese));

        let mut stringly = Dictionary::new();
        stringly.insert(
            Name::new(b"Lang".to_vec()),
            Object::String(std::sync::Arc::from(b"ko-KR".as_slice())),
        );
        assert_eq!(language(&document, &stringly), Some(Language::Korean));

        // An entry of any other type states no tag, and an absent one states nothing at all —
        // Table 122 makes the entry optional and §9.8.3.1 says an absence "provides no
        // information as to the language".
        let mut wrong = Dictionary::new();
        wrong.insert(Name::new(b"Lang".to_vec()), Object::Integer(1));
        assert_eq!(language(&document, &wrong), None);
        assert_eq!(language(&document, &Dictionary::new()), None);
    }

    /// §9.9.2:
    ///
    /// > The tag shall consist of exactly six uppercase letters
    ///
    /// So the rule is not "split on the first plus": a face whose own name contains one, or
    /// a producer whose tag is the wrong length, states a name rather than a subset tag.
    #[test]
    fn a_subset_prefix_is_removed_only_when_it_is_one() {
        assert_eq!(strip_subset_prefix("ABCDEF+Times-Roman"), "Times-Roman");
        assert_eq!(strip_subset_prefix("Times-Roman"), "Times-Roman");
        assert_eq!(
            strip_subset_prefix("ABCDE+Times-Roman"),
            "ABCDE+Times-Roman"
        );
        assert_eq!(
            strip_subset_prefix("ABCDEFG+Times-Roman"),
            "ABCDEFG+Times-Roman"
        );
        assert_eq!(
            strip_subset_prefix("abcdef+Times-Roman"),
            "abcdef+Times-Roman"
        );
    }

    /// A simple font with no program, whose descriptor states `entries` and nothing that names a
    /// style: the `/BaseFont` carries no weight, slope or width word, so what is read is Table 120's.
    fn described(entries: &str) -> (Document, Dictionary) {
        let font = format!(
            "1 0 obj\n<< /Type /Font /Subtype /TrueType /BaseFont /ABCDEF+Corporate \
             /Encoding /WinAnsiEncoding /FirstChar 32 /LastChar 126 \
             /FontDescriptor << /Type /FontDescriptor /FontName /ABCDEF+Corporate \
             /FontBBox [0 -200 1000 800] /Ascent 800 /Descent -200 /CapHeight 700 {entries} >> \
             >>\nendobj\n"
        );
        crate::fixture::document_of(&[font.as_bytes()])
    }

    /// The face a fixture's font is drawn in, as the face states its own style.
    fn chosen(entries: &str) -> Option<FaceStyle> {
        let (document, dict) = described(entries);
        let font = crate::LoadedFont::load(&document, &dict, "F1").expect("a substitute loads");
        FaceStyle::read(font.substitute_program().expect("the font is substituted"))
    }

    /// Whether this machine offers a face of `family`'s preference list satisfying `test`, read
    /// off the catalogue and the faces' own tables rather than off the ranking under test (ADR
    /// 1154: the machine is asked, not the route).
    fn machine_offers(family: Family, test: impl Fn(FaceStyle) -> bool) -> bool {
        let Some((_, names)) = super::PREFERENCES.iter().find(|(f, _)| *f == family) else {
            return false;
        };
        catalogue().iter().any(|candidate| {
            names.iter().any(|name| {
                candidate
                    .stem
                    .strip_prefix(&super::normalise(name))
                    .is_some_and(names_a_style)
            }) && std::fs::read(&candidate.path)
                .ok()
                .and_then(|bytes| FaceStyle::read(&bytes))
                .is_some_and(&test)
        })
    }

    /// Table 120's `/StemV`, where no name word, `/FontWeight` or PANOSE states a weight, decides
    /// bold by ADR 1441's rule: a stem of 140 thousandths is Helvetica-Bold's, and the face chosen
    /// states a weight class of 600 or more.
    #[test]
    fn a_stem_as_thick_as_a_bold_ones_is_drawn_in_a_bold_face() {
        if !machine_offers(Family::SansSerif, |face| face.weight >= 600) {
            println!("skipped: this machine offers no bold face of the sans-serif preference list");
            return;
        }
        let face = chosen("/Flags 32 /ItalicAngle 0 /StemV 140").expect("an sfnt face");
        assert!(face.weight >= 600, "{face:?}");
        let face = chosen("/Flags 32 /ItalicAngle 0 /StemV 80").expect("an sfnt face");
        assert!(face.weight < 600, "{face:?}");
    }

    /// `/ItalicAngle` other than zero asks for a sloped face: Table 120, "[t]he value shall be
    /// negative for fonts that slope to the right, as almost all italic fonts do".
    #[test]
    fn an_italic_angle_is_drawn_in_a_sloped_face() {
        if !machine_offers(Family::SansSerif, |face| face.italic) {
            println!(
                "skipped: this machine offers no sloped face of the sans-serif preference list"
            );
            return;
        }
        let face = chosen("/Flags 32 /ItalicAngle -12 /StemV 80").expect("an sfnt face");
        assert!(face.italic, "{face:?}");
    }

    /// Table 121's `FixedPitch`, "[a]ll glyphs have the same width", asks for a face whose `post`
    /// table states `isFixedPitch`.
    #[test]
    fn a_fixed_pitch_flag_is_drawn_in_a_fixed_pitch_face() {
        if !machine_offers(Family::Monospace, |face| face.fixed_pitch) {
            println!("skipped: this machine offers no fixed-pitch face of the monospace list");
            return;
        }
        let face = chosen("/Flags 33 /ItalicAngle 0 /StemV 80").expect("an sfnt face");
        assert!(face.fixed_pitch, "{face:?}");
    }

    /// Table 121's Serif asks for a serifed face, where the name says nothing; the face chosen
    /// neither classes itself sans serif nor is fixed-pitch, which is the proportional serifed
    /// design the flag and a clear `FixedPitch` describe.
    #[test]
    fn a_serif_flag_is_drawn_in_a_serifed_face() {
        if !machine_offers(Family::Serif, |face| !face.fixed_pitch) {
            println!("skipped: this machine offers no face of the serif preference list");
            return;
        }
        let face = chosen("/Flags 34 /ItalicAngle 0 /StemV 80").expect("an sfnt face");
        assert!(!face.fixed_pitch, "{face:?}");
        assert_ne!(face.serif, Some(false), "{face:?}");
        // And the flag is what chose it: the same descriptor without the bit asks for a sans.
        let sans = chosen("/Flags 32 /ItalicAngle 0 /StemV 80").expect("an sfnt face");
        assert_ne!(sans.serif, Some(true), "{sans:?}");
    }

    /// Table 120's `/FontStretch` asks for a width: a `Condensed` font is drawn in the family's
    /// narrow member where the family has one.
    #[test]
    fn a_condensed_stretch_is_drawn_in_a_narrow_face() {
        if !machine_offers(Family::SansSerif, |face| face.width < Style::NORMAL_WIDTH) {
            println!("skipped: this machine offers no narrow face of the sans-serif list");
            return;
        }
        let face = chosen("/Flags 32 /ItalicAngle 0 /StemV 80 /FontStretch /Condensed")
            .expect("an sfnt face");
        assert!(face.width < Style::NORMAL_WIDTH, "{face:?}");
    }

    /// One of §9.6.2.2's fourteen states its weight by its name: `Helvetica` is the regular
    /// because it is not `Helvetica-Bold`, whatever stem its descriptor states.
    #[test]
    fn a_standard_name_states_its_own_weight() {
        let document = Document::empty();
        let mut descriptor = descriptor(32);
        descriptor.insert(Name::new(b"StemV".to_vec()), Object::Integer(140));
        let helvetica = font(&[("BaseFont", "Helvetica")]);
        assert_eq!(
            Style::derive(&document, &helvetica, Some(&descriptor)).weight,
            400
        );
        let bold = font(&[("BaseFont", "Helvetica-Bold")]);
        assert_eq!(
            Style::derive(&document, &bold, Some(&descriptor)).weight,
            700
        );
        let request = Request::derive(&document, &helvetica, Some(&descriptor));
        assert!(!request.bold && request.standard);
    }

    /// `/FontWeight` is read on its nine classes, and outranked by a weight word in the name it is
    /// "the weight (thickness) component of".
    #[test]
    fn a_font_weight_is_read_as_the_nearest_class() {
        let document = Document::empty();
        let with = |weight: i64| {
            let mut d = descriptor(32);
            d.insert(Name::new(b"FontWeight".to_vec()), Object::Integer(weight));
            d
        };
        let plain = font(&[("BaseFont", "Corporate")]);
        assert_eq!(
            Style::derive(&document, &plain, Some(&with(300))).weight,
            300
        );
        assert_eq!(
            Style::derive(&document, &plain, Some(&with(649))).weight,
            600
        );
        assert_eq!(Style::derive(&document, &plain, Some(&with(1))).weight, 100);
        assert_eq!(
            Style::derive(&document, &plain, Some(&with(1001))).weight,
            400
        );
        let named = font(&[("BaseFont", "Corporate-Black")]);
        assert_eq!(
            Style::derive(&document, &named, Some(&with(400))).weight,
            900
        );
    }

    /// ADR 1441's stem rule, and Table 120's zero, "an unknown stem thickness".
    #[test]
    fn a_stem_is_bold_from_one_hundred_and_twenty() {
        assert_eq!(weight_from_stem(0.0), None);
        assert_eq!(weight_from_stem(f64::NAN), None);
        assert_eq!(weight_from_stem(88.0), Some(400));
        assert_eq!(weight_from_stem(119.9), Some(400));
        assert_eq!(weight_from_stem(120.0), Some(700));
        assert_eq!(weight_from_stem(140.0), Some(700));
    }

    /// A family's members are the files whose names add only style words to it.
    #[test]
    fn a_family_member_adds_only_style_words() {
        for rest in [
            "",
            "regular",
            "bolditalic",
            "narrowboldoblique",
            "extracondensedlight",
        ] {
            assert!(names_a_style(rest), "{rest}");
        }
        for rest in ["mono", "arabicregular", "display", "boldx"] {
            assert!(!names_a_style(rest), "{rest}");
        }
    }

    /// The covering search never trades characters for style: a face with fewer characters in the
    /// right weight loses to a wider one in the wrong weight, and among faces equally wide the
    /// nearest style wins. The first is this machine's `NotoTraditionalNushu-Bold` against
    /// `DroidSansFallbackFull` for a bold Chinese font; the second, `NotoSansArabic`'s weights.
    #[test]
    fn repertoire_is_ranked_before_style() {
        let face = |weight: u16, italic: bool| FaceStyle {
            weight,
            italic,
            width: Style::NORMAL_WIDTH,
            fixed_pitch: false,
            serif: Some(false),
        };
        let request = Request {
            family: Family::SansSerif,
            bold: true,
            italic: false,
            standard: false,
        };
        let bold = Style::of(request);
        let faces = vec![
            (PathBuf::from("narrow-bold"), 300, face(700, false)),
            (PathBuf::from("wide-regular"), 30_000, face(400, false)),
        ];
        assert_eq!(
            best_covering(&faces, request, bold),
            Some(PathBuf::from("wide-regular").as_path())
        );
        let family = vec![
            (PathBuf::from("light"), 1250, face(300, false)),
            (PathBuf::from("regular"), 1250, face(400, false)),
            (PathBuf::from("bold-italic"), 1250, face(700, true)),
            (PathBuf::from("bold"), 1250, face(700, false)),
            (PathBuf::from("black"), 1250, face(900, false)),
        ];
        assert_eq!(
            best_covering(&family, request, bold),
            Some(PathBuf::from("bold").as_path())
        );
        let regular = Style::of(Request {
            bold: false,
            ..request
        });
        assert_eq!(
            best_covering(&family, request, regular),
            Some(PathBuf::from("regular").as_path())
        );
    }
}
