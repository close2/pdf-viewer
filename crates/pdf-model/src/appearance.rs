//! Constructing an appearance stream for an annotation that carries none.
//!
//! ISO 32000-2 Table 166 requires a writer to supply an appearance dictionary for every
//! annotation but three — a `Popup`, a `Projection`, or a `Link` — and real files break that
//! requirement constantly. What a processor is then to draw is stated by the annotation's own
//! subtype clause, out of the entries §12.5.2 calls appearance characteristics: `/C`, `/IC`,
//! `/Border`, `/BS`, and for a widget the `/MK` dictionary of §12.5.6.19. §12.7.4.3 names the
//! operation — "the PDF processor shall construct an appearance stream dynamically at
//! rendering time" — and that is literally what this module does: it writes a content stream,
//! which [`crate::annotation`] places by §12.5.5's algorithm and `crate::content` then runs
//! like any other form `XObject`.
//!
//! # The same keys, two roles
//!
//! §12.5.2 closes with the rule that decides this module's whole shape:
//!
//! > A PDF reader shall render the appearance dictionary without regard to any other keys and
//! > values in the annotation dictionary and shall ignore the values of the C, IC, Border, BS,
//! > BE, BM, CA, ca, H, DA, Q, DS, LE, LL, LLE, and Sy keys.
//!
//! A stored appearance stream is self-contained and these entries mean nothing to it; an
//! annotation without one is drawn *from* them. Table 166 says the same of `/CA` and `/ca` from
//! the other side — each is the opacity "when regenerating the annotation's appearance stream",
//! and "shall not be used if the annotation has an appearance stream ... in that case, the
//! appearance stream shall specify any transparency".
//!
//! # Where this module stops, and why that is not laziness
//!
//! A subtype whose clause states a *shape* is constructed here. A subtype whose clause names an
//! appearance without stating what it looks like is refused and reported, because there is
//! nothing to derive:
//!
//! - `Text`, `FileAttachment`, `Sound` and `Stamp` display an *icon*. §12.5.6.4 requires a
//!   processor to "provide predefined icon appearances" for seven names and says nothing about
//!   what any of them is. The artwork is the processor's, not the document's.
//! - `Highlight`, `Underline`, `StrikeOut` and `Squiggly` (§12.5.6.10) state their
//!   `/QuadPoints` and the edge the text is oriented against, and *no* mark: not the thickness
//!   of an underline, not where in the quadrilateral a strikeout crosses, not how a highlight
//!   keeps the text under it visible. Table 182 does not even admit a `/BS` to take a width
//!   from.
//! - `FreeText` (§12.5.6.6), and a widget holding a value, need §12.7.4.3's variable text — a
//!   font, a size and a quadding read out of a `/DA` string — which is form work this crate
//!   does not do yet.
//! - `Caret`, `Redact`, `Screen`, `Movie`, `PrinterMark`, `TrapNet` and `Watermark` state no
//!   geometry of their own.
//!
//! Guessing at any of those would put marks on the page the document never described, which is
//! the failure principle 5 exists to prevent — and the corpus says the guesses do differ: the
//! three reference renderers draw three different pictures of
//! `annotation-highlight-without-appearance.pdf`.

use pdf_syntax::{Dictionary, Document, Object};
use std::fmt::Write as _;

/// How many `/Parent` links a field's inheritable entry is followed through.
///
/// §12.7.4.1 makes `/FT`, `/Ff`, `/V` and `/DV` inheritable, so a widget merged with its field
/// may hold none of them itself. **The clause forbids this bound** — "An interactive PDF
/// processor shall not limit the range of inheritance for field dictionaries" — and it exists
/// anyway, because a `/Parent` chain in a hostile file can be a cycle and principle 3's budgets
/// outrank a depth no legitimate form comes near. Reaching it is reported rather than treated as
/// "no value", so the departure cannot hide: a bound that is silent is the defect, not the
/// bound.
const MAX_FIELD_ANCESTRY: usize = 32;

/// The default border width §12.5.4 states: "If neither the Border nor the BS entry is present,
/// the border shall be drawn as a solid line with a width of 1 point."
const DEFAULT_BORDER_WIDTH: f32 = 1.0;

/// The control-point distance, as a fraction of the radius, that makes a cubic Bézier an arc.
///
/// `4/3 × (√2 − 1)`, the value at which the curve passes exactly through the quarter-arc's
/// midpoint; the remaining error is under 0.03% of the radius, which is a fifth of a pixel on a
/// thousand-point circle. §12.5.6.8 asks for "an ellipse" and PDF has no operator that draws
/// one, so this constant is where that word becomes a path — a documented approximation, not a
/// reading of anything.
const ARC: f32 = 0.552_284_8;

/// What was constructed, and what could not be.
///
/// The two are independent because one annotation can need both: a widget states a background
/// colour this module can draw and a field value it cannot, and drawing the first while
/// reporting the second says two true things instead of losing one of them. That is the same
/// pairing §11.6.5.2's `/Matte` and `/NeedAppearances` already use.
pub(crate) struct Constructed {
    /// The content stream, if anything is drawn at all.
    pub content: Option<Vec<u8>>,
    /// What the clause asks for and this module cannot derive, for the report.
    pub report: Option<String>,
}

/// A colour read from an appearance-characteristics array.
///
/// Table 166's `/C` states the rule every one of these arrays follows: "The number of array
/// elements determines the colour space in which the colour shall be defined: 0 No colour;
/// transparent 1 `DeviceGray` 3 `DeviceRGB` 4 `DeviceCMYK`".
#[derive(Debug, Clone, Copy, PartialEq)]
enum Colour {
    /// One, three or four components, in the space the count names.
    Components([f32; 4], usize),
    /// An empty or absent array: "No colour; transparent", so nothing is painted with it.
    None,
}

/// What a subtype's routine drew, and what it could not.
struct Painted {
    drawn: bool,
    report: Option<Refusal>,
}

impl Painted {
    /// Everything the clause asks for is on the page.
    const DRAWN: Self = Self {
        drawn: true,
        report: None,
    };
    /// The annotation states nothing to draw, which is not a gap.
    const EMPTY: Self = Self {
        drawn: false,
        report: None,
    };

    /// Drawn as far as the clause states it, with the rest named.
    fn partly(refusal: Refusal) -> Self {
        Self {
            drawn: true,
            report: Some(refusal),
        }
    }
}

/// What an annotation asks for that this module cannot derive.
///
/// Each becomes a report rather than a guess, and each names the entry: an annotation drawn
/// without the thing it asked for is the silently-wrong page trap 5 forbids.
enum Refusal {
    /// A colour array whose length names no colour space (Table 166).
    ColourComponents(&'static str, usize),
    /// An entry the subtype's clause requires is absent or unreadable.
    Missing(&'static str),
    /// The clause names an appearance without stating what it looks like.
    NotDerivable(&'static str),
}

impl Refusal {
    /// The report's detail, which follows the subtype's name.
    fn detail(&self) -> String {
        match *self {
            Self::ColourComponents(key, count) => {
                format!("{key} has {count} components, which names no colour space")
            }
            Self::Missing(key) => format!("no appearance stream and no usable {key}"),
            Self::NotDerivable(why) => format!("no appearance stream, and {why}"),
        }
    }
}

/// Decides what to draw for an annotation with no appearance stream.
///
/// A `Constructed` with neither content nor report is the answer for an annotation that
/// legitimately draws nothing — a widget stating no background, no border and holding no value
/// is *empty*, not unsupported, and reporting it would name 25 corpus documents for a gap that
/// is not one.
pub(crate) fn construct(
    document: &Document,
    annotation: &Dictionary,
    subtype: &[u8],
) -> Constructed {
    let mut stream = Stream::new();
    let outcome = match subtype {
        b"Link" => link(document, annotation, &mut stream),
        b"Square" | b"Circle" => square_or_circle(document, annotation, &mut stream, subtype),
        b"Polygon" | b"PolyLine" => polygon(document, annotation, &mut stream, subtype),
        b"Ink" => ink(document, annotation, &mut stream),
        b"Line" => line(document, annotation, &mut stream),
        b"Widget" => widget(document, annotation, &mut stream),
        b"Highlight" | b"Underline" | b"StrikeOut" | b"Squiggly" => Err(Refusal::NotDerivable(
            "§12.5.6.10 states its /QuadPoints without stating what mark to make in them",
        )),
        b"FreeText" => Err(Refusal::NotDerivable(
            "its text needs §12.7.4.3's variable text",
        )),
        b"Text" | b"FileAttachment" | b"Sound" | b"Stamp" => Err(Refusal::NotDerivable(
            "its icon's artwork is the processor's own, and no clause states it",
        )),
        _ => Err(Refusal::NotDerivable("its clause states no geometry")),
    };

    let painted = match outcome {
        Ok(painted) => painted,
        Err(refusal) => Painted {
            drawn: false,
            report: Some(refusal),
        },
    };
    Constructed {
        content: painted.drawn.then(|| stream.text.into_bytes()),
        report: painted.report.map(|refusal| refusal.detail()),
    }
}

/// Draws a link's border: §12.5.4's rounded rectangle, in Table 166's `/C`.
///
/// Table 166 lists a link as one of the three subtypes a writer need not give an appearance
/// dictionary to, and names the colour it is drawn in — `/C` is "a colour used for ... The
/// border of a link annotation". §12.5.4 supplies the rest: the border "shall be drawn
/// completely inside the annotation rectangle", so the stroke is inset by half its own width.
///
/// A link with no `/C` draws nothing. Table 166 gives an *empty* `/C` the meaning "No colour;
/// transparent" and states no default for an absent one, so this is a deliberate choice between
/// two silences: an unstated colour is at most the transparent one, and the alternative —
/// inventing black — would put a box around every link whose file says nothing about its
/// colour.
fn link(document: &Document, annotation: &Dictionary, stream: &mut Stream) -> Outcome {
    let rect = rectangle(document, annotation)?;
    let border = Border::read(document, annotation, annotation, "C")?;
    if !border.strokes() {
        return Ok(Painted::EMPTY);
    }
    border.apply(stream);
    border.outline(stream, rect);
    stream.paint(false, true);
    Ok(border.simulated())
}

/// Draws §12.5.6.8's rectangle or ellipse, inscribed in `/Rect` less `/RD`.
///
/// > shall display, respectively, a rectangle or an ellipse on the page
///
/// > The rectangle or ellipse shall be inscribed within the annotation rectangle defined by the
/// > annotation dictionary's Rect entry
///
/// Table 180's `/IC` fills it and Table 166's `/C` strokes it. `/RD` is the difference between
/// `/Rect` and the shape, which exists because a `/BE` border effect can push the two apart.
fn square_or_circle(
    document: &Document,
    annotation: &Dictionary,
    stream: &mut Stream,
    subtype: &[u8],
) -> Outcome {
    if cloudy(document, annotation) {
        return Err(CLOUDY);
    }
    let rect = rectangle(document, annotation)?;
    let border = Border::read(document, annotation, annotation, "C")?;
    let interior = colour(document, annotation, "IC")?;
    if !border.strokes() && interior == Colour::None {
        return Ok(Painted::EMPTY);
    }

    let box_ = border.inset(differences(document, annotation, rect));
    border.apply(stream);
    stream.set_colour(interior, false);
    if subtype == b"Circle" {
        stream.ellipse(box_);
    } else {
        stream.rectangle(box_);
    }
    stream.paint(interior != Colour::None, border.strokes());
    Ok(border.simulated())
}

/// Draws §12.5.6.9's polygon or polyline, from its `/Vertices` or its `/Path`.
///
/// Table 181 divides the interior colour by subtype, and that difference is the whole reason
/// these two share a clause but not a routine: "For Polyline annotations, the value of the IC
/// key is used to fill only the line ending. However, for Polygon annotations, the value of the
/// IC key is used to fill the entire shape, much as the F operator would fill a shape in a
/// content stream." A polyline's `/IC` is a line-ending colour, and line endings are refused.
fn polygon(
    document: &Document,
    annotation: &Dictionary,
    stream: &mut Stream,
    subtype: &[u8],
) -> Outcome {
    if cloudy(document, annotation) {
        return Err(CLOUDY);
    }
    if line_endings(document, annotation) {
        return Err(LINE_ENDINGS);
    }

    let closed = subtype == b"Polygon";
    let border = Border::read(document, annotation, annotation, "C")?;
    let interior = if closed {
        colour(document, annotation, "IC")?
    } else {
        Colour::None
    };
    if !border.strokes() && interior == Colour::None {
        return Ok(Painted::EMPTY);
    }

    border.apply(stream);
    stream.set_colour(interior, false);
    if !path(document, annotation, stream)? {
        let Some(vertices) = points(document, annotation, "Vertices") else {
            return Err(Refusal::Missing("/Vertices or /Path"));
        };
        polyline(stream, &vertices, closed);
    }
    stream.paint(interior != Colour::None, border.strokes());
    Ok(Painted::DRAWN)
}

/// Draws §12.5.6.13's scribble: one stroked path per `/InkList` entry.
///
/// > When drawn, the points shall be connected by straight lines or curves in an
/// > implementation-dependent way.
///
/// Straight lines, then — the clause offers the choice outright, and a curve fitted through a
/// digitiser's own samples would be this module inventing smoothing the document did not ask
/// for. A `/Path` (PDF 2.0) supersedes `/InkList` and carries its own curves.
fn ink(document: &Document, annotation: &Dictionary, stream: &mut Stream) -> Outcome {
    let border = Border::read(document, annotation, annotation, "C")?;
    if !border.strokes() {
        return Ok(Painted::EMPTY);
    }
    border.apply(stream);

    if path(document, annotation, stream)? {
        stream.paint(false, true);
        return Ok(Painted::DRAWN);
    }
    let list = document.get_key(annotation, "InkList");
    let Some(strokes) = list.as_array() else {
        return Err(Refusal::Missing("/InkList or /Path"));
    };
    let mut drawn = false;
    for entry in strokes {
        let resolved = document.resolve(entry);
        let Some(values) = resolved.as_array() else {
            continue;
        };
        let vertices = pairs(document, values);
        if vertices.len() >= 2 {
            polyline(stream, &vertices, false);
            drawn = true;
        }
    }
    if !drawn {
        return Err(Refusal::Missing("/InkList or /Path"));
    }
    stream.paint(false, true);
    Ok(Painted::DRAWN)
}

/// Draws §12.5.6.7's single straight line, between `/L`'s two points.
///
/// Three of the clause's entries change where the line goes or what is written along it, and
/// none of the three can be derived: `/LL` makes `/L` "represent the endpoints of the leader
/// lines rather than the endpoints of the line itself", so drawing `/L` would put the line in
/// the wrong place; `/LE`'s endings have no stated size; `/Cap` replicates `/Contents` as a
/// caption, which is text. Each is refused rather than approximated.
fn line(document: &Document, annotation: &Dictionary, stream: &mut Stream) -> Outcome {
    if document.get_key(annotation, "LL").as_number().is_some() {
        return Err(Refusal::NotDerivable(
            "§12.5.6.7's /LL makes /L the leader lines' endpoints rather than the line's",
        ));
    }
    if matches!(document.get_key(annotation, "Cap"), Object::Boolean(true)) {
        return Err(Refusal::NotDerivable(
            "§12.5.6.7's /Cap asks for /Contents as a caption along the line",
        ));
    }
    if line_endings(document, annotation) {
        return Err(LINE_ENDINGS);
    }

    let ends = points(document, annotation, "L").unwrap_or_default();
    let (Some(start), Some(end)) = (ends.first(), ends.get(1)) else {
        return Err(Refusal::Missing("/L"));
    };
    let border = Border::read(document, annotation, annotation, "C")?;
    if !border.strokes() {
        return Ok(Painted::EMPTY);
    }
    border.apply(stream);
    stream.move_to(*start);
    stream.line_to(*end);
    stream.paint(false, true);
    Ok(Painted::DRAWN)
}

/// Draws §12.5.6.19's widget: Table 192's background and border, and nothing else.
///
/// Table 192's `/BG` is "the colour of the widget annotation's background" and `/BC` "the
/// colour of the widget annotation's border"; the width and style are §12.5.4's, as for any
/// annotation. A field's *value* is not here — §12.7.4.3's variable text needs a font read out
/// of a `/DA` string — so a widget holding one draws its frame and says the rest out loud.
///
/// Table 192's `/R` is not read, and cannot matter yet: it rotates the widget's *contents*
/// within `/Rect`, and a background filling that rectangle with a border inside it is unchanged
/// by any multiple of 90 degrees. It becomes load-bearing with the first glyph.
fn widget(document: &Document, annotation: &Dictionary, stream: &mut Stream) -> Outcome {
    let rect = rectangle(document, annotation)?;
    let characteristics = document.get_key(annotation, "MK").as_dict().cloned();
    let source = characteristics.as_ref().unwrap_or(annotation);
    let background = colour(document, source, "BG")?;
    let border = Border::read(document, annotation, source, "BC")?;

    let frame = background != Colour::None || border.strokes();
    if frame {
        if background != Colour::None {
            stream.set_colour(background, false);
            stream.rectangle(rect);
            stream.paint(true, false);
        }
        if border.strokes() {
            border.apply(stream);
            border.outline(stream, rect);
            stream.paint(false, true);
        }
    }

    match has_value(document, annotation) {
        Value::Present => {
            return Ok(Painted {
                drawn: frame,
                report: Some(Refusal::NotDerivable(
                    "its field's value needs §12.7.4.3's variable text",
                )),
            });
        }
        Value::TooDeep => {
            return Ok(Painted {
                drawn: frame,
                report: Some(Refusal::NotDerivable(
                    "its field's /Parent chain is longer than this crate follows, so whether it \
                     holds a value is unknown",
                )),
            });
        }
        Value::Absent => {}
    }
    if frame {
        Ok(border.simulated())
    } else {
        Ok(Painted::EMPTY)
    }
}

/// What a subtype's routine returns: what it painted, or what it could not read.
type Outcome = Result<Painted, Refusal>;

/// Table 169's cloudy border effect: "the border should be drawn as a series of convex curved
/// line segments in a manner that simulates the appearance of a cloud" — which states no curve,
/// no segment count, and no relation between `/I`'s intensity and either.
const CLOUDY: Refusal =
    Refusal::NotDerivable("§12.5.4's cloudy /BE border states no curve to draw");

/// Table 179's line endings: nine named shapes — "A square", "Two short lines meeting in an
/// acute angle" — and not one dimension among them.
const LINE_ENDINGS: Refusal =
    Refusal::NotDerivable("Table 179's line endings state no size to draw them at");

/// The border style names of §12.5.4 Table 168.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Style {
    /// `S`, the default: "A solid rectangle surrounding the annotation."
    Solid,
    /// `D`: dashed, by `/D` or by `/Border`'s fourth element.
    Dashed,
    /// `U`: "A single line along the bottom of the annotation rectangle."
    Underline,
    /// `B` or `I`: "A simulated embossed rectangle that appears to be raised above the surface
    /// of the page", or an engraved one. Table 168 states neither the highlight nor the shadow
    /// colour that produces the illusion, so the rectangle is drawn and the effect reported.
    Simulated,
}

/// An annotation's border: §12.5.4's width, style and dash, with the colour to stroke it in.
struct Border {
    colour: Colour,
    width: f32,
    dash: Vec<f32>,
    style: Style,
    /// Table 166's `/Border` corner radii, horizontal then vertical.
    radii: [f32; 2],
}

impl Border {
    /// Reads a border, taking its colour out of `source` — `/MK` for a widget, the annotation
    /// itself for everything else — and everything else out of the annotation.
    fn read(
        document: &Document,
        annotation: &Dictionary,
        source: &Dictionary,
        key: &'static str,
    ) -> Result<Self, Refusal> {
        let colour = colour(document, source, key)?;
        let entry = document.get_key(annotation, "Border");
        let border = entry.as_array().unwrap_or_default();
        let radii = [
            number(document, border.first()).unwrap_or_default(),
            number(document, border.get(1)).unwrap_or_default(),
        ];

        // Table 166: "If an annotation dictionary includes the BS entry, then the Border entry
        // is ignored." §12.5.4 supplies the default width the two of them share.
        let (width, style, dash) = if let Some(style) = document.get_key(annotation, "BS").as_dict()
        {
            Self::from_style_dictionary(document, style)
        } else {
            let width = number(document, border.get(2)).unwrap_or(DEFAULT_BORDER_WIDTH);
            let fourth = border.get(3).map(|item| document.resolve(item));
            let dash = fourth
                .as_ref()
                .and_then(Object::as_array)
                .and_then(|values| numbers(document, values));
            let style = if dash.is_some() {
                Style::Dashed
            } else {
                Style::Solid
            };
            (width, style, dash.unwrap_or_default())
        };

        Ok(Self {
            colour,
            width: if width.is_finite() {
                width.max(0.0)
            } else {
                0.0
            },
            dash,
            style,
            radii,
        })
    }

    /// Reads Table 168's width, style and dash out of a `/BS` dictionary.
    fn from_style_dictionary(document: &Document, style: &Dictionary) -> (f32, Style, Vec<f32>) {
        let width = document
            .get_key(style, "W")
            .as_number()
            .map_or(DEFAULT_BORDER_WIDTH, narrow);
        let name = document.get_key(style, "S");
        let kind = match name.as_name().map(pdf_syntax::Name::as_bytes) {
            Some(b"D") => Style::Dashed,
            Some(b"U") => Style::Underline,
            Some(b"B" | b"I") => Style::Simulated,
            // Table 168: "An interactive PDF processor shall tolerate other border styles that
            // it does not recognise", which leaves the default.
            _ => Style::Solid,
        };
        let dash = if kind == Style::Dashed {
            let entry = document.get_key(style, "D");
            entry
                .as_array()
                .and_then(|values| numbers(document, values))
                // Table 168's default for `/D` is `[3]`.
                .unwrap_or_else(|| vec![3.0])
        } else {
            Vec::new()
        };
        (width, kind, dash)
    }

    /// Whether this border marks the page at all: Table 168's "If this value is 0, no border
    /// shall be drawn", and Table 166's "if the border width is 0, no border is drawn".
    fn strokes(&self) -> bool {
        self.width > 0.0 && self.colour != Colour::None
    }

    /// Shrinks a rectangle by half the border width.
    ///
    /// §12.5.4: "If present, the border shall be drawn completely inside the annotation
    /// rectangle." A stroke straddles its path, so the path is the rectangle inset by half the
    /// width. A border wider than the rectangle would invert it; the inset stops at the centre
    /// line, which fills the rectangle solid and is what a border that thick asks for.
    fn inset(&self, rect: [f32; 4]) -> [f32; 4] {
        let half = self.width * 0.5;
        let inset_x = half.min((rect[2] - rect[0]) * 0.5);
        let inset_y = half.min((rect[3] - rect[1]) * 0.5);
        [
            rect[0] + inset_x,
            rect[1] + inset_y,
            rect[2] - inset_x,
            rect[3] - inset_y,
        ]
    }

    /// Appends the path this border's style asks for around a rectangle.
    fn outline(&self, stream: &mut Stream, rect: [f32; 4]) {
        if self.style == Style::Underline {
            // Table 168: "A single line along the bottom of the annotation rectangle." The
            // line's width is centred on that edge, as any stroke is on its path.
            stream.move_to([rect[0], rect[1]]);
            stream.line_to([rect[2], rect[1]]);
        } else {
            stream.rounded_rectangle(self.inset(rect), self.radii);
        }
    }

    /// The report a `B` or `I` border owes: the rectangle is drawn, the illusion is not.
    fn simulated(&self) -> Painted {
        if self.style == Style::Simulated && self.strokes() {
            Painted::partly(Refusal::NotDerivable(
                "Table 168's beveled and inset borders state no highlight or shadow colour",
            ))
        } else {
            Painted::DRAWN
        }
    }

    /// Writes the colour, width and dash this border needs into the stream.
    fn apply(&self, stream: &mut Stream) {
        stream.set_colour(self.colour, true);
        stream.set_stroke(self.width, &self.dash);
    }
}

/// A content stream under construction, in the annotation's own default user space.
///
/// Every number is written with `{value}`, Rust's shortest round-tripping form for an `f32`. A
/// content stream's numbers are written in decimal, and a value with no decimal form never
/// reaches here because [`number`] refuses anything but a finite one.
struct Stream {
    text: String,
}

impl Stream {
    fn new() -> Self {
        Self {
            text: String::new(),
        }
    }

    /// Sets the non-stroking or stroking colour, in the space the component count names.
    fn set_colour(&mut self, colour: Colour, stroking: bool) {
        let Colour::Components(values, count) = colour else {
            return;
        };
        let operator = match (count, stroking) {
            (1, false) => "g",
            (1, true) => "G",
            (3, false) => "rg",
            (3, true) => "RG",
            (_, false) => "k",
            (_, true) => "K",
        };
        for value in values.iter().take(count) {
            let _ = write!(self.text, "{value} ");
        }
        let _ = writeln!(self.text, "{operator}");
    }

    /// Sets the line width (§8.4.3.2) and, where the style asks for one, the dash pattern.
    fn set_stroke(&mut self, width: f32, dash: &[f32]) {
        let _ = writeln!(self.text, "{width} w");
        if !dash.is_empty() {
            let _ = write!(self.text, "[");
            for (index, value) in dash.iter().enumerate() {
                let separator = if index == 0 { "" } else { " " };
                let _ = write!(self.text, "{separator}{value}");
            }
            // §12.5.4: "The dash phase shall not be specified and shall be assumed to be 0."
            let _ = writeln!(self.text, "] 0 d");
        }
    }

    fn move_to(&mut self, point: [f32; 2]) {
        let _ = writeln!(self.text, "{} {} m", point[0], point[1]);
    }

    fn line_to(&mut self, point: [f32; 2]) {
        let _ = writeln!(self.text, "{} {} l", point[0], point[1]);
    }

    fn curve_to(&mut self, first: [f32; 2], second: [f32; 2], end: [f32; 2]) {
        let _ = writeln!(
            self.text,
            "{} {} {} {} {} {} c",
            first[0], first[1], second[0], second[1], end[0], end[1]
        );
    }

    fn close(&mut self) {
        let _ = writeln!(self.text, "h");
    }

    /// Paints the current path: filled, stroked, or both, with Table 59's operators.
    fn paint(&mut self, fill: bool, stroke: bool) {
        let operator = match (fill, stroke) {
            (true, true) => "B",
            (true, false) => "f",
            (false, true) => "S",
            (false, false) => "n",
        };
        let _ = writeln!(self.text, "{operator}");
    }

    /// A rectangle, as `re` states one: a corner, then a width and a height.
    fn rectangle(&mut self, box_: [f32; 4]) {
        let _ = writeln!(
            self.text,
            "{} {} {} {} re",
            box_[0],
            box_[1],
            box_[2] - box_[0],
            box_[3] - box_[1]
        );
    }

    /// A rectangle with the corner radii Table 166's `/Border` gives it.
    ///
    /// Table 166: the border "shall be drawn as a rounded rectangle. The array consists of
    /// three numbers defining the horizontal corner radius, vertical corner radius, and border
    /// width ... If the corner radii are 0, the border has square (not rounded) corners".
    fn rounded_rectangle(&mut self, box_: [f32; 4], radii: [f32; 2]) {
        let (half_width, half_height) = ((box_[2] - box_[0]) * 0.5, (box_[3] - box_[1]) * 0.5);
        let radius = [
            radii[0].clamp(0.0, half_width.max(0.0)),
            radii[1].clamp(0.0, half_height.max(0.0)),
        ];
        if radius[0] <= 0.0 || radius[1] <= 0.0 {
            self.rectangle(box_);
            return;
        }

        let (left, bottom, right, top) = (box_[0], box_[1], box_[2], box_[3]);
        let (grip_x, grip_y) = (radius[0] * (1.0 - ARC), radius[1] * (1.0 - ARC));
        self.move_to([left + radius[0], bottom]);
        self.line_to([right - radius[0], bottom]);
        self.curve_to(
            [right - grip_x, bottom],
            [right, bottom + grip_y],
            [right, bottom + radius[1]],
        );
        self.line_to([right, top - radius[1]]);
        self.curve_to(
            [right, top - grip_y],
            [right - grip_x, top],
            [right - radius[0], top],
        );
        self.line_to([left + radius[0], top]);
        self.curve_to(
            [left + grip_x, top],
            [left, top - grip_y],
            [left, top - radius[1]],
        );
        self.line_to([left, bottom + radius[1]]);
        self.curve_to(
            [left, bottom + grip_y],
            [left + grip_x, bottom],
            [left + radius[0], bottom],
        );
        self.close();
    }

    /// An ellipse inscribed in a box, as four Bézier arcs (§12.5.6.8, and [`ARC`]).
    fn ellipse(&mut self, box_: [f32; 4]) {
        let (left, bottom, right, top) = (box_[0], box_[1], box_[2], box_[3]);
        let (middle_x, middle_y) = ((left + right) * 0.5, (bottom + top) * 0.5);
        let (grip_x, grip_y) = ((right - left) * 0.5 * ARC, (top - bottom) * 0.5 * ARC);

        self.move_to([left, middle_y]);
        self.curve_to(
            [left, middle_y + grip_y],
            [middle_x - grip_x, top],
            [middle_x, top],
        );
        self.curve_to(
            [middle_x + grip_x, top],
            [right, middle_y + grip_y],
            [right, middle_y],
        );
        self.curve_to(
            [right, middle_y - grip_y],
            [middle_x + grip_x, bottom],
            [middle_x, bottom],
        );
        self.curve_to(
            [middle_x - grip_x, bottom],
            [left, middle_y - grip_y],
            [left, middle_y],
        );
        self.close();
    }
}

/// Appends a run of points as one subpath, closed or open.
fn polyline(stream: &mut Stream, vertices: &[[f32; 2]], closed: bool) {
    let Some((first, rest)) = vertices.split_first() else {
        return;
    };
    stream.move_to(*first);
    for point in rest {
        stream.line_to(*point);
    }
    if closed {
        stream.close();
    }
}

/// Builds §12.5.6.9's and §12.5.6.13's PDF 2.0 `/Path`, if the annotation has one.
///
/// Table 181: "An array of n arrays, each supplying the operands for a path building operator
/// (m, l or c). ... The first array shall be of length 2 and specifies the operand of a moveto
/// operator ... Subsequent arrays of length 2 specify the operands of lineto operators. Arrays
/// of length 6 specify the operands for curveto operators." A `/Path` supersedes `/Vertices`
/// and `/InkList`, which the same tables say shall be ignored when it is present.
fn path(
    document: &Document,
    annotation: &Dictionary,
    stream: &mut Stream,
) -> Result<bool, Refusal> {
    let entry = document.get_key(annotation, "Path");
    let Some(segments) = entry.as_array() else {
        return Ok(false);
    };
    let mut started = false;
    for segment in segments {
        let resolved = document.resolve(segment);
        let Some(values) = resolved.as_array() else {
            continue;
        };
        let coordinates = pairs(document, values);
        match (started, coordinates.as_slice()) {
            (false, [start]) => {
                stream.move_to(*start);
                started = true;
            }
            (true, [next]) => stream.line_to(*next),
            (true, [first, second, end]) => stream.curve_to(*first, *second, *end),
            _ => return Err(Refusal::Missing("/Path")),
        }
    }
    Ok(started)
}

/// Reads the annotation's `/Rect`, normalised, or refuses.
fn rectangle(document: &Document, annotation: &Dictionary) -> Result<[f32; 4], Refusal> {
    crate::annotation::rectangle(document, annotation, "Rect").ok_or(Refusal::Missing("/Rect"))
}

/// Reads an entry as a list of points, refusing an empty one.
fn points(document: &Document, dict: &Dictionary, key: &'static str) -> Option<Vec<[f32; 2]>> {
    let entry = document.get_key(dict, key);
    let vertices = pairs(document, entry.as_array()?);
    (!vertices.is_empty()).then_some(vertices)
}

/// Turns alternating x and y numbers into points, stopping at the first unreadable one.
fn pairs(document: &Document, values: &[Object]) -> Vec<[f32; 2]> {
    let numbers: Vec<f32> = values
        .iter()
        .map_while(|value| number(document, Some(value)))
        .collect();
    numbers
        .chunks_exact(2)
        .map(|pair| [pair[0], pair[1]])
        .collect()
}

/// Applies §12.5.6.8 Table 180's `/RD`: the differences between `/Rect` and the shape in it.
///
/// > The four numbers shall correspond to the differences in default user space between the
/// > left, top, right, and bottom coordinates of Rect and those of the square or circle,
/// > respectively.
///
/// The order is left, *top*, right, bottom, which is not the order a rectangle's own four
/// numbers come in. A `/RD` whose insets exceed the rectangle is ignored rather than allowed to
/// invert it, which is the clause's own constraint: "The sum of the top and bottom differences
/// shall be less than the height of Rect".
fn differences(document: &Document, annotation: &Dictionary, rect: [f32; 4]) -> [f32; 4] {
    let entry = document.get_key(annotation, "RD");
    let Some(values) = entry.as_array() else {
        return rect;
    };
    let read = |index: usize| {
        number(document, values.get(index))
            .filter(|value| *value >= 0.0)
            .unwrap_or_default()
    };
    let (left, top, right, bottom) = (read(0), read(1), read(2), read(3));
    if left + right >= rect[2] - rect[0] || top + bottom >= rect[3] - rect[1] {
        return rect;
    }
    [
        rect[0] + left,
        rect[1] + bottom,
        rect[2] - right,
        rect[3] - top,
    ]
}

/// Whether the annotation's `/BE` asks for §12.5.4's cloudy border.
fn cloudy(document: &Document, annotation: &Dictionary) -> bool {
    document
        .get_key(annotation, "BE")
        .as_dict()
        .map(|effect| document.get_key(effect, "S"))
        .and_then(|name| name.as_name().cloned())
        .is_some_and(|name| name.as_bytes() == b"C")
}

/// Whether `/LE` names a line ending other than Table 179's `None`.
fn line_endings(document: &Document, annotation: &Dictionary) -> bool {
    let entry = document.get_key(annotation, "LE");
    let Some(values) = entry.as_array() else {
        return false;
    };
    values.iter().any(|value| {
        document
            .resolve(value)
            .as_name()
            .is_some_and(|name| name.as_bytes() != b"None")
    })
}

/// Whether a field holds text this module would have to lay out.
enum Value {
    /// It does: §12.7.4.3 is owed.
    Present,
    /// It does not, so the frame is the whole appearance.
    Absent,
    /// The `/Parent` chain ran past [`MAX_FIELD_ANCESTRY`], so the answer is unknown.
    TooDeep,
}

/// Whether a field has a value to draw, following §12.7.4.1's inheritable entries.
///
/// A value that is present but *empty* is not a value: a text field whose `/V` is the empty
/// string draws no text, so reporting §12.7.4.3 for it would name pages where there is nothing
/// to lay out. A button's caption (Table 192's `/CA`) counts, because it is text this module
/// cannot set either.
fn has_value(document: &Document, annotation: &Dictionary) -> Value {
    if let Some(characteristics) = document.get_key(annotation, "MK").as_dict()
        && let Some(caption) = document.get_key(characteristics, "CA").as_string()
        && !caption.is_empty()
    {
        return Value::Present;
    }

    let mut dict = annotation.clone();
    for _ in 0..MAX_FIELD_ANCESTRY {
        let value = document.get_key(&dict, "V");
        if let Some(text) = value.as_string() {
            return if text.is_empty() {
                Value::Absent
            } else {
                Value::Present
            };
        }
        if value.as_name().is_some() || value.as_array().is_some() {
            return Value::Present;
        }
        let Some(parent) = document.get_key(&dict, "Parent").as_dict().cloned() else {
            return Value::Absent;
        };
        dict = parent;
    }
    Value::TooDeep
}

/// Reads one of Table 166's colour arrays.
fn colour(document: &Document, dict: &Dictionary, key: &'static str) -> Result<Colour, Refusal> {
    let entry = document.get_key(dict, key);
    let Some(values) = entry.as_array() else {
        return Ok(Colour::None);
    };
    let Some(read) = numbers(document, values) else {
        return Ok(Colour::None);
    };
    let mut components = [0.0; 4];
    match read.len() {
        0 => Ok(Colour::None),
        count @ (1 | 3 | 4) => {
            for (slot, value) in components.iter_mut().zip(&read) {
                *slot = value.clamp(0.0, 1.0);
            }
            Ok(Colour::Components(components, count))
        }
        count => Err(Refusal::ColourComponents(key, count)),
    }
}

/// Reads an array of numbers, refusing the whole array if any entry is not a finite number.
fn numbers(document: &Document, values: &[Object]) -> Option<Vec<f32>> {
    values
        .iter()
        .map(|value| number(document, Some(value)))
        .collect()
}

/// Resolves one array entry to a finite `f32`.
fn number(document: &Document, value: Option<&Object>) -> Option<f32> {
    let narrowed = narrow(document.resolve(value?).as_number()?);
    narrowed.is_finite().then_some(narrowed)
}

fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a coordinate outside f32's range cannot place anything on a page, and \
                  `number` refuses the infinity the cast produces"
    )]
    {
        value as f32
    }
}
