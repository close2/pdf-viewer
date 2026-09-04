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
//! Metrics were the other half of the same problem and were closed in the thirtieth session:
//! [`crate::standard_metrics`] carries the standard 14's advances, so a page whose font
//! states no `/Widths` is laid out by the document rather than by the substitute. The two
//! halves now come from the same faces. ADRs 0007, 0133.

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

        // Table 120's `/FontWeight` is an integer between 1 and 1000 inclusive since Errata
        // Collection 3 — Issue #474 widens the value set from the published nine hundreds,
        // which become a `should`, and Issue #152 makes the type integer. A threshold at 600
        // (Demi, the same line PANOSE draws) reads every conforming value under either
        // printing; `as_number` is wider than the amended type on purpose, a reader's
        // tolerance for a file writing `700.0`.
        let bold = folded.contains("bold")
            || folded.contains("black")
            || folded.contains("heavy")
            || descriptor.is_some_and(|d| {
                document
                    .get_key(d, "FontWeight")
                    .as_number()
                    .is_some_and(|weight| weight >= 600.0)
            })
            || panose
                .and_then(crate::panose::Panose::is_bold)
                .unwrap_or(false)
            || descriptor.is_some_and(|d| flag(document, d, Flags::FORCE_BOLD));

        let italic = folded.contains("italic")
            || folded.contains("oblique")
            || descriptor.is_some_and(|d| {
                document
                    .get_key(d, "ItalicAngle")
                    .as_number()
                    .is_some_and(|angle| angle != 0.0)
                    || flag(document, d, Flags::ITALIC)
            });

        Self {
            family: family_of(&folded, document, dict, descriptor, panose),
            bold,
            italic,
            standard: names_a_standard_font(&folded),
        }
    }
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

fn flag(document: &Document, descriptor: &Dictionary, bit: u32) -> bool {
    document
        .get_key(descriptor, "Flags")
        .as_integer()
        .and_then(|flags| u32::try_from(flags).ok())
        .is_some_and(|flags| flags & bit != 0)
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

/// A style suffix as font file names spell it, in the order they are tried.
///
/// A file whose name carries no suffix at all is treated as the regular face, which is how
/// most families name their upright weight.
fn suffixes(bold: bool, italic: bool) -> &'static [&'static str] {
    match (bold, italic) {
        (true, true) => &["bolditalic", "boldoblique", "bold", "italic", "regular", ""],
        (true, false) => &["bold", "regular", ""],
        (false, true) => &["italic", "oblique", "regular", ""],
        (false, false) => &["regular", "roman", "book", ""],
    }
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
/// `/usr/share/fonts` ends the worker with `SIGSYS` before any `else` branch runs. The
/// nine-hundred-and-fourteenth session's corpus walk found this on the first sixty documents it
/// read — four of them name a CJK or Arabic face without embedding it, and each killed the
/// generator outright, taking the whole generation with it.
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

/// Finds a font program to stand in for the requested one.
///
/// **This never fails**, since the hundred-and-forty-eighth session: [`crate::standard`] has a
/// face for every [`Family`], so a machine with no fonts installed at all draws the text. What
/// the order decides is *which* answer comes first, and [`Request::standard`] is what decides
/// the order — the fourteen the standard says a processor has are answered from the binary, and
/// everything else is answered from the machine with the binary behind it.
#[must_use]
pub fn find(request: Request) -> (Arc<[u8]>, Format) {
    if request.standard {
        let (bytes, format) = crate::standard::face(request);
        return (Arc::from(bytes), format);
    }
    if let Some(bytes) = installed(request) {
        return (bytes, Format::Sfnt);
    }
    let (bytes, format) = crate::standard::face(request);
    (Arc::from(bytes), format)
}

/// The best face this machine offers for a request, or `None` if it offers none.
///
/// Every candidate is an `sfnt` container: [`catalogue`] admits no other extension, because a
/// bare Type 1 program on disk carries no name this could match against without opening it.
///
/// **Public because a composite font needs exactly this and not [`find`].** §9.7.4.2 leaves a
/// substituted composite font reachable only through `/ToUnicode`, so its face has to answer *by
/// character* — which an `sfnt`'s `cmap` does and a name-keyed CFF cannot. Handing the compiled-in
/// Foxit faces to that path would refuse five corpus documents that a machine font draws.
#[must_use]
pub fn installed(request: Request) -> Option<Arc<[u8]>> {
    installed_accepted(request, |_| true)
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
pub fn installed_wider(request: Request, accept: impl Fn(&Arc<[u8]>) -> bool) -> Option<Arc<[u8]>> {
    installed_accepted(request, accept)
}

/// The best face on this machine that matches the request's family *and* satisfies `accept`.
///
/// The preference list is walked in its own order and each match is offered to `accept`, so a
/// caller that needs more than a family match — [`installed_covering`] needs a repertoire — gets
/// the *next* face of the same family rather than nothing. That distinction is the difference
/// between a Cyrillic document drawn in a serif face and one drawn in whatever face on the
/// machine happens to have the widest `cmap`: this machine's preference list for `Serif` begins
/// with `NimbusRoman`, which has no Cyrillic, and continues with `LiberationSerif`, which has.
fn installed_accepted(request: Request, accept: impl Fn(&Arc<[u8]>) -> bool) -> Option<Arc<[u8]>> {
    // A process that has stated it cannot read the machine's fonts has no catalogue to walk, and
    // [`crate::provider`] is the one way a face can still reach it: the broker walks the list
    // below in the broker's own process and hands one candidate over at a time, so `accept` —
    // which judges a face by its *contents* and cannot cross a wire — still sees the same
    // preference list in the same order. Nothing is armed unless a host armed it, so this is
    // `None` in every process that has not asked for the port.
    if !machine_fonts() {
        for skip in 0..MAX_OFFERS {
            let (bytes, _name) = crate::provider::offered(request, &[], skip)?;
            if accept(&bytes) {
                return Some(bytes);
            }
        }
        return None;
    }

    // Not this one is not the end of it: the same family's next name may still answer, which is
    // why the walk is a list and the accept is a filter over it.
    for path in preferred_paths(request) {
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
/// [`preferred_paths`] is [`PREFERENCES`]'s families crossed with [`suffixes`]'s endings, so its
/// length is bounded by those two tables and is well under this on every family. The constant is
/// a bound on *round trips* rather than a policy about faces: a broker that answered every `skip`
/// would otherwise be able to keep a worker asking.
pub(crate) const MAX_OFFERS: u32 = 64;

/// Every face this machine offers for a request, in the order [`PREFERENCES`] puts them.
///
/// **Family is the outer loop**: a Helvetica-metric face in the wrong style beats a
/// correctly-styled face with unrelated metrics, because the style is cosmetic and the metrics
/// move every glyph on the line. One entry per family and ending, because a second file with the
/// same stem is the same face under another directory.
///
/// Public in this module rather than a loop inside one caller because two callers need the same
/// order and a third needs it **as paths**: [`machine_face`] is what a broker answers a confined
/// worker's description with, and a broker matching in some other order would be a second matcher
/// for the confined side to disagree with the unconfined one about.
fn preferred_paths(request: Request) -> Vec<&'static Path> {
    let Some(families) = PREFERENCES
        .iter()
        .find(|(family, _)| *family == request.family)
        .map(|(_, names)| *names)
    else {
        return Vec::new();
    };

    let mut found: Vec<&'static Path> = Vec::new();
    for family in families {
        let family = normalise(family);
        for suffix in suffixes(request.bold, request.italic) {
            for candidate in catalogue() {
                if candidate
                    .stem
                    .strip_prefix(&family)
                    .is_some_and(|rest| rest == *suffix)
                {
                    found.push(candidate.path.as_path());
                    break;
                }
            }
        }
    }
    found
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
/// is meaningful only where `wanted` is empty: a covering search is a search for *the* widest face
/// that draws a script (§9.10.2 gives a composite font's substitute characters and nothing else),
/// and there is no second answer to it — a `skip` past zero with characters asked for is `None`.
#[must_use]
pub fn machine_face(request: Request, wanted: &[char], skip: u32) -> Option<PathBuf> {
    if wanted.is_empty() {
        let index = usize::try_from(skip).ok()?;
        return preferred_paths(request)
            .get(index)
            .map(|path| path.to_path_buf());
    }
    if skip != 0 {
        return None;
    }
    covering_path(request, wanted)
}

/// The best face this machine offers that can draw `wanted`, or `None`.
///
/// # Why a composite font needs this and [`installed`] is not enough
///
/// [`installed`] ranks candidates by the *generic family* a descriptor implies — serif, sans
/// serif, monospace — which is the right question for a Latin face and cannot express "this
/// one has to be able to draw Chinese". A non-embedded `Adobe-GB1` font therefore resolved to
/// a Latin face with no glyph for any character §9.10.2 gave it, and the page came out blank:
/// `issue8372.pdf`, and seven more like it (ADR 0152).
///
/// So the family's preference list is tried first and *kept only if it covers*, and the whole
/// catalogue is searched in path order otherwise. The order makes the answer deterministic on
/// one machine and says nothing about which machine — which is inherent: §9.10.2 leaves the
/// choice of substitute open, and ADR 0133 is why only §9.6.2.2's fourteen are compiled in.
///
/// **Coverage means every character in `wanted`, not some.** A face with one of the
/// collection's characters and not the rest is worse than the family match, because it draws
/// part of a line and leaves the rest blank at a different metric.
///
/// Costs one `cmap` lookup per candidate per character, over faces already read for the
/// catalogue; it runs only where a composite font is substituted *and* its collection is a
/// registered one, which is ten of the 974 corpus documents.
#[must_use]
pub fn installed_covering(request: Request, wanted: &[char]) -> Option<Arc<[u8]>> {
    if wanted.is_empty() {
        return installed(request);
    }
    // The port, for a process that cannot read a font file: the description that crosses carries
    // the characters, so the broker runs `covering_path` — this function's own search — in the
    // process that has the filesystem. The answer is checked here anyway, because a broker is a
    // peer and a face that does not cover is worse than the compiled-in one (see below).
    if !machine_fonts() {
        let (bytes, _name) = crate::provider::offered(request, wanted, 0)?;
        return covers(&bytes, wanted).then_some(bytes);
    }
    covering_path(request, wanted).and_then(|path| read_cached(&path))
}

/// Whether a face answers every one of the characters asked for.
fn covers(bytes: &Arc<[u8]>, wanted: &[char]) -> bool {
    let Ok(font) = skrifa::FontRef::new(bytes) else {
        return false;
    };
    let charmap = skrifa::MetadataProvider::charmap(&font);
    wanted.iter().all(|c| charmap.map(*c).is_some())
}

/// [`installed_covering`]'s search, answering the path rather than the bytes.
///
/// Split out because [`machine_face`] answers a broker with a path to open, and a broker that
/// searched differently would be a second matcher.
fn covering_path(request: Request, wanted: &[char]) -> Option<PathBuf> {
    let accept = |bytes: &Arc<[u8]>| covers(bytes, wanted);
    for path in preferred_paths(request) {
        if let Some(bytes) = read_cached(path)
            && accept(&bytes)
        {
            return Some(path.to_path_buf());
        }
    }

    // Memoised on the characters, because the search is the expensive part: it reads font
    // files until one covers them, and a document with three Japanese fonts would otherwise
    // walk the machine's catalogue three times. Measured: 215 ms the first time on this
    // machine's 1 400 faces, and nothing after it. Keyed by the characters alone, which is
    // exactly right for this half of the search: it walks the whole catalogue and asks nothing
    // about the family.
    let key: Vec<char> = wanted.to_vec();
    let memo = COVERING.get_or_init(|| RwLock::new(Vec::new()));
    if let Ok(held) = memo.read()
        && let Some((_, found)) = held.iter().find(|(cached, _)| *cached == key)
    {
        return found.clone();
    }

    // Read straight from the filesystem rather than through `read_cached`: a coverage search
    // touches most of the catalogue, and caching every face it rejects would hold the
    // machine's entire font collection in memory to answer one question. Only the winner is
    // read again through the cache, where the pages that use it will find it.
    let found = catalogue()
        .iter()
        .filter_map(|candidate| {
            let bytes: Arc<[u8]> = std::fs::read(&candidate.path).ok()?.into();
            if !accept(&bytes) {
                return None;
            }
            // **The widest repertoire among the faces that qualify**, which is a choice and
            // is here because the first qualifying face is a worse one. This machine's
            // catalogue offers `KanjiStrokeOrders.ttf` before `DroidSansFallback.ttf`, and
            // it is a teaching font: it has 的 and 中 and not the characters
            // `issue2128r.pdf` shows. Counting a `cmap`'s entries asks the question the
            // sample is a proxy for — how likely is this face to have the *rest* of the
            // document's characters — and it is computed only for faces that already
            // cover the sample.
            let font = skrifa::FontRef::new(&bytes).ok()?;
            let mappings = skrifa::MetadataProvider::charmap(&font).mappings().count();
            Some((mappings, &candidate.path))
        })
        .max_by_key(|(mappings, _)| *mappings)
        .map(|(_, path)| path.clone());
    if let Ok(mut held) = memo.write() {
        held.push((key, found.clone()));
    }
    found
}

/// One remembered answer to [`covering_path`]'s catalogue search.
type Covering = (Vec<char>, Option<PathBuf>);

/// Answers to [`installed_covering`]'s catalogue search, by the characters asked for.
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

    use super::{Family, Request, strip_subset_prefix};

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
}
