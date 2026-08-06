//! ISO 32000-2 §12.7.4.3's variable text: an appearance built from a value and a `/DA` string.
//!
//! Everything else this crate draws is a content stream the document wrote. This is the one
//! place a content stream is *written here*, because the clause says it has to be:
//!
//! > In such cases, the PDF document cannot provide a statically defined appearance stream for
//! > displaying the field. Instead, the PDF processor shall construct an appearance stream
//! > dynamically at rendering time.
//!
//! Four things in clause 12 need it and they differ only in where the text comes from: a text
//! field's `/V` (§12.7.5.3), a choice field's (§12.7.5.4), a button's `/MK` caption (Table 192)
//! and a free text annotation's `/Contents` (§12.5.6.6). [`Request`] is that difference and
//! nothing else.
//!
//! # What the clause states, and what it hands back
//!
//! Stated, and implemented here: the `/DA` string supplies "the field's text size and colour"
//! and "at a minimum … a `Tf` (text font) operator along with its two operands"; the font name
//! "shall match a resource name in the `Font` entry of the default resource dictionary"; `/Q`
//! chooses among left, centred and right; a size of zero means the font "shall be auto-sized";
//! at most one `Tm` may appear and a processor "shall replace the horizontal and vertical
//! translation components"; and the marks go inside a `/Tx BMC` … `EMC` pair.
//!
//! Handed back to the processor, in the clause's own words — "positioning values it determines
//! to be appropriate, based on the field value, the quadding (`Q`) attribute, and any layout
//! rules it employs", and for auto-sizing "an implementation dependent function". So **where a
//! baseline sits, how far apart two lines are, and what size auto-sizing picks are choices**,
//! not readings. Each is made once below, next to the reason, and each is a place where two
//! correct implementations may legitimately differ by a pixel or ten.
//!
//! # Where it is not implemented, it says so
//!
//! A `/DA` naming a font `/DR` does not define is refused by name, exactly as a content stream
//! naming a font its `/Resources` lacks already is — the clause makes the match a requirement
//! on the writer and states no recovery, and inventing a font from a resource *name* would put
//! glyphs on the page the document never described.

use pdf_syntax::{Dictionary, Document, Lexer, Object, Token};
use std::fmt::Write as _;

/// Table 228's `/Q`: "A code specifying the form of quadding (justification) that shall be
/// used in displaying the text."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Quadding {
    /// `0 Left-justified`, and Table 228's default.
    Left,
    /// `1 Centred`.
    Centred,
    /// `2 Right-justified`.
    Right,
}

impl Quadding {
    /// Reads `/Q` from the first dictionary that states it, defaulting to left.
    ///
    /// Table 228 marks `/Q` inheritable and Table 224 gives the interactive form dictionary a
    /// "document-wide default value for the Q attribute", so the caller passes the chain it
    /// wants consulted, nearest first.
    pub(crate) fn read(document: &Document, sources: &[&Dictionary]) -> Self {
        for source in sources {
            match document.get_key(source, "Q").as_integer() {
                Some(1) => return Self::Centred,
                Some(2) => return Self::Right,
                Some(0) => return Self::Left,
                // Table 228 states three codes. Anything else names no justification, so the
                // default stands rather than a fourth behaviour being invented.
                _ => {}
            }
        }
        Self::Left
    }
}

/// How the text fills the box: one line, several, or one character per comb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    /// Table 231's Multiline clear: "the field's text shall be restricted to a single line."
    SingleLine,
    /// Table 231 bit 13: "the field may contain multiple lines of text".
    Multiline,
    /// Table 231 bit 25: "the field shall be automatically divided into as many equally
    /// spaced positions, or combs, as the value of `MaxLen`, and the text is laid out into
    /// those combs." The count is Table 232's `/MaxLen`.
    Comb(u32),
}

/// Where the next character would go: the segment a host draws a text cursor along.
///
/// **A choice, and the standard states none.** ISO 32000-2 describes a text cursor nowhere —
/// §12.5.6.11's *caret annotation* is a different object entirely, a mark left in a document to
/// say that text was edited there — so how wide a cursor is, what colour it is and whether it
/// blinks are the host's. What this crate owes is the *place*, and the place is the position the
/// next glyph would be drawn at: the same x [`write_lines`] and [`comb`] position a line at, and
/// the same ascent and descent they measure a line's height by.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Caret {
    /// The end on the descent side of the baseline, `[x, y]`.
    pub from: [f32; 2],
    /// The end on the ascent side, which is the same point one line height up.
    pub to: [f32; 2],
}

/// One layout job: the text, the box, and the entries §12.7.4.3 reads.
pub(crate) struct Request<'a> {
    /// The value to show, already decoded from its §7.9.2.2 text string.
    pub text: &'a str,
    /// The region to lay the text out in, in the appearance's own coordinates.
    pub box_: [f32; 4],
    /// Table 228's `/DA`, the default appearance string.
    pub default_appearance: &'a [u8],
    /// Table 224's `/DR`, the resource dictionary the `/DA`'s font name is resolved in.
    pub resources: &'a Dictionary,
    /// Table 228's `/Q`.
    pub quadding: Quadding,
    /// How the text fills the box.
    pub shape: Shape,
    /// Where in [`Self::text`] a caret is wanted, as a byte offset — `None` for a layout that
    /// only draws.
    ///
    /// **Nothing in ISO 32000-2 asks for this**: the standard says where a *glyph* goes and says
    /// nothing whatever about a text cursor. What makes it belong here rather than in a caller is
    /// that the answer is the position the next glyph would be placed at, and that position is
    /// this module's arithmetic — a second opinion about it in a host would sit the caret beside
    /// the text instead of in it. See [`LaidOut::caret`].
    pub caret: Option<usize>,
}

/// What §12.7.4.3 asks for that this module cannot supply.
pub(crate) enum Owed {
    /// The `/DA` string carries no `Tf`, which the clause requires: "At a minimum, the string
    /// shall include a Tf (text font) operator along with its two operands, font and size."
    NoFont,
    /// The `/DA`'s font name matches no entry in `/DR`'s `/Font`, which the clause requires it
    /// to. The name is carried so the report can say which.
    FontNotInResources(String),
    /// The named font could not be loaded, or is one this crate cannot address by character.
    FontUnusable(String),
    /// `/DR` defines no font under the `/DA`'s name, **and** the one stood in for it cannot
    /// encode the value. Both halves, because either alone misnames what happened.
    InventedFontFellShort {
        /// The `/DA`'s font name, which `/DR` does not define.
        name: String,
        /// The characters the stand-in states no code for.
        characters: String,
    },
    /// The value contains characters the font states no code for, so they cannot be shown.
    CharactersNotInFont(String),
    /// The value is longer than [`MAX_CODES`] and the rest is not laid out.
    Truncated(usize),
    /// §12.7.5.4's list box: the clause states which items are selected and nothing about how
    /// a selection looks.
    ListBoxSelection,
    /// The `/DA`'s `Tm` scales or rotates, and the positions computed here do not account for
    /// it. The clause admits at most one `Tm` and has a processor "replace the horizontal and
    /// vertical translation components", which is what happens; the rest of the matrix is
    /// carried through and the layout is measured in unscaled text space.
    TransformedTextMatrix,
}

impl Owed {
    /// The report's detail, following the annotation's subtype name.
    pub(crate) fn detail(&self) -> String {
        match self {
            Self::NoFont => {
                "§12.7.4.3 requires a /DA holding a /Tf, and this one holds none".to_owned()
            }
            Self::FontNotInResources(name) => format!(
                "its /DA names the font /{name}, which the interactive form dictionary's /DR \
                 does not define"
            ),
            Self::FontUnusable(detail) => format!("its /DA's font is unusable: {detail}"),
            Self::InventedFontFellShort { name, characters } => format!(
                "its /DA names the font /{name}, which the interactive form dictionary's /DR \
                 does not define, and the standard font stood in for it states no code for \
                 {characters} — so the value is not drawn at all rather than in part"
            ),
            Self::CharactersNotInFont(characters) => {
                format!("its value contains {characters}, for which its /DA's font states no code")
            }
            Self::ListBoxSelection => "§12.7.5.4 states no appearance for a list box's \
                                       selected items"
                .to_owned(),
            Self::Truncated(limit) => {
                format!("its value is longer than the {limit} characters laid out here")
            }
            Self::TransformedTextMatrix => "its /DA sets a text matrix that scales or rotates, \
                                            and the text is positioned as if it did not"
                .to_owned(),
        }
    }
}

/// A laid-out appearance: the content stream, and anything the clause asked for and did not get.
pub(crate) struct LaidOut {
    /// The content stream, `/Tx BMC` to `EMC`.
    pub content: String,
    /// What could not be shown, if anything.
    pub owed: Option<Owed>,
    /// Whether the value needs more room than [`Request::box_`] gives it, on the axis
    /// §12.7.5.3's Table 231 bit 24 names.
    ///
    /// The clause names one axis per shape — "horizontally for single-line fields, vertically
    /// for multiple-line fields" — so this is one question with three answers rather than a
    /// bounding box: a line wider than a multiline field's box is not what the flag is about,
    /// because [`wrap`] has already decided where that line ends.
    ///
    /// **The stream is written either way.** The layout clips to the box and always has; what
    /// this adds is the ability to *ask*, which is what a program that fills a field needs and a
    /// program that only draws one does not. See [`crate::view::ViewState::set_field`].
    pub overflows: bool,
    /// Where a caret at [`Request::caret`] sits, in the appearance's own coordinates.
    ///
    /// `None` where none was asked for, and where the value could not be laid out at all.
    pub caret: Option<Caret>,
    /// A font dictionary this module invented, to be added to the appearance's `/Resources`
    /// under the name the `/DA` used.
    ///
    /// Present only where `/DR` defines no font under that name — see [`substituted_font`]. The
    /// stream this module writes says `/{name} {size} Tf`, so the resource has to exist by the
    /// time the interpreter runs it or the appearance would name nothing.
    pub font: Option<(pdf_syntax::Name, Dictionary)>,
}

/// Finds the `/DA`'s font in `/DR`, or stands one in; the flag says which happened.
///
/// A name `/DR` does not define is the document breaking §12.7.4.3's own `shall`, and
/// [`substituted_font`] is why that is answered with a stand-in and a report rather than with a
/// blank field.
fn resolve_font(
    document: &Document,
    resources: &Dictionary,
    name: &str,
) -> (Dictionary, Resolution) {
    let fonts = document.get_key(resources, "Font");
    let entry = fonts
        .as_dict()
        .and_then(|fonts| fonts.get(name))
        .map(|font| document.resolve(font));
    match entry.as_ref().and_then(|font| font.as_dict()) {
        Some(dict) => (dict.clone(), Resolution::Named),
        // A name that *conventionally* denotes one of §9.6.2.2's fourteen is not a stand-in
        // for reporting purposes: this binary carries that font program, so the value is drawn
        // in the face the name means. See [`STANDARD_ABBREVIATIONS`].
        // A name that is *itself* one of §9.6.2.2's fourteen is the same case a fortiori, and
        // §7.8.3's route into it is ADR 0183 — a `Tf` in a stream whose resources define nothing.
        // The two questions are one question and this is where they meet.
        None if pdf_font::standard::is_standard_name(name) => {
            (substituted_font(name), Resolution::Abbreviated)
        }
        None => match standard_abbreviation(name) {
            Some(standard) => (substituted_font(standard), Resolution::Abbreviated),
            None => (substituted_font(name), Resolution::StoodIn),
        },
    }
}

/// Where a `/DA`'s font came from, which decides two different things.
///
/// **Two questions, and they have different answers for the middle case**, which is why this is
/// three states rather than a flag:
///
/// - *Is anything owed?* Only [`Self::StoodIn`] owes a report. A font `/DR` defines is the
///   document's own, and one the name conventionally denotes is the one the name means.
/// - *May it fall short?* [`Self::Named`] may: a code the document's own font lacks is the
///   document's choice, reported and the rest drawn. The other two may **not**, and ADR 0112
///   is why — `freetext_no_appearance.pdf`'s value is a paragraph of Arabic under `/DA (/Helv 10
///   Tf)`, and a Latin face draws its spaces and full stops and nothing else, which is trap 1's
///   archetype and worse than the blank a refusal leaves. That the name *denotes* Helvetica does
///   not change whose inference it is: `/DR` defines nothing, so reading `/Helv` is this
///   program's reading and an inference may not fall short.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Resolution {
    /// Table 224's `/DR` defines a font under this name, as §12.7.4.3 requires.
    Named,
    /// `/DR` defines nothing, and the name is one of [`STANDARD_ABBREVIATIONS`].
    Abbreviated,
    /// `/DR` defines nothing and the name denotes nothing this program knows.
    StoodIn,
}

/// The fourteen `/DA` resource names that denote §9.6.2.2's fourteen font programs.
///
/// **A documented choice about a malformed file, not a reading of the clause.** §12.7.4.3 says
/// "[t]he specified font value shall match a resource name in the Font entry of the default
/// resource dictionary", and a file whose `/DA` names something `/DR` does not define has broken
/// that `shall`. The standard says nothing about what to do next, so something has to be chosen,
/// and what is chosen decides whether five corpus documents draw their free text in Helvetica or
/// in whatever sans-serif face the machine running the program happens to offer.
///
/// **The argument for the table is that it is a bijection with the clause's own list.** These are
/// not fourteen names picked out of the corpus: each is the four-letter abbreviation of exactly
/// one of the standard 14, and there is no fifteenth. That is what separates it from
/// [`substituted_font`]'s hint, where a resource name is arbitrary and passing `/F1` to a family
/// match costs nothing precisely because it means nothing.
///
/// **And what it buys is stated in the same terms ADR 0133 used**: the value is drawn from the
/// binary rather than from this machine, so those pages reproduce where no fonts are installed.
/// A page drawn from a face the document did not name and the machine happened to have is the
/// last machine-dependent thing left in a form's appearance.
///
/// The corpus's other two names — `/Rufscript` and `/F1` — are not on this list and still stand
/// in and report, which is the case the list is deliberately narrow enough to leave alone.
const STANDARD_ABBREVIATIONS: [(&str, &str); 14] = [
    ("Helv", "Helvetica"),
    ("HeBo", "Helvetica-Bold"),
    ("HeOb", "Helvetica-Oblique"),
    ("HeBO", "Helvetica-BoldOblique"),
    ("Cour", "Courier"),
    ("CoBo", "Courier-Bold"),
    ("CoOb", "Courier-Oblique"),
    ("CoBO", "Courier-BoldOblique"),
    ("TiRo", "Times-Roman"),
    ("TiBo", "Times-Bold"),
    ("TiIt", "Times-Italic"),
    ("TiBI", "Times-BoldItalic"),
    ("Symb", "Symbol"),
    ("ZaDb", "ZapfDingbats"),
];

/// The standard font a `/DA` resource name denotes, where it denotes one.
///
/// Case-sensitive, because a PDF name is a sequence of bytes and these fourteen are written one
/// way; a reader that folded them would start matching `/helv` and `/HELV`, which no producer
/// writes and which would widen a deliberately narrow table.
fn standard_abbreviation(name: &str) -> Option<&'static str> {
    STANDARD_ABBREVIATIONS
        .iter()
        .find(|(abbreviation, _)| *abbreviation == name)
        .map(|(_, standard)| *standard)
}

/// The font the `/DA` names, the value encoded through it, and the dictionary that produced both.
///
/// Split out of [`lay_out`] because it is three steps with one shape — resolve, load, encode —
/// and the third can send the first two round again.
///
/// # Errors
///
/// [`Owed::FontUnusable`] where the font will not load, or is composite: every route into this
/// crate's text drawing needs a code, and a composite font's codes cannot be produced from a
/// character without inverting a `CMap`'s codespace ranges (§9.7.6.2). Named rather than guessed
/// at.
fn set_in(
    document: &Document,
    request: &Request,
    font_name: &str,
) -> Result<(Dictionary, pdf_font::LoadedFont, Encoded, Resolution), Owed> {
    let (dict, resolution) = resolve_font(document, request.resources, font_name);
    let font = pdf_font::LoadedFont::load(document, &dict, font_name)
        .map_err(|error| Owed::FontUnusable(error.to_string()))?;
    if !font.addresses_characters() {
        return Err(Owed::FontUnusable(format!(
            "/{font_name} is a composite font, and this crate cannot yet address one by \
             character"
        )));
    }

    let runs = encode(&font, request.text, request.caret);
    // **A character the base encoding has no code for, given one.** §9.6.5.1 lets an encoding
    // dictionary name glyphs directly — "the value of the Differences entry [is] an array of
    // character codes and glyph names" — and a font *this module invented* is one whose encoding
    // it may state. `bug1865341.pdf` is the witness and its arithmetic is the argument: the value
    // is *Załącznik*, every Helvetica has an `aogonek`, and the one character that would not draw
    // was missing a **code** rather than a glyph — neither §9.6.5.2 encoding has an ogonek, while
    // `StandardEncoding` happens to include `lslash`, which is why `ł` was never on the list.
    //
    // Only for an invented font, never for the document's own: a `/DR` font's encoding is what
    // the document says its field is set in, and rewriting it would be answering a different
    // question from the one the file asked. And the codes go at the bottom of the range, which
    // both of §9.6.5.2's encodings leave unmapped.
    if resolution != Resolution::Named
        && !runs.missing.is_empty()
        && let Some((named, reloaded, again)) =
            named_glyphs_reach_more(document, &dict, font_name, request, &runs)
    {
        return Ok((named, reloaded, again, resolution));
    }
    Ok((dict, font, runs, resolution))
}

/// The same value re-encoded through an invented font whose `/Differences` names what it missed.
///
/// `None` unless the second attempt reaches strictly more characters than the first, which is
/// what keeps a face without the glyph — `freetext_no_appearance.pdf`'s Arabic, which no
/// Helvetica carries — refused exactly as it was rather than traded for a different refusal.
fn named_glyphs_reach_more(
    document: &Document,
    dict: &Dictionary,
    font_name: &str,
    request: &Request,
    runs: &Encoded,
) -> Option<(Dictionary, pdf_font::LoadedFont, Encoded)> {
    let named = with_differences(dict, &runs.missing)?;
    let reloaded = pdf_font::LoadedFont::load(document, &named, font_name).ok()?;
    let again = encode(&reloaded, request.text, request.caret);
    (again.missing.len() < runs.missing.len()).then_some((named, reloaded, again))
}

/// The lowest code a `/Differences` array may use here.
///
/// §9.6.5.2's two Latin encodings both begin at 32 — Annex D's tables state nothing below it —
/// so the codes under it are free for an invented encoding to name glyphs at. Zero is left out
/// because a glyph at code 0 is `.notdef` by every font format's convention.
const FIRST_DIFFERENCE_CODE: u8 = 1;

/// The same font dictionary with an `/Encoding` naming the glyphs for `missing`.
///
/// §9.6.5.1's `/Differences` is "an array of character codes and glyph names", and the names come
/// from the Adobe Glyph List through [`pdf_font::encoding::glyph_name`] — the same table
/// §9.10.2's third method reads in the other direction, so nothing is vendored for this.
///
/// `None` where there is no room (more distinct characters than the 31 free codes) or where the
/// AGL states no name for one of them, because a partial array would draw some of the value and
/// leave the rest silently absent — and `Owed::InventedFontFellShort` says more than that would.
fn with_differences(dict: &Dictionary, missing: &str) -> Option<Dictionary> {
    let mut differences = Vec::new();
    let mut code = FIRST_DIFFERENCE_CODE;
    for character in missing.chars() {
        let name = pdf_font::encoding::glyph_name(character)?;
        // One `code name` pair each rather than one run: §9.6.5.1 reads a number as the code the
        // names after it start at, so consecutive pairs are equivalent and this is the form that
        // stays right if a name is ever dropped from the middle.
        differences.push(Object::Integer(i64::from(code)));
        differences.push(Object::Name(pdf_syntax::Name::new(name.into_bytes())));
        code = code.checked_add(1).filter(|next| *next < 32)?;
    }
    let mut encoding = Dictionary::new();
    encoding.insert(
        pdf_syntax::Name::new(b"Type".to_vec()),
        Object::Name(pdf_syntax::Name::new(b"Encoding".to_vec())),
    );
    encoding.insert(
        pdf_syntax::Name::new(b"Differences".to_vec()),
        Object::Array(differences),
    );
    let mut out = dict.clone();
    out.insert(
        pdf_syntax::Name::new(b"Encoding".to_vec()),
        Object::Dictionary(encoding),
    );
    Some(out)
}

/// A font dictionary standing in for one Table 224's `/DR` does not define.
///
/// §12.7.4.3 requires the document to define it — "[t]he specified font value shall match a
/// resource name in the Font entry of the default resource dictionary" — and six corpus
/// documents do not, five of them naming `/Helv`. Refusing them drew **nothing**: a free text
/// annotation's text is the whole of its appearance, so `freetext_no_appearance.pdf` came out
/// as an empty page. That is ADR 0106's rule: an optional detail must not erase what the clause
/// requires, and here what the clause requires is the value on the page.
///
/// **The resource name is passed on as the base font name, and that is a hint rather than a
/// derivation.** A resource name is arbitrary — `/F1` as often as `/Helv` — so nothing here
/// claims it names a typeface. It is handed to `pdf_font`'s substitution because that is where
/// a name is *ranked* against the other evidence (ADR 0086) and where a name it does not
/// recognise costs nothing: `/F1` matches no family and falls through to the default, which is
/// the same answer as passing no name at all. What is never done is silence — the report says
/// which name `/DR` failed to define, so the page says the document is malformed while still
/// showing what the document says.
fn substituted_font(name: &str) -> Dictionary {
    let entry = |key: &str, value: &str| {
        (
            pdf_syntax::Name::new(key.as_bytes().to_vec()),
            Object::Name(pdf_syntax::Name::new(value.as_bytes().to_vec())),
        )
    };
    let mut dict = Dictionary::new();
    for (key, value) in [
        entry("Type", "Font"),
        entry("Subtype", "Type1"),
        entry("BaseFont", name),
    ] {
        dict.insert(key, value);
    }
    dict
}

/// The ratio of one line's height to the font size, when the `/DA` sets no leading.
///
/// **A choice**, and the clause is why one is needed at all: it hands line layout to "any
/// layout rules [the processor] employs". The value is not invented, though — §12.7.5.3's own
/// EXAMPLE lays out two lines of a multiline text field at `/Ti 12 Tf` with `0 -13 Td` between
/// them, so the standard's only worked example of this operation spaces lines at 13/12 of the
/// size. A `TL` in the `/DA` outranks it, because that is the document stating the same thing.
const LINE_HEIGHT: f32 = 13.0 / 12.0;

/// Where the baseline sits below the top of the em box, as a fraction of the font size.
///
/// **A choice.** Nothing in ISO 32000-2 says where in a field's box its text sits vertically —
/// the clause asks only for "positioning values [the processor] determines to be appropriate".
/// A font descriptor that states Table 122's `/Ascent` and `/Descent` is the document
/// answering the question and outranks this; a standard-14 font has no descriptor at all, and
/// this is what stands in. Splitting the em three-to-one puts the baseline where Latin text
/// normally sits, and being a constant it makes the layout independent of which fonts are
/// installed — which the substitute glyphs themselves are not, and layout should not be.
const DEFAULT_DESCENT: f32 = -0.25;

/// The matching ascent for [`DEFAULT_DESCENT`], so that the two span exactly one em.
const DEFAULT_ASCENT: f32 = 0.75;

/// How many characters of one value are laid out.
///
/// A bound rather than a rule, and reported when it is reached: §12.7.4.3 states no length and
/// a `/V` is a string a document controls, so an unbounded one is a content stream, a display
/// list and a glyph count all growing with it. Sixteen thousand characters is far past what
/// any field shows and far short of what a decompression bomb would ask for. The habit this
/// follows is that a silent cap is a defect, not safety.
const MAX_CODES: usize = 1 << 14;

/// The smallest size auto-sizing will choose.
///
/// A bound rather than a rule: below this the text is not legible anyway, and a box a few
/// hundredths of a point tall would otherwise drive the size — and the glyph count with it —
/// towards zero. Reaching it is not reported, because the text is still drawn: a value too
/// long for its box is the document's arrangement, not a gap in this crate.
const MIN_AUTO_SIZE: f32 = 1.0;

/// How many halvings the auto-size search makes.
///
/// Twenty steps take the interval from the box's height to under a millionth of a point, which
/// is far below any size difference a device can show. A fixed count rather than a tolerance
/// keeps the cost of a pathological box bounded, which principle 3 asks of every loop that a
/// document's own numbers drive.
const AUTO_SIZE_STEPS: u32 = 20;

/// Builds the appearance stream §12.7.4.3 describes.
///
/// # Errors
///
/// Returns what the clause asks for and the document does not supply — a `/DA` with no `Tf`, or
/// a font name `/DR` does not define. A shortfall that still leaves something to draw is
/// reported through [`LaidOut::owed`] instead, because drawing what is stated while naming what
/// is not says two true things rather than one.
pub(crate) fn lay_out(document: &Document, request: &Request) -> Result<LaidOut, Owed> {
    let appearance = DefaultAppearance::parse(request.default_appearance);
    let Some(font_name) = appearance.font.clone() else {
        return Err(Owed::NoFont);
    };

    let (dict, font, runs, resolution) = set_in(document, request, &font_name)?;

    let metrics = Metrics::read(document, &dict);
    if resolution != Resolution::Named && !runs.missing.is_empty() {
        // **A font this crate invented may not fall short.** `freetext_no_appearance.pdf` is
        // the reason the rule is asymmetric: its value is a paragraph of Arabic, and a Latin
        // stand-in draws its spaces and full stops and nothing else — a scatter of dots on an
        // otherwise empty page, which is trap 1's archetype and worse than the blank the
        // refusal leaves. Where the *document* names the font, a code it lacks is reported and
        // the rest is drawn, because there the shortfall is the document's own choice; here it
        // is ours, and the only honest thing an invention can do is decline.
        //
        // **What it declines with names both halves**, since the two-hundred-and-eighty-third
        // session. `FontNotInResources` alone said the document had not defined the name — true,
        // and by itself misleading twice over: since the two-hundred-and-fifty-eighth a `/Helv`
        // *is* drawn from the binary, so the undefined name is no longer what stops the value;
        // and `bug1865341.pdf`'s value is *Załącznik*, whose `ł` and `ą` are in Liberation Sans
        // and in neither §9.6.5.2 encoding a simple font may use. The reason is the **encoding**
        // rather than the face, and a report that does not say so sends the next session looking
        // in the wrong place. `doc/todo/22` holds what closing it would take.
        return Err(Owed::InventedFontFellShort {
            name: font_name,
            characters: runs.missing,
        });
    }
    let mut owed = if resolution == Resolution::StoodIn {
        // Named ahead of the two below: a value laid out in a font the document did not name
        // is a different statement from one whose length fell short, and it is the one that
        // explains the other when it follows from it.
        Some(Owed::FontNotInResources(font_name.clone()))
    } else if runs.truncated {
        Some(Owed::Truncated(MAX_CODES))
    } else {
        (!runs.missing.is_empty()).then_some(Owed::CharactersNotInFont(runs.missing))
    };
    let measure = Measure {
        font: &font,
        appearance: &appearance,
    };
    let box_ = request.box_;
    let (width, height) = ((box_[2] - box_[0]).max(0.0), (box_[3] - box_[1]).max(0.0));

    let size = match appearance.size {
        Some(size) if size > 0.0 => size,
        // Table 228's `/DA` "shall include a Tf … along with its two operands", and
        // §12.7.4.3: "A zero value for size means that the font shall be auto-sized".
        _ => auto_size(&measure, &runs.codes, request.shape, width, height),
    };

    let leading = appearance.leading.unwrap_or(size * LINE_HEIGHT);
    let lines = match request.shape {
        Shape::Multiline => wrap(&measure, &runs.codes, size, width),
        Shape::SingleLine | Shape::Comb(_) => std::iter::once(0..runs.codes.len()).collect(),
    };

    let mut stream = String::new();
    open_marked_content(&mut stream, &appearance, (&font_name, size), box_);

    let caret = if let Shape::Comb(count) = request.shape {
        comb(
            &mut stream,
            &measure,
            Written {
                codes: &runs.codes,
                caret: runs.caret,
                // The identity, because a comb writes one `Tm` per cell and that is the matrix it
                // writes: Table 231 bit 25 places each character in a position of its own, so a
                // `/DA`'s own `Tm` has nothing here to translate.
                matrix: IDENTITY,
            },
            Set {
                size,
                metrics: &metrics,
            },
            count,
            request,
        )
    } else {
        if appearance
            .matrix
            .is_some_and(|stated| stated[..4] != IDENTITY[..4])
            && owed.is_none()
        {
            owed = Some(Owed::TransformedTextMatrix);
        }
        write_lines(
            &mut stream,
            &lines,
            &measure,
            Set {
                size,
                metrics: &metrics,
            },
            leading,
            request,
            Written {
                codes: &runs.codes,
                caret: runs.caret,
                matrix: appearance.matrix.unwrap_or(IDENTITY),
            },
        )
    };

    stream.push_str("ET\nQ\nEMC\n");
    Ok(LaidOut {
        content: stream,
        owed,
        caret,
        overflows: overflows(
            &measure,
            &runs.codes,
            &lines,
            Set {
                size,
                metrics: &metrics,
            },
            leading,
            request,
        ),
        // The invented dictionary has to reach the appearance's `/Resources` under the name
        // the `/DA` used, whichever of the two ways this crate arrived at it — the stream says
        // `/{name} {size} Tf` either way, and a resource the interpreter cannot find is a
        // stream that names nothing.
        font: (resolution != Resolution::Named)
            .then(|| (pdf_syntax::Name::new(font_name.into_bytes()), dict)),
    })
}

/// Writes everything §12.7.4.3's EXAMPLE puts before the first positioning operator.
///
/// The clause's own example, element by element: `/Tx BMC`, `q`, "any required graphics state
/// changes, such as clipping", `BT`, the default appearance string, and the `Tf` — which is
/// written after the `/DA` rather than trusting it, because auto-sizing has to replace the zero
/// the clause puts there and a size the document did state is reproduced unchanged.
fn open_marked_content(
    stream: &mut String,
    appearance: &DefaultAppearance,
    font: (&str, f32),
    box_: [f32; 4],
) {
    let (width, height) = ((box_[2] - box_[0]).max(0.0), (box_[3] - box_[1]).max(0.0));
    stream.push_str("/Tx BMC\nq\n");
    let _ = writeln!(
        stream,
        "{} {} {} {} re W n",
        box_[0], box_[1], width, height
    );
    stream.push_str("BT\n");
    stream.push_str(&appearance.operators);
    let (name, size) = font;
    let _ = writeln!(stream, "/{name} {size} Tf");
}

/// The size a run is set at and the font's vertical metrics at one em.
#[derive(Clone, Copy)]
struct Set<'a> {
    size: f32,
    metrics: &'a Metrics,
}

/// The identity text matrix, which is what a `/DA` stating no `Tm` leaves.
const IDENTITY: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// Table 122's `/Ascent` and `/Descent`, in text-space units where one em is 1.0.
struct Metrics {
    ascent: f32,
    descent: f32,
}

impl Metrics {
    /// Reads the font descriptor's vertical metrics, or falls back to the documented split.
    ///
    /// Table 122: `/Ascent` is "the maximum height above the baseline reached by glyphs in
    /// this font" and `/Descent` "the maximum depth below the baseline", both in glyph space,
    /// whose unit is one thousandth of an em. Both are required of every font descriptor and
    /// a descriptor is required of every font but the standard 14 — which is exactly the case
    /// a `/DA` usually names, so the fallback is the common path rather than the odd one.
    ///
    /// A font whose stated ascent and descent do not straddle the baseline states nothing
    /// usable, and the fallback stands: an `/Ascent` of 0 would put every line's baseline on
    /// the top edge of its box.
    fn read(document: &Document, dict: &Dictionary) -> Self {
        let descriptor = document.get_key(dict, "FontDescriptor");
        let read = |key: &str| {
            descriptor
                .as_dict()
                .map(|descriptor| document.get_key(descriptor, key))
                .and_then(|value| value.as_number())
                .map(|value| narrow(value) / 1000.0)
                .filter(|value| value.is_finite())
        };
        match (read("Ascent"), read("Descent")) {
            (Some(ascent), Some(descent)) if ascent > 0.0 && descent < 0.0 => {
                Self { ascent, descent }
            }
            _ => Self {
                ascent: DEFAULT_ASCENT,
                descent: DEFAULT_DESCENT,
            },
        }
    }
}

/// The codes a value became, and the characters that produced none.
struct Encoded {
    codes: Vec<pdf_font::Code>,
    /// The distinct characters the font states no code for, ready for a report.
    missing: String,
    /// Whether [`MAX_CODES`] was reached and the rest of the value dropped.
    truncated: bool,
    /// Which code [`Request::caret`]'s byte offset falls before, where one was asked for.
    ///
    /// An index into [`Self::codes`] rather than into the value, because that is what the layout
    /// works in: a character the font states no code for produces none, and a caret cannot sit
    /// beside a glyph that is not drawn.
    caret: Option<usize>,
}

/// Turns a value into the font's own character codes, collecting what it cannot represent.
///
/// The caret's byte offset is turned into a code index here, in the one loop that knows how many
/// codes a character produced — which is none for a character the font cannot spell, and one for
/// each of §12.7.5.3's line breaks. An offset inside a character counts that character as still
/// to come, and one past the end is the end.
fn encode(font: &pdf_font::LoadedFont, text: &str, caret: Option<usize>) -> Encoded {
    let mut codes = Vec::with_capacity(text.len().min(MAX_CODES));
    let mut missing = String::new();
    let mut truncated = false;
    let mut caret_code = None;
    // A carriage return, a line feed and the pair are all one line break, and none of them is
    // a glyph. They travel through the layout as `BREAK` and `show` drops them. Read from the
    // value as it stands rather than from a normalised copy, so that the caret's offset means
    // the same thing here as it does to the host that sent it.
    let mut after_return = false;
    for (at, character) in text.char_indices() {
        if caret_code.is_none() && caret.is_some_and(|offset| offset <= at) {
            caret_code = Some(codes.len());
        }
        if codes.len() >= MAX_CODES {
            truncated = true;
            break;
        }
        let character = match character {
            '\n' if after_return => {
                after_return = false;
                continue;
            }
            '\r' => {
                after_return = true;
                '\n'
            }
            other => {
                after_return = false;
                other
            }
        };
        if character == '\n' {
            codes.push(BREAK);
            continue;
        }
        match font
            .code_for(character)
            .or_else(|| substitutable(character, font))
        {
            Some(code) => codes.push(code),
            None if missing.contains(character) => {}
            None => missing.push(character),
        }
    }
    Encoded {
        caret: caret.map(|_| caret_code.unwrap_or(codes.len())),
        codes,
        missing,
        truncated,
    }
}

/// A code for a character the font's encoding has no code for, where the standard names one.
///
/// One character so far, and the standard states the whole of the argument. Annex D's note 6 —
/// the notes under §9.6.5.2's Latin character set — says that the space U+0020 is *also* encoded
/// at 240 octal in `WinAnsiEncoding` and 312 octal in `MacRomanEncoding`, that Windows Code Page
/// 1252 associates that code with the non-breaking space U+00A0, and that a producer meaning the
/// second has to say so with a `/Differences` array naming `nonbreakingspace`. (Prose rather than
/// a blockquote: the sentence lives in an annex, and this tree's quotations are checked against
/// the numbered clause cited beside them.)
///
/// So `WinAnsiEncoding` gives U+00A0 no code of its own, and a field whose value holds one — as
/// `bug1871353.pdf`'s does, nine of them between two letters — has a character §12.7.4.3 must
/// write and the encoding cannot spell.
///
/// **A no-break space is a space, and drawing it as one loses nothing a display can show.** The
/// only difference between the two is where a line may break, and this module has already
/// decided that: [`wrap`] breaks on the value's own white space, and it sees the character
/// rather than the code. What the alternative costs is measured — the field is reported and
/// **nothing at all is drawn**, because a font this program inferred may not fall short.
///
/// Deliberately not a table of near-equivalents. The soft hyphen the same note names is *not*
/// here: it is a character a layout engine shows only where it breaks a line, and drawing it
/// always or never are both decisions about hyphenation rather than about encoding.
fn substitutable(character: char, font: &pdf_font::LoadedFont) -> Option<pdf_font::Code> {
    (character == '\u{a0}')
        .then(|| font.code_for(' '))
        .flatten()
}

/// The code standing for a line break, which is never shown.
///
/// A line feed's own code, which `show` drops and `wrap` treats as the end of a line. It is
/// never drawn, so nothing depends on whether the font gives code 10 a glyph.
const BREAK: pdf_font::Code = pdf_font::Code::single_byte(b'\n');

/// Measures a run of codes the way the interpreter will draw it.
///
/// §9.4.4's displacement, with the parameters the `/DA` set: `(w0 × size + Tc + Tw) × Th`,
/// with `Tw` on a single-byte code 32 only (§9.3.3). Measuring with a different formula from
/// the one that draws is how centred text comes out off-centre.
struct Measure<'a> {
    font: &'a pdf_font::LoadedFont,
    appearance: &'a DefaultAppearance,
}

impl Measure<'_> {
    /// The displacement a run of codes produces at a given size.
    fn width(&self, codes: &[pdf_font::Code], size: f32) -> f32 {
        let scale = self.appearance.horizontal_scaling;
        codes
            .iter()
            .filter(|code| **code != BREAK)
            .map(|code| {
                let word = if code.takes_word_spacing() {
                    self.appearance.word_spacing
                } else {
                    0.0
                };
                (self
                    .font
                    .advance(*code)
                    .mul_add(size, self.appearance.character_spacing)
                    + word)
                    * scale
            })
            .sum()
    }
}

/// Chooses the size §12.7.4.3 leaves to "an implementation dependent function".
///
/// The rule, stated once: the largest size at which the value fits the box on both axes. It is
/// found by halving rather than solved, because line breaking is not a continuous function of
/// the size — a size that fits on three lines may not fit on four — and because the same
/// search then serves the single-line and multiline cases.
fn auto_size(
    measure: &Measure,
    codes: &[pdf_font::Code],
    shape: Shape,
    width: f32,
    height: f32,
) -> f32 {
    let fits = |size: f32| {
        let lines = match shape {
            Shape::Multiline => wrap(measure, codes, size, width),
            Shape::SingleLine | Shape::Comb(_) => std::iter::once(0..codes.len()).collect(),
        };
        let widest = lines
            .iter()
            .map(|line| measure.width(line_codes(codes, line), size))
            .fold(0.0_f32, f32::max);
        widest <= width && size * LINE_HEIGHT * count(lines.len()) <= height
    };

    // The box's height is an upper bound on any size that can fit in it, whatever the text.
    let mut low = MIN_AUTO_SIZE;
    let mut high = height.max(MIN_AUTO_SIZE);
    if fits(high) {
        return high;
    }
    for _ in 0..AUTO_SIZE_STEPS {
        let middle = (low + high) * 0.5;
        if fits(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    low
}

/// Positions each line by `/Q` and writes it.
///
/// Where the lines sit vertically is the choice [`DEFAULT_DESCENT`] records: a single line is
/// centred in its box, several start at the top and run down, and in both the first baseline
/// sits one ascent below the top of the space the text occupies.
fn write_lines(
    stream: &mut String,
    lines: &[std::ops::Range<usize>],
    measure: &Measure,
    set: Set,
    leading: f32,
    request: &Request,
    written: Written,
) -> Option<Caret> {
    let box_ = request.box_;
    let matrix = written.matrix;
    let (width, height) = ((box_[2] - box_[0]).max(0.0), (box_[3] - box_[1]).max(0.0));
    let ascent = set.metrics.ascent * set.size;
    let descent = set.metrics.descent * set.size;
    let text_height = leading.mul_add(count(lines.len().saturating_sub(1)), ascent - descent);
    let top = if matches!(request.shape, Shape::Multiline) {
        box_[3]
    } else {
        box_[1] + (height + text_height) * 0.5
    };
    let first = top - ascent;
    let mut caret = None;

    for (index, line) in lines.iter().enumerate() {
        let codes = line_codes(written.codes, line);
        let advance = measure.width(codes, set.size);
        let x = match request.quadding {
            Quadding::Left => box_[0],
            Quadding::Centred => box_[0] + (width - advance) * 0.5,
            Quadding::Right => box_[0] + width - advance,
        };
        let baseline = leading.mul_add(-count(index), first);
        // The caret sits on the first line the offset reaches the end of, which decides the one
        // case the offset alone cannot: a soft wrap puts the same position at the end of one line
        // and the start of the next, and this is the end of the earlier one. A hard break is not
        // ambiguous — `BREAK` occupies an index of its own, so the position after it is past this
        // line's end and lands on the next.
        if caret.is_none()
            && let Some(at) = written.caret.filter(|at| *at <= line.end)
        {
            let before = codes
                .get(..at.saturating_sub(line.start).min(codes.len()))
                .unwrap_or(codes);
            let x = x + measure.width(before, set.size);
            caret = Some(Caret {
                from: [x, baseline + descent],
                to: [x, baseline + ascent],
            });
        }
        let _ = writeln!(
            stream,
            "{} {} {} {} {x} {baseline} Tm",
            matrix[0], matrix[1], matrix[2], matrix[3]
        );
        show(stream, codes);
    }
    caret
}

/// What is being written out, beside the box and the metrics it is written into.
///
/// Three things a line needs that are not properties of the *box*: the codes its range indexes,
/// where a caret goes if one was asked for, and the `/DA`'s own text matrix.
#[derive(Clone, Copy)]
struct Written<'a> {
    codes: &'a [pdf_font::Code],
    caret: Option<usize>,
    matrix: [f32; 6],
}

/// Whether the laid-out value needs more room than the box gives, on the axis §12.7.5.3 names.
///
/// > If set, the field shall not scroll (horizontally for single-line fields, vertically for
/// > multiple-line fields) to accommodate more text than fits within its annotation rectangle.
///
/// One axis per shape, because that is what the sentence states. The measurements are the ones
/// [`write_lines`] and [`comb`] use to place the text, deliberately: a question answered with a
/// different formula from the one that draws would say a value fits and then clip it.
///
/// A comb field is a third case the sentence does not spell out, and Table 231's own rule for
/// bit 25 settles it — the box is divided into `/MaxLen` positions and "the text is laid out
/// into those combs", so the room is a count rather than a length.
fn overflows(
    measure: &Measure,
    codes: &[pdf_font::Code],
    lines: &[std::ops::Range<usize>],
    set: Set,
    leading: f32,
    request: &Request,
) -> bool {
    let box_ = request.box_;
    let (width, height) = ((box_[2] - box_[0]).max(0.0), (box_[3] - box_[1]).max(0.0));
    match request.shape {
        Shape::Comb(cells) => {
            lines
                .iter()
                .flat_map(|line| line_codes(codes, line))
                .filter(|code| **code != BREAK)
                .count()
                .saturating_sub(usize::try_from(cells).unwrap_or(usize::MAX))
                > 0
        }
        Shape::SingleLine => lines
            .iter()
            .any(|line| measure.width(line_codes(codes, line), set.size) > width),
        // The same height [`write_lines`] lays out: one ascent above the first baseline, one
        // descent below the last, and a leading between each pair.
        Shape::Multiline => {
            let ascent = set.metrics.ascent * set.size;
            let descent = set.metrics.descent * set.size;
            leading.mul_add(count(lines.len().saturating_sub(1)), ascent - descent) > height
        }
    }
}

/// Breaks a run of codes into lines that fit a width.
///
/// Table 231's Multiline flag says only that "the field may contain multiple lines of text"
/// and nothing about where a line ends, so this is a layout rule of the kind the clause hands
/// to the processor: break at the last space that fits, and where a single word does not fit,
/// break inside it rather than let it run out of the box. An explicit line break in the value
/// always ends a line.
///
/// The running width is carried rather than remeasured, which is what keeps this linear:
/// [`auto_size`] runs it twenty-one times, and a value is a `/V` a document states, so an
/// implementation that measured the whole line per character would turn a long field value
/// into a quadratic cost twenty-one times over. Principle 3's rule about a document's own
/// numbers driving a loop.
fn wrap(
    measure: &Measure,
    codes: &[pdf_font::Code],
    size: f32,
    width: f32,
) -> Vec<std::ops::Range<usize>> {
    let mut lines = Vec::new();
    let mut start = 0_usize;
    let mut last_space: Option<usize> = None;
    let mut reached = 0.0_f32;

    for (index, code) in codes.iter().enumerate() {
        if *code == BREAK {
            lines.push(start..index);
            start = index.saturating_add(1);
            last_space = None;
            reached = 0.0;
            continue;
        }
        reached += measure.width(std::slice::from_ref(code), size);
        if code.takes_word_spacing() {
            last_space = Some(index.saturating_add(1));
        }
        if reached <= width || index.saturating_sub(start) == 0 {
            continue;
        }
        // Past the edge: break at the last space if there was one, otherwise between the
        // last two codes.
        let at = last_space.unwrap_or(index);
        lines.push(start..at);
        reached = measure.width(codes.get(at..=index).unwrap_or_default(), size);
        start = at;
        last_space = None;
    }
    lines.push(start..codes.len());
    lines
}

/// The codes of one line, from the range [`wrap`] produced.
///
/// Empty for a range no slice of `codes` answers, which cannot happen for a range this module
/// produced — every one of them is built from the same slice's own indices — and which is
/// answered without a panic rather than with one.
fn line_codes<'a>(
    codes: &'a [pdf_font::Code],
    line: &std::ops::Range<usize>,
) -> &'a [pdf_font::Code] {
    codes.get(line.clone()).unwrap_or_default()
}

/// Lays one character per comb, as §12.7.5.3's Table 231 bit 25 states.
///
/// > If set, the field shall be automatically divided into as many equally spaced positions,
/// > or combs , as the value of MaxLen , and the text is laid out into those combs.
///
/// The clause states the division and the one-per-comb rule. Where in its comb a character
/// sits is a layout rule, and centring it is the choice made here — a comb is a box drawn for
/// one character, and any other position would put the character against one of its edges.
/// Quadding chooses which combs are used when the value is shorter than `/MaxLen`.
fn comb(
    stream: &mut String,
    measure: &Measure,
    written: Written,
    set: Set,
    cell_count: u32,
    request: &Request,
) -> Option<Caret> {
    let size = set.size;
    let box_ = request.box_;
    let (width, height) = ((box_[2] - box_[0]).max(0.0), (box_[3] - box_[1]).max(0.0));
    let cells = count(usize::try_from(cell_count).unwrap_or(usize::MAX)).max(1.0);
    let cell = width / cells;
    let codes = written.codes;
    let shown: Vec<pdf_font::Code> = codes.iter().copied().filter(|c| *c != BREAK).collect();
    let used = count(shown.len());

    let first = match request.quadding {
        Quadding::Left => 0.0,
        Quadding::Centred => ((cells - used) * 0.5).max(0.0).floor(),
        Quadding::Right => (cells - used).max(0.0),
    };
    // A comb field is single-line by Table 231's own rule for bit 25, so the baseline is the
    // centred one.
    let baseline = size.mul_add(
        -set.metrics.descent,
        box_[1] + (height - size * (set.metrics.ascent - set.metrics.descent)) * 0.5,
    );

    for (index, code) in shown.iter().enumerate() {
        let slot = first + count(index);
        let advance = measure.width(std::slice::from_ref(code), size);
        let x = cell.mul_add(slot, (cell - advance) * 0.5) + box_[0];
        let _ = writeln!(stream, "1 0 0 1 {x} {baseline} Tm");
        show(stream, std::slice::from_ref(code));
    }

    // The caret goes at the *left edge* of the comb the next character would be laid into, rather
    // than beside the last character drawn: Table 231 bit 25 divides the box into positions and
    // puts one character in each, so the place a person is about to type into is a cell and not a
    // gap between glyphs. A caret past the last cell stays on the box's right edge, which is
    // where the value has stopped fitting.
    let at = written.caret?;
    let filled = codes.get(..at.min(codes.len())).unwrap_or_default();
    let slot = first + count(filled.iter().filter(|code| **code != BREAK).count());
    let x = cell.mul_add(slot.min(cells), 0.0) + box_[0];
    Some(Caret {
        from: [x, baseline + set.metrics.descent * size],
        to: [x, baseline + set.metrics.ascent * size],
    })
}

/// Writes one run of codes as a `Tj` with a literal string operand.
///
/// §7.3.4.2's three characters that shall be escaped inside a literal string — the
/// parentheses and the reverse solidus — plus every byte outside the printable ASCII range as
/// a three-digit octal escape, which the same subclause defines and which keeps the stream
/// free of bytes a text editor would mangle.
fn show(stream: &mut String, codes: &[pdf_font::Code]) {
    stream.push('(');
    for code in codes {
        if *code == BREAK {
            continue;
        }
        let byte = u8::try_from(code.value()).unwrap_or(b'?');
        match byte {
            b'(' | b')' | b'\\' => {
                stream.push('\\');
                stream.push(char::from(byte));
            }
            0x20..=0x7e => stream.push(char::from(byte)),
            other => {
                let _ = write!(stream, "\\{other:03o}");
            }
        }
    }
    stream.push_str(") Tj\n");
}

/// A parsed `/DA` string (§12.7.4.3, Table 228).
///
/// > The default appearance string ( DA ) contains any graphics state or text state operators
/// > needed to establish the graphics state parameters, such as text size and colour, for
/// > displaying the field's variable text.
struct DefaultAppearance {
    /// The operators to replay verbatim, in order, minus the `Tm` and anything the clause
    /// does not permit here.
    operators: String,
    /// The `Tf`'s first operand.
    font: Option<String>,
    /// The `Tf`'s second operand; `Some(0.0)` is the clause's auto-size request.
    size: Option<f32>,
    /// The one `Tm` the string may carry, whose translation this module replaces.
    matrix: Option<[f32; 6]>,
    /// `TL`, which decides the distance between lines when the string sets one.
    leading: Option<f32>,
    /// `Tc`, `Tw` and `Tz`, which change how wide the text measures.
    character_spacing: f32,
    word_spacing: f32,
    horizontal_scaling: f32,
}

/// The operators §12.7.4.3 admits in a `/DA`.
///
/// > Only operators that are allowed within text objects shall occur in this string
///
/// Table 50's three categories Figure 9 admits inside a text object and a `/DA` can carry:
/// text state, colour, and general graphics state. `q` and `Q` are excluded although Table 50
/// files them under the third — Figure 9 places them at the page description level, and an
/// unbalanced one inside the stream written here would leak into the rest of the page.
/// Text-positioning and text-showing operators are excluded because this module supplies them:
/// a `Tj` in a `/DA` would draw the string twice.
const PERMITTED: [&[u8]; 27] = [
    b"Tc", b"Tw", b"Tz", b"TL", b"Tr", b"Ts", b"CS", b"cs", b"SC", b"SCN", b"sc", b"scn", b"G",
    b"g", b"RG", b"rg", b"K", b"k", b"w", b"J", b"j", b"M", b"d", b"ri", b"i", b"gs", b"Tf",
];

impl DefaultAppearance {
    /// Reads a `/DA` string into the parts this module needs and the rest to replay.
    fn parse(bytes: &[u8]) -> Self {
        let mut parsed = Self {
            operators: String::new(),
            font: None,
            size: None,
            matrix: None,
            leading: None,
            character_spacing: 0.0,
            word_spacing: 0.0,
            // §9.3.5's `Tz` is "a percentage of the normal width", and Table 102's initial
            // value is 100.
            horizontal_scaling: 1.0,
        };

        let mut lexer = Lexer::new(bytes);
        let mut operands: Vec<Token> = Vec::new();
        let mut source = String::new();
        while let Some(token) = lexer.next_token() {
            let Token::Keyword(word) = &token else {
                write(&mut source, &token);
                operands.push(token);
                continue;
            };
            parsed.take(word, &operands, &source);
            operands.clear();
            source.clear();
        }
        parsed
    }

    /// Applies one operator and its operands.
    fn take(&mut self, operator: &[u8], operands: &[Token], source: &str) {
        let number = |index: usize| operands.get(index).and_then(as_number);
        match operator {
            b"Tf" => {
                // The clause names the operands in order: "a Tf (text font) operator along
                // with its two operands, font and size".
                self.font = operands.first().and_then(|token| match token {
                    Token::Name(name) => Some(String::from_utf8_lossy(name).into_owned()),
                    _ => None,
                });
                self.size = number(1);
                // Replayed by the caller with the resolved size, so it is not passed through.
                return;
            }
            b"Tm" if operands.len() >= 6 => {
                let mut matrix = [0.0; 6];
                for (slot, index) in matrix.iter_mut().zip(0..6) {
                    *slot = number(index).unwrap_or_default();
                }
                self.matrix = Some(matrix);
                return;
            }
            b"TL" => self.leading = number(0).filter(|value| *value > 0.0),
            b"Tc" => self.character_spacing = number(0).unwrap_or_default(),
            b"Tw" => self.word_spacing = number(0).unwrap_or_default(),
            b"Tz" => self.horizontal_scaling = number(0).unwrap_or(100.0) / 100.0,
            _ => {}
        }
        if !PERMITTED.contains(&operator) {
            return;
        }
        self.operators.push_str(source);
        self.operators.push_str(&String::from_utf8_lossy(operator));
        self.operators.push('\n');
    }
}

/// Writes one operand back out in the syntax it came in.
fn write(out: &mut String, token: &Token) {
    match token {
        Token::Integer(value) => {
            let _ = write!(out, "{value} ");
        }
        Token::Real(value) => {
            let _ = write!(out, "{} ", narrow(*value));
        }
        Token::Name(name) => {
            let _ = write!(out, "/{} ", String::from_utf8_lossy(name));
        }
        Token::ArrayOpen => out.push_str("[ "),
        Token::ArrayClose => out.push_str("] "),
        // A `/DA` holds graphics and text state operators, whose operands are numbers, names
        // and the one array a `d` takes. A string, a dictionary or a brace belongs to no
        // operator this module replays, and dropping it keeps the stream well formed.
        _ => {}
    }
}

/// A token's value as a number, for an operand.
///
/// An integer beyond `i32` is not a text size, a spacing or a matrix element on any page, so
/// narrowing it before widening loses nothing a `/DA` can legitimately state.
fn as_number(token: &Token) -> Option<f32> {
    match token {
        Token::Integer(value) => i32::try_from(*value).ok().map(|value| narrow(value.into())),
        Token::Real(value) => Some(narrow(*value)),
        _ => None,
    }
}

/// A count of lines, characters or combs as a number to multiply a length by.
///
/// Saturating at `u16::MAX` rather than casting: past sixty-five thousand lines the layout is
/// off the page whatever the arithmetic says, and an exact conversion keeps every number in
/// this module free of a lossy cast.
fn count(value: usize) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}

fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a text size or coordinate outside f32's range cannot place anything on a page"
    )]
    {
        value as f32
    }
}

/// Reads an entry as a §7.9.2.2 text string, from the first dictionary that states one.
pub(crate) fn string(document: &Document, sources: &[&Dictionary], key: &str) -> Option<String> {
    sources.iter().find_map(|source| {
        document
            .get_key(source, key)
            .as_string()
            .map(pdf_syntax::text_string)
    })
}

/// Reads an entry as raw bytes, from the first dictionary that states one.
///
/// A `/DA` is a *string* rather than a text string (Table 228), so its bytes are content
/// stream syntax and are not decoded.
pub(crate) fn bytes(document: &Document, sources: &[&Dictionary], key: &str) -> Option<Vec<u8>> {
    sources.iter().find_map(|source| {
        document
            .get_key(source, key)
            .as_string()
            .map(<[u8]>::to_vec)
    })
}

/// Resolves an object to a text string, following an array's first element.
///
/// §12.7.5.4: a choice field's value "is a text string representing the selected item" and,
/// where more than one is selected, "an array of such strings".
pub(crate) fn value_text(document: &Document, value: &Object) -> Option<String> {
    match document.resolve(value) {
        Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
        Object::Array(items) => items.iter().find_map(|item| value_text(document, item)),
        _ => None,
    }
}
