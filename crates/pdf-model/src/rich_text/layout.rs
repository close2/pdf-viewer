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
use super::style::{Align, Character, Linear, Spacing, Underline, VerticalAlign};
use crate::variable_text::{
    self, DefaultAppearance, Face, LaidOut, Owed, Placed, Quadding, Resolution,
};

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
    let paragraphs = encode(request, &styles, &mut faces);
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
    let plan = plan(&measure, &paragraphs.list, root, area, request);

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

    let mut owed = None;
    if paragraphs.truncated {
        owed = Some(Owed::Truncated(variable_text::MAX_CODES));
    } else if !paragraphs.missing.is_empty() {
        owed = Some(Owed::CharactersNotInFont(paragraphs.missing));
    } else if !request.rich.unapplied.is_empty() {
        owed = Some(Owed::RichTextUnapplied(request.rich.unapplied.phrase()));
    }
    if faces.base_resolution == Resolution::StoodIn {
        owed = Some(Owed::FontNotInResources(base_name));
    }
    Ok(LaidOut {
        content: stream,
        owed,
        overflows: plan.overflows(request.multiline),
        caret: None,
        offset: None,
        selection: Vec::new(),
        glyphs: Vec::new(),
        advance: plan.advance,
        fonts: faces.invented(),
    })
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

/// What a search path is resolved from: the family list, bold, italic.
type StyleKey = (Vec<String>, bool, bool);

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
    /// The `/DA`'s face's family, weight and posture.
    base: (String, bool, bool),
    base_resolution: Resolution,
    /// Each style's search path, by the key it is resolved from.
    paths: Vec<(StyleKey, Vec<usize>)>,
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
        })
    }

    /// The faces a style's characters are looked for in, nearest first, the `/DA`'s last.
    fn path(&mut self, style: &Character) -> Vec<usize> {
        let key = (style.families.clone(), style.bold(), style.italic);
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
            if let Some(index) = self.resolve(family, style.bold(), style.italic)
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

    fn resolve(&mut self, family: &str, bold: bool, italic: bool) -> Option<usize> {
        let wanted = normalised(family);
        if wanted == normalised(&self.base.0) && bold == self.base.1 && italic == self.base.2 {
            return Some(0);
        }
        // Table 224's `/DR`, the document's own faces.
        let fonts = self.document.get_key(self.resources, "Font");
        if let Some(fonts) = fonts.as_dict() {
            for (name, entry) in fonts.iter() {
                let Some(dict) = self.document.resolve(entry).as_dict().cloned() else {
                    continue;
                };
                let (stated, stated_bold, stated_italic) = family_of(self.document, &dict);
                if normalised(&stated) == wanted && stated_bold == bold && stated_italic == italic {
                    if let Some(index) = self.held.iter().position(|held| held.name == *name) {
                        return Some(index);
                    }
                    let face = Face::load(self.document, dict, name).ok()?;
                    return Some(self.hold(name.clone(), face, false));
                }
            }
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

/// A font dictionary's family, and whether it is bold and italic.
///
/// Table 120's `/FontFamily` and `/FontWeight` (PDF 1.5, the version rich text arrived in) and
/// `/Flags` bit 7, Italic, where the descriptor states them; the `/BaseFont` read the way
/// §9.6.2.2's fourteen and §9.6.3's `,Bold`-suffixed names spell a style otherwise, with a subset
/// tag (§9.9.2's six letters and a plus) taken off first.
fn family_of(document: &Document, dict: &Dictionary) -> (String, bool, bool) {
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
    let descriptor = variable_text::descriptor_of(document, dict);
    if let Some(descriptor) = descriptor.as_dict() {
        if let Some(stated) = document.get_key(descriptor, "FontFamily").as_string() {
            family = pdf_syntax::text_string(stated);
        }
        if let Some(weight) = document.get_key(descriptor, "FontWeight").as_number() {
            bold = weight > 500.0;
        }
        if let Some(flags) = document.get_key(descriptor, "Flags").as_integer() {
            // Table 121's bit 7, Italic; bit 19, ForceBold, says the face is drawn bold.
            italic = italic || flags & (1 << 6) != 0;
            bold = bold || flags & (1 << 18) != 0;
        }
    }
    (family, bold, italic)
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

/// One glyph: its face, its code, and the style it is set in.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Atom {
    face: usize,
    code: pdf_font::Code,
    space: bool,
    style: usize,
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

/// A paragraph as glyphs.
struct Encoded {
    /// `None` is a line break.
    items: Vec<Option<Atom>>,
    block: super::style::Block,
    strut: usize,
    tag: Option<Vec<Atom>>,
    list: ListIndent,
}

/// Every paragraph as glyphs, and what could not be encoded.
struct Paragraphs {
    list: Vec<Encoded>,
    missing: String,
    truncated: bool,
}

/// Turns each run into codes of the first face in its path that draws each character.
fn encode(request: &Request, styles: &Styles, faces: &mut Faces) -> Paragraphs {
    let mut out = Paragraphs {
        list: Vec::new(),
        missing: String::new(),
        truncated: false,
    };
    let mut total = 0_usize;
    for paragraph in &request.rich.paragraphs {
        let mut items = Vec::new();
        for piece in &paragraph.pieces {
            match piece {
                Piece::Break => items.push(None),
                Piece::Text(text, style) => {
                    let index = styles.index(style);
                    let atoms = encode_run(text, style, index, faces, &mut out.missing);
                    total = total.saturating_add(atoms.len());
                    if total > variable_text::MAX_CODES {
                        out.truncated = true;
                        break;
                    }
                    items.extend(atoms.into_iter().map(Some));
                }
            }
        }
        let tag = paragraph.tag.as_ref().map(|tag| {
            encode_run(
                &tag.text,
                &tag.style,
                styles.index(&tag.style),
                faces,
                &mut out.missing,
            )
        });
        out.list.push(Encoded {
            items,
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

/// One run's characters, each from the first face of its path that has a code for it, and the
/// consecutive characters one face draws encoded together so a cursive script joins within them.
fn encode_run(
    text: &str,
    style: &Character,
    index: usize,
    faces: &mut Faces,
    missing: &mut String,
) -> Vec<Atom> {
    let path = faces.path(style);
    let mut atoms = Vec::new();
    let mut segment = String::new();
    let mut segment_face: Option<usize> = None;
    let flush =
        |segment: &mut String, face: Option<usize>, atoms: &mut Vec<Atom>, faces: &Faces| {
            let Some(face) = face else {
                return;
            };
            let Some(held) = faces.get(face) else {
                return;
            };
            held.used.set(true);
            for placed in held.face.encode(segment).codes {
                match placed {
                    Placed::Shown(code) => atoms.push(Atom {
                        face,
                        code,
                        space: false,
                        style: index,
                    }),
                    Placed::Space(code) => atoms.push(Atom {
                        face,
                        code,
                        space: true,
                        style: index,
                    }),
                    Placed::Break => {}
                }
            }
            segment.clear();
        };
    for character in text.chars() {
        let face = path.iter().copied().find(|face| {
            faces
                .get(*face)
                .is_some_and(|held| held.face.code(character).is_some())
        });
        let Some(face) = face else {
            if !missing.contains(character) {
                missing.push(character);
            }
            continue;
        };
        if segment_face != Some(face) {
            flush(&mut segment, segment_face, &mut atoms, faces);
            segment_face = Some(face);
        }
        segment.push(character);
    }
    flush(&mut segment, segment_face, &mut atoms, faces);
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

    fn width(&self, atom: Atom, root: f32) -> f32 {
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
    atoms: Vec<Atom>,
    /// Where the line starts, after its alignment.
    x: f32,
    baseline: f32,
    /// Extra space after each space on the line: a justified line's.
    gap: f32,
    tag: Option<(Vec<Atom>, f32)>,
}

/// The whole text, placed.
struct Plan {
    lines: Vec<Line>,
    /// Whether some line is longer than the room it was given: a word longer than a line, or a
    /// single line's whole text.
    too_wide: bool,
    /// Whether the block is taller than the box.
    too_tall: bool,
    advance: f32,
}

impl Plan {
    /// Table 231 bit 24's question, on the axis the clause names for the shape.
    fn overflows(&self, multiline: bool) -> bool {
        if multiline {
            self.too_tall
        } else {
            self.too_wide
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
            .flat_map(|paragraph| paragraph.items.iter().filter_map(|item| *item))
            .collect();
        out.push(Broken {
            atoms,
            paragraph: 0,
            first: true,
            last: true,
        });
        return out;
    }
    for (index, paragraph) in paragraphs.iter().enumerate() {
        let start_at = out.len();
        let (_, width) = horizontal(measure, paragraph, root, area, true);
        let (_, rest_width) = horizontal(measure, paragraph, root, area, false);
        let mut line: Vec<Atom> = Vec::new();
        let mut reached = 0.0_f32;
        let mut last_space: Option<usize> = None;
        let mut limit = width;
        let push = |out: &mut Vec<Broken>, atoms: Vec<Atom>| {
            out.push(Broken {
                atoms,
                paragraph: index,
                first: false,
                last: false,
            });
        };
        for item in &paragraph.items {
            let Some(atom) = item else {
                push(&mut out, std::mem::take(&mut line));
                reached = 0.0;
                last_space = None;
                limit = rest_width;
                continue;
            };
            reached += measure.width(*atom, root);
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
            push(&mut out, std::mem::take(&mut line));
            line = carried;
            reached = measure.widths(&line, root);
            last_space = None;
            limit = rest_width;
        }
        push(&mut out, line);
        if let Some(first) = out.get_mut(start_at) {
            first.first = true;
        }
        if let Some(last) = out.last_mut() {
            last.last = true;
        }
    }
    out
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
) -> Plan {
    let broken = break_lines(measure, paragraphs, root, area, request.multiline);
    let (baselines, total) = stack(measure, paragraphs, &broken, root, area);
    let valign = area.valign.unwrap_or(if request.multiline {
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
    let too_tall = total > area.height();
    let mut too_wide = false;
    for (line, baseline) in broken.into_iter().zip(baselines) {
        let Some(paragraph) = paragraphs.get(line.paragraph) else {
            continue;
        };
        let (start, width) = horizontal(measure, paragraph, root, area, line.first);
        let shown = trimmed(&line.atoms);
        let used = measure.widths(shown, root);
        advance = advance.max(used);
        if used > width {
            too_wide = true;
        }
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
            x,
            baseline: offset + baseline,
            gap,
            tag,
        });
    }
    Plan {
        lines,
        too_wide,
        too_tall,
        advance,
    }
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
        let planned = plan(measure, paragraphs, root, area, request);
        !planned.too_wide && !planned.too_tall
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
    let mut x = line.x;
    let mut start = 0_usize;
    while start < line.atoms.len() {
        let Some(first) = line.atoms.get(start) else {
            break;
        };
        // A group is the longest run of one face and one style, and on a justified line it ends
        // after each space, where the extra room is inserted.
        let mut end = start.saturating_add(1);
        if !(line.gap > 0.0 && first.space) {
            while let Some(next) = line.atoms.get(end) {
                if next.face != first.face || next.style != first.style {
                    break;
                }
                end = end.saturating_add(1);
                if line.gap > 0.0 && next.space {
                    break;
                }
            }
        }
        let group = line.atoms.get(start..end).unwrap_or_default();
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
        x += measure.widths(group, root);
        if group.last().is_some_and(|atom| atom.space) {
            x += line.gap;
        }
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
        let (_, bold, italic) = family_of(document, &dict);
        root.weight = if bold { 700 } else { 400 };
        root.italic = italic;
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

/// The one-style form of a string: what a value in a right-to-left script, a comb field, or a
/// question a host asks of the text is laid out with, until the runs reach those constructions
/// too.
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
                Piece::Break => None,
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

/// `fmt::Write` onto a byte buffer, for the `/DA` string [`one_style`] extends.
struct ByteWriter<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for ByteWriter<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}
