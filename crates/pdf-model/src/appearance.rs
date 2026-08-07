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
use crate::variable_text::{self, Owed, Quadding, Request, Shape};
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
    /// **Four subtypes state their geometry in default user space and are therefore not bounded
    /// by it**: §12.5.6.7's `/L`, §12.5.6.9's `/Vertices`, §12.5.6.10's `/QuadPoints` and
    /// §12.5.6.13's `/InkList` are all "in default user space", which is the page's space and
    /// not a box's — so a file whose `/Rect` does not contain them has written a bounding box
    /// that is wrong, and the marks the clause states are still where the clause states them.
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
    value: FieldValue<'_>,
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
    );
    let outcome = match subtype {
        b"Link" => link(document, annotation, &mut stream),
        b"Square" | b"Circle" => square_or_circle(document, annotation, &mut stream, subtype),
        b"Polygon" | b"PolyLine" => polygon(document, annotation, &mut stream, subtype),
        b"Ink" => ink(document, annotation, &mut stream),
        b"Line" => line(document, annotation, &mut stream),
        b"Widget" => widget(document, annotation, &mut stream, value),
        b"Highlight" | b"Underline" | b"StrikeOut" | b"Squiggly" => {
            text_markup(document, annotation, &mut stream, subtype)
        }
        b"FreeText" => free_text(document, annotation, &mut stream),
        b"Text" => text_icon(document, annotation, &mut stream, rect),
        b"FileAttachment" | b"Sound" => symbol_icon(document, annotation, &mut stream, subtype),
        // §12.5.6.12's stamp is the one of the four icon clauses whose standard names are not
        // *objects*. Table 186's list is `Approved`, `Experimental`, `NotApproved`, `AsIs`,
        // `Expired`, `Draft` and the rest — legends rather than symbols, so drawing one means
        // inventing typography and a border, and what a reader would then see is a word this
        // program chose to set in a face this program chose. The clause says **should**, and a
        // recommendation is not a licence to invent a different kind of thing from the one the
        // name names. `doc/todo/26` holds the argument.
        b"Stamp" => Err(Refusal::NotDerivable(
            "its clause recommends rather than requires a predefined icon, and Table 186's \
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
    Constructed {
        content: painted.drawn.then(|| stream.text.into_bytes()),
        report: painted.report.map(|refusal| refusal.detail()),
        resources: stream.resources.unwrap_or_default(),
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
        None,
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
        match word.as_slice() {
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
/// - a push-button "retains no permanent value … it shall not use the V and DV entries"
///   (§12.7.5.2.2);
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
    let constructed = construct(document, annotation, b"Widget", value, rect);
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
/// `/Open` is not read. §12.5.6.4 gives it a popup window "containing the text of the note",
/// and [`crate::annotation`] draws no popup for any subtype, on the ground that a window is not
/// part of the page.
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
    let background = colour(document, annotation, "C")?;
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
    // As on a line annotation, and drawn since the three-hundred-and-fourteenth session. `/BE`
    // above is *not* the same case and stays a refusal — a cloudy border is a different border
    // rather than an extra mark, and drawing a straight one would put a shape on the page the
    // file did not describe.
    let endings = line_endings(document, annotation)?;

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
    // §12.5.6.9 gives a polygon no end to put one on — "the first and last vertex shall be
    // implicitly connected" — so this is the polyline half alone, which is also the only half
    // Table 181's `/IC` is a line-ending colour for.
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
    // colour states no mark, which is empty rather than unsupported.
    let colour = colour(document, annotation, "C")?;
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
/// Two entries are still owed and each states a different kind of nothing: `/LE`'s endings
/// name shapes with no size (Table 179 says "[a] square", "[t]wo short lines meeting in an
/// acute angle" — re-read in the eighty-fifth session and it still states no dimension), and
/// `/Cap` replicates `/Contents` as a caption, which needs a font no entry of a line annotation
/// supplies. **Both are named beside the drawn line rather than instead of it**, since the
/// hundred-and-sixteenth session: each is optional and additive where `/L` is required, so
/// declining the whole annotation for either drew nothing where the clause states a line. That
/// is the same reasoning the refusal above records for `/LL`, applied one entry over.
fn line(document: &Document, annotation: &Dictionary, stream: &mut Stream) -> Outcome {
    // `/Cap` is additive: it changes nothing about the line and is optional where `/L` is
    // required, so it is named beside the drawn line rather than instead of it.
    let captioned = matches!(document.get_key(annotation, "Cap"), Object::Boolean(true));
    let owed = captioned.then_some(LINE_CAPTION);
    let drawn = |painted: Painted| match (painted.drawn, owed) {
        // An annotation that draws nothing owes nothing: there is no line for a caption to sit
        // on, so naming it would report a gap on a blank page.
        (false, _) | (_, None) => Ok(painted),
        (true, Some(refusal)) => Ok(Painted::partly(refusal)),
    };

    let endings = line_endings(document, annotation)?;
    let ends = points(document, annotation, "L").unwrap_or_default();
    let (Some(start), Some(end)) = (ends.first().copied(), ends.get(1).copied()) else {
        return Err(Refusal::Missing("/L"));
    };
    let border = Border::read(document, annotation, annotation, "C")?;
    if !border.strokes() {
        return drawn(Painted::EMPTY);
    }
    border.apply(stream);
    let interior = colour(document, annotation, "IC")?;
    stream.set_colour(interior, false);

    let leader = entry_number(document, annotation, "LL").unwrap_or(0.0);
    let Some(offset) = perpendicular(start, end) else {
        // A line of no length has no direction to be perpendicular to, so its leader lines
        // have nowhere to go; the degenerate line itself is still §8.5.3.2's business — and an
        // ending has no direction to point in either.
        stream.move_to(start);
        stream.line_to(end);
        stream.paint(false, true);
        return drawn(Painted::DRAWN);
    };

    if leader == 0.0 {
        stream.move_to(start);
        stream.line_to(end);
        stream.paint(false, true);
        draw_endings(stream, endings, [start, end], border.width, interior);
        return drawn(Painted::DRAWN);
    }

    // "A non-negative number": a negative `/LLE` or `/LLO` states no length, so it is dropped
    // rather than reflected — the sign the clause gives a meaning to is `/LL`'s.
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
    // The line proper, at the leader lines' far end.
    stream.move_to(along(start, leader));
    stream.line_to(along(end, leader));
    // Each leader: from the offset the annotation states to the line, and `/LLE` past it.
    for point in [start, end] {
        stream.move_to(along(point, away * gap));
        stream.line_to(along(point, away.mul_add(extension, leader)));
    }
    stream.paint(false, true);
    // **On the line proper's ends, not on `/L`'s**, where the two differ. Table 178 says `/LE`
    // states the styles "for the endpoints defined … by the first and second pairs of
    // coordinates … in the L array", and says four rows earlier that with `/LL` present those
    // coordinates are "the endpoints of the leader lines rather than the endpoints of the line
    // itself". So the endpoints the first sentence names are not on the line; Figure 80 draws
    // the arrowhead on the line proper, and an ending is an ending *of a line*.
    draw_endings(
        stream,
        endings,
        [along(start, leader), along(end, leader)],
        border.width,
        interior,
    );
    drawn(Painted::DRAWN)
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
/// Table 192's `/R` is read nowhere yet and is the one entry a glyph makes load-bearing: it
/// rotates the widget's *contents* inside `/Rect`, which a background filling that rectangle
/// cannot see but a line of text can. It is reported where a widget both states one and has
/// text to put in it.
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
    let laid_out = match field_text(document, annotation, source, inner, value, None) {
        Ok(laid_out) => laid_out,
        Err(refusal) => {
            rotation.end(stream);
            return Ok(Painted {
                drawn: frame,
                report: Some(refusal),
            });
        }
    };
    let Some(laid_out) = laid_out else {
        rotation.end(stream);
        return Ok(if frame {
            border.simulated()
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
            .or(border.simulated().report),
    })
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

/// Lays out whatever text the field behind a widget states, if any.
///
/// One function because the field types differ only in where the text comes from; §12.7.4.3
/// does the same thing with all of them. `Ok(None)` is a field that states no text, which is
/// the common case and not a gap: 147 widgets on the corpus's first pages are empty text
/// fields waiting for a person.
///
/// `caret` is a byte offset into the *value*, asked for by [`caret`] and by nothing that draws:
/// where it is `Some`, an empty field is laid out rather than skipped, because a place for the
/// next character is exactly what an empty field can still be asked for.
fn field_text(
    document: &Document,
    annotation: &Dictionary,
    characteristics: &Dictionary,
    box_: [f32; 4],
    value: FieldValue<'_>,
    caret: Option<usize>,
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
            if value.is_empty() && caret.is_none() {
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
        // §12.7.5.4: the value "identifies the item or items currently selected". A combo box
        // shows the value in an edit box; a list box shows the whole `/Opt` array with the
        // selected items marked, and the clause states nothing about what a marked item looks
        // like — no highlight colour, no rule, nothing.
        FieldKind::Choice { combo: false } => return Err(Refusal::Text(Owed::ListBoxSelection)),
        FieldKind::Choice { combo: true } => {
            let value = field
                .value
                .as_ref()
                .and_then(|value| variable_text::value_text(document, value))
                .unwrap_or_default();
            if value.is_empty() && caret.is_none() {
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
        caret,
    };
    variable_text::lay_out(document, &request)
        .map(Some)
        .map_err(Refusal::Text)
}

/// Where the caret sits inside a widget's value, in **default user space**.
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
/// `None` for a field §12.7.4.3 lays no text out for — a button, a signature, a list box — and for
/// a widget whose value could not be laid out at all, which is the same condition that makes the
/// page report the field.
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
    let (box_, onto_page) = if let Some((bbox, placement)) =
        crate::annotation::stored_frame(document, annotation, view)
    {
        (inset(bbox, width), placement)
    } else {
        let rect = rectangle(document, annotation).ok()?;
        let rotation = Rotation::read(document, source)?;
        (
            inset(rotation.content_box(rect), width),
            rotation.transform(rect),
        )
    };
    let laid_out = field_text(document, annotation, source, box_, view.value, Some(offset))
        .ok()
        .flatten()?;
    let caret = laid_out.caret?;
    let from = onto_page.apply(Point::new(caret.from[0], caret.from[1]));
    let to = onto_page.apply(Point::new(caret.to[0], caret.to[1]));
    Some([from.x, from.y, to.x, to.y])
}

/// The text a text or combo-box field would be laid out with, as §12.7.4.3 sees it.
///
/// The value a *host* is told a field holds, which is not simply Table 226's `/V`: it is whichever
/// of the four statements about a value is current (`value`), read through §12.7.4.1's `/Parent`
/// chain and decoded from §7.9.2.2's text string type — and for a password field it is the bullets
/// Table 231 bit 14 requires be echoed instead, because a host may not be handed a secret it is
/// only allowed to draw as asterisks.
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
) -> Option<String> {
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
        Some(text)
    } else {
        Some("\u{2022}".repeat(text.chars().count()))
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
    let field = Field::read(document, annotation, FieldValue::Edited(None));
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
            caret: None,
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
/// Nothing is drawn around it. Table 177's `/BS` gives "the line width and dash pattern that
/// shall be used in drawing the annotation's border" and no clause states its *colour* — Table
/// 166's `/C` is the icon background, the popup title bar and a link's border, none of which a
/// free text annotation has. So a border is refused on the same grounds as §12.5.6.10's marks,
/// and reported only where a width is stated for it.
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
fn free_text(document: &Document, annotation: &Dictionary, stream: &mut Stream) -> Outcome {
    let rect = rectangle(document, annotation)?;
    let text = variable_text::string(document, &[annotation], "Contents")
        .filter(|contents| !contents.is_empty())
        .or_else(|| crate::popup::rich_text(document, annotation));
    let Some(text) = text.filter(|text| !text.is_empty()) else {
        return Ok(Painted::EMPTY);
    };

    let form = interactive_form(document).unwrap_or_default();
    let sources = [annotation, &form];
    let Some(default_appearance) = variable_text::bytes(document, &sources, "DA") else {
        return Err(Refusal::Text(Owed::NoFont));
    };
    let resources = default_resources(document);
    let request = Request {
        text: &text,
        // §12.5.6.6's `/RD` uses the same left, top, right, bottom order §12.5.6.8's does,
        // which `differences` already reads.
        box_: differences(document, annotation, rect),
        default_appearance: &default_appearance,
        resources: &resources,
        quadding: Quadding::read(document, &sources),
        // Table 177 states no single-line free text: the annotation is a box of prose.
        shape: Shape::Multiline,
        // §12.5.6.6's text is not a field and nothing types into it — see `doc/todo/33`.
        caret: None,
    };
    let laid_out = variable_text::lay_out(document, &request).map_err(Refusal::Text)?;
    stream.text.push_str(&laid_out.content);
    stream.resources = Some(with_stand_in_font(resources, laid_out.font));

    let bordered =
        Border::read(document, annotation, annotation, "C").is_ok_and(|border| border.width > 0.0);
    Ok(Painted {
        drawn: true,
        report: laid_out.owed.map(Refusal::Text).or_else(|| {
            bordered.then_some(Refusal::NotDerivable(
                "Table 177's /BS gives its border a width, and no clause states the colour",
            ))
        }),
    })
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

/// Table 227 bit 13: "the field may contain multiple lines of text".
const FLAG_MULTILINE: i64 = 1 << 12;
/// Table 231 bit 14: the field "is intended for entering a secure password".
const FLAG_PASSWORD: i64 = 1 << 13;
/// Table 229 bit 17: "If set, the field is a push-button that does not retain a permanent
/// value."
const FLAG_PUSHBUTTON: i64 = 1 << 16;
/// Table 233 bit 18: "If set, the field is a combo box; if clear, the field is a list box."
const FLAG_COMBO: i64 = 1 << 17;
/// Table 231 bit 24: "If set, the field shall not scroll … to accommodate more text than fits
/// within its annotation rectangle."
const FLAG_DO_NOT_SCROLL: i64 = 1 << 23;
/// Table 231 bit 25: the field "shall be automatically divided into as many equally spaced
/// positions, or combs, as the value of `MaxLen`".
const FLAG_COMB: i64 = 1 << 24;

/// The four field types §12.7.5.1 lists, with the flags that subdivide them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldKind {
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
struct Field {
    kind: Option<FieldKind>,
    /// Table 227's `/Ff`.
    flags: i64,
    /// Table 226's `/V`.
    value: Option<Object>,
    /// The widget and its ancestors, nearest first, for the inheritable entries of Table 228
    /// that are read later.
    ancestry: Vec<Dictionary>,
    /// Whether the `/Parent` chain ran past [`MAX_FIELD_ANCESTRY`].
    too_deep: bool,
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
    fn read(document: &Document, annotation: &Dictionary, source: FieldValue<'_>) -> Self {
        let mut field = Self {
            kind: None,
            flags: 0,
            // §12.7.8.3.2's "replace" is done here and not by a walk: an imported value comes
            // from another file, so there is nothing in this document's `/Parent` chain to read
            // it from, and an FDF field stating no `/V` leaves the widget with no value at all.
            value: match source {
                FieldValue::Imported { value, .. } => value.cloned(),
                // What a person typed, encoded as §7.9.2.2's text string type — which is what
                // Table 226 makes `/V` for a text field, and so what §12.7.4.3 lays out.
                FieldValue::Edited(text) => text.map(|text| {
                    Object::String(pdf_syntax::text_string::encode_text_string(text).into())
                }),
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
            FieldValue::Imported { .. } | FieldValue::Edited(_) => None,
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
                    FieldValue::Stored | FieldValue::Default | FieldValue::Edited(_) => {
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
    fn is_on(&self, document: &Document, annotation: &Dictionary) -> bool {
        // §12.7.6.3 and §12.7.8 again: once the value has been replaced, the file's `/AS`
        // describes the state the widget was *saved* in, which is exactly what was replaced. So
        // such a widget answers from its new value alone, and a check box whose replacement is
        // unstated is off — which §12.7.5.2.4 gives as the default anyway.
        if self.overridden {
            return self
                .value
                .as_ref()
                .and_then(Object::as_name)
                .is_some_and(|name| name.as_bytes() != b"Off");
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
        name.is_some_and(|name| name != b"Off")
    }
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

/// §12.5.6.7's `/Cap`, which replicates `/Contents` "as a caption in the appearance of the
/// line" and gives no entry from which to take a font.
///
/// Additive like `/LE`, and named the same way: the line is drawn and the caption is not.
const LINE_CAPTION: Refusal = Refusal::NotDerivable(
    "§12.5.6.7's /Cap asks for /Contents as a caption, and no entry gives it a font",
);

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
    /// Table 224's `/DR`, set by whichever routine laid text out.
    ///
    /// Read from the document only where a `/DA` string is, rather than for every
    /// construction: reaching the catalog and resolving `/AcroForm` per annotation cost 0.8%
    /// of the interpretation pass on a specification page full of link borders, none of which
    /// names a resource. Measured by callgrind on `examples/callgrind_interpret`.
    resources: Option<Dictionary>,
}

impl Stream {
    fn new() -> Self {
        Self {
            text: String::new(),
            resources: None,
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
