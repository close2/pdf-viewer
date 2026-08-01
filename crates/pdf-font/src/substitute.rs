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
            family: family_of(&folded, document, descriptor, panose),
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

/// Chooses a family from the font name, then §9.8.3.2's classification, then the flags.
fn family_of(
    folded: &str,
    document: &Document,
    descriptor: Option<&Dictionary>,
    panose: Option<crate::panose::Panose>,
) -> Family {
    // The two symbolic standard-14 fonts are matched first: their names are unambiguous
    // and getting them wrong substitutes Latin letters for symbols, which is unreadable
    // rather than merely imperfect.
    if folded.contains("zapfdingbat") || folded.contains("dingbat") {
        return Family::ZapfDingbats;
    }
    if folded.contains("symbol") {
        return Family::Symbol;
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
    let families = PREFERENCES
        .iter()
        .find(|(family, _)| *family == request.family)
        .map(|(_, names)| *names)?;

    // Family is the outer loop: a Helvetica-metric face in the wrong style beats a
    // correctly-styled face with unrelated metrics, because the style is cosmetic and the
    // metrics move every glyph on the line.
    for family in families {
        let family = normalise(family);
        for suffix in suffixes(request.bold, request.italic) {
            for candidate in catalogue() {
                if candidate
                    .stem
                    .strip_prefix(&family)
                    .is_some_and(|rest| rest == *suffix)
                {
                    return read_cached(&candidate.path);
                }
            }
        }
    }
    None
}

/// Reads a font file, reusing the bytes if they have been read already.
fn read_cached(path: &Path) -> Option<Arc<[u8]>> {
    let cache = LOADED.get_or_init(|| RwLock::new(Vec::new()));

    if let Ok(loaded) = cache.read()
        && let Some((_, bytes)) = loaded.iter().find(|(cached, _)| cached == path)
    {
        return Some(Arc::clone(bytes));
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
    use super::strip_subset_prefix;

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
