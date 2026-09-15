//! `redact` — applying a redaction annotation, ISO 32000-2 §12.5.6.23.
//!
//! The owner's `doc/questions/A64`: applying a redaction is in scope. §12.5.6.23's second
//! phase — "remove all content identified by the redaction annotation, as well as the
//! annotation itself … remove all traces of the specified content" — is a *removal*, which is
//! why it is a **write-new-file** operation through RFC 0002's serializer and never §7.5.6's
//! incremental update: an update appends and leaves the producer's bytes in the file byte for
//! byte, and a removal that left the bytes would be no redaction at all (principle 1: a
//! redaction that only covers is a security defect).
//!
//! # What is removed, and how the bytes go
//!
//! Table 195 gives the region: `/QuadPoints` where present, else `/Rect`. "Within the region"
//! is **bounding-box intersection** — the choice §12.5.6.23 leaves open, recorded as a choice
//! (principle 5): centre-in-region leaves legible half-glyphs and containment leaks by
//! construction, so the box that *touches* the region is removed.
//!
//! The page is interpreted **without `/Annots`** — [`crate::render::page_to_draw`] with
//! `annotations` false — because [`pdf_model::content::Interpretation::text_layer`] otherwise
//! carries the §12.5.3 annotation-appearance pass as well as the content, and the removal reads
//! only `/Contents`. Each code the interpreter placed has a quadrilateral in the display
//! list's own space ([`pdf_model::content::base_transform`] maps the region into the same
//! space); a code whose box meets the region is deleted from the content stream. A deleted
//! glyph's advance is restored as a §9.4.3 `TJ` adjustment so the surviving text does not
//! slide, and the adjustment needs **no font metrics**: §9.4.4's `w0` is the placed
//! quadrilateral's own advance vector, read back through the text-rendering matrix the walk
//! tracks. The walk is held to the interpreter by a **code count**: where the two disagree the
//! page is refused rather than cut wrong (trap 13's calibration).
//!
//! # What is refused, never cut wrong (trap 5)
//!
//! A page is refused by name — its content and its `/Redact` annotations left as the file
//! wrote them — where removal cannot be proven to leave no trace: a Type 3 font (§9.6.5's glyph
//! procedures draw outside the advance box this walk measures), a composite font not encoded
//! `Identity-H` (§9.7.5's codespace decides the code-byte width), the `sh` operator (§8.7.4.2
//! paints the whole clip), a soft-mask group, an image or a painted path or a form meeting the
//! region, or any code count the interpreter does not confirm. An encrypted document is refused
//! outright, because this writer emits no `/Encrypt`.
//!
//! # What is a documented departure (A64/A65's fence)
//!
//! Table 195's `/OverlayText`, `/IC` and `/RO` describe the mark drawn *over* the region after
//! removal, and composing them is composing content the document did not hold — the far side of
//! `CLAUDE.md`'s authoring line. So the removal happens and the overlay is not drawn, reported
//! as a per-page [`crate::Departure`] from §12.5.6.23's full application semantics.

use std::fmt::Write as _;
use std::io::Write as _;
use std::sync::Arc;

use pdf_model::content::{Interpretation, base_transform, interpret};
use pdf_model::{Page, Pages};
use pdf_render::Transform;
use pdf_render::geom::Point;
use pdf_syntax::Document;
use pdf_syntax::lexer::{Lexer, Token};
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::{Assembly, Form, Options, serialize};

use crate::optimize::{catalog_of, copy_closure, refuse_a_document_only_recovery_reads};
use crate::pattern::{Fill, Pattern};
use crate::{Declined, Departure, Origin, Output, Refusal, Report, Sinks, Warning};

/// The deepest a reference chain is followed when carrying a replaced page's entries.
const MAX_DEPTH: usize = 64;

/// One document's `/Redact` annotations applied, and the result written as a new file.
#[derive(Debug, Clone, PartialEq)]
pub struct RedactPlan {
    /// Which source.
    pub source: usize,
    /// How the output is named.
    pub names: Pattern,
}

/// Applies every `/Redact` annotation this document states and writes the redacted document.
///
/// `at` is the document's position among the opened ones — one, for this verb.
///
/// # Errors
///
/// [`Refusal::NoSuchSource`], [`Refusal::Reconstructed`] where the document's structure is only
/// §C.4's recovery, [`Refusal::Assembly`] where the document is encrypted or cannot be
/// rewritten, and [`Refusal::Sink`] where the output cannot be written.
pub(crate) fn run(
    plan: &RedactPlan,
    at: usize,
    documents: &[Document],
    sinks: &dyn Sinks,
    report: &mut Report,
) -> Result<(), Refusal> {
    let document = documents.get(at).ok_or(Refusal::NoSuchSource {
        at: plan.source,
        count: documents.len(),
    })?;
    let root = catalog_of(document)?;
    refuse_a_document_only_recovery_reads(document, root)?;
    if document.is_encrypted() {
        // §7.6: this writer emits no /Encrypt, so a copied stream's bytes would be written
        // without the key that reads them, and the original encrypted content would sit in the
        // file besides. Both are the far side of "remove all traces", so the whole operation is
        // refused rather than a redaction produced whose removed content might be recoverable.
        return Err(Refusal::Assembly(
            "§7.6: this document is encrypted, and redaction is refused rather than write a file \
             whose removed content might be recoverable"
                .to_owned(),
        ));
    }

    let pages = Pages::new(document);
    let mut applied: Vec<AppliedPage> = Vec::new();
    let mut annotations = 0usize;
    let mut glyphs = 0usize;
    let mut departures: Vec<usize> = Vec::new();

    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let Some(page_id) = page.id else {
            continue;
        };
        let regions = redactions(document, &page);
        if regions.is_empty() {
            continue;
        }
        match plan_page(document, &page, &regions) {
            Ok(edit) => {
                annotations = annotations.saturating_add(regions.len());
                glyphs = glyphs.saturating_add(edit.removed);
                if regions.iter().any(|region| region.has_overlay) {
                    departures.push(index.saturating_add(1));
                }
                applied.push(AppliedPage {
                    page_id,
                    placed: page_id,
                    content: edit.content,
                });
            }
            Err(skip) => report.refused.push(Declined {
                source: plan.source,
                page: Some(index.saturating_add(1)),
                subject: "§12.5.6.23 redaction".to_owned(),
                detail: skip,
            }),
        }
    }

    let written = write_document(document, root, &mut applied, plan, sinks)?;
    if written.dangling {
        report.warnings.push(Warning {
            source: plan.source,
            page: None,
            detail: "§7.3.10: a reference named an object this document does not hold and was \
                     written as null"
                .to_owned(),
        });
    }
    report.outputs.push(Output {
        name: written.name,
        bytes: written.bytes,
        sanitised: written.sanitised,
        origin: Origin::Redacted {
            source: plan.source,
            pages: pages.len(),
            annotations,
            glyphs,
        },
    });
    for page in departures {
        report.departures.push(Departure {
            source: plan.source,
            page: Some(page),
            detail: "§12.5.6.23: the content was removed; Table 195's overlay (/OverlayText, \
                     /IC or /RO) was not composed, because it is a mark this program does not \
                     author (doc/questions/A64, A65's provenance fence)"
                .to_owned(),
        });
    }
    Ok(())
}

/// A page whose redactions were applied: the object it is, the slot it takes in the output, and
/// the content stream with the removed bytes gone.
struct AppliedPage {
    page_id: ObjectId,
    placed: ObjectId,
    content: Vec<u8>,
}

/// One `/Redact` annotation's region and whether it states an overlay appearance.
struct Redaction {
    /// The region to remove, as bounding boxes in default user space.
    boxes: Vec<[f32; 4]>,
    /// Whether Table 195's `/OverlayText`, `/IC` or `/RO` is present — the departure's trigger.
    has_overlay: bool,
}

/// Every `/Redact` annotation on a page, with its region read from `/QuadPoints` or `/Rect`.
fn redactions(document: &Document, page: &Page) -> Vec<Redaction> {
    let annots = document.get_key(&page.dict, "Annots");
    let Some(items) = annots.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items {
        let resolved = document.resolve(item);
        let Some(dict) = resolved.as_dict() else {
            continue;
        };
        if document
            .get_key(dict, "Subtype")
            .as_name()
            .is_none_or(|name| name.as_bytes() != b"Redact")
        {
            continue;
        }
        let boxes = region_boxes(document, dict);
        if boxes.is_empty() {
            continue;
        }
        let has_overlay = ["RO", "OverlayText", "IC"]
            .iter()
            .any(|key| !matches!(document.get_key(dict, key), Object::Null));
        out.push(Redaction { boxes, has_overlay });
    }
    out
}

/// Table 195's region: `/QuadPoints`' `8×n` quadrilaterals as bounding boxes, else `/Rect`.
fn region_boxes(document: &Document, dict: &Dictionary) -> Vec<[f32; 4]> {
    if let Some(values) = numbers(document, dict, "QuadPoints")
        && !values.is_empty()
        && values.len().is_multiple_of(8)
    {
        return values.chunks_exact(8).map(bbox_of_points).collect();
    }
    match numbers(document, dict, "Rect") {
        Some(rect) if rect.len() >= 4 => vec![[
            rect[0].min(rect[2]),
            rect[1].min(rect[3]),
            rect[0].max(rect[2]),
            rect[1].max(rect[3]),
        ]],
        _ => Vec::new(),
    }
}

/// An array of numbers, each resolved and finite; `None` where the entry is not such an array.
fn numbers(document: &Document, dict: &Dictionary, key: &str) -> Option<Vec<f32>> {
    let values: Vec<f32> = document
        .get_key(dict, key)
        .as_array()?
        .iter()
        .filter_map(|item| document.resolve(item).as_number().map(narrow))
        .filter(|value| value.is_finite())
        .collect();
    Some(values)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "a coordinate outside f32's range cannot place anything on a page"
)]
fn narrow(value: f64) -> f32 {
    value as f32
}

/// The axis-aligned bounding box of a flat `[x0, y0, x1, y1, …]` point list.
fn bbox_of_points(points: &[f32]) -> [f32; 4] {
    let mut bbox = [
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    for pair in points.chunks_exact(2) {
        bbox[0] = bbox[0].min(pair[0]);
        bbox[1] = bbox[1].min(pair[1]);
        bbox[2] = bbox[2].max(pair[0]);
        bbox[3] = bbox[3].max(pair[1]);
    }
    bbox
}

/// Two axis-aligned boxes overlap, touching edges excluded (a touch removes nothing).
fn overlaps(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1]
}

/// A page's content stream with the region's content removed, and how many glyphs went.
struct PageEdit {
    content: Vec<u8>,
    removed: usize,
}

/// Plans one page's removal, or refuses it by name (trap 5: never cut it wrong).
fn plan_page(document: &Document, page: &Page, regions: &[Redaction]) -> Result<PageEdit, String> {
    let draw = crate::render::page_to_draw(page, None, false);
    let interpretation = interpret(document, &draw);
    let base = base_transform(page);
    let region_boxes: Vec<[f32; 4]> = regions
        .iter()
        .flat_map(|region| region.boxes.iter())
        .map(|user| map_box(base, *user))
        .collect();

    let content = page.content(document);
    let walk = Walk::new(document, page, &interpretation, region_boxes);
    walk.run(&content)
}

/// A user-space box mapped through the base transform into the display list's coordinates.
///
/// The four corners are mapped and their bounding box taken, so a rotation on the page widens
/// the box rather than shearing it — which over-removes and never under-removes (safe).
fn map_box(base: Transform, user: [f32; 4]) -> [f32; 4] {
    let corners = [
        base.apply(Point::new(user[0], user[1])),
        base.apply(Point::new(user[2], user[1])),
        base.apply(Point::new(user[2], user[3])),
        base.apply(Point::new(user[0], user[3])),
    ];
    let flat: Vec<f32> = corners
        .iter()
        .flat_map(|point| [point.x, point.y])
        .collect();
    bbox_of_points(&flat)
}

/// How a font's codes are read from a show string's bytes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CodeWidth {
    /// A simple font (§9.6): one byte, one code.
    One,
    /// A composite font encoded `Identity-H` (§9.7.5.2): two bytes, one code.
    Two,
}

impl CodeWidth {
    fn bytes(self) -> usize {
        match self {
            Self::One => 1,
            Self::Two => 2,
        }
    }
}

/// One operand read while scanning between operators.
enum Operand {
    Number(f64),
    Name(Vec<u8>),
    Str(Vec<u8>),
    Array(Vec<ArrayElement>),
    Other,
}

/// One element of a `TJ` array (§9.4.3): a shown string or a positioning adjustment.
enum ArrayElement {
    Number(f64),
    Str(Vec<u8>),
}

/// The content-stream walk: it enumerates codes in the interpreter's order, tests each placed
/// quadrilateral against the region, and edits the bytes.
struct Walk<'a> {
    document: &'a Document,
    page: &'a Page,
    /// One quadrilateral per code the interpreter placed, in content order — copied out of the
    /// interpretation because the walk indexes them and nothing else keeps them alive.
    quads: Vec<[f32; 8]>,
    regions: Vec<[f32; 4]>,
    ctm: Transform,
    ctm_stack: Vec<Transform>,
    text_linear: Transform,
    font_size: f32,
    horizontal: f32,
    char_spacing: f32,
    word_spacing: f32,
    width: Option<CodeWidth>,
    path: Option<[f32; 4]>,
    code_index: usize,
    removed: usize,
    edits: Vec<(usize, usize, Vec<u8>)>,
}

impl<'a> Walk<'a> {
    fn new(
        document: &'a Document,
        page: &'a Page,
        interpretation: &Interpretation,
        regions: Vec<[f32; 4]>,
    ) -> Self {
        Self {
            document,
            page,
            quads: interpretation
                .text_layer
                .iter()
                .map(|placed| placed.quad)
                .collect(),
            regions,
            ctm: base_transform(page),
            ctm_stack: Vec::new(),
            text_linear: Transform::IDENTITY,
            font_size: 0.0,
            horizontal: 1.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            width: None,
            path: None,
            code_index: 0,
            removed: 0,
            edits: Vec::new(),
        }
    }

    /// Walks the whole content stream, returning the edited bytes or a refusal by name.
    fn run(mut self, content: &[u8]) -> Result<PageEdit, String> {
        let mut lexer = Lexer::at(content, 0);
        let mut operands: Vec<(Operand, usize)> = Vec::new();
        loop {
            lexer.skip_whitespace();
            let start = lexer.position();
            let Some(token) = lexer.next_token() else {
                break;
            };
            match token {
                Token::Integer(value) => {
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "a content-stream integer is a coordinate, far inside f64"
                    )]
                    operands.push((Operand::Number(value as f64), start));
                }
                Token::Real(value) => operands.push((Operand::Number(value), start)),
                Token::Name(bytes) => operands.push((Operand::Name(bytes), start)),
                Token::String(bytes) => operands.push((Operand::Str(bytes), start)),
                Token::ArrayOpen => {
                    operands.push((Operand::Array(collect_array(&mut lexer)), start));
                }
                Token::DictOpen => {
                    skip_dictionary(&mut lexer);
                    operands.push((Operand::Other, start));
                }
                Token::ArrayClose | Token::DictClose => {}
                Token::Keyword(keyword) => {
                    if keyword == b"BI" {
                        self.inline_image(content, &mut lexer)?;
                    } else {
                        self.operator(keyword, start, &operands)?;
                    }
                    operands.clear();
                }
            }
        }
        if self.code_index != self.quads.len() {
            return Err(format!(
                "the content walk found {} code(s) where the interpreter placed {}; the page is \
                 refused rather than cut against a walk the interpreter does not confirm",
                self.code_index,
                self.quads.len()
            ));
        }
        Ok(PageEdit {
            content: apply_edits(content, self.edits),
            removed: self.removed,
        })
    }

    /// Dispatches one operator against the operands scanned before it.
    fn operator(
        &mut self,
        keyword: &[u8],
        keyword_start: usize,
        operands: &[(Operand, usize)],
    ) -> Result<(), String> {
        match keyword {
            b"q" => self.ctm_stack.push(self.ctm),
            b"Q" => {
                if let Some(previous) = self.ctm_stack.pop() {
                    self.ctm = previous;
                }
            }
            b"cm" => {
                if let Some(matrix) = matrix(&plain_numbers(operands)) {
                    self.ctm = matrix.then(self.ctm);
                }
            }
            b"BT" => self.text_linear = Transform::IDENTITY,
            b"Tm" => {
                if let Some(matrix) = matrix(&plain_numbers(operands)) {
                    self.text_linear = linear(matrix);
                }
            }
            b"Tf" => self.select_font(operands)?,
            b"Tz" => {
                if let Some(scale) = plain_numbers(operands).first() {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "§9.3.4 horizontal scaling is a percentage, far inside f32"
                    )]
                    {
                        self.horizontal = (*scale as f32) / 100.0;
                    }
                }
            }
            b"Tc" => self.char_spacing = first(&plain_numbers(operands)),
            b"Tw" => self.word_spacing = first(&plain_numbers(operands)),
            b"Tj" | b"'" => self.show_one(operands, keyword, keyword_start)?,
            b"\"" => self.show_quote(operands)?,
            b"TJ" => self.show_array(operands, keyword_start)?,
            b"m" | b"l" | b"c" | b"v" | b"y" => self.extend_path(&plain_numbers(operands)),
            b"re" => self.extend_rectangle(&plain_numbers(operands)),
            b"f" | b"F" | b"f*" | b"S" | b"s" | b"B" | b"B*" | b"b" | b"b*" => {
                self.paint_path(keyword)?;
            }
            b"n" => self.path = None,
            b"Do" => self.do_xobject(operands)?,
            b"sh" => {
                return Err(
                    "§8.7.4.2: the sh operator paints the whole clip, which no byte-range \
                            edit can redact in part; the page is refused"
                        .to_owned(),
                );
            }
            b"gs" => self.ext_gstate(operands)?,
            _ => {}
        }
        Ok(())
    }

    /// `Tf`: reads the font and settles how many bytes one of its codes is (§9.6, §9.7.5).
    fn select_font(&mut self, operands: &[(Operand, usize)]) -> Result<(), String> {
        let name = operands.iter().find_map(|(operand, _)| match operand {
            Operand::Name(bytes) => Some(bytes.clone()),
            _ => None,
        });
        let size = plain_numbers(operands).last().copied();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "§9.3.1 text font size is a coordinate, far inside f32"
        )]
        {
            self.font_size = size.unwrap_or(0.0) as f32;
        }
        self.width = match name {
            Some(name) => Some(self.code_width(&name)?),
            None => None,
        };
        Ok(())
    }

    /// The code-byte width of the named font, or a refusal for a font the walk will not cut.
    fn code_width(&self, name: &[u8]) -> Result<CodeWidth, String> {
        let fonts = self.document.get_key(&self.page.resources, "Font");
        let font = fonts
            .as_dict()
            .and_then(|fonts| fonts.get_by_name(&Name::new(name)))
            .map(|entry| self.document.resolve(entry));
        let Some(font) = font.as_ref().and_then(Object::as_dict) else {
            return Err(format!(
                "the content shows /{} which /Resources /Font does not define; the page is \
                 refused rather than guess a code width",
                String::from_utf8_lossy(name)
            ));
        };
        let subtype = self
            .document
            .get_key(font, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec())
            .unwrap_or_default();
        match subtype.as_slice() {
            b"Type1" | b"TrueType" | b"MMType1" => Ok(CodeWidth::One),
            b"Type3" => Err(
                "§9.6.5: a Type 3 font's glyphs are content streams that may draw outside the \
                 advance box this walk measures; the page is refused"
                    .to_owned(),
            ),
            b"Type0" => self.composite_width(font),
            other => Err(format!(
                "the content shows a /{} font, whose code width this walk does not settle; the \
                 page is refused",
                String::from_utf8_lossy(other)
            )),
        }
    }

    /// A Type 0 font's width: two bytes for `Identity-H`, else a refusal (§9.7.5 codespace).
    fn composite_width(&self, font: &Dictionary) -> Result<CodeWidth, String> {
        if self
            .document
            .get_key(font, "Encoding")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Identity-H")
        {
            return Ok(CodeWidth::Two);
        }
        Err(
            "§9.7.5: a composite font encoded by anything but Identity-H has a codespace that \
             decides the code-byte width; the page is refused"
                .to_owned(),
        )
    }

    /// `Tj`/`'`: one string. `'` (§9.4.3) is `T*` then a show, which does not move the text
    /// matrix's linear part, so its rewriting is the same as `Tj`'s but keeps the line advance.
    fn show_one(
        &mut self,
        operands: &[(Operand, usize)],
        keyword: &[u8],
        keyword_start: usize,
    ) -> Result<(), String> {
        let Some((bytes, start)) = operands.iter().find_map(|(operand, start)| match operand {
            Operand::Str(bytes) => Some((bytes.clone(), *start)),
            _ => None,
        }) else {
            return Ok(());
        };
        let elements = [ArrayElement::Str(bytes)];
        if let Some(inner) = self.rewrite_show(&elements)? {
            let prefix = if keyword == b"'" { "T* " } else { "" };
            let replacement = format!("{prefix}{inner} TJ").into_bytes();
            let end = keyword_start.saturating_add(keyword.len());
            self.edits.push((start, end, replacement));
        }
        Ok(())
    }

    /// `"` (§9.4.3): `aw ac string "`. A `"` with anything removed is refused, because
    /// restating its spacing exactly beside a `TJ` gap is not something this walk does.
    fn show_quote(&mut self, operands: &[(Operand, usize)]) -> Result<(), String> {
        let Some(bytes) = operands.iter().find_map(|(operand, _)| match operand {
            Operand::Str(bytes) => Some(bytes.clone()),
            _ => None,
        }) else {
            return Ok(());
        };
        if self.string_meets_region(&bytes)? {
            return Err(
                "§9.4.3: the \" operator sets word and character spacing as it shows; a \
                        redaction inside it is refused rather than cut with the spacing wrong"
                    .to_owned(),
            );
        }
        Ok(())
    }

    /// `TJ` (§9.4.3): an array of strings and adjustments.
    fn show_array(
        &mut self,
        operands: &[(Operand, usize)],
        keyword_start: usize,
    ) -> Result<(), String> {
        let Some((elements, start)) = operands.iter().find_map(|(operand, start)| match operand {
            Operand::Array(elements) => Some((elements, *start)),
            _ => None,
        }) else {
            return Ok(());
        };
        if let Some(inner) = self.rewrite_show(elements)? {
            let replacement = format!("{inner} TJ").into_bytes();
            let end = keyword_start.saturating_add(2);
            self.edits.push((start, end, replacement));
        }
        Ok(())
    }

    /// Rewrites a show operator's elements, deleting the codes that meet the region and
    /// restoring their advance as adjustments. `None` means nothing was removed, so the source
    /// bytes are left untouched (the "outside byte-identical" guarantee).
    fn rewrite_show(&mut self, elements: &[ArrayElement]) -> Result<Option<String>, String> {
        let width = self.require_width()?;
        let scale = self.advance_scale();
        let refusal = if self.font_size == 0.0 || self.horizontal == 0.0 {
            Some(
                "§9.4.4: text drawn at a zero size or horizontal scale meets the region; the \
                 page is refused rather than adjust an advance that cannot be expressed"
                    .to_owned(),
            )
        } else if self.char_spacing != 0.0 || self.word_spacing != 0.0 {
            Some(
                "§9.4.4: character or word spacing is in force where a glyph is removed; the \
                 page is refused rather than restate the spacing beside a TJ gap"
                    .to_owned(),
            )
        } else if !(scale.is_finite() && scale > 0.0) {
            Some(
                "§9.4.4: the text-rendering matrix is degenerate where a glyph is removed; the \
                 page is refused"
                    .to_owned(),
            )
        } else {
            None
        };

        if let Some(reason) = refusal {
            // Still count the codes so the interpreter's total is met, and refuse only where the
            // region is actually touched — a page whose problematic text is nowhere near a
            // redaction is redacted fine.
            return if self.count_only(elements, width)? {
                Err(reason)
            } else {
                Ok(None)
            };
        }

        let mut out = String::from("[");
        let mut any = false;
        for element in elements {
            match element {
                ArrayElement::Number(value) => write_number(&mut out, *value),
                ArrayElement::Str(bytes) => {
                    any |= self.rewrite_string(bytes, width, &mut out)?;
                }
            }
        }
        out.push(']');
        Ok(any.then_some(out))
    }

    /// Splits one string into runs of kept and removed codes, emitting kept runs as literals and
    /// removed runs as a single `TJ` adjustment; returns whether anything was removed.
    fn rewrite_string(
        &mut self,
        bytes: &[u8],
        width: CodeWidth,
        out: &mut String,
    ) -> Result<bool, String> {
        let codes = split_codes(bytes, width)?;
        let mut kept: Vec<u8> = Vec::new();
        let mut removed_first: Option<[f32; 8]> = None;
        let mut removed_last: [f32; 8] = [0.0; 8];
        let mut any = false;
        for (start, len) in codes {
            let quad = self.quad_at(self.code_index)?;
            let meets = self
                .regions
                .iter()
                .any(|region| overlaps(*region, bbox_of_points(&quad)));
            self.code_index = self.code_index.saturating_add(1);
            if meets {
                if removed_first.is_none() {
                    if !kept.is_empty() {
                        write_pdf_string(out, &kept);
                        kept.clear();
                    }
                    removed_first = Some(quad);
                }
                removed_last = quad;
                self.removed = self.removed.saturating_add(1);
                any = true;
            } else {
                if let Some(first) = removed_first.take() {
                    self.write_gap(out, first, removed_last);
                }
                kept.extend_from_slice(&bytes[start..start.saturating_add(len)]);
            }
        }
        if let Some(first) = removed_first.take() {
            self.write_gap(out, first, removed_last);
        }
        if !kept.is_empty() {
            write_pdf_string(out, &kept);
        }
        Ok(any)
    }

    /// The `TJ` adjustment restoring a removed run's advance: §9.4.4's `w0` read back from the
    /// placed quadrilaterals rather than the font. The run's advance vector in the display
    /// list's space is the last removed glyph's far corner minus the first's near corner; that,
    /// projected onto the text-rendering x-axis and turned into thousandths of text space, is
    /// the number `TJ` subtracts to move the pen the same distance.
    fn write_gap(&self, out: &mut String, first: [f32; 8], last: [f32; 8]) {
        let axis = self.text_linear.then(self.ctm);
        let length_squared = axis.a * axis.a + axis.b * axis.b;
        if !(length_squared.is_finite() && length_squared > 0.0) {
            return;
        }
        let advance = (last[2] - first[0], last[3] - first[1]);
        let text_advance = (advance.0 * axis.a + advance.1 * axis.b) / length_squared;
        let denominator = self.font_size * self.horizontal;
        if denominator == 0.0 {
            return;
        }
        write_number(out, f64::from(-1000.0 * text_advance / denominator));
    }

    /// Counts a show operator's codes and whether any meets the region, without editing.
    fn count_only(&mut self, elements: &[ArrayElement], width: CodeWidth) -> Result<bool, String> {
        let mut met = false;
        for element in elements {
            if let ArrayElement::Str(bytes) = element {
                for _ in split_codes(bytes, width)? {
                    met |= self.take_code()?;
                }
            }
        }
        Ok(met)
    }

    /// Tests every code of a string against the region without editing (the refused operators).
    fn string_meets_region(&mut self, bytes: &[u8]) -> Result<bool, String> {
        let width = self.require_width()?;
        let mut met = false;
        for _ in split_codes(bytes, width)? {
            met |= self.take_code()?;
        }
        Ok(met)
    }

    /// Consumes one code, advancing the index, and answers whether its quad meets the region.
    fn take_code(&mut self) -> Result<bool, String> {
        let quad = self.quad_at(self.code_index)?;
        self.code_index = self.code_index.saturating_add(1);
        Ok(self
            .regions
            .iter()
            .any(|region| overlaps(*region, bbox_of_points(&quad))))
    }

    /// The placed quadrilateral for a code index, or a refusal where the walk has outrun the
    /// interpreter (the code-count check, met early).
    fn quad_at(&self, index: usize) -> Result<[f32; 8], String> {
        self.quads.get(index).copied().ok_or_else(|| {
            "the content walk showed more codes than the interpreter placed; the page is refused"
                .to_owned()
        })
    }

    fn require_width(&self) -> Result<CodeWidth, String> {
        self.width.ok_or_else(|| {
            "the content shows text before any /Tf selects a font; the page is refused".to_owned()
        })
    }

    /// The length of the text-rendering matrix's x-axis in the display list's units, which the
    /// gap adjustment divides a display advance by.
    fn advance_scale(&self) -> f32 {
        let axis = self.text_linear.then(self.ctm);
        (axis.a * axis.a + axis.b * axis.b).sqrt()
    }

    fn extend_path(&mut self, numbers: &[f64]) {
        for pair in numbers.chunks_exact(2) {
            self.add_point(pair[0], pair[1]);
        }
    }

    fn extend_rectangle(&mut self, numbers: &[f64]) {
        if let [x, y, w, h] = numbers {
            self.add_point(*x, *y);
            self.add_point(*x + *w, *y);
            self.add_point(*x + *w, *y + *h);
            self.add_point(*x, *y + *h);
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "a content-stream path coordinate is far inside f32"
    )]
    fn add_point(&mut self, x: f64, y: f64) {
        let point = self.ctm.apply(Point::new(x as f32, y as f32));
        let bbox = self
            .path
            .get_or_insert([point.x, point.y, point.x, point.y]);
        bbox[0] = bbox[0].min(point.x);
        bbox[1] = bbox[1].min(point.y);
        bbox[2] = bbox[2].max(point.x);
        bbox[3] = bbox[3].max(point.y);
    }

    /// A painted path that meets the region is a mark this walk does not remove; refuse.
    fn paint_path(&mut self, keyword: &[u8]) -> Result<(), String> {
        let bbox = self.path.take();
        if let Some(bbox) = bbox
            && self.regions.iter().any(|region| overlaps(*region, bbox))
        {
            return Err(format!(
                "§8.5: a painted path ({}) meets the region; the page is refused rather than \
                 leave a mark the removal does not reach",
                String::from_utf8_lossy(keyword)
            ));
        }
        Ok(())
    }

    /// `Do`: an image or form that meets the region is refused; §12.5.6.23 requires an image's
    /// data destroyed, which this walk does not do.
    fn do_xobject(&mut self, operands: &[(Operand, usize)]) -> Result<(), String> {
        let Some(name) = operands.iter().find_map(|(operand, _)| match operand {
            Operand::Name(bytes) => Some(bytes.clone()),
            _ => None,
        }) else {
            return Ok(());
        };
        let xobjects = self.document.get_key(&self.page.resources, "XObject");
        let object = xobjects
            .as_dict()
            .and_then(|dict| dict.get_by_name(&Name::new(name.as_slice())))
            .map(|entry| self.document.resolve(entry));
        let Some(dict) = object.as_ref().and_then(Object::as_dict) else {
            return Err(format!(
                "the content draws /{} which /Resources /XObject does not define; the page is \
                 refused",
                String::from_utf8_lossy(&name)
            ));
        };
        let subtype = self
            .document
            .get_key(dict, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec())
            .unwrap_or_default();
        let bbox = match subtype.as_slice() {
            b"Form" => match numbers(self.document, dict, "BBox") {
                Some(values) if values.len() >= 4 => self.transformed_box(&values),
                _ => self.unit_square(),
            },
            _ => self.unit_square(),
        };
        if self.regions.iter().any(|region| overlaps(*region, bbox)) {
            return Err(format!(
                "§12.5.6.23: a /{} XObject meets the region, whose content the removal does not \
                 reach; the page is refused",
                String::from_utf8_lossy(&subtype)
            ));
        }
        Ok(())
    }

    /// `gs`: a soft mask in the named graphics state is refused (§11.6.4.3 over the region).
    fn ext_gstate(&self, operands: &[(Operand, usize)]) -> Result<(), String> {
        let Some(name) = operands.iter().find_map(|(operand, _)| match operand {
            Operand::Name(bytes) => Some(bytes.clone()),
            _ => None,
        }) else {
            return Ok(());
        };
        let states = self.document.get_key(&self.page.resources, "ExtGState");
        let state = states
            .as_dict()
            .and_then(|dict| dict.get_by_name(&Name::new(name.as_slice())))
            .map(|entry| self.document.resolve(entry));
        let masked = state
            .as_ref()
            .and_then(Object::as_dict)
            .map(|dict| self.document.get_key(dict, "SMask"))
            .is_some_and(|mask| match mask {
                Object::Name(name) => name.as_bytes() != b"None",
                Object::Null => false,
                _ => true,
            });
        if masked {
            return Err(
                "§11.6.4.3: a soft-mask group is in force; the page is refused rather \
                        than redact under a mask the removal does not model"
                    .to_owned(),
            );
        }
        Ok(())
    }

    /// An inline image (§8.9.7): skipped so its data is never lexed, and refused where it meets
    /// the region.
    fn inline_image(&mut self, content: &[u8], lexer: &mut Lexer<'_>) -> Result<(), String> {
        let scan = pdf_model::inline_image::scan(
            self.document,
            content,
            lexer.position(),
            &self.page.resources,
            true,
        );
        lexer.seek(scan.resume);
        let bbox = self.unit_square();
        if self.regions.iter().any(|region| overlaps(*region, bbox)) {
            return Err(
                "§8.9.7: an inline image meets the region, whose data the removal does \
                        not destroy; the page is refused"
                    .to_owned(),
            );
        }
        Ok(())
    }

    /// The unit square mapped through the current transform — an image or form's placement.
    fn unit_square(&self) -> [f32; 4] {
        self.transformed_box(&[0.0, 0.0, 1.0, 1.0])
    }

    fn transformed_box(&self, rect: &[f32]) -> [f32; 4] {
        let corners = [
            self.ctm.apply(Point::new(rect[0], rect[1])),
            self.ctm.apply(Point::new(rect[2], rect[1])),
            self.ctm.apply(Point::new(rect[2], rect[3])),
            self.ctm.apply(Point::new(rect[0], rect[3])),
        ];
        let flat: Vec<f32> = corners
            .iter()
            .flat_map(|point| [point.x, point.y])
            .collect();
        bbox_of_points(&flat)
    }
}

/// The plain numeric operands, in order (the geometry operators take only numbers).
fn plain_numbers(operands: &[(Operand, usize)]) -> Vec<f64> {
    operands
        .iter()
        .filter_map(|(operand, _)| match operand {
            Operand::Number(value) => Some(*value),
            _ => None,
        })
        .collect()
}

/// The first number, or zero.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a text-state scalar is far inside f32"
)]
fn first(numbers: &[f64]) -> f32 {
    numbers.first().copied().unwrap_or(0.0) as f32
}

/// A six-number matrix operand as a transform.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a content-stream matrix component is far inside f32"
)]
fn matrix(numbers: &[f64]) -> Option<Transform> {
    if let [a, b, c, d, e, f] = numbers {
        Some(Transform::new(
            *a as f32, *b as f32, *c as f32, *d as f32, *e as f32, *f as f32,
        ))
    } else {
        None
    }
}

/// The linear part of a transform (its translation dropped).
fn linear(transform: Transform) -> Transform {
    Transform::new(transform.a, transform.b, transform.c, transform.d, 0.0, 0.0)
}

/// One string's byte offsets and lengths per code, or a refusal for a `Two`-byte string of odd
/// length (a half code the walk will not cut).
fn split_codes(bytes: &[u8], width: CodeWidth) -> Result<Vec<(usize, usize)>, String> {
    let step = width.bytes();
    if width == CodeWidth::Two && !bytes.len().is_multiple_of(2) {
        return Err(
            "§9.7.5: a two-byte-encoded string has an odd byte count; the page is refused"
                .to_owned(),
        );
    }
    Ok((0..bytes.len())
        .step_by(step)
        .map(|start| (start, step))
        .collect())
}

/// Reads a `[ … ]` array's elements from the lexer, keeping strings and numbers and skipping
/// anything else (a nested array or dictionary a `TJ` never holds).
fn collect_array(lexer: &mut Lexer<'_>) -> Vec<ArrayElement> {
    let mut elements = Vec::new();
    let mut depth = 0usize;
    while let Some(token) = lexer.next_token() {
        match token {
            Token::ArrayClose if depth == 0 => break,
            Token::ArrayClose => depth = depth.saturating_sub(1),
            Token::ArrayOpen => depth = depth.saturating_add(1),
            Token::DictOpen => skip_dictionary(lexer),
            Token::Integer(value) if depth == 0 => {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a TJ adjustment is far inside f64"
                )]
                elements.push(ArrayElement::Number(value as f64));
            }
            Token::Real(value) if depth == 0 => elements.push(ArrayElement::Number(value)),
            Token::String(bytes) if depth == 0 => elements.push(ArrayElement::Str(bytes)),
            _ => {}
        }
    }
    elements
}

/// Consumes a `<< … >>` dictionary, having read its opening, so its bytes never reach the walk.
fn skip_dictionary(lexer: &mut Lexer<'_>) {
    let mut depth = 1usize;
    while depth > 0 {
        match lexer.next_token() {
            Some(Token::DictOpen) => depth = depth.saturating_add(1),
            Some(Token::DictClose) => depth = depth.saturating_sub(1),
            Some(_) => {}
            None => break,
        }
    }
}

/// Applies the byte-range edits to the content, leaving every other byte exactly as it was.
fn apply_edits(content: &[u8], mut edits: Vec<(usize, usize, Vec<u8>)>) -> Vec<u8> {
    edits.sort_by_key(|(start, _, _)| *start);
    let mut out = Vec::with_capacity(content.len());
    let mut cursor = 0usize;
    for (start, end, replacement) in edits {
        if start < cursor || end > content.len() || start > end {
            // Overlapping or out-of-range edits are not produced by the walk; skipping one
            // rather than corrupting the stream keeps a defect visible as a code-count mismatch.
            continue;
        }
        out.extend_from_slice(&content[cursor..start]);
        out.extend_from_slice(&replacement);
        cursor = end;
    }
    out.extend_from_slice(&content[cursor..]);
    out
}

/// Writes a number into a `TJ` array with a minimal, re-lexable representation.
fn write_number(out: &mut String, value: f64) {
    if !value.is_finite() {
        out.push_str(" 0");
        return;
    }
    if (value - value.round()).abs() < 1e-6 {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a TJ adjustment rounded to an integer is far inside i64"
        )]
        let rounded = value.round() as i64;
        out.push(' ');
        let _ = write!(out, "{rounded}");
    } else {
        let text = format!(" {value:.3}");
        out.push_str(text.trim_end_matches('0').trim_end_matches('.'));
    }
}

/// Emits a byte string as a §7.3.4.2 literal string, escaping what a lexer would misread.
fn write_pdf_string(out: &mut String, bytes: &[u8]) {
    out.push('(');
    for &byte in bytes {
        match byte {
            b'\\' => out.push_str("\\\\"),
            b'(' => out.push_str("\\("),
            b')' => out.push_str("\\)"),
            0x20..=0x7e => out.push(byte as char),
            _ => {
                let _ = write!(out, "\\{byte:03o}");
            }
        }
    }
    out.push(')');
}

/// What [`write_document`] produced.
struct Written {
    name: String,
    bytes: u64,
    sanitised: bool,
    dangling: bool,
}

/// Assembles the redacted document and serialises it.
///
/// Every non-redacted page and its content stream crosses byte for byte (a copied slot); a
/// redacted page is *replaced* — its `/Contents` a new stream carrying the edited bytes, its
/// `/Redact` annotations removed (§12.5.6.23: "the redact annotations are removed"), and every
/// other entry carried into the output's numbering. The original content stream and the removed
/// annotations are then referenced by nothing and copied by nothing, which is what makes the
/// removal a removal rather than an orphan the file still holds.
fn write_document(
    document: &Document,
    root: ObjectId,
    applied: &mut [AppliedPage],
    plan: &RedactPlan,
    sinks: &dyn Sinks,
) -> Result<Written, Refusal> {
    let mut assembly = Assembly::new(vec![document]);
    for page in applied.iter_mut() {
        page.placed = assembly
            .replace(0, page.page_id)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
    }
    // The reachable closure, pruned: a replaced page short-circuits the walk, so its old content
    // and its removed annotations are reached from nowhere else and never copied.
    let mapped_root = copy_closure(&mut assembly, document, root, true)
        .map_err(|error| Refusal::Assembly(error.to_string()))?;
    assembly.set_root(mapped_root);
    if let Some(info) = document
        .trailer()
        .get("Info")
        .and_then(Object::as_reference)
    {
        let carried = copy_closure(&mut assembly, document, info, true)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
        assembly.set_info(Some(carried));
    }

    for page in applied.iter() {
        let object = build_page(&mut assembly, document, page)?;
        assembly
            .place(page.placed, object)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
    }

    let version = document
        .version()
        .unwrap_or(pdf_syntax::Version { major: 1, minor: 7 });
    let expanded = plan.names.expand(&Fill {
        ordinal: 1,
        count: 1,
        page: None,
        label: None,
        title: None,
    });
    let mut writer = sinks.open(&expanded.name).map_err(|error| Refusal::Sink {
        name: expanded.name.clone(),
        error,
    })?;
    let written = serialize(
        &assembly,
        version,
        Options::new(Form::of(document)),
        &mut writer,
    )
    .map_err(|error| Refusal::Assembly(format!("{}: {error}", expanded.name)))?;
    writer.flush().map_err(|error| Refusal::Sink {
        name: expanded.name.clone(),
        error,
    })?;
    drop(writer);
    Ok(Written {
        name: expanded.name,
        bytes: written.bytes,
        sanitised: expanded.sanitised,
        dangling: written.dangling > 0,
    })
}

/// Builds a replaced page dictionary: the edited content, the surviving annotations, and every
/// other entry with its references carried into the output's numbering.
fn build_page(
    assembly: &mut Assembly<'_>,
    document: &Document,
    page: &AppliedPage,
) -> Result<Object, Refusal> {
    let mut length = Dictionary::new();
    length.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(page.content.len()).unwrap_or(i64::MAX)),
    );
    let content = Object::Stream(Arc::new(Stream {
        dict: length,
        data: Arc::from(page.content.as_slice()),
        decryption_failed: false,
    }));
    let content_id = assembly
        .add(content)
        .map_err(|error| Refusal::Assembly(error.to_string()))?;

    let original = document.get(page.page_id);
    let Some(source) = original.as_dict() else {
        return Err(Refusal::Assembly(format!(
            "page object {} is not a dictionary",
            page.page_id.number
        )));
    };

    let surviving = surviving_annotations(assembly, document, source);
    let mut dict = Dictionary::new();
    for (key, value) in source.iter() {
        if key.as_bytes() == b"Contents" || key.as_bytes() == b"Annots" {
            continue;
        }
        dict.insert(key.clone(), carry(assembly, document, value, 0));
    }
    dict.insert(Name::new(&b"Contents"[..]), Object::Reference(content_id));
    if let Some(annots) = surviving {
        dict.insert(Name::new(&b"Annots"[..]), annots);
    }
    Ok(Object::Dictionary(dict))
}

/// The page's annotations with every `/Redact` removed, carried into the output — or `None`
/// where none survive, so the entry is dropped rather than left empty.
fn surviving_annotations(
    assembly: &mut Assembly<'_>,
    document: &Document,
    page: &Dictionary,
) -> Option<Object> {
    let items = document.get_key(page, "Annots").as_array()?.to_vec();
    let mut kept = Vec::new();
    for item in &items {
        let is_redact = document
            .resolve(item)
            .as_dict()
            .and_then(|dict| document.get_key(dict, "Subtype").as_name().cloned())
            .is_some_and(|name| name.as_bytes() == b"Redact");
        if !is_redact {
            kept.push(carry(assembly, document, item, 0));
        }
    }
    (!kept.is_empty()).then_some(Object::Array(kept))
}

/// One value with every reference copied into the assembly and restated in the output's
/// numbering — the built page's entries, whose references must already be the output's.
fn carry(assembly: &mut Assembly<'_>, document: &Document, value: &Object, depth: usize) -> Object {
    if depth >= MAX_DEPTH {
        return Object::Null;
    }
    match value {
        Object::Reference(id) => match copy_closure(assembly, document, *id, true) {
            Ok(mapped) => Object::Reference(mapped),
            Err(_) => Object::Null,
        },
        Object::Array(items) => Object::Array(
            items
                .iter()
                .map(|item| carry(assembly, document, item, depth.saturating_add(1)))
                .collect(),
        ),
        Object::Dictionary(dict) => {
            let mut out = Dictionary::new();
            for (key, entry) in dict.iter() {
                out.insert(
                    key.clone(),
                    carry(assembly, document, entry, depth.saturating_add(1)),
                );
            }
            Object::Dictionary(out)
        }
        other => other.clone(),
    }
}
