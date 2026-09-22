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
//! an image another page also draws is **copied for the redacted page** rather than replaced: the
//! marks that other placement draws are content the annotation did not identify, and the copy is
//! what the redacted page's resources name (ADR 1196).
//!
//! An image behind a **codec** — `DCTDecode` (§7.4.8), `CCITTFaxDecode` (§7.4.6) or `JBIG2Decode`
//! (§7.4.7) — cannot be zeroed in its packed grid, because its bytes are the codec's input rather
//! than samples. It is instead **decoded to samples the interpreter's own way**
//! ([`pdf_model::image::decode`], so the codec runs under the process's isolation — confined in
//! the program, principle 3), the region cleared in the decoded raster, and the result written as
//! a `FlateDecode` image: a lossless, non-codec filter, so the redacted region is exactly zero and
//! cannot round-trip back through the codec that would leak it. A colour codec becomes an 8-bit
//! `DeviceRGB` raster; the bilevel `JBIG2Decode` becomes a 1-bit `DeviceGray` raster, at its own
//! depth rather than inflated (ADR 1143), its zero constant one bit of black. The replacement is
//! opaque and carries no `/Decode`, `/Mask` or `/SMask`, so a codec image that decodes with
//! transparency (a soft mask, a colour key, or an image mask) is refused rather than flattened —
//! the re-encode would not preserve it. `JPXDecode` stays refused (below).
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
//! - an **image carrying §8.9.5.4 `/Alternates`**, because an alternate is a variant
//!   representation of the same picture and destroying the base's samples would leave the
//!   region readable in the variant — and §8.9.5.4 step c) draws one of them when the output is
//!   a printing;
//! - an **image encoded by `JPXDecode`** (§8.9.5) — because a codestream over the decoder's budget
//!   comes back at a reduced resolution level (§7.4.9 NOTE 3), so the raster is not the image's
//!   grid and a redaction that silently changed the image's resolution cannot be proven to have
//!   replaced the full-resolution content the region maps into. An image whose colour space this
//!   build cannot count the components of, or whose declared grid its sample data does not fill,
//!   is refused the same way;
//! - an **inline image** (§8.9.7) encoded by a codec, or whose colour space resolves to a
//!   resource object an inline image cannot carry, or whose declared grid its data does not fill
//!   — the same three limits as an image `XObject`, at the splice above rather than at a `Do`;
//! - a **painted path** whose marks the cut cannot take exactly: a stroke (§8.5.3.2's marks are
//!   the path's *outline*, so cutting the path would place caps and joins the producer never
//!   wrote), a path with a §8.5.2.2 Bézier segment (the crossing parameter is a root this build
//!   does not solve, and flattening it would approximate the producer's geometry), a path that is
//!   also §8.5.4's clipping boundary (cutting it would move the boundary every later mark is held
//!   to), one whose path object another operator interrupted, and one whose surviving coordinates
//!   are too large for [`paths::Cut::margin_holds`] to prove the cut edge cannot round into the
//!   region;
//! - a **form `XObject`** (§8.10) whose content does not decode, one that draws itself, or one
//!   the page's resources name directly rather than by reference, so no object can be replaced.
//!
//! # Cutting a painted path (§8.5, §12.5.6.23)
//!
//! A glyph's unit of removal is its code and an image's is its sample; a painted path has neither,
//! so its removal is **geometric**. The path's subpaths are cut to the complement of the region
//! and the surviving geometry is written back as fresh §8.5.2 construction operators, so the
//! coordinates that described the removed marks are gone from the file — a clip would have left
//! them, which is the failure the clause names for an image when it forbids a mask. A path wholly
//! inside the region loses its painting operator with its geometry; one clear of the region keeps
//! its own bytes. [`paths`] is the construction, its exactness argument, and the margin that makes
//! the rounding at the cut edge a proof rather than a hope. ADR 1195.
//!
//! # Entering a form (§8.10)
//!
//! A form `XObject` is "a self-contained description of any sequence of graphics objects", so the
//! marks it draws under the region are described in *its* stream and the walk enters it — always,
//! not only when it meets the region, because the interpreter runs a form's content inline and the
//! placed-code count this walk is calibrated against therefore includes the form's codes. §8.10.1's
//! step b) puts the form's `/Matrix` in front of the transform at the `Do`, and Table 93's
//! `/Resources` is what its names resolve in, the page's where it states none (ADR 0255).
//!
//! # A shared object is copied, never replaced (ADR 1196)
//!
//! An object drawn on the redacted page **and** somewhere else carries marks the annotation did
//! not identify, so destroying them would remove content nobody asked for. The removal is written
//! into a copy the redacted page's own `/Resources` names; the original stays exactly as the
//! producer wrote it. Everything on the path to that copy — the `/XObject` subdictionary, a
//! nested form's `/Resources` — is copied with it, and everything that reaches nothing replaced is
//! still shared.
//!
//! # What is a documented departure (A64/A65's fence)
//!
//! Table 195's `/OverlayText`, `/IC` and `/RO` describe the mark drawn *over* the region after
//! removal, and composing them is composing content the document did not hold — the far side of
//! `CLAUDE.md`'s authoring line. So the removal happens and the overlay is not drawn, reported
//! as a per-page [`crate::Departure`] from §12.5.6.23's full application semantics.

mod paths;

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
use pdf_syntax::serialize::{Assembly, Form, Options, flate_encode};

use crate::optimize::{catalog_of, copy_closure, refuse_a_document_only_recovery_reads};
use crate::pattern::{Fill, Pattern};
use crate::{Declined, Departure, Origin, Output, Protect, Refusal, Report, Sinks, Warning};

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
    protect: Option<&Protect>,
    report: &mut Report,
) -> Result<(), Refusal> {
    let document = documents.get(at).ok_or(Refusal::NoSuchSource {
        at: plan.source,
        count: documents.len(),
    })?;
    let root = catalog_of(document)?;
    refuse_a_document_only_recovery_reads(document, root)?;
    if document.is_encrypted() && protect.is_none() {
        // §12.5.6.23 requires a redaction to "remove all traces of the specified content", and a
        // person who put a password on a document meant its contents not to be read. Writing the
        // survivors into a file with no `/Encrypt` would answer the first requirement by
        // breaking the second, and it would do so silently — every remaining page of a
        // protected document in the clear. So a source the caller stated no protection for is
        // refused rather than downgraded; supplying one, through `apply_protected`, writes a
        // redaction protected by §7.6.4's handler instead. ADR 1162.
        return Err(Refusal::Assembly(
            "§7.6: this document is encrypted and no passwords were supplied to encrypt the \
             redaction with, so it is refused rather than written unprotected"
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
    let mut cleared_images = 0usize;
    let mut annotations = 0usize;
    let mut glyphs = 0usize;
    let mut inline_images = 0usize;
    let mut cut_paths = 0usize;
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
                // §12.5.6.23: destroy the image data. Grouped and cleared per page, so a shared
                // image's destroyed samples belong to this page's copy alone.
                let images = clear_images(document, edit.clears)?;
                annotations = annotations.saturating_add(regions.len());
                glyphs = glyphs.saturating_add(edit.removed);
                inline_images = inline_images.saturating_add(edit.inline_images);
                cut_paths = cut_paths.saturating_add(edit.paths);
                cleared_images = cleared_images.saturating_add(images.len());
                if regions.iter().any(|region| region.has_overlay) {
                    departures.push(index.saturating_add(1));
                }
                applied.push(AppliedPage {
                    page_id,
                    placed: page_id,
                    content: edit.content,
                    resources: page.resources.clone(),
                    images,
                    forms: edit.forms,
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

    // §12.5.6.23's destroyed images are the image XObjects cleared as new streams and the inline
    // images spliced in the content stream — both had their region samples set to the constant.
    let images = cleared_images.saturating_add(inline_images);

    let written = write_document(document, root, &mut applied, plan, sinks, protect)?;
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
            paths: cut_paths,
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
    /// The object whose stream is replaced, or copied where it is shared.
    id: ObjectId,
    /// Whether the replacement is a copy this page alone draws (§12.5.6.23 and the sharing rule
    /// in [`Walk::exclusively_owned`]).
    private: bool,
    /// The cleared samples, re-encoded `FlateDecode`.
    encoded: Vec<u8>,
    /// `Some(layout)` when the image was decoded from a codec and re-expressed as a fresh raster,
    /// so [`build_cleared_image`] must write a new dictionary stating that grid rather than carry
    /// the source's codec `/Filter`, colour space and masks: an opaque 8-bit `DeviceRGB` raster
    /// for a colour codec (`DCTDecode`, `CCITTFaxDecode`), a 1-bit `DeviceGray` raster for the
    /// bilevel `JBIG2Decode` (§7.4.7). `None` when the codec-free samples keep the source
    /// dictionary's grid.
    codec: Option<ImageLayout>,
}

/// Groups **one page's** image clears by object and destroys each image's samples once.
///
/// A page may draw one image twice, so the placements of one object are unioned into a single
/// cleared stream (§12.5.6.23). The grouping is per page rather than per document because a
/// shared image's destroyed samples are this page's alone: two pages redacting the same object
/// get a copy each, cleared under their own placements, and the original keeps the picture every
/// unredacted placement draws.
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
        let (encoded, codec) =
            cleared_image_samples(document, stream, &placements).map_err(|detail| {
                Refusal::Assembly(format!("§12.5.6.23: image object {}: {detail}", id.number))
            })?;
        let private = placements.iter().any(|clear| clear.private);
        cleared.push(ClearedImage {
            id,
            private,
            encoded,
            codec,
        });
    }
    Ok(cleared)
}

/// A page whose redactions were applied: the object it is, the slot it takes in the output, and
/// the content stream with the removed bytes gone.
struct AppliedPage {
    page_id: ObjectId,
    placed: ObjectId,
    content: Vec<u8>,
    /// The resource dictionary in effect for the page, after §7.7.3.4's inheritance — what a
    /// private replacement is written into where the page's own entry is shared.
    resources: Dictionary,
    /// The images whose samples this page's removal destroyed.
    images: Vec<ClearedImage>,
    /// The forms whose content this page's removal edited.
    forms: Vec<FormEdit>,
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
    /// How many painted paths (§8.5) this page cut to the region's complement.
    paths: usize,
    /// The form `XObject`s whose own content streams the removal edited (§8.10).
    forms: Vec<FormEdit>,
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
    /// Whether the image object is shared, so the destroyed samples must be written as a copy
    /// this page alone draws and the original left holding the picture another placement shows.
    private: bool,
    /// The decoded samples when the source image was behind a `DCTDecode` or `CCITTFaxDecode`
    /// codec: opaque 8-bit `DeviceRGB` from [`pdf_model::image::decode`], laid out as `layout`
    /// describes (3 components, 8 bits). `None` for a codec-free image, whose packed samples are
    /// read from the stream at clearing time. Held here because a codec's bytes are not samples,
    /// so the destruction cannot address the packed grid the source stream carries.
    decoded: Option<Arc<[u8]>>,
}

/// One form `XObject` (§8.10) whose own content stream the removal edited.
///
/// A form's marks are described inside its own stream, so removing the marks it draws under the
/// region is an edit of *that* stream rather than of the page's. Whether the edited stream
/// replaces the form or becomes a copy this page alone draws is decided by whether the form is
/// the page's own: see [`Walk::exclusively_owned`].
#[derive(Clone)]
struct FormEdit {
    /// The form object the page's resources name.
    id: ObjectId,
    /// Its content stream with the region's marks removed.
    content: Vec<u8>,
    /// Whether the form is shared, so the edited stream must be a copy placed for this page and
    /// the original left holding the marks the other pages' placements draw.
    private: bool,
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

/// One subpath of a path object, with whether the producer closed it.
///
/// The distinction is §8.5.3.1's: a fill closes every subpath implicitly — "any subpaths that
/// are open shall be implicitly closed before being filled" — and a stroke does not, because an
/// open subpath is stroked with §8.4.3.3's caps at its two ends and a closed one with a join.
struct BuiltSubPath {
    path: kurbo::BezPath,
    closed: bool,
}

/// What state the subpath under construction is in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Building {
    /// None has been begun since the last was finished.
    Nothing,
    /// One is being built and the producer has not closed it.
    Open,
    /// One is being built and an `h` or an `re` closed it.
    Closed,
}

/// A path object under construction (§8.2): §8.5.2's construction operators accumulate here until
/// a painting operator decides what becomes of it.
struct PathObject {
    /// The subpaths finished so far, in the content stream's own user space.
    subpaths: Vec<BuiltSubPath>,
    /// The subpath still being built, finished by `h`, `re`, a fresh `m`, or the painting
    /// operator.
    current: kurbo::BezPath,
    /// What state the subpath under construction is in.
    building: Building,
    /// §8.5.2's current point, which `v` uses as its first control point.
    current_point: kurbo::Point,
    /// Every point named so far, in the display list's space — the box the region is tested
    /// against. A curve contributes its control points, which contain the curve, so the box
    /// over-approximates and never misses a region.
    bbox: Option<[f32; 4]>,
    /// Whether §8.5.4's `W` or `W*` made this path the clipping boundary as well as a mark.
    clips: bool,
    /// Whether an operator that is not path construction ran while the path was open, so the
    /// byte range between the first operand and the painting operator is not the path's alone.
    interrupted: bool,
    /// Where the path's first operand starts, which is where a cut's replacement begins.
    start: usize,
    /// The transform in force when the path was begun; `interrupted` is what proves it is still
    /// the one in force at the painting operator.
    ctm: Transform,
}

impl PathObject {
    fn new(start: usize, ctm: Transform) -> Self {
        Self {
            subpaths: Vec::new(),
            current: kurbo::BezPath::new(),
            building: Building::Nothing,
            current_point: kurbo::Point::ZERO,
            bbox: None,
            clips: false,
            interrupted: false,
            start,
            ctm,
        }
    }

    /// Notes one user-space point in the device-space box, without adding to the geometry.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a content-stream path coordinate is far inside f32"
    )]
    fn mark(&mut self, x: f64, y: f64) {
        let point = self.ctm.apply(Point::new(x as f32, y as f32));
        let bbox = self
            .bbox
            .get_or_insert([point.x, point.y, point.x, point.y]);
        bbox[0] = bbox[0].min(point.x);
        bbox[1] = bbox[1].min(point.y);
        bbox[2] = bbox[2].max(point.x);
        bbox[3] = bbox[3].max(point.y);
    }

    /// Begins a new subpath at a point, finishing whatever was being built.
    fn begin(&mut self, x: f64, y: f64) {
        self.finish();
        self.current.move_to(kurbo::Point::new(x, y));
        self.building = Building::Open;
        self.current_point = kurbo::Point::new(x, y);
        self.mark(x, y);
    }

    /// Appends a straight segment. A file stating one with no current point has begun the
    /// subpath there, which is the reading that loses nothing.
    fn line(&mut self, x: f64, y: f64) {
        if self.building == Building::Nothing {
            self.begin(x, y);
            return;
        }
        self.current.line_to(kurbo::Point::new(x, y));
        self.current_point = kurbo::Point::new(x, y);
        self.mark(x, y);
    }

    /// Appends a §8.5.2.2 cubic Bézier by its three remaining control points.
    ///
    /// Every control point is marked as well as drawn, so the device-space box covers the curve's
    /// control polygon — which contains the curve — and a curve near the region is never missed.
    fn curve(&mut self, one: kurbo::Point, two: kurbo::Point, three: kurbo::Point) {
        if self.building == Building::Nothing {
            return;
        }
        self.current.curve_to(one, two, three);
        self.current_point = three;
        for point in [one, two, three] {
            self.mark(point.x, point.y);
        }
    }

    /// `re` (§8.5.2.1): a complete rectangular subpath, which closes itself.
    fn rectangle(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.finish();
        self.current.move_to(kurbo::Point::new(x, y));
        self.current.line_to(kurbo::Point::new(x + width, y));
        self.current
            .line_to(kurbo::Point::new(x + width, y + height));
        self.current.line_to(kurbo::Point::new(x, y + height));
        self.current.close_path();
        self.building = Building::Closed;
        for corner in [
            (x, y),
            (x + width, y),
            (x + width, y + height),
            (x, y + height),
        ] {
            self.mark(corner.0, corner.1);
        }
        self.finish();
    }

    /// `h` (§8.5.2.1): closes the subpath under construction.
    fn close(&mut self) {
        if self.building != Building::Nothing {
            self.current.close_path();
            self.building = Building::Closed;
            self.finish();
        }
    }

    /// Finishes the subpath under construction, if it has one, and starts a fresh one.
    fn finish(&mut self) {
        if self.building != Building::Nothing {
            self.subpaths.push(BuiltSubPath {
                path: std::mem::take(&mut self.current),
                closed: self.building == Building::Closed,
            });
        }
        self.current = kurbo::BezPath::new();
        self.building = Building::Nothing;
    }

    /// The subpaths as §8.5.3.1's fill sees them: "any subpaths that are open shall be
    /// implicitly closed before being filled".
    fn filled_rings(&self) -> Vec<paths::SubPath> {
        self.subpaths
            .iter()
            .map(|subpath| {
                let mut ring = subpath.path.clone();
                if !subpath.closed {
                    ring.close_path();
                }
                ring
            })
            .collect()
    }

    /// The whole path as §8.5.3.2's stroke sees it, with `closes` applying §8.5.3.1's `s` — "the
    /// same effect as the sequence h S", which closes the *current* subpath and so the last one.
    fn stroked_path(&self, closes: bool) -> kurbo::BezPath {
        let mut out = kurbo::BezPath::new();
        let last = self.subpaths.len().saturating_sub(1);
        for (index, subpath) in self.subpaths.iter().enumerate() {
            out.extend(subpath.path.elements().iter().copied());
            if !subpath.closed && closes && index == last {
                out.close_path();
            }
        }
        out
    }
}

/// The per-stream state [`Walk::run_form`] sets aside while it walks a form's own content.
///
/// The graphics state a form inherits is everything but its own transform and its own resources
/// (§8.10.1), so the text state and the region are not here: they carry into the form unchanged.
/// The edits are here because each belongs to the byte string it indexes, and a form's stream is
/// not the page's.
struct Frame {
    edits: Vec<(usize, usize, Vec<u8>)>,
    resources: Dictionary,
    ctm: Transform,
    ctm_stack: Vec<Transform>,
    path: Option<PathObject>,
    graphics: GraphicsState,
    graphics_stack: Vec<GraphicsState>,
}

/// The parts of §8.4's graphics state a cut of a §8.5.3.2 stroke needs, and nothing else.
///
/// Three groups, each for one question the cut has to answer:
///
/// - **§8.4.3's line parameters** decide the outline the stroke marks: `w`, `J`, `j`, `M` and
///   `d`, together with the `/LW`, `/LC`, `/LJ`, `/ML` and `/D` an `ExtGState` may state.
/// - **§8.6.8's stroking colour** decides what the outline is painted with. The cut writes the
///   surviving outline back as a *fill*, because §8.5.3.2's marks are the region the outline
///   encloses and `f` is the operator that paints one — so the fill has to be given the stroking
///   colour, which is done by replaying the producer's own colour operands under the
///   corresponding non-stroking operator of Table 74. What is held is the operand **bytes** the
///   file wrote, so no number is reformatted and no colour is reinterpreted.
/// - **§11.6.4.4's two constant alphas**, because a fill takes `/ca` where a stroke takes `/CA`.
///   Where an `ExtGState` has made them differ the substitution would change what is drawn,
///   the page is refused instead.
///
/// Saved and restored by `q` and `Q` (§8.4.2), like the transform beside it. ADR 1236.
#[derive(Clone, Debug)]
struct GraphicsState {
    /// §8.4.3.2's line width, "in user space units". Table 51's initial value is 1.0.
    width: f64,
    /// §8.4.3.3's line cap style, Table 52: 0 butt, 1 round, 2 projecting square.
    cap: i64,
    /// §8.4.3.4's line join style, Table 53: 0 miter, 1 round, 2 bevel.
    join: i64,
    /// §8.4.3.5's miter limit, whose initial value Table 51 gives as 10.0.
    miter_limit: f64,
    /// §8.4.3.6's dash array, in user space units; empty is the solid line Table 51 starts with.
    dashes: Vec<f64>,
    /// §8.4.3.6's dash phase.
    dash_phase: f64,
    /// The operand bytes of the last `CS`, to be replayed as `cs`, where one has run.
    space: Option<Vec<u8>>,
    /// The operand bytes of the last `SC`, `SCN`, `G`, `RG` or `K`, with the non-stroking
    /// operator of Table 74 they are replayed under.
    value: Option<(Vec<u8>, &'static str)>,
    /// §11.6.4.4's `/CA`, the stroking alpha constant. Table 58's initial value is 1.0.
    stroking_alpha: f64,
    /// §11.6.4.4's `/ca`, the non-stroking alpha constant.
    fill_alpha: f64,
}

impl Default for GraphicsState {
    /// Table 51's initial values, which are what a content stream starts with (§8.4.1).
    fn default() -> Self {
        Self {
            width: 1.0,
            cap: 0,
            join: 0,
            miter_limit: 10.0,
            dashes: Vec::new(),
            dash_phase: 0.0,
            space: None,
            value: None,
            stroking_alpha: 1.0,
            fill_alpha: 1.0,
        }
    }
}

impl GraphicsState {
    /// How far past a path's own control points its marks reach when it is stroked in this
    /// state.
    ///
    /// Half the line width by §8.4.3.2, and a miter join reaches further: §8.4.3.5 makes the limit
    /// "the maximum length of a miter as a ratio of the line width", so the limit times half the
    /// width bounds every join. Used to widen the box the region is tested against, so that a
    /// centre line outside the region whose *stroke* reaches in is not missed.
    fn stroke_reach(&self) -> f64 {
        self.width / 2.0 * self.miter_limit.max(1.0)
    }

    /// The `kurbo` style that expands this state's stroke into the outline §8.5.3.2 marks.
    fn stroke_style(&self) -> kurbo::Stroke {
        kurbo::Stroke::new(self.width)
            .with_caps(match self.cap {
                1 => kurbo::Cap::Round,
                2 => kurbo::Cap::Square,
                _ => kurbo::Cap::Butt,
            })
            .with_join(match self.join {
                1 => kurbo::Join::Round,
                2 => kurbo::Join::Bevel,
                _ => kurbo::Join::Miter,
            })
            // §8.4.3.5 defines the limit as a ratio of at least 1; a smaller value from a
            // malformed file behaves as the smallest legal one.
            .with_miter_limit(self.miter_limit.max(1.0))
            .with_dashes(self.dash_phase, self.dashes.iter().copied())
    }

    /// The operators that give a fill this state's **stroking** colour, or `None` where no
    /// stroking colour operator has run.
    ///
    /// Table 74 pairs each stroking operator with the non-stroking one that sets the same thing,
    /// and the operands written are the file's own bytes rather than numbers this program
    /// re-formatted. A `CS` with no `SC` or `SCN` after it is enough on its own: §8.6.8 has that
    /// operator "set the colour to its initial value" for the space it names, and `cs` sets the
    /// same initial value for the same space.
    fn non_stroking_colour(&self) -> Option<Vec<u8>> {
        if self.space.is_none() && self.value.is_none() {
            return None;
        }
        let mut out = Vec::new();
        if let Some(space) = &self.space {
            out.extend_from_slice(space);
            out.extend_from_slice(b" cs\n");
        }
        if let Some((operands, operator)) = &self.value {
            out.extend_from_slice(operands);
            out.push(b' ');
            out.extend_from_slice(operator.as_bytes());
            out.push(b'\n');
        }
        Some(out)
    }

    /// Records one stroking colour operator's operands under the non-stroking operator that sets
    /// the same thing (§8.6.8, Table 74).
    ///
    /// `G`, `RG` and `K` name a device colour space as well as a colour, so they clear whatever
    /// `CS` had stated; `SC` and `SCN` set a colour inside the space `CS` named and leave it.
    fn record_colour(&mut self, keyword: &[u8], operands: Vec<u8>) {
        match keyword {
            b"CS" => {
                self.space = Some(operands);
                self.value = None;
            }
            b"SC" => self.value = Some((operands, "sc")),
            b"SCN" => self.value = Some((operands, "scn")),
            b"G" => {
                self.space = None;
                self.value = Some((operands, "g"));
            }
            b"RG" => {
                self.space = None;
                self.value = Some((operands, "rg"));
            }
            b"K" => {
                self.space = None;
                self.value = Some((operands, "k"));
            }
            _ => {}
        }
    }
}

/// Whether an operator sets §8.6.8's **stroking** colour, which the cut of a stroke replays.
fn is_stroking_colour_operator(keyword: &[u8]) -> bool {
    matches!(keyword, b"CS" | b"SC" | b"SCN" | b"G" | b"RG" | b"K")
}

/// Whether an operator belongs inside a path object (§8.2): §8.5.2's construction operators,
/// §8.5.4's clip operators, and the painting operators that end one.
fn is_path_operator(keyword: &[u8]) -> bool {
    matches!(
        keyword,
        b"m" | b"l"
            | b"c"
            | b"v"
            | b"y"
            | b"re"
            | b"h"
            | b"W"
            | b"W*"
            | b"f"
            | b"F"
            | b"f*"
            | b"S"
            | b"s"
            | b"B"
            | b"B*"
            | b"b"
            | b"b*"
            | b"n"
    )
}

/// How closely `kurbo::stroke` is asked to follow the true outline of a stroke.
///
/// A tenth of [`paths::REGION_PAD`], the widening the cut is proven against — and the honest
/// statement of the number is that it decides nothing this build admits. The tolerance governs
/// the approximation of arcs, and [`paths::is_polygonal`] refuses every expansion that produced
/// one; what is left is the straight-segment case, whose offsets, butt and projecting-square caps
/// and miter and bevel joins the expansion computes in closed form. A later round that admits an
/// arc owes the margin arithmetic for this value too, carried into the display list's space by
/// the mapping's norm the way [`paths::Cut::margin_holds`] carries the other two roundings.
/// ADR 1236.
const STROKE_TOLERANCE: f64 = paths::REGION_PAD / 10.0;

/// Tables 52 and 53 each state three integer codes for a line style.
///
/// A value outside them is a malformed file and takes code 0 — butt caps and miter joins, which
/// is the initial value Table 51 gives both.
#[expect(
    clippy::cast_possible_truncation,
    reason = "Table 52 and Table 53 state the codes 0, 1 and 2; everything else falls to 0"
)]
fn style_code(value: f64) -> i64 {
    match value as i64 {
        code @ (1 | 2) => code,
        _ => 0,
    }
}

/// One expanded outline split into the closed subpaths a cut takes one at a time.
///
/// `kurbo::stroke` returns every contour of the outline in one path, each begun by a `MoveTo`;
/// the cut's Sutherland–Hodgman construction is stated per subpath, so they are separated here.
fn split_subpaths(outline: &kurbo::BezPath) -> Vec<paths::SubPath> {
    let mut out: Vec<paths::SubPath> = Vec::new();
    for element in outline.elements() {
        if matches!(element, kurbo::PathEl::MoveTo(_)) {
            out.push(kurbo::BezPath::new());
        }
        if let Some(current) = out.last_mut() {
            current.push(*element);
        }
    }
    for subpath in &mut out {
        if !matches!(subpath.elements().last(), Some(kurbo::PathEl::ClosePath)) {
            subpath.close_path();
        }
    }
    out
}

/// Whether this path object's own bytes are a cut's to replace, or the refusal by name.
///
/// Three questions, none of them about the geometry: whether cutting would move something other
/// than the marks (§8.5.4's clipping boundary), whether the byte range the replacement occupies
/// is the path's alone (§8.2's path object), and whether the path has any area in device space
/// at all (§8.3.4).
fn admits_a_cut(path: &PathObject) -> Result<(), String> {
    if path.clips {
        return Err(
            "§8.5.4: the path meeting the region is also the clipping path (W or W*), which \
             bounds every mark after the painting operator; cutting its geometry would move that \
             boundary and change content the annotation did not identify; the page is refused"
                .to_owned(),
        );
    }
    if path.interrupted {
        return Err(
            "§8.2: an operator that is not path construction ran inside the path object meeting \
             the region, so the bytes the cut would replace are not the path's alone; the page \
             is refused"
                .to_owned(),
        );
    }
    if path.ctm.determinant() == 0.0 {
        return Err(
            "§8.3.4: the transform in force where a painted path meets the region is singular, \
             so the path has no area in device space to cut; the page is refused"
                .to_owned(),
        );
    }
    Ok(())
}

/// One set of subpaths cut to the complement of every region, or the refusal by name.
fn cut_to_complement(
    subpaths: &[paths::SubPath],
    regions: &[[f64; 4]],
    to_display: paths::Mapping,
) -> Result<Vec<paths::SubPath>, String> {
    let cut = paths::subtract(subpaths, regions, to_display).ok_or_else(|| {
        "§12.5.6.23: cutting a painted path against these regions exceeds this build's bound on \
         the surviving pieces; the page is refused rather than truncated"
            .to_owned()
    })?;
    if !cut.margin_holds() {
        return Err(
            "§7.3.3: a coordinate of the cut path is large enough that writing it back and \
             reading it as a single-precision real could move the cut edge inside the region; \
             the page is refused rather than leave a sliver of the removed marks"
                .to_owned(),
        );
    }
    Ok(cut.polygons)
}

/// The transform as the double-precision mapping [`paths`] takes its decisions in.
fn mapping(transform: Transform) -> paths::Mapping {
    paths::Mapping {
        a: f64::from(transform.a),
        b: f64::from(transform.b),
        c: f64::from(transform.c),
        d: f64::from(transform.d),
        e: f64::from(transform.e),
        f: f64::from(transform.f),
    }
}

/// The content-stream walk: it enumerates codes in the interpreter's order, tests each placed
/// quadrilateral against the region, and edits the bytes.
struct Walk<'a> {
    document: &'a Document,
    page: &'a Page,
    /// The resource dictionary the stream now running resolves its names in — the page's, or a
    /// form's own where it states one (§7.8.3, §8.10.1, ADR 0255).
    resources: Dictionary,
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
    /// The path object under construction (§8.5.2), or `None` between path objects.
    path: Option<PathObject>,
    /// §8.4's graphics state, as far as cutting a §8.5.3.2 stroke needs it.
    graphics: GraphicsState,
    /// What `q` saved and `Q` restores (§8.4.2).
    graphics_stack: Vec<GraphicsState>,
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
    /// How many painted paths this page cut against the region (§8.5, §12.5.6.23).
    paths_cut: usize,
    /// The form `XObject`s whose own content the removal edited.
    form_edits: Vec<FormEdit>,
    /// The forms whose content streams are on this walk's stack, so a form that draws itself is
    /// refused rather than followed for ever.
    forms_open: Vec<ObjectId>,
    /// How many forms on that stack are shared. An object reached only through a form another
    /// page also draws is that page's too, whatever its own reference count says, so nothing
    /// inside one may be replaced in place.
    inside_shared: usize,
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
            resources: page.resources.clone(),
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
            graphics: GraphicsState::default(),
            graphics_stack: Vec::new(),
            code_index: 0,
            removed: 0,
            edits: Vec::new(),
            counts,
            clears: Vec::new(),
            inline_cleared: 0,
            paths_cut: 0,
            form_edits: Vec::new(),
            forms_open: Vec::new(),
            inside_shared: 0,
        }
    }

    /// Walks the page's content stream, returning the edited bytes or a refusal by name.
    fn run(mut self, content: &[u8]) -> Result<PageEdit, String> {
        self.run_stream(content)?;
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
            paths: self.paths_cut,
            forms: self.form_edits,
        })
    }

    /// Walks one content stream — the page's, or a form's under [`Walk::run_form`] — leaving its
    /// edits in `self.edits`.
    fn run_stream(&mut self, content: &[u8]) -> Result<(), String> {
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
                        // §8.6.8's stroking colour is recorded as the **bytes** the producer
                        // wrote, because a cut stroke is painted as a fill and the fill has to
                        // be given that colour without any number being re-formatted. The
                        // operands run from the first of them to this keyword.
                        if is_stroking_colour_operator(keyword)
                            && let Some((_, from)) = operands.first()
                        {
                            let bytes = content.get(*from..start).unwrap_or_default();
                            let bytes = bytes.trim_ascii_end();
                            if !bytes.is_empty() {
                                self.graphics.record_colour(keyword, bytes.to_vec());
                            }
                        }
                        self.operator(keyword, start, &operands)?;
                    }
                    operands.clear();
                }
            }
        }
        Ok(())
    }

    /// Dispatches one operator against the operands scanned before it.
    fn operator(
        &mut self,
        keyword: &[u8],
        keyword_start: usize,
        operands: &[(Operand, usize)],
    ) -> Result<(), String> {
        if !is_path_operator(keyword)
            && let Some(path) = self.path.as_mut()
        {
            path.interrupted = true;
        }
        match keyword {
            b"q" => {
                self.ctm_stack.push(self.ctm);
                self.graphics_stack.push(self.graphics.clone());
            }
            b"Q" => {
                if let Some(previous) = self.ctm_stack.pop() {
                    self.ctm = previous;
                }
                if let Some(previous) = self.graphics_stack.pop() {
                    self.graphics = previous;
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
            // §8.4.3's line parameters, which decide the outline a stroke marks.
            b"w" => {
                if let Some(width) = plain_numbers(operands).first() {
                    self.graphics.width = *width;
                }
            }
            b"J" => {
                if let Some(cap) = plain_numbers(operands).first() {
                    self.graphics.cap = style_code(*cap);
                }
            }
            b"j" => {
                if let Some(join) = plain_numbers(operands).first() {
                    self.graphics.join = style_code(*join);
                }
            }
            b"M" => {
                if let Some(limit) = plain_numbers(operands).first() {
                    self.graphics.miter_limit = *limit;
                }
            }
            b"d" => self.set_dash(operands),
            b"m" => self.begin_subpath(operands),
            b"l" => self.extend_subpath(operands),
            b"c" | b"v" | b"y" => self.curve_subpath(keyword, operands),
            b"re" => self.add_rectangle(operands),
            b"h" => self.close_subpath(),
            b"W" | b"W*" => {
                if let Some(path) = self.path.as_mut() {
                    path.clips = true;
                }
            }
            b"f" | b"F" | b"f*" | b"S" | b"s" | b"B" | b"B*" | b"b" | b"b*" => {
                self.paint_path(keyword, keyword_start)?;
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
        let fonts = self.document.get_key(&self.resources, "Font");
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

    /// `m` (§8.5.2.1): begins a new subpath at the given point.
    fn begin_subpath(&mut self, operands: &[(Operand, usize)]) {
        self.open_path(operands);
        let numbers = plain_numbers(operands);
        if let Some(path) = self.path.as_mut()
            && let [x, y] = numbers.as_slice()
        {
            path.begin(*x, *y);
        }
    }

    /// `l` (§8.5.2.1): appends a straight segment to the subpath under construction.
    fn extend_subpath(&mut self, operands: &[(Operand, usize)]) {
        self.open_path(operands);
        let numbers = plain_numbers(operands);
        if let Some(path) = self.path.as_mut()
            && let [x, y] = numbers.as_slice()
        {
            path.line(*x, *y);
        }
    }

    /// `c`, `v`, `y` (§8.5.2.2): a cubic Bézier, in the three spellings the clause gives it.
    ///
    /// `c` states all three remaining control points, and the curve runs from the current point
    /// to the last of them.
    /// `v` states two pairs and uses the current point as its first control point; `y` states two
    /// pairs and uses the final point as its second. All three are one curve, and the cut splits
    /// it at the parameter where it crosses the region's edge rather than flattening it
    /// (`paths::crossings`, ADR 1236).
    fn curve_subpath(&mut self, keyword: &[u8], operands: &[(Operand, usize)]) {
        self.open_path(operands);
        let numbers = plain_numbers(operands);
        let Some(path) = self.path.as_mut() else {
            return;
        };
        match (keyword, numbers.as_slice()) {
            (b"c", [x1, y1, x2, y2, x3, y3]) => path.curve(
                kurbo::Point::new(*x1, *y1),
                kurbo::Point::new(*x2, *y2),
                kurbo::Point::new(*x3, *y3),
            ),
            (b"v", [x2, y2, x3, y3]) => {
                let current = path.current_point;
                path.curve(
                    current,
                    kurbo::Point::new(*x2, *y2),
                    kurbo::Point::new(*x3, *y3),
                );
            }
            (b"y", [x1, y1, x3, y3]) => {
                let end = kurbo::Point::new(*x3, *y3);
                path.curve(kurbo::Point::new(*x1, *y1), end, end);
            }
            _ => {}
        }
    }

    /// `re` (§8.5.2.1): a complete rectangular subpath, which closes itself.
    fn add_rectangle(&mut self, operands: &[(Operand, usize)]) {
        self.open_path(operands);
        let numbers = plain_numbers(operands);
        if let Some(path) = self.path.as_mut()
            && let [x, y, w, h] = numbers.as_slice()
        {
            path.rectangle(*x, *y, *w, *h);
        }
    }

    /// `h` (§8.5.2.1): closes the subpath under construction.
    fn close_subpath(&mut self) {
        if let Some(path) = self.path.as_mut() {
            path.close();
        }
    }

    /// `d` (§8.4.3.6): the dash pattern and phase.
    ///
    /// The pattern decides which stretches of a path are marked at all, so it is applied
    /// **before** the outline is cut: `kurbo::stroke` takes it in the style and emits each dash
    /// as its own contour of the outline, which the cut then treats like any other geometry.
    /// A malformed array — one with a negative entry, or one whose entries are all zero — leaves
    /// the solid line §8.4.3.6 says a conforming file would have written.
    fn set_dash(&mut self, operands: &[(Operand, usize)]) {
        let Some((Operand::Array(items), _)) = operands.first() else {
            return;
        };
        let dashes: Vec<f64> = items
            .iter()
            .filter_map(|item| match item {
                ArrayElement::Number(value) => Some(*value),
                ArrayElement::Str(_) => None,
            })
            .collect();
        if dashes.iter().any(|dash| *dash < 0.0) || dashes.iter().all(|dash| *dash == 0.0) {
            self.graphics.dashes = Vec::new();
            self.graphics.dash_phase = 0.0;
            return;
        }
        self.graphics.dashes = dashes;
        self.graphics.dash_phase = plain_numbers(operands).first().copied().unwrap_or_default();
    }

    /// Opens a path object if none is open, remembering where its first operand starts — the
    /// offset a cut's replacement bytes begin at.
    fn open_path(&mut self, operands: &[(Operand, usize)]) {
        if self.path.is_none() {
            let start = operands.first().map_or(0, |(_, at)| *at);
            self.path = Some(PathObject::new(start, self.ctm));
        }
    }

    /// A painted path that meets the region has the region's share of its marks **cut out**
    /// (§12.5.6.23: the content is removed, not covered), or the page is refused by name where
    /// the cut cannot be taken exactly (trap 5, principle 1).
    ///
    /// The surviving geometry replaces the whole path object — its construction operators and
    /// its painting operator — so the coordinates that described the removed marks are gone from
    /// the file rather than clipped away. [`paths`] is the construction and its exactness
    /// argument; what is decided here is which paths it may be applied to.
    ///
    /// **A painting operator states up to two marks and each is cut on its own.** §8.5.3 gives
    /// `B` and its three relatives a fill *and* a stroke, and the two are different regions in
    /// different colours: the fill is the path's interior with §8.5.3.1's implicit close, the
    /// stroke is the outline §8.5.3.2 marks along it. So the replacement is the cut fill painted
    /// first and the cut outline second, which is the clause's own order — "[f]ill and then
    /// stroke the path". ADR 1236.
    fn paint_path(&mut self, keyword: &[u8], keyword_start: usize) -> Result<(), String> {
        let Some(mut path) = self.path.take() else {
            return Ok(());
        };
        path.finish();
        let Some(bbox) = path.bbox else {
            return Ok(());
        };
        // §8.5.3.1 and §8.5.3.2: which of the two marks this operator makes, and whether it
        // closes the current subpath first ("s … the same effect as the sequence h S").
        let (fill, stroked, closes) = match keyword {
            b"f" | b"F" => (Some("f"), false, false),
            b"f*" => (Some("f*"), false, false),
            b"S" => (None, true, false),
            b"s" => (None, true, true),
            b"B" => (Some("f"), true, false),
            b"B*" => (Some("f*"), true, false),
            b"b" => (Some("f"), true, true),
            b"b*" => (Some("f*"), true, true),
            _ => return Ok(()),
        };
        // A stroke's marks reach past the path's own points by half the line width and by what a
        // join adds, so the box the region is tested against is widened by that much first.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a line width carried into device space is a page coordinate, inside f32"
        )]
        let widened = {
            let reach = if stroked {
                (self.graphics.stroke_reach() * mapping(path.ctm).norm()) as f32
            } else {
                0.0
            };
            [
                bbox[0] - reach,
                bbox[1] - reach,
                bbox[2] + reach,
                bbox[3] + reach,
            ]
        };
        if !self.regions.iter().any(|region| overlaps(*region, widened)) {
            // The path is nowhere near a redaction: its bytes cross the output untouched.
            return Ok(());
        }
        admits_a_cut(&path)?;
        let regions = self.region_bounds();
        let to_display = mapping(path.ctm);
        let mut replacement: Vec<u8> = Vec::new();
        if let Some(fill) = fill {
            let cut = cut_to_complement(&path.filled_rings(), &regions, to_display)?;
            if !cut.is_empty() {
                let mut text = String::new();
                paths::write_polygons(&mut text, &cut);
                text.push_str(fill);
                text.push('\n');
                replacement.extend_from_slice(text.as_bytes());
            }
        }
        if stroked {
            let outline = self.stroke_outline(&path, closes)?;
            let cut = cut_to_complement(&outline, &regions, to_display)?;
            if !cut.is_empty() {
                let colour = self.graphics.non_stroking_colour().ok_or_else(|| {
                    "§8.6.8: a stroked path meets the region and no stroking colour operator has \
                     run in this content stream before it, so the colour the surviving outline \
                     would be filled with is one this walk has not seen stated; the page is \
                     refused rather than painted a colour the file does not state"
                        .to_owned()
                })?;
                if (self.graphics.stroking_alpha - self.graphics.fill_alpha).abs() > 0.0 {
                    return Err(
                        "§11.6.4.4: an ExtGState has made the stroking alpha constant /CA differ \
                         from the non-stroking /ca, and a surviving outline is painted as a \
                         fill, which takes the second; the page is refused rather than drawn at \
                         an opacity the file does not state for it"
                            .to_owned(),
                    );
                }
                let mut text = String::new();
                paths::write_polygons(&mut text, &cut);
                // Balanced inside the replacement (§8.4.2), so the fill colour the producer set
                // for whatever comes next is the one that comes back.
                replacement.extend_from_slice(b"q\n");
                replacement.extend_from_slice(&colour);
                replacement.extend_from_slice(text.as_bytes());
                // §8.5.3.2's marks are the region the outline encloses, and `kurbo::stroke` winds
                // every contour of that outline the same way — so the nonzero rule is the one
                // that paints it, and `f*` would put holes where the outline overlaps itself.
                replacement.extend_from_slice(b"f\nQ\n");
            }
        }
        self.paths_cut = self.paths_cut.saturating_add(1);
        self.edits.push((
            path.start,
            keyword_start.saturating_add(keyword.len()),
            replacement,
        ));
        Ok(())
    }

    /// The outline §8.5.3.2's stroke marks, as the subpaths a fill of it would paint.
    ///
    /// > The S operator shall paint a line along the current path.
    ///
    /// That line is the region a pen of the current width sweeps along the path, closed off by
    /// §8.4.3.3's caps and turned by §8.4.3.4's joins, with §8.4.3.6's dash pattern deciding
    /// which stretches are marked at all. `kurbo::stroke` computes it, dashes included, and what
    /// comes back is geometry the cut takes exactly as it takes a fill's.
    ///
    /// **Admitted only where the expansion is exact**, which [`paths::is_polygonal`] decides from
    /// the output rather than from this tree's model of the input: offsetting a straight segment
    /// and closing it with a butt or projecting-square cap and a miter or bevel join is computed
    /// in closed form, while a round cap, a round join and the offset of a curved segment are
    /// *approximations* of arcs. Cutting an approximation would replace the producer's marks
    /// outside the region with marks this program computed, which is the far side of
    /// `CLAUDE.md`'s provenance line, so those are refused by name.
    fn stroke_outline(
        &self,
        path: &PathObject,
        closes: bool,
    ) -> Result<Vec<paths::SubPath>, String> {
        if self.graphics.width <= 0.0 {
            return Err(
                "§8.4.3.2: a stroked path meets the region with a line width of 0, which shall \
                 denote the thinnest line that can be rendered at device resolution, one device \
                 pixel wide — a width in device pixels rather than in the user space an outline \
                 is written in; the page is refused"
                    .to_owned(),
            );
        }
        let source = path.stroked_path(closes);
        let outline = kurbo::stroke(
            source.elements().iter().copied(),
            &self.graphics.stroke_style(),
            &kurbo::StrokeOpts::default(),
            STROKE_TOLERANCE,
        );
        if !paths::is_polygonal(&outline) {
            return Err(
                "§8.5.3.2: the outline of a stroked path meeting the region came back with an \
                 arc in it — a round cap or join (§8.4.3.3, §8.4.3.4), or the offset of a \
                 §8.5.2.2 curve — which an expansion can only approximate; the page is refused \
                 rather than have the producer's marks outside the region replaced by an \
                 approximation of them"
                    .to_owned(),
            );
        }
        Ok(split_subpaths(&outline))
    }

    /// The region boxes as double-precision numbers, which the cut's decisions are taken in.
    fn region_bounds(&self) -> Vec<[f64; 4]> {
        self.regions
            .iter()
            .map(|region| {
                [
                    f64::from(region[0]),
                    f64::from(region[1]),
                    f64::from(region[2]),
                    f64::from(region[3]),
                ]
            })
            .collect()
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
        let xobjects = self.document.get_key(&self.resources, "XObject");
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
            b"Form" => self.run_form(&name, &entry, &object),
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

    /// `Do` on a form `XObject` (§8.10): the walk **enters** the form's own content stream.
    ///
    /// A form is "a self-contained description of any sequence of graphics objects" (§8.10.1), so
    /// the marks it draws under the region are described in *its* stream and removing them is an
    /// edit of that stream. Entering it is not optional even for a form the region misses: the
    /// interpreter runs a form's content inline, so every code it shows is in the placed
    /// quadrilaterals this walk is held to, and a walk that skipped the form would disagree with
    /// the interpreter's count and refuse every page whose text is inside one.
    ///
    /// §8.10.1's step b) concatenates the form's `/Matrix` with the CTM before its content runs,
    /// and Table 93's `/Resources` is what its names resolve in — the page's dictionary where it
    /// states none, which is the direction §7.8.3's NOTE 3 gives and the reading ADR 0255 fixed.
    /// The graphics state is otherwise inherited, so the text state a `Tf` outside the form set
    /// is still in force inside it.
    ///
    /// Refused by name: a form whose content does not decode, and a form that draws itself
    /// (§8.10.1 states no recursion, and a walk that followed one would not end).
    fn run_form(&mut self, name: &[u8], entry: &Object, object: &Object) -> Result<(), String> {
        let shown = String::from_utf8_lossy(name);
        let Some(stream) = object.as_stream() else {
            return Err(format!(
                "the content draws the form /{shown}, which is not a stream; the page is refused"
            ));
        };
        let form_id = entry.as_reference();
        if let Some(id) = form_id
            && self.forms_open.contains(&id)
        {
            return Err(format!(
                "§8.10.1: the form /{shown} draws itself; the page is refused rather than walked \
                 for ever"
            ));
        }
        if self.forms_open.len() >= MAX_DEPTH {
            return Err(format!(
                "§8.10.1: forms are nested deeper than this removal walks at /{shown}; the page \
                 is refused"
            ));
        }
        let Some(data) = self.document.decoded_stream_data(stream) else {
            return Err(format!(
                "§8.10.1: the form /{shown} did not decode, so what it draws under the region \
                 cannot be read; the page is refused"
            ));
        };

        let matrix = numbers(self.document, &stream.dict, "Matrix")
            .and_then(|values| {
                matrix(
                    &values
                        .iter()
                        .map(|value| f64::from(*value))
                        .collect::<Vec<_>>(),
                )
            })
            .unwrap_or(Transform::IDENTITY);
        let resources = self
            .document
            .get_key(&stream.dict, "Resources")
            .as_dict()
            .cloned()
            .unwrap_or_else(|| self.page.resources.clone());

        let inner_ctm = matrix.then(self.ctm);
        let saved = Frame {
            edits: std::mem::take(&mut self.edits),
            resources: std::mem::replace(&mut self.resources, resources),
            ctm: std::mem::replace(&mut self.ctm, inner_ctm),
            ctm_stack: std::mem::take(&mut self.ctm_stack),
            path: self.path.take(),
            // §8.10.1: a form inherits the graphics state, and its own `q`/`Q` nesting is its
            // own — so the state carries in and the stack is set aside.
            graphics: self.graphics.clone(),
            graphics_stack: std::mem::take(&mut self.graphics_stack),
        };
        let shared = !form_id.is_some_and(|id| self.owns(id));
        if let Some(id) = form_id {
            self.forms_open.push(id);
        }
        if shared {
            self.inside_shared = self.inside_shared.saturating_add(1);
        }
        let walked = self.run_stream(&data);
        if shared {
            self.inside_shared = self.inside_shared.saturating_sub(1);
        }
        if form_id.is_some() {
            self.forms_open.pop();
        }
        let edits = std::mem::replace(&mut self.edits, saved.edits);
        self.resources = saved.resources;
        self.ctm = saved.ctm;
        self.ctm_stack = saved.ctm_stack;
        self.path = saved.path;
        self.graphics = saved.graphics;
        self.graphics_stack = saved.graphics_stack;
        walked?;

        if edits.is_empty() {
            // Nothing of this form falls under the region: its stream crosses the output byte
            // for byte, and so does the `Do` that draws it.
            return Ok(());
        }
        let Some(id) = form_id else {
            return Err(format!(
                "§12.5.6.23: the form /{shown} draws marks under the region but is a direct \
                 object this removal cannot replace; the page is refused"
            ));
        };
        self.form_edits.push(FormEdit {
            id,
            content: apply_edits(&data, edits),
            private: !self.owns(id),
        });
        Ok(())
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
        let Some(stream) = object.as_stream() else {
            return Err(format!(
                "the image /{shown} is not a stream; the page is refused"
            ));
        };
        // §8.9.5.4's `/Alternates` are "variant representations of the base image", and
        // §12.5.6.23 requires the data in the region to be destroyed rather than hidden: an
        // alternate holds the same picture at another resolution or in another colour space, so
        // clearing the base alone would leave the redacted content in the file and — since step
        // c) draws the `/DefaultForPrinting` alternate when the output is a printing — on paper.
        // Destroying an alternate's samples too is a capability this writer does not have (each
        // is its own grid, its own filter, and may be shared), so the page is refused by name.
        if self.document.get_key(&stream.dict, "Alternates") != Object::Null {
            return Err(format!(
                "§12.5.6.23: the image /{shown} states §8.9.5.4 /Alternates, whose variant \
                 representations of the same picture this removal does not destroy; the page is \
                 refused rather than leave the region readable in an alternate"
            ));
        }
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
            private: !self.owns(image_id),
            decoded: None,
        })
    }

    /// Plans clearing a `DCTDecode`, `CCITTFaxDecode` or `JBIG2Decode` image by decoding it to
    /// samples, or refuses by name (§8.9.5, §7.4.8, §7.4.6, §7.4.7).
    ///
    /// A codec's stream bytes are its input, not samples, so the packed-grid destruction the
    /// codec-free path takes cannot address them. The image is decoded the interpreter's own way
    /// ([`pdf_model::image::decode`], so the codec runs under the process's isolation — confined
    /// in the program, principle 3) to straight-alpha `RGBA8`, and the opaque samples are carried
    /// to clear. The result is written as a `FlateDecode` image ([`build_cleared_image`]): a
    /// lossless, non-codec filter, so the cleared region is exactly zero and cannot round-trip
    /// back through a codec that would leak it. A colour codec (`DCTDecode`, `CCITTFaxDecode`) is
    /// re-expressed as an opaque 8-bit `DeviceRGB` raster; the bilevel `JBIG2Decode` (§7.4.7) is
    /// re-expressed at its own depth, a 1-bit `DeviceGray` raster whose zero sample is one bit
    /// (ADR 1143), rather than inflated to eight.
    ///
    /// Refused, each an owed capability rather than a silence (trap 5, principle 1): `JPXDecode`,
    /// whose over-budget decode is a reduced resolution level (§7.4.9 NOTE 3), so the raster is not
    /// the image's grid; any other codec; an image that does not decode; one that decodes short of
    /// its grid, so the re-encode would drop rows; or one that decodes with transparency (a soft
    /// mask, a colour key, or an image mask), which the opaque re-encode cannot preserve.
    fn plan_codec_clear(
        &self,
        shown: &str,
        image_id: ObjectId,
        stream: &Stream,
        codec: &[u8],
    ) -> Result<ImageClear, String> {
        // A bilevel codec re-expresses at 1 bit; a colour codec at 8-bit RGB. Every other codec is
        // refused by name — `JPXDecode` with its own reason (§7.4.9 NOTE 3), the rest generically.
        let bilevel = match codec {
            b"DCTDecode" | b"DCT" | b"CCITTFaxDecode" | b"CCF" => false,
            b"JBIG2Decode" => true,
            other => {
                return Err(format!(
                    "§8.9.5: the image /{shown} is encoded with the {} codec, whose samples this \
                     removal does not re-encode; the page is refused",
                    String::from_utf8_lossy(other)
                ));
            }
        };
        let Flattened { image, shortfall } = pdf_model::image::decode(
            self.document,
            stream,
            &self.resources,
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
        // opaque decode is representable as an opaque raster. A soft mask, a colour key or an
        // image mask leaves some sample non-opaque, and dropping it would change the picture —
        // refuse rather than flatten (principle 1).
        for pixel in image.data.chunks_exact(4) {
            if pixel.get(3) != Some(&0xFF) {
                return Err(format!(
                    "§8.9.5: the codec image /{shown} carries transparency the FlateDecode \
                     re-encode cannot preserve; the page is refused rather than flatten it"
                ));
            }
        }
        let (samples, layout) = if bilevel {
            bilevel_samples(&image.data, width, height)
        } else {
            colour_samples(&image.data, width, height)
        };
        Ok(ImageClear {
            image_id,
            ctm: self.ctm,
            regions: self.regions.clone(),
            layout,
            private: !self.owns(image_id),
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
            let components = ColourSpace::parse(self.document, &space, &self.resources)
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

    /// Whether the redacted page owns an object outright, so the removal may replace it rather
    /// than copy it: the reference test below, **and** no shared form between the page and it —
    /// an object reached only through a form another page draws is that page's too, however few
    /// references name it (ADR 1196).
    fn owns(&self, id: ObjectId) -> bool {
        self.inside_shared == 0 && self.exclusively_owned(id)
    }

    /// Whether an object is safe to overwrite in place: referenced exactly once in the
    /// document, and reached by a resource path this page does not share, so its marks are the
    /// redacted page's alone, so the removal may replace it. An object this cannot prove is the
    /// page's is copied for the page instead ([`private_copies`], ADR 1196).
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
    fn ext_gstate(&mut self, operands: &[(Operand, usize)]) -> Result<(), String> {
        let Some(name) = operands.iter().find_map(|(operand, _)| match operand {
            Operand::Name(bytes) => Some(bytes.clone()),
            _ => None,
        }) else {
            return Ok(());
        };
        let states = self.document.get_key(&self.resources, "ExtGState");
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
        let Some(dict) = state.as_ref().and_then(Object::as_dict) else {
            return Ok(());
        };
        // Table 58 states these in §8.4.3's own terms — `/LW` "[t]he line width", `/LC` "[t]he
        // line cap style", `/LJ` "[t]he line join style", `/ML` "[t]he miter limit", `/D` "[t]he
        // line dash pattern" — so they set exactly what the operators beside them set and are
        // read into the same state. `/CA` and `/ca` are §11.6.4.4's two alpha constants, which
        // decide whether a stroke's outline may be painted as a fill at all.
        let number = |key: &str| self.document.get_key(dict, key).as_number();
        if let Some(width) = number("LW") {
            self.graphics.width = width;
        }
        if let Some(cap) = number("LC") {
            self.graphics.cap = style_code(cap);
        }
        if let Some(join) = number("LJ") {
            self.graphics.join = style_code(join);
        }
        if let Some(limit) = number("ML") {
            self.graphics.miter_limit = limit;
        }
        if let Some(alpha) = number("CA") {
            self.graphics.stroking_alpha = alpha;
        }
        if let Some(alpha) = number("ca") {
            self.graphics.fill_alpha = alpha;
        }
        let dash = self.document.get_key(dict, "D");
        if let Some([array, phase]) = dash.as_array() {
            let array = self.document.resolve(array);
            let dashes: Vec<f64> = array
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| self.document.resolve(item).as_number())
                        .collect()
                })
                .unwrap_or_default();
            if dashes.iter().any(|dash| *dash < 0.0) || dashes.iter().all(|dash| *dash == 0.0) {
                self.graphics.dashes = Vec::new();
                self.graphics.dash_phase = 0.0;
            } else {
                self.graphics.dashes = dashes;
                self.graphics.dash_phase =
                    self.document.resolve(phase).as_number().unwrap_or_default();
            }
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
            &self.resources,
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

/// The opaque colour components of a decoded codec image as 8-bit `DeviceRGB` samples, contiguous
/// (§8.9.5.2 packs 8-bit samples with no intra-row padding), and the grid that describes them.
///
/// A colour codec (`DCTDecode`, `CCITTFaxDecode`) is re-expressed at eight bits per component so a
/// lossless `FlateDecode` stream can carry the pixels the lossy or bilevel filter held (ADR 1133).
/// The alpha the decode leaves was proved fully opaque before this is called, so the fourth byte
/// of each pixel carries nothing and is dropped.
fn colour_samples(rgba: &[u8], width: usize, height: usize) -> (Vec<u8>, ImageLayout) {
    let mut samples = Vec::with_capacity(width.saturating_mul(height).saturating_mul(3));
    for pixel in rgba.chunks_exact(4) {
        if let Some(rgb) = pixel.get(..3) {
            samples.extend_from_slice(rgb);
        }
    }
    let layout = ImageLayout {
        width,
        height,
        components: 3,
        bits: 8,
    };
    (samples, layout)
}

/// A decoded `JBIG2Decode` image (§7.4.7) as 1-bit `DeviceGray` samples, MSB-first per row with
/// each row filled to a byte boundary (§8.9.5.2), and the grid that describes them.
///
/// A bilevel codec is re-expressed at its own one bit per pixel rather than inflated to eight
/// (ADR 1143): a white pixel is sample 1 (Dmax), a black pixel sample 0 (Dmin), which is the
/// `DeviceGray` sense of the bits [`pdf_model::image`] delivers (Table 87). Clearing zeroes the
/// bit ([`zero_sample`]), so the region's zero constant is one bit of black — §8.9.5.2's sample 0
/// through the default `/Decode`. The decode was proved fully opaque before this is called.
fn bilevel_samples(rgba: &[u8], width: usize, height: usize) -> (Vec<u8>, ImageLayout) {
    let layout = ImageLayout {
        width,
        height,
        components: 1,
        bits: 1,
    };
    let stride = width.div_ceil(8);
    let mut samples = vec![0u8; stride.saturating_mul(height)];
    // A zero-width grid decodes to no bytes and needs none: the row walk below would divide the
    // input into empty chunks, which `chunks_exact` forbids, so it is answered before the walk.
    let row_pixels = width.saturating_mul(4);
    if row_pixels == 0 {
        return (samples, layout);
    }
    for (row, line) in rgba.chunks_exact(row_pixels).enumerate() {
        let base = row.saturating_mul(stride);
        for (col, pixel) in line.chunks_exact(4).enumerate() {
            // A white pixel sets its sample bit; a black one leaves the zero the buffer starts at.
            if pixel.first().is_some_and(|luma| *luma >= 0x80) {
                let byte = base.saturating_add(col / 8);
                if let Some(cell) = samples.get_mut(byte) {
                    // `0x80 >> (col % 8)` is the MSB-first mask for this sample, the packing
                    // [`zero_sample`] clears with and `pdf_model::image` reads samples back with.
                    *cell |= 0x80u8 >> (col % 8);
                }
            }
        }
    }
    (samples, layout)
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

/// The re-encoded cleared samples, and the fresh grid the writer must redescribe them by where the
/// source was behind a codec (`None` for a codec-free image that keeps its dictionary).
type ClearedSamples = (Vec<u8>, Option<ImageLayout>);

/// The image's decoded samples with every region's samples zeroed, re-encoded with `FlateDecode`,
/// and the fresh grid to redescribe it by where the source was behind a codec.
///
/// §12.5.6.23: "that portion of the image data shall be destroyed". For a **codec-free** image the
/// samples come from [`Document::image_stream`], which runs every filter before the codec, so they
/// are the packed samples themselves; the codec-free precondition was proved at planning time
/// ([`Walk::plan_image_clear`]) and is re-checked here rather than trusted across the two phases.
/// For a **codec** image ([`Walk::plan_codec_clear`]) the re-expressed samples were decoded then —
/// 8-bit `DeviceRGB` for a colour codec, 1-bit `DeviceGray` for the bilevel `JBIG2Decode` — and
/// travel on the placement; the source stream's bytes are the codec's input, not samples, so they
/// are never read here. Either way every placement's region is cleared and the result re-encoded
/// `FlateDecode`; the returned grid is `Some` exactly for the codec case, so the writer states a
/// fresh dictionary for the raster's own colour space and depth.
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
    let codec = first.decoded.as_ref().map(|_| layout);
    Ok((encoded, codec))
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

/// An object the redacted page owns outright, whose replacement takes its slot in the output.
enum Owned<'a> {
    /// An image `XObject` whose samples the removal destroyed.
    Image(&'a ClearedImage),
    /// A form `XObject` whose own content the removal edited.
    Form(&'a FormEdit),
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
    protect: Option<&Protect>,
) -> Result<Written, Refusal> {
    let mut assembly = Assembly::new(vec![document]);
    for page in applied.iter_mut() {
        page.placed = assembly
            .replace(0, page.page_id)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
    }
    // An object the redacted page **owns** is replaced the same way the page is: its slot is
    // reserved before the closure walk, so `copy_closure` short-circuits on it and the original
    // samples or marks are reached from nowhere and never copied — which is what makes the
    // destruction a destruction rather than the file still holding what was removed behind a new
    // object. A **shared** object cannot be replaced, because another page's placement draws it
    // and that placement's marks are content the annotation did not identify; it is copied for
    // this page instead, below.
    // Two kinds of replacement, and the difference is whether the redacted page owns the object.
    //
    // An object it **owns** takes the original's slot: reserved before the closure walk, so
    // `copy_closure` short-circuits on it and the original samples or marks are reached from
    // nowhere and never copied — which is what makes the destruction a destruction rather than
    // the file still holding what was removed behind a new object. An object another page also
    // draws is **copied** instead, into a slot of its own, because that page's placement draws
    // marks the annotation did not identify (ADR 1196).
    //
    // Every slot is taken before any object is built, so a copy's own resources can name the
    // page's other copies: a form holding a nested shared form is the case that needs it, and
    // building in one pass would have left the nested copy reachable from nothing while the page
    // went on drawing the original.
    let mut replacements: Vec<(ObjectId, Owned<'_>, usize)> = Vec::new();
    let mut private: Vec<HashMap<ObjectId, ObjectId>> = Vec::with_capacity(applied.len());
    for (index, page) in applied.iter().enumerate() {
        let mut copies = HashMap::new();
        for image in &page.images {
            let slot = if image.private {
                assembly.reserve()
            } else {
                assembly.replace(0, image.id)
            }
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
            if image.private {
                copies.insert(image.id, slot);
            }
            replacements.push((slot, Owned::Image(image), index));
        }
        for form in &page.forms {
            let slot = if form.private {
                assembly.reserve()
            } else {
                assembly.replace(0, form.id)
            }
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
            if form.private {
                copies.insert(form.id, slot);
            }
            replacements.push((slot, Owned::Form(form), index));
        }
        private.push(copies);
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

    for (index, page) in applied.iter().enumerate() {
        let copies = private.get(index).cloned().unwrap_or_default();
        let object = build_page(&mut assembly, document, page, &copies)?;
        assembly
            .place(page.placed, object)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
    }

    for (placed, source, index) in replacements {
        let copies = private.get(index).cloned().unwrap_or_default();
        let object = match source {
            Owned::Image(image) => build_cleared_image(&mut assembly, document, image, &copies)?,
            Owned::Form(form) => build_form(&mut assembly, document, form, &copies)?,
        };
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
    let written = Protect::write(
        protect,
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
///
/// Where the removal had to copy an object rather than replace it — a form or an image another
/// page's placement also draws — the page is also given a `/Resources` of its own, so that the
/// copies are what *this* page resolves its names to and every other page keeps the original.
fn build_page(
    assembly: &mut Assembly<'_>,
    document: &Document,
    page: &AppliedPage,
    private: &HashMap<ObjectId, ObjectId>,
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
        let skipped = matches!(key.as_bytes(), b"Contents" | b"Annots")
            || (key.as_bytes() == b"Resources" && !private.is_empty());
        if skipped {
            continue;
        }
        dict.insert(key.clone(), carry(assembly, document, value, 0));
    }
    dict.insert(Name::new(&b"Contents"[..]), Object::Reference(content_id));
    if !private.is_empty() {
        // §7.7.3.3 lets a page state its own `/Resources`, and §7.7.3.4's inheritance is what
        // `Page::resources` has already resolved — so writing the effective dictionary here is
        // the same resources the producer's page had, with the copied objects substituted.
        let resources = privatise(
            assembly,
            document,
            &Object::Dictionary(page.resources.clone()),
            private,
            0,
        );
        dict.insert(Name::new(&b"Resources"[..]), resources);
    }
    if let Some(annots) = surviving {
        dict.insert(Name::new(&b"Annots"[..]), annots);
    }
    Ok(Object::Dictionary(dict))
}

/// Carries a value into the output, copying every object on the path to a privately replaced one.
///
/// A dictionary that *reaches* a replaced object is itself shared — the page's `/XObject`
/// subdictionary is the usual one, a nested form's `/Resources` the next — so carrying it by
/// reference would point every other page at the copy. It is rebuilt instead, and everything that
/// reaches nothing replaced is carried by reference and shared as before, so the copying stops
/// where it stops mattering.
fn privatise(
    assembly: &mut Assembly<'_>,
    document: &Document,
    value: &Object,
    private: &HashMap<ObjectId, ObjectId>,
    depth: usize,
) -> Object {
    if depth >= MAX_DEPTH {
        return Object::Null;
    }
    let next = depth.saturating_add(1);
    match value {
        Object::Reference(id) => {
            if let Some(placed) = private.get(id) {
                return Object::Reference(*placed);
            }
            if !reaches(document, *id, private, &mut Vec::new(), 0) {
                return carry(assembly, document, value, depth);
            }
            let object = document.get(*id);
            let copy = privatise(assembly, document, &object, private, next);
            match assembly.add(copy) {
                Ok(placed) => Object::Reference(placed),
                Err(_) => Object::Null,
            }
        }
        Object::Array(items) => Object::Array(
            items
                .iter()
                .map(|item| privatise(assembly, document, item, private, next))
                .collect(),
        ),
        Object::Dictionary(dict) => {
            let mut out = Dictionary::new();
            for (key, entry) in dict.iter() {
                out.insert(
                    key.clone(),
                    privatise(assembly, document, entry, private, next),
                );
            }
            Object::Dictionary(out)
        }
        Object::Stream(stream) => {
            let mut out = Dictionary::new();
            for (key, entry) in stream.dict.iter() {
                out.insert(
                    key.clone(),
                    privatise(assembly, document, entry, private, next),
                );
            }
            Object::Stream(Arc::new(Stream {
                dict: out,
                data: Arc::clone(&stream.data),
                decryption_failed: stream.decryption_failed,
            }))
        }
        other => other.clone(),
    }
}

/// Whether any object privately replaced for this page is reachable from `id`.
///
/// `seen` is the chain of objects already on this descent, which is what makes a `/Parent` or any
/// other cycle terminate: an object already being visited answers nothing new.
fn reaches(
    document: &Document,
    id: ObjectId,
    private: &HashMap<ObjectId, ObjectId>,
    seen: &mut Vec<ObjectId>,
    depth: usize,
) -> bool {
    if private.contains_key(&id) {
        return true;
    }
    if depth >= MAX_DEPTH || seen.contains(&id) {
        return false;
    }
    seen.push(id);
    let object = document.get(id);
    let found = reaches_in(document, &object, private, seen, depth.saturating_add(1));
    seen.pop();
    found
}

/// The same question asked of a value rather than an object.
fn reaches_in(
    document: &Document,
    value: &Object,
    private: &HashMap<ObjectId, ObjectId>,
    seen: &mut Vec<ObjectId>,
    depth: usize,
) -> bool {
    if depth >= MAX_DEPTH {
        return false;
    }
    let next = depth.saturating_add(1);
    match value {
        Object::Reference(id) => reaches(document, *id, private, seen, next),
        Object::Array(items) => items
            .iter()
            .any(|item| reaches_in(document, item, private, seen, next)),
        Object::Dictionary(dict) => dict
            .iter()
            .any(|(_key, entry)| reaches_in(document, entry, private, seen, next)),
        Object::Stream(stream) => stream
            .dict
            .iter()
            .any(|(_key, entry)| reaches_in(document, entry, private, seen, next)),
        _ => false,
    }
}

/// Builds a form `XObject`'s replacement stream: its own content with the region's marks cut out,
/// under the dictionary the producer wrote (§8.10.2 Table 93) with only the encoding restated.
///
/// Every other entry — `/BBox`, `/Matrix`, `/Resources`, `/Group` — describes the form and still
/// does: what changed is the marks its content draws, not the space it draws them in.
fn build_form(
    assembly: &mut Assembly<'_>,
    document: &Document,
    form: &FormEdit,
    private: &HashMap<ObjectId, ObjectId>,
) -> Result<Object, Refusal> {
    let object = document.get(form.id);
    let source = object.as_stream().ok_or_else(|| {
        Refusal::Assembly(format!(
            "form object {} is not a stream where its marks must be removed",
            form.id.number
        ))
    })?;
    let mut dict = Dictionary::new();
    for (key, value) in source.dict.iter() {
        match key.as_bytes() {
            // The producer's encoding is dropped and §7.3.8.2's `/Length` restated: the content
            // written here is the decoded stream with the region's marks cut out of it.
            b"Filter" | b"DecodeParms" | b"DP" | b"Length" => {}
            _ => {
                dict.insert(
                    key.clone(),
                    privatise(assembly, document, value, private, 0),
                );
            }
        }
    }
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(form.content.len()).unwrap_or(i64::MAX)),
    );
    Ok(Object::Stream(Arc::new(Stream {
        dict,
        data: Arc::from(form.content.as_slice()),
        decryption_failed: false,
    })))
}

/// Builds a cleared image's replacement stream: the destroyed samples re-encoded as `FlateDecode`,
/// under a dictionary describing them.
///
/// A **codec-free** image keeps the source dictionary — its grid, colour space, bit depth,
/// `/Decode` and masks — with references carried into the output's numbering (like
/// [`build_page`]'s entries), because the samples are still on the grid it describes; only the
/// encoding is replaced. A **codec** image ([`ClearedImage::codec`]) was decoded and re-expressed
/// as a fresh raster, so it gets a **fresh** dictionary stating exactly that grid — an opaque
/// 8-bit `DeviceRGB` raster for a colour codec, a 1-bit `DeviceGray` raster for the bilevel
/// `JBIG2Decode` — carrying none of the source's codec `/Filter`, `/DecodeParms`, `/Decode`,
/// colour space, or masks, every one of which would misdescribe the new samples.
fn build_cleared_image(
    assembly: &mut Assembly<'_>,
    document: &Document,
    image: &ClearedImage,
    private: &HashMap<ObjectId, ObjectId>,
) -> Result<Object, Refusal> {
    let mut dict = Dictionary::new();
    if let Some(layout) = image.codec {
        // One component is the bilevel `DeviceGray` raster, three the colour `DeviceRGB` one; the
        // depth is the raster's own (§7.4.7's one bit, or eight). Both are opaque and carry no
        // `/Decode`, `/Mask` or `/SMask`, so the default `/Decode` maps sample 0 to Dmin — black.
        let space: &[u8] = if layout.components == 1 {
            b"DeviceGray"
        } else {
            b"DeviceRGB"
        };
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
            Object::Integer(i64::try_from(layout.width).unwrap_or(i64::MAX)),
        );
        dict.insert(
            Name::new(&b"Height"[..]),
            Object::Integer(i64::try_from(layout.height).unwrap_or(i64::MAX)),
        );
        dict.insert(
            Name::new(&b"ColorSpace"[..]),
            Object::Name(Name::new(space)),
        );
        dict.insert(
            Name::new(&b"BitsPerComponent"[..]),
            Object::Integer(i64::try_from(layout.bits).unwrap_or(i64::MAX)),
        );
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
                    dict.insert(
                        key.clone(),
                        privatise(assembly, document, value, private, 0),
                    );
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
        Object::Reference(id) => {
            // A reference to an object already placed — copied, or **replaced** by a reserved slot
            // (a redacted page, a cleared image) — maps to that slot without walking its source
            // value. Walking a replaced image's original dictionary would copy the codec resources
            // the replacement dropped back into the output: a `/JBIG2Globals` symbol dictionary is
            // the case that exposed this, and leaving it would be an orphan holding the original
            // bilevel image's bytes (principle 1, §12.5.6.23). A codec-free replacement stated its
            // resources inline, which is why the leak was latent until §7.4.7's globals stream.
            if let Some(mapped) = assembly.copied(0, *id) {
                return Object::Reference(mapped);
            }
            match copy_closure(assembly, document, *id, true) {
                Ok(mapped) => Object::Reference(mapped),
                Err(_) => Object::Null,
            }
        }
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
