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
//! # Destroying image data (§12.5.6.23)
//!
//! §12.5.6.23: "If a portion of an image is contained in a redaction region, that portion of the
//! image data shall be destroyed; clipping or image masks shall not be used to hide that data."
//! An image `XObject` whose placement meets the region has its **samples overwritten in the
//! source grid**: each sample whose centre, mapped through the placing transform into the display
//! list's space, lies in a region box has all of its component bits set to **zero** — the image's
//! own zero in the integer sample domain of §8.9.5.2, which maps through any `/Decode` array to
//! that array's `Dmin` and so carries none of the original sample. The image is then re-encoded
//! with `FlateDecode` and written as a new stream, so the cleared samples are gone from the
//! decoded image rather than covered. Samples outside every region are byte-identical to the
//! original decode. Because clearing an image object modifies bytes every placement of it shares,
//! a shared image is refused (below) rather than cleared where it would alter another placement.
//!
//! An image behind a lossy or bilevel **codec** — `DCTDecode` (§7.4.8) or `CCITTFaxDecode`
//! (§7.4.6) — cannot be zeroed in its packed grid, because its bytes are the codec's input rather
//! than samples. It is instead **decoded to samples the interpreter's own way**
//! ([`pdf_model::image::decode`], so the codec runs under the process's isolation — confined in
//! the program, principle 3), the region cleared in the decoded raster, and the result written as
//! an 8-bit `DeviceRGB` `FlateDecode` image: a lossless, non-codec filter, so the redacted region
//! is exactly zero and cannot round-trip back through the lossy codec that would leak it. The
//! replacement is opaque and carries no `/Decode`, `/Mask` or `/SMask`, so a codec image that
//! decodes with transparency (a soft mask, a colour key, or an image mask) is refused rather than
//! flattened — the re-encode would not preserve it. `JPXDecode` and `JBIG2Decode` stay refused
//! (below).
//!
//! An **inline image** (§8.9.7's `BI`\u{2026}`ID`\u{2026}`EI`) has its samples in the content
//! stream itself rather than a referenced object, so destroying them is a **content-stream
//! splice**: the whole run is replaced with a freshly built inline image whose region samples are
//! the same zero constant, re-encoded `FlateDecode`, and every byte of `/Contents` outside the
//! run is left exactly as it was. The replacement carries every §8.9.7 key the source stated
//! (grid, colour space, `/Decode`, `/ImageMask`\u{2026}) with only the encoding replaced; a value
//! an inline image cannot hold — a colour space resolved to a resource object carrying a §7.3.8
//! reference or stream — refuses the page rather than write indirection into a content stream.
//!
//! # What is refused, never cut wrong (trap 5)
//!
//! A page is refused by name — its content and its `/Redact` annotations left as the file
//! wrote them — where removal cannot be proven to leave no trace: a Type 3 font (§9.6.5's glyph
//! procedures draw outside the advance box this walk measures), a composite font not encoded
//! `Identity-H` (§9.7.5's codespace decides the code-byte width), the `sh` operator (§8.7.4.2
//! paints the whole clip), a soft-mask group, or any code count the interpreter does not confirm.
//! An encrypted document is refused outright, because this writer emits no `/Encrypt`.
//!
//! Three marks meeting the region stay refused with their own narrower reason, each an owed
//! capability rather than a silence:
//!
//! - an **image encoded by `JPXDecode` or `JBIG2Decode`** (§8.9.5) — `JPXDecode` because a
//!   codestream over the decoder's budget comes back at a reduced resolution level (§7.4.9 NOTE 3),
//!   so the raster is not the image's grid and a redaction that silently changed the image's
//!   resolution cannot be proven to have replaced the full-resolution content the region maps
//!   into; `JBIG2Decode` because its clear is an owed capability this round did not build and prove
//!   — the `DCTDecode`/`CCITTFaxDecode` route above is the codec case that is built. An image whose
//!   colour space this build cannot count the components of, or whose declared grid its sample data
//!   does not fill, or a **shared** image the guard cannot prove exclusive to the redacted page
//!   (clearing it would alter another placement), is refused the same way;
//! - an **inline image** (§8.9.7) encoded by a codec, or whose colour space resolves to a
//!   resource object an inline image cannot carry, or whose declared grid its data does not fill
//!   — the same three limits as an image `XObject`, at the splice above rather than at a `Do`;
//! - a **painted path** or a **form** (§8.5) — removing only the portion of a vector mark within
//!   the region needs geometric path subtraction, and deleting the whole painting operator would
//!   destroy content the annotation did not identify (its bbox reaches outside the region), which
//!   is the opposite failure from the one the clause forbids.
//!
//! # What is a documented departure (A64/A65's fence)
//!
//! Table 195's `/OverlayText`, `/IC` and `/RO` describe the mark drawn *over* the region after
//! removal, and composing them is composing content the document did not hold — the far side of
//! `CLAUDE.md`'s authoring line. So the removal happens and the overlay is not drawn, reported
//! as a per-page [`crate::Departure`] from §12.5.6.23's full application semantics.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::sync::Arc;

use pdf_model::colour::{ColourSpace, Conversion};
use pdf_model::content::{Interpretation, base_transform, interpret};
use pdf_model::image::Flattened;
use pdf_model::{Page, Pages};
use pdf_render::geom::Point;
use pdf_render::{Color, Transform};
use pdf_syntax::Document;
use pdf_syntax::lexer::{Lexer, Token};
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::{Assembly, Form, Options, flate_encode, serialize};

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

    // The single-referrer guard on image clearing (see `exclusively_owned`) reads how many
    // times each object is referenced across the whole document. It is counted once here rather
    // than per placement; `redact` is an offline verb, so the one walk of every in-use object is
    // affordable where it would not be on the launch path.
    let counts = reference_counts(document);

    let pages = Pages::new(document);
    let mut applied: Vec<AppliedPage> = Vec::new();
    let mut clears: Vec<ImageClear> = Vec::new();
    let mut annotations = 0usize;
    let mut glyphs = 0usize;
    let mut inline_images = 0usize;
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
        match plan_page(document, &page, &regions, &counts) {
            Ok(edit) => {
                annotations = annotations.saturating_add(regions.len());
                glyphs = glyphs.saturating_add(edit.removed);
                inline_images = inline_images.saturating_add(edit.inline_images);
                clears.extend(edit.clears);
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

    // §12.5.6.23: destroy the image data. The single-referrer guard has already refused any
    // shared image, so each cleared object belongs to one page.
    let cleared_images = clear_images(document, clears)?;
    // §12.5.6.23's destroyed images are the image XObjects cleared as new streams and the inline
    // images spliced in the content stream — both had their region samples set to the constant.
    let images = cleared_images.len().saturating_add(inline_images);

    let written = write_document(document, root, &mut applied, &cleared_images, plan, sinks)?;
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
            images,
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

/// One image `XObject` whose samples the removal destroyed: its object, the re-encoded
/// `FlateDecode` bytes, and how the writer must describe them.
struct ClearedImage {
    /// The object whose stream is replaced.
    id: ObjectId,
    /// The cleared samples, re-encoded `FlateDecode`.
    encoded: Vec<u8>,
    /// `Some((width, height))` when the image was decoded from a codec and re-expressed as an
    /// 8-bit `DeviceRGB` raster, so [`build_cleared_image`] must write a fresh dictionary rather
    /// than carry the source's codec `/Filter`, colour space and masks. `None` when the codec-free
    /// samples keep the source dictionary's grid.
    rgb: Option<(usize, usize)>,
}

/// Groups the pages' image clears by object and destroys each image's samples once.
///
/// A page may draw one image twice, so the placements of one object are unioned into a single
/// cleared stream (§12.5.6.23). The single-referrer guard ([`Walk::exclusively_owned`]) has
/// already refused any image a second page shares, so this cannot destroy another page's picture.
fn clear_images(
    document: &Document,
    clears: Vec<ImageClear>,
) -> Result<Vec<ClearedImage>, Refusal> {
    let mut grouped: HashMap<ObjectId, Vec<ImageClear>> = HashMap::new();
    for clear in clears {
        grouped.entry(clear.image_id).or_default().push(clear);
    }
    let mut cleared: Vec<ClearedImage> = Vec::with_capacity(grouped.len());
    for (id, placements) in grouped {
        let object = document.get(id);
        let stream = object.as_stream().ok_or_else(|| {
            Refusal::Assembly(format!(
                "§8.9.5: image object {} is not a stream where its samples must be destroyed",
                id.number
            ))
        })?;
        let (encoded, rgb) =
            cleared_image_samples(document, stream, &placements).map_err(|detail| {
                Refusal::Assembly(format!("§12.5.6.23: image object {}: {detail}", id.number))
            })?;
        cleared.push(ClearedImage { id, encoded, rgb });
    }
    Ok(cleared)
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

/// A page's content stream with the region's content removed, how many glyphs went, and which
/// image `XObject`s the page's placements ask the removal to clear (§12.5.6.23).
struct PageEdit {
    content: Vec<u8>,
    removed: usize,
    clears: Vec<ImageClear>,
    /// How many inline images (§8.9.7) this page spliced — each had its region samples
    /// destroyed in the content stream, so each counts among the report's destroyed images.
    inline_images: usize,
}

/// One image `XObject` placement the removal clears: which object, the transform that placed its
/// unit square into the display list's space, and the page's region boxes in that same space.
#[derive(Clone)]
struct ImageClear {
    /// The image `XObject` to overwrite.
    image_id: ObjectId,
    /// The placement: the unit square [0,1]² of image space maps through this into the display
    /// list's coordinates, where the region boxes are.
    ctm: Transform,
    /// The display-space region boxes a sample centre is tested against.
    regions: Vec<[f32; 4]>,
    /// The sample grid and packing, read once at planning time so the clearing needs no
    /// resources: width, height, colour components, and bits per component (§8.9.5).
    layout: ImageLayout,
    /// The decoded samples when the source image was behind a `DCTDecode` or `CCITTFaxDecode`
    /// codec: opaque 8-bit `DeviceRGB` from [`pdf_model::image::decode`], laid out as `layout`
    /// describes (3 components, 8 bits). `None` for a codec-free image, whose packed samples are
    /// read from the stream at clearing time. Held here because a codec's bytes are not samples,
    /// so the destruction cannot address the packed grid the source stream carries.
    decoded: Option<Arc<[u8]>>,
}

/// An image's sample layout: enough of §8.9.5 to address one packed sample.
#[derive(Clone, Copy)]
struct ImageLayout {
    width: usize,
    height: usize,
    components: usize,
    bits: usize,
}

/// Plans one page's removal, or refuses it by name (trap 5: never cut it wrong).
fn plan_page(
    document: &Document,
    page: &Page,
    regions: &[Redaction],
    counts: &HashMap<u32, usize>,
) -> Result<PageEdit, String> {
    let draw = crate::render::page_to_draw(page, None, false);
    let interpretation = interpret(document, &draw);
    let base = base_transform(page);
    let region_boxes: Vec<[f32; 4]> = regions
        .iter()
        .flat_map(|region| region.boxes.iter())
        .map(|user| map_box(base, *user))
        .collect();

    let content = page.content(document);
    let walk = Walk::new(document, page, &interpretation, region_boxes, counts);
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
    /// How many times each object number is referenced across the document — the single-referrer
    /// guard on clearing a shared image (`exclusively_owned`).
    counts: &'a HashMap<u32, usize>,
    /// The image `XObject` placements this page asks the removal to clear.
    clears: Vec<ImageClear>,
    /// How many inline images this page has spliced (§8.9.7), for the destroyed-image count.
    inline_cleared: usize,
}

impl<'a> Walk<'a> {
    fn new(
        document: &'a Document,
        page: &'a Page,
        interpretation: &Interpretation,
        regions: Vec<[f32; 4]>,
        counts: &'a HashMap<u32, usize>,
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
            counts,
            clears: Vec::new(),
            inline_cleared: 0,
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
                        self.inline_image(content, &mut lexer, start)?;
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
            clears: self.clears,
            inline_images: self.inline_cleared,
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

    /// A painted path that meets the region is refused: removing only the portion of a vector
    /// mark within the region needs geometric path subtraction this build does not do, and
    /// dropping the whole painting operator would destroy content outside the region (the path's
    /// bbox reaches past it) — the opposite failure from the one §12.5.6.23 forbids.
    fn paint_path(&mut self, keyword: &[u8]) -> Result<(), String> {
        let bbox = self.path.take();
        if let Some(bbox) = bbox
            && self.regions.iter().any(|region| overlaps(*region, bbox))
        {
            return Err(format!(
                "§8.5: a painted path ({}) meets the region; removing only its portion within the \
                 region needs geometric subtraction this build does not do; the page is refused",
                String::from_utf8_lossy(keyword)
            ));
        }
        Ok(())
    }

    /// `Do`: an image that meets the region is cleared (§12.5.6.23); a form or an image the
    /// removal cannot clear is refused by its own narrower reason.
    fn do_xobject(&mut self, operands: &[(Operand, usize)]) -> Result<(), String> {
        let Some(name) = operands.iter().find_map(|(operand, _)| match operand {
            Operand::Name(bytes) => Some(bytes.clone()),
            _ => None,
        }) else {
            return Ok(());
        };
        let xobjects = self.document.get_key(&self.page.resources, "XObject");
        let Some(entry) = xobjects
            .as_dict()
            .and_then(|dict| dict.get_by_name(&Name::new(name.as_slice())))
            .cloned()
        else {
            return Err(format!(
                "the content draws /{} which /Resources /XObject does not define; the page is \
                 refused",
                String::from_utf8_lossy(&name)
            ));
        };
        let object = self.document.resolve(&entry);
        let Some(dict) = object.as_dict() else {
            return Err(format!(
                "the content draws /{}, which does not resolve to a stream; the page is refused",
                String::from_utf8_lossy(&name)
            ));
        };
        let subtype = self
            .document
            .get_key(dict, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec())
            .unwrap_or_default();
        match subtype.as_slice() {
            b"Image" => {
                if !self
                    .regions
                    .iter()
                    .any(|region| overlaps(*region, self.unit_square()))
                {
                    // The image is nowhere near a redaction: it crosses the output untouched.
                    return Ok(());
                }
                let clear = self.plan_image_clear(&name, &entry, &object)?;
                self.clears.push(clear);
                Ok(())
            }
            b"Form" => {
                let bbox = match numbers(self.document, dict, "BBox") {
                    Some(values) if values.len() >= 4 => self.transformed_box(&values),
                    _ => self.unit_square(),
                };
                if self.regions.iter().any(|region| overlaps(*region, bbox)) {
                    return Err(
                        "§8.5: a form XObject meets the region; removing only the portion of its \
                         marks within the region needs geometric subtraction this build does not \
                         do, and dropping the whole form would destroy content outside the \
                         region; the page is refused"
                            .to_owned(),
                    );
                }
                Ok(())
            }
            _ => {
                if self
                    .regions
                    .iter()
                    .any(|region| overlaps(*region, self.unit_square()))
                {
                    return Err(format!(
                        "§12.5.6.23: a /{} XObject meets the region, whose content the removal \
                         does not reach; the page is refused",
                        String::from_utf8_lossy(&subtype)
                    ));
                }
                Ok(())
            }
        }
    }

    /// Plans clearing an image `XObject` that meets the region (§12.5.6.23), or refuses by name.
    ///
    /// The clearing itself is deferred to [`cleared_image_samples`], because a page may draw one
    /// image twice and the destroyed samples are the union across its placements. What this does
    /// is prove the image *can* be cleared without trace — its layout is known, it is not shared,
    /// and if it is behind a codec that codec is one this build decodes and re-encodes losslessly
    /// — so the refusal is decided here, at the page.
    fn plan_image_clear(
        &self,
        name: &[u8],
        entry: &Object,
        object: &Object,
    ) -> Result<ImageClear, String> {
        let shown = String::from_utf8_lossy(name);
        let Some(image_id) = entry.as_reference() else {
            return Err(format!(
                "the image /{shown} is a direct object rather than an indirect reference this \
                 removal can replace; the page is refused"
            ));
        };
        if !self.exclusively_owned(image_id) {
            return Err(format!(
                "§12.5.6.23: the image /{shown} is shared, so overwriting its samples would \
                 destroy content in another placement; the page is refused rather than reach past \
                 the region"
            ));
        }
        let Some(stream) = object.as_stream() else {
            return Err(format!(
                "the image /{shown} is not a stream; the page is refused"
            ));
        };
        let image = self.document.image_stream(stream).ok_or_else(|| {
            format!("the image /{shown} did not decode to samples; the page is refused")
        })?;
        if let Some(codec) = &image.codec {
            return self.plan_codec_clear(&shown, image_id, stream, codec);
        }
        let layout = self.image_layout(&stream.dict, &shown)?;
        let stride = row_stride(layout)
            .ok_or_else(|| format!("the image /{shown}'s grid overflows; the page is refused"))?;
        let expected = stride
            .checked_mul(layout.height)
            .ok_or_else(|| format!("the image /{shown}'s grid overflows; the page is refused"))?;
        if image.data.len() < expected {
            return Err(format!(
                "§8.9.5: the image /{shown}'s sample data is shorter than its {}×{} grid; the \
                 page is refused rather than clear samples that are not there",
                layout.width, layout.height
            ));
        }
        Ok(ImageClear {
            image_id,
            ctm: self.ctm,
            regions: self.regions.clone(),
            layout,
            decoded: None,
        })
    }

    /// Plans clearing a `DCTDecode` or `CCITTFaxDecode` image by decoding it to samples, or
    /// refuses by name (§8.9.5, §7.4.8, §7.4.6).
    ///
    /// A codec's stream bytes are its input, not samples, so the packed-grid destruction the
    /// codec-free path takes cannot address them. The image is decoded the interpreter's own way
    /// ([`pdf_model::image::decode`], so the codec runs under the process's isolation — confined
    /// in the program, principle 3) to straight-alpha `RGBA8`, and the opaque colour components
    /// are carried as the samples to clear. The result is written as an 8-bit `DeviceRGB`
    /// `FlateDecode` image ([`build_cleared_image`]): a lossless, non-codec filter, so the cleared
    /// region is exactly zero and cannot round-trip back through a lossy codec that would leak it.
    ///
    /// Refused, each an owed capability rather than a silence (trap 5, principle 1): a codec other
    /// than the two built here (`JPXDecode`, whose over-budget decode is a reduced resolution
    /// level §7.4.9 NOTE 3, so the raster is not the image's grid; `JBIG2Decode`, not built this
    /// round); an image that does not decode; one that decodes short of its grid, so the re-encode
    /// would drop rows; or one that decodes with transparency (a soft mask, a colour key, or an
    /// image mask), which the opaque `DeviceRGB` re-encode cannot preserve.
    fn plan_codec_clear(
        &self,
        shown: &str,
        image_id: ObjectId,
        stream: &Stream,
        codec: &[u8],
    ) -> Result<ImageClear, String> {
        match codec {
            b"DCTDecode" | b"DCT" | b"CCITTFaxDecode" | b"CCF" => {}
            other => {
                return Err(format!(
                    "§8.9.5: the image /{shown} is encoded with the {} codec, whose samples this \
                     removal does not re-encode; the page is refused",
                    String::from_utf8_lossy(other)
                ));
            }
        }
        let Flattened { image, shortfall } = pdf_model::image::decode(
            self.document,
            stream,
            &self.page.resources,
            Color::BLACK,
            &Conversion::device(),
        )
        .map_err(|error| {
            format!(
                "§8.9.5: the codec image /{shown} did not decode to samples ({error}); the page \
                 is refused"
            )
        })?;
        if let Some(said) = shortfall {
            return Err(format!(
                "§8.9.5: the codec image /{shown} decoded short of its grid ({said}); the page is \
                 refused rather than redact a partial decode"
            ));
        }
        let width = usize::try_from(image.width).map_err(|_| {
            format!("the codec image /{shown}'s grid overflows; the page is refused")
        })?;
        let height = usize::try_from(image.height).map_err(|_| {
            format!("the codec image /{shown}'s grid overflows; the page is refused")
        })?;
        // §11.6.4.2 gives the alpha channel [`pdf_model::image::decode`] leaves; only a fully
        // opaque decode is representable as an opaque `DeviceRGB` raster. A soft mask, a colour
        // key or an image mask leaves some sample non-opaque, and dropping it would change the
        // picture — refuse rather than flatten (principle 1).
        let mut samples = Vec::with_capacity(width.saturating_mul(height).saturating_mul(3));
        for pixel in image.data.chunks_exact(4) {
            if pixel[3] != 0xFF {
                return Err(format!(
                    "§8.9.5: the codec image /{shown} carries transparency the FlateDecode \
                     re-encode cannot preserve; the page is refused rather than flatten it"
                ));
            }
            samples.extend_from_slice(&pixel[..3]);
        }
        let layout = ImageLayout {
            width,
            height,
            components: 3,
            bits: 8,
        };
        Ok(ImageClear {
            image_id,
            ctm: self.ctm,
            regions: self.regions.clone(),
            layout,
            decoded: Some(Arc::from(samples.as_slice())),
        })
    }

    /// The image's sample layout (§8.9.5): its grid, colour components and bit depth.
    fn image_layout(&self, dict: &Dictionary, shown: &str) -> Result<ImageLayout, String> {
        let width = positive_dim(self.document, dict, "Width", shown)?;
        let height = positive_dim(self.document, dict, "Height", shown)?;
        let is_mask = matches!(
            self.document.get_key(dict, "ImageMask"),
            Object::Boolean(true)
        );
        let (components, bits) = if is_mask {
            // §8.9.6.2: an image mask is one bit per sample and carries no colour space of its
            // own; §8.9.5.1 Table 87 requires /BitsPerComponent 1 where it is stated at all.
            (1, 1)
        } else {
            let space = self.document.get_key(dict, "ColorSpace");
            let components = ColourSpace::parse(self.document, &space, &self.page.resources)
                .map(|resolved| resolved.components())
                .filter(|components| *components > 0)
                .ok_or_else(|| {
                    format!(
                        "the image /{shown}'s colour space is one this removal cannot count the \
                         components of; the page is refused"
                    )
                })?;
            let bits = match self.document.get_key(dict, "BitsPerComponent").as_integer() {
                Some(bits @ (1 | 2 | 4 | 8 | 16)) => usize::try_from(bits).unwrap_or(8),
                _ => {
                    return Err(format!(
                        "§8.9.5: the image /{shown}'s /BitsPerComponent is not 1, 2, 4, 8 or 16; \
                         the page is refused"
                    ));
                }
            };
            (components, bits)
        };
        Ok(ImageLayout {
            width,
            height,
            components,
            bits,
        })
    }

    /// Whether an image object is safe to overwrite in place: referenced exactly once in the
    /// document, and reached by a resource path this page does not share, so its samples are the
    /// redacted page's alone. A shared image is refused rather than cleared (trap 5).
    fn exclusively_owned(&self, image_id: ObjectId) -> bool {
        if self.counts.get(&image_id.number).copied() != Some(1) {
            return false;
        }
        // The page's own /Resources must be private: a direct dictionary on the page dict, or an
        // indirect object referenced once. Inherited resources (no /Resources on the page dict)
        // are an ancestor's, shared by construction.
        match self.page.dict.get("Resources") {
            Some(Object::Dictionary(_)) => self.xobject_private(),
            Some(Object::Reference(id)) => {
                self.counts.get(&id.number).copied() == Some(1) && self.xobject_private()
            }
            _ => false,
        }
    }

    /// Whether the page's `/XObject` resource subdictionary is not itself a shared object.
    fn xobject_private(&self) -> bool {
        let resources = self.document.get_key(&self.page.dict, "Resources");
        match resources.as_dict().and_then(|dict| dict.get("XObject")) {
            Some(Object::Reference(id)) => self.counts.get(&id.number).copied() == Some(1),
            _ => true,
        }
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

    /// An inline image (§8.9.7): its `BI`\u{2026}`ID`\u{2026}`EI` run is spliced when it meets the
    /// region, and skipped so its data is never lexed otherwise.
    ///
    /// Unlike an image `XObject`, an inline image lives in the content stream itself, so
    /// destroying the samples §12.5.6.23 asks for is a byte-range edit of `/Contents` rather than
    /// the replacement of a referenced object — the whole run is replaced with a freshly built
    /// inline image whose region samples are zero, and every byte outside `[bi_start, resume)` is
    /// left exactly as it was.
    fn inline_image(
        &mut self,
        content: &[u8],
        lexer: &mut Lexer<'_>,
        bi_start: usize,
    ) -> Result<(), String> {
        let scan = pdf_model::inline_image::scan(
            self.document,
            content,
            lexer.position(),
            &self.page.resources,
            true,
        );
        lexer.seek(scan.resume);
        let bbox = self.unit_square();
        if !self.regions.iter().any(|region| overlaps(*region, bbox)) {
            // The image is nowhere near a redaction: its run crosses the output byte for byte.
            return Ok(());
        }
        // The run meets the region, so its samples must be destroyed. A run that could not be read
        // is refused rather than left in place under a redaction (principle 1).
        let stream = scan.image.map_err(|error| {
            format!(
                "§8.9.7: an inline image meets the region but could not be read ({error}); the \
                 page is refused rather than leave its samples under a redaction"
            )
        })?;
        let replacement = self.spliced_inline_image(&stream)?;
        // `scan.resume` is past `EI` *and* the white space that delimits it (§7.2.3); the edit
        // ends at `EI` itself so that separator crosses the output byte for byte with the rest of
        // the surrounding stream. Trimming the white space back off `resume` finds it.
        let end = end_of_ei(content, bi_start, scan.resume);
        self.edits.push((bi_start, end, replacement));
        self.inline_cleared = self.inline_cleared.saturating_add(1);
        Ok(())
    }

    /// Builds the replacement `BI`\u{2026}`ID`\u{2026}`EI` run for an inline image whose region
    /// samples are destroyed (§12.5.6.23), or refuses the page by name where it cannot be cleared
    /// without trace (trap 5, principle 1).
    ///
    /// The samples come from [`Document::image_stream`] exactly as an image `XObject`'s do, are
    /// zeroed by [`clear_region`] under this placement, re-encoded `FlateDecode`, and written back
    /// under a dictionary carrying every §8.9.7 key the source stated (its `/Width`, `/Height`,
    /// `/BitsPerComponent`, colour space, `/Decode`, `/ImageMask`\u{2026}) with only the old
    /// encoding replaced. Zero is the sample domain's own constant: §8.9.5.2 maps it through any
    /// `/Decode` to that array's `Dmin`, carrying none of the original sample (ADR 1126).
    fn spliced_inline_image(&self, stream: &Stream) -> Result<Vec<u8>, String> {
        let image = self.document.image_stream(stream).ok_or_else(|| {
            "§8.9.7: an inline image meeting the region did not decode to samples; the page is \
             refused"
                .to_owned()
        })?;
        if let Some(codec) = &image.codec {
            return Err(format!(
                "§8.9.5: an inline image meeting the region is encoded with the {} codec, whose \
                 samples this removal does not re-encode; the page is refused",
                String::from_utf8_lossy(codec)
            ));
        }
        let layout = self.image_layout(&stream.dict, "an inline image")?;
        let stride = row_stride(layout).ok_or_else(|| {
            "§8.9.5: an inline image meeting the region has a grid that overflows; the page is \
             refused"
                .to_owned()
        })?;
        let expected = stride.checked_mul(layout.height).ok_or_else(|| {
            "§8.9.5: an inline image meeting the region has a grid that overflows; the page is \
             refused"
                .to_owned()
        })?;
        if image.data.len() < expected {
            return Err(
                "§8.9.5: an inline image meeting the region has less sample data than its declared \
                 grid; the page is refused rather than clear samples that are not there"
                    .to_owned(),
            );
        }
        let mut samples = image.data.to_vec();
        clear_region(&mut samples, stride, layout, self.ctm, &self.regions);
        let encoded = flate_encode(&samples, 6).ok_or_else(|| {
            "§8.9.7: an inline image's cleared samples could not be re-encoded; the page is refused"
                .to_owned()
        })?;
        build_inline_image(&stream.dict, &encoded)
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

/// How many times each object number is referenced across the whole document.
///
/// The single-referrer guard on clearing a shared image ([`Walk::exclusively_owned`]) reads this:
/// an image referenced exactly once, from a resource path the redacted page does not share, is
/// the redacted page's alone, so overwriting its samples cannot destroy another placement's
/// picture. Counted over the trailer and every in-use object; [`Document::get`] keys its cache by
/// object number, so generation zero reaches every object.
fn reference_counts(document: &Document) -> HashMap<u32, usize> {
    let mut counts: HashMap<u32, usize> = HashMap::new();
    for (_key, value) in document.trailer().iter() {
        tally_references(value, &mut counts, 0);
    }
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        tally_references(&object, &mut counts, 0);
    }
    counts
}

/// Adds every indirect reference in `object` to `counts`, following arrays, dictionaries and a
/// stream's dictionary; bounded by [`MAX_DEPTH`] against a pathological nesting.
fn tally_references(object: &Object, counts: &mut HashMap<u32, usize>, depth: usize) {
    if depth >= MAX_DEPTH {
        return;
    }
    let next = depth.saturating_add(1);
    match object {
        Object::Reference(id) => {
            let count = counts.entry(id.number).or_insert(0);
            *count = count.saturating_add(1);
        }
        Object::Array(items) => {
            for item in items {
                tally_references(item, counts, next);
            }
        }
        Object::Dictionary(dict) => {
            for (_key, value) in dict.iter() {
                tally_references(value, counts, next);
            }
        }
        Object::Stream(stream) => {
            for (_key, value) in stream.dict.iter() {
                tally_references(value, counts, next);
            }
        }
        _ => {}
    }
}

/// A positive integer dimension entry (`/Width`, `/Height`), or a refusal naming it.
fn positive_dim(
    document: &Document,
    dict: &Dictionary,
    key: &str,
    shown: &str,
) -> Result<usize, String> {
    match document.get_key(dict, key).as_integer() {
        Some(value) if value > 0 => usize::try_from(value)
            .map_err(|_| format!("the image /{shown}'s /{key} overflows; the page is refused")),
        _ => Err(format!(
            "the image /{shown} has no positive /{key}; the page is refused"
        )),
    }
}

/// The packed length of one image row in bytes: §8.9.5.2's samples run left to right within a
/// row, each row filled to a byte boundary. `None` where the grid overflows `usize`.
fn row_stride(layout: ImageLayout) -> Option<usize> {
    let bits = layout
        .width
        .checked_mul(layout.components)?
        .checked_mul(layout.bits)?;
    Some(bits.div_ceil(8))
}

/// The re-encoded cleared samples, and the `DeviceRGB` grid the writer must redescribe them by
/// where the source was behind a codec (`None` for a codec-free image that keeps its dictionary).
type ClearedSamples = (Vec<u8>, Option<(usize, usize)>);

/// The image's decoded samples with every region's samples zeroed, re-encoded with `FlateDecode`,
/// and the `DeviceRGB` grid to redescribe it by where the source was behind a codec.
///
/// §12.5.6.23: "that portion of the image data shall be destroyed". For a **codec-free** image the
/// samples come from [`Document::image_stream`], which runs every filter before the codec, so they
/// are the packed samples themselves; the codec-free precondition was proved at planning time
/// ([`Walk::plan_image_clear`]) and is re-checked here rather than trusted across the two phases.
/// For a **codec** image ([`Walk::plan_codec_clear`]) the opaque 8-bit `DeviceRGB` samples were
/// decoded then, and travel on the placement — the source stream's bytes are the codec's input,
/// not samples, so they are never read here. Either way every placement's region is cleared and
/// the result re-encoded `FlateDecode`; the returned grid is `Some` exactly for the codec case, so
/// the writer states a fresh `DeviceRGB` dictionary.
fn cleared_image_samples(
    document: &Document,
    stream: &Stream,
    placements: &[ImageClear],
) -> Result<ClearedSamples, String> {
    let first = placements
        .first()
        .ok_or_else(|| "no placement to clear".to_owned())?;
    let layout = first.layout;
    // All placements of one object decoded the same image, so any one's samples are the object's;
    // a codec-free image reads its packed samples from the stream instead. Either way the
    // placements' regions are unioned onto the samples below.
    let mut samples = if let Some(decoded) = &first.decoded {
        decoded.to_vec()
    } else {
        let image = document
            .image_stream(stream)
            .ok_or_else(|| "the image no longer decodes to samples".to_owned())?;
        if image.codec.is_some() {
            return Err(
                "the image is behind a codec whose samples this removal does not re-encode"
                    .to_owned(),
            );
        }
        image.data.to_vec()
    };
    let stride = row_stride(layout).ok_or_else(|| "the image grid overflows".to_owned())?;
    let expected = stride
        .checked_mul(layout.height)
        .ok_or_else(|| "the image grid overflows".to_owned())?;
    if samples.len() < expected {
        return Err("the image sample data is shorter than its declared grid".to_owned());
    }
    for clear in placements {
        clear_region(
            &mut samples,
            stride,
            clear.layout,
            clear.ctm,
            &clear.regions,
        );
    }
    let encoded = flate_encode(&samples, 6)
        .ok_or_else(|| "the cleared samples could not be re-encoded".to_owned())?;
    let rgb = first
        .decoded
        .as_ref()
        .map(|_| (layout.width, layout.height));
    Ok((encoded, rgb))
}

/// Zeroes every sample whose centre lies in a region box under one placement.
///
/// The region is inverse-mapped into image space to bound the work, then each sample centre in
/// that block is mapped forward and tested exactly — so a rotated placement clears only the
/// samples truly inside the region, and the destruction is the region's and no more.
fn clear_region(
    samples: &mut [u8],
    stride: usize,
    layout: ImageLayout,
    ctm: Transform,
    regions: &[[f32; 4]],
) {
    let Some(inverse) = ctm.invert() else {
        return;
    };
    let (cols, rows) = candidate_block(inverse, layout, regions);
    let width = as_f32(layout.width);
    let height = as_f32(layout.height);
    for row in rows.0..rows.1 {
        for col in cols.0..cols.1 {
            // §8.9.5.2: the image is a unit square with the first sample at the upper-left, so
            // the first row is the top and v runs down as `row` increases.
            let u = (as_f32(col) + 0.5) / width;
            let v = 1.0 - (as_f32(row) + 0.5) / height;
            let point = ctm.apply(Point::new(u, v));
            if regions.iter().any(|region| contains(*region, point)) {
                zero_sample(samples, stride, layout, row, col);
            }
        }
    }
}

/// The block of sample columns and rows a region can reach, over-approximated from its inverse-
/// mapped corners and padded by one sample; the exact per-sample test narrows it.
fn candidate_block(
    inverse: Transform,
    layout: ImageLayout,
    regions: &[[f32; 4]],
) -> ((usize, usize), (usize, usize)) {
    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    let mut bottom = f32::INFINITY;
    let mut top = f32::NEG_INFINITY;
    for region in regions {
        for (x, y) in [
            (region[0], region[1]),
            (region[2], region[1]),
            (region[2], region[3]),
            (region[0], region[3]),
        ] {
            let point = inverse.apply(Point::new(x, y));
            left = left.min(point.x);
            right = right.max(point.x);
            bottom = bottom.min(point.y);
            top = top.max(point.y);
        }
    }
    // v is measured up the unit square, `row` down from the top, so the smaller row bounds the
    // larger v.
    (
        span(left, right, layout.width),
        span(1.0 - top, 1.0 - bottom, layout.height),
    )
}

/// A padded, clamped `[start, end)` index range for a normalised span `lo..hi` over `n` samples.
fn span(lo: f32, hi: f32, n: usize) -> (usize, usize) {
    let scale = as_f32(n);
    let start = clamp_index((lo * scale).floor() - 1.0, n);
    let end = clamp_index((hi * scale).ceil() + 1.0, n);
    (start, end.max(start))
}

/// A finite scalar clamped into `[0, max]` and taken as an index; a non-finite one is `0`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped into [0, max] before the cast, so it is a valid usize index"
)]
fn clamp_index(value: f32, max: usize) -> usize {
    if !value.is_finite() {
        return 0;
    }
    value.clamp(0.0, as_f32(max)) as usize
}

/// A sample count as `f32` for the sample-centre arithmetic.
#[expect(
    clippy::cast_precision_loss,
    reason = "a MAX_SAMPLES-bounded count's imprecision only shifts the candidate block, which \
              the exact per-sample test then narrows; it never leaves a region sample uncleared"
)]
fn as_f32(value: usize) -> f32 {
    value as f32
}

/// Whether a point lies in an axis-aligned box, edges included (a sample centre on the boundary
/// is cleared — the safe direction).
fn contains(box_: [f32; 4], point: Point) -> bool {
    point.x >= box_[0] && point.x <= box_[2] && point.y >= box_[1] && point.y <= box_[3]
}

/// Zeroes the `components × bits` bits of one sample, MSB first within the row (§8.9.5.2). All
/// bit depths go through the same bit loop, so a 1-bit mask and a 16-bit colour need no special
/// case; the arithmetic cannot overflow because [`row_stride`] proved the grid fits `usize`.
fn zero_sample(samples: &mut [u8], stride: usize, layout: ImageLayout, row: usize, col: usize) {
    let per_sample = layout.components.saturating_mul(layout.bits);
    let bit_start = col.saturating_mul(per_sample);
    let row_base = row.saturating_mul(stride);
    for offset in 0..per_sample {
        let bit = bit_start.saturating_add(offset);
        let byte = row_base.saturating_add(bit / 8);
        if let Some(cell) = samples.get_mut(byte) {
            // `0x80 >> (bit % 8)` is the MSB-first mask for this bit within its byte, the same
            // packing `pdf_model::image` reads samples back with.
            *cell &= !(0x80u8 >> (bit % 8));
        } else {
            break;
        }
    }
}

/// The offset just past the `EI` that ends an inline image, given `resume` (past `EI` and its
/// delimiting white space) and the run's start.
///
/// [`pdf_model::inline_image::scan`] resumes past the white space §7.2.3 lets follow `EI`; the
/// splice ends at `EI` itself so that separator, part of the surrounding stream, is left exactly
/// as it was. Where the bytes before the trimmed point are not `EI` — a scan this walk did not
/// produce — the untrimmed `resume` is used, which never leaves the run half-spliced.
fn end_of_ei(content: &[u8], start: usize, resume: usize) -> usize {
    let mut end = resume.min(content.len());
    while end > start
        && content
            .get(end.saturating_sub(1))
            .is_some_and(u8::is_ascii_whitespace)
    {
        end = end.saturating_sub(1);
    }
    if end >= start.saturating_add(2) && content.get(end.saturating_sub(2)..end) == Some(&b"EI"[..])
    {
        end
    } else {
        resume
    }
}

/// Builds an inline image's `BI` … `ID` … `EI` run from a source dictionary and freshly encoded
/// data (§8.9.7), or refuses where a carried entry cannot be written inline.
///
/// The source dictionary is the one [`pdf_model::inline_image::scan`] expanded — Table 91's keys
/// in full and Table 92's colour-space and filter abbreviations resolved — so every entry is
/// written the long way, which §8.9.7 permits ("the abbreviations … may be used in place of the
/// full names"). The old encoding is dropped and this writer's own `/Filter /FlateDecode` and
/// exact `/Length` stated for the re-encoded bytes; every other entry — the grid, the colour
/// space, `/Decode`, `/ImageMask`, `/Interpolate`, `/Intent` — is carried unchanged, because the
/// cleared samples are still on the grid it describes.
///
/// A carried value must be one an inline image may hold: §8.9.7 makes the run part of a content
/// stream, where §7.3.8's indirect references cannot appear, so a colour space resolved to a
/// resource object that holds a reference or a stream (an `/ICCBased` space, a `/Separation`
/// tint) cannot be restated inline. Such a page is **refused by name** rather than written with a
/// reference no content stream can carry (principle 1).
fn build_inline_image(dict: &Dictionary, encoded: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::from(&b"BI"[..]);
    for (key, value) in dict.iter() {
        match key.as_bytes() {
            // Dropped: the old encoding is replaced below. §8.9.7's `/L` and `/Length` are one
            // key, and `/DP` is `/DecodeParms`; the scan has already expanded both.
            b"Filter" | b"DecodeParms" | b"DP" | b"Length" | b"L" => continue,
            _ => {}
        }
        if !inline_safe(value) {
            return Err(format!(
                "§8.9.7: an inline image meeting the region states /{}, a value an inline image \
                 cannot carry (an indirect reference or stream lives in no content stream); the \
                 page is refused",
                String::from_utf8_lossy(key.as_bytes())
            ));
        }
        out.push(b' ');
        pdf_syntax::write::object(&Object::Name(key.clone()), &mut out);
        out.push(b' ');
        pdf_syntax::write::object(value, &mut out);
    }
    out.extend_from_slice(b" /Filter /FlateDecode /Length ");
    let mut length = String::new();
    let _ = write!(length, "{}", encoded.len());
    out.extend_from_slice(length.as_bytes());
    // §8.9.7: "the ID operator shall be followed by a single white-space character, and the next
    // character shall be interpreted as the first byte of image data" — one LINE FEED here — and
    // the /Length above "exclud[es] the white-space delimiting" the operators, so one LINE FEED
    // delimits the EI that follows the data.
    out.extend_from_slice(b" ID\n");
    out.extend_from_slice(encoded);
    out.extend_from_slice(b"\nEI");
    Ok(out)
}

/// Whether an object is one an inline image dictionary may carry (§8.9.7): a name, number,
/// boolean, string or an array of such. A dictionary, stream or indirect reference is not — a
/// content stream carries no §7.3.8 indirection — so a value holding one refuses the splice.
fn inline_safe(value: &Object) -> bool {
    match value {
        Object::Null
        | Object::Boolean(_)
        | Object::Integer(_)
        | Object::Real(_)
        | Object::String(_)
        | Object::Name(_) => true,
        Object::Array(items) => items.iter().all(inline_safe),
        Object::Dictionary(_) | Object::Stream(_) | Object::Reference(_) => false,
    }
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
    cleared_images: &[ClearedImage],
    plan: &RedactPlan,
    sinks: &dyn Sinks,
) -> Result<Written, Refusal> {
    let mut assembly = Assembly::new(vec![document]);
    for page in applied.iter_mut() {
        page.placed = assembly
            .replace(0, page.page_id)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
    }
    // A cleared image is replaced the same way a redacted page is: its slot is reserved before
    // the closure walk, so `copy_closure` short-circuits on it and the original samples are
    // reached from nowhere and never copied — which is what makes the destruction a destruction
    // rather than the file still holding the pixels behind a new object.
    let mut placed_images: Vec<(ObjectId, &ClearedImage)> =
        Vec::with_capacity(cleared_images.len());
    for image in cleared_images {
        let placed = assembly
            .replace(0, image.id)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
        placed_images.push((placed, image));
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

    for (placed, image) in placed_images {
        let object = build_cleared_image(&mut assembly, document, image)?;
        assembly
            .place(placed, object)
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

/// Builds a cleared image's replacement stream: the destroyed samples re-encoded as `FlateDecode`,
/// under a dictionary describing them.
///
/// A **codec-free** image keeps the source dictionary — its grid, colour space, bit depth,
/// `/Decode` and masks — with references carried into the output's numbering (like
/// [`build_page`]'s entries), because the samples are still on the grid it describes; only the
/// encoding is replaced. A **codec** image ([`ClearedImage::rgb`]) was decoded and re-expressed as
/// an opaque 8-bit `DeviceRGB` raster, so it gets a **fresh** dictionary stating exactly that —
/// carrying none of the source's codec `/Filter`, `/DecodeParms`, `/Decode`, colour space, or
/// masks, every one of which would misdescribe the new samples.
fn build_cleared_image(
    assembly: &mut Assembly<'_>,
    document: &Document,
    image: &ClearedImage,
) -> Result<Object, Refusal> {
    let mut dict = Dictionary::new();
    if let Some((width, height)) = image.rgb {
        dict.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"XObject"[..])),
        );
        dict.insert(
            Name::new(&b"Subtype"[..]),
            Object::Name(Name::new(&b"Image"[..])),
        );
        dict.insert(
            Name::new(&b"Width"[..]),
            Object::Integer(i64::try_from(width).unwrap_or(i64::MAX)),
        );
        dict.insert(
            Name::new(&b"Height"[..]),
            Object::Integer(i64::try_from(height).unwrap_or(i64::MAX)),
        );
        dict.insert(
            Name::new(&b"ColorSpace"[..]),
            Object::Name(Name::new(&b"DeviceRGB"[..])),
        );
        dict.insert(Name::new(&b"BitsPerComponent"[..]), Object::Integer(8));
    } else {
        let object = document.get(image.id);
        let source = object.as_stream().ok_or_else(|| {
            Refusal::Assembly(format!(
                "image object {} is not a stream at clearing time",
                image.id.number
            ))
        })?;
        for (key, value) in source.dict.iter() {
            match key.as_bytes() {
                // The old encoding is dropped and §7.3.8.2's /Length re-stated for the new bytes;
                // /Filter becomes the one filter this writer emits. Every other entry is carried
                // unchanged, because the samples are still on the grid it describes.
                b"Filter" | b"DecodeParms" | b"DP" | b"Length" => {}
                _ => {
                    dict.insert(key.clone(), carry(assembly, document, value, 0));
                }
            }
        }
    }
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(image.encoded.len()).unwrap_or(i64::MAX)),
    );
    Ok(Object::Stream(Arc::new(Stream {
        dict,
        data: Arc::from(image.encoded.as_slice()),
        decryption_failed: false,
    })))
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
