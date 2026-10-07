//! A rich text string laid out into §12.7.4.3's appearance stream.
//!
//! The construction is §12.7.4.3's own wherever a rich text string leaves it standing — the
//! marks are clipped to the box, written inside the `/Tx BMC` … `EMC` pair, under the `/DA`'s
//! operators — and XFA 3.3 section 27's wherever the string states something the plain layout
//! cannot: a face, a size, a weight, a posture, a colour, an alignment, a margin, an indent, a
//! line height, a baseline shift, a decoration, a letter spacing or a glyph scale per run, and a
//! list item's tag. Each run is set in a face [`Faces`] chooses, its codes come out of the same
//! encoding a plain value goes through ([`variable_text::Face`]), and its advances are measured by
//! the formula that draws them, which is §9.4.4's: `(w0 × size + Tc + Tw) × Th`.
//!
//! # What is chosen here, and where the plain layout made the same choice
//!
//! The two texts hand a processor the positions — §12.7.4.3's "positioning values it determines
//! to be appropriate", and chapter 27's own description of its examples as informative — so
//! where a baseline sits and how far apart two lines are is chosen, once, and chosen to be what
//! [`variable_text`] already does: a single line centred in its box, several running down from
//! its top, the first baseline one ascent below it, and one line `13/12` of its tallest size
//! below the last ([`variable_text::LINE_HEIGHT`]). So a rich text string whose every run has
//! the `/DA`'s style lands where the plain value would. ADR 1634 section 5 lists the rest:
//! where an underline and a line through sit and how thick they are, which CSS2 section 16.3.1
//! leaves to the reader too, and the two list measurements chapter 27 fixes itself.

use std::fmt::Write as _;

use pdf_syntax::{Dictionary, Document, Name, Object};

use super::markup::{ListIndent, Piece, RichText};
use super::style::{
    Align, Character, Linear, NORMAL_STRETCH, STRETCHES, Spacing, Underline, VerticalAlign,
};
use crate::variable_text::{
    self, Asked, Caret, DefaultAppearance, Face, Glyph, LaidOut, Order, Owed, Placed, Quadding,
    Resolution,
};
use pdf_font::shaping::{Paragraphs as Levels, Shaped};

/// Chapter 27's single list indent: half an inch, which *List Layout* fixes (page 1218).
const LIST_INDENT: f32 = 36.0;

/// Chapter 27's tag gap: six points between a tag's right edge and the item's content.
const TAG_GAP: f32 = 6.0;

/// Where an underline sits below the baseline, in ems. **A choice**: chapter 27 says an underline
/// is drawn at the baseline in the text's colour (page 1208) and CSS2 section 16.3.1 leaves its
/// position and thickness to the reader; a tenth of an em clears the baseline of a Latin face and
/// stays above its descenders' depth.
const UNDERLINE_BELOW: f32 = 0.1;

/// How thick a decoration line is, in ems: the same choice's other number.
const DECORATION_THICKNESS: f32 = 0.05;

/// How far below the first a double underline's second line sits, in ems.
const SECOND_UNDERLINE_BELOW: f32 = 0.1;

/// Where a line through sits above the baseline, in ems: about half a Latin face's x-height,
/// which is where the line crosses the lower-case letters it strikes.
const LINE_THROUGH_ABOVE: f32 = 0.28;

/// One layout job.
pub(crate) struct Request<'a> {
    /// The string, read.
    pub(crate) rich: &'a RichText,
    /// The region the text is laid out in, in the appearance's own coordinates.
    pub(crate) box_: [f32; 4],
    /// Table 228's `/DA`, whose font, size and colour are the text's where nothing else states one.
    pub(crate) default_appearance: &'a [u8],
    /// Table 224's `/DR`, where the `/DA`'s font and every face a run names are looked for.
    pub(crate) resources: &'a Dictionary,
    /// Table 228's `/Q`, the alignment where the string states none.
    pub(crate) quadding: Quadding,
    /// Whether the text may run over several lines: Table 231 bit 13 for a field, and always for
    /// a free text annotation.
    pub(crate) multiline: bool,
    /// Table 231 bit 25's cells, where the field is a comb of this many.
    pub(crate) comb: Option<u32>,
    /// What a host asked of the layout beside the stream: a caret, a point, a range, the glyphs.
    pub(crate) asked: Asked,
    /// The characters the offsets in [`Self::asked`] index, and the answers' offsets index: the
    /// value a host edits, whose words [`Self::rich`] states (ADR 1635's comparison).
    pub(crate) value: &'a str,
}

/// Lays the string out.
///
/// # Errors
///
/// What [`variable_text::lay_out`] refuses for its `/DA` — no `Tf`, or a font that will not load
/// — and [`Owed::InventedFontFellShort`] where a character no face of the string's draws would
/// have been left out of a face this program chose: the caller then lays the characters out in
/// the one-style layout, which reaches a machine face for them (ADR 1414).
pub(crate) fn lay_out(document: &Document, request: &Request) -> Result<LaidOut, Owed> {
    let appearance = DefaultAppearance::parse(request.default_appearance);
    let Some(base_name) = appearance.font.clone() else {
        return Err(Owed::NoFont);
    };
    let mut faces = Faces::new(document, request.resources, &base_name)?;
    let styles = Styles::collect(request.rich);
    let characters = request.rich.text();
    let levels = Levels::new(&characters);
    let shaping = Shaping::of(&characters, levels.as_ref());
    let paragraphs = encode(request, &styles, &mut faces, &shaping);
    if !paragraphs.missing.is_empty() && faces.invented_in_use() {
        return Err(Owed::InventedFontFellShort {
            name: base_name,
            characters: paragraphs.missing,
        });
    }
    let measure = Measure {
        faces: &faces,
        styles: &styles,
        appearance: &appearance,
    };
    let area = Room::of(request.box_, request.rich);
    let root = match appearance.size {
        Some(size) if size > 0.0 => size,
        // §12.7.4.3: "A zero value for size means that the font shall be auto-sized". The rule is
        // the plain layout's — the largest size at which the text fits — with every size the
        // string states relative to it following it, and every absolute one standing.
        _ => auto_size(&measure, &paragraphs.list, area, request),
    };
    let plan = plan(
        &measure,
        &paragraphs.list,
        root,
        area,
        request,
        levels.as_ref(),
    );

    let mut stream = String::new();
    let (width, height) = (
        (request.box_[2] - request.box_[0]).max(0.0),
        (request.box_[3] - request.box_[1]).max(0.0),
    );
    stream.push_str("/Tx BMC\nq\n");
    let _ = writeln!(
        stream,
        "{} {} {width} {height} re W n",
        request.box_[0], request.box_[1]
    );
    stream.push_str("BT\n");
    stream.push_str(&appearance.operators);
    let mut state = State::default();
    let mut decorations = String::new();
    for line in &plan.lines {
        write_line(
            &mut stream,
            &mut decorations,
            &mut state,
            &measure,
            line,
            root,
        );
    }
    stream.push_str("ET\n");
    stream.push_str(&decorations);
    stream.push_str("Q\nEMC\n");

    let owed = if faces.base_resolution == Resolution::StoodIn {
        Some(Owed::FontNotInResources(base_name))
    } else {
        owed_by(paragraphs, request, &faces, &plan)
    };
    let answers = if request.asked == Asked::default() {
        Answers::default()
    } else {
        answer(&plan, request, &Alignment::of(&characters, request.value))
    };
    Ok(LaidOut {
        content: stream,
        owed,
        overflows: plan.overflows(request),
        caret: answers.caret,
        offset: answers.offset,
        selection: answers.selection,
        glyphs: answers.glyphs,
        advance: plan.advance,
        fonts: faces.invented(),
    })
}

/// What the layout could not show, most telling first: what was cut, what no face draws, what is
/// drawn in a form other than the one displayed, and what the string states that is not carried
/// out.
fn owed_by(paragraphs: Paragraphs, request: &Request, faces: &Faces, plan: &Plan) -> Option<Owed> {
    if paragraphs.truncated {
        return Some(Owed::Truncated(variable_text::MAX_CODES));
    }
    if !paragraphs.missing.is_empty() {
        return Some(Owed::CharactersNotInFont(paragraphs.missing));
    }
    if !paragraphs.unformed.is_empty() {
        return Some(Owed::FormsNotInFont(paragraphs.unformed));
    }
    let mut unapplied = request.rich.unapplied.clone();
    for width in &faces.unmet_widths {
        unapplied.note(format!("font-stretch:{width}"));
    }
    if plan.tabs_reversed {
        unapplied.note("a tab stop in a line read right to left");
    }
    (!unapplied.is_empty()).then(|| Owed::RichTextUnapplied(unapplied.phrase()))
}

/// Every distinct character style the string uses, so a glyph names its style by index.
struct Styles(Vec<Character>);

impl Styles {
    fn collect(rich: &RichText) -> Self {
        let mut styles: Vec<Character> = Vec::new();
        let mut add = |style: &Character| {
            if !styles.contains(style) {
                styles.push(style.clone());
            }
        };
        for paragraph in &rich.paragraphs {
            add(&paragraph.strut);
            if let Some(tag) = &paragraph.tag {
                add(&tag.style);
            }
            for piece in &paragraph.pieces {
                if let Piece::Text(_, style) = piece {
                    add(style);
                }
            }
        }
        Self(styles)
    }

    fn index(&self, style: &Character) -> usize {
        self.0
            .iter()
            .position(|known| known == style)
            .unwrap_or_default()
    }

    fn get(&self, index: usize) -> Option<&Character> {
        self.0.get(index)
    }
}

/// One face a run is set in, and the resource name the stream calls it by.
struct Held {
    name: Name,
    face: Face,
    /// Whether this program made the dictionary up, so that it has to reach the appearance's
    /// resources and may not fall short.
    invented: bool,
    /// Whether any glyph was set in it.
    used: std::cell::Cell<bool>,
}

/// What a search path is resolved from: the family list, bold, italic, and the width.
type StyleKey = (Vec<String>, bool, bool, u8);

/// The faces of one layout, and which of them a style's family list reaches.
///
/// # How a face is chosen
///
/// Chapter 27's `font-family` is a search path: a character is taken from the first face in the
/// list holding a glyph for it (page 1201). Each family in the list is resolved to a face, in this
/// order, with the run's weight and posture, and ADR 1634 section 2 is the argument:
///
/// 1. **The `/DA`'s own face**, where the family, the weight and the posture are its own — so a
///    string that restates the field's default style is drawn exactly as the plain value is.
/// 2. **A face Table 224's `/DR` holds**, matched by Table 120's `/FontFamily`, `/FontWeight`
///    and `/Flags` where the descriptor states them, and by `/BaseFont` otherwise — the document's
///    own faces before anything else, because they are what its producer had.
/// 3. **One of §9.6.2.2's fourteen**, for the three families they come in and the two faces
///    that have no family; this binary carries them, so those runs draw the same everywhere.
/// 4. **A stand-in named after the family and style**, `Family,Bold` in §9.6.3's spelling, which
///    `pdf_font`'s substitution places as it places any font a page names and does not embed.
///
/// The `/DA`'s own face is the last resort of every path, so a character no named face draws is
/// still drawn where the field's default face can.
struct Faces<'a> {
    document: &'a Document,
    resources: &'a Dictionary,
    held: Vec<Held>,
    /// The `/DA`'s face's family, weight, posture and width.
    base: (String, bool, bool, u8),
    base_resolution: Resolution,
    /// Each style's search path, by the key it is resolved from.
    paths: Vec<(StyleKey, Vec<usize>)>,
    /// The widths a run asked for and no face of its family was found in, by chapter 27's name.
    unmet_widths: std::collections::BTreeSet<&'static str>,
}

impl<'a> Faces<'a> {
    fn new(
        document: &'a Document,
        resources: &'a Dictionary,
        base_name: &Name,
    ) -> Result<Self, Owed> {
        let (dict, resolution) = variable_text::resolve_font(document, resources, base_name);
        let base = family_of(document, &dict);
        let face = Face::load(document, dict, base_name)?;
        Ok(Self {
            document,
            resources,
            held: vec![Held {
                name: base_name.clone(),
                face,
                invented: resolution != Resolution::Named,
                used: std::cell::Cell::new(false),
            }],
            base,
            base_resolution: resolution,
            paths: Vec::new(),
            unmet_widths: std::collections::BTreeSet::new(),
        })
    }

    /// The faces a style's characters are looked for in, nearest first, the `/DA`'s last.
    fn path(&mut self, style: &Character) -> Vec<usize> {
        let key = (
            style.families.clone(),
            style.bold(),
            style.italic,
            style.stretch,
        );
        if let Some((_, path)) = self.paths.iter().find(|(known, _)| *known == key) {
            return path.clone();
        }
        let families = if style.families.is_empty() {
            vec![self.base.0.clone()]
        } else {
            style.families.clone()
        };
        let mut path = Vec::new();
        for family in &families {
            if let Some(index) = self.resolve(family, style.bold(), style.italic, style.stretch)
                && !path.contains(&index)
            {
                path.push(index);
            }
        }
        if !path.contains(&0) {
            path.push(0);
        }
        self.paths.push((key, path.clone()));
        path
    }

    fn resolve(&mut self, family: &str, bold: bool, italic: bool, stretch: u8) -> Option<usize> {
        let wanted = normalised(family);
        if wanted == normalised(&self.base.0)
            && bold == self.base.1
            && italic == self.base.2
            && stretch == self.base.3
        {
            return Some(0);
        }
        // Table 224's `/DR`, the document's own faces.
        let fonts = self.document.get_key(self.resources, "Font");
        if let Some(fonts) = fonts.as_dict() {
            for (name, entry) in fonts.iter() {
                let Some(dict) = self.document.resolve(entry).as_dict().cloned() else {
                    continue;
                };
                let (stated, stated_bold, stated_italic, stated_stretch) =
                    family_of(self.document, &dict);
                if normalised(&stated) == wanted
                    && stated_bold == bold
                    && stated_italic == italic
                    && stated_stretch == stretch
                {
                    if let Some(index) = self.held.iter().position(|held| held.name == *name) {
                        return Some(index);
                    }
                    let face = Face::load(self.document, dict, name).ok()?;
                    return Some(self.hold(name.clone(), face, false));
                }
            }
        }
        // §9.6.2.2's fourteen come in one width, and §9.6.3's spelling of a stand-in states none:
        // a width no face of `/DR` was found in is drawn in the family's normal one, and said.
        if stretch != NORMAL_STRETCH {
            if let Some(name) = STRETCHES.get(usize::from(stretch)) {
                self.unmet_widths.insert(name);
            }
            return self.resolve(family, bold, italic, NORMAL_STRETCH);
        }
        let base_font = standard_face(&wanted, bold, italic)
            .map_or_else(|| stand_in_name(family, bold, italic), str::to_owned);
        if let Some(index) = self
            .held
            .iter()
            .position(|held| held.invented && base_font_of(&held.face.dict) == base_font)
        {
            return Some(index);
        }
        let name = self.fresh_name();
        let dict = variable_text::substituted_font(&Name::new(base_font.as_bytes()));
        let face = Face::load(self.document, dict, &name).ok()?;
        Some(self.hold(name, face, true))
    }

    fn hold(&mut self, name: Name, face: Face, invented: bool) -> usize {
        self.held.push(Held {
            name,
            face,
            invented,
            used: std::cell::Cell::new(false),
        });
        self.held.len().saturating_sub(1)
    }

    /// A resource name no `/DR` font and no face already held uses.
    fn fresh_name(&self) -> Name {
        let fonts = self.document.get_key(self.resources, "Font");
        let taken = |name: &Name| {
            fonts
                .as_dict()
                .is_some_and(|fonts| fonts.get_by_name(name).is_some())
                || self.held.iter().any(|held| held.name == *name)
        };
        let mut number = self.held.len();
        loop {
            let name = Name::new(format!("RichText{number}").into_bytes());
            if !taken(&name) {
                return name;
            }
            number = number.saturating_add(1);
        }
    }

    fn get(&self, index: usize) -> Option<&Held> {
        self.held.get(index)
    }

    fn invented_in_use(&self) -> bool {
        self.held
            .iter()
            .any(|held| held.invented && held.used.get())
    }

    /// The invented faces any glyph was set in, for the appearance's resources.
    fn invented(&self) -> Vec<(Name, Dictionary)> {
        self.held
            .iter()
            .filter(|held| held.invented && held.used.get())
            .map(|held| (held.name.clone(), held.face.dict.clone()))
            .collect()
    }
}

/// A family name as two spellings of it compare: folded, with white space, hyphens and the
/// `MT`, `PS` and `PSMT` a PostScript name carries after its family removed.
fn normalised(family: &str) -> String {
    let folded: String = family
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-')
        .flat_map(char::to_lowercase)
        .collect();
    for suffix in ["psmt", "mt", "ps"] {
        if let Some(stem) = folded.strip_suffix(suffix)
            && !stem.is_empty()
        {
            return stem.to_owned();
        }
    }
    folded
}

/// A font dictionary's family, whether it is bold and italic, and its width.
///
/// Table 120's `/FontFamily`, `/FontStretch` and `/FontWeight` (PDF 1.5, the version rich text
/// arrived in) and `/Flags` bit 7, Italic, where the descriptor states them; the `/BaseFont` read
/// the way §9.6.2.2's fourteen and §9.6.3's `,Bold`-suffixed names spell a style otherwise, with a
/// subset tag (§9.9.2's six letters and a plus) taken off first. A descriptor stating no
/// `/FontStretch` is taken as `Normal`, the width the fourteen and an unstyled run are set in.
fn family_of(document: &Document, dict: &Dictionary) -> (String, bool, bool, u8) {
    let base = base_font_of(dict);
    let base = match base.split_once('+') {
        Some((tag, rest)) if tag.len() == 6 && tag.chars().all(|c| c.is_ascii_uppercase()) => {
            rest.to_owned()
        }
        _ => base,
    };
    let (family, style) = base
        .split_once([',', '-'])
        .map_or((base.as_str(), ""), |(family, style)| (family, style));
    let lower = style.to_ascii_lowercase();
    let mut bold = lower.contains("bold");
    let mut italic = lower.contains("italic") || lower.contains("oblique");
    let mut family = match family {
        // `Times-Roman`'s family is Times: the fourteen's one name whose regular face is named.
        "Times" => "Times".to_owned(),
        other => other.to_owned(),
    };
    let mut stretch = NORMAL_STRETCH;
    let descriptor = variable_text::descriptor_of(document, dict);
    if let Some(descriptor) = descriptor.as_dict() {
        if let Some(stated) = document.get_key(descriptor, "FontFamily").as_string() {
            family = pdf_syntax::text_string(stated);
        }
        if let Some(weight) = document.get_key(descriptor, "FontWeight").as_number() {
            bold = weight > 500.0;
        }
        // Table 120: "It shall be one of these names (ordered from narrowest to widest):
        // UltraCondensed, ExtraCondensed, Condensed, SemiCondensed, Normal, SemiExpanded,
        // Expanded, ExtraExpanded or UltraExpanded".
        if let Some(name) = document.get_key(descriptor, "FontStretch").as_name() {
            let stated = String::from_utf8_lossy(name.as_bytes()).to_ascii_lowercase();
            if let Some(at) = STRETCHES
                .iter()
                .position(|width| width.replace('-', "") == stated)
            {
                stretch = u8::try_from(at).unwrap_or(NORMAL_STRETCH);
            }
        }
        if let Some(flags) = document.get_key(descriptor, "Flags").as_integer() {
            // Table 121's bit 7, Italic; bit 19, ForceBold, says the face is drawn bold.
            italic = italic || flags & (1 << 6) != 0;
            bold = bold || flags & (1 << 18) != 0;
        }
    }
    (family, bold, italic, stretch)
}

/// A font dictionary's `/BaseFont`, as text.
fn base_font_of(dict: &Dictionary) -> String {
    dict.get("BaseFont")
        .and_then(Object::as_name)
        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
        .unwrap_or_default()
}

/// One of §9.6.2.2's fourteen, for a family and style, where the family is one of theirs.
fn standard_face(family: &str, bold: bool, italic: bool) -> Option<&'static str> {
    let faces: [&'static str; 4] = match family {
        "helvetica" => [
            "Helvetica",
            "Helvetica-Bold",
            "Helvetica-Oblique",
            "Helvetica-BoldOblique",
        ],
        "times" | "timesroman" => [
            "Times-Roman",
            "Times-Bold",
            "Times-Italic",
            "Times-BoldItalic",
        ],
        "courier" => [
            "Courier",
            "Courier-Bold",
            "Courier-Oblique",
            "Courier-BoldOblique",
        ],
        "symbol" => ["Symbol"; 4],
        "zapfdingbats" => ["ZapfDingbats"; 4],
        _ => return None,
    };
    let index = usize::from(bold) | (usize::from(italic) << 1);
    faces.get(index).copied()
}

/// §9.6.3's spelling of a face by family and style: the family with its spaces removed, and
/// `,Bold`, `,Italic` or `,BoldItalic` after it.
fn stand_in_name(family: &str, bold: bool, italic: bool) -> String {
    let stem: String = family.chars().filter(|c| !c.is_whitespace()).collect();
    match (bold, italic) {
        (false, false) => stem,
        (true, false) => format!("{stem},Bold"),
        (false, true) => format!("{stem},Italic"),
        (true, true) => format!("{stem},BoldItalic"),
    }
}

/// One glyph: its face, its code, the style it is set in, and where in the string's characters
/// it came from.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Atom {
    face: usize,
    code: pdf_font::Code,
    space: bool,
    style: usize,
    /// The byte of [`RichText::text`] its character starts at; `None` for a list tag's, which
    /// chapter 27 generates rather than states.
    byte: Option<usize>,
    /// How many tab stops this advances by: an `xfa-tab-count`, which draws nothing and is as
    /// wide as the way to its stop (*Tab Stops*, page 1206). Zero for a glyph.
    tab: u16,
    /// Whether the character is the radix a decimal tab stop aligns: the full stop, a choice the
    /// chapter leaves to the implementation (page 1207).
    radix: bool,
}

impl Atom {
    /// An advance of `count` tab stops, standing at `byte` of the string's characters.
    fn tab(count: u16, style: usize, byte: usize) -> Self {
        Self {
            face: 0,
            code: pdf_font::Code::single_byte(0),
            space: false,
            style,
            byte: Some(byte),
            tab: count,
            radix: false,
        }
    }
}

impl Atom {
    fn placed(self) -> Placed {
        if self.space {
            Placed::Space(self.code)
        } else {
            Placed::Shown(self.code)
        }
    }
}

/// One thing a paragraph holds, in order.
#[derive(Debug, Clone, Copy)]
enum Item {
    Glyph(Atom),
    /// XHTML's `br`, at the byte of [`RichText::text`] it stands at; `consumes` where that text
    /// spells it as a carriage return, which a paragraph's closing break it does not.
    Break {
        at: usize,
        consumes: bool,
    },
}

/// A paragraph as glyphs.
struct Encoded {
    items: Vec<Item>,
    /// The bytes of [`RichText::text`] the paragraph's characters occupy.
    span: (usize, usize),
    block: super::style::Block,
    strut: usize,
    tag: Option<Vec<Atom>>,
    list: ListIndent,
}

/// Every paragraph as glyphs, and what could not be encoded.
struct Paragraphs {
    list: Vec<Encoded>,
    missing: String,
    /// The letters drawn as stored where their joined form or mirror image has no code.
    unformed: String,
    truncated: bool,
}

/// The string's characters, shaped once across every run.
///
/// Cursive joining is a property of the character sequence and not of its styling, so a word
/// set half in one colour joins across the boundary; and UAX #9's levels are the whole string's,
/// because a run's direction depends on its neighbours (ADR 1649).
struct Shaping<'a> {
    characters: Vec<(usize, char)>,
    shaped: Vec<Shaped>,
    levels: Option<&'a Levels<'a>>,
}

impl<'a> Shaping<'a> {
    fn of(text: &str, levels: Option<&'a Levels<'a>>) -> Self {
        let characters: Vec<(usize, char)> = text.char_indices().collect();
        let letters: Vec<char> = characters.iter().map(|(_, character)| *character).collect();
        Self {
            shaped: pdf_font::shaping::shape(&letters),
            characters,
            levels,
        }
    }
}

/// Turns each run into codes of the first face in its path that draws each character.
///
/// Walks the paragraphs in step with [`RichText::text`], so each glyph knows the byte of that
/// text its character came from — what a host's caret, point and range are answered in.
fn encode(request: &Request, styles: &Styles, faces: &mut Faces, shaping: &Shaping) -> Paragraphs {
    let mut out = Paragraphs {
        list: Vec::new(),
        missing: String::new(),
        unformed: String::new(),
        truncated: false,
    };
    let mut total = 0_usize;
    let mut cursor = 0_usize;
    let mut next = 0_usize;
    for (index, paragraph) in request.rich.paragraphs.iter().enumerate() {
        if index > 0 {
            cursor = cursor.saturating_add(1);
        }
        let begins = cursor;
        let mut items = Vec::new();
        let last = paragraph.pieces.len().saturating_sub(1);
        for (at, piece) in paragraph.pieces.iter().enumerate() {
            match piece {
                Piece::Tab(count) => items.push(Item::Glyph(Atom::tab(
                    *count,
                    styles.index(&paragraph.strut),
                    cursor,
                ))),
                Piece::Break => {
                    let consumes = at < last;
                    items.push(Item::Break {
                        at: cursor,
                        consumes,
                    });
                    cursor = cursor.saturating_add(usize::from(consumes));
                }
                Piece::Text(text, style) => {
                    let span = cursor..cursor.saturating_add(text.len());
                    cursor = span.end;
                    let index = styles.index(style);
                    while shaping
                        .shaped
                        .get(next)
                        .and_then(|item| shaping.characters.get(item.source))
                        .is_some_and(|(byte, _)| *byte < span.start)
                    {
                        next = next.saturating_add(1);
                    }
                    let from = next;
                    while shaping
                        .shaped
                        .get(next)
                        .and_then(|item| shaping.characters.get(item.source))
                        .is_some_and(|(byte, _)| *byte < span.end)
                    {
                        next = next.saturating_add(1);
                    }
                    let atoms = encode_items(
                        shaping.shaped.get(from..next).unwrap_or_default(),
                        &shaping.characters,
                        shaping.levels,
                        (style, index),
                        faces,
                        (&mut out.missing, &mut out.unformed),
                        true,
                    );
                    total = total.saturating_add(atoms.len());
                    if total > variable_text::MAX_CODES {
                        out.truncated = true;
                        break;
                    }
                    items.extend(atoms.into_iter().map(Item::Glyph));
                }
            }
        }
        let tag = paragraph.tag.as_ref().map(|tag| {
            let tag_shaping = Shaping::of(&tag.text, None);
            encode_items(
                &tag_shaping.shaped,
                &tag_shaping.characters,
                None,
                (&tag.style, styles.index(&tag.style)),
                faces,
                (&mut out.missing, &mut out.unformed),
                false,
            )
        });
        out.list.push(Encoded {
            items,
            span: (begins, cursor),
            block: paragraph.block.clone(),
            strut: styles.index(&paragraph.strut),
            tag,
            list: paragraph.list,
        });
        if out.truncated {
            break;
        }
    }
    out
}

/// Shaped characters of one run, each from the first face of its style's path that has a code
/// for the stored character, asked for the form the joining rules and rule L4 display.
///
/// The same choice [`variable_text::encode`] makes for a plain value, per character: the
/// displayed form where the face has it, the stored letter where it has only that — said in
/// `unformed` — and nothing where it has neither, said in `missing`. `bytes` is whether the
/// characters are the string's own, so each glyph carries the byte it came from.
fn encode_items(
    items: &[Shaped],
    characters: &[(usize, char)],
    levels: Option<&Levels>,
    (style, index): (&Character, usize),
    faces: &mut Faces,
    (missing, unformed): (&mut String, &mut String),
    bytes: bool,
) -> Vec<Atom> {
    let path = faces.path(style);
    let mut atoms = Vec::with_capacity(items.len());
    for item in items {
        let Some(&(at, original)) = characters.get(item.source) else {
            continue;
        };
        let face = path.iter().copied().find(|face| {
            faces
                .get(*face)
                .is_some_and(|held| held.face.code(original).is_some())
        });
        let Some((face, held)) = face.and_then(|face| faces.get(face).map(|held| (face, held)))
        else {
            if !missing.contains(original) {
                missing.push(original);
            }
            continue;
        };
        let drawn = pdf_font::shaping::displayed(item.character, at, levels);
        let lookup = |character: char| held.face.code(character);
        let Some(code) =
            variable_text::code_of(&lookup, (drawn, original), item.unformed, unformed)
        else {
            if !missing.contains(original) {
                missing.push(original);
            }
            continue;
        };
        held.used.set(true);
        atoms.push(Atom {
            face,
            code,
            space: original == ' ',
            style: index,
            byte: bytes.then_some(at),
            tab: 0,
            radix: original == '.',
        });
    }
    atoms
}

/// The box the text is laid out in, less the body's margins.
#[derive(Debug, Clone, Copy)]
struct Room {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    /// The body's own top and bottom margins, which the vertical placement counts as text.
    margin_top: Linear,
    margin_bottom: Linear,
    margin_left: Linear,
    margin_right: Linear,
    valign: Option<VerticalAlign>,
}

impl Room {
    fn of(box_: [f32; 4], rich: &RichText) -> Self {
        let [top, right, bottom, left] = rich.body.margins;
        Self {
            left: box_[0].min(box_[2]),
            right: box_[0].max(box_[2]),
            top: box_[1].max(box_[3]),
            bottom: box_[1].min(box_[3]),
            margin_top: top,
            margin_bottom: bottom,
            margin_left: left,
            margin_right: right,
            valign: rich
                .body
                .valign
                .or_else(|| rich.paragraphs.first().and_then(|first| first.block.valign)),
        }
    }

    fn height(self) -> f32 {
        (self.top - self.bottom).max(0.0)
    }
}

/// §9.4.4's displacement for one glyph, with the run's own size, spacing and scale.
struct Measure<'a> {
    faces: &'a Faces<'a>,
    styles: &'a Styles,
    appearance: &'a DefaultAppearance,
}

impl Measure<'_> {
    fn style(&self, index: usize) -> Option<&Character> {
        self.styles.get(index)
    }

    /// `Tc` for a run: the `/DA`'s, and the string's letter spacing beside it.
    fn character_spacing(&self, atom: Atom, root: f32) -> f32 {
        let Some(style) = self.style(atom.style) else {
            return self.appearance.character_spacing;
        };
        let size = size_at(style, root);
        let extra = match style.letter_spacing {
            Spacing::Length(length) => length.at(root),
            Spacing::OfSpace(share) => self
                .faces
                .get(atom.face)
                .and_then(|held| held.face.code(' ').map(|code| held.face.font.advance(code)))
                .map_or(0.0, |advance| advance * size * share),
        };
        self.appearance.character_spacing + extra
    }

    /// `Th` for a run: the `/DA`'s `Tz` and the string's horizontal glyph scale.
    fn horizontal_scale(&self, atom: Atom) -> f32 {
        self.appearance.horizontal_scaling
            * self
                .style(atom.style)
                .map_or(1.0, |style| style.horizontal_scale)
    }

    /// A glyph's advance; a tab's is its stop's, which [`tab_advance`] measures from where the
    /// tab stands, so it is nothing here.
    fn width(&self, atom: Atom, root: f32) -> f32 {
        if atom.tab > 0 {
            return 0.0;
        }
        let Some(style) = self.style(atom.style) else {
            return 0.0;
        };
        let Some(held) = self.faces.get(atom.face) else {
            return 0.0;
        };
        let word = if atom.code.takes_word_spacing() {
            self.appearance.word_spacing
        } else {
            0.0
        };
        (held
            .face
            .font
            .advance(atom.code)
            .mul_add(size_at(style, root), self.character_spacing(atom, root))
            + word)
            * self.horizontal_scale(atom)
    }

    fn widths(&self, atoms: &[Atom], root: f32) -> f32 {
        atoms.iter().map(|atom| self.width(*atom, root)).sum()
    }

    /// How far a glyph reaches above and below the line's baseline, and how tall its em is.
    fn extent(&self, atom: Atom, root: f32) -> (f32, f32, f32) {
        let Some(style) = self.style(atom.style) else {
            return (0.0, 0.0, 0.0);
        };
        let metrics = self.faces.get(atom.face).map_or(
            (
                variable_text::DEFAULT_ASCENT,
                variable_text::DEFAULT_DESCENT,
            ),
            |held| (held.face.metrics.ascent, held.face.metrics.descent),
        );
        let size = size_at(style, root) * style.vertical_scale;
        let rise = style.rise.at(root);
        (
            metrics.0.mul_add(size, rise),
            metrics.1.mul_add(size, rise),
            size,
        )
    }

    /// The extent an empty line takes, from its paragraph's own style and the first face it
    /// would be set in.
    fn strut(&self, style: usize, root: f32) -> (f32, f32, f32) {
        let Some(character) = self.style(style) else {
            return (0.0, 0.0, 0.0);
        };
        let size = size_at(character, root) * character.vertical_scale;
        let held = self.faces.get(0);
        let (ascent, descent) = held.map_or(
            (
                variable_text::DEFAULT_ASCENT,
                variable_text::DEFAULT_DESCENT,
            ),
            |held| (held.face.metrics.ascent, held.face.metrics.descent),
        );
        (ascent * size, descent * size, size)
    }
}

/// A run's size at a root size, never below zero.
fn size_at(style: &Character, root: f32) -> f32 {
    style.size.at(root).max(0.0)
}

/// One line, placed.
struct Line {
    /// The glyphs, in the order they are displayed: UAX #9's rule L2 over the logical order.
    atoms: Vec<Atom>,
    /// Which logical glyph each displayed one is, and back.
    order: Order,
    /// Where the line starts, after its alignment.
    x: f32,
    baseline: f32,
    /// How far the line reaches above and below its baseline, which is where a caret stands.
    ascent: f32,
    descent: f32,
    /// Extra space after each space on the line: a justified line's.
    gap: f32,
    tag: Option<(Vec<Atom>, f32)>,
    /// Where each displayed glyph is drawn from: Table 231 bit 25's cell centres, on a comb.
    cells: Option<Vec<f32>>,
    /// The boundaries between displayed glyphs, left to right, one more than there are glyphs:
    /// the advances the line is written with, or a comb's cell edges.
    edges: Vec<f32>,
    /// The bytes of [`RichText::text`] the line holds, a soft wrap's trailing spaces included.
    span: (usize, usize),
}

/// The whole text, placed.
struct Plan {
    lines: Vec<Line>,
    fit: Fit,
    /// Whether a line holding a right-to-left run holds a tab, which advances nothing there.
    tabs_reversed: bool,
    advance: f32,
}

/// Whether the text fits, on each of the axes Table 231 bit 24 asks about.
#[derive(Default)]
struct Fit {
    /// Whether some line is longer than the room it was given: a word longer than a line, or a
    /// single line's whole text.
    too_wide: bool,
    /// Whether the block is taller than the box.
    too_tall: bool,
    /// Whether a comb's characters outnumber its cells.
    too_many: bool,
}

impl Plan {
    /// Table 231 bit 24's question, on the axis the clause names for the shape: across the box
    /// for one line, down it for several, and in cells for a comb, as the plain layout asks it.
    fn overflows(&self, request: &Request) -> bool {
        if request.comb.is_some() {
            self.fit.too_many
        } else if request.multiline {
            self.fit.too_tall
        } else {
            self.fit.too_wide
        }
    }
}

/// A line before it is placed.
struct Broken {
    atoms: Vec<Atom>,
    /// The paragraph it belongs to.
    paragraph: usize,
    first: bool,
    last: bool,
    /// The bytes of [`RichText::text`] it holds.
    span: (usize, usize),
}

/// Breaks every paragraph into lines that fit its room.
fn break_lines(
    measure: &Measure,
    paragraphs: &[Encoded],
    root: f32,
    area: Room,
    multiline: bool,
) -> Vec<Broken> {
    let mut out = Vec::new();
    if !multiline {
        // Table 231's Multiline clear: "the field's text shall be restricted to a single line".
        // Paragraphs and breaks join, as the plain layout's breaks draw nothing on one line.
        let atoms: Vec<Atom> = paragraphs
            .iter()
            .flat_map(|paragraph| paragraph.items.iter())
            .filter_map(|item| match item {
                Item::Glyph(atom) => Some(*atom),
                Item::Break { .. } => None,
            })
            .collect();
        out.push(Broken {
            atoms,
            paragraph: 0,
            first: true,
            last: true,
            span: (
                paragraphs.first().map_or(0, |first| first.span.0),
                paragraphs.last().map_or(0, |last| last.span.1),
            ),
        });
        return out;
    }
    for (index, paragraph) in paragraphs.iter().enumerate() {
        let start_at = out.len();
        let (first_start, width) = horizontal(measure, paragraph, root, area, true);
        let (origin, rest_width) = horizontal(measure, paragraph, root, area, false);
        let mut from_margin = first_start - origin;
        let mut line: Vec<Atom> = Vec::new();
        let mut begins = paragraph.span.0;
        let mut reached = 0.0_f32;
        let mut last_space: Option<usize> = None;
        let mut limit = width;
        let push = |out: &mut Vec<Broken>, atoms: Vec<Atom>, span: (usize, usize)| {
            out.push(Broken {
                atoms,
                paragraph: index,
                first: false,
                last: false,
                span,
            });
        };
        for (at_item, item) in paragraph.items.iter().enumerate() {
            let atom = match item {
                Item::Glyph(atom) => atom,
                Item::Break { at, consumes } => {
                    push(&mut out, std::mem::take(&mut line), (begins, *at));
                    begins = at.saturating_add(usize::from(*consumes));
                    reached = 0.0;
                    last_space = None;
                    limit = rest_width;
                    from_margin = 0.0;
                    continue;
                }
            };
            reached += if atom.tab > 0 {
                tab_advance(
                    measure,
                    &paragraph.block,
                    (from_margin + reached, atom.tab),
                    &aligned_by(&paragraph.items, at_item),
                    root,
                )
            } else {
                measure.width(*atom, root)
            };
            line.push(*atom);
            if atom.space {
                last_space = Some(line.len());
            }
            if reached <= limit || line.len() <= 1 {
                continue;
            }
            // Past the edge: break after the last space if there was one, else before this
            // glyph — the plain layout's rule ([`variable_text`]'s `wrap`).
            let at = last_space
                .filter(|at| *at < line.len())
                .unwrap_or(line.len().saturating_sub(1));
            let carried = line.split_off(at);
            let wraps_at = carried.first().and_then(|atom| atom.byte).unwrap_or(begins);
            push(&mut out, std::mem::take(&mut line), (begins, wraps_at));
            begins = wraps_at;
            line = carried;
            from_margin = 0.0;
            reached = advances(measure, &paragraph.block, &line, root, Some(0.0))
                .last()
                .copied()
                .unwrap_or_default();
            last_space = None;
            limit = rest_width;
        }
        push(&mut out, line, (begins, paragraph.span.1.max(begins)));
        if let Some(first) = out.get_mut(start_at) {
            first.first = true;
        }
        if let Some(last) = out.last_mut() {
            last.last = true;
        }
    }
    out
}

/// The glyphs a tab at `at` aligns: those after it, up to the next tab or line break.
fn aligned_by(items: &[Item], at: usize) -> Vec<Atom> {
    items
        .get(at.saturating_add(1)..)
        .unwrap_or_default()
        .iter()
        .map_while(|item| match item {
            Item::Glyph(next) if next.tab == 0 => Some(*next),
            Item::Glyph(_) | Item::Break { .. } => None,
        })
        .collect()
}

/// Where a paragraph's line may start, and how long it may be: the box less the body's and the
/// paragraph's margins, a list's indent, and on its first line the paragraph's indent.
fn horizontal(
    measure: &Measure,
    paragraph: &Encoded,
    root: f32,
    area: Room,
    first: bool,
) -> (f32, f32) {
    let [_, right, _, left] = paragraph.block.margins;
    let list = match paragraph.list {
        ListIndent::None => 0.0,
        ListIndent::Levels(levels) => LIST_INDENT * f32::from(levels),
        ListIndent::Minimal => {
            paragraph
                .tag
                .as_ref()
                .map_or(0.0, |tag| measure.widths(tag, root))
                + TAG_GAP
        }
    };
    // *List Support*'s `text-indent` row: a list item's first-line indent is ignored, because
    // the tag stands where it would go (page 1214).
    let indent = if first && paragraph.tag.is_none() {
        paragraph.block.indent.at(root)
    } else {
        0.0
    };
    let start = area.left + area.margin_left.at(root) + left.at(root) + list + indent;
    let end = area.right - area.margin_right.at(root) - right.at(root);
    (start, (end - start).max(0.0))
}

/// The atoms of a line that are drawn and measured for its alignment: CSS2 section 16.6.1 keeps
/// no space at a line's end.
fn trimmed(atoms: &[Atom]) -> &[Atom] {
    let end = atoms
        .iter()
        .rposition(|atom| !atom.space)
        .map_or(0, |at| at.saturating_add(1));
    atoms.get(..end).unwrap_or_default()
}

/// Each line's baseline below the top of the box, and how tall the whole block is.
///
/// The lines run down from the top, each one its own ascent below the last one's depth plus the
/// leading — or a paragraph's `line-height` apart — with the paragraphs' margins between them;
/// the block's place in the box is the caller's.
fn stack(
    measure: &Measure,
    paragraphs: &[Encoded],
    broken: &[Broken],
    root: f32,
    area: Room,
) -> (Vec<f32>, f32) {
    // Vertical positions relative to the top of the room, first; the block's placement after.
    let mut baselines = Vec::with_capacity(broken.len());
    let mut y = -area.margin_top.at(root);
    let mut previous: Option<(f32, usize)> = None;
    let mut last_descent = 0.0_f32;
    for line in broken {
        let paragraph = paragraphs.get(line.paragraph);
        let (ascent, descent, tall) = line_extent(measure, line, paragraphs, root);
        let line_height = paragraph
            .and_then(|paragraph| paragraph.block.line_height)
            .map(|height| height.at(root));
        match previous {
            None => {
                let top_margin =
                    paragraph.map_or(0.0, |paragraph| paragraph.block.margins[0].at(root));
                y -= top_margin + ascent;
            }
            Some((previous_descent, previous_paragraph)) => {
                if line.first && previous_paragraph != line.paragraph {
                    // CSS2 section 8.3.1: adjoining vertical margins collapse to the larger.
                    let bottom = paragraphs
                        .get(previous_paragraph)
                        .map_or(0.0, |paragraph| paragraph.block.margins[2].at(root));
                    let top =
                        paragraph.map_or(0.0, |paragraph| paragraph.block.margins[0].at(root));
                    y -= bottom.max(top);
                }
                let step = line_height.unwrap_or_else(|| {
                    // *Line Spacing*'s default, the tallest thing on the line (page 1191): the
                    // plain layout's 13/12 of the size for a line of one size, made of the
                    // previous line's depth, this one's height and the leading between them.
                    let leading = (tall * variable_text::LINE_HEIGHT - (ascent - descent)).max(0.0);
                    -previous_descent + ascent + leading
                });
                y -= step;
            }
        }
        baselines.push(y);
        previous = Some((descent, line.paragraph));
        last_descent = descent;
    }
    let bottom_margin = broken
        .last()
        .and_then(|line| paragraphs.get(line.paragraph))
        .map_or(0.0, |paragraph| paragraph.block.margins[2].at(root));
    let total = -(baselines.last().copied().unwrap_or(0.0) + last_descent)
        + bottom_margin
        + area.margin_bottom.at(root);
    (baselines, total)
}

/// Places every line.
fn plan(
    measure: &Measure,
    paragraphs: &[Encoded],
    root: f32,
    area: Room,
    request: &Request,
    levels: Option<&Levels>,
) -> Plan {
    let multiline = request.multiline && request.comb.is_none();
    let broken = break_lines(measure, paragraphs, root, area, multiline);
    let (baselines, total) = stack(measure, paragraphs, &broken, root, area);
    let valign = area.valign.unwrap_or(if multiline {
        VerticalAlign::Top
    } else {
        VerticalAlign::Middle
    });
    let offset = match valign {
        VerticalAlign::Top => area.top,
        VerticalAlign::Middle => area.top - (area.height() - total) * 0.5,
        VerticalAlign::Bottom => area.top - (area.height() - total),
    };

    let mut lines = Vec::with_capacity(broken.len());
    let mut advance = 0.0_f32;
    let mut fit = Fit {
        too_tall: total > area.height(),
        ..Fit::default()
    };
    let mut tabs_reversed = false;
    for (line, baseline) in broken.into_iter().zip(baselines) {
        let Some(paragraph) = paragraphs.get(line.paragraph) else {
            continue;
        };
        let (ascent, descent, _) = line_extent(measure, &line, paragraphs, root);
        let (start, width) = horizontal(measure, paragraph, root, area, line.first);
        let (origin, _) = horizontal(measure, paragraph, root, area, false);
        let (order, shown, reversed) = displayed(trimmed(&line.atoms), line.span, levels);
        // Stops are measured from the left margin, which is where a line read left to right
        // starts; a line holding a right-to-left run would measure them from its other side, and
        // this tree does not, so its tabs advance nothing and the report says so.
        if reversed && shown.iter().any(|atom| atom.tab > 0) {
            tabs_reversed = true;
        }
        let shown = shown.as_slice();
        if let Some(cells) = request.comb {
            let (placed, many) = comb_line(measure, shown, root, (start, width), cells, request);
            // Auto-sizing a comb asks the plain layout's question, the whole advance against the
            // box; whether it overflows is asked in cells.
            fit.too_wide |= measure.widths(shown, root) > width;
            fit.too_many |= many;
            advance = advance.max(placed.edges.last().copied().unwrap_or(start) - start);
            lines.push(Line {
                atoms: shown.to_vec(),
                order,
                x: start,
                baseline: offset + baseline,
                ascent,
                descent,
                gap: 0.0,
                tag: None,
                cells: Some(placed.cells),
                edges: placed.edges,
                span: line.span,
            });
            continue;
        }
        let (x, gap, edges) = aligned(
            measure,
            (paragraph, &line),
            shown,
            root,
            (start, width, (!reversed).then_some(start - origin)),
            request,
        );
        let used = edges.last().copied().unwrap_or(x)
            - x
            - gap * variable_text::count(shown.iter().filter(|atom| atom.space).count());
        advance = advance.max(used);
        if used > width {
            fit.too_wide = true;
        }
        let tag = if line.first {
            paragraph.tag.as_ref().map(|tag| {
                let tag_width = measure.widths(tag, root);
                (tag.clone(), start - TAG_GAP - tag_width)
            })
        } else {
            None
        };
        lines.push(Line {
            atoms: shown.to_vec(),
            order,
            x,
            baseline: offset + baseline,
            ascent,
            descent,
            gap,
            tag,
            cells: None,
            edges,
            span: line.span,
        });
    }
    Plan {
        lines,
        fit,
        tabs_reversed,
        advance,
    }
}

/// Where a line starts after its alignment, the extra room after each space a justified line
/// takes, and the boundaries between its displayed glyphs.
fn aligned(
    measure: &Measure,
    (paragraph, line): (&Encoded, &Broken),
    shown: &[Atom],
    root: f32,
    (start, width, from_margin): (f32, f32, Option<f32>),
    request: &Request,
) -> (f32, f32, Vec<f32>) {
    let advanced = advances(measure, &paragraph.block, shown, root, from_margin);
    let used = advanced.last().copied().unwrap_or_default();
    let align = paragraph.block.align.unwrap_or(match request.quadding {
        Quadding::Left => Align::Left,
        Quadding::Centred => Align::Centre,
        Quadding::Right => Align::Right,
    });
    let spaces = shown.iter().filter(|atom| atom.space).count();
    let justified = match align {
        Align::Justify => !line.last,
        Align::JustifyAll => true,
        Align::Left | Align::Centre | Align::Right => false,
    } && spaces > 0
        && used < width;
    let (x, gap) = match align {
        _ if justified => (start, (width - used) / variable_text::count(spaces)),
        Align::Centre => (start + (width - used) * 0.5, 0.0),
        Align::Right => (start + width - used, 0.0),
        Align::Left | Align::Justify | Align::JustifyAll => (start, 0.0),
    };
    let mut edges = Vec::with_capacity(shown.len().saturating_add(1));
    let mut extra = 0.0_f32;
    edges.push(x);
    for (atom, at) in shown.iter().zip(advanced.iter().skip(1)) {
        if atom.space {
            extra += gap;
        }
        edges.push(x + at + extra);
    }
    (x, gap, edges)
}

/// The running advance at each boundary of a line's glyphs, from zero, a tab as wide as the way
/// to its stop. `from_margin` is how far the line starts from the paragraph's left margin, which
/// is where *Tab Stops* measures every stop from (page 1207); `None` where the stops are not
/// measured on this line, whose tabs then advance nothing.
fn advances(
    measure: &Measure,
    block: &super::style::Block,
    atoms: &[Atom],
    root: f32,
    from_margin: Option<f32>,
) -> Vec<f32> {
    let mut out = Vec::with_capacity(atoms.len().saturating_add(1));
    let mut at = 0.0_f32;
    out.push(at);
    for (index, atom) in atoms.iter().enumerate() {
        at += match from_margin {
            Some(from_margin) if atom.tab > 0 => {
                let group: Vec<Atom> = atoms
                    .get(index.saturating_add(1)..)
                    .unwrap_or_default()
                    .iter()
                    .take_while(|next| next.tab == 0)
                    .copied()
                    .collect();
                tab_advance(measure, block, (from_margin + at, atom.tab), &group, root)
            }
            _ => measure.width(*atom, root),
        };
        out.push(at);
    }
    out
}

/// How far a tab at `position` from the left margin advances, by `count` stops, before `group` —
/// the text it aligns, up to the next tab or the line's end.
///
/// *Tab Stops* (pages 1205 to 1207): the stated stops first, then the default ones at every
/// multiple of `tab-interval` beyond the last stated; each tab moves to the next stop past the
/// cursor, and the last one reached aligns the group — its left edge at a left stop, its right
/// edge at a right one, its middle at a centred one, and its first radix at a decimal one, or its
/// right edge where it has none. A stop the group would have to start behind the cursor to meet
/// is met as near as the cursor allows, and no stop past the cursor is no advance: the chapter
/// sets none where neither property states one.
fn tab_advance(
    measure: &Measure,
    block: &super::style::Block,
    (position, count): (f32, u16),
    group: &[Atom],
    root: f32,
) -> f32 {
    use super::style::TabAlign;
    let stated: Vec<(TabAlign, f32)> = block
        .tab_stops
        .iter()
        .map(|(align, at)| (*align, at.at(root)))
        .collect();
    let interval = block
        .tab_interval
        .map(|interval| interval.at(root))
        .filter(|interval| *interval > 0.0);
    let last_stated = stated.iter().map(|(_, at)| *at).fold(0.0_f32, f32::max);
    let mut cursor = position;
    let mut reached: Option<(TabAlign, f32)> = None;
    for _ in 0..count {
        let next = stated
            .iter()
            .copied()
            .find(|(_, at)| *at > cursor + f32::EPSILON)
            .or_else(|| {
                let interval = interval?;
                let from = cursor.max(last_stated);
                let steps = (from / interval).floor() + 1.0;
                Some((TabAlign::Left, steps * interval))
            });
        let Some(stop) = next else {
            break;
        };
        cursor = stop.1;
        reached = Some(stop);
    }
    let Some((align, stop)) = reached else {
        return 0.0;
    };
    let whole = measure.widths(group, root);
    let lead = match align {
        TabAlign::Left => 0.0,
        TabAlign::Centre => whole * 0.5,
        TabAlign::Right => whole,
        TabAlign::Decimal => group
            .iter()
            .position(|atom| atom.radix)
            .map_or(whole, |radix| {
                measure.widths(group.get(..radix).unwrap_or_default(), root)
            }),
    };
    (stop - lead - position).max(0.0)
}

/// A line's glyphs in display order, the order itself, and whether any reads right to left.
fn displayed(
    logical: &[Atom],
    span: (usize, usize),
    levels: Option<&Levels>,
) -> (Order, Vec<Atom>, bool) {
    let levels = line_levels(logical, span, levels);
    let reversed = levels.iter().any(|level| level % 2 == 1);
    let order = Order::from_levels(&levels);
    let shown = order
        .visual
        .iter()
        .filter_map(|index| logical.get(*index).copied())
        .collect();
    (order, shown, reversed)
}

/// Each glyph's level, by UAX #9 over the whole string with rule L1 applied at this line's ends;
/// every level 0 where the string has no right-to-left character.
fn line_levels(atoms: &[Atom], span: (usize, usize), levels: Option<&Levels>) -> Vec<u8> {
    let Some(levels) = levels else {
        return vec![0; atoms.len()];
    };
    let by_byte = levels.line_levels(span.0..span.1.max(span.0));
    atoms
        .iter()
        .map(|atom| {
            atom.byte
                .and_then(|byte| by_byte.get(byte.saturating_sub(span.0)).copied())
                .unwrap_or_default()
        })
        .collect()
}

/// A comb's line: where each displayed glyph is drawn, and the cell edges between them.
struct CombLine {
    cells: Vec<f32>,
    edges: Vec<f32>,
}

/// Table 231 bit 25: the box divided into `cells` equal positions and one character in each,
/// left to right in display order, each glyph centred in its cell at its own size — the plain
/// comb's placement (`variable_text`'s `comb`), run by run. The second answer is whether the
/// characters outnumber the cells.
fn comb_line(
    measure: &Measure,
    shown: &[Atom],
    root: f32,
    (start, width): (f32, f32),
    cells: u32,
    request: &Request,
) -> (CombLine, bool) {
    let count = variable_text::count(usize::try_from(cells).unwrap_or(usize::MAX)).max(1.0);
    let used = variable_text::count(shown.len());
    let first = match request.quadding {
        Quadding::Left => 0.0,
        Quadding::Centred => ((count - used) * 0.5).max(0.0).floor(),
        Quadding::Right => (count - used).max(0.0),
    };
    let cell = width / count;
    let edge = |slot: f32| cell.mul_add(slot.min(count), start);
    let mut placed = CombLine {
        cells: Vec::with_capacity(shown.len()),
        edges: vec![edge(first)],
    };
    for (index, atom) in shown.iter().enumerate() {
        let slot = first + variable_text::count(index);
        let advance = measure.width(*atom, root);
        placed
            .cells
            .push(cell.mul_add(slot, (cell - advance) * 0.5) + start);
        placed.edges.push(edge(slot + 1.0));
    }
    (placed, used > count)
}

/// A line's ascent, descent and tallest em, from its glyphs or, for an empty line, its
/// paragraph's own style.
fn line_extent(
    measure: &Measure,
    line: &Broken,
    paragraphs: &[Encoded],
    root: f32,
) -> (f32, f32, f32) {
    let mut extent: Option<(f32, f32, f32)> = None;
    let tag = if line.first {
        paragraphs
            .get(line.paragraph)
            .and_then(|paragraph| paragraph.tag.as_deref())
            .unwrap_or_default()
    } else {
        &[]
    };
    for atom in line.atoms.iter().chain(tag) {
        let (ascent, descent, tall) = measure.extent(*atom, root);
        extent = Some(extent.map_or((ascent, descent, tall), |(a, d, t)| {
            (a.max(ascent), d.min(descent), t.max(tall))
        }));
    }
    extent.unwrap_or_else(|| {
        paragraphs
            .get(line.paragraph)
            .map_or((0.0, 0.0, 0.0), |paragraph| {
                measure.strut(paragraph.strut, root)
            })
    })
}

/// §12.7.4.3's auto-size over the whole string: the largest root size at which every line fits
/// its room and the block fits the box, found by the same bounded halving the plain layout uses.
fn auto_size(measure: &Measure, paragraphs: &[Encoded], area: Room, request: &Request) -> f32 {
    let fits = |root: f32| {
        // The order a line is displayed in moves nothing that fits or does not, so the levels
        // are not asked for here.
        let planned = plan(measure, paragraphs, root, area, request, None);
        !planned.fit.too_wide && !planned.fit.too_tall
    };
    let mut low = variable_text::MIN_AUTO_SIZE;
    let mut high = area.height().max(variable_text::MIN_AUTO_SIZE);
    if fits(high) {
        return high;
    }
    for _ in 0..variable_text::AUTO_SIZE_STEPS {
        let middle = (low + high) * 0.5;
        if fits(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    low
}

/// What the stream has set so far, so a run states only what changes.
#[derive(Default)]
struct State {
    font: Option<(usize, u32)>,
    colour: Option<Ink>,
    spacing: Option<u32>,
    scale: Option<u32>,
}

/// A run's colour as the stream sets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ink {
    /// The `/DA`'s own, by replaying its operators.
    Default,
    /// An sRGB colour, as the three bytes `#rrggbb` states.
    Rgb([u8; 3]),
}

/// A number as the stream writes it.
fn bits(value: f32) -> u32 {
    value.to_bits()
}

/// A colour as the stream compares it: three bytes, which is what `#rrggbb` states.
fn colour_key(colour: Option<[f32; 3]>) -> Ink {
    colour.map_or(Ink::Default, |components| {
        Ink::Rgb(components.map(|component| {
            let scaled = (component.clamp(0.0, 1.0) * 255.0).round();
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "clamped to 0..=255 and rounded the line above"
            )]
            {
                scaled as u8
            }
        }))
    })
}

/// Writes one line: its tag, then its glyphs grouped by face and style, and its decorations into
/// a second stream that follows the text object.
fn write_line(
    stream: &mut String,
    decorations: &mut String,
    state: &mut State,
    measure: &Measure,
    line: &Line,
    root: f32,
) {
    if let Some((atoms, x)) = &line.tag {
        write_group(
            stream,
            decorations,
            state,
            measure,
            atoms,
            *x,
            line.baseline,
            root,
        );
    }
    // Table 231 bit 25: a glyph to a cell, each written where its cell centres it.
    if let Some(cells) = &line.cells {
        for (atom, x) in line.atoms.iter().zip(cells) {
            write_group(
                stream,
                decorations,
                state,
                measure,
                std::slice::from_ref(atom),
                *x,
                line.baseline,
                root,
            );
        }
        return;
    }
    let mut start = 0_usize;
    while start < line.atoms.len() {
        let Some(first) = line.atoms.get(start) else {
            break;
        };
        // A tab draws nothing: the boundary after it is where the next group starts.
        if first.tab > 0 {
            start = start.saturating_add(1);
            continue;
        }
        // A group is the longest run of one face and one style, and on a justified line it ends
        // after each space, where the extra room is inserted; a tab ends it too.
        let mut end = start.saturating_add(1);
        if !(line.gap > 0.0 && first.space) {
            while let Some(next) = line.atoms.get(end) {
                if next.face != first.face || next.style != first.style || next.tab > 0 {
                    break;
                }
                end = end.saturating_add(1);
                if line.gap > 0.0 && next.space {
                    break;
                }
            }
        }
        let group = line.atoms.get(start..end).unwrap_or_default();
        // Each group is written at the boundary the line was placed with, so the stream and a
        // host's answers read one set of positions.
        let x = line.edges.get(start).copied().unwrap_or(line.x);
        write_group(
            stream,
            decorations,
            state,
            measure,
            group,
            x,
            line.baseline,
            root,
        );
        start = end;
    }
}

/// Writes one group of glyphs of one face and one style at a position.
#[expect(
    clippy::too_many_arguments,
    reason = "the two streams, the state between groups, and the group's place are one call's \
              inputs; bundling them would name a struct used once"
)]
fn write_group(
    stream: &mut String,
    decorations: &mut String,
    state: &mut State,
    measure: &Measure,
    group: &[Atom],
    x: f32,
    baseline: f32,
    root: f32,
) {
    let Some(first) = group.first().copied() else {
        return;
    };
    let (Some(style), Some(held)) = (measure.style(first.style), measure.faces.get(first.face))
    else {
        return;
    };
    let size = size_at(style, root);
    if state.font != Some((first.face, bits(size))) {
        let _ = writeln!(stream, "/{} {size} Tf", held.name.escaped());
        state.font = Some((first.face, bits(size)));
    }
    let colour = colour_key(style.colour);
    if state.colour != Some(colour) {
        if let Some([r, g, b]) = style.colour {
            let _ = writeln!(stream, "{r} {g} {b} rg");
        } else {
            // The `/DA`'s own colour, by replaying its operators: they are what set it.
            stream.push_str(&measure.appearance.operators);
            state.spacing = None;
            state.scale = None;
        }
        state.colour = Some(colour);
    }
    let spacing = measure.character_spacing(first, root);
    if state.spacing != Some(bits(spacing)) {
        let _ = writeln!(stream, "{spacing} Tc");
        state.spacing = Some(bits(spacing));
    }
    let scale = measure.horizontal_scale(first) * 100.0;
    if state.scale != Some(bits(scale)) {
        let _ = writeln!(stream, "{scale} Tz");
        state.scale = Some(bits(scale));
    }
    let rise = style.rise.at(root);
    let _ = writeln!(
        stream,
        "1 0 0 {} {x} {} Tm",
        style.vertical_scale,
        baseline + rise
    );
    let placed: Vec<Placed> = group.iter().map(|atom| atom.placed()).collect();
    variable_text::show(stream, &placed);

    // Chapter 27's underline and line through, in the run's own colour (page 1208).
    let underline = style.underline;
    if underline == Underline::None && !style.line_through {
        return;
    }
    let thickness = DECORATION_THICKNESS * size;
    let mut spans: Vec<(f32, f32)> = Vec::new();
    let mut at = x;
    let mut open: Option<f32> = None;
    for atom in group {
        let width = measure.width(*atom, root);
        if atom.space && underline.by_word() {
            if let Some(from) = open.take() {
                spans.push((from, at));
            }
        } else if open.is_none() {
            open = Some(at);
        }
        at += width;
    }
    if let Some(from) = open {
        spans.push((from, at));
    }
    let whole = (x, at);
    decorations.push_str("q\n");
    if let Some([r, g, b]) = style.colour {
        let _ = writeln!(decorations, "{r} {g} {b} rg");
    } else {
        decorations.push_str(&measure.appearance.operators);
    }
    let base = baseline + rise;
    for line in 0..underline.lines() {
        let depth = UNDERLINE_BELOW + SECOND_UNDERLINE_BELOW * f32::from(line);
        let y = depth.mul_add(-size, base) - thickness;
        let parts = if underline.by_word() {
            spans.clone()
        } else {
            vec![whole]
        };
        for (from, to) in parts {
            let _ = writeln!(decorations, "{from} {y} {} {thickness} re", to - from);
        }
    }
    if style.line_through {
        let y = LINE_THROUGH_ABOVE.mul_add(size, base);
        let _ = writeln!(
            decorations,
            "{} {y} {} {thickness} re",
            whole.0,
            whole.1 - whole.0
        );
    }
    decorations.push_str("f\nQ\n");
}

/// The style of text nothing styles: the `/DA`'s face, in that face's own weight and posture.
///
/// A rich text string that states no `font-weight` sets its text in the face the field's `/DA`
/// names, bold where that face is bold — so the root of the cascade is read from the face, and a
/// span asking for `font-weight:normal` under a bold `/DA` reaches the regular one.
pub(crate) fn root_style(
    document: &Document,
    resources: &Dictionary,
    default_appearance: &[u8],
) -> Character {
    let mut root = Character::root();
    if let Some(name) = DefaultAppearance::parse(default_appearance).font {
        let (dict, _) = variable_text::resolve_font(document, resources, &name);
        let (_, bold, italic, stretch) = family_of(document, &dict);
        root.weight = if bold { 700 } else { 400 };
        root.italic = italic;
        root.stretch = stretch;
    }
    root
}

/// A rich text string's first style as a `/DA` the one-style layout reads, with the face it
/// names where this program chose it.
pub(crate) struct OneStyle {
    /// The `/DA`'s own operators, then the face, size and colour of the string's first run.
    pub(crate) default_appearance: Vec<u8>,
    /// The face, where it is not one `/DR` holds under that name.
    pub(crate) font: Option<(Name, Dictionary)>,
    /// The first paragraph's alignment, where the string states one.
    pub(crate) quadding: Option<Quadding>,
}

/// The one-style form of a string: what a string none of whose run faces draws a character, in a
/// face this program chose, is laid out with — the plain layout reaches a machine face for it
/// (ADR 1414) and the runs' faces do not.
///
/// # Errors
///
/// What [`lay_out`] refuses for the `/DA` itself.
pub(crate) fn one_style(document: &Document, request: &Request) -> Result<OneStyle, Owed> {
    let appearance = DefaultAppearance::parse(request.default_appearance);
    let Some(base_name) = appearance.font.clone() else {
        return Err(Owed::NoFont);
    };
    let mut faces = Faces::new(document, request.resources, &base_name)?;
    let first = request.rich.paragraphs.first();
    let style = first
        .and_then(|paragraph| {
            paragraph.pieces.iter().find_map(|piece| match piece {
                Piece::Text(_, style) => Some(style.clone()),
                Piece::Break | Piece::Tab(_) => None,
            })
        })
        .or_else(|| first.map(|paragraph| paragraph.strut.clone()))
        .unwrap_or_else(Character::root);
    let index = faces.path(&style).first().copied().unwrap_or_default();
    let mut default_appearance = request.default_appearance.to_vec();
    let mut font = None;
    if let Some(held) = faces.get(index) {
        let auto = appearance.size.is_none_or(|size| size <= 0.0);
        let size = if auto && style.size.points.abs() < f32::EPSILON {
            0.0
        } else {
            style.size.at(appearance.size.unwrap_or_default()).max(0.0)
        };
        let _ = write!(
            ByteWriter(&mut default_appearance),
            " /{} {size} Tf",
            held.name.escaped()
        );
        if let Some([r, g, b]) = style.colour {
            let _ = write!(ByteWriter(&mut default_appearance), " {r} {g} {b} rg");
        }
        if index != 0 && held.invented {
            font = Some((held.name.clone(), held.face.dict.clone()));
        }
    }
    let quadding = first
        .and_then(|paragraph| paragraph.block.align)
        .map(|align| match align {
            Align::Centre => Quadding::Centred,
            Align::Right => Quadding::Right,
            Align::Left | Align::Justify | Align::JustifyAll => Quadding::Left,
        });
    Ok(OneStyle {
        default_appearance,
        font,
        quadding,
    })
}

/// What a host asked, answered from the lines as they were placed.
#[derive(Default)]
struct Answers {
    caret: Option<Caret>,
    offset: Option<usize>,
    selection: Vec<[f32; 8]>,
    glyphs: Vec<Glyph>,
}

/// How the bytes of a rich text string's characters ([`RichText::text`]) meet the bytes of the
/// value a host edits.
///
/// The two state the same words — ADR 1635 draws a rich text string only where they do — and
/// differ only in white space: chapter 27 compresses a run of it to one space and keeps none at a
/// paragraph's ends (page 1194), and a paragraph is one carriage return where the value may spell
/// a line end as two characters. So the characters that are not white space correspond one to
/// one, in order, and a run of white space in one is the run between the same two characters in
/// the other. Where the two are one text, as for a value a person is typing, this is the identity.
struct Alignment {
    /// For each byte of the characters, and their end, the value's byte.
    to_value: Vec<usize>,
    /// For each byte of the value, and its end, the characters' byte.
    to_rich: Vec<usize>,
}

impl Alignment {
    fn of(rich: &str, value: &str) -> Self {
        let mut to_value = vec![value.len(); rich.len().saturating_add(1)];
        let mut to_rich = vec![rich.len(); value.len().saturating_add(1)];
        let letters: Vec<(usize, char)> = rich.char_indices().collect();
        let stated: Vec<(usize, char)> = value.char_indices().collect();
        let byte = |at: usize| stated.get(at).map_or(value.len(), |(byte, _)| *byte);
        let white = |at: usize| {
            stated
                .get(at)
                .is_some_and(|(_, letter)| letter.is_whitespace())
        };
        let fill = |map: &mut Vec<usize>, range: std::ops::Range<usize>, to: usize| {
            for slot in map.get_mut(range).unwrap_or_default() {
                *slot = to;
            }
        };
        let mut next = 0_usize;
        for (index, &(at, letter)) in letters.iter().enumerate() {
            let ends = at.saturating_add(letter.len_utf8());
            let from = byte(next);
            if letter.is_whitespace() {
                fill(&mut to_value, at..ends, from);
                let more = letters
                    .get(index.saturating_add(1))
                    .is_some_and(|(_, after)| after.is_whitespace());
                if more {
                    // One of several the string keeps: one of the value's, a CR LF being one.
                    let pair = stated.get(next).is_some_and(|(_, first)| *first == '\r')
                        && stated
                            .get(next.saturating_add(1))
                            .is_some_and(|(_, second)| *second == '\n');
                    if white(next) {
                        next = next.saturating_add(if pair { 2 } else { 1 });
                    }
                } else {
                    while white(next) {
                        next = next.saturating_add(1);
                    }
                }
                fill(&mut to_rich, from..byte(next), at);
            } else {
                while white(next) {
                    next = next.saturating_add(1);
                }
                fill(&mut to_rich, from..byte(next), at);
                let here = byte(next);
                fill(&mut to_value, at..ends, here);
                if next < stated.len() {
                    next = next.saturating_add(1);
                    fill(&mut to_rich, here..byte(next), at);
                }
            }
        }
        let rest = byte(next);
        fill(
            &mut to_rich,
            rest..value.len().saturating_add(1),
            rich.len(),
        );
        Self { to_value, to_rich }
    }

    /// The value's byte for a byte of the characters.
    fn value(&self, rich: usize) -> usize {
        self.to_value
            .get(rich)
            .or_else(|| self.to_value.last())
            .copied()
            .unwrap_or_default()
    }

    /// The characters' byte for a byte of the value.
    fn rich(&self, value: usize) -> usize {
        self.to_rich
            .get(value)
            .or_else(|| self.to_rich.last())
            .copied()
            .unwrap_or_default()
    }
}

/// The caret, the point, the range and the glyphs a host asked about, from the placed lines.
///
/// **None of this is ISO 32000-2's**, as none of the plain layout's answers is (ADRs 0211, 0225);
/// what makes them right is that they are read off the lines the stream was written from — the
/// same edges, the same baselines, the same order — so a caret stands where the next glyph of
/// *this* layout goes, in its own run's face and size. The conventions are the plain layout's:
/// a caret on the first line whose end the offset reaches, between the line's descent and ascent;
/// a point nearest a line vertically and then a boundary along it; a range as one box per run of
/// the line it covers; and on a comb, cells in place of advances (ADR 1649).
fn answer(plan: &Plan, request: &Request, alignment: &Alignment) -> Answers {
    let asked = request.asked;
    // Each line's glyph bytes in logical order, so a boundary is a count of glyphs before a byte.
    // A tab stands at the byte of the run it advances to and counts as before it, so a caret at
    // that run's first byte stands past the tab, where the run is drawn.
    let logical: Vec<Vec<Stood>> = plan
        .lines
        .iter()
        .map(|line| {
            let mut bytes = vec![
                Stood {
                    byte: line.span.0,
                    tab: false
                };
                line.atoms.len()
            ];
            for (shown_at, index) in line.order.visual.iter().enumerate() {
                if let (Some(slot), Some(atom)) = (bytes.get_mut(*index), line.atoms.get(shown_at))
                {
                    *slot = Stood {
                        byte: atom.byte.unwrap_or(line.span.0),
                        tab: atom.tab > 0,
                    };
                }
            }
            bytes
        })
        .collect();
    let lines: Vec<(&Line, &[Stood])> = plan
        .lines
        .iter()
        .zip(logical.iter().map(Vec::as_slice))
        .collect();
    let mut out = Answers {
        caret: asked
            .caret
            .and_then(|caret| caret_at(&lines, alignment.rich(caret))),
        offset: asked
            .point
            .and_then(|point| byte_at(&lines, point))
            .map(|byte| alignment.value(byte)),
        ..Answers::default()
    };
    if let Some((from, to)) = asked.selection {
        let (from, to) = (alignment.rich(from), alignment.rich(to));
        for (line, bytes) in &lines {
            let (bottom, top) = (line.baseline + line.descent, line.baseline + line.ascent);
            for (first, last) in line.order.runs(before(bytes, from)..before(bytes, to)) {
                let (x0, x1) = (edge(line, first), edge(line, last));
                if x1 > x0 {
                    out.selection
                        .push([x0, top, x1, top, x1, bottom, x0, bottom]);
                }
            }
        }
    }
    if asked.glyphs {
        for (index, (line, bytes)) in lines.iter().enumerate() {
            let (bottom, top) = (line.baseline + line.descent, line.baseline + line.ascent);
            for (shown_at, at) in line.order.visual.iter().enumerate() {
                if line.atoms.get(shown_at).is_some_and(|atom| atom.tab > 0) {
                    continue;
                }
                let start = bytes.get(*at).map_or(line.span.0, |stood| stood.byte);
                let end = bytes
                    .get(at.saturating_add(1))
                    .map_or(line.span.1, |stood| stood.byte);
                let (x0, x1) = (edge(line, shown_at), edge(line, shown_at.saturating_add(1)));
                out.glyphs.push(Glyph {
                    line: index,
                    bytes: alignment.value(start)..alignment.value(end),
                    quad: [x0, top, x1, top, x1, bottom, x0, bottom],
                });
            }
        }
    }
    out
}

/// Where in the string's characters a placed glyph or tab stands.
#[derive(Debug, Clone, Copy)]
struct Stood {
    byte: usize,
    /// A tab stands at the byte of the run it advances to and counts as before it, so a caret at
    /// that run's first byte stands past the tab, where the run is drawn.
    tab: bool,
}

/// How many of a line's glyphs and tabs, in logical order, stand before a byte.
fn before(bytes: &[Stood], at: usize) -> usize {
    bytes
        .iter()
        .filter(|stood| stood.byte < at || (stood.tab && stood.byte <= at))
        .count()
}

/// The `shown`-th boundary of a line from its left, clamped to its last.
fn edge(line: &Line, shown: usize) -> f32 {
    line.edges
        .get(shown)
        .or_else(|| line.edges.last())
        .copied()
        .unwrap_or(line.x)
}

/// Where a caret at a byte of the characters stands: on the first line whose end it reaches.
fn caret_at(lines: &[(&Line, &[Stood])], at: usize) -> Option<Caret> {
    let found = lines
        .iter()
        .find(|(line, _)| line.span.0 <= at && at <= line.span.1)
        .or_else(|| lines.last())?;
    let (line, bytes) = found;
    let x = edge(line, line.order.visual_boundary(before(bytes, at)));
    Some(Caret {
        from: [x, line.baseline + line.descent],
        to: [x, line.baseline + line.ascent],
    })
}

/// The byte of the characters a point is nearest: the nearest line vertically, then the nearest
/// boundary along it.
fn byte_at(lines: &[(&Line, &[Stood])], [px, py]: [f32; 2]) -> Option<usize> {
    let mut nearest: Option<(f32, f32, usize)> = None;
    for (line, bytes) in lines {
        let (bottom, top) = (line.baseline + line.descent, line.baseline + line.ascent);
        let dy = if py < bottom {
            bottom - py
        } else if py > top {
            py - top
        } else {
            0.0
        };
        let mut best = (f32::INFINITY, 0_usize);
        for (shown, x) in line.edges.iter().enumerate() {
            let distance = (px - x).abs();
            if distance < best.0 {
                best = (distance, shown);
            }
        }
        let at = line.order.logical_boundary(best.1);
        let byte = bytes.get(at).map_or(line.span.1, |stood| stood.byte);
        if nearest
            .is_none_or(|(best_y, best_x, _)| dy < best_y || (dy <= best_y && best.0 < best_x))
        {
            nearest = Some((dy, best.0, byte));
        }
    }
    nearest.map(|(_, _, byte)| byte)
}

/// `fmt::Write` onto a byte buffer, for the `/DA` string [`one_style`] extends.
struct ByteWriter<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for ByteWriter<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Alignment;

    /// The same text is the identity, both ways.
    #[test]
    fn one_text_aligns_with_itself() {
        let alignment = Alignment::of("ab c", "ab c");
        for at in 0..=4 {
            assert_eq!(alignment.value(at), at);
            assert_eq!(alignment.rich(at), at);
        }
    }

    /// A run of the value's white space the string compresses to one space, and a CR LF the
    /// string spells as one carriage return, each meet at their ends (chapter 27, page 1194).
    #[test]
    fn compressed_white_space_meets_at_its_ends() {
        let alignment = Alignment::of("a b\rc", " a   b\r\nc");
        assert_eq!(alignment.value(0), 1, "a");
        assert_eq!(alignment.value(1), 2, "the space");
        assert_eq!(alignment.value(2), 5, "b");
        assert_eq!(alignment.value(4), 8, "c");
        assert_eq!(alignment.value(5), 9, "the end");
        assert_eq!(alignment.rich(5), 2, "b's own byte");
        assert_eq!(alignment.rich(3), 1, "inside the run is the space");
        assert_eq!(alignment.rich(8), 4, "c");
        assert_eq!(alignment.rich(9), 5, "the end");
    }

    /// Spaces the string keeps (`xfa-spacerun:yes`) are the value's one for one, and a line end
    /// between two kept breaks is one of the value's.
    #[test]
    fn kept_white_space_meets_one_for_one() {
        let alignment = Alignment::of("a  b\r\rc", "a  b\r\n\r\nc");
        assert_eq!(alignment.value(2), 2);
        assert_eq!(alignment.value(3), 3);
        assert_eq!(alignment.value(4), 4);
        assert_eq!(alignment.value(5), 6);
        assert_eq!(alignment.value(6), 8);
    }
}
