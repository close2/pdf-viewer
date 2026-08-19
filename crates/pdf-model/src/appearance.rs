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
//! §12.5.2 closes with the rule that decides this module's whole shape. `doc/md/` still carries
//! the 2020 sentence, which read
//!
//! > A PDF reader shall render the appearance dictionary without regard to any other keys and
//! > values in the annotation dictionary and shall ignore the values of the C, IC, Border, BS,
//! > BE, BM, CA, ca, H, DA, Q, DS, LE, LL, LLE, and Sy keys.
//!
//! # Errata Collection 3 has rewritten that sentence, and the blockquote above is the old one
//!
//! It stays because it is what the conformance gate's copy of the standard contains: the
//! sponsored copy records EC3 as review markup and the Markdown conversion dropped every
//! annotation in all fourteen documents (ADR 0252, ADR 0253). Three marks on page 485 change it,
//! all `/State` `Review` `Completed` — two strikeouts under Issue #23 and #34 and a caret under
//! Issue #56 — and with them applied the clause reads "When rendering the appearance dictionary,
//! a PDF reader shall ignore the values of the C, IC, Border, BS, BE, CA, ca, H, DA, Q, DS, LE,
//! LL, MK, LLE, and Sy keys."
//!
//! Three things moved, and each matters here:
//!
//! - **The blanket clause is gone.** "[W]ithout regard to any other keys and values in the
//!   annotation dictionary" was the sentence's principle; what is left is a list, so an entry's
//!   absence from it no longer means a reader may consult it *by default* — but neither does the
//!   sentence forbid every entry it does not name.
//! - **`BM` left the list**, which [`crate::annotation::blend_mode`] is the consequence of.
//! - **`MK` joined it**, so a widget's appearance characteristics are for *constructing* a stream
//!   and never for painting over a stored one — which is what [`widget`] already does, since
//!   `/MK` is read on no path a stored appearance reaches.
//!
//! A stored appearance stream is self-contained and the listed entries mean nothing to it; an
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
//! - `FileAttachment`, `Sound` and `Stamp` display an *icon* whose artwork no clause states.
//!   §12.5.6.12, §12.5.6.15 and §12.5.6.16 each say a reader "**should** provide predefined
//!   icon appearances" for the names their tables list — a recommendation, and an invention
//!   with only a recommendation behind it is a mark the document never described.
//! - `Caret`, `Redact`, `Screen`, `Movie`, `PrinterMark`, `TrapNet` and `Watermark` state no
//!   geometry of their own.
//!
//! Guessing at either would put marks on the page the document never described, which is the
//! failure principle 5 exists to prevent.
//!
//! **`Text` was on that first list for a hundred and nineteen sessions and does not belong on
//! it**, because §12.5.6.4 says something the other three do not: "Interactive PDF processors
//! **shall** provide predefined icon appearances for at least the following standard names".
//! The artwork is as unstated there as anywhere — that is why `CLAUDE.md` uses this very icon
//! as its standing example of a silence — but the *obligation to have some* is normative, so
//! refusing was a conformance failure rather than a restraint. [`crate::icon`] holds the seven
//! shapes and the argument for each; [`text_icon`] holds what the document gets to say about
//! them. One clause of four obliging and three recommending is exactly the distinction a single
//! `match` arm over all four subtypes had hidden.
//!
//! **This list used to include §12.5.6.10's four text markup subtypes, and had since before
//! [`text_markup`] was written.** The thirty-fourth session read the clause again and found it
//! states four things — the mark's kind, its region, its orientation and Table 166's colour —
//! and leaves only a thickness, which the quadrilateral's own height supplies as a fraction
//! (ADR 0043). The refusal that stood for thirteen sessions had said the clause states nothing.
//! A comment naming a refusal outlived the refusal by eighty sessions; the ledger row did the
//! same, and both were corrected in the hundred-and-fifteenth. The corpus is still what says
//! the *thickness* is a choice rather than a derivation: the three reference renderers draw
//! three different pictures of `annotation-highlight-without-appearance.pdf`.
//!
//! # Text is a different kind of construction
//!
//! A `FreeText` (§12.5.6.6) and a field holding a value need §12.7.4.3's variable text, which
//! is not a shape read off an entry but a layout: a font, a size and a colour parsed out of a
//! `/DA` string, resolved against the interactive form dictionary's `/DR`, measured, wrapped
//! and positioned. [`crate::variable_text`] is that, and this module is what decides *which*
//! text each subtype and field type states — the two questions are separate and the clauses
//! that answer them are in different subclauses.

use pdf_render::{Transform, geom::Point};
use pdf_syntax::{Dictionary, Document, Object};
use std::fmt::Write as _;

use crate::icon::{self, Mark};
use crate::variable_text::{self, Asked, Owed, Quadding, Request, Shape};
use crate::view::FieldValue;

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

/// The colour a construction paints in when no entry states one.
///
/// Not a choice of colour so much as the absence of one: §8.4.1's Table 51 gives the graphics
/// state's colour parameter "Initial value: black", so a stream that names nothing paints in it.
/// It is written out anyway, so that the mark cannot depend on what ran before the appearance.
const BLACK: Colour = Colour::Components([0.0; 4], 1);

/// The control-point distance, as a fraction of the radius, that makes a cubic Bézier an arc.
///
/// `4/3 × (√2 − 1)`, the value at which the curve passes exactly through the quarter-arc's
/// midpoint; the remaining error is under 0.03% of the radius, which is a fifth of a pixel on a
/// thousand-point circle. §12.5.6.8 asks for "an ellipse" and PDF has no operator that draws
/// one, so this constant is where that word becomes a path — a documented approximation, not a
/// reading of anything.
pub(crate) const ARC: f32 = 0.552_284_8;

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
    /// Table 224's `/DR`, which §12.7.4.3 makes the resource dictionary of an appearance a
    /// processor constructs: "The resource dictionary (Resources) shall be created using
    /// resources from the interactive form dictionary's DR entry."
    ///
    /// Empty for every construction that draws only paths, which is most of them — a
    /// rectangle in a device colour names no resource.
    pub resources: Dictionary,
    /// Whether Table 166's `/Rect` bounds what this construction drew.
    ///
    /// **Five subtypes state their geometry in default user space and are therefore not bounded
    /// by it**: §12.5.6.7's `/L`, §12.5.6.9's `/Vertices`, §12.5.6.10's `/QuadPoints`,
    /// §12.5.6.13's `/InkList` and §12.5.6.6's `/CL` are all "in default user space", which is
    /// the page's space and not a box's — so a file whose `/Rect` does not contain them has
    /// written a bounding box that is wrong, and the marks the clause states are still where the
    /// clause states them.
    ///
    /// **The fifth was missing from ADR 0193's table**, which listed the four it found while
    /// drawing a line annotation. Table 177's `/CL` uses that ADR's own words — "the starting,
    /// knee point, and ending coordinates of the line in default user space" — so a free text
    /// annotation belongs on the list, and joining it costs its *text* nothing: §12.7.4.3's own
    /// example puts "any required graphics state changes, such as clipping" inside the
    /// construction, so [`crate::variable_text::lay_out`] clips the value to the box itself
    /// rather than relying on a `/BBox`.
    ///
    /// Every other construction here *derives* its geometry from `/Rect` — an icon on the square
    /// inside it, a border along it, a field's text laid out in it — and stays inside by
    /// construction, so bounding those changes nothing except in the one case where it must not:
    /// §12.7.4.3's value, which is clipped to the field it does not fit in.
    pub bounded: bool,
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
    /// Table 175's `/Name` is outside the set §12.5.6.4 requires appearances for.
    NonStandardIcon(String),
    /// The clause names an appearance without stating what it looks like.
    NotDerivable(&'static str),
    /// Table 192's `/TP` names the side the caption goes on and not how much room it gets.
    CaptionBeside(i64),
    /// §12.7.4.3's variable text could not be laid out, or not entirely.
    Text(Owed),
}

impl Refusal {
    /// The report's detail, which follows the subtype's name.
    fn detail(&self) -> String {
        match self {
            Self::ColourComponents(key, count) => {
                format!("{key} has {count} components, which names no colour space")
            }
            Self::Missing(key) => format!("no appearance stream and no usable {key}"),
            Self::NonStandardIcon(name) => format!(
                "no appearance stream, and its icon /{name} is outside the seven names \
                 §12.5.6.4 requires an appearance for"
            ),
            Self::NotDerivable(why) => format!("no appearance stream, and {why}"),
            Self::CaptionBeside(code) => format!(
                "no appearance stream, and Table 192's /TP {code} states which side of the icon \
                 the caption goes on and not how much of the rectangle it takes"
            ),
            Self::Text(owed) => owed.detail(),
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
    view: crate::view::AnnotationView<'_>,
    rect: [f32; 4],
) -> Constructed {
    let mut stream = Stream::new();
    let bounded = !matches!(
        subtype,
        b"Line"
            | b"Polygon"
            | b"PolyLine"
            | b"Ink"
            | b"Highlight"
            | b"Underline"
            | b"StrikeOut"
            | b"Squiggly"
            | b"FreeText"
    );
    let outcome = match subtype {
        b"Link" => link(document, annotation, &mut stream),
        b"Square" | b"Circle" => square_or_circle(document, annotation, &mut stream, subtype),
        b"Polygon" | b"PolyLine" => polygon(document, annotation, &mut stream, subtype),
        b"Ink" => ink(document, annotation, &mut stream),
        b"Line" => line(document, annotation, &mut stream),
        b"Widget" => widget(document, annotation, &mut stream, view.value),
        b"Highlight" | b"Underline" | b"StrikeOut" | b"Squiggly" => {
            text_markup(document, annotation, &mut stream, subtype)
        }
        b"FreeText" => free_text(document, annotation, &mut stream, view.contents),
        b"Text" => text_icon(document, annotation, &mut stream, rect),
        b"FileAttachment" | b"Sound" => symbol_icon(document, annotation, &mut stream, subtype),
        // §12.5.6.12's stamp is the one of the four icon clauses whose standard names are not
        // *objects*. Table 184's list is `Approved`, `Experimental`, `NotApproved`, `AsIs`,
        // `Expired`, `Draft` and the rest — legends rather than symbols, so drawing one means
        // inventing typography and a border, and what a reader would then see is a word this
        // program chose to set in a face this program chose. The clause says **should**, and a
        // recommendation is not a licence to invent a different kind of thing from the one the
        // name names. `doc/todo/26` holds the argument.
        b"Stamp" => Err(Refusal::NotDerivable(
            "its clause recommends rather than requires a predefined icon, and Table 184's \
             standard names are legends rather than symbols",
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
    let content = painted.drawn.then(|| stream.text.clone().into_bytes());
    Constructed {
        content,
        report: painted.report.map(|refusal| refusal.detail()),
        resources: stream.finish(),
        bounded,
    }
}

/// The document's interactive form dictionary (§12.7.3), if the catalog names one.
fn interactive_form(document: &Document) -> Option<Dictionary> {
    let catalog = document.catalog().ok()?;
    document.get_key(&catalog, "AcroForm").as_dict().cloned()
}

/// Table 224's `/DR`, "a resource dictionary … containing default resources … that shall be
/// used by form field appearance streams".
///
/// §12.7.4.3 makes it the whole of a constructed appearance's `/Resources`, and §12.5.6.6 sends
/// a free text annotation to the same subclause, so it is where *any* `/DA`'s font is looked
/// up — not only a field's. A document with no interactive form dictionary has none, and a
/// `/DA` naming a font is then naming nothing, which is reported by name.
fn default_resources(document: &Document) -> Dictionary {
    interactive_form(document)
        .map(|form| document.get_key(&form, "DR"))
        .and_then(|resources| resources.as_dict().cloned())
        .unwrap_or_default()
}

/// An existing appearance stream with §12.7.4.3's new marks spliced into it.
pub(crate) struct Regenerated {
    /// The stored stream's own bytes, with the `/Tx` marked-content region replaced.
    pub content: Vec<u8>,
    /// The stream's `/Resources` with `/DR`'s entries added under the clause's precedence.
    pub resources: Dictionary,
    /// What the clause asked for and did not get, for the report.
    pub report: Option<String>,
}

/// Rewrites the variable-text region of an appearance stream the file already carries.
///
/// §12.7.4.3's closing paragraph states the operation, and it is a *splice* rather than a
/// replacement — which is the whole reason this function exists beside [`construct`]:
///
/// > The interactive PDF processor shall then replace the existing contents of the appearance
/// > stream from … BMC to the matching EMC with the corresponding new contents
///
/// > If the existing appearance stream contains no marked-content with tag … the new contents
/// > shall be appended to the end of the original stream.
///
/// Everything outside that pair survives, so a widget's own background, border and artwork are
/// kept and only the text is rewritten. `text_field_own_canvas_calc.pdf` is what makes the
/// difference visible: its whole stream is inside the pair and its field holds no value, so
/// the correct regeneration leaves the page blank — which `poppler` and `mupdf` also produce,
/// and a wholesale reconstruction from `/MK` would not.
///
/// The resources follow the same paragraph's first sentence: `/DR`'s entries are copied in,
/// and "If the DR and Resources dictionaries contain resources with the same name, the one
/// already in the Resources dictionary shall be left intact".
///
/// Returns `None` where the stream cannot be read, which leaves the caller drawing it as it
/// stands — the ordinary undecodable-appearance path reports that.
pub(crate) fn regenerate(
    document: &Document,
    annotation: &Dictionary,
    stored: &pdf_syntax::Stream,
    bbox: [f32; 4],
    value: FieldValue<'_>,
) -> Option<Regenerated> {
    let data = document.decoded_stream_data(stored)?;
    let characteristics = document.get_key(annotation, "MK").as_dict().cloned();
    let source = characteristics.as_ref().unwrap_or(annotation);
    let width = Border::read(document, annotation, source, "BC")
        .map(|border| border.width)
        .unwrap_or_default();

    // §12.7.4.3 puts the appearance stream's `/BBox` at the origin, so the text is laid out in
    // the stream's own space rather than the page's.
    let (marks, report) = match field_text(
        document,
        annotation,
        source,
        inset(bbox, width),
        value,
        Asked::default(),
    ) {
        Ok(Some(laid_out)) => (laid_out.content, laid_out.owed.map(|owed| owed.detail())),
        // A field with no value has no marks, and an empty marked-content region is the
        // "corresponding new contents" for it. This is the case the clause's own wording
        // makes different from leaving the stream alone.
        Ok(None) => ("/Tx BMC\nEMC\n".to_owned(), None),
        // Nothing can be laid out, so the stored marks are the best the file offers and are
        // left where they are — with the shortfall named.
        Err(refusal) => {
            return Some(Regenerated {
                content: data.to_vec(),
                resources: stream_resources(document, stored),
                report: Some(refusal.detail()),
            });
        }
    };

    Some(Regenerated {
        content: spliced(&data, marks.as_bytes()),
        resources: with_default_resources(document, stream_resources(document, stored)),
        report,
    })
}

/// Replaces the `/Tx BMC` … `EMC` region of a content stream, or appends where there is none.
///
/// The clause's parenthesis is the whole of the second case: "If the existing appearance stream
/// contains no marked-content with tag Tx, the new contents shall be appended to the end of the
/// original stream."
///
/// *Matching* `EMC` is what makes this a scan rather than a search: §14.6.1 nests marked
/// content, so the first `EMC` after the `/Tx BMC` may close something else. Depth is counted
/// over `BMC`, `BDC` and `EMC` as whole tokens.
fn spliced(stream: &[u8], marks: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(stream.len().saturating_add(marks.len()));
    let Some(start) = find_tx_marked_content(stream) else {
        out.extend_from_slice(stream);
        out.push(b'\n');
        out.extend_from_slice(marks);
        return out;
    };

    let mut depth = 0i32;
    let mut lexer = pdf_syntax::Lexer::new(stream.get(start..).unwrap_or_default());
    let mut end = stream.len();
    while let Some(token) = lexer.next_token() {
        let pdf_syntax::Token::Keyword(word) = token else {
            continue;
        };
        match word {
            b"BMC" | b"BDC" => depth = depth.saturating_add(1),
            b"EMC" => {
                depth = depth.saturating_sub(1);
                if depth <= 0 {
                    end = start.saturating_add(lexer.position());
                    break;
                }
            }
            _ => {}
        }
    }

    out.extend_from_slice(stream.get(..start).unwrap_or_default());
    out.extend_from_slice(marks);
    out.extend_from_slice(stream.get(end..).unwrap_or_default());
    out
}

/// Finds where a `/Tx BMC` begins, as a byte offset into a content stream.
///
/// Lexed rather than searched for as a substring: `(/Tx BMC)` inside a string operand is not
/// an operator, and a stream that shows one would otherwise be spliced in the middle of its
/// own text.
fn find_tx_marked_content(stream: &[u8]) -> Option<usize> {
    let mut lexer = pdf_syntax::Lexer::new(stream);
    let mut tag_at = None;
    let mut tag = false;
    loop {
        let before = lexer.position();
        let token = lexer.next_token()?;
        match token {
            pdf_syntax::Token::Name(name) if name == b"Tx" => {
                tag = true;
                tag_at = Some(before);
            }
            pdf_syntax::Token::Keyword(word) if word == b"BMC" && tag => return tag_at,
            _ => tag = false,
        }
    }
}

/// A stream's own `/Resources`, which §12.7.4.3 has `/DR` added to rather than replaced.
fn stream_resources(document: &Document, stream: &pdf_syntax::Stream) -> Dictionary {
    document
        .get_key(&stream.dict, "Resources")
        .as_dict()
        .cloned()
        .unwrap_or_default()
}

/// Copies `/DR`'s resources into a stream's own, keeping the stream's where both name one.
///
/// §12.7.4.3 states the precedence:
///
/// > If the DR and Resources dictionaries contain resources with the same name, the one
/// > already in the Resources dictionary shall be left intact, not replaced with the
/// > corresponding value from the DR dictionary.
///
/// "The same name" is the *resource's* name rather than the category's, so the merge is two
/// levels deep: a stream that names a `/Font` of its own still gains `/DR`'s other fonts.
fn with_default_resources(document: &Document, mut resources: Dictionary) -> Dictionary {
    let defaults = default_resources(document);
    for (category, entry) in defaults.iter() {
        let Some(from_default) = entry.as_dict() else {
            if resources.get_by_name(category).is_none() {
                resources.insert(category.clone(), entry.clone());
            }
            continue;
        };
        let mut merged = resources
            .get_by_name(category)
            .and_then(Object::as_dict)
            .cloned()
            .unwrap_or_default();
        for (name, resource) in from_default.iter() {
            if merged.get_by_name(name).is_none() {
                merged.insert(name.clone(), resource.clone());
            }
        }
        resources.insert(category.clone(), Object::Dictionary(merged));
    }
    resources
}

/// Whether §12.7.4.3 has this annotation's appearance constructed rather than read from `/AP`.
///
/// Table 224's `/NeedAppearances` is "a flag specifying whether to construct appearance streams
/// and appearance dictionaries for all widget annotations in the document", and §12.7.2 says
/// what the writer is admitting by setting it: "If such an object defines an appearance stream,
/// the appearance shall be consistent with the object's current value as a field" — so a stored
/// stream under this flag may not be.
///
/// **It does not reach every widget, and the reason is in the field types rather than in this
/// flag.** §12.7.4.3's subject is a field "that may contain text whose value is not known until
/// viewing time", and three of the four field types have no such text:
///
/// - a push-button is "a purely interactive control that responds immediately to user input
///   without retaining a permanent value" (§12.7.5.2.2), so there is nothing for `/V` to hold —
///   **this line quoted the sentence that said so outright until the four-hundred-and-nineteenth
///   session**, and Errata Collection 3 strikes "Because this type of retains no permanent
///   value, it shall not use the V and DV entries in the field dictionary" with no replacement
///   (Issue #386, `/State` `Review` `Completed`), leaving the definition it drew its reason
///   from;
/// - a check box's and a radio button's states each "shall be defined by an appearance stream in
///   the appearance dictionary of the field's widget annotation" (§12.7.5.2.3, §12.7.5.2.4), and
///   the value selects among them rather than describing them;
/// - a signature field's value is a signature dictionary, and signing "entails updating at
///   least the V entry and usually also the AP entry" (§12.7.5.5).
///
/// Only a text field and a choice field hold text the clause has a processor lay out, so only
/// those two have their stored appearance set aside. Regenerating the others would throw away
/// artwork the file does state in exchange for nothing the clause asks for.
///
/// # The second reason, which is not the flag
///
/// **A value this program replaced makes the stored stream stale whatever the flag says.**
/// §12.7.2's sentence — "[i]f such an object defines an appearance stream, the appearance shall
/// be consistent with the object's current value as a field" — is an obligation on whoever wrote
/// the file, and the file kept it: the stream matches the `/V` the file states. It stops being
/// kept the moment §12.7.6.3's reset, §12.7.8's import or a person's typing replaces that value,
/// and at that point drawing the stored stream would show a value the field no longer has, which
/// is the one failure this clause family exists to prevent. §12.7.4.3 states the algorithm for
/// exactly the two field types whose text is "not known until viewing time", and it is the only
/// construction available. The clause's own strongest form of this is one line further on, for a
/// `RichText` field: "the entire annotation appearance shall be regenerated each time the value
/// is changed".
///
/// Without this, a widget with a stored `/AP` in a document that does not set `/NeedAppearances`
/// showed its *old* value after an import or a reset — silently, since the sixty-second session.
/// No fixture caught it because the fixtures state no `/AP`, which is the path that constructs
/// rather than splices.
pub(crate) fn regenerates(
    document: &Document,
    annotation: &Dictionary,
    subtype: &[u8],
    value: FieldValue<'_>,
) -> bool {
    if subtype != b"Widget" {
        return false;
    }
    if matches!(value, FieldValue::Stored) {
        let Some(form) = interactive_form(document) else {
            return false;
        };
        if !matches!(
            document.get_key(&form, "NeedAppearances"),
            Object::Boolean(true)
        ) {
            return false;
        }
    }
    matches!(
        Field::read(document, annotation, FieldValue::Stored).kind,
        Some(FieldKind::Text | FieldKind::Choice { .. })
    )
}

/// What §7.5.6's writer puts in a widget's `/AP` after a person has changed its value.
///
/// The same two constructions the *drawing* path chooses between, which is the point of this
/// existing rather than a third one: [`regenerate`] where the file already carries a stream,
/// [`construct`] where it does not. What a saved file shows is then what this program shows,
/// because it is the same bytes — and a reader that ignores Table 224's `/NeedAppearances` sees
/// the new value rather than the old one.
pub(crate) enum ForSaving {
    /// There is nothing to write, and that is the clause's answer rather than a shortfall.
    ///
    /// §12.7.5.2.3 and §12.7.5.2.4 make a check box's and a radio button's states "defined by an
    /// appearance stream in the appearance dictionary of the field's widget annotation", which
    /// the *value* selects among — so writing the new `/V` is the whole of the change, and
    /// replacing `/AP` with one stream would destroy the states it selects from. A push button
    /// holds no value at all (§12.7.5.2.2) and a signature field's is a dictionary (§12.7.5.5).
    Selected,
    /// The stream to write.
    Stream(SavedStream),
    /// The stream this program cannot produce, so Table 224's flag is what the file gets.
    Owed,
}

/// The appearance stream itself, and what to write it over.
pub(crate) struct SavedStream {
    /// The stream's own bytes, uncompressed.
    pub content: Vec<u8>,
    /// §12.7.4.3's `/Resources`, built from `/DR` by whichever construction produced the bytes.
    pub resources: Dictionary,
    /// §8.10.2's `/BBox`, in the space the bytes are written in.
    pub bbox: [f32; 4],
    /// The stream dictionary this replaces, when the widget already had one.
    ///
    /// Carried so that the entries §8.10.2 and §12.5.5 read from it — `/Type`, `/Subtype`,
    /// `/Matrix`, and anything else its producer stated — survive a rewrite of its marks. Only
    /// the bytes and what describes the bytes are replaced.
    pub existing: Option<(pdf_syntax::ObjectId, Dictionary)>,
    /// What the clause asked for and did not get, for the caller to say.
    pub report: Option<String>,
}

/// Builds what a widget's `/AP` should hold once a person has changed the field's value.
///
/// [`regenerates`] decides whether there is anything to build, and it is the same decision the
/// drawing path makes: only a text field and a choice field hold text §12.7.4.3 has a processor
/// lay out. Asking it here rather than repeating its reasoning is what stops a saved check box
/// from losing the states its `/V` selects among.
pub(crate) fn for_saving(
    document: &Document,
    annotation: &Dictionary,
    value: FieldValue<'_>,
) -> ForSaving {
    if !regenerates(document, annotation, b"Widget", value) {
        return ForSaving::Selected;
    }
    let Ok(rect) = rectangle(document, annotation) else {
        return ForSaving::Owed;
    };
    let normal = document
        .get_key(annotation, "AP")
        .as_dict()
        .and_then(|appearances| appearances.get("N").cloned());
    let stored = normal
        .as_ref()
        .map(|entry| document.resolve(entry))
        .and_then(|entry| entry.as_stream().cloned());

    if let Some(stored) = stored {
        // §12.5.5 reads the stream's own `/BBox`; §12.7.4.3 states the one to use when it has
        // none, and `crate::annotation` argues why that extension rather than a refusal.
        let bbox = crate::annotation::rectangle(document, &stored.dict, "BBox").unwrap_or([
            0.0,
            0.0,
            rect[2] - rect[0],
            rect[3] - rect[1],
        ]);
        let Some(regenerated) = regenerate(document, annotation, &stored, bbox, value) else {
            return ForSaving::Owed;
        };
        return ForSaving::Stream(SavedStream {
            content: regenerated.content,
            resources: regenerated.resources,
            bbox,
            existing: normal
                .as_ref()
                .and_then(Object::as_reference)
                .map(|id| (id, stored.dict.clone())),
            report: regenerated.report,
        });
    }

    // With no stream to splice into, the appearance is built from Table 192's characteristics
    // and the field's value — and `crate::annotation::construct` says why its `/BBox` is the
    // annotation's `/Rect`: the marks are written in the page's own default user space, so
    // §12.5.5's algorithm reduces to the identity.
    // A widget's rectangle is the file's: §12.5.6.4's fixed-size icon is the only case where a
    // constructed appearance's box is not `/Rect`, and a widget is not it.
    let rect = rectangle(document, annotation).unwrap_or([0.0; 4]);
    let constructed = construct(
        document,
        annotation,
        b"Widget",
        crate::view::AnnotationView {
            value,
            ..crate::view::AnnotationView::default()
        },
        rect,
    );
    let Some(content) = constructed.content else {
        // A field with no value, no background and no border draws nothing, and there is no
        // stream to *replace* here — so adding an object that draws nothing would grow the file
        // to no purpose.
        return match constructed.report {
            Some(_) => ForSaving::Owed,
            None => ForSaving::Selected,
        };
    };
    ForSaving::Stream(SavedStream {
        content,
        resources: constructed.resources,
        bbox: rect,
        existing: None,
        report: constructed.report,
    })
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

/// Draws §12.5.6.4's icon: the background the standard states, and the symbol it does not.
///
/// > When closed, the annotation shall appear as an icon
///
/// > Interactive PDF processors shall provide predefined icon appearances for at least the
/// > following standard names: Comment, Key, Note, Help, NewParagraph, Paragraph, Insert
///
/// That second sentence is the reason this routine exists rather than a refusal, and it is
/// worth being exact about which half of it this tree can derive. The obligation is a *shall*
/// on the reader; the artwork is stated nowhere at all. So the shapes are
/// [`crate::icon`]'s invention, documented there as one, and everything read out of the
/// document is here: which icon (Table 175's `/Name`, defaulting to `Note`) and what colour
/// sits behind it (Table 166's `/C`, "The background of the annotation's icon when closed").
///
/// The three neighbouring subtypes that also show an icon — §12.5.6.12's stamp, §12.5.6.15's
/// file attachment, §12.5.6.16's sound — say "PDF readers **should** provide predefined icon
/// appearances", and are still refused. One clause obliges and three recommend; drawing an
/// invention where the standard only recommends one would be a choice with no requirement
/// behind it, which is a different thing from a choice a requirement forces.
///
/// `/Open` is not read *here*, and that is a statement about this routine rather than about the
/// entry: §12.5.6.4 gives an open text annotation a popup window "containing the text of the
/// note", and a window is not part of the page. `crate::popup`'s `opens_with_the_page` is what
/// reads it, since the four-hundred-and-fifty-ninth session — this comment said the entry was not
/// read at all, on the ground that this program drew no popup for any subtype, which stopped being
/// true in the three-hundred-and-twelfth.
/// A clause's mapping from an icon's name to its artwork.
type IconLookup = fn(&[u8]) -> Option<&'static [icon::Figure]>;

/// §12.5.6.15's and §12.5.6.16's icons, on the same construction as §12.5.6.4's.
///
/// One function for two clauses because they ask the same thing in the same words — Table 187 and
/// Table 188 both say a reader "should provide predefined icon appearances for at least" a list
/// of names, and both name *objects*. The difference from [`text_icon`] is Table 166's `/C`: that
/// entry is "[t]he background of the annotation's icon when closed" for a text annotation, and
/// neither of these two clauses says anything of the kind, so nothing is filled behind them.
///
/// A `/Name` outside the clause's list is reported by name rather than drawn as the default, for
/// [`icon::text_annotation`]'s reason: a default is what an *absent* entry means.
fn symbol_icon(
    document: &Document,
    annotation: &Dictionary,
    stream: &mut Stream,
    subtype: &[u8],
) -> Outcome {
    let rect = rectangle(document, annotation)?;
    let (default, lookup): (&[u8], IconLookup) = match subtype {
        b"Sound" => (icon::DEFAULT_SOUND_NAME, icon::sound),
        _ => (icon::DEFAULT_FILE_ATTACHMENT_NAME, icon::file_attachment),
    };
    let name = document
        .get_key(annotation, "Name")
        .as_name()
        .map_or_else(|| default.to_vec(), |n| n.as_bytes().to_vec());
    let Some(figures) = lookup(&name) else {
        return Err(Refusal::NonStandardIcon(
            String::from_utf8_lossy(&name).into_owned(),
        ));
    };
    let box_ = largest_square_within(rect);
    let side = box_[2] - box_[0];
    draw_icon(stream, figures, box_, side);
    Ok(Painted::DRAWN)
}

fn text_icon(
    document: &Document,
    annotation: &Dictionary,
    stream: &mut Stream,
    rect: [f32; 4],
) -> Outcome {
    // **The caller's rectangle and not the file's**, which is the one place the two differ:
    // §12.5.6.4 attaches a text annotation to a *point* and gives its icon a fixed size, so a
    // `/Rect` with no area gets `annotation::anchored_icon`'s square and this is what has to
    // draw into it.
    let name = document.get_key(annotation, "Name").as_name().map_or_else(
        || icon::DEFAULT_TEXT_NAME.to_vec(),
        |n| n.as_bytes().to_vec(),
    );
    let Some(figures) = icon::text_annotation(&name) else {
        return Err(Refusal::NonStandardIcon(
            String::from_utf8_lossy(&name).into_owned(),
        ));
    };

    let box_ = largest_square_within(rect);
    let side = box_[2] - box_[0];
    // §12.5.6.2 makes `/C` a group attribute, so a subordinate's own is ignored and the primary's
    // is the ink. `crate::markup` has the sentence and the list of ten.
    let shared = crate::markup::group_source(document, annotation);
    let background = colour(document, &shared, "C")?;
    if background != Colour::None {
        stream.set_colour(background, false);
        let radius = side * icon::BACKGROUND_RADIUS;
        stream.rounded_rectangle(box_, [radius, radius]);
        stream.paint(true, false);
    }

    draw_icon(stream, figures, box_, side);
    Ok(Painted::DRAWN)
}

/// Writes an icon's figures into a square, in the colour the module comment argues for.
///
/// Black, because the only colour any of the four icon clauses states is Table 166's `/C`
/// background: one invention is fewer than two, and `icon.rs`'s own comment names the cost.
fn draw_icon(stream: &mut Stream, figures: &[icon::Figure], box_: [f32; 4], side: f32) {
    stream.set_colour(Colour::Components([0.0; 4], 1), false);
    stream.set_colour(Colour::Components([0.0; 4], 1), true);
    stream.set_stroke(side * icon::STROKE_WIDTH, &[]);
    let place = |point: [f32; 2]| [box_[0] + point[0] * side, box_[1] + point[1] * side];
    for figure in figures {
        for mark in figure.marks {
            match *mark {
                Mark::Move(point) => stream.move_to(place(point)),
                Mark::Line(point) => stream.line_to(place(point)),
                Mark::Curve(first, second, end) => {
                    stream.curve_to(place(first), place(second), place(end));
                }
                Mark::Close => stream.close(),
            }
        }
        stream.paint(figure.filled, !figure.filled);
    }
}

/// The largest square that fits inside a rectangle, centred in it.
///
/// §12.5.6.4's icons carry their meaning in their proportions, so they are not stretched onto a
/// `/Rect` of another shape; see [`crate::icon`] for that choice and for the `NoZoom` sentence
/// it stands in for.
fn largest_square_within(rect: [f32; 4]) -> [f32; 4] {
    let side = (rect[2] - rect[0]).min(rect[3] - rect[1]);
    let left = rect[0] + ((rect[2] - rect[0]) - side) * 0.5;
    let bottom = rect[1] + ((rect[3] - rect[1]) - side) * 0.5;
    [left, bottom, left + side, bottom + side]
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
///
/// # What this subtype's `/BS` supplies, and what it does not
///
/// Table 180 gives it two of Table 168's entries and no more — "specifying the line width and
/// dash pattern that shall be used in drawing the rectangle or ellipse" — and §12.5.4 says the
/// same thing from the other end, naming the four subtypes it holds for:
///
/// > Such dictionaries may also be used to specify the width and dash pattern for the lines drawn
/// > by line, square, circle, and ink annotations.
///
/// So Table 168's `/S` styles nothing here: the mark is the annotation's own rectangle or ellipse
/// rather than §12.5.4's border around one, and there is no `U` underline or `B` bevel of it to
/// draw. **This reported a beveled or inset `/S` as an appearance it could not derive until the
/// four-hundred-and-fifty-ninth session**, which named a gap the clause does not have — the
/// mirror of [`Border::outline`]'s departure, one sentence over: an entry consulted where its own
/// table says it supplies nothing. §12.5.6.9's polygon, under the identically worded Table 181,
/// never reported it.
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
    // §12.5.6.2's group attributes: `/C` is on the list and `/IC` is not, so they are read from
    // two dictionaries where this annotation is a group's subordinate.
    let shared = crate::markup::group_source(document, annotation);
    let border = Border::read(document, annotation, &shared, "C")?;
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
    Ok(Painted::DRAWN)
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
    // As on a line annotation, and drawn since the three-hundred-and-fourteenth session. `/BE`
    // above is *not* the same case and stays a refusal — a cloudy border is a different border
    // rather than an extra mark, and drawing a straight one would put a shape on the page the
    // file did not describe.
    let endings = line_endings(document, annotation)?;

    let closed = subtype == b"Polygon";
    // §12.5.6.2: `/C` is a group attribute and `/IC` is not.
    let shared = crate::markup::group_source(document, annotation);
    let border = Border::read(document, annotation, &shared, "C")?;
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
    let vertices = if path(document, annotation, stream)? {
        // §12.5.6.9's `/Path` is a sequence of curves, and Table 181 makes `/Vertices` "not
        // present" where it is used — so the ends an ending would decorate are inside a
        // construction this routine does not hold. Reported below rather than guessed at.
        None
    } else {
        let Some(vertices) = points(document, annotation, "Vertices") else {
            return Err(Refusal::Missing("/Vertices or /Path"));
        };
        polyline(stream, &vertices, closed);
        Some(vertices)
    };
    stream.paint(interior != Colour::None, border.strokes());

    // **A polygon's ends meet, so it has none.** Table 181 gives `/LE` to both subtypes and
    // §12.5.6.9 gives a polygon no end to put one on: a polyline is what it calls a polygon
    // "except that the first and last vertex are not implicitly connected", so a polygon's are.
    // This is therefore the polyline half alone, which is also the only half Table 181's `/IC`
    // is a line-ending colour for.
    if !closed
        && let Some(vertices) = vertices.as_deref()
        && let (Some(first), Some(second)) = (vertices.first(), vertices.get(1))
        && let (Some(last), Some(before)) = (
            vertices.last(),
            vertices.get(vertices.len().saturating_sub(2)),
        )
    {
        // Each end's direction is the segment it belongs to, not the whole shape's: a polyline
        // bends, and an arrowhead follows the last leg.
        draw_endings(
            stream,
            [endings[0], Ending::None],
            [*second, *first],
            border.width,
            interior,
        );
        draw_endings(
            stream,
            [Ending::None, endings[1]],
            [*before, *last],
            border.width,
            interior,
        );
    } else if endings != [Ending::None; 2] {
        return Ok(Painted::partly(ENDINGS_WITH_NO_END));
    }
    Ok(Painted::DRAWN)
}

/// Draws §12.5.6.10's four text markup annotations, from their `/QuadPoints`.
///
/// # What the clause states, and what it leaves to this function
///
/// It states the mark's *kind* — the annotations "shall appear as highlights, underlines,
/// strikeouts (all PDF 1.3), or jagged ("squiggly") underlines" — its *region*, Table 182's
/// required `/QuadPoints`, "an array of 8×n numbers specifying the coordinates of n
/// quadrilaterals in default user space", and its *orientation*: "The text shall be oriented
/// with respect to the edge connecting points ( x 1 , y 1 ) and ( x 2 , y 2 )." Table 166's
/// `/C` gives the colour.
///
/// It states no thickness, no position within the quadrilateral for a strikeout, and no
/// period for a squiggle. Those are decided here, and the whole of the decision is that **the
/// quadrilateral's own height is the only length the annotation gives**: it "shall encompass a
/// word or group of contiguous words", so its height is the text's. A thickness of one
/// sixteenth of it, a squiggle of one twelfth in amplitude and a third in wavelength, and a
/// strikeout across the middle are choices at that scale rather than measurements. They are
/// stated as fractions so that they are right at every font size without a constant that is
/// right at one.
///
/// # Two readings of "counterclockwise", and a construction that needs neither
///
/// Table 182 says the four vertices are "in counterclockwise order" and Figure 84 shows
/// (x1, y1) and (x2, y2) as the *top* edge — which is not counterclockwise in a y-up space,
/// and every producer follows the figure. Rather than choose, this takes the clause's one
/// unambiguous sentence: the edge from (x1, y1) to (x2, y2) is the text's direction. The
/// other two vertices are the opposite edge, and which of them pairs with (x1, y1) is decided
/// by projecting them onto that direction — true under either reading.
///
/// Which of the two edges is the *bottom* is then the one open question, and it is answered by
/// the page rather than by the clause: the lower midpoint in default user space, ties going to
/// the second edge, which is Figure 84's arrangement.
///
/// # Why a highlight multiplies
///
/// The clause says these annotations appear "in the text of a document", and a mark that
/// covers the text is not a highlight in it — §12.5 draws an annotation *over* the page.
/// §11.3.5.2 defines exactly one blend mode whose "result colour is always at least as dark as
/// either of the two constituent colours", which is the standard's own guarantee that what was
/// under the wash survives it. So a highlight is its quadrilateral filled under `Multiply`,
/// and the `/ExtGState` that selects it is the constructed stream's only resource.
fn text_markup(
    document: &Document,
    annotation: &Dictionary,
    stream: &mut Stream,
    subtype: &[u8],
) -> Outcome {
    let Some(points) = points(document, annotation, "QuadPoints") else {
        return Err(Refusal::Missing("/QuadPoints"));
    };
    let quads: Vec<Quad> = points.chunks_exact(4).filter_map(Quad::read).collect();
    if quads.is_empty() {
        return Err(Refusal::Missing("/QuadPoints"));
    }

    // Table 166's `/C`, which §12.5.6.10 gives no entry of its own for. A markup with no
    // colour states no mark, which is empty rather than unsupported. §12.5.6.2 makes it a group
    // attribute, and this is the subtype a group's subordinate usually is — a strike-out beside
    // the caret that replaces it.
    let shared = crate::markup::group_source(document, annotation);
    let colour = colour(document, &shared, "C")?;
    if colour == Colour::None {
        return Ok(Painted::EMPTY);
    }

    if subtype == b"Highlight" {
        stream.multiply();
    }
    stream.set_colour(colour, false);
    for quad in &quads {
        match subtype {
            b"Highlight" => quad.outline(stream),
            b"StrikeOut" => quad.bar(stream, 0.5),
            b"Squiggly" => quad.squiggle(stream),
            // Underline: a bar sitting on the bottom edge, inside the quadrilateral.
            _ => quad.bar(stream, 0.0),
        }
    }
    stream.paint(true, false);
    Ok(Painted::DRAWN)
}

/// One of §12.5.6.10's quadrilaterals, in the frame the clause's own sentence defines.
///
/// `origin` is the bottom edge's first vertex, `along` runs to its second, and `up` is the
/// offset from the bottom edge to the top one — so `along` is the text's direction and
/// `up.length()` is the height every fraction below is taken of.
struct Quad {
    origin: [f32; 2],
    along: [f32; 2],
    up: [f32; 2],
}

impl Quad {
    /// Reads one quadrilateral, or `None` if it encloses nothing.
    fn read(vertices: &[[f32; 2]]) -> Option<Self> {
        let (&first, &second) = (vertices.first()?, vertices.get(1)?);
        let (&third, &fourth) = (vertices.get(2)?, vertices.get(3)?);
        let stated = [second[0] - first[0], second[1] - first[1]];

        // The opposite edge, ordered by where each vertex falls along the text's direction,
        // which is what makes this independent of the two readings of "counterclockwise".
        let project = |point: [f32; 2]| {
            (point[0] - first[0]).mul_add(stated[0], (point[1] - first[1]) * stated[1])
        };
        let (near, far) = if project(third) <= project(fourth) {
            (third, fourth)
        } else {
            (fourth, third)
        };

        // Which of the two edges is the bottom: the lower midpoint in default user space,
        // ties going to the second edge, which is Figure 84's arrangement.
        let (origin, along, opposite) = if near[1] + far[1] <= first[1] + second[1] {
            (near, [far[0] - near[0], far[1] - near[1]], first)
        } else {
            (first, stated, near)
        };
        let up = [opposite[0] - origin[0], opposite[1] - origin[1]];
        (along[0] != 0.0 || along[1] != 0.0).then_some(Self { origin, along, up })
    }

    /// The point at `u` along the bottom edge and `v` of the way up.
    fn at(&self, u: f32, v: f32) -> [f32; 2] {
        [
            self.origin[0] + self.along[0] * u + self.up[0] * v,
            self.origin[1] + self.along[1] * u + self.up[1] * v,
        ]
    }

    /// The whole quadrilateral, as a closed subpath.
    fn outline(&self, stream: &mut Stream) {
        stream.move_to(self.at(0.0, 0.0));
        stream.line_to(self.at(1.0, 0.0));
        stream.line_to(self.at(1.0, 1.0));
        stream.line_to(self.at(0.0, 1.0));
        stream.close();
    }

    /// A bar across the quadrilateral, its lower edge `v` of the way up.
    ///
    /// One sixteenth of the height thick; see the note on [`text_markup`] for why that is a
    /// choice and why it is a fraction.
    fn bar(&self, stream: &mut Stream, v: f32) {
        const THICKNESS: f32 = 1.0 / 16.0;
        let low = (v - THICKNESS * 0.5).max(0.0);
        let high = low + THICKNESS;
        stream.move_to(self.at(0.0, low));
        stream.line_to(self.at(1.0, low));
        stream.line_to(self.at(1.0, high));
        stream.line_to(self.at(0.0, high));
        stream.close();
    }

    /// A jagged underline: a zig-zag filled as a ribbon along the bottom edge.
    fn squiggle(&self, stream: &mut Stream) {
        const AMPLITUDE: f32 = 1.0 / 12.0;
        const THICKNESS: f32 = 1.0 / 16.0;
        // A wavelength of a third of the height, rounded to a whole number of periods so the
        // squiggle begins and ends at the quadrilateral's own edges, and bounded so that a
        // long quadrilateral with a tiny height cannot ask for an unbounded path.
        let length = self.along[0].hypot(self.along[1]);
        let height = self.up[0].hypot(self.up[1]).max(f32::MIN_POSITIVE);
        let periods = (length / (height / 3.0)).clamp(1.0, 512.0);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 1.0..=512.0 above, so the conversion is exact"
        )]
        let steps = (periods.round() * 2.0) as u32;

        // Out along the top of the ribbon and back along the bottom, so one filled path
        // describes a stroke of constant thickness without needing one.
        let fraction = |step: u32| f32::from(u16::try_from(step).unwrap_or(u16::MAX));
        let total = fraction(steps).max(1.0);
        let peak = |step: u32| if step % 2 == 1 { AMPLITUDE } else { 0.0 };
        for step in 0..=steps {
            let u = fraction(step) / total;
            let point = self.at(u, peak(step) + THICKNESS);
            if step == 0 {
                stream.move_to(point);
            } else {
                stream.line_to(point);
            }
        }
        for step in (0..=steps).rev() {
            let u = fraction(step) / total;
            stream.line_to(self.at(u, peak(step)));
        }
        stream.close();
    }
}

/// Draws §12.5.6.13's scribble: one stroked path per `/InkList` entry.
///
/// > When drawn, the points shall be connected by straight lines or curves in an
/// > implementation-dependent way.
///
/// Straight lines, then — the clause offers the choice outright, and a curve fitted through a
/// digitiser's own samples would be this module inventing smoothing the document did not ask
/// for.
///
/// A `/Path` (PDF 2.0) is drawn in preference to `/InkList`, and **that is a choice rather
/// than Table 185's rule**: §12.5.6.9's Table 181 makes `/Vertices` "(Required unless a Path
/// key is present, in which case it shall be ignored)" and Table 185 states no such ordering
/// for `/InkList`, which it simply marks "(Required)". A file writing both describes one
/// scribble twice, so the entry that can carry curves wins and the page is not marked twice.
/// `an_ink_annotations_path_is_drawn_and_outranks_its_ink_list` is where that is recorded.
fn ink(document: &Document, annotation: &Dictionary, stream: &mut Stream) -> Outcome {
    // §12.5.6.2: `/C` is a group attribute.
    let shared = crate::markup::group_source(document, annotation);
    let border = Border::read(document, annotation, &shared, "C")?;
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

/// Draws §12.5.6.7's single straight line, with the leader lines Table 178 states.
///
/// `/L` is normally the line's own endpoints. With `/LL` it is not: the entry makes `/L`
/// "represent the endpoints of the leader lines rather than the endpoints of the line itself",
/// and the clause then states exactly where the line goes —
///
/// > The length of leader lines in default user space that extend from each endpoint of the
/// > line perpendicular to the line itself
///
/// — with the sign deciding which side: "[a] positive value shall mean that the leader lines
/// appear in the direction that is clockwise when traversing the line from its starting point
/// to its ending point (as specified by L); a negative value shall indicate the opposite
/// direction." `/LLE` extends each leader "from the line proper 180 degrees from the leader
/// lines", and `/LLO` is "the amount of empty space between the endpoints of the annotation and
/// the beginning of the leader lines". That is a complete construction, and this draws it.
///
/// **It was refused on the true observation that `/L` is not the line** — which is a reason to
/// compute the line rather than a reason to decline, and is the same shape as the text-markup
/// refusal ADR 0043 removed. The refusal also fired on the entry's *presence* rather than on
/// its value, so `annotation-line-without-appearance.pdf`, which states `/LL 0` — Table 178's
/// own "no leader lines" — was declined for asking for nothing.
///
/// One entry is still owed and it states a different kind of nothing: `/LE`'s endings
/// name shapes with no size (Table 179 says "[a] square", "[t]wo short lines meeting in an
/// acute angle" — re-read in the eighty-fifth session and it still states no dimension).
/// **It is named beside the drawn line rather than instead of it**, since the
/// hundred-and-sixteenth session: it is optional and additive where `/L` is required, so
/// declining the whole annotation for it drew nothing where the clause states a line. That
/// is the same reasoning the refusal above records for `/LL`, applied one entry over.
///
/// **`/Cap` stood on that list until the five-hundred-and-seventy-fourth session and is drawn
/// now** — [`caption`] holds the reading, and what it retires is a refusal whose sentence was
/// true and whose inference was not.
fn line(document: &Document, annotation: &Dictionary, stream: &mut Stream) -> Outcome {
    let endings = line_endings(document, annotation)?;
    let ends = points(document, annotation, "L").unwrap_or_default();
    let (Some(start), Some(end)) = (ends.first().copied(), ends.get(1).copied()) else {
        return Err(Refusal::Missing("/L"));
    };
    // §12.5.6.2: `/C` is a group attribute and `/IC` below is not.
    let shared = crate::markup::group_source(document, annotation);
    let border = Border::read(document, annotation, &shared, "C")?;
    if !border.strokes() {
        // An annotation that draws nothing owes nothing: there is no line for a caption to sit
        // on, so naming one would report a gap on a blank page.
        return Ok(Painted::EMPTY);
    }
    border.apply(stream);
    let interior = colour(document, annotation, "IC")?;
    stream.set_colour(interior, false);

    let leader = entry_number(document, annotation, "LL").unwrap_or(0.0);
    // A line of no length has no direction to be perpendicular to, so its leader lines have
    // nowhere to go; the degenerate line itself is still §8.5.3.2's business — and an ending has
    // no direction to point in either, which [`draw_endings`] answers for itself.
    let (proper, leaders) = match perpendicular(start, end).filter(|_| leader != 0.0) {
        None => ([start, end], Vec::new()),
        Some(offset) => {
            // "A non-negative number": a negative `/LLE` or `/LLO` states no length, so it is
            // dropped rather than reflected — the sign the clause gives a meaning to is `/LL`'s.
            let extension = entry_number(document, annotation, "LLE")
                .unwrap_or(0.0)
                .max(0.0);
            let gap = entry_number(document, annotation, "LLO")
                .unwrap_or(0.0)
                .max(0.0);
            let away = if leader < 0.0 { -1.0 } else { 1.0 };
            let along = |point: [f32; 2], distance: f32| {
                [
                    distance.mul_add(offset[0], point[0]),
                    distance.mul_add(offset[1], point[1]),
                ]
            };
            // Each leader: from the offset the annotation states to the line, and `/LLE` past it.
            let leaders = [start, end]
                .map(|point| {
                    [
                        along(point, away * gap),
                        along(point, away.mul_add(extension, leader)),
                    ]
                })
                .to_vec();
            // The line proper, at the leader lines' far end.
            ([along(start, leader), along(end, leader)], leaders)
        }
    };

    // Read before the line is stroked rather than after, because Figure 81's inline caption is
    // drawn in a *break* in the line and where that break goes is what the layout answers.
    let caption = caption(document, annotation, proper, &border);
    for [from, to] in caption.segments(proper).into_iter().chain(leaders) {
        stream.move_to(from);
        stream.line_to(to);
    }
    stream.paint(false, true);
    // **On the line proper's ends, not on `/L`'s**, where the two differ. Table 178 says `/LE`
    // states the styles "for the endpoints defined … by the first and second pairs of
    // coordinates … in the L array", and says four rows earlier that with `/LL` present those
    // coordinates are "the endpoints of the leader lines rather than the endpoints of the line
    // itself". So the endpoints the first sentence names are not on the line; Figure 80 draws
    // the arrowhead on the line proper, and an ending is an ending *of a line*.
    draw_endings(stream, endings, proper, border.width, interior);
    if let Some(mark) = caption.drawn {
        mark.write(stream);
    }
    Ok(Painted {
        drawn: true,
        report: caption.owed,
    })
}

/// Table 178's `/Cap`: the annotation's own text, replicated as a caption on the line.
///
/// # What the table states, and what it does not
///
/// ISO 32000-2 §12.5.6.7, Table 178, on the entry:
///
/// > If true , the text specified by the Contents or RC entries shall be replicated as a caption
/// > in the appearance of the line, as shown in "Figure 81 - Lines with captions appearing as
/// > part of the line" and "Figure 82 - Line with a caption appearing as part of the offset". The
/// > text shall be rendered in a manner appropriate to the content, taking into account factors
/// > such as writing direction. Default value: false .
///
/// That is a **`shall`** about the appearance, and the two entries beside it state where the
/// caption goes with no room left over. `/CP` names the placement — "Valid values are Inline ,
/// meaning the caption shall be centred inside the line, and Top , meaning the caption shall be
/// on top of the line. Default value: Inline " — and `/CO` states the offset in the line's own
/// axes:
///
/// > The first value shall be the horizontal offset along the annotation line from its midpoint,
/// > with a positive value indicating offset to the right and a negative value indicating offset
/// > to the left. The second value shall be the vertical offset perpendicular to the annotation
/// > line, with a positive value indicating a shift up and a negative value indicating a shift
/// > down.
///
/// **`/CO`'s own wording is what says the caption is set along the line rather than along the
/// page**: an offset measured "along the annotation line" and "perpendicular to the annotation
/// line" is a statement in a frame whose axes are the line's, and the caption is placed from its
/// midpoint in that frame. So the position is the clause's, to the point.
///
/// # The refusal this replaces, and why it was wrong
///
/// Until the five-hundred-and-seventy-fourth session this entry was refused whole, on the
/// sentence *"§12.5.6.7's /Cap asks for /Contents as a caption, and no entry gives it a font"*.
/// The sentence is true — no entry of a line annotation is a `/DA`, and Table 172 gives a markup
/// annotation none either — and the inference from it is not, which is ADR 0109's rule: the
/// question a silence poses is not *may I fill this* but *does a sentence around it require me
/// to*, and the `shall` quoted above does. It is the same shape §12.7.5.4's list box was refused
/// on one round earlier (ADR 0407): a true observation about what the clause leaves open, taken
/// as a reason to draw nothing where the same clause states a mark outright.
///
/// # The two choices, and what the clause's own figure settled
///
/// - **The face.** §9.6.2.2's Helvetica, from this binary, which is exactly what
///   `variable_text`'s stand-in already draws with where §12.7.4.3's `/DR` defines no font
///   (ADR 0112). What it buys is ADR 0133's argument: the caption is drawn from the binary
///   rather than from whatever face this machine happens to have installed, so the page
///   reproduces where no fonts are.
/// - **The size**, [`CAPTION_SIZE`], which is the one number the standard does not state here
///   and is taken from the one worked example it does state.
///
/// **A first attempt auto-sized the caption to the line's own length**, on the reasoning that
/// "centred inside the line" and "on top of the line" make the line's length the only extent the
/// clause gives the caption. Figure 81 — which the entry cites by name — refutes it: its third
/// example is captioned *This is a caption that is longer than the line* and is drawn at the same
/// size as the other two, overhanging both ends. So the size does not depend on the line, and the
/// figure settles two more things the sentence leaves open: **an inline caption sits in a break in
/// the line** rather than over it (the figure draws the line in two pieces either side of the
/// words), and the caption is the same colour as the line.
///
/// That colour is the annotation's `/C`, which is what the line itself is stroked in: the clause
/// makes the caption part of "the appearance of the line", and Table 178 reserves `/IC` for "the
/// annotation's line endings". A line with no `/C` strokes nothing and never reaches here.
fn caption(
    document: &Document,
    annotation: &Dictionary,
    ends: [[f32; 2]; 2],
    border: &Border,
) -> Caption {
    if !matches!(document.get_key(annotation, "Cap"), Object::Boolean(true)) {
        return Caption::NONE;
    }
    // §12.5.6.2 makes `/Contents` and `/RC` group attributes, so a captioned line that is a
    // group's subordinate replicates the primary's words. The order is [`free_text_layout`]'s
    // and for its reason: the table names "the Contents or RC entries" without ranking them, and
    // `/Contents` is the plain text a reader typed.
    let shared = crate::markup::group_source(document, annotation);
    let text = variable_text::string(document, &[&shared], "Contents")
        .filter(|contents| !contents.is_empty())
        .or_else(|| crate::popup::rich_text(document, &shared))
        .unwrap_or_default();
    if text.is_empty() {
        // `/Cap` replicates "the text specified by the Contents or RC entries" and this
        // annotation specifies none, so there is nothing the clause asks for and nothing owed.
        return Caption::NONE;
    }
    let Some(placement) = CaptionPlacement::read(document, annotation) else {
        return Caption::owing(UNKNOWN_CAPTION_PLACEMENT);
    };
    let Some(offset) = caption_offset(document, annotation) else {
        return Caption::owing(CAPTION_OFFSET_SHAPE);
    };

    let (dx, dy) = (ends[1][0] - ends[0][0], ends[1][1] - ends[0][1]);
    // ADR 0189: the two backends must agree about every number a shape is built from, so a
    // length here is the IEEE operations and nothing else.
    let length = dx.mul_add(dx, dy * dy).sqrt();
    if !(length.is_finite() && length > 0.0) {
        return Caption::owing(CAPTION_WITHOUT_A_LINE);
    }
    // **Which way along the line the text reads is a choice**, and it is the narrowest one
    // available: the clause fixes the axis and says nothing about the sense, and a line drawn
    // right to left would otherwise carry its caption upside down. So the sense that leaves the
    // text the right way up on the page is taken, which is the line's own direction unless that
    // runs leftwards.
    let sense = if dx < 0.0 { -1.0 } else { 1.0 };
    let forward = [sense * dx / length, sense * dy / length];
    // A quarter turn counterclockwise from the reading direction, which is "up" for the text and
    // therefore what `/CO`'s second number and `/CP`'s `Top` are measured against.
    let up = [-forward[1], forward[0]];
    let middle = [
        (ends[0][0] + ends[1][0]) * 0.5,
        (ends[0][1] + ends[1][1]) * 0.5,
    ];
    let origin = [
        offset[1].mul_add(up[0], offset[0].mul_add(forward[0], middle[0])),
        offset[1].mul_add(up[1], offset[0].mul_add(forward[1], middle[1])),
    ];

    let resources = Dictionary::new();
    // `/Helv` is one of `variable_text`'s fourteen standard abbreviations, so the layout resolves
    // it to §9.6.2.2's Helvetica out of this binary and owes no report for having done so.
    let default_appearance = format!("/Helv {CAPTION_SIZE} Tf");
    let height = CAPTION_SIZE * variable_text::LINE_HEIGHT;
    let lay_out = |box_: [f32; 4]| {
        variable_text::lay_out(
            document,
            &Request {
                text: &text,
                box_,
                default_appearance: default_appearance.as_bytes(),
                resources: &resources,
                // Both of `/CP`'s values centre the caption on the line, so the quadding is not the
                // document's to state and is not read from one: Table 228's `/Q` belongs to variable
                // text and Table 178 has no such entry.
                quadding: Quadding::Centred,
                // Table 178 states one caption rather than a paragraph, and `/CO` places it from a
                // single midpoint.
                shape: Shape::SingleLine,
                asked: Asked::default(),
            },
        )
    };
    // **Two passes, and the first draws nothing.** §12.7.4.3's layout clips to the box it is
    // given, and the box that does not clip this caption is the caption's own width — which is
    // not known until it has been laid out. So the first pass is a measurement, taken from the
    // advances the glyphs are positioned by rather than from a second opinion about them, and
    // the second is the one whose stream is kept.
    let measured = match lay_out([0.0, 0.0, 0.0, height]) {
        Ok(laid_out) => laid_out.advance,
        Err(owed) => return Caption::owing(Refusal::Text(owed)),
    };
    let half = measured * 0.5;
    let box_ = match placement {
        // "the caption shall be centred inside the line": the band is centred on the line.
        CaptionPlacement::Inline => [-half, -height * 0.5, half, height * 0.5],
        // "the caption shall be on top of the line": the band sits on it.
        CaptionPlacement::Top => [-half, 0.0, half, height],
    };
    let laid_out = match lay_out(box_) {
        Ok(laid_out) => laid_out,
        Err(owed) => return Caption::owing(Refusal::Text(owed)),
    };
    Caption {
        drawn: Some(CaptionMark {
            content: laid_out.content,
            font: laid_out.font,
            colour: border.colour,
            axes: [forward[0], forward[1], up[0], up[1]],
            origin,
            // Figure 81 draws an inline caption with the line broken around it, and a `Top` one
            // over an unbroken line. The break is the caption's own extent, projected back onto
            // the line: the first pass measured it and `/CO`'s first number is where its centre
            // sits along the line from the midpoint.
            //
            // **And a `/CO` that lifts the caption clear of the line takes the break with it**,
            // which is Figure 82's case — the entry offsets the caption "from its normal
            // position", and the break exists because the words occupy that stretch of the line.
            // Where they no longer do, nothing is in the line's way.
            break_: match placement {
                CaptionPlacement::Inline if offset[1].abs() < height * 0.5 => {
                    let from = length * 0.5 + sense * (offset[0] - half);
                    let to = length * 0.5 + sense * (offset[0] + half);
                    Some((from.min(to), from.max(to)))
                }
                CaptionPlacement::Inline | CaptionPlacement::Top => None,
            },
        }),
        owed: laid_out.owed.map(Refusal::Text),
    }
}

/// What [`caption`] made of Table 178's `/Cap`, and what it could not.
#[derive(Default)]
struct Caption {
    drawn: Option<CaptionMark>,
    owed: Option<Refusal>,
}

impl Caption {
    /// The annotation asks for no caption, which is every line but the ones that set `/Cap`.
    const NONE: Self = Self {
        drawn: None,
        owed: None,
    };

    /// A caption the clause asks for and this construction cannot place.
    fn owing(refusal: Refusal) -> Self {
        Self {
            drawn: None,
            owed: Some(refusal),
        }
    }

    /// The pieces of the line proper that survive an inline caption's break.
    ///
    /// §12.5.6.7 makes the line required — "[t]he purpose of a line annotation … is to display a
    /// single straight line on the page" — so a break that would leave nothing is not taken at
    /// all: a caption wider than the line it sits on is Figure 81's third example, where the line
    /// is drawn whole under the overhanging words.
    fn segments(&self, ends: [[f32; 2]; 2]) -> Vec<[[f32; 2]; 2]> {
        let whole = vec![ends];
        let Some((from, to)) = self.drawn.as_ref().and_then(|mark| mark.break_) else {
            return whole;
        };
        let (dx, dy) = (ends[1][0] - ends[0][0], ends[1][1] - ends[0][1]);
        let length = dx.mul_add(dx, dy * dy).sqrt();
        if !(length.is_finite() && length > 0.0) {
            return whole;
        }
        let direction = [dx / length, dy / length];
        let at = |distance: f32| {
            [
                distance.mul_add(direction[0], ends[0][0]),
                distance.mul_add(direction[1], ends[0][1]),
            ]
        };
        let mut pieces = Vec::with_capacity(2);
        if from > 0.0 {
            pieces.push([ends[0], at(from.min(length))]);
        }
        if to < length {
            pieces.push([at(to.max(0.0)), ends[1]]);
        }
        if pieces.is_empty() { whole } else { pieces }
    }
}

/// One caption, ready to be written after the line it belongs to.
struct CaptionMark {
    /// §12.7.4.3's marked-content section, as [`variable_text::lay_out`] wrote it.
    content: String,
    /// The font dictionary the layout invented, for the appearance's `/Resources`.
    font: Option<(pdf_syntax::Name, Dictionary)>,
    /// Table 166's `/C`, which is what the line is stroked in.
    colour: Colour,
    /// The reading direction and the perpendicular, as the first four operands of a `cm`.
    axes: [f32; 4],
    /// Where the caption's own origin sits on the page, `/CO` already applied.
    origin: [f32; 2],
    /// The interval of the line the caption occupies, measured from the first endpoint, where
    /// `/CP` asks for the break Figure 81 draws.
    break_: Option<(f32, f32)>,
}

impl CaptionMark {
    /// Writes the caption into the appearance, in the line's own axes.
    ///
    /// A `cm` rather than a `/Matrix`: a constructed appearance is written in the page's own
    /// space, so §12.5.5's placement reduces to the identity and there is nowhere else to put the
    /// turn — the same reason [`Rotation::begin`] writes one for Table 192's `/R`.
    fn write(self, stream: &mut Stream) {
        let [a, b, c, d] = self.axes;
        stream.rotate(&format!("{a} {b} {c} {d}"), self.origin);
        stream.set_colour(self.colour, false);
        stream.text.push_str(&self.content);
        stream.text.push_str("Q\n");
        let resources = stream.resources.take().unwrap_or_default();
        stream.resources = Some(with_stand_in_font(resources, self.font));
    }
}

/// Table 178's `/CP`, which decides where on the line the caption sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptionPlacement {
    /// "Inline , meaning the caption shall be centred inside the line", and the table's default.
    Inline,
    /// "Top , meaning the caption shall be on top of the line".
    Top,
}

impl CaptionPlacement {
    /// Reads `/CP`, or `None` for a value outside the two the table defines.
    ///
    /// Table 178 lists exactly two valid values and states no tolerance for a third, unlike Table
    /// 168's border styles ("[a]n interactive PDF processor shall tolerate other border styles
    /// that it does not recognise"). A name outside the pair therefore states *no* position, and
    /// drawing the caption at the default would put a mark where the file did not ask for one —
    /// which is [`UNKNOWN_LINE_ENDING`]'s reasoning on the entry two rows above.
    fn read(document: &Document, annotation: &Dictionary) -> Option<Self> {
        match document.get_key(annotation, "CP") {
            Object::Null => Some(Self::Inline),
            Object::Name(name) => match name.as_bytes() {
                b"Inline" => Some(Self::Inline),
                b"Top" => Some(Self::Top),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Table 178's `/CO`, "[a]n array of two numbers", defaulting to no offset.
///
/// `None` for an entry that is present and is not two numbers: the table gives the offset its
/// meaning one number at a time, so an array of some other length says which of its numbers is
/// the horizontal one no more than `/CL`'s does (see [`CALLOUT_SHAPE`]).
fn caption_offset(document: &Document, annotation: &Dictionary) -> Option<[f32; 2]> {
    let entry = document.get_key(annotation, "CO");
    if matches!(entry, Object::Null) {
        // "Default value: [0, 0] (no offset from normal positioning)".
        return Some([0.0, 0.0]);
    }
    let values = entry
        .as_array()
        .and_then(|values| numbers(document, values))?;
    match values[..] {
        [horizontal, vertical] => Some([horizontal, vertical]),
        _ => None,
    }
}

/// Draws both of Table 179's endings on a two-ended line.
///
/// The outward direction at each end is the line's own, away from the other end — see
/// [`draw_ending`] for what "outward" is deciding.
fn draw_endings(
    stream: &mut Stream,
    endings: [Ending; 2],
    ends: [[f32; 2]; 2],
    width: f32,
    interior: Colour,
) {
    let [start, end] = ends;
    let (dx, dy) = (end[0] - start[0], end[1] - start[1]);
    // ADR 0189: `hypot` is not correctly rounded and the two backends must agree about every
    // number a shape is built from, so a length here is the IEEE operations and nothing else.
    let length = dx.mul_add(dx, dy * dy).sqrt();
    if !(length.is_finite() && length > 0.0) {
        return;
    }
    let forward = [dx / length, dy / length];
    let backward = [-forward[0], -forward[1]];
    draw_ending(stream, endings[0], start, backward, width, interior);
    draw_ending(stream, endings[1], end, forward, width, interior);
}

/// Table 179's ten line ending styles. ISO 32000-2 §12.5.6.7, of Table 178's `/LE`:
///
/// > The first and second elements of the array shall specify the line ending styles for the
/// > endpoints defined, respectively, by the first and second pairs of coordinates, ( x 1 , y 1 )
/// > and ( x 2 , y 2 ), in the L array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ending {
    /// "No line ending", Table 179's default for both ends.
    None,
    /// "A square filled with the annotation's interior colour, if any".
    Square,
    /// "A circle filled with the annotation's interior colour, if any".
    Circle,
    /// "A diamond shape filled with the annotation's interior colour, if any".
    Diamond,
    /// "Two short lines meeting in an acute angle to form an open arrowhead".
    OpenArrow,
    /// The same "connected by a third line to form a triangular closed arrowhead filled with the
    /// annotation's interior colour, if any".
    ClosedArrow,
    /// "Two short lines in the reverse direction from `OpenArrow`".
    ReverseOpenArrow,
    /// "A triangular closed arrowhead in the reverse direction from `ClosedArrow`".
    ReverseClosedArrow,
    /// "A short line at the endpoint perpendicular to the line itself".
    Butt,
    /// "A short line at the endpoint approximately 30 degrees clockwise from perpendicular to the
    /// line itself".
    Slash,
}

impl Ending {
    /// Table 179's names, and `None` for a name the table does not have.
    ///
    /// An unknown name is not the default: Table 178 makes `[/None /None]` the value of an
    /// *absent* `/LE`, and a name outside the table is a file saying something this reader cannot
    /// read. It is reported rather than silently drawn as nothing — trap 5, which is why this
    /// returns an `Option` where [`Self::None`] already exists.
    fn read(name: &[u8]) -> Option<Self> {
        Some(match name {
            b"None" => Self::None,
            b"Square" => Self::Square,
            b"Circle" => Self::Circle,
            b"Diamond" => Self::Diamond,
            b"OpenArrow" => Self::OpenArrow,
            b"ClosedArrow" => Self::ClosedArrow,
            b"ROpenArrow" => Self::ReverseOpenArrow,
            b"RClosedArrow" => Self::ReverseClosedArrow,
            b"Butt" => Self::Butt,
            b"Slash" => Self::Slash,
            _ => return None,
        })
    }

    /// Whether Table 179 fills this shape with "the annotation's interior colour, if any".
    ///
    /// Four of the ten say so and the other six do not, which is the whole of the difference an
    /// `/IC` makes here — and for a polyline it is the only thing `/IC` does at all (Table 181:
    /// "[f]or Polyline annotations, the value of the IC key is used to fill only the line
    /// ending").
    fn filled(self) -> bool {
        matches!(
            self,
            Self::Square
                | Self::Circle
                | Self::Diamond
                | Self::ClosedArrow
                | Self::ReverseClosedArrow
        )
    }
}

/// How large a line ending is, as a multiple of the line's own width.
///
/// **The clause states no size and this is a choice, in the same place §12.5.6.10's thickness is
/// one**: Table 179 describes ten shapes — "[a] square", "[t]wo short lines meeting in an acute
/// angle", "[a] short line at the endpoint perpendicular to the line itself" — and gives not one
/// dimension. The only length a line annotation supplies its ending is §12.5.4's border width,
/// which is what draws the line the ending decorates, so the size is stated as a multiple of it
/// and is therefore right at every scale rather than right at one. Four widths is about what a
/// drawing program's arrowhead is, and nothing derives it.
const ENDING_SIZE: f32 = 4.0;

/// Half the angle at an arrowhead's apex, in radians.
///
/// Table 179 says "an acute angle" and no more, so this is a choice bounded by the word: 60° at
/// the apex is acute, and it is the angle every arrowhead in ordinary type is drawn at.
const ARROW_HALF_ANGLE: f32 = std::f32::consts::FRAC_PI_6;

/// Table 179's own "approximately 30 degrees clockwise from perpendicular", for `Slash`.
const SLASH_ANGLE: f32 = std::f32::consts::FRAC_PI_6;

/// Draws one of Table 179's endings at a line's endpoint.
///
/// `out` is the unit vector pointing **away from the line** at this end, which is the one thing
/// the table does not say and every reader has to decide: an `OpenArrow` at (x1, y1) points along
/// it, so a line with arrows at both ends is an arrow at each end pointing outwards, and the two
/// reverse styles are Table 179's own way of asking for the other direction ("in the reverse
/// direction from `OpenArrow`"). Recorded as a choice.
///
/// The stroking colour and width are already the line's; `interior` is Table 178's `/IC`, and a
/// shape the table does not fill is stroked alone.
fn draw_ending(
    stream: &mut Stream,
    ending: Ending,
    at: [f32; 2],
    out: [f32; 2],
    width: f32,
    interior: Colour,
) {
    let size = width * ENDING_SIZE;
    if ending == Ending::None || !(size.is_finite() && size > 0.0) {
        return;
    }
    // The line's own direction and the perpendicular to it, both unit length: everything below is
    // written in that frame so that no shape has to know the line's angle.
    let (ux, uy) = (out[0], out[1]);
    let (vx, vy) = (-uy, ux);
    let point = |along: f32, across: f32| {
        [
            along.mul_add(ux, across.mul_add(vx, at[0])),
            along.mul_add(uy, across.mul_add(vy, at[1])),
        ]
    };
    let half = size * 0.5;
    let fill = ending.filled() && interior != Colour::None;
    match ending {
        Ending::None => {}
        Ending::Square => {
            stream.move_to(point(-half, -half));
            stream.line_to(point(half, -half));
            stream.line_to(point(half, half));
            stream.line_to(point(-half, half));
            stream.close();
            stream.paint(fill, true);
        }
        Ending::Circle => {
            stream.circle(at, half);
            stream.paint(fill, true);
        }
        Ending::Diamond => {
            stream.move_to(point(-half, 0.0));
            stream.line_to(point(0.0, -half));
            stream.line_to(point(half, 0.0));
            stream.line_to(point(0.0, half));
            stream.close();
            stream.paint(fill, true);
        }
        Ending::Butt => {
            stream.move_to(point(0.0, -half));
            stream.line_to(point(0.0, half));
            stream.paint(false, true);
        }
        Ending::Slash => {
            // "Approximately 30 degrees clockwise from perpendicular": in this y-up space a
            // clockwise turn takes the perpendicular towards the line's own backward direction.
            let (sin, cos) = SLASH_ANGLE.sin_cos();
            stream.move_to(point(-half * sin, -half * cos));
            stream.line_to(point(half * sin, half * cos));
            stream.paint(false, true);
        }
        Ending::OpenArrow
        | Ending::ClosedArrow
        | Ending::ReverseOpenArrow
        | Ending::ReverseClosedArrow => {
            let reverse = matches!(
                ending,
                Ending::ReverseOpenArrow | Ending::ReverseClosedArrow
            );
            let closed = matches!(ending, Ending::ClosedArrow | Ending::ReverseClosedArrow);
            let direction = if reverse { -1.0 } else { 1.0 };
            let (sin, cos) = ARROW_HALF_ANGLE.sin_cos();
            // The apex sits on the endpoint and the two barbs run back from it, so the whole
            // arrowhead is inside the line's own end rather than beyond it.
            let back = -direction * size * cos;
            let spread = size * sin;
            stream.move_to(point(back, spread));
            stream.line_to(at);
            stream.line_to(point(back, -spread));
            if closed {
                stream.close();
            }
            stream.paint(closed && interior != Colour::None, true);
        }
    }
}

/// The unit vector §12.5.6.7 calls clockwise from `start` to `end`, or `None` for no direction.
///
/// This space is PDF's y-up default user space, so a clockwise quarter turn takes `(dx, dy)` to
/// `(dy, -dx)` — the same sign question §7.7.3.3's `/Rotate` asks, and the one a reader working
/// in a y-down raster gets backwards.
fn perpendicular(start: [f32; 2], end: [f32; 2]) -> Option<[f32; 2]> {
    let (dx, dy) = (end[0] - start[0], end[1] - start[1]);
    let length = dx.hypot(dy);
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some([dy / length, -dx / length])
}

/// Draws §12.5.6.19's widget: Table 192's background and border, then §12.7.4.3's text.
///
/// Table 192's `/BG` is "the colour of the widget annotation's background" and `/BC` "the
/// colour of the widget annotation's border"; the width and style are §12.5.4's, as for any
/// annotation. Over that goes whatever the field type says the widget shows — a text field's
/// value, a choice field's selection, a button's caption — laid out by
/// [`crate::variable_text`].
///
/// Table 192's `/R` is the one entry a glyph makes load-bearing: it rotates the widget's
/// *contents* inside `/Rect`, which a background filling that rectangle cannot see but a line of
/// text can. **This comment said it was "read nowhere yet" and had been false since the
/// hundred-and-fifth session**, which is where [`Rotation`] arrived and where §12.5.6.19's ledger
/// row has said it is read and applied ever since; found by `doc/todo/02` §4's sweep run over
/// `crates/` as that section asks. What is refused rather than applied is a value the table
/// forbids — "[t]he value shall be a multiple of 90" — and that is named rather than rounded.
fn widget(
    document: &Document,
    annotation: &Dictionary,
    stream: &mut Stream,
    value: FieldValue<'_>,
) -> Outcome {
    let rect = rectangle(document, annotation)?;
    let characteristics = document.get_key(annotation, "MK").as_dict().cloned();
    let source = characteristics.as_ref().unwrap_or(annotation);
    let background = colour(document, source, "BG")?;
    let border = Border::read(document, annotation, source, "BC")?;

    // §12.5.6.19's Table 192: "[t]he number of degrees by which the widget annotation shall be
    // rotated counterclockwise relative to the page. The value shall be a multiple of 90."
    let Some(rotation) = Rotation::read(document, source) else {
        return Err(Refusal::NotDerivable(
            "Table 192's /R is not a multiple of 90, which the table requires",
        ));
    };
    // Everything below is drawn in the *widget's* own axes, in a box at the origin, and the
    // transform below puts that box onto `/Rect`. For a quarter turn the box's sides are `/Rect`'s
    // swapped, which is the whole of what rotating a rectangle by 90° does to it — so a field
    // that is tall on the page is laid out wide, and §12.7.4.3's wrapping and auto-sizing see the
    // width the text actually has.
    rotation.begin(stream, rect);
    let rect = rotation.content_box(rect);

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

    // §12.5.4 has the border "drawn completely inside the annotation rectangle", so the part
    // of the rectangle the border does not cover is it inset by the whole width — which is
    // where text can go without being struck through by its own frame. Nothing states a
    // further margin and none is added.
    let inner = inset(rect, border.width);

    // Table 192's push-button half: the icon, its fit and where the caption goes. Reached only
    // where the `/MK` states one of the entries, so the 807 corpus widgets that are not
    // push-buttons pay two dictionary lookups rather than a second walk of §12.7.4.1's chain.
    let button = match push_button_icon(document, annotation, source, value, rect, inner, stream) {
        Ok(button) => button,
        Err(refusal) => {
            rotation.end(stream);
            return Ok(Painted {
                drawn: frame,
                report: Some(refusal),
            });
        }
    };

    if !button.draws_caption {
        rotation.end(stream);
        return Ok(Painted {
            drawn: frame || button.drawn,
            report: button.report.or(border.simulated().report),
        });
    }

    let laid_out = match field_text(document, annotation, source, inner, value, Asked::default()) {
        Ok(laid_out) => laid_out,
        Err(refusal) => {
            rotation.end(stream);
            return Ok(Painted {
                drawn: frame || button.drawn,
                report: Some(refusal),
            });
        }
    };
    let Some(laid_out) = laid_out else {
        rotation.end(stream);
        let report = button.report.or(border.simulated().report);
        return Ok(if frame || button.drawn || report.is_some() {
            Painted {
                drawn: frame || button.drawn,
                report,
            }
        } else {
            Painted::EMPTY
        });
    };

    stream.text.push_str(&laid_out.content);
    stream.resources = Some(with_stand_in_font(
        default_resources(document),
        laid_out.font,
    ));
    rotation.end(stream);
    Ok(Painted {
        drawn: true,
        report: laid_out
            .owed
            .map(Refusal::Text)
            .or(button.report)
            .or(border.simulated().report),
    })
}

/// What Table 192's push-button entries decided, for the caller that draws the rest of a widget.
struct ButtonIcon {
    /// Whether an icon reached the stream.
    drawn: bool,
    /// Whether `/TP` leaves the caption to be laid out at all.
    draws_caption: bool,
    /// What the clause states and this did not draw.
    report: Option<Refusal>,
}

impl ButtonIcon {
    /// Every widget that is not a push-button, and every push-button stating none of the entries.
    const NONE: Self = Self {
        drawn: false,
        draws_caption: true,
        report: None,
    };
}

/// Draws Table 192's `/I` where a push-button states one, and says what `/TP` did with it.
///
/// §12.5.6.19's Table 191 is what makes these entries reachable at all. Its `/MK` is an
/// appearance characteristics dictionary
///
/// > that shall be used in constructing a dynamic appearance stream specifying the annotation's
/// > visual presentation on the page.
///
/// so a widget that states its own `/AP` is drawn from that by §12.5.5 and never arrives here —
/// 33 of the corpus's 42 push-buttons, counted by `examples/push_button_census`.
///
/// The seven entries that were unread before this divide three ways, and the census is what
/// decides which of them is work rather than a clause with no witness:
///
/// - `/I` and `/IF` are drawn. Both are fully stated: Table 192 gives the icon as a form `XObject`
///   and Table 250 gives every rule for fitting it, with a default for every entry.
/// - `/TP` decides which of the icon and the caption is drawn, for the three codes that say so
///   without stating a proportion; the other four are reported by [`CaptionPosition::Beside`].
/// - `/RI`, `/IX`, `/RC` and `/AC` are the rollover and down states. Each is defined by what the
///   *pointer* is doing — "when the user rolls the cursor into its active area", "when the mouse
///   button is pressed" — and a constructed appearance is one stream rather than the three
///   §12.5.5's `/N`, `/R` and `/D` subdictionaries hold, so a file stating them is told so. **No
///   corpus document states any of the four**, over all 974.
fn push_button_icon(
    document: &Document,
    annotation: &Dictionary,
    characteristics: &Dictionary,
    value: FieldValue<'_>,
    rect: [f32; 4],
    inner: [f32; 4],
    stream: &mut Stream,
) -> Result<ButtonIcon, Refusal> {
    const ENTRIES: [&str; 7] = ["I", "RI", "IX", "IF", "TP", "RC", "AC"];
    if !ENTRIES
        .iter()
        .any(|key| !matches!(characteristics.get(key), None | Some(Object::Null)))
    {
        return Ok(ButtonIcon::NONE);
    }
    // Table 192 marks all seven "push-button fields only", and §12.7.5.2.2's push-button is
    // Table 229 bit 17 on a `/Btn` — read up §12.7.4.1's `/Parent` chain, because the flags are
    // inheritable and a widget in a field hierarchy states none of its own.
    let field = Field::read(document, annotation, value);
    if !matches!(field.kind, Some(FieldKind::Button { toggling: false })) {
        return Ok(ButtonIcon::NONE);
    }

    let Some(position) = CaptionPosition::read(document, characteristics) else {
        return Err(Refusal::NotDerivable(
            "Table 192's /TP is outside the seven codes the table defines",
        ));
    };

    // The rollover and down states, named where the file states one. They are a report rather
    // than a refusal: the normal icon and caption below are what a still frame shows, and
    // dropping those to say the other two were not built would lose the mark the clause states.
    let interactive = ["RI", "IX", "RC", "AC"]
        .into_iter()
        .find(|key| !matches!(characteristics.get(key), None | Some(Object::Null)));
    let report = interactive.map(|key| match key {
        "RI" => Refusal::NotDerivable(
            "Table 192's /RI is the icon for a cursor rolled into the widget, and a constructed \
             appearance is one stream rather than §12.5.5's three",
        ),
        "IX" => Refusal::NotDerivable(
            "Table 192's /IX is the icon for a pressed mouse button, and a constructed \
             appearance is one stream rather than §12.5.5's three",
        ),
        "RC" => Refusal::NotDerivable(
            "Table 192's /RC is the caption for a cursor rolled into the widget, and a \
             constructed appearance is one stream rather than §12.5.5's three",
        ),
        _ => Refusal::NotDerivable(
            "Table 192's /AC is the caption for a pressed mouse button, and a constructed \
             appearance is one stream rather than §12.5.5's three",
        ),
    });
    let report = match position {
        CaptionPosition::Beside(code) => Some(Refusal::CaptionBeside(code)).or(report),
        _ => report,
    };

    if !position.draws_icon() {
        return Ok(ButtonIcon {
            drawn: false,
            draws_caption: position.draws_caption(),
            report,
        });
    }

    let Some(icon) = Icon::read(document, characteristics) else {
        return Ok(ButtonIcon {
            drawn: false,
            draws_caption: position.draws_caption(),
            report: report.or(
                matches!(characteristics.get("I"), None | Some(Object::Null))
                    .then_some(Refusal::Missing("/I"))
                    .or(Some(Refusal::NotDerivable(
                        "Table 192's /I names no form XObject with a §8.10.2 /BBox",
                    ))),
            ),
        });
    };

    let fit = IconFit::read(document, characteristics);
    // `/FB`: "the button appearance shall be scaled to fit fully within the bounds of the
    // annotation without taking into consideration the line width of the border" — so the
    // target is `/Rect` itself rather than `/Rect` inset by §12.5.4's border.
    let target = if fit.ignore_border { rect } else { inner };
    let Some(placement) = fit.place(icon.extent, target) else {
        return Ok(ButtonIcon {
            drawn: false,
            draws_caption: position.draws_caption(),
            report: report.or(Some(Refusal::NotDerivable(
                "Table 192's /I has a /BBox of no area, so Table 250 has nothing to fit",
            ))),
        });
    };
    stream.form(document, icon.reference, placement);
    Ok(ButtonIcon {
        drawn: true,
        draws_caption: position.draws_caption(),
        report,
    })
}

/// Table 192's `/TP`: where a push-button's caption sits relative to its icon.
///
/// §12.5.6.19, Table 192:
///
/// > A code indicating where to position the text of the widget annotation's caption relative to
/// > its icon: 0 No icon; caption only 1 No caption; icon only 2 Caption below the icon 3 Caption
/// > above the icon 4 Caption to the right of the icon 5 Caption to the left of the icon 6 Caption
/// > overlaid directly on the icon Default value: 0 .
///
/// **Three of the seven codes are carried out and four are named, and the split is the standard's
/// own.** Codes 0, 1 and 6 each say which of the two things is drawn and give both of them the
/// whole rectangle; codes 2 to 5 say which *side* the caption is on and state nothing about how
/// much of the rectangle it takes. Choosing that proportion would be inventing a layout the
/// document did not ask for — the same refusal §12.5.6.12's stamp legends get — so those four are
/// reported by name with the icon drawn and the caption left off, which is the half the clause
/// does state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptionPosition {
    /// 0, the table's default: the caption is drawn and the icon is not.
    CaptionOnly,
    /// 1: the icon is drawn and the caption is not.
    IconOnly,
    /// 6: both, each over the whole rectangle.
    Overlaid,
    /// 2, 3, 4 and 5, which name a side and no size. The code is kept for the report.
    Beside(i64),
}

impl CaptionPosition {
    /// Table 192's `/TP`, or `None` for a value outside the seven codes it defines.
    fn read(document: &Document, characteristics: &Dictionary) -> Option<Self> {
        match document.get_key(characteristics, "TP") {
            Object::Null | Object::Integer(0) => Some(Self::CaptionOnly),
            Object::Integer(1) => Some(Self::IconOnly),
            Object::Integer(6) => Some(Self::Overlaid),
            Object::Integer(code @ 2..=5) => Some(Self::Beside(code)),
            _ => None,
        }
    }

    /// Whether the caption is drawn at all under this code.
    fn draws_caption(self) -> bool {
        matches!(self, Self::CaptionOnly | Self::Overlaid)
    }

    /// Whether the icon is drawn at all under this code.
    fn draws_icon(self) -> bool {
        !matches!(self, Self::CaptionOnly)
    }
}

/// Table 250's `/SW`: when the icon is scaled into the rectangle at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ScaleWhen {
    /// `A`, the table's default: "Always scale."
    #[default]
    Always,
    /// `B`: "Scale only when the icon is bigger than the annotation rectangle."
    Bigger,
    /// `S`: "Scale only when the icon is smaller than the annotation rectangle."
    Smaller,
    /// `N`: "Never scale."
    Never,
}

/// Table 250's `/S`: which of the two scalings is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Scaling {
    /// `A`: "Scale the icon to fill the annotation rectangle exactly, without regard to its
    /// original aspect ratio".
    Anamorphic,
    /// `P`, the table's default: "Scale the icon to fit the width or height of the annotation
    /// rectangle while maintaining the icon's original aspect ratio."
    #[default]
    Proportional,
}

/// Table 250's icon fit dictionary, whole: §12.5.6.19's Table 192 `/IF`.
///
/// > An icon fit dictionary (see "Table 250 - Entries in an icon fit dictionary") specifying how
/// > the widget annotation's icon shall be displayed within its annotation rectangle. If present,
/// > the icon fit dictionary shall apply to all of the annotation's icons (normal, rollover, and
/// > alternate).
///
/// **Table 250 itself is printed under §12.7.8.3.2**, where an FDF field's own `/IF` names it,
/// and a widget's is the second entry to point at the same dictionary. The entries below are
/// that table's, and every one of them has a default it states — so a widget with no `/IF` fits
/// its icon by this structure's own [`Default`]: proportional scaling, always, centred.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct IconFit {
    when: ScaleWhen,
    scaling: Scaling,
    /// `/A`: "the fraction of leftover space to allocate at the left and bottom of the icon".
    anchor: [f32; 2],
    /// `/FB`: whether the icon is fitted "without taking into consideration the line width of
    /// the border".
    ignore_border: bool,
}

impl IconFit {
    /// The table's default: `/SW A`, `/S P`, `/A [0.5 0.5]`, `/FB false`.
    const DEFAULT: Self = Self {
        when: ScaleWhen::Always,
        scaling: Scaling::Proportional,
        anchor: [0.5, 0.5],
        ignore_border: false,
    };

    /// Table 192's `/IF`, with Table 250's default for every entry the dictionary omits.
    fn read(document: &Document, characteristics: &Dictionary) -> Self {
        let entry = document.get_key(characteristics, "IF");
        let Some(fit) = entry.as_dict() else {
            return Self::DEFAULT;
        };
        let when = match document.get_key(fit, "SW") {
            Object::Name(name) => match name.as_bytes() {
                b"B" => ScaleWhen::Bigger,
                b"S" => ScaleWhen::Smaller,
                b"N" => ScaleWhen::Never,
                _ => ScaleWhen::Always,
            },
            _ => ScaleWhen::Always,
        };
        let scaling = match document.get_key(fit, "S") {
            Object::Name(name) if name.as_bytes() == b"A" => Scaling::Anamorphic,
            _ => Scaling::Proportional,
        };
        // "An array of two numbers that shall be between 0.0 and 1.0": a value outside that is
        // the file breaking the table's own bound, and clamping keeps the icon inside the
        // rectangle the entry is a fraction of.
        let anchor = crate::annotation::numbers(document, fit, "A")
            .filter(|values| values.len() >= 2)
            .map_or(Self::DEFAULT.anchor, |values| {
                [values[0].clamp(0.0, 1.0), values[1].clamp(0.0, 1.0)]
            });
        let ignore_border = matches!(document.get_key(fit, "FB"), Object::Boolean(true));
        Self {
            when,
            scaling,
            anchor,
            ignore_border,
        }
    }

    /// The transform that puts an icon of extent `icon` into `target`, under this fit.
    ///
    /// The scaling question is Table 250's two entries taken in the order it states them: `/SW`
    /// decides *whether* there is any, `/S` decides which kind. "Bigger than the annotation
    /// rectangle" is read as exceeding it on either axis and "smaller" as fitting on both with
    /// room to spare, which the table does not spell out and which is the only reading under
    /// which `B` and `S` are complementary rather than overlapping.
    ///
    /// `/A` is applied only under proportional scaling, because the table says so in as many
    /// words — "This entry shall be used only if the icon is scaled proportionally" — so an
    /// anamorphically fitted icon that does not fill its rectangle sits at the corner `[0.0 0.0]`
    /// names. That case exists only under `/SW N` and `/SW B`, where the file has asked for the
    /// icon's own size and said nothing about where to put it.
    fn place(self, icon: [f32; 4], target: [f32; 4]) -> Option<Transform> {
        let (width, height) = (icon[2] - icon[0], icon[3] - icon[1]);
        let (room_x, room_y) = (target[2] - target[0], target[3] - target[1]);
        if !(width > 0.0 && height > 0.0 && room_x > 0.0 && room_y > 0.0) {
            return None;
        }
        let scales = match self.when {
            ScaleWhen::Always => true,
            ScaleWhen::Bigger => width > room_x || height > room_y,
            ScaleWhen::Smaller => width < room_x && height < room_y,
            ScaleWhen::Never => false,
        };
        let (scale_x, scale_y) = if scales {
            match self.scaling {
                Scaling::Anamorphic => (room_x / width, room_y / height),
                Scaling::Proportional => {
                    let scale = (room_x / width).min(room_y / height);
                    (scale, scale)
                }
            }
        } else {
            (1.0, 1.0)
        };
        let anchor = match self.scaling {
            Scaling::Proportional => self.anchor,
            Scaling::Anamorphic => [0.0, 0.0],
        };
        let left = target[0] + anchor[0] * (room_x - width * scale_x) - icon[0] * scale_x;
        let bottom = target[1] + anchor[1] * (room_y - height * scale_y) - icon[1] * scale_y;
        let transform = Transform::new(scale_x, 0.0, 0.0, scale_y, left, bottom);
        transform.a.is_finite().then_some(transform)
    }
}

/// A push-button's normal icon: §12.5.6.19's Table 192 `/I`.
///
/// > (Optional; push-button fields only; shall be an indirect reference) A form XObject defining
/// > the widget annotation's normal icon , which shall be displayed when it is not interacting
/// > with the user.
///
/// The indirect reference the table requires is what makes the icon usable as a resource without
/// copying the stream: the reference goes into the constructed appearance's `/XObject` and the
/// interpreter resolves it against the same document.
///
/// The icon's own extent is §8.10.2's, since a form `XObject` states its size nowhere else: the
/// `/BBox` "in the form coordinate system", transformed by the `/Matrix` that maps that system
/// into the space the `Do` runs in. Table 250 then fits *that* rectangle into the annotation's.
struct Icon {
    /// The `/I` entry as the file wrote it — a reference, kept unresolved.
    reference: Object,
    /// `/BBox` under `/Matrix`, which is what Table 250 scales.
    extent: [f32; 4],
}

impl Icon {
    /// Table 192's `/I`, or `None` where the entry is absent or names no usable form `XObject`.
    fn read(document: &Document, characteristics: &Dictionary) -> Option<Self> {
        let reference = characteristics.get("I")?.clone();
        if matches!(reference, Object::Null) {
            return None;
        }
        let stream = document.resolve(&reference);
        let stream = stream.as_dict()?;
        let bbox = crate::annotation::rectangle(document, stream, "BBox")?;
        let extent =
            crate::annotation::transformed(bbox, crate::annotation::matrix(document, stream));
        Some(Self { reference, extent })
    }
}

/// Table 192's `/R`: which way up a widget's contents are drawn.
///
/// §12.5.6.19, Table 192:
///
/// > The number of degrees by which the widget annotation shall be rotated counterclockwise
/// > relative to the page. The value shall be a multiple of 90. Default value: 0 .
///
/// A *constructed* appearance is written in the page's own space, so §12.5.5's placement
/// algorithm reduces to the identity and there is no `/Matrix` to carry this — which is why the
/// rotation is written into the stream as a `cm` rather than onto the appearance. A widget with a
/// stored `/AP` needs none of this: its stream's own `/Matrix` is where a producer puts the turn,
/// and §12.5.5 maps the rotated bounding box onto `/Rect` already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rotation {
    /// The table's default, and 289 of the corpus's 290 widgets.
    None,
    /// A quarter turn counterclockwise.
    Quarter,
    /// A half turn.
    Half,
    /// Three quarters counterclockwise, which is a quarter clockwise.
    ThreeQuarters,
}

impl Rotation {
    /// Reads `/R`, or `None` for a value the table forbids.
    ///
    /// "[T]he value shall be a multiple of 90" is a requirement on the *file*, and a widget
    /// stating 45 has described a rotation no `cm` this function could write would be the one it
    /// meant. Refused by name rather than rounded, because rounding would draw a widget the
    /// document did not ask for and say nothing.
    fn read(document: &Document, source: &Dictionary) -> Option<Self> {
        let Some(degrees) = document.get_key(source, "R").as_integer() else {
            return Some(Self::None);
        };
        match degrees.rem_euclid(360) {
            0 => Some(Self::None),
            90 => Some(Self::Quarter),
            180 => Some(Self::Half),
            270 => Some(Self::ThreeQuarters),
            _ => None,
        }
    }

    /// The box the contents are laid out in.
    ///
    /// `/Rect` itself where there is no turn, so an unrotated widget is written exactly as it was
    /// before this type existed. Otherwise a box at the *origin*, because the `cm` below is what
    /// puts it on the page — and with its sides swapped for a quarter turn, which is the whole of
    /// what rotating a rectangle by 90° does to it.
    fn content_box(self, rect: [f32; 4]) -> [f32; 4] {
        let (width, height) = (rect[2] - rect[0], rect[3] - rect[1]);
        match self {
            Self::None => rect,
            Self::Half => [0.0, 0.0, width, height],
            Self::Quarter | Self::ThreeQuarters => [0.0, 0.0, height, width],
        }
    }

    /// The turn as a transform, which is what the `cm` [`Self::begin`] writes means.
    ///
    /// Each matrix is the rotation followed by the translation that brings the turned box back
    /// onto `/Rect`. A quarter turn counterclockwise takes `(x, y)` to `(−y, x)`, so a box
    /// `height` tall in its own axes lands at `x ∈ [−height, 0]`, and that height is `/Rect`'s
    /// *width* by `content_box` above, so the shift right is by that width. A half
    /// turn needs no swapped box and is still written as a `cm`, because a half turn is not the
    /// identity for anything with a direction, and text has one.
    ///
    /// Written here rather than only as the string below, because a caller that maps a *point*
    /// out of the widget's axes — [`caret`] — needs the same turn as arithmetic, and two
    /// spellings of one matrix are two chances to spell it differently.
    fn transform(self, rect: [f32; 4]) -> Transform {
        let (width, height) = (rect[2] - rect[0], rect[3] - rect[1]);
        match self {
            Self::None => Transform::IDENTITY,
            Self::Quarter => Transform::new(0.0, 1.0, -1.0, 0.0, rect[0] + width, rect[1]),
            Self::Half => Transform::new(-1.0, 0.0, 0.0, -1.0, rect[0] + width, rect[1] + height),
            Self::ThreeQuarters => Transform::new(0.0, -1.0, 1.0, 0.0, rect[0], rect[1] + height),
        }
    }

    /// Opens the rotated space, where there is one.
    fn begin(self, stream: &mut Stream, rect: [f32; 4]) {
        if self == Self::None {
            return;
        }
        let turn = self.transform(rect);
        stream.rotate(
            &format!("{} {} {} {}", turn.a, turn.b, turn.c, turn.d),
            [turn.e, turn.f],
        );
    }

    /// Closes it again.
    fn end(self, stream: &mut Stream) {
        if self != Self::None {
            stream.text.push_str("Q\n");
        }
    }
}

/// What §12.7.5.4's list box shows, and whether its selection is left unmarked.
///
/// **The clause states the whole of what is drawn here and one thing about it that it does not
/// state**, and this arm used to refuse the annotation over the second. What is displayed is
/// Table 234's `/Opt` array — "[t]he Opt array specifies the list of options in the choice
/// field, each of which shall be represented by a text string that shall be displayed on the
/// screen" — in the array's own order, which Table 233 bit 20 makes a `shall` addressed to a
/// reader: "PDF readers shall display the options in the order in which they occur in the Opt
/// array". §12.7.4.3's own NOTE names a scrollable list box as its example of what a processor
/// "shall construct … dynamically at rendering time".
///
/// What no clause states is the *selection's* appearance. `/V` "identifies the item or items
/// currently selected" and nothing anywhere says what a selected item looks like — no highlight
/// colour, no rule, nothing. That is a mark **added over** an item that is drawn either way,
/// which is ADR 0106's test for whether a refusal may take the rest of an annotation down with
/// it; the answer is no, so the options are drawn and the mark is reported (ADR 0030's shape).
/// A host that builds a real list draws the selection in its own colours from
/// [`crate::form::ChoiceControl::selected`] — the same division this tree makes for a text
/// selection, whose colour is likewise nobody's here to invent.
///
/// `None` for a field stating no `/Opt`, which is Table 234's own answer rather than a gap: "If
/// this entry is not present, no choices should be presented to the user."
fn list_box_options(
    document: &Document,
    field: &Field,
    annotation: &Dictionary,
) -> Option<(String, bool)> {
    let options = crate::form::options(document, field);
    let last = options.len().checked_sub(1)?;
    // Table 234's `/TI`, "the index in the Opt array of the first option visible in the list",
    // default 0. Clamped rather than obeyed where it names an index the array does not have: an
    // optional entry may not erase what the clause states (ADR 0111), and a list scrolled past
    // its end is one showing its last option.
    let top = crate::form::inherited_number(document, field, annotation, "TI")
        .and_then(|top| usize::try_from(top).ok())
        .unwrap_or_default()
        .min(last);
    // Reported only where there is something to mark. §12.7.5.4 gives `/V` the default null,
    // "indicating that no item is currently selected", and a list with nothing selected is drawn
    // completely — trap 11's rule that a report fires on the clause's own condition rather than
    // wherever the unimplemented thing could be involved.
    let unmarked = !crate::form::selected(document, field, &options).is_empty();
    let shown = options
        .get(top..)
        .unwrap_or_default()
        .iter()
        .map(|option| option.label.as_str())
        .collect::<Vec<&str>>()
        .join("\n");
    Some((shown, unmarked))
}

/// Lays out whatever text the field behind a widget states, if any.
///
/// One function because the field types differ only in where the text comes from; §12.7.4.3
/// does the same thing with all of them. `Ok(None)` is a field that states no text, which is
/// the common case and not a gap: 147 widgets on the corpus's first pages are empty text
/// fields waiting for a person.
///
/// `asked` is what a *question* wants out of the layout and is empty for everything that draws:
/// where it asks anything at all, an empty field is laid out rather than skipped, because a place
/// for the next character is exactly what an empty field can still be asked for.
fn field_text(
    document: &Document,
    annotation: &Dictionary,
    characteristics: &Dictionary,
    box_: [f32; 4],
    value: FieldValue<'_>,
    asked: Asked,
) -> Result<Option<variable_text::LaidOut>, Refusal> {
    let field = Field::read(document, annotation, value);
    if field.too_deep {
        return Err(Refusal::NotDerivable(
            "its field's /Parent chain is longer than this crate follows, so what it holds is \
             unknown",
        ));
    }
    let Some(kind) = field.kind else {
        // §12.7.4.2: "A field dictionary that does not have a partial field name (T entry) of
        // its own shall not be considered a field but simply a Widget annotation." With no
        // `/FT` anywhere up the chain there is no field type, so nothing states any text.
        return Ok(None);
    };

    // Set by the one field type whose drawing is complete and whose *marking* is not; see the
    // list box's arm for why that is a report beside the options rather than a refusal of them.
    let mut selection_unmarked = false;
    let (text, shape) = match kind {
        // Table 192's `/CA`, "the widget annotation's normal caption, which shall be displayed
        // when it is not interacting with the user" — the entry that "may be used with any
        // type of button field, including check boxes and radio buttons". A check box or radio
        // button shows it only in its on state, because §12.7.5.2.3 makes the value select an
        // appearance rather than describe one, and an off box showing its tick would be wrong
        // in the one way a check box can be wrong.
        FieldKind::Button { toggling } => {
            if toggling && !field.is_on(document, annotation) {
                return Ok(None);
            }
            match variable_text::string(document, &[characteristics], "CA") {
                Some(caption) => (caption, Shape::SingleLine),
                // A check box or radio button the document says is *on*, with neither an
                // appearance dictionary to select a state from nor a caption to draw, states
                // that it is ticked and states nothing that shows it. That is worth saying: a
                // box drawn empty is not a near miss, it is the opposite answer.
                None if toggling => {
                    return Err(Refusal::NotDerivable(
                        "§12.7.5.2.3 puts a check box's on state in its /AP, and this one has \
                         neither an /AP nor Table 192's /CA caption",
                    ));
                }
                None => return Ok(None),
            }
        }
        FieldKind::Text => {
            let value = field
                .value
                .as_ref()
                .and_then(|value| variable_text::value_text(document, value))
                .unwrap_or_default();
            // An empty field draws nothing — and it still has somewhere the next character
            // goes, which is the one thing an empty field can be asked about. So the layout is
            // skipped only when nobody asked where that place is.
            if value.is_empty() && asked == Asked::default() {
                return Ok(None);
            }
            // Table 231 bit 14: a password field's characters "shall instead be echoed in some
            // unreadable form, such as asterisks or bullet characters". A value stored in the
            // file at all breaks the same row's NOTE; echoing it as it stands would publish it.
            let value = if field.flags & FLAG_PASSWORD == 0 {
                value
            } else {
                "\u{2022}".repeat(value.chars().count())
            };
            (value, field.text_shape(document, annotation))
        }
        FieldKind::Choice { combo: false } => {
            match list_box_options(document, &field, annotation) {
                Some((shown, unmarked)) => {
                    selection_unmarked = unmarked;
                    (shown, Shape::ListBox)
                }
                None => return Ok(None),
            }
        }
        FieldKind::Choice { combo: true } => {
            let value = field
                .value
                .as_ref()
                .and_then(|value| variable_text::value_text(document, value))
                .unwrap_or_default();
            if value.is_empty() && asked == Asked::default() {
                return Ok(None);
            }
            (value, Shape::SingleLine)
        }
        // §12.7.5.5: a signature field's value is a signature dictionary and signing "entails
        // updating at least the V entry and usually also the AP entry". With no `/AP` there is
        // no appearance, and no clause states one to build.
        FieldKind::Signature => return Ok(None),
    };

    let form = interactive_form(document).unwrap_or_default();
    let sources: Vec<&Dictionary> = field
        .ancestry
        .iter()
        .chain(std::iter::once(&form))
        .collect();
    // Table 228 marks `/DA` and `/Q` inheritable, and Table 224 gives the interactive form
    // dictionary "a document-wide default value" for each — so the chain runs from the widget
    // up its parents and ends at the form.
    let Some(default_appearance) = variable_text::bytes(document, &sources, "DA") else {
        return Err(Refusal::Text(Owed::NoFont));
    };
    let resources = default_resources(document);
    let request = Request {
        text: &text,
        box_,
        default_appearance: &default_appearance,
        resources: &resources,
        quadding: Quadding::read(document, &sources),
        shape,
        asked,
    };
    variable_text::lay_out(document, &request)
        .map(|mut laid_out| {
            // Behind whatever the layout itself owed, and the order is an argument rather than a
            // habit: `Owed` carries one statement, and a shortfall in the glyphs that *were*
            // drawn — a font `/DR` does not define, a character it states no code for — explains
            // the picture, where an unmarked selection only adds to it.
            if selection_unmarked {
                laid_out.owed = laid_out.owed.or(Some(Owed::ListBoxSelection));
            }
            Some(laid_out)
        })
        .map_err(Refusal::Text)
}

/// Where the caret sits inside the text of an annotation, in **default user space**.
///
/// **Two subtypes, and [`frame`] says why**: a widget whose field §12.7.4.3 lays text out for, and
/// §12.5.6.6's free text annotation, whose own clause sends it to that same subclause. Everything
/// below is written of a field because that is where it started, and every word of it is true of
/// the other — the layout underneath is one layout, and the way in is what differs.
///
/// The place the next character will be drawn, which §12.7.4.3's layout is what knows: the x a
/// line is positioned at plus the width of the value's own prefix, between the ascent and the
/// descent [`variable_text`] measures a line's height by. A host that computed it instead would
/// be laying the value out a second time, with its own font and its own auto-sizing, and would
/// sit the caret beside the text rather than in it.
///
/// **Nothing in ISO 32000-2 states a caret**, and §12.5.6.11's caret *annotation* is a different
/// object — a mark left in a document to say text was edited there, with its own `/Rect` and its
/// own appearance. What the standard decides here is only where the glyphs go; the segment this
/// answers with is derived from that, and what it looks like is the host's (ADR 0211).
///
/// `None` for a field whose value is not characters — a button, a signature, and §12.7.5.4's list
/// box, whose value names *items* and whose laid-out text is the `/Opt` array rather than the
/// value, so a byte offset into it would answer about some option's spelling and not about `/V` —
/// and for a widget whose value could not be laid out at all, which is the same condition that
/// makes the page report the field. [`frame`] is where that population is chosen, once, for this
/// question and for its two siblings.
///
/// # Which space the value is laid out in
///
/// The two `crate::annotation::decide` chooses between, chosen the same way. Where the file
/// carries an appearance stream, the value is laid out in *that stream's* `/BBox` and §12.5.5's
/// algorithm maps it onto `/Rect` — and the caret is answered from that stream's box whether or
/// not the stream has been rewritten yet, because the next character typed rewrites it (the
/// clause's own splice) and the caret's job is to say where that character will land. Where there
/// is none, `construct` writes the marks in the page's own space and the only turn is Table 192's
/// `/R`.
pub(crate) fn caret(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
    offset: usize,
) -> Option<[f32; 4]> {
    let asked = Asked {
        caret: Some(offset),
        ..Asked::default()
    };
    let (laid_out, onto_page) = ask(document, annotation, view, asked)?;
    let caret = laid_out.caret?;
    let from = onto_page.apply(Point::new(caret.from[0], caret.from[1]));
    let to = onto_page.apply(Point::new(caret.to[0], caret.to[1]));
    Some([from.x, from.y, to.x, to.y])
}

/// Which byte of a widget's value a point in **default user space** falls nearest.
///
/// [`caret`]'s inverse, and the piece a click inside a value needs: that one takes an offset and
/// answers a place, this takes a place and answers an offset, and the two are computed in the same
/// walk of §12.7.4.3's layout so that they cannot disagree. An offset this answers with, handed
/// straight back to [`caret`], puts the cursor where the click was.
///
/// **Nearest rather than inside**, which is the choice a point outside every glyph forces: a click
/// past the end of a line answers the end of that line, one above the first line answers the
/// first, and one in a field whose value is empty answers zero. There is no such thing as a click
/// inside a field that names no place to type, so refusing would leave a host with nothing to do
/// with a press it has already decided is a press into the field. ADR 0225.
///
/// `None` in exactly the cases [`caret`] answers `None` in, and for the same reasons.
pub(crate) fn offset_at(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
    point: (f32, f32),
) -> Option<usize> {
    // The layout works in the appearance's own coordinates, so the *point* is what has to move —
    // the inverse of the map every shape leaves through. A transform with no inverse is a widget
    // whose appearance collapses to a line or a point, which has no inside for a click to be in.
    let (box_, onto_page) = frame(document, annotation, view)?;
    let inside = onto_page.invert()?.apply(Point::new(point.0, point.1));
    let asked = Asked {
        point: Some([inside.x, inside.y]),
        ..Asked::default()
    };
    laid_out_in(document, annotation, view, box_, asked)?.offset
}

/// The shapes covering a byte range of a widget's value, in **default user space**.
///
/// One quadrilateral per line the range touches, `[x0, y0, … x3, y3]` — four corners rather than a
/// rectangle because Table 192's `/R`, the appearance's `/Matrix` and §12.5.5's placement can each
/// turn the box, exactly as they can turn the caret.
///
/// **Deliberately not two carets.** A host holding both ends of a selection could join them itself
/// on a single-line field and could not on a multiline one: §12.7.5.3's Table 231 bit 13 lets
/// `variable_text::wrap` break the value where the host cannot see, and the lines *between* the
/// two ends are this crate's to name. ADR 0225.
///
/// Empty where the range covers no glyph — two equal offsets, or a range holding nothing but a
/// line break — because a highlight over nothing is nothing, and `None` in the cases [`caret`]
/// answers `None` in.
pub(crate) fn selection(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
    range: (usize, usize),
) -> Option<Vec<[f32; 8]>> {
    let asked = Asked {
        selection: Some((range.0.min(range.1), range.0.max(range.1))),
        ..Asked::default()
    };
    let (laid_out, onto_page) = ask(document, annotation, view, asked)?;
    Some(
        laid_out
            .selection
            .iter()
            .map(|[x0, y0, x1, y1]| {
                let corners = [(*x0, *y1), (*x1, *y1), (*x1, *y0), (*x0, *y0)];
                let mut quad = [0.0_f32; 8];
                for (corner, place) in corners.iter().zip(quad.chunks_exact_mut(2)) {
                    let point = onto_page.apply(Point::new(corner.0, corner.1));
                    place[0] = point.x;
                    place[1] = point.y;
                }
                quad
            })
            .collect(),
    )
}

/// Lays a widget's value out to answer a question, with the map onto the page beside it.
///
/// One function rather than three copies of it: [`caret`], [`offset_at`] and [`selection`] differ
/// only in what they ask for and in what they do with the answer, and a second reading of which
/// space the value is laid out in would be a second chance to read it differently.
fn ask(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
    asked: Asked,
) -> Option<(variable_text::LaidOut, Transform)> {
    let (box_, onto_page) = frame(document, annotation, view)?;
    let laid_out = laid_out_in(document, annotation, view, box_, asked)?;
    Some((laid_out, onto_page))
}

/// The box the text is laid out in, and the map from that box onto the page.
///
/// The two `crate::annotation::decide` chooses between, chosen the same way — see [`caret`]'s own
/// note on which space is which.
///
/// **Two subtypes reach here and the standard is why**: §12.7.4.3 lays text out for a widget whose
/// field states some, and §12.5.6.6 sends its own annotation to that same subclause — "[s]ubclause
/// 12.7.4.3, 'Variable text', describes the process of using these entries to generate the
/// appearance of the text in these annotations". What differs is the *box*, because Table 177
/// states one entry Table 192 does not and neither of Table 192's two: a free text annotation's
/// text sits in `/RD`'s inner rectangle, a widget's inside its border and under Table 192's `/R`.
fn frame(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
) -> Option<([f32; 4], Transform)> {
    if is_free_text(document, annotation) {
        return free_text_frame(document, annotation, view);
    }
    let field = Field::read(document, annotation, view.value);
    if field.too_deep
        || !matches!(
            field.kind,
            Some(FieldKind::Text | FieldKind::Choice { combo: true })
        )
    {
        return None;
    }
    let characteristics = document.get_key(annotation, "MK").as_dict().cloned();
    let source = characteristics.as_ref().unwrap_or(annotation);
    let width = Border::read(document, annotation, source, "BC")
        .map(|border| border.width)
        .unwrap_or_default();
    if let Some((bbox, placement)) = crate::annotation::stored_frame(document, annotation, view) {
        return Some((inset(bbox, width), placement));
    }
    let rect = rectangle(document, annotation).ok()?;
    let rotation = Rotation::read(document, source)?;
    Some((
        inset(rotation.content_box(rect), width),
        rotation.transform(rect),
    ))
}

/// The box §12.5.6.6's text is laid out in, and the map from it onto the page.
///
/// [`frame`]'s other half. Where the file carries an appearance stream the text is laid out in
/// that stream's `/BBox` and §12.5.5's algorithm maps it onto `/Rect`, exactly as for a widget;
/// where there is none, [`free_text`] writes its marks in Table 177's `/RD` rectangle — which is
/// already in the page's own space, so the map is the identity and not a coincidence. `/BBox` is
/// `/Rect` for an appearance this program writes (ADR 0196's argument, one subtype over), which
/// makes §12.5.5's map the identity too.
fn free_text_frame(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
) -> Option<([f32; 4], Transform)> {
    if let Some((bbox, placement)) = crate::annotation::stored_frame(document, annotation, view) {
        return Some((bbox, placement));
    }
    let rect = rectangle(document, annotation).ok()?;
    Some((differences(document, annotation, rect), Transform::IDENTITY))
}

/// The layout itself, in the box [`frame`] chose.
fn laid_out_in(
    document: &Document,
    annotation: &Dictionary,
    view: crate::view::AnnotationView<'_>,
    box_: [f32; 4],
    asked: Asked,
) -> Option<variable_text::LaidOut> {
    if is_free_text(document, annotation) {
        return free_text_layout(document, annotation, box_, view.contents, asked)
            .ok()
            .flatten();
    }
    let characteristics = document.get_key(annotation, "MK").as_dict().cloned();
    let source = characteristics.as_ref().unwrap_or(annotation);
    field_text(document, annotation, source, box_, view.value, asked)
        .ok()
        .flatten()
}

/// The text a text or combo-box field would be laid out with, as §12.7.4.3 sees it.
///
/// The value a *host* is told a field holds, which is not simply Table 226's `/V`: it is whichever
/// of the four statements about a value is current (`value`), read through §12.7.4.1's `/Parent`
/// chain and decoded from §7.9.2.2's text string type — and for a password field it is the bullets
/// Table 231 bit 14 requires be echoed instead, because a host may not be handed a secret it is
/// only allowed to draw as asterisks. **[`ShownValue::obscured`] says which of the two it is**,
/// beside the string rather than in a doc comment about it (ADR 0247).
///
/// **`None` and `Some("")` are different answers and the difference is the point.** `None` is a
/// field whose value is not text at all — §12.7.5.2's buttons select an appearance, §12.7.5.5's
/// signatures hold a dictionary, §12.7.5.4's list box states which items are selected — and
/// `Some("")` is a text field with nothing in it, which is what 147 of the corpus's first-page
/// widgets are. A host deciding where to send the keyboard needs exactly that distinction.
///
/// [`crate::view::ViewState::field_value`] is the caller and its doc comment is why this exists.
pub(crate) fn field_text_value(
    document: &Document,
    annotation: &Dictionary,
    value: FieldValue<'_>,
) -> Option<crate::view::ShownValue> {
    let field = Field::read(document, annotation, value);
    if field.too_deep {
        return None;
    }
    if !matches!(
        field.kind,
        Some(FieldKind::Text | FieldKind::Choice { combo: true })
    ) {
        return None;
    }
    let text = field
        .value
        .as_ref()
        .and_then(|value| variable_text::value_text(document, value))
        .unwrap_or_default();
    if field.flags & FLAG_PASSWORD == 0 {
        Some(crate::view::ShownValue {
            text,
            obscured: false,
        })
    } else {
        Some(crate::view::ShownValue {
            text: "\u{2022}".repeat(text.chars().count()),
            obscured: true,
        })
    }
}

/// How much of a value one widget will take, where §12.7.5.3's Table 231 bit 24 binds.
///
/// > If set, the field shall not scroll (horizontally for single-line fields, vertically for
/// > multiple-line fields) to accommodate more text than fits within its annotation rectangle.
/// > Once the field is full, no further text shall be accepted for interactive form filling;
/// > for non-interactive form filling, the filler should take care not to add more character
/// > than will visibly fit in the defined area.
///
/// Two sentences and only the second binds a reader — a `shall` about *accepting* text, which
/// binds this tree because [`crate::view::ViewState::set_field`] made it a program that fills a
/// field. `None` is a widget the flag does not constrain: it is clear, or the field is not a
/// text field (Table 231 is §12.7.5.3's alone), or nothing about the widget can be laid out at
/// all, in which case refusing text on the strength of a layout that does not exist would be a
/// guess. `Some(n)` is the byte length of the longest prefix of `value` that fits.
///
/// **The search is a bisection over the value's character boundaries**, on the property that a
/// longer value never fits where a shorter one does not: [`variable_text::lay_out`]'s wrapping
/// is greedy left to right, so the lines a prefix produces are what the full value's layout had
/// reached at that point, and auto-sizing only shrinks as the value grows. The answer is checked
/// rather than assumed — the prefix returned is one this function measured — so a font with
/// negative advances costs an early cut and not a wrong one.
pub(crate) fn accepted_prefix(
    document: &Document,
    annotation: &Dictionary,
    value: &str,
) -> Option<usize> {
    let field = Field::read(
        document,
        annotation,
        FieldValue::Edited {
            value: None,
            indices: None,
        },
    );
    if field.too_deep
        || field.kind != Some(FieldKind::Text)
        || field.flags & FLAG_DO_NOT_SCROLL == 0
    {
        return None;
    }

    // The same box the appearance is laid out in, arrived at the same way: `/Rect` turned into
    // the widget's own axes and inset by the border that would otherwise strike the text
    // through. A different box here would accept text the appearance then clips.
    let rect = rectangle(document, annotation).ok()?;
    let characteristics = document.get_key(annotation, "MK").as_dict().cloned();
    let source = characteristics.as_ref().unwrap_or(annotation);
    let border = Border::read(document, annotation, source, "BC").ok()?;
    let rect = Rotation::read(document, source)?.content_box(rect);
    let box_ = inset(rect, border.width);

    let form = interactive_form(document).unwrap_or_default();
    let sources: Vec<&Dictionary> = field
        .ancestry
        .iter()
        .chain(std::iter::once(&form))
        .collect();
    let default_appearance = variable_text::bytes(document, &sources, "DA")?;
    let resources = default_resources(document);
    let quadding = Quadding::read(document, &sources);
    let shape = field.text_shape(document, annotation);
    // Table 231 bit 14 has a password field's characters "echoed in some unreadable form", and
    // `field_text` echoes them as bullets — so the width that decides whether the field is full
    // is the bullets' and not the characters'.
    let password = field.flags & FLAG_PASSWORD != 0;
    let fits = |prefix: &str| {
        let shown = if password {
            "\u{2022}".repeat(prefix.chars().count())
        } else {
            prefix.to_owned()
        };
        let request = Request {
            text: &shown,
            box_,
            default_appearance: &default_appearance,
            resources: &resources,
            quadding,
            shape,
            asked: Asked::default(),
        };
        // A layout this crate cannot build says nothing about how much text the box holds, so
        // it constrains nothing — the report `field_text` raises is the honest answer there.
        variable_text::lay_out(document, &request).is_ok_and(|laid_out| !laid_out.overflows)
    };

    if fits(value) {
        return Some(value.len());
    }
    let boundaries: Vec<usize> = value
        .char_indices()
        .map(|(at, _)| at)
        .chain(std::iter::once(value.len()))
        .collect();
    // An empty value fits by construction, so the bisection's lower bound needs no measuring.
    let (mut low, mut high) = (0_usize, boundaries.len().saturating_sub(1));
    while low < high {
        let middle = low.saturating_add(high.saturating_add(1).saturating_sub(low) / 2);
        let at = boundaries.get(middle).copied()?;
        if fits(value.get(..at)?) {
            low = middle;
        } else {
            high = middle.saturating_sub(1);
        }
    }
    boundaries.get(low).copied()
}

/// Draws a free text annotation's `/Contents`, laid out by §12.7.4.3, as §12.5.6.6 states:
///
/// > A free text annotation ( PDF 1.3 ) displays text directly on the page.
///
/// The text *is* the annotation, so unlike a widget there is nothing else to draw and a
/// failure to lay it out leaves the page unmarked. Table 177's `/DA` is Required for exactly
/// that reason, and Table 177's `/RD` states where the text goes: "The inner rectangle is
/// where the annotation's text should be displayed."
///
/// Table 177's `/CL` is drawn beside it where the annotation's `/IT` asks for one — see
/// [`callout`], which holds the whole of that entry and of `/LE`.
///
/// §12.5.4's border is drawn around the same inner rectangle — see [`free_text_border`], which
/// holds the whole of that entry, the colour no clause states, and why it stopped being a
/// refusal. [`crate::view::ViewState::add_free_text`] still writes a `/BS` with Table 168's `/W`
/// 0, which is now a statement rather than an evasion: a note this program adds asks for no
/// border, in the entry the table provides for saying so.
///
/// # Where the text comes from, and Table 177's second source for it
///
/// Table 166's `/Contents` first, and Table 177's `/RC` where the file states none. That entry is
/// a `shall` about *this annotation's appearance* rather than about a window — §12.5.6.6:
///
/// > A rich text string (see Adobe XML Architecture, XML Forms Architecture (XFA) Specification,
/// > version 3.3 ) that shall be used to generate the appearance of the annotation.
///
/// and its own NOTE separates it from the identically named entry [`crate::popup`] reads: "As
/// freetext annotations do not have an open state this cannot apply to the popup window as
/// described for the RC key in "Table 172 - Additional entries in an annotation dictionary
/// specific to markup annotations"." ADR 0199's reading carries over unchanged — the *characters*
/// are what the clause requires and the XFA markup is what `CLAUDE.md` excludes — and the
/// ordering is §12.5.6.2 NOTE 1's, which makes the two "textually equivalent" where a file states
/// both, so no document stating `/Contents` changes at all. ADR 0224.
///
/// Until the three-hundred-and-eighty-seventh session a free text annotation stating only `/RC`
/// drew nothing and reported nothing, which on this subtype is a blank page: the text *is* the
/// annotation.
fn free_text(
    document: &Document,
    annotation: &Dictionary,
    stream: &mut Stream,
    retyped: Option<&str>,
) -> Outcome {
    let rect = rectangle(document, annotation)?;
    // §12.5.6.6's `/RD` uses the same left, top, right, bottom order §12.5.6.8's does, which
    // `differences` already reads. It is the *text's* box and not the callout's — Table 177 says
    // "[t]he inner rectangle is where the annotation's text should be displayed" — so the line
    // below is drawn before it and in `/Rect`'s own space.
    let box_ = differences(document, annotation, rect);
    // Drawn first so that the note sits over the line where the two meet, and read before the
    // layout so that a callout still reaches the page where the text cannot be laid out: Table
    // 177 states the two independently and each is a mark the file asked for.
    let callout = callout(document, annotation, stream);
    // Before the text, so that a note whose glyphs reach the inner rectangle's edge sits over its
    // own border rather than under it — §12.5.4 puts the border "completely inside the annotation
    // rectangle" and says nothing about which of the two is on top.
    let border = free_text_border(document, annotation, stream, box_);
    let drawn_already = callout.drawn || border.drawn;
    let decoration = border.report;
    let laid_out = match free_text_layout(document, annotation, box_, retyped, Asked::default()) {
        Ok(laid_out) => laid_out,
        // ADR 0075's rule, one subtype over: an entry that cannot be read is a reason to draw
        // the part that can be rather than a reason to decline the whole annotation.
        Err(refusal) if drawn_already => return Ok(Painted::partly(refusal)),
        Err(refusal) => return Err(refusal),
    };
    let Some(laid_out) = laid_out else {
        // An empty note with a border is still a mark on the page: `drawn` is what decides
        // whether the stream this function has been writing into reaches the display list.
        return Ok(Painted {
            drawn: drawn_already,
            report: callout.owed.or(decoration),
        });
    };
    stream.text.push_str(&laid_out.content);
    stream.resources = Some(with_stand_in_font(
        default_resources(document),
        laid_out.font,
    ));
    Ok(Painted {
        drawn: true,
        report: laid_out
            .owed
            .map(Refusal::Text)
            .or(callout.owed)
            .or(decoration),
    })
}

/// What [`callout`] put on the page, and what it could not.
struct Callout {
    drawn: bool,
    owed: Option<Refusal>,
}

impl Callout {
    /// The annotation asks for no callout line, which is the case for every intent but one.
    const NONE: Self = Self {
        drawn: false,
        owed: None,
    };
}

/// Draws Table 177's `/CL`, the line from a free text annotation to the place it is about.
///
/// # What the table states
///
/// ISO 32000-2 §12.5.6.6, Table 177, on the entry:
///
/// > (Optional; meaningful only if IT is FreeTextCallout; PDF 1.6) An array of four or six
/// > numbers specifying a callout line attached to the free text annotation. Six numbers [ x 1 y
/// > 1 x 2 y 2 x 3 y 3 ] represent the starting, knee point, and ending coordinates of the line
/// > in default user space, as shown in "Figure 79 - Free text annotation with callout". Four
/// > numbers [ x 1 y 1 x 2 y 2 ] represent the starting and ending coordinates of the line.
///
/// with `/LE` naming "the line ending style that shall be used in drawing the callout line
/// specified in CL. The name shall specify the line ending style for the endpoint defined by the
/// pairs of coordinates ( x 1 , y 1 )". So the geometry is complete: two or three points, and one
/// of Table 179's ten shapes at the first of them, which is Figure 79's arrow tip.
///
/// # The condition to draw is the table's own, and it is not the entry's presence
///
/// The quotation above makes `/CL` meaningful only where `/IT` names one particular intent, which
/// is a condition on the *value* of another entry rather than on this one. Table 177's `/IT` makes
/// the plain `FreeText` its default and says of its third value that with it "no callout line is
/// drawn". So an annotation stating `/CL` without the callout intent has stated something the
/// table declares meaningless, and **nothing is reported for it**: a report names what this
/// program owes, and the standard owes a mark here only under the intent that asks for one
/// (trap 11).
///
/// # Two things the table does not state, and where each is taken from
///
/// - **The colour.** Table 166's `/C` is "the background of the annotation's icon when closed,
///   the title bar of the annotation's popup window, [and] the border of a link annotation", and
///   this subtype has none of the three; `/DA` is "the default appearance string that shall be
///   used in formatting the text", which this line is not. So the construction states **black**,
///   which is not an invention but the absence of one: §8.4.1's Table 51 gives the graphics
///   state's colour parameter "Initial value: black", so a stream that names no colour paints in
///   it. It is written out rather than left to the initial state so that the mark cannot depend
///   on what ran before the appearance.
/// - **The width.** Table 177 binds `/BS` to a different mark — its `/RD` row says border styles
///   and effects "shall be applied to the border of the inner rectangle" — so the one entry of
///   this subtype that carries a line width is a statement about the box round the text. One
///   point is therefore a choice, taken at [`DEFAULT_BORDER_WIDTH`], which is the number §12.5.4
///   states when nothing else does.
///
/// # Why this is drawn where `/BS`'s border is refused
///
/// Both lack a colour, and the difference is which of them the file *asked for*. Table 166's
/// `/Border` states "Default value: [0 0 1]", so a border is what an annotation saying nothing at
/// all about one has — drawing it in a colour of this program's choosing would put a mark on
/// nearly every free text annotation in the world on the strength of a default. A callout exists
/// only where a producer wrote four or six numbers *and* an intent, and those numbers say
/// something no other mark on the page says: which place the note is about.
fn callout(document: &Document, annotation: &Dictionary, stream: &mut Stream) -> Callout {
    let intent = document.get_key(annotation, "IT");
    if intent
        .as_name()
        .is_none_or(|name| name.as_bytes() != b"FreeTextCallout")
    {
        return Callout::NONE;
    }
    let entry = document.get_key(annotation, "CL");
    let Some(values) = entry.as_array() else {
        return Callout::NONE;
    };
    let line = pairs(document, values);
    // "An array of four or six numbers": a length outside those two states neither the
    // two-point line nor the three-point one, and which points a reader would keep is not
    // something the table answers.
    if !matches!((values.len(), line.len()), (4, 2) | (6, 3)) {
        return Callout {
            drawn: false,
            owed: Some(CALLOUT_SHAPE),
        };
    }
    let stated = document.get_key(annotation, "LE");
    let ending = stated
        .as_name()
        .and_then(|name| Ending::read(name.as_bytes()));
    // Table 177 gives `/LE` "Default value: None", which answers an *absent* entry; a name
    // outside Table 179 is a file asking for a shape this reader has no description of, and the
    // line is drawn without it rather than instead of it.
    let owed = match stated {
        Object::Null => None,
        _ => ending.is_none().then_some(UNKNOWN_LINE_ENDING),
    };
    let ending = ending.unwrap_or(Ending::None);

    stream.text.push_str("q\n");
    stream.set_colour(BLACK, true);
    stream.set_stroke(DEFAULT_BORDER_WIDTH, &[]);
    polyline(stream, &line, false);
    stream.paint(false, true);
    if let (Some(first), Some(second)) = (line.first(), line.get(1)) {
        // The one ending goes at (x1, y1) and points away from the knee — or from the far end
        // where there is no knee — which is what makes an arrowhead there an arrow *at* the
        // place the note is about. Table 179's four filled styles are stroked alone: Table 177
        // gives this subtype no `/IC` for them to be filled with.
        draw_endings(
            stream,
            [ending, Ending::None],
            [*first, *second],
            DEFAULT_BORDER_WIDTH,
            Colour::None,
        );
    }
    stream.text.push_str("Q\n");
    Callout { drawn: true, owed }
}

/// Draws §12.5.4's border around a free text annotation's inner rectangle.
///
/// # The shape the standard states, in full
///
/// ISO 32000-2 §12.5.4 opens the subclause and closes the question of what an annotation stating
/// nothing has:
///
/// > An annotation may optionally be surrounded by a border when displayed or printed. If
/// > present, the border shall be drawn completely inside the annotation rectangle.
///
/// > If neither the Border nor the BS entry is present, the border shall be drawn as a solid line
/// > with a width of 1 point.
///
/// Table 177 gives this subtype the entry that carries it — a `/BS` "specifying the line width
/// and dash pattern that shall be used in drawing **the annotation's border**" — and its `/RD`
/// row says where the mark goes: "Any border styles and/or border effects specified by BS and BE
/// entries, respectively, shall be applied to the border of the inner rectangle."
///
/// **Table 168's `/S` styles this border, unlike a square's or a circle's.** §12.5.4 names the
/// subtypes whose `/BS` supplies less than a whole border style dictionary — "[s]uch dictionaries
/// may also be used to specify the width and dash pattern for the lines drawn by line, square,
/// circle, and ink annotations" — and free text is not among the four; `/RD`'s "border styles"
/// is Table 168's `/S` by name. See [`square_or_circle`] for the other side of that division.
///
/// # The colour, which no clause states, and what black is taken from
///
/// Nothing in ISO 32000-2 gives this border a colour. Table 166's `/C` is a closed list of three
/// purposes — "[t]he background of the annotation's icon when closed[,] [t]he title bar of the
/// annotation's popup window[,] [t]he border of a link annotation" — and on a free text
/// annotation it is the second of them, because a markup annotation has a `/Popup` and no icon
/// and is not a link. §12.5.6.19's `/MK` `/BC` is "the colour of the widget annotation's border"
/// and this is not a widget. Read across the neighbourhood, the standard states a border colour
/// for a link and for a widget and for nothing else.
///
/// So **black is a choice**, and it is written down as one. What makes it the cheap choice rather
/// than an invention is §8.4.1's Table 51, which gives the graphics state's colour parameter
/// "Initial value: black": a construction that names no colour paints in it, and this one names
/// it explicitly only so that the mark cannot depend on what ran before the appearance. It is the
/// same choice [`callout`] takes one entry over.
///
/// # Why this is drawn where it was refused for a hundred sessions
///
/// The refusal rested on a claim about producers rather than about the standard — that Table
/// 166's `/Border` default `[0 0 1]` would put a mark on nearly every free text annotation in the
/// world on the strength of a default nobody wrote. That claim is measurable and
/// `examples/free_text_census` measured it: of the corpus's 73 free text annotations, 67 carry an
/// appearance stream and are not this path's business at all, and of the remaining six **every
/// one states `/Border` explicitly and four of them state a width of zero**. Not one relies on
/// §12.5.4's default. A producer who wants no border says so, in the entry the table provides for
/// saying it.
///
/// What is left is ADR 0106's test, and the border passes it: an entry that states no shape must
/// not erase the shape the clause does state. No entry claims this colour, so painting in the
/// initial one substitutes for nothing — where a cloudy `/BE` *does* state a different shape and
/// is refused below, exactly as it is on a square.
fn free_text_border(
    document: &Document,
    annotation: &Dictionary,
    stream: &mut Stream,
    box_: [f32; 4],
) -> Painted {
    // Table 177 gives this subtype a `/BE`, and §12.5.4 says so: "Beginning with PDF 1.6, free
    // text annotations may also have a BE entry". A cloudy border is a different border rather
    // than an extra mark, so it is refused whole (ADR 0106) — the text is drawn either way.
    if cloudy(document, annotation) {
        return Painted {
            drawn: false,
            report: Some(CLOUDY),
        };
    }
    let border = Border::geometry(document, annotation, BLACK);
    if !border.strokes() {
        return Painted::EMPTY;
    }
    stream.text.push_str("q\n");
    border.apply(stream);
    border.outline(stream, box_);
    stream.paint(false, true);
    stream.text.push_str("Q\n");
    border.simulated()
}

/// §12.5.6.6's text, laid out by §12.7.4.3 in the box the annotation leaves for it.
///
/// **One function for the two things that ask for this layout**, which is why it is not simply
/// [`free_text`]'s middle: the appearance that draws, and the three questions [`caret`],
/// [`offset_at`] and [`selection`] ask of an annotation a person is typing into. A second copy of
/// the wiring — where the text comes from, which `/DA` applies, which `/Q` — would be a second
/// chance to answer one of those differently, and the cursor would sit beside the text rather
/// than in it.
///
/// `Ok(None)` is an annotation stating no text, which draws nothing. **An empty one is still laid
/// out where a question asked something of it**, exactly as an empty field is: somewhere for the
/// first character to go is the one thing an empty box can be asked, and an annotation a person
/// has just drawn with a pointer is empty by construction.
///
/// `retyped` is [`crate::view::AnnotationView::contents`], and where it is `Some` it is the whole
/// answer: a person who took the text out of a note has not asked for Table 177's `/RC` to appear
/// from underneath it, and the fallback below is between two things the *file* states.
fn free_text_layout(
    document: &Document,
    annotation: &Dictionary,
    box_: [f32; 4],
    retyped: Option<&str>,
    asked: Asked,
) -> Result<Option<variable_text::LaidOut>, Refusal> {
    // §12.5.6.2 makes `/Contents` and `/RC` group attributes, so a free text annotation that is a
    // group's subordinate displays the primary's words: "the corresponding entries in the
    // subordinate annotations shall be ignored". No document in any population this project
    // measures does that — `examples/annotation_group_census` finds one `/RT /Group` in the corpus
    // and none of them free text — and the rule is applied anyway, because the list is the
    // clause's rather than the corpus's.
    let shared = crate::markup::group_source(document, annotation);
    let text = match retyped {
        Some(retyped) => retyped.to_owned(),
        None => variable_text::string(document, &[&shared], "Contents")
            .filter(|contents| !contents.is_empty())
            .or_else(|| crate::popup::rich_text(document, &shared))
            .unwrap_or_default(),
    };
    if text.is_empty() && asked == Asked::default() {
        return Ok(None);
    }
    let form = interactive_form(document).unwrap_or_default();
    let sources = [annotation, &form];
    let Some(default_appearance) = variable_text::bytes(document, &sources, "DA") else {
        return Err(Refusal::Text(Owed::NoFont));
    };
    let resources = default_resources(document);
    let request = Request {
        text: &text,
        box_,
        default_appearance: &default_appearance,
        resources: &resources,
        quadding: Quadding::read(document, &sources),
        // Table 177 states no single-line free text: the annotation is a box of prose.
        shape: Shape::Multiline,
        asked,
    };
    variable_text::lay_out(document, &request)
        .map(Some)
        .map_err(Refusal::Text)
}

/// Whether this annotation is §12.5.6.6's, which Table 177 makes `/Subtype /FreeText`.
fn is_free_text(document: &Document, annotation: &Dictionary) -> bool {
    document
        .get_key(annotation, "Subtype")
        .as_name()
        .is_some_and(|subtype| subtype.as_bytes() == b"FreeText")
}

/// Adds the font [`variable_text::lay_out`] invented, if it invented one, to `/DR`'s resources.
///
/// §12.7.4.3 makes a constructed appearance's `/Resources` "created using resources from the
/// interactive form dictionary's DR entry", and this is the one addition to that rule: a `/DA`
/// naming a font `/DR` does not define leaves the stream saying `/Name … Tf` for a name nothing
/// answers, so the stand-in has to arrive with it or the interpreter would report a missing
/// resource instead of the missing definition. `/DR`'s own entry always wins, because there is
/// only a stand-in where `/DR` had none.
fn with_stand_in_font(
    mut resources: Dictionary,
    font: Option<(pdf_syntax::Name, Dictionary)>,
) -> Dictionary {
    let Some((name, dict)) = font else {
        return resources;
    };
    let mut fonts = resources
        .get("Font")
        .and_then(Object::as_dict)
        .cloned()
        .unwrap_or_default();
    fonts.insert(name, Object::Dictionary(dict));
    resources.insert(
        pdf_syntax::Name::new(b"Font".to_vec()),
        Object::Dictionary(fonts),
    );
    resources
}

/// Shrinks a rectangle on all four sides, stopping at its centre line.
fn inset(rect: [f32; 4], by: f32) -> [f32; 4] {
    let x = by.min((rect[2] - rect[0]) * 0.5).max(0.0);
    let y = by.min((rect[3] - rect[1]) * 0.5).max(0.0);
    [rect[0] + x, rect[1] + y, rect[2] - x, rect[3] - y]
}

/// Table 227 bit 1: "an interactive PDF processor shall not allow a user to change the value of
/// the field".
pub(crate) const FLAG_READ_ONLY: i64 = 1;
/// Table 227 bit 2: "the field shall have a value at the time it is exported by a submit-form
/// action".
pub(crate) const FLAG_REQUIRED: i64 = 1 << 1;
/// Table 227 bit 3: "the field shall not be exported by a submit-form action".
pub(crate) const FLAG_NO_EXPORT: i64 = 1 << 2;
/// Table 231 bit 13: "the field may contain multiple lines of text".
///
/// **This constant's comment said Table 227 until the three-hundred-and-ninety-eighth session.**
/// That table is the three flags above and stops at bit 3; bit 13 is §12.7.5.3's, and the
/// difference is not cosmetic — a reader looking the sentence up in the cited table would not
/// find it. Found by `doc/todo/02` §4's ninth sweep, run over `crates/` as that section asks.
const FLAG_MULTILINE: i64 = 1 << 12;
/// Table 231 bit 14: the field "is intended for entering a secure password".
const FLAG_PASSWORD: i64 = 1 << 13;
/// Table 229 bit 15: "(Radio buttons only) If set, exactly one radio button shall be selected at
/// all times".
pub(crate) const FLAG_NO_TOGGLE_TO_OFF: i64 = 1 << 14;
/// Table 229 bit 16: "If set, the field is a set of radio buttons; if clear, the field is a check
/// box."
pub(crate) const FLAG_RADIO: i64 = 1 << 15;
/// Table 229 bit 17: "If set, the field is a push-button that does not retain a permanent
/// value."
const FLAG_PUSHBUTTON: i64 = 1 << 16;
/// Table 233 bit 18: "If set, the field is a combo box; if clear, the field is a list box."
const FLAG_COMBO: i64 = 1 << 17;
/// Table 233 bit 19: "If set, the combo box shall include an editable text box as well as a
/// drop-down list".
pub(crate) const FLAG_EDIT: i64 = 1 << 18;
/// Table 231 bit 21: "If set, the text entered in the field represents the pathname of a file
/// whose contents shall be submitted as the value of the field."
pub(crate) const FLAG_FILE_SELECT: i64 = 1 << 20;
/// Table 233 bit 22: "If set, more than one of the field's option items may be selected
/// simultaneously".
pub(crate) const FLAG_MULTI_SELECT: i64 = 1 << 21;
/// Table 231 bit 23 and Table 233 bit 23: "text entered in the field shall not be spell-checked".
pub(crate) const FLAG_DO_NOT_SPELL_CHECK: i64 = 1 << 22;
/// Table 231 bit 24: "If set, the field shall not scroll … to accommodate more text than fits
/// within its annotation rectangle."
const FLAG_DO_NOT_SCROLL: i64 = 1 << 23;
/// Table 231 bit 25: the field "shall be automatically divided into as many equally spaced
/// positions, or combs, as the value of `MaxLen`".
const FLAG_COMB: i64 = 1 << 24;
/// Table 231 bit 26: "the value of this field shall be a rich text string".
pub(crate) const FLAG_RICH_TEXT: i64 = 1 << 25;
/// Table 229 bit 26: "a group of radio buttons within a radio button field that use the same value
/// for the on state will turn on and off in unison".
///
/// **The same bit as [`FLAG_RICH_TEXT`], and deliberately two constants.** §12.7.4.1 makes `/Ff`
/// one flag word whose upper bits are read against the field's *type*, so bit 26 means one thing
/// on a `Btn` and another on a `Tx`; naming it once would make a reader believe the two were
/// related, which is the mistake the wrong table citation above is a smaller version of.
pub(crate) const FLAG_RADIOS_IN_UNISON: i64 = 1 << 25;
/// Table 233 bit 27: "the new value shall be committed as soon as a selection is made".
pub(crate) const FLAG_COMMIT_ON_SELECTION: i64 = 1 << 26;

/// The four field types §12.7.5.1 lists, with the flags that subdivide them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldKind {
    /// `Btn` (§12.7.5.2). `toggling` separates the two kinds that have an on state — a check
    /// box and a radio button — from a push-button, which has none.
    Button { toggling: bool },
    /// `Tx` (§12.7.5.3).
    Text,
    /// `Ch` (§12.7.5.4).
    Choice { combo: bool },
    /// `Sig` (§12.7.5.5).
    Signature,
}

/// A widget's field, read through §12.7.4.1's inheritance.
pub(crate) struct Field {
    pub(crate) kind: Option<FieldKind>,
    /// Table 227's `/Ff`.
    pub(crate) flags: i64,
    /// Table 226's `/V`.
    pub(crate) value: Option<Object>,
    /// The widget and its ancestors, nearest first, for the inheritable entries of Table 228
    /// that are read later.
    pub(crate) ancestry: Vec<Dictionary>,
    /// Whether the `/Parent` chain ran past [`MAX_FIELD_ANCESTRY`].
    pub(crate) too_deep: bool,
    /// Whether the value above is the file's own `/V`, or something that replaced it.
    ///
    /// Two clauses replace it and both have the same consequence for a check box, which is why
    /// this is one flag rather than two: §12.7.6.3's reset makes the value `/DV`, §12.7.8's
    /// import makes it another file's, and in either case the `/AS` in the document describes
    /// the state that was replaced.
    overridden: bool,
}

impl Field {
    /// Walks §12.7.4.1's `/Parent` chain once, taking each inheritable entry from the nearest
    /// dictionary that states it.
    ///
    /// > Many field attributes are inheritable , meaning that if they are not explicitly
    /// > specified for a given field, their values are taken from those of its parent in the
    /// > field hierarchy.
    ///
    /// One walk rather than one per entry, because the chain is the same for all of them and
    /// [`MAX_FIELD_ANCESTRY`] should bound the work once rather than once per key.
    pub(crate) fn read(
        document: &Document,
        annotation: &Dictionary,
        source: FieldValue<'_>,
    ) -> Self {
        let mut field = Self {
            kind: None,
            flags: 0,
            // Both of the values that come from outside this document's `/Parent` chain, and
            // neither is walked for: §12.7.8.3.2's "replace" puts another file's value here, and
            // an FDF field stating no `/V` leaves the widget with no value at all; what a person
            // entered arrives already in the object type Table 226's `/V` takes — §7.9.2.2's text
            // string for characters and for §12.7.5.2's state names, an array of them where
            // §12.7.5.4's field has several items selected.
            // `crate::view::ViewState::set_field` builds the second, once, so that this and the
            // file a save writes cannot disagree about what was chosen.
            value: match source {
                FieldValue::Imported { value, .. } | FieldValue::Edited { value, .. } => {
                    value.cloned()
                }
                FieldValue::Stored | FieldValue::Default => None,
            },
            ancestry: Vec::new(),
            too_deep: false,
            overridden: !matches!(source, FieldValue::Stored),
        };
        let stated_value = match source {
            FieldValue::Stored => Some("V"),
            // §12.7.6.3: the action "shall set the value of the V entry in the field dictionary
            // to that of the DV entry … If no default value is defined for a field, its V entry
            // shall be removed". So the *same* walk reads a different entry, and a field with no
            // `/DV` anywhere in its ancestry ends with no value at all — which is what the
            // clause's "removed" means for a program that does not write to the file.
            FieldValue::Default => Some("DV"),
            // Both come from outside the file, so there is nothing in this document's `/Parent`
            // chain to read them from and an absent value is an absent value.
            FieldValue::Imported { .. } | FieldValue::Edited { .. } => None,
        };
        let mut current = annotation.clone();
        let mut flags = None;
        let mut kind: Option<Vec<u8>> = None;
        for _ in 0..MAX_FIELD_ANCESTRY {
            if kind.is_none() {
                kind = document
                    .get_key(&current, "FT")
                    .as_name()
                    .map(|name| name.as_bytes().to_vec());
            }
            if flags.is_none() {
                flags = document.get_key(&current, "Ff").as_integer();
            }
            if let Some(key) = stated_value
                && field.value.is_none()
            {
                let value = document.get_key(&current, key);
                if !value.is_null() {
                    field.value = Some(value);
                }
            }
            let parent = document.get_key(&current, "Parent").as_dict().cloned();
            field.ancestry.push(current);
            let Some(parent) = parent else {
                // Table 249's `/Ff` "shall replace that of the Ff entry in the form's
                // corresponding field dictionary", and `/SetFf` and `/ClrFf` modify it — so the
                // import applies to whatever §12.7.4.1's inheritance produced, which is exactly
                // the flag word this walk has just finished computing.
                field.flags = match source {
                    FieldValue::Imported { flags: change, .. } => {
                        change.applied_to(flags.unwrap_or_default())
                    }
                    // Typing into a field changes its value and not its flags: §12.7.8's is the
                    // one statement about a value that carries Table 249's `/Ff` beside it.
                    FieldValue::Stored | FieldValue::Default | FieldValue::Edited { .. } => {
                        flags.unwrap_or_default()
                    }
                };
                field.kind = FieldKind::of(kind.as_deref(), field.flags);
                return field;
            };
            current = parent;
        }
        field.too_deep = true;
        field
    }

    /// Table 231's shape for a text field: comb, multiline, or one line.
    ///
    /// Table 231's own rule for bit 25 decides the order: Comb "may be set only if the `MaxLen`
    /// entry is present … and if the Multiline, Password, and `FileSelect` flags are clear",
    /// so
    /// a field claiming both is not a comb field.
    fn text_shape(&self, document: &Document, annotation: &Dictionary) -> Shape {
        if self.flags & FLAG_MULTILINE != 0 {
            return Shape::Multiline;
        }
        if self.flags & FLAG_COMB != 0 && self.flags & FLAG_PASSWORD == 0 {
            let sources: Vec<&Dictionary> = self.ancestry.iter().collect();
            let length = sources
                .iter()
                .find_map(|source| document.get_key(source, "MaxLen").as_integer())
                .or_else(|| document.get_key(annotation, "MaxLen").as_integer())
                .and_then(|value| u32::try_from(value).ok())
                .filter(|value| *value > 0);
            if let Some(length) = length {
                return Shape::Comb(length);
            }
        }
        Shape::SingleLine
    }

    /// Whether a check box or radio button is in its on state.
    ///
    /// §12.7.5.2.3 settles which entry decides, and it is not the value:
    ///
    /// > The value of the V key shall also be the value of the AS key. If they are not equal,
    /// > then the value of the AS key shall be used instead of the V key to determine which
    /// > appearance to use.
    ///
    /// So `/AS` wins wherever it is stated, and `/V` answers only for a widget that has none —
    /// which is a widget with no appearance dictionary, since Table 166 requires `/AS` whenever
    /// there is one. Either way `Off` is the off state, which §12.7.5.2.3 names and
    /// §12.7.5.2.4 gives as the default.
    pub(crate) fn is_on(&self, document: &Document, annotation: &Dictionary) -> bool {
        // §12.7.6.3 and §12.7.8 again: once the value has been replaced, the file's `/AS`
        // describes the state the widget was *saved* in, which is exactly what was replaced. So
        // such a widget answers from its new value alone, and a check box whose replacement is
        // unstated is off — which §12.7.5.2.4 gives as the default anyway.
        if self.overridden {
            return self.replacement_state(document, annotation) != OFF;
        }
        let state = document.get_key(annotation, "AS");
        let name = state
            .as_name()
            .map(|name| name.as_bytes().to_vec())
            .or_else(|| {
                self.value
                    .as_ref()
                    .and_then(Object::as_name)
                    .map(|name| name.as_bytes().to_vec())
            });
        name.is_some_and(|name| name != OFF)
    }

    /// Which of Table 170's appearance states a value **this reader replaced** puts this widget
    /// in.
    ///
    /// The other half of [`Field::is_on`]'s first branch, and the same clause read forwards
    /// instead of backwards. §12.7.5.2.3 states the invariant:
    ///
    /// > The value of the V key shall also be the value of the AS key.
    ///
    /// That sentence binds a *file*, and until the three-hundred-and-ninety-eighth session this
    /// tree read only its second half — the file's `/AS` decided, always. So a person who checked
    /// a box changed `/V` and nothing changed `/AS`, and the widget went on drawing the state it
    /// was saved in. The reader is now the one that changed `/V`, so the reader is what has to
    /// carry `/AS` with it.
    ///
    /// Two rules, one from each button subclause:
    ///
    /// - the name is the replaced value's, spelled as a name or as §7.9.2.2's text string —
    ///   [`crate::view::ViewState::set_field`] encodes what a host sends as the latter, and
    ///   Table 230 spells a button's export values as text strings for the same reason;
    /// - a widget whose `/AP` states no stream under that name is one of the *other* buttons of a
    ///   §12.7.5.2.4 set — "[t]he parent field's V entry holds a name object corresponding to the
    ///   appearance state of whichever child field is currently in the on state" — so it is off.
    ///   Applied only where the file states a state subdictionary to check against, because a
    ///   widget with no `/AP` at all has no states for the value to miss.
    fn replacement_state(&self, document: &Document, annotation: &Dictionary) -> Vec<u8> {
        let named = match self.value.as_ref() {
            Some(Object::Name(name)) => name.as_bytes().to_vec(),
            Some(Object::String(bytes)) => pdf_syntax::text_string(bytes).into_bytes(),
            // §12.7.5.2.4 gives `Off` as the default, and a value a reset or a clear removed is a
            // widget with no value at all.
            _ => return OFF.to_vec(),
        };
        let appearances = document.get_key(annotation, "AP");
        let states = appearances
            .as_dict()
            .map(|appearances| document.get_key(appearances, "N"));
        match states.as_ref().and_then(Object::as_dict) {
            Some(states) if states.get(&String::from_utf8_lossy(&named)).is_none() => OFF.to_vec(),
            _ if self.an_earlier_button_answers_to(document, annotation, &named) => OFF.to_vec(),
            _ => named,
        }
    }

    /// Table 229 bit 26 read the way round that needs code: whether this button must stay **off**
    /// because a button before it in `/Kids` answers to the same state name.
    ///
    /// ISO 32000-2 §12.7.5.2.1 makes this a requirement rather than a courtesy — "[f]or button
    /// fields, bits 15, 16, 17, and 26 shall indicate the intended behaviour of the button field.
    /// An interactive PDF processor shall follow the intended behaviour" — and Table 229's own
    /// row states both halves:
    ///
    /// > (PDF 1.5) If set, a group of radio buttons within a radio button field that use the same
    /// > value for the on state will turn on and off in unison; that is if one is checked, they are
    /// > all checked. If clear, the buttons are mutually exclusive (the same behaviour as HTML
    /// > radio buttons).
    ///
    /// **The half that needs code is the second, and this tree had it backwards for want of
    /// noticing.** `/V` is a name and a widget is on when its `/AP /N` holds a stream under that
    /// name, so two widgets sharing a name go on together *by construction* — the flag being
    /// **set** was already obeyed, by an implementation that had never read it. What was not obeyed
    /// is §12.7.5.2.3's sentence for the flag being clear:
    ///
    /// > For radio buttons, the same behaviour shall occur only if the `RadiosInUnison` flag is
    /// > set. If it is not set, at most one radio button in a field shall be set at a time.
    ///
    /// **Which one is a documented choice, because the clause states none and the file cannot.**
    /// The value is a name; a producer that gave two buttons the same name has written a document
    /// whose own `/V` cannot distinguish them, and Table 230 is the standard's instrument for a
    /// producer that wants them distinguishable — "the names used to represent the on state in the
    /// AP dictionary of each annotation may use numerical position (starting with 0) … This allows
    /// distinguishing between the annotations even if two or more of them have the same value in
    /// the Opt array". So the choice is the **first** kid that answers to the name, which is the
    /// field's own order and the order Table 230's `/Opt` is indexed by.
    ///
    /// **It binds a value this reader replaced and not the file's own `/AS`.** The rule is about
    /// what happens when a button is turned *on*, and the caller that turns one on is this program;
    /// a file that states `/AS` on two widgets has said which of its own buttons are on, and
    /// §12.7.5.2.3 gives that entry precedence — overriding it would be inventing a correction to a
    /// document rather than obeying a clause about a reader. [`Field::is_on`]'s first branch is the
    /// only caller, and `crate::appearance::appearance_state` reaches the drawing path through the
    /// same function, so the description a host reads and the picture the page draws cannot
    /// disagree about which button went on.
    fn an_earlier_button_answers_to(
        &self,
        document: &Document,
        annotation: &Dictionary,
        state: &[u8],
    ) -> bool {
        if self.flags & FLAG_PUSHBUTTON != 0
            || self.flags & FLAG_RADIO == 0
            || self.flags & FLAG_RADIOS_IN_UNISON != 0
            || state == OFF
        {
            return false;
        }
        // §12.7.5.2.4: "[t]he Kids entry in the radio button field's field dictionary holds an
        // array of widget annotations representing the individual buttons in the set." Taken from
        // the nearest ancestor that states one, which is the walk `/Ff` and `/FT` already took; a
        // field whose one widget is merged into it states none, and a set of one has nothing to be
        // exclusive with.
        let kids = self
            .ancestry
            .iter()
            .map(|source| document.get_key(source, "Kids"))
            .find(|value| value.as_array().is_some());
        let Some(kids) = kids.as_ref().and_then(Object::as_array) else {
            return false;
        };
        for kid in kids {
            let resolved = document.resolve(kid);
            let Some(button) = resolved.as_dict() else {
                continue;
            };
            if !holds_state(document, button, state) {
                continue;
            }
            // The first kid holding the name is the one that stays on. Two kids whose whole
            // dictionaries are equal are indistinguishable in every respect the standard names, so
            // comparing them as values rather than as references is not a weaker test than
            // comparing identities — it answers "off" for the second of the pair either way.
            return button != annotation;
        }
        false
    }
}

/// Whether a widget's `/AP /N` states a stream under this appearance state.
///
/// §12.7.5.2.3 makes those keys the states a button has: each state "can have a separate
/// appearance, which shall be defined by an appearance stream in the appearance dictionary of the
/// field's widget annotation".
fn holds_state(document: &Document, widget: &Dictionary, state: &[u8]) -> bool {
    let appearances = document.get_key(widget, "AP");
    let Some(appearances) = appearances.as_dict() else {
        return false;
    };
    let normal = document.get_key(appearances, "N");
    normal
        .as_dict()
        .is_some_and(|states| states.get(&String::from_utf8_lossy(state)).is_some())
}

impl Field {
    /// Whether the **field's** value names an on state, whichever of the four statements about it
    /// is current.
    ///
    /// [`Field::is_on`] is the same question asked of one *widget*, and the two differ exactly
    /// where §12.7.5.2.4's set does: "[t]he parent field's V entry holds a name object
    /// corresponding to the appearance state of whichever child field is currently in the on
    /// state", so the field is on when any of its buttons is and only the widget's own states say
    /// which. A value a host sent arrives as §7.9.2.2's text string rather than as a name, which
    /// is why both spellings are read — see [`Field::replacement_state`].
    pub(crate) fn value_is_on(&self) -> bool {
        match self.value.as_ref() {
            Some(Object::Name(name)) => name.as_bytes() != OFF,
            Some(Object::String(bytes)) => pdf_syntax::text_string(bytes).as_bytes() != OFF,
            // "the default value for this entry is Off" (§12.7.5.2.4).
            _ => false,
        }
    }

    /// Table 231 bit 13, which decides whether §12.7.4.3's layout may wrap.
    pub(crate) fn is_multiline(&self) -> bool {
        self.flags & FLAG_MULTILINE != 0
    }

    /// Table 231 bit 14, which decides whether the value may be shown at all.
    pub(crate) fn is_password(&self) -> bool {
        self.flags & FLAG_PASSWORD != 0
    }

    /// Table 231 bit 24, which decides whether more text is accepted once the field is full.
    pub(crate) fn does_not_scroll(&self) -> bool {
        self.flags & FLAG_DO_NOT_SCROLL != 0
    }

    /// Table 231 bit 25's cell count, where the bit is one the table permits.
    ///
    /// [`Field::text_shape`] is the same question asked for the *layout*, and this is it asked for
    /// a host's control — one reading of the table's condition on bit 25, so that a description
    /// and a drawing cannot disagree about whether a field is a comb.
    pub(crate) fn comb_cells(&self, document: &Document, annotation: &Dictionary) -> Option<u32> {
        match self.text_shape(document, annotation) {
            Shape::Comb(cells) => Some(cells),
            // `text_shape` reads Table 231, which is §12.7.5.3's alone, so it never answers
            // with the list box shape — that one is chosen by field *type* rather than by a
            // flag, in `field_text`. Named rather than swept into a wildcard so that a shape
            // added later has to be thought about here.
            Shape::SingleLine | Shape::Multiline | Shape::ListBox => None,
        }
    }
}

/// The off state, which §12.7.5.2.4 also gives as a toggling button's default value.
///
/// ISO 32000-2 §12.7.5.2.3:
///
/// > The appearance for the off state is optional but, if present, shall be stored in the
/// > appearance dictionary under the name Off .
pub(crate) const OFF: &[u8] = b"Off";

/// Which of Table 170's states an annotation shows, where **this reader** replaced its value.
///
/// `None` means the file's `/AS` decides, which is every annotation in a document nothing has been
/// done to and every widget that is not one of §12.7.5.2's two toggling kinds — a text field's
/// value is laid out rather than selected among, and a push-button holds no value at all
/// (§12.7.5.2.2). `Some(name)` is the state §12.7.5.2.3 says the new value selects.
///
/// Narrow on purpose: this is the one place a *viewer's* state is allowed to displace an entry the
/// file wrote, so it applies to the one field type whose appearance the value chooses.
pub(crate) fn appearance_state(
    document: &Document,
    annotation: &Dictionary,
    value: FieldValue<'_>,
) -> Option<Vec<u8>> {
    if matches!(value, FieldValue::Stored) {
        return None;
    }
    let field = Field::read(document, annotation, value);
    if !matches!(field.kind, Some(FieldKind::Button { toggling: true })) {
        return None;
    }
    Some(field.replacement_state(document, annotation))
}

impl FieldKind {
    /// Table 226's `/FT`, subdivided by the flags each type's own table defines.
    fn of(name: Option<&[u8]>, flags: i64) -> Option<Self> {
        match name? {
            b"Btn" => Some(Self::Button {
                toggling: flags & FLAG_PUSHBUTTON == 0,
            }),
            b"Tx" => Some(Self::Text),
            b"Ch" => Some(Self::Choice {
                combo: flags & FLAG_COMBO != 0,
            }),
            b"Sig" => Some(Self::Signature),
            // Table 226 lists four types. A fifth names no field this crate knows how to
            // display, and inventing one would be the guess §12.5.6.10 is refused for.
            _ => None,
        }
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
///
/// **Named beside a drawn line rather than instead of one, since the hundred-and-sixteenth
/// session.** `/LE` is optional and defaults to `[/None /None]`; `/L` and `/Vertices` are
/// required. So an annotation stating an ending this module cannot size has still stated the
/// line, and refusing the whole of it draws nothing where the clause states something. This is
/// ADR 0075's finding one entry over: an entry that cannot be derived is a reason to draw the
/// part that can be, not a reason to decline.
/// A `/LE` naming something Table 179 does not.
///
/// The ten styles are the whole of the table and the entry is "[a]n array of two names", so a
/// name outside it is a file asking for a shape this reader has no description of. Reported
/// rather than dropped to `None`, which would draw a line that quietly lost its arrowheads.
/// A `/LE` on a shape with no end this routine holds: a `/Path`, or fewer than two vertices.
const ENDINGS_WITH_NO_END: Refusal = Refusal::NotDerivable(
    "its /LE states line endings and its /Path or /Vertices gives no two points to put them on",
);

const UNKNOWN_LINE_ENDING: Refusal =
    Refusal::NotDerivable("its /LE names a line ending style Table 179 does not define");

/// A `/CL` whose length is neither of the two Table 177 states.
///
/// "An array of four or six numbers": four are a start and an end, six a start, a knee and an
/// end. A length outside those two states neither, and which of its numbers a reader should keep
/// is not something the table answers — so the line is refused and named rather than drawn from
/// a prefix the producer may not have meant.
const CALLOUT_SHAPE: Refusal = Refusal::NotDerivable(
    "its /CL states neither the four numbers of a two-point callout line nor the six of a \
     three-point one",
);

/// A `/CP` naming neither of the two placements Table 178 defines.
///
/// The table lists exactly two and tolerates no third, so a file stating a name outside them has
/// asked for the caption and said nothing about where it goes. Named rather than drawn at the
/// default: `/CP`'s default answers an *absent* entry, and using it for a stated one would put a
/// mark where the file did not ask for it.
const UNKNOWN_CAPTION_PLACEMENT: Refusal =
    Refusal::NotDerivable("its /CP names a caption placement Table 178 does not define");

/// A `/CO` that is present and is not the two numbers Table 178 states.
///
/// [`CALLOUT_SHAPE`]'s reasoning on the entry one subtype over: the table gives each of the two
/// numbers its own meaning, so an array of any other length says which is the horizontal offset
/// no more than a four-number `/CL` says which points are its ends.
const CAPTION_OFFSET_SHAPE: Refusal =
    Refusal::NotDerivable("its /CO states neither of the two numbers Table 178's offset is");

/// A `/Cap` on a line of no length.
///
/// §8.5.3.2 still draws the degenerate line, and `/CO`'s "along the annotation line" and
/// "perpendicular to the annotation line" name axes a line with no direction does not have — so
/// the caption has no frame to be placed in, and this says so beside the mark that was made.
const CAPTION_WITHOUT_A_LINE: Refusal = Refusal::NotDerivable(
    "its /Cap asks for a caption along a line whose two endpoints are the same point",
);

/// The size a caption is set at where its line is long enough to hold one.
///
/// **The one number here that the clause does not state, and it is not invented.** §12.5.6.7
/// says a caption "shall be replicated … in the appearance of the line" and states no size; the
/// standard's only worked example of laying variable text out is §12.7.5.3's, which sets two
/// lines at `/Ti 12 Tf` — the same example [`variable_text::LINE_HEIGHT`] takes its line spacing
/// from. So a caption is 12 points, and less where the line it sits on is too short for that.
const CAPTION_SIZE: f32 = 12.0;

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

/// The width §12.5.4 gives an annotation's border, whichever entry states it.
///
/// Table 166 settles the precedence — "[i]f an annotation dictionary includes the BS entry, then
/// the Border entry is ignored" — and §12.5.4 supplies the default the two share. Public to the
/// crate for §12.5.6.19's `/H /O`, which strokes that border and has no colour of its own to
/// read.
///
/// Errata Collection 3 sharpens that sentence to "shall be ignored" (Issue #287, `/State`
/// `Review` `Completed`), which is the precedence this reads either way; the quotation stays as
/// `doc/md/` has it, because the amended words are not in the copy the gate checks (ADR 0252).
pub(crate) fn border_width(document: &Document, annotation: &Dictionary) -> f32 {
    let width = if let Some(style) = document.get_key(annotation, "BS").as_dict() {
        Border::from_style_dictionary(document, style).0
    } else {
        document
            .get_key(annotation, "Border")
            .as_array()
            .and_then(|border| {
                border
                    .get(2)
                    .and_then(|item| document.resolve(item).as_number())
            })
            .map_or(DEFAULT_BORDER_WIDTH, |width| {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a border width is a small number of user space units"
                )]
                let width = width as f32;
                width
            })
    };
    if width.is_finite() {
        width.max(0.0)
    } else {
        0.0
    }
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
        Ok(Self::geometry(
            document,
            annotation,
            colour(document, source, key)?,
        ))
    }

    /// §12.5.4's width, style, dash and corner radii, stroked in a colour the caller supplies.
    ///
    /// Split from [`Self::read`] for §12.5.6.6, whose border has a geometry the standard states
    /// in full and a colour no clause states at all — see [`free_text_border`].
    fn geometry(document: &Document, annotation: &Dictionary, colour: Colour) -> Self {
        let entry = document.get_key(annotation, "Border");
        let border = entry.as_array().unwrap_or_default();

        // Table 166: "If an annotation dictionary includes the BS entry, then the Border entry
        // is ignored." §12.5.4 supplies the default width the two of them share. Errata
        // Collection 3 makes it "shall be ignored" (Issue #287) — the same precedence, stated
        // as a requirement.
        //
        // **The corner radii are part of what is ignored, and were read out of `/Border`
        // whatever `/BS` said until the four-hundred-and-fifty-eighth session.** They are the
        // one thing Table 166's array states that Table 168 has no entry for, which is what made
        // reading them beside a `/BS` look like completeness rather than the departure it is: a
        // `/BS` annotation that also carries `/Border [10 10 1]` is one whose border the standard
        // says is square, and this drew it round without a word.
        let (width, style, dash, radii) =
            if let Some(style) = document.get_key(annotation, "BS").as_dict() {
                let (width, style, dash) = Self::from_style_dictionary(document, style);
                (width, style, dash, [0.0, 0.0])
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
                let radii = [
                    number(document, border.first()).unwrap_or_default(),
                    number(document, border.get(1)).unwrap_or_default(),
                ];
                (width, style, dash.unwrap_or_default(), radii)
            };

        Self {
            colour,
            width: if width.is_finite() {
                width.max(0.0)
            } else {
                0.0
            },
            dash,
            style,
            radii,
        }
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
            // Table 168: "A single line along the bottom of the annotation rectangle", and
            // §12.5.4's sentence binds this style as much as the rectangular ones: "If present,
            // the border shall be drawn completely inside the annotation rectangle."
            //
            // **This centred the line on that edge until the four-hundred-and-fifty-eighth
            // session**, on a comment saying a stroke is centred on its path — which is true of
            // the stroke and says nothing about where the path goes. Half the line fell below
            // `/Rect`, where [`Constructed::bounded`]'s clip cut it off, so what a reader saw
            // was an underline half the width the document asked for rather than ink outside
            // the rectangle. The path is the bottom edge raised by half the width, the same
            // arithmetic [`Self::inset`] does for the other four styles, and the butt caps a
            // constructed appearance never changes keep the line's ends on the rectangle's own
            // sides.
            let bottom = self.inset(rect)[1];
            stream.move_to([rect[0], bottom]);
            stream.line_to([rect[2], bottom]);
        } else {
            stream.rounded_rectangle(self.inset(rect), self.radii);
        }
    }

    /// The report a `B` or `I` border owes: the rectangle is drawn, the illusion is not.
    ///
    /// Asked only by the subtypes whose `/BS` is a *border* — §12.5.6.5's link, §12.5.6.19's
    /// widget and, since [`free_text_border`], §12.5.6.6's note. §12.5.4 gives "line, square,
    /// circle, and ink annotations" a `/BS` that supplies
    /// "the width and dash pattern" alone, so on those four there is no style to be unable to
    /// draw; see [`square_or_circle`].
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
    /// Table 224's `/DR`, set by whichever routine laid text out.
    ///
    /// Read from the document only where a `/DA` string is, rather than for every
    /// construction: reaching the catalog and resolving `/AcroForm` per annotation cost 0.8%
    /// of the interpretation pass on a specification page full of link borders, none of which
    /// names a resource. Measured by callgrind on `examples/callgrind_interpret`.
    resources: Option<Dictionary>,
    /// Table 192's `/I`, under the name [`Self::form`] gave it.
    ///
    /// Held apart from [`Self::resources`] rather than written into it, because the two are
    /// filled in at opposite ends of a widget's construction: the icon is drawn before the
    /// caption so that §12.5.6.19's code 6 puts the caption *over* it, and the caption is what
    /// replaces the resource dictionary wholesale with `/DR`'s. Merging at the end is what keeps
    /// the drawing order from deciding which resource survives.
    icon: Option<(pdf_syntax::Name, Object)>,
}

impl Stream {
    fn new() -> Self {
        Self {
            text: String::new(),
            resources: None,
            icon: None,
        }
    }

    /// The resource dictionary this stream needs, with any icon added to its `/XObject`.
    fn finish(mut self) -> Dictionary {
        let mut resources = self.resources.take().unwrap_or_default();
        let Some((name, reference)) = self.icon else {
            return resources;
        };
        let key = pdf_syntax::Name::new(b"XObject".to_vec());
        let mut forms = resources
            .get_by_name(&key)
            .and_then(Object::as_dict)
            .cloned()
            .unwrap_or_default();
        forms.insert(name, reference);
        resources.insert(key, Object::Dictionary(forms));
        resources
    }

    /// Draws a form `XObject` under `placement`: `q a b c d e f cm /Name Do Q`.
    ///
    /// The name is chosen against `/DR`'s own `/XObject` names, because §12.7.4.3 makes those the
    /// constructed appearance's resources and a collision would draw the document's form instead
    /// of the icon.
    fn form(&mut self, document: &Document, reference: Object, placement: Transform) {
        let taken = default_resources(document);
        let taken = taken
            .get_by_name(&pdf_syntax::Name::new(b"XObject".to_vec()))
            .and_then(Object::as_dict)
            .cloned()
            .unwrap_or_default();
        let mut name = pdf_syntax::Name::new(b"Icon".to_vec());
        for suffix in 0..=u8::MAX {
            if taken.get_by_name(&name).is_none() {
                break;
            }
            name = pdf_syntax::Name::new(format!("Icon{suffix}").into_bytes());
        }
        let _ = writeln!(
            self.text,
            "q {} {} {} {} {} {} cm /{} Do Q",
            placement.a,
            placement.b,
            placement.c,
            placement.d,
            placement.e,
            placement.f,
            name.as_str().unwrap_or("Icon")
        );
        self.icon = Some((name, reference));
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

    /// Selects §11.3.5's `Multiply` blend mode, through an `/ExtGState` resource of its own.
    ///
    /// The only place a constructed appearance needs one; see [`text_markup`] for the
    /// argument that a highlight has to be one.
    fn multiply(&mut self) {
        let mut blend = Dictionary::new();
        blend.insert(
            pdf_syntax::Name::new(b"BM".to_vec()),
            Object::Name(pdf_syntax::Name::new(b"Multiply".to_vec())),
        );
        let mut states = Dictionary::new();
        states.insert(
            pdf_syntax::Name::new(b"Mul".to_vec()),
            Object::Dictionary(blend),
        );
        let mut resources = self.resources.take().unwrap_or_default();
        resources.insert(
            pdf_syntax::Name::new(b"ExtGState".to_vec()),
            Object::Dictionary(states),
        );
        self.resources = Some(resources);
        let _ = writeln!(self.text, "/Mul gs");
    }

    /// Opens a rotated space for Table 192's `/R`: `q a b c d e f cm`.
    fn rotate(&mut self, matrix: &str, offset: [f32; 2]) {
        let _ = writeln!(self.text, "q {matrix} {} {} cm", offset[0], offset[1]);
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
    /// A circle of a radius, centred on a point — [`Self::ellipse`] with one number instead of
    /// four, for Table 179's `Circle` ending, whose size is a length rather than a box.
    fn circle(&mut self, centre: [f32; 2], radius: f32) {
        self.ellipse([
            centre[0] - radius,
            centre[1] - radius,
            centre[0] + radius,
            centre[1] + radius,
        ]);
    }

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
/// of length 6 specify the operands for curveto operators." A `/Path` supersedes §12.5.6.9's
/// `/Vertices`, which Table 181 says "shall be ignored" where it is present — and it is drawn
/// ahead of §12.5.6.13's `/InkList` too, which **Table 185 does not say**: see [`ink`] for why
/// that one is this crate's choice rather than a rule.
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
fn line_endings(document: &Document, annotation: &Dictionary) -> Result<[Ending; 2], Refusal> {
    let entry = document.get_key(annotation, "LE");
    let Some(values) = entry.as_array() else {
        return Ok([Ending::None; 2]);
    };
    let mut endings = [Ending::None; 2];
    for (slot, value) in endings.iter_mut().zip(values) {
        let resolved = document.resolve(value);
        let Some(name) = resolved.as_name() else {
            // "An array of two names": an entry that is not a name states no style, and the
            // table's default answers an *absent* array rather than a malformed one.
            return Err(UNKNOWN_LINE_ENDING);
        };
        let Some(ending) = Ending::read(name.as_bytes()) else {
            return Err(UNKNOWN_LINE_ENDING);
        };
        *slot = ending;
    }
    Ok(endings)
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

/// Resolves one dictionary entry to a finite `f32`.
fn entry_number(document: &Document, dict: &Dictionary, key: &str) -> Option<f32> {
    let narrowed = narrow(document.get_key(dict, key).as_number()?);
    narrowed.is_finite().then_some(narrowed)
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
