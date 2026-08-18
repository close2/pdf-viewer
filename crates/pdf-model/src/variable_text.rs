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
//! **One of the four does not take a value at all**, and it is the one the clause names first:
//! §12.7.4.3's own NOTE gives "scrollable list boxes whose contents are determined interactively
//! at the time the document is displayed" as an example of what has to be built here, and what
//! a list box shows is Table 234's `/Opt` array rather than Table 226's `/V`. [`Shape::ListBox`]
//! is where that difference lives.
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
//! **This section said a `/DA` naming a font `/DR` does not define was refused by name, and had
//! been false since the hundred-and-twenty-third session**, when ADR 0112 gave that case a
//! stand-in and a report — a free text annotation's text *is* its appearance, so refusing drew a
//! blank page. [`substituted_font`] and [`Resolution`] are what actually happens, and the
//! asymmetry between a font the document named and one this module inferred is stated there.
//!
//! What is refused whole, each because the clause's own answer is a refusal rather than a
//! shortfall: a `/DA` with no `Tf` ([`Owed::NoFont`]), a font that will not load or whose codes
//! cannot be reached from a character ([`Owed::FontUnusable`]), and a composite font whose
//! `CMap` asks for §9.7.5.1's writing mode 1 ([`Owed::VerticalWritingMode`]). A refusal here
//! leaves whatever appearance stream the file itself states standing, which is why it is the
//! right answer where a wrong axis or a wrong glyph would be the alternative.

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
    /// §12.7.5.4's scrollable list box, whose lines are Table 234's `/Opt` entries.
    ///
    /// The one shape whose text is not the field's value. §12.7.5.4 makes the `/Opt` array the
    /// thing displayed — each option "a text string that shall be displayed on the screen" —
    /// and Table 233 bit 20 fixes the order: "PDF readers shall display the options in the
    /// order in which they occur in the Opt array". So the caller joins the options with line
    /// breaks and this shape lays one out per line, top down, from wherever Table 234's `/TI`
    /// starts the window.
    ///
    /// It differs from [`Shape::Multiline`] in the two places a *list* differs from a
    /// paragraph, and each is stated beside the code that reads it: a line is an item, so it is
    /// never rewrapped, and the box is a scrolling window onto the array rather than a bound on
    /// it, so auto-sizing fits one line rather than all of them.
    ListBox,
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

/// What a *question* wants out of the layout, beside the stream it always writes.
///
/// **None of this is in ISO 32000-2**, and the empty value is what everything that only draws
/// asks for. The standard states where a glyph goes; a caret, the offset a click lands on and the
/// shapes covering a range of a value are what an interface needs on top of that, and each is
/// computed in the walk that places the glyphs rather than beside it — a second implementation of
/// §12.7.4.3's auto-sizing, wrapping and quadding would put a cursor next to the text instead of
/// in it. ADRs 0211 and 0225.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Asked {
    /// Where in the value a caret is wanted, as a byte offset — `None` for a layout that only
    /// draws. See [`LaidOut::caret`].
    pub caret: Option<usize>,
    /// A point in the box's own coordinates to turn into a byte offset of the value.
    ///
    /// The caret's inverse, and what a click inside a value needs. See [`LaidOut::offset`].
    pub point: Option<[f32; 2]>,
    /// A byte range of the value to answer with the shapes covering it.
    ///
    /// What a selection inside a value needs, and deliberately not two carets: §12.7.5.3's
    /// Multiline flag lets [`wrap`] break a line where a caller cannot see, so the lines between
    /// the two ends are this module's to name. See [`LaidOut::selection`].
    pub selection: Option<(usize, usize)>,
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
    /// What a question wants out of the layout, on top of the stream.
    ///
    /// [`Asked::default`] for everything that only draws, which is every caller but the three
    /// queries in [`crate::appearance`].
    pub asked: Asked,
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
    /// The `/DA` names a composite font whose `CMap` sets §9.7.5.1's writing mode 1.
    ///
    /// The name is carried so the report can say which. A whole refusal rather than a report
    /// beside a drawing: the clause makes the mode decide which metrics place the next glyph,
    /// and this module places them along one axis.
    VerticalWritingMode(String),
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
    /// §12.7.5.4's list box: its options are drawn and which of them are selected is not marked.
    ///
    /// **A report beside a drawing rather than a refusal**, since the round that read the clause
    /// past the sentence this variant was named after. §12.7.5.4 states what is *shown* — the
    /// `/Opt` array, in its own order, from Table 234's `/TI` — and states no appearance for the
    /// selection alone. A mark distinguishing a selected item is added over an item that is
    /// drawn either way, which is ADR 0106's test for an entry a refusal may not take the whole
    /// annotation down with.
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
            Self::VerticalWritingMode(name) => format!(
                "its /DA names the font /{name}, whose CMap sets §9.7.5.1's writing mode 1, and \
                 the variable text of §12.7.4.3 is laid out along the horizontal axis here"
            ),
            Self::InventedFontFellShort { name, characters } => format!(
                "its /DA names the font /{name}, which the interactive form dictionary's /DR \
                 does not define, and the standard font stood in for it states no code for \
                 {characters} — so the value is not drawn at all rather than in part"
            ),
            Self::CharactersNotInFont(characters) => {
                format!("its value contains {characters}, for which its /DA's font states no code")
            }
            Self::ListBoxSelection => "its /Opt options are drawn and which of them its value \
                                       selects is not marked, because §12.7.5.4 states no \
                                       appearance for a list box's selected items"
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
    /// Where a caret at [`Asked::caret`] sits, in the appearance's own coordinates.
    ///
    /// `None` where none was asked for, and where the value could not be laid out at all.
    pub caret: Option<Caret>,
    /// Which byte of the value [`Asked::point`] landed nearest, as an offset into it.
    ///
    /// The caret's inverse: an offset this answers with, handed back as [`Asked::caret`], puts the
    /// caret at the boundary the point was nearest. `None` where no point was asked about.
    pub offset: Option<usize>,
    /// The boxes covering [`Asked::selection`], in the appearance's own coordinates.
    ///
    /// `[x0, y0, x1, y1]` apiece, one per line the range touches, between the same descent and
    /// ascent the caret stands between — so a host draws a highlight the same height as a cursor.
    /// Empty where nothing was asked for, and where the range covers no glyph.
    pub selection: Vec<[f32; 4]>,
    /// How wide the widest line came out, at the size the layout chose.
    ///
    /// The sum of the advances [`write_lines`] positions each line by, so it is the same number
    /// the glyphs were placed with rather than a second measurement of them. §12.5.6.7's caption
    /// is what needs it: Figure 81 breaks the line around an inline caption, and where the break
    /// goes is where the text is.
    pub advance: f32,
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
/// [`Owed::FontUnusable`] where the font will not load or cannot be addressed by character, and
/// [`Owed::VerticalWritingMode`] where its `CMap` asks for §9.7.5.1's writing mode 1. Both are
/// named rather than guessed at.
fn set_in(
    document: &Document,
    request: &Request,
    font_name: &str,
) -> Result<(Dictionary, pdf_font::LoadedFont, Encoded, Resolution), Owed> {
    let (dict, resolution) = resolve_font(document, request.resources, font_name);
    let font = pdf_font::LoadedFont::load(document, &dict, font_name)
        .map_err(|error| Owed::FontUnusable(error.to_string()))?;
    // **The whole layout below is horizontal**, and §9.7.5.1 makes the writing mode decide
    // "which metrics shall be used when glyphs are painted from that font". A field's text
    // placed along x with a font whose displacement is `w1` and whose glyphs sit at `-v` would
    // be a confident wrong mark rather than a partial one, and refusing here leaves the
    // document's own appearance stream standing where it has one.
    if font.is_vertical() {
        return Err(Owed::VerticalWritingMode(font_name.to_owned()));
    }
    if !font.addresses_characters() {
        return Err(Owed::FontUnusable(format!(
            "/{font_name}'s /Encoding CMap states more codes than can be inverted, so no \
             character of the value can be turned into one (§9.7.6.2)"
        )));
    }

    let runs = encode(&font, request.text, request.asked);
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
    let again = encode(&reloaded, request.text, request.asked);
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
pub(crate) const LINE_HEIGHT: f32 = 13.0 / 12.0;

/// Where the baseline sits below the top of the em box, as a fraction of the font size.
///
/// **A choice.** Nothing in ISO 32000-2 says where in a field's box its text sits vertically —
/// the clause asks only for "positioning values [the processor] determines to be appropriate".
/// A font descriptor that states Table 120's `/Ascent` and `/Descent` is the document
/// answering the question and outranks this — where the pair could be a measurement of a face,
/// which is [`Metrics::read`]'s band. A standard-14 font has no descriptor at all, and
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
        Shape::ListBox => hard_lines(&runs.codes),
        Shape::SingleLine | Shape::Comb(_) => std::iter::once(0..runs.codes.len()).collect(),
    };

    let mut stream = String::new();
    open_marked_content(&mut stream, &appearance, (&font_name, size), box_);

    // One value for both branches, because everything but the matrix is the same in each and two
    // spellings of it would be two chances to leave a question out of one of them.
    let written = |matrix: [f32; 6]| Written {
        codes: &runs.codes,
        caret: runs.caret,
        selection: runs.selection,
        point: request.asked.point,
        offsets: &runs.offsets,
        matrix,
    };
    let marks = if let Shape::Comb(count) = request.shape {
        comb(
            &mut stream,
            &measure,
            // The identity, because a comb writes one `Tm` per cell and that is the matrix it
            // writes: Table 231 bit 25 places each character in a position of its own, so a
            // `/DA`'s own `Tm` has nothing here to translate.
            written(IDENTITY),
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
            written(appearance.matrix.unwrap_or(IDENTITY)),
        )
    };

    stream.push_str("ET\nQ\nEMC\n");
    Ok(LaidOut {
        content: stream,
        owed,
        caret: marks.caret,
        offset: marks.offset,
        selection: marks.selection,
        advance: marks.advance,
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

/// Table 120's `/Ascent` and `/Descent`, in text-space units where one em is 1.0.
struct Metrics {
    ascent: f32,
    descent: f32,
}

impl Metrics {
    /// Reads the font descriptor's vertical metrics, or falls back to the documented split.
    ///
    /// ISO 32000-2 §9.8.1's Table 120 defines both entries as measurements of the face rather
    /// than as free parameters. `/Ascent`:
    ///
    /// > The maximum height above the baseline reached by glyphs in this font. The height of
    /// > glyphs for accented characters shall be excluded.
    ///
    /// and `/Descent`:
    ///
    /// > The maximum depth below the baseline reached by glyphs in this font. The value shall be
    /// > a negative number.
    ///
    /// Both are in glyph space, whose unit §9.2.4 makes one thousandth of a text space unit.
    /// Both are required of every font descriptor and a descriptor is required of every font but
    /// the standard 14 — which is exactly the case a `/DA` usually names, so the fallback is the
    /// common path rather than the odd one.
    ///
    /// **Whether a stated pair could be a measurement at all is
    /// [`pdf_font::measured_extent`]'s question**, and asking it here is what ADR 0216 named as
    /// a divergence and left for a round that could measure it. The guard used to be
    /// `ascent > 0.0 && descent < 0.0`, which is a statement that the pair straddles the
    /// baseline and nothing more: it believed `/Ascent 4000 /Descent -1140`, a five-em line that
    /// would put a field's baseline four ems above the bottom of its own box, and refused
    /// `/Ascent 905 /Descent 211`, which is Arial's real metrics with Table 120's sign
    /// convention broken rather than its measurement withheld. Sharing the band settles both,
    /// and one rule for the two things this tree reads these entries for is worth more than the
    /// two constants either would have needed.
    ///
    /// A pair the band refuses gets [`DEFAULT_ASCENT`] and [`DEFAULT_DESCENT`] rather than
    /// `pdf_font::vertical_extent`'s em box, and the difference is not an oversight: that one
    /// answers *how tall is this line* for a selection highlight, and this one answers *where in
    /// its box does this field's text sit*, which is the choice those two constants record.
    fn read(document: &Document, dict: &Dictionary) -> Self {
        let descriptor = descriptor_of(document, dict);
        let read = |key: &str| {
            descriptor
                .as_dict()
                .map(|descriptor| document.get_key(descriptor, key))
                .and_then(|value| value.as_number())
                .map(narrow)
                .filter(|value| value.is_finite())
        };
        let stated = match (read("Ascent"), read("Descent")) {
            (Some(ascent), Some(descent)) => pdf_font::measured_extent(ascent, descent),
            _ => None,
        };
        stated.map_or(
            Self {
                ascent: DEFAULT_ASCENT,
                descent: DEFAULT_DESCENT,
            },
            |(ascent, descent)| Self { ascent, descent },
        )
    }
}

/// One position in the laid-out value: a code the font draws, or a break that draws nothing.
///
/// **Not a [`pdf_font::Code`] with a sentinel value, and the difference is a defect this shape
/// removes.** A line break used to travel through the layout as the code for a line feed, which
/// is sound exactly as long as no font gives that code a glyph. §9.6.5.1's `/Differences` lets a
/// simple font's encoding put one there, and §9.7.6.2 lets a composite font's `CMap` state a
/// one-byte code 10 outright — and either would have had a *character of the value* laid out as
/// a break, dropped by [`show`] and never drawn, with nothing reported. The enum makes the two
/// unmistakable to the compiler, which is the construction principle 4 asks for over a comment
/// warning about the collision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Placed {
    /// §12.7.5.3's line break: [`wrap`] ends a line at it and [`show`] writes nothing for it.
    Break,
    /// A character code of the `/DA`'s font, from [`pdf_font::LoadedFont::code_for`].
    Shown(pdf_font::Code),
}

impl Placed {
    /// The code to draw, or `None` for a break.
    fn code(self) -> Option<pdf_font::Code> {
        match self {
            Self::Break => None,
            Self::Shown(code) => Some(code),
        }
    }

    /// Whether §9.3.3's word spacing applies here, which it never does to a break.
    fn takes_word_spacing(self) -> bool {
        self.code().is_some_and(pdf_font::Code::takes_word_spacing)
    }
}

/// The font descriptor a font dictionary reaches, following a Type 0 font to its descendant.
///
/// Table 119 lists six entries for a Type 0 font dictionary and `/FontDescriptor` is not among
/// them: the descriptor belongs to Table 115's `CIDFont`, which §9.7.4.1 says "shall be used
/// only as a descendant of a Type 0 font". So a reader of Table 120's `/Ascent` and `/Descent`
/// that stopped at the dictionary the `/DA` names would find nothing for *every* composite font
/// and fall back to [`DEFAULT_ASCENT`] without ever saying that the document had stated a pair.
///
/// One indirection deep and no further: a descendant is a `CIDFont`, and Table 115 gives it no
/// descendants of its own.
fn descriptor_of(document: &Document, dict: &Dictionary) -> Object {
    let stated = document.get_key(dict, "FontDescriptor");
    if stated.as_dict().is_some() {
        return stated;
    }
    let descendants = document.get_key(dict, "DescendantFonts");
    descendants
        .as_array()
        .and_then(<[Object]>::first)
        .map(|first| document.resolve(first))
        .map_or(Object::Null, |descendant| {
            descendant.as_dict().map_or(Object::Null, |descendant| {
                document.get_key(descendant, "FontDescriptor")
            })
        })
}

/// The codes a value became, and the characters that produced none.
struct Encoded {
    codes: Vec<Placed>,
    /// The distinct characters the font states no code for, ready for a report.
    missing: String,
    /// Whether [`MAX_CODES`] was reached and the rest of the value dropped.
    truncated: bool,
    /// Which code [`Asked::caret`]'s byte offset falls before, where one was asked for.
    ///
    /// An index into [`Self::codes`] rather than into the value, because that is what the layout
    /// works in: a character the font states no code for produces none, and a caret cannot sit
    /// beside a glyph that is not drawn.
    caret: Option<usize>,
    /// The same translation for [`Asked::selection`]'s two ends, in the same direction.
    selection: Option<(usize, usize)>,
    /// Which byte of the value each code came from, with the end after the last.
    ///
    /// The mapping the other two run backwards, and the only one that needs a vector: a point
    /// lands between two codes and the answer owed is a byte offset. Built **only** where
    /// [`Asked::point`] asked for one, so that nothing that draws pays for it.
    offsets: Vec<usize>,
}

/// Turns a value into the font's own character codes, collecting what it cannot represent.
///
/// The byte offsets [`Asked`] carries are turned into code indices here, in the one loop that
/// knows how many codes a character produced — which is none for a character the font cannot
/// spell, and one for each of §12.7.5.3's line breaks. An offset inside a character counts that
/// character as still to come, and one past the end is the end. [`Encoded::offsets`] is the same
/// correspondence written down, for the one question that runs it backwards.
fn encode(font: &pdf_font::LoadedFont, text: &str, asked: Asked) -> Encoded {
    let mut codes = Vec::with_capacity(text.len().min(MAX_CODES));
    let mut missing = String::new();
    let mut truncated = false;
    let wanted = [
        asked.caret,
        asked.selection.map(|(from, _)| from),
        asked.selection.map(|(_, to)| to),
    ];
    let mut marks: [Option<usize>; 3] = [None; 3];
    let mut offsets = Vec::new();
    // A carriage return, a line feed and the pair are all one line break, and none of them is
    // a glyph. They travel through the layout as `Placed::Break` and `show` drops them. Read
    // from the value as it stands rather than from a normalised copy, so that a caret's offset
    // means the same thing here as it does to the host that sent it.
    let mut after_return = false;
    let mut end = text.len();
    for (at, character) in text.char_indices() {
        for (mark, want) in marks.iter_mut().zip(wanted) {
            if mark.is_none() && want.is_some_and(|offset| offset <= at) {
                *mark = Some(codes.len());
            }
        }
        if codes.len() >= MAX_CODES {
            truncated = true;
            end = at;
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
        let produced = if character == '\n' {
            Some(Placed::Break)
        } else {
            match font
                .code_for(character)
                .or_else(|| substitutable(character, font))
            {
                Some(code) => Some(Placed::Shown(code)),
                None if missing.contains(character) => None,
                None => {
                    missing.push(character);
                    None
                }
            }
        };
        if let Some(code) = produced {
            codes.push(code);
            if asked.point.is_some() {
                offsets.push(at);
            }
        }
    }
    if asked.point.is_some() {
        // The end, so that an index of `codes.len()` — a point past the last glyph — has a byte
        // offset to answer with.
        offsets.push(end);
    }
    // An offset no character index reached is one past the value's last character, which is the
    // end of the codes — the case a caret at the end of a value is in every time.
    let last = codes.len();
    Encoded {
        caret: wanted[0].map(|_| marks[0].unwrap_or(last)),
        selection: asked
            .selection
            .map(|_| (marks[1].unwrap_or(last), marks[2].unwrap_or(last))),
        offsets,
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
    fn width(&self, codes: &[Placed], size: f32) -> f32 {
        let scale = self.appearance.horizontal_scaling;
        codes
            .iter()
            .filter_map(|placed| placed.code())
            .map(|code| {
                let word = if code.takes_word_spacing() {
                    self.appearance.word_spacing
                } else {
                    0.0
                };
                (self
                    .font
                    .advance(code)
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
///
/// **§12.7.5.4's list box takes the same rule, and Table 234's `/TI` is why it needs no
/// exception of its own.** A first attempt gave it one, reasoning that a list box's rectangle is
/// a scrolling *window* onto its options rather than a bound on them, so the height to fit was
/// one line rather than all of them. The picture rejected it: fitting a single line's width to
/// the whole box made a 120-point-wide list of six short labels choose 34-point type and show
/// two and a half of them, which is not a list. The window argument survives and is answered
/// somewhere better — [`crate::appearance`] lays out the options **from `/TI` onward**, so the
/// run measured here already *is* the window a producer scrolled to, and fitting it to the box
/// is fitting the visible options. Trap 1's rule applied to a rule rather than to a defect.
fn auto_size(measure: &Measure, codes: &[Placed], shape: Shape, width: f32, height: f32) -> f32 {
    let fits = |size: f32| {
        let lines = match shape {
            Shape::Multiline => wrap(measure, codes, size, width),
            Shape::ListBox => hard_lines(codes),
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
) -> Marks {
    let box_ = request.box_;
    let matrix = written.matrix;
    let (width, height) = ((box_[2] - box_[0]).max(0.0), (box_[3] - box_[1]).max(0.0));
    let ascent = set.metrics.ascent * set.size;
    let descent = set.metrics.descent * set.size;
    let text_height = leading.mul_add(count(lines.len().saturating_sub(1)), ascent - descent);
    // A list box runs from the top for the reason a paragraph does and one more: Table 234's
    // `/TI` names the option the list *starts* at, so the first line drawn has to be the first
    // line of the box or the entry names nothing.
    let top = if matches!(request.shape, Shape::Multiline | Shape::ListBox) {
        box_[3]
    } else {
        box_[1] + (height + text_height) * 0.5
    };
    let first = top - ascent;
    let mut marks = Marks::default();
    let mut nearest: Option<(f32, f32, usize)> = None;

    for (index, line) in lines.iter().enumerate() {
        let codes = line_codes(written.codes, line);
        let advance = measure.width(codes, set.size);
        marks.advance = marks.advance.max(advance);
        let x = match request.quadding {
            Quadding::Left => box_[0],
            Quadding::Centred => box_[0] + (width - advance) * 0.5,
            Quadding::Right => box_[0] + width - advance,
        };
        let baseline = leading.mul_add(-count(index), first);
        // The caret sits on the first line the offset reaches the end of, which decides the one
        // case the offset alone cannot: a soft wrap puts the same position at the end of one line
        // and the start of the next, and this is the end of the earlier one. A hard break is not
        // ambiguous — a break occupies an index of its own, so the position after it is past this
        // line's end and lands on the next.
        if marks.caret.is_none()
            && let Some(at) = written.caret.filter(|at| *at <= line.end)
        {
            let before = codes
                .get(..at.saturating_sub(line.start).min(codes.len()))
                .unwrap_or(codes);
            let x = x + measure.width(before, set.size);
            marks.caret = Some(Caret {
                from: [x, baseline + descent],
                to: [x, baseline + ascent],
            });
        }
        // The part of the selected range this line holds, as one box between the same descent
        // and ascent the caret stands between: a highlight and a cursor the same height is what
        // makes the two look like one thing, and neither is the standard's.
        if let Some((from, to)) = written.selection {
            let (start, end) = (from.max(line.start), to.min(line.end));
            if start < end {
                let x0 =
                    x + measure.width(line_codes(written.codes, &(line.start..start)), set.size);
                let x1 = x + measure.width(line_codes(written.codes, &(line.start..end)), set.size);
                // A range covering nothing but a line break has no shape, because a break draws
                // no glyph — the same reason `show` skips it.
                if x1 > x0 {
                    marks
                        .selection
                        .push([x0, baseline + descent, x1, baseline + ascent]);
                }
            }
        }
        if let Some([px, py]) = written.point {
            // Vertically first and horizontally within the line, which is the order the two
            // questions are asked in: a point below every line belongs to the last one, and a
            // point past the end of a line belongs to that line's end.
            let (bottom, top) = (baseline + descent, baseline + ascent);
            let dy = if py < bottom {
                bottom - py
            } else if py > top {
                py - top
            } else {
                0.0
            };
            let (at, dx) = nearest_boundary(measure, codes, set.size, x, px);
            if nearest
                .is_none_or(|(best_y, best_x, _)| dy < best_y || (dy <= best_y && dx < best_x))
            {
                nearest = Some((dy, dx, line.start.saturating_add(at)));
            }
        }
        let _ = writeln!(
            stream,
            "{} {} {} {} {x} {baseline} Tm",
            matrix[0], matrix[1], matrix[2], matrix[3]
        );
        show(stream, codes);
    }
    marks.offset = nearest.map(|(_, _, code)| written.byte(code));
    marks
}

/// The boundary between two glyphs a point is nearest, within one line already positioned.
///
/// The index of the code the boundary is *before*, and how far the point was from it. Walked
/// rather than solved because the advance of a code is the font's and the running total is the
/// same one [`Measure::width`] sums — a boundary found by any other arithmetic would be a second
/// opinion about where the glyphs are.
fn nearest_boundary(
    measure: &Measure,
    codes: &[Placed],
    size: f32,
    x: f32,
    point: f32,
) -> (usize, f32) {
    let mut at = x;
    let mut best = ((point - x).abs(), 0_usize);
    for (index, code) in codes.iter().enumerate() {
        at += measure.width(std::slice::from_ref(code), size);
        let distance = (point - at).abs();
        if distance < best.0 {
            best = (distance, index.saturating_add(1));
        }
    }
    (best.1, best.0)
}

/// What a question asked of the layout, in the appearance's own coordinates.
///
/// Three answers from one walk, because they are three views of the same placement: where the
/// next glyph goes, which glyph boundary a point is nearest, and the boxes a range of the value
/// occupies. [`LaidOut`] carries them out.
#[derive(Default)]
struct Marks {
    caret: Option<Caret>,
    offset: Option<usize>,
    selection: Vec<[f32; 4]>,
    advance: f32,
}

/// What is being written out, beside the box and the metrics it is written into.
///
/// The codes a line's range indexes, the `/DA`'s own text matrix, and the three questions
/// [`Asked`] carries — the first two of them already translated from bytes into code indices by
/// [`encode`], because that is the space a line is measured in.
#[derive(Clone, Copy)]
struct Written<'a> {
    codes: &'a [Placed],
    caret: Option<usize>,
    selection: Option<(usize, usize)>,
    point: Option<[f32; 2]>,
    /// Which byte of the value each code came from, from [`Encoded::offsets`].
    offsets: &'a [usize],
    matrix: [f32; 6],
}

impl Written<'_> {
    /// The byte of the value a code index stands at.
    ///
    /// The last entry for an index past the end, which is what [`encode`] put there: a point past
    /// the last glyph is the end of the value. Zero where no offsets were collected, which cannot
    /// happen for a caller that asked for a point and is answered without a panic rather than with
    /// one.
    fn byte(self, code: usize) -> usize {
        self.offsets
            .get(code)
            .or_else(|| self.offsets.last())
            .copied()
            .unwrap_or_default()
    }
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
    codes: &[Placed],
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
                .filter(|placed| !matches!(placed, Placed::Break))
                .count()
                .saturating_sub(usize::try_from(cells).unwrap_or(usize::MAX))
                > 0
        }
        Shape::SingleLine => lines
            .iter()
            .any(|line| measure.width(line_codes(codes, line), set.size) > width),
        // Table 231 is §12.7.5.3's alone and reaches no choice field, so no flag asks this
        // question of a list box — and the answer it would want is *no* either way: §12.7.5.4
        // makes the control "a scrollable list box" and Table 234's `/TI` says which option its
        // window starts at, so more options than the box holds is the arrangement the clause
        // describes rather than a value that will not fit.
        Shape::ListBox => false,
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
fn wrap(measure: &Measure, codes: &[Placed], size: f32, width: f32) -> Vec<std::ops::Range<usize>> {
    let mut lines = Vec::new();
    let mut start = 0_usize;
    let mut last_space: Option<usize> = None;
    let mut reached = 0.0_f32;

    for (index, code) in codes.iter().enumerate() {
        if matches!(code, Placed::Break) {
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

/// Breaks a run of codes at its line breaks and nowhere else.
///
/// [`wrap`] without the width, and the difference is the whole of what a list is: §12.7.5.4's
/// options are separate *items*, so an option too wide for the box is clipped by the box's own
/// clip rather than continued on the next line. Rewrapping it would show the array as holding
/// more entries than it does, against the sentence that makes each option "a text string that
/// shall be displayed on the screen" — one string, one item, one line.
fn hard_lines(codes: &[Placed]) -> Vec<std::ops::Range<usize>> {
    let mut lines = Vec::new();
    let mut start = 0_usize;
    for (index, code) in codes.iter().enumerate() {
        if matches!(code, Placed::Break) {
            lines.push(start..index);
            start = index.saturating_add(1);
        }
    }
    lines.push(start..codes.len());
    lines
}

/// The codes of one line, from the range [`wrap`] produced.
///
/// Empty for a range no slice of `codes` answers, which cannot happen for a range this module
/// produced — every one of them is built from the same slice's own indices — and which is
/// answered without a panic rather than with one.
fn line_codes<'a>(codes: &'a [Placed], line: &std::ops::Range<usize>) -> &'a [Placed] {
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
) -> Marks {
    let size = set.size;
    let box_ = request.box_;
    let (width, height) = ((box_[2] - box_[0]).max(0.0), (box_[3] - box_[1]).max(0.0));
    let cells = count(usize::try_from(cell_count).unwrap_or(usize::MAX)).max(1.0);
    let cell = width / cells;
    let codes = written.codes;
    let shown: Vec<Placed> = codes
        .iter()
        .copied()
        .filter(|placed| !matches!(placed, Placed::Break))
        .collect();
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

    let (bottom, top) = (
        baseline + set.metrics.descent * size,
        baseline + set.metrics.ascent * size,
    );
    // Everything below is stated in *cells*, because Table 231 bit 25 divides the box into
    // positions and puts one character in each: the place a person is about to type into is a
    // cell and not a gap between glyphs, and so is the place a click lands on and the run a
    // selection covers. `edge` is the left side of the comb a slot names, clamped to the box —
    // a position past the last cell stays on the right edge, which is where the value has
    // stopped fitting.
    let slot_of = |at: usize| {
        let filled = codes.get(..at.min(codes.len())).unwrap_or_default();
        first
            + count(
                filled
                    .iter()
                    .filter(|placed| !matches!(placed, Placed::Break))
                    .count(),
            )
    };
    let edge = |slot: f32| cell.mul_add(slot.min(cells), 0.0) + box_[0];

    let caret = written.caret.map(|at| Caret {
        from: [edge(slot_of(at)), bottom],
        to: [edge(slot_of(at)), top],
    });
    let mut selection = Vec::new();
    if let Some((from, to)) = written.selection {
        let (x0, x1) = (edge(slot_of(from)), edge(slot_of(to)));
        if x1 > x0 {
            selection.push([x0, bottom, x1, top]);
        }
    }
    let mut offset = None;
    if let Some([point, _]) = written.point {
        // The nearest cell edge, and then the code that cell holds. A comb is single-line by the
        // same rule, so the vertical half of the question does not arise.
        let mut nearest = (0_usize, (point - edge(first)).abs());
        for index in 1..=shown.len() {
            let distance = (point - edge(first + count(index))).abs();
            if distance < nearest.1 {
                nearest = (index, distance);
            }
        }
        // Back from a count of *shown* characters to an index into the codes, which differ by the
        // line breaks a comb does not lay out.
        let mut seen = 0_usize;
        let mut at = codes.len();
        for (index, code) in codes.iter().enumerate() {
            if seen == nearest.0 {
                at = index;
                break;
            }
            if !matches!(code, Placed::Break) {
                seen = seen.saturating_add(1);
            }
        }
        offset = Some(written.byte(at));
    }
    Marks {
        caret,
        offset,
        selection,
        // A comb occupies the cells Table 231 bit 25 divides the box into rather than the sum of
        // its advances, so the width it fills is the box's own up to the last cell it reached.
        advance: edge(slot_of(codes.len())) - box_[0],
    }
}

/// Writes one run of codes as a `Tj` with a literal string operand.
///
/// §7.3.4.2's three characters that shall be escaped inside a literal string — the
/// parentheses and the reverse solidus — plus every byte outside the printable ASCII range as
/// a three-digit octal escape, which the same subclause defines and which keeps the stream
/// free of bytes a text editor would mangle.
///
/// **A code is its bytes rather than a byte**, which is §9.7.6.2's doing: a composite font's
/// code is one to four bytes, most significant first, and the interpreter reading this stream
/// back splits the operand by the same `CMap`'s codespace ranges that produced the code. A
/// two-byte code written as one byte would decode to something else entirely, which is the
/// failure this function had while nothing could produce such a code.
fn show(stream: &mut String, codes: &[Placed]) {
    stream.push('(');
    for code in codes.iter().filter_map(|placed| placed.code()) {
        for byte in code_bytes(code) {
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
    }
    stream.push_str(") Tj\n");
}

/// A code's bytes, most significant first, as many of them as §9.7.6.2 gives it.
///
/// > A sequence of one or more bytes shall be extracted from the string and matched against the
/// > codespace ranges in the CMap.
///
/// The inverse of that extraction, and the only place this crate turns a code back into the
/// bytes a string states it with. A simple font's code is one byte by §9.7.1, which is the same
/// arithmetic with a length of one.
fn code_bytes(code: pdf_font::Code) -> impl Iterator<Item = u8> {
    let value = code.value().to_be_bytes();
    let length = usize::from(code.length()).min(value.len());
    // The low `length` bytes, which is where `value_of` put them.
    value
        .into_iter()
        .skip(value.len().saturating_sub(length))
        .take(length)
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
///
/// **Marked content is admitted inside a text object and is not on this list**, which is a
/// documented choice rather than an oversight. Errata Collection 3 strikes §14.6.1's "[t]hey may
/// not occur within a graphics object" (Issue #335) and states outright that a marked-content
/// sequence may appear "within a text object", so Figure 9 no longer excludes `BMC`, `BDC`,
/// `EMC`, `MP` and `DP` from where a `/DA` runs — and this list drops them, silently, on the
/// older reading. What is lost is nothing: this module replays a `/DA`'s operators into the
/// appearance stream it builds, and a bracket round the text it draws marks a sequence nothing
/// downstream of here reads. Admitting them would have to carry the nesting as well, because an
/// unbalanced `BDC` would leak into the rest of the stream exactly as `q` would.
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
        let mut operands: Vec<Token<'_>> = Vec::new();
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
    fn take(&mut self, operator: &[u8], operands: &[Token<'_>], source: &str) {
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
fn write(out: &mut String, token: &Token<'_>) {
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
fn as_number(token: &Token<'_>) -> Option<f32> {
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
