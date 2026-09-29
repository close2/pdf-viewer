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
//! An image behind a **codec** — `DCTDecode` (§7.4.8), `CCITTFaxDecode` (§7.4.6), `JBIG2Decode`
//! (§7.4.7) or `JPXDecode` (§7.4.9) — cannot be zeroed in its packed grid, because its bytes are
//! the codec's input rather than samples. But a codec is a filter, and Table 87 says what it
//! delivers: one-bit samples from the two bilevel filters and eight-bit samples from `DCTDecode`,
//! with `/BitsPerComponent` consistent with them. Those are the image's
//! samples, the integers its `/ColorSpace`, `/Decode`, colour key and a mask's `/Matte` describe,
//! so they are taken from the codec run as a filter ([`pdf_model::image::filter_samples`], under
//! the process's isolation — confined in the program, principle 3), cleared in the grid the
//! dictionary states, and written under `FlateDecode` with the dictionary carried: a lossless,
//! non-codec filter, so the redacted region is exactly zero and cannot round-trip back through
//! the codec that would leak it (ADR 1371). A **`JPXDecode`** image is written as the decoder's
//! own integers, each component at its own depth in the field of the widest, in the colour space
//! §7.4.9 names ([`pdf_model::image::jpx_samples`], ADRs 1333 and 1371). It is decoded at full
//! resolution within the operator's `--image-samples` budget, never at the reduced level a viewer
//! may take.
//!
//! Nothing outside the region is converted, requantised or resampled, so a picture's §8.9.6.4
//! **colour key** is carried with its samples, and a picture pre-blended with §11.6.5.2's
//! **matte** is cleared to the matte, which Table 144's formula gives at the zero opacity the
//! cleared soft mask states, in whatever colour space the picture is in (ADRs 1333, 1371).
//!
//! **A mask is image data too** (ADR 1277). A picture's §11.6.5.2 `/SMask` and §8.9.6.3 `/Mask`
//! streams hold its shape sample by sample, and both lie on the picture's unit square, so each is
//! cleared as an image of its own — on its own grid, under the picture's placement — and the
//! picture names the cleared copy. A codec picture is decoded with its masks set aside, so they
//! are carried rather than multiplied into an alpha the re-encode has no channel for; a
//! `JPXDecode` picture whose `/SMaskInData` puts the opacity inside the codestream has that
//! channel cleared beside its samples and written as the soft-mask image Table 87 says the
//! processor creates from it.
//!
//! An **inline image** (§8.9.7's `BI`\u{2026}`ID`\u{2026}`EI`) has its samples in the content
//! stream itself rather than a referenced object, so destroying them is a **content-stream
//! splice**: the whole run is replaced with a freshly built inline image whose region samples are
//! the same zero constant, re-encoded `FlateDecode`, and every byte of `/Contents` outside the
//! run is left exactly as it was. The replacement carries every §8.9.7 key the source stated
//! (grid, colour space, `/Decode`, `/ImageMask`\u{2026}) with only the encoding replaced, a colour
//! space resource under the name the producer gave it; one behind `DCTDecode` or
//! `CCITTFaxDecode` carries the samples its filter delivers, as an image `XObject` behind them
//! does.
//!
//! # Every font's codes, and where a Type 3 glyph marks (ADR 1351)
//!
//! The code is the unit of removal, so a string is split exactly where a reader splits it: one
//! byte for a simple font, and for a composite font as many bytes as §9.7.6.2's codespace ranges
//! extract for each code, through the `CMap` the font loader resolves
//! ([`pdf_font::composite_cmap`]) and the function it decodes with. A Type 3 glyph is a content
//! stream that may mark past its advance, so its code is tested against the box its description
//! declares — `d1`'s, or the font's `/FontBBox` for a `d0` glyph, or where that box is all zero
//! the box its own marks fill when the description is run ([`type3`], ADR 1363) — carried to the
//! page by `/FontMatrix` and §9.4.4's text rendering matrix. The glyph description itself is the
//! font's and stays; only the code that invoked it goes, and its advance is restored like any
//! other — in writing mode 1 by §9.4.4's `ty`, from the descendant's `/W2` and `/DW2`.
//!
//! # `sh`, soft-mask groups and a stroke's own state (ADRs 1351, 1352)
//!
//! `sh` paints the clip, so where it meets the region the clip is cut: the shading is painted
//! through the clip intersected with the region's complement ([`shading`]). A soft mask's group
//! is content that marks the page as shape or opacity, so it is entered at the `gs` that
//! establishes it and cut like any other stream. A stroke's surviving outline is filled in the
//! stroke's own colour, alpha and overprint — stated by a graphics state dictionary the stream's
//! resources gain where the fill's differ.
//!
//! # Located shading data, a hairline and a form met twice (ADR 1363)
//!
//! A shading's data located in the region alone — a mesh's vertices and corners, a radial
//! shading's inner circles, a function-based shading's samples — are destroyed where they are
//! stated, and the shading is written afresh for the page ([`located`], [`mesh`]); an axial
//! shading's values lie on lines that leave any region, so it is cut by its clip alone. A stroke
//! of width 0 is cut along its path rather than as an outline, since §8.4.3.2 states its width in
//! device pixels. A form drawn more than once that the region meets differently is given a copy
//! per edit it is asked for, each named where it is drawn ([`forms`]).
//!
//! # What is refused, never cut wrong (trap 5)
//!
//! A page is refused by name — its content and its `/Redact` annotations left as the file
//! wrote them — where removal cannot be proven to leave no trace, and every refusal is an owed
//! capability or a reading of the clause rather than a silence:
//!
//! - any **code count** the interpreter does not confirm;
//! - a **Type 3 glyph** whose description the interpreter could not draw in full, where its marks
//!   are measured rather than declared;
//! - a **composite font** whose `CMap` does not resolve;
//! - a **calculator function** (§7.10.5) serving colours only the region carries, since a program
//!   is not divided by the values it serves, and a **shading** whose data meeting the region do
//!   not read — a mesh stream or a sample table that does not decode as its clause states;
//! - a **codec image whose decode is not its own grid** — a `JPXDecode` codestream larger than
//!   the operator's budget, which is refused rather than decoded at a reduced resolution level
//!   (§7.4.9 NOTE 3) that would resample the image outside the region too, and a filter that
//!   stopped short of the grid — or whose samples no image this removal writes can hold: a
//!   `JPXDecode` component deeper than Table 87's sixteen bits;
//! - a **codec image whose dictionary contradicts its filter's output** — components its colour
//!   space does not take, an image mask whose samples are not one bit, a grid the codestream does
//!   not state — which has no one image to carry;
//! - an **inline image** behind a filter §8.9.7 forbids inline, or whose colour space holds a
//!   reference no resource name in force reaches;
//! - a **painted path** whose marks the cut cannot take exactly: one whose path object another
//!   operator interrupted, and one whose surviving coordinates are too large for
//!   [`paths::Cut::margin_holds`] to prove the cut edge cannot round into the region;
//! - a **form `XObject`** (§8.10) whose content does not decode, one that draws itself, one the
//!   page's resources name directly rather than by reference, so no object can be replaced, and
//!   a page asking for more copies of its forms than [`forms::MAX_COPIES`].
//!
//! An encrypted document is refused outright where no protection is stated for the output,
//! because the redaction would otherwise be written in the clear (ADR 1162).
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

mod forms;
mod located;
mod mesh;
mod paths;
mod shading;
mod type3;

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::sync::Arc;

use pdf_model::colour::ColourSpace;
use pdf_model::content::{Interpretation, base_transform, interpret};
use pdf_model::image::{ImageError, ORDINARY_JPX_SAMPLES};
use pdf_model::{Page, Pages};
use pdf_render::Transform;
use pdf_render::geom::Point;
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
    /// The most samples — pixels times channels — a `JPXDecode` image may be decoded to at full
    /// resolution, where the operator states more than the viewer's own budget
    /// (`pdf_model::image::ORDINARY_JPX_SAMPLES`); `None` for that budget.
    ///
    /// A viewer may draw a codestream over its budget at a reduced resolution level (§7.4.9
    /// NOTE 3), and a redaction cannot, because it writes the image back: a reduced grid would
    /// resample every sample outside the region. So the redaction decodes at full resolution or
    /// refuses by name, and how much memory and time that may take is a decision about the
    /// machine the verb runs on, which is its operator's (`--image-samples`, ADR 1333).
    pub image_samples: Option<u64>,
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
        let Some((page, page_id)) = pages
            .get(index)
            .and_then(|page| page.id.map(|id| (page, id)))
        else {
            continue;
        };
        let regions = redactions(document, &page);
        if regions.is_empty() {
            continue;
        }
        let image_samples = plan.image_samples.unwrap_or(ORDINARY_JPX_SAMPLES);
        match plan_page(document, &page, &regions, (&counts, image_samples)) {
            Ok(edit) => {
                // §12.5.6.23: destroy the image data. Grouped and cleared per page, so a shared
                // image's destroyed samples belong to this page's copy alone.
                let images = clear_images(document, edit.clears)?;
                annotations = annotations.saturating_add(regions.len());
                glyphs = glyphs.saturating_add(edit.removed);
                inline_images = inline_images.saturating_add(edit.inline_images);
                cut_paths = cut_paths.saturating_add(edit.paths);
                let pictures = images.iter().filter(|image| image.role == Role::Picture);
                cleared_images = cleared_images.saturating_add(pictures.count());
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
                    states: edit.states,
                    names: edit.names,
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
    /// Whether this is a picture the page draws or the §8.9.6.3 / §11.6.5.2 mask of one, so the
    /// report counts the pictures and not the masks that travel with them.
    role: Role,
    /// The cleared samples, re-encoded `FlateDecode`.
    encoded: Vec<u8>,
    /// How [`build_cleared_image`] describes the samples: the source dictionary carried, carried
    /// with the codec's depth restated, or a `JPXDecode` image's own description.
    written: Described,
    /// §7.4.9's opacity channel, cleared under the same placements and re-encoded `FlateDecode`,
    /// where a `JPXDecode` image stated a non-zero `/SMaskInData`: Table 87 has the processor
    /// "create a soft-mask image from the information", and this is that image, written.
    opacity: Option<Vec<u8>>,
}

/// Whether a cleared image is a picture or a mask that belongs to one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    /// An image the content draws with `Do`.
    Picture,
    /// A §11.6.5.2 soft-mask image named by a picture's `/SMask`.
    SoftMask,
    /// A §8.9.6.3 explicit mask named by a picture's `/Mask`.
    ExplicitMask,
}

/// Where a cleared image's samples come from before the region is cleared.
///
/// Every codec's output is the image's sample data, which Table 87 says of `/BitsPerComponent`:
/// it "shall be consistent with the size of the data samples that the filter delivers". So an
/// image behind a codec is carried in the domain its dictionary describes, exactly as a
/// codec-free one is, and only the encoding changes (ADR 1371).
#[derive(Clone)]
enum Source {
    /// A codec-free image, whose packed samples are read from the stream at clearing time.
    Packed,
    /// The samples a `DCTDecode`, `CCITTFaxDecode` or `JBIG2Decode` filter delivered, or the
    /// one-bit samples of a `JPXDecode` image mask, packed as the dictionary describes them
    /// ([`pdf_model::image::filter_samples`]).
    Filtered(Arc<[u8]>),
    /// A `JPXDecode` image's own integers, whose depth and colour space the codec decided and
    /// the written dictionary must state ([`OwnRaster`], ADRs 1333 and 1371).
    Own(Decoded),
}

/// How a cleared image's dictionary is written, decided by its [`Source`].
#[derive(Clone)]
enum Described {
    /// The source dictionary, with only the encoding replaced.
    Carried,
    /// The source dictionary with the codec's encoding replaced and `/BitsPerComponent` stated
    /// as the depth the filter delivered.
    Filtered {
        /// Bits per component the filter delivered.
        bits: usize,
    },
    /// A `JPXDecode` image's own samples, described by [`own_dictionary`].
    Own {
        /// The grid the samples are written on.
        layout: ImageLayout,
        /// Their description.
        own: Arc<OwnRaster>,
    },
}

/// How a `JPXDecode` image's own samples are described once they are written under
/// `FlateDecode`: what [`Source::Own`] carries beside its samples (ADRs 1333, 1371).
///
/// The samples are the decoder's integers unchanged, so every entry that described them in the
/// source still does — the colour space the dictionary stated, `/Intent`, a colour-key `/Mask`,
/// a mask's `/Matte` — and is carried. What is restated is what the codec had decided and a
/// `FlateDecode` stream must say: the depth, and the colour space where it was the codestream's.
#[derive(Clone)]
struct OwnRaster {
    /// The colour space the samples are in.
    space: pdf_model::image::JpxSpace,
    /// The depth the samples are written at: 8 where every component is eight bits or fewer, 16
    /// above.
    bits: usize,
    /// The `/Decode` to state: the pairs the source's samples were read through, each component's
    /// own depth widened to [`Self::bits`] keeping its integers and moving the pair's far end so
    /// that §8.9.5.2's map gives every integer the value it had ([`widened_decode`]).
    decode: Vec<f32>,
    /// The same for the opacity channel's soft-mask image, which Table 143 lets state a
    /// `/Decode` of its own.
    opacity_decode: Vec<f32>,
    /// Whether `/SMaskInData` 2 multiplied the colour by the opacity, which the written soft-mask
    /// image then states as §11.6.5.2's `/Matte` of the space's zero.
    premultiplied: bool,
    /// Each component's own depth, which its integers are still at.
    depths: Vec<u8>,
    /// The `/Decode` pair a reader maps each component through at its own depth.
    pairs: Vec<(f32, f32)>,
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
        let ClearedSamples {
            encoded,
            written,
            opacity,
        } = cleared_image_samples(document, stream, &placements).map_err(|detail| {
            Refusal::Assembly(format!("§12.5.6.23: image object {}: {detail}", id.number))
        })?;
        let private = placements.iter().any(|clear| clear.private);
        let role = placements.first().map_or(Role::Picture, |clear| clear.role);
        cleared.push(ClearedImage {
            id,
            private,
            role,
            encoded,
            written,
            opacity,
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
    /// Graphics state dictionaries the edited content names that the page's resources gain.
    states: Vec<(Vec<u8>, Dictionary)>,
    /// The new objects the edited content names.
    names: Names,
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
    /// Graphics state dictionaries the edited page content names that its resources must gain.
    states: Vec<(Vec<u8>, Dictionary)>,
    /// The new objects the edited page content names (ADR 1363).
    names: Names,
}

/// What a page's edited streams name that the producer's resources do not hold (ADR 1363).
struct Names {
    /// Form copies the page content names, which its resources must gain.
    forms: AddedForms,
    /// Shading and pattern entries of the page's resources whose located data were destroyed.
    overrides: Overrides,
    /// Every destruction of located shading data the page's streams name, by what it replaces.
    located: HashMap<located::Key, located::Built>,
    /// Graphics states the page content names that establish a soft mask on a group's copy.
    masks: AddedMasks,
}

/// Every placement of a form the walk reached, with the edited stream it asked for.
type FormOutcomes = Vec<(ObjectId, Option<Vec<u8>>)>;

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
    /// Whether this placement is a picture's or the mask of one, which the report counts apart.
    role: Role,
    /// Where the samples come from: the stream itself for a codec-free image, and for one behind
    /// a codec the samples decoded at planning time, because a codec's bytes are not samples and
    /// the destruction cannot address the packed grid the source stream carries.
    source: Source,
    /// The sample a cleared place takes, one integer per component, where it is not the
    /// domain's zero: a picture pre-blended with §11.6.5.2's matte colour takes the matte
    /// itself, which is Table 144's `c′ = m + α × (c − m)` at the zero opacity its cleared
    /// soft mask states there (ADR 1333).
    fill: Option<Vec<u32>>,
}

/// A `JPXDecode` image's own samples, with §7.4.9's opacity channel beside them where the
/// codestream carried one.
#[derive(Clone)]
struct Decoded {
    /// The decoder's integers, laid out as the placement's [`ImageLayout`] describes.
    samples: Arc<[u8]>,
    /// One opacity sample per pixel, on the same grid, where `/SMaskInData` is non-zero, at
    /// [`OwnRaster::bits`].
    opacity: Option<Arc<[u8]>>,
    /// How the samples are described.
    own: Arc<OwnRaster>,
}

/// What [`Walk::stated_masks`] found in a picture's dictionary.
#[derive(Default)]
struct StatedMasks {
    /// The mask streams, each to be cleared as an image of its own.
    stated: Vec<StatedMask>,
    /// The soft mask's §11.6.5.2 `/Matte`, where it states one.
    matte: Option<Vec<f32>>,
}

/// A mask an image dictionary names by reference, to be cleared as an image of its own.
struct StatedMask {
    /// The mask's object.
    id: ObjectId,
    /// Which of the two masks it is.
    role: Role,
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
    /// The resource dictionary the form's names were resolved in — its own, or the page's where
    /// it states none — which is what it is written with when [`Self::states`] adds to it.
    resources: Dictionary,
    /// Graphics state dictionaries the edited stream names that its resources must gain.
    states: Vec<(Vec<u8>, Dictionary)>,
    /// Copies of forms the edited stream draws under new names, which its resources must gain.
    forms: AddedForms,
    /// Shading and pattern entries of its resources whose located data the removal destroyed.
    overrides: Overrides,
    /// Graphics states its edited stream names that establish a soft mask on a group's copy.
    masks: AddedMasks,
    /// Whether this is one placement's own copy of a form drawn more than once, written into a
    /// slot of its own and named by the placements that draw it rather than replacing the form or
    /// standing in for it on the page (ADR 1363).
    placement_copy: bool,
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
    (counts, image_samples): (&HashMap<u32, usize>, u64),
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
    let mut walk = Walk::new(
        document,
        page,
        &interpretation,
        region_boxes.clone(),
        counts,
    );
    walk.image_samples = image_samples;
    let (edit, outcomes) = walk.run(&content)?;
    // A form the region meets differently at two of its placements is one stream asked to be
    // written two ways; the first walk finds which, and a second writes each placement's own
    // (ADR 1363).
    let split = forms::split(&outcomes)?;
    if split.is_empty() {
        return Ok(edit);
    }
    let mut walk = Walk::new(document, page, &interpretation, region_boxes, counts);
    walk.image_samples = image_samples;
    walk.split = split;
    Ok(walk.run(&content)?.0)
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

/// How a font's codes are read from a show string's bytes, and where each one marks the page.
///
/// The code is the unit of removal, so the walk has to split a string exactly where a reader
/// does; the cut is held to the interpreter by the code count as well (trap 13). ADR 1351.
#[derive(Clone)]
enum Codes {
    /// A simple font with a glyph program (§9.6): one byte, one code, marking within the box the
    /// interpreter placed.
    Simple,
    /// A Type 3 font (§9.6.4): one byte, one code — a simple font — whose glyph is a content
    /// stream that may mark outside the advance box, so each code is tested against the box its
    /// own description declares ([`Type3Marks`]).
    Type3(Arc<Type3Marks>),
    /// A composite font (§9.7): each code as long as the `CMap`'s codespace ranges make it, which
    /// is how many bytes §9.7.6.2 extracts from the string for each successive code.
    Composite(Arc<CompositeCodes>),
}

/// What the walk needs of a composite font: how its codes are delimited, and — in writing mode 1
/// — how far each one moves the pen.
struct CompositeCodes {
    /// The `CMap` the font loader resolves ([`pdf_font::composite_cmap`]).
    cmap: pdf_font::cmap::CMap,
    /// §9.7.4.3's vertical displacements where the `CMap`'s `/WMode` is 1, `None` in writing
    /// mode 0.
    ///
    /// In writing mode 1 §9.4.4's displacement is `ty`, computed from the glyph's `w1`, which the
    /// placed quadrilateral does not carry: the quad is the glyph's horizontal box moved back by
    /// its position vector. So a removed code's advance is read from `/W2` and `/DW2` — the
    /// numbers the interpreter moved the pen by — rather than from the quad (ADR 1363).
    vertical: Option<pdf_font::VerticalDisplacements>,
}

impl CompositeCodes {
    /// The CID a code selects, as the font loader selects it: the `CMap`'s mapping, else its
    /// `notdef` mapping, else CID 0 (§9.7.6.3).
    fn cid(&self, code: &[u8]) -> u32 {
        let code = self.cmap.next_code(code);
        self.cmap
            .cid(code)
            .or_else(|| self.cmap.notdef_cid(code))
            .unwrap_or(0)
    }
}

/// What the walk needs of a Type 3 font to say where one of its glyphs marks the page.
///
/// §9.6.4 states two boxes, and between them every glyph has one: Table 111's `d1` operands are
/// the glyph's bounding box, of which it says "[t]he declared bounding box shall be correct - in
/// other words, sufficiently large to enclose the entire glyph", and Table 110's `/FontBBox` is "the smallest rectangle
/// enclosing all marks that would result if all of the glyphs of the font were placed with their
/// origins coincident". A `d0` glyph declares no box of its own, so the font's is the one that
/// holds — unless all four of its numbers are zero, where the clause withdraws it ("a PDF
/// processor shall make no assumptions about glyph sizes based on the font bounding box") and the
/// glyph's own marks are measured instead ([`type3`]). ADRs 1351, 1363.
struct Type3Marks {
    /// The font, for §9.6.4's steps a) and b) and its `/FontMatrix`.
    font: pdf_model::type3::Type3Font,
    /// `/FontBBox` in glyph space, or `None` where every element is zero.
    font_box: Option<[f64; 4]>,
}

impl Type3Marks {
    /// The glyph-space box a code's glyph description marks within, `Ok(None)` where the code
    /// reaches no description (§9.6.4 step b): "no glyph shall be painted") or the description
    /// paints nothing, or a refusal where its marks cannot be measured.
    ///
    /// The declared box where there is one — `d1`'s, else `/FontBBox` — and for a `d0` glyph under
    /// an all-zero `/FontBBox` the box its own marks are measured to fill when the description is
    /// run ([`type3::measured_marks`], ADR 1363). `stroke` is the line width and miter limit in
    /// force at the text-showing operator, which the description inherits.
    fn glyph_box(
        &self,
        document: &Document,
        page: &Page,
        stroke: (f64, f64),
        code: u8,
    ) -> Result<Option<[f64; 4]>, String> {
        let Some(stream) = self.font.glyph(document, u32::from(code)) else {
            return Ok(None);
        };
        let declared = document
            .decoded_stream_data(&stream)
            .and_then(|data| declared_glyph_box(&data));
        match declared.or(self.font_box) {
            Some(declared) => Ok(Some(declared)),
            None => type3::measured_marks(document, page, &self.font, &stream, stroke),
        }
    }
}

/// The bounding box a Type 3 glyph description's `d1` declares (Table 111), or `None` where the
/// description begins with anything else.
///
/// Table 110 requires the stream to "include as its first operator either d0 or d1", so only the
/// first operator is read.
fn declared_glyph_box(data: &[u8]) -> Option<[f64; 4]> {
    let mut lexer = Lexer::at(data, 0);
    let mut operands: Vec<f64> = Vec::new();
    loop {
        lexer.skip_whitespace();
        match lexer.next_token()? {
            Token::Integer(value) => {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a glyph-space coordinate is far inside f64"
                )]
                operands.push(value as f64);
            }
            Token::Real(value) => operands.push(value),
            Token::Keyword(keyword) if keyword == b"d1" => {
                let [.., llx, lly, urx, ury] = operands.as_slice() else {
                    return None;
                };
                return Some([llx.min(*urx), lly.min(*ury), llx.max(*urx), lly.max(*ury)]);
            }
            _ => return None,
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
    /// The producer's own bytes for the whole path object, up to its painting operator.
    ///
    /// What the boundary is re-stated from when the marks are cut away from under it, so that not
    /// one coordinate of it is re-derived (§8.5.4, ADR 1248). Taken at the painting operator
    /// because that is where the path is whole, and only for a path [`Self::clips`] is set on.
    clip_bytes: Option<Vec<u8>>,
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
            clip_bytes: None,
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
    text: TextState,
    added_states: Vec<(Vec<u8>, Dictionary)>,
    added_forms: AddedForms,
    added_overrides: Overrides,
    added_masks: AddedMasks,
    stream_base: Transform,
}

/// Byte-range edits to one content stream: start, end, and the bytes that replace the range.
type Edits = Vec<(usize, usize, Vec<u8>)>;

/// Graphics state parameter dictionaries a stream's edits name, each under the resource name it
/// is added with.
type AddedStates = Vec<(Vec<u8>, Dictionary)>;

/// Form `XObject` copies a stream's edits name, each under the resource name it is added with and
/// as an index into the page's recorded form edits (ADR 1363).
type AddedForms = Vec<(Vec<u8>, usize)>;

/// Resource entries a stream's resources give a replacement for: the category and name, and which
/// destruction replaces it ([`located`], ADR 1363).
type Overrides = Vec<(located::Category, Vec<u8>, located::Key)>;

/// Graphics state parameter dictionaries a stream's edits name that establish a soft mask whose
/// group is a copy of its own: the new name, the producer's `/ExtGState` entry it restates, and
/// the index of the group's copy among the page's recorded form edits (ADR 1363).
type AddedMasks = Vec<(Vec<u8>, Object, usize)>;

/// How the walk came to be running a form's content stream.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Entered {
    /// Drawn with `Do` (§8.10.1), inheriting the graphics state: where the operator's name
    /// operand begins and where the operator ends, which a placement given its own copy of the
    /// form rewrites (ADR 1363).
    Drawn {
        /// Where the `Do`'s name operand begins in the enclosing stream.
        name_start: usize,
        /// Where the `Do` operator ends.
        do_end: usize,
    },
    /// Evaluated as a soft mask's transparency group (§11.6.5.1), from the initial state: where
    /// the `gs` that established it begins its name operand and where it ends, which a placement
    /// given its own copy of the group rewrites (ADR 1363).
    MaskGroup {
        /// Where the `gs`'s name operand begins in the enclosing stream.
        name_start: usize,
        /// Where the `gs` operator ends.
        gs_end: usize,
    },
}

/// §9.3's text state as far as the walk tracks it, with the text matrix's linear part.
#[derive(Clone)]
struct TextState {
    linear: Transform,
    size: f32,
    horizontal: f32,
    char_spacing: f32,
    word_spacing: f32,
    codes: Option<Codes>,
}

impl Default for TextState {
    /// The initial values §9.3 gives the parameters of Table 102: no font, a size of zero,
    /// horizontal scaling of 100 per cent, no spacing.
    fn default() -> Self {
        Self {
            linear: Transform::IDENTITY,
            size: 0.0,
            horizontal: 1.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            codes: None,
        }
    }
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
    /// Table 51's overprint parameter for stroking, set by Table 58's `/OP`.
    stroke_overprint: bool,
    /// The same for every other painting operation, set by `/op` — or by `/OP` where the same
    /// dictionary states no `/op`.
    fill_overprint: bool,
    /// The pattern the non-stroking colour names, where `scn` named one (§8.7.2).
    fill_pattern: Option<Vec<u8>>,
    /// The pattern the stroking colour names, where `SCN` named one.
    stroke_pattern: Option<Vec<u8>>,
    /// A box in the display list's space enclosing §8.5.4's current clipping path, or `None`
    /// while nothing has clipped the page.
    ///
    /// Each `W` or `W*` narrows it to the box of the path it intersects the clip with, which
    /// contains that path, so the box only ever over-states the clip — the direction that tests
    /// more marks against the region, never fewer. What `sh` paints is bounded by it (ADR 1351).
    clip: Option<[f32; 4]>,
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
            stroke_overprint: false,
            fill_overprint: false,
            fill_pattern: None,
            stroke_pattern: None,
            clip: None,
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

    /// The operators that give a fill this state's **stroking** colour.
    ///
    /// Table 74 pairs each stroking operator with the non-stroking one that sets the same thing,
    /// and the operands written are the file's own bytes rather than numbers this program
    /// re-formatted. A `CS` with no `SC` or `SCN` after it is enough on its own: §8.6.8 has that
    /// operator "set the colour to its initial value" for the space it names, and `cs` sets the
    /// same initial value for the same space.
    ///
    /// Where no `CS` has run, the stroking space is Table 51's initial one, `DeviceGray` for both
    /// colour spaces, which the non-stroking space need not still be, so it
    /// is stated before an `SC` or `SCN` is replayed. Where no stroking colour operator has run at
    /// all, the colour is that space's initial black, which `0 g` states (ADR 1351).
    fn non_stroking_colour(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if let Some(space) = &self.space {
            out.extend_from_slice(space);
            out.extend_from_slice(b" cs\n");
        } else if self.value.is_none() {
            out.extend_from_slice(b"0 g\n");
        } else if matches!(&self.value, Some((_, "sc" | "scn"))) {
            out.extend_from_slice(b"/DeviceGray cs\n");
        }
        if let Some((operands, operator)) = &self.value {
            out.extend_from_slice(operands);
            out.push(b' ');
            out.extend_from_slice(operator.as_bytes());
            out.push(b'\n');
        }
        out
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

/// Whether this keyword is one of §8.5.3's path-painting operators (Table 59, Table 60's `n`).
///
/// What terminates a path object, which is where the path is whole: §8.5.4 lets the clipping
/// operator appear anywhere before it, so this is the position the boundary's bytes are taken at.
fn is_painting_operator(keyword: &[u8]) -> bool {
    matches!(
        keyword,
        b"f" | b"F" | b"f*" | b"S" | b"s" | b"B" | b"B*" | b"b" | b"b*" | b"n"
    )
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

/// How far, in the display list's units, the outline a stroke is cut as may depart from the arc
/// §8.4.3.3's round cap, §8.4.3.4's round join or the offset of a §8.5.2.2 curve describes.
///
/// A path can state no circle — "[c]urved path segments shall be specified as cubic Bézier
/// curves" — so the outline written back for a round cap is cubics fitted to it, and this is the
/// fitting's bound. A hundredth of [`paths::REGION_PAD`]: at a magnification where the widening
/// the cut already takes past the quad covers one device pixel, the approximation covers a
/// hundredth of one, below the eight-bit step any backend resolves, and it is the same order as
/// the §7.3.3 single-precision reading [`paths::Cut::margin_holds`] already allows a processor
/// to hold the producer's own coordinates at. It decides fidelity only: the cut is exact on the
/// fitted outline, so nothing it writes can lie inside the region whatever this is. Carried into
/// the path's own user space by the mapping's norm, since `kurbo::stroke` works there. A straight
/// segment's outline is closed form and this bounds nothing on it. ADRs 1236, 1324.
const ARC_TOLERANCE: f64 = paths::REGION_PAD / 100.0;

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

/// A path split into its subpaths as the producer stated them, open or closed, for a cut taken
/// along the path rather than over the area it encloses.
fn split_open(path: &kurbo::BezPath) -> Vec<paths::SubPath> {
    let mut out: Vec<paths::SubPath> = Vec::new();
    for element in path.elements() {
        if matches!(element, kurbo::PathEl::MoveTo(_)) {
            out.push(kurbo::BezPath::new());
        }
        if let Some(current) = out.last_mut() {
            current.push(*element);
        }
    }
    out
}

/// Whether this path object's own bytes are a cut's to replace, or the refusal by name.
///
/// Two questions, neither about the geometry: whether the byte range the replacement occupies is
/// the path's alone (§8.2's path object), and whether the path has any area in device space at
/// all (§8.3.4).
///
/// **A path that is also §8.5.4's clipping boundary is not one of them, since ADR 1248.** The
/// boundary is not a mark and a cut does not have to move it: the clause separates the two acts
/// in time — "[a]lthough the clipping path operator appears before the painting operator, it
/// shall not alter the clipping path at the point where it appears … After the path has been
/// painted, the clipping path in the graphics state shall be set to the intersection of the
/// current clipping path and the newly constructed path" — so the marks are painted under the
/// clip that was already in force, and the boundary is set afterwards. [`Walk::paint_path`]
/// writes exactly that order back: the cut marks first, then the producer's own bytes for the
/// construction and the clipping operator followed by `n`, which §8.5.4 says "shall cause no
/// marks to be placed on the page, but can be used with a clipping path operator to establish a
/// new clipping path".
fn admits_a_cut(path: &PathObject) -> Result<(), String> {
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
    /// How the current font's codes are split and where they mark, from the last `Tf`.
    codes: Option<Codes>,
    /// Graphics state dictionaries this stream's edits name and its resources must gain: the
    /// stroking alpha and overprint a surviving outline is filled under ([`Walk::restate_stroke`]).
    added_states: Vec<(Vec<u8>, Dictionary)>,
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
    /// The most samples a `JPXDecode` image is decoded to at full resolution
    /// ([`RedactPlan::image_samples`]).
    image_samples: u64,
    /// Every placement of a form `XObject` that is an object, in the order the walk reached it,
    /// with the edited stream it asked for or `None` where the region left it alone — what the
    /// first walk of a page learns, and a form whose placements disagree is split by (ADR 1363).
    form_outcomes: FormOutcomes,
    /// How each placement of a form whose placements disagree is written, by the order the walk
    /// reaches them: empty on the first walk of a page.
    split: HashMap<ObjectId, Vec<forms::Slot>>,
    /// How many placements of each split form this walk has reached.
    placements: HashMap<ObjectId, usize>,
    /// Which recorded form edit each copy of a split form became.
    copies: HashMap<(ObjectId, usize), usize>,
    /// Form copies this stream's edits name and its resources must gain.
    added_forms: AddedForms,
    /// Every shading or pattern entry whose located data some placement asked destroyed.
    located: Vec<(located::Key, located::Request)>,
    /// The entries of this stream's resources those destructions replace.
    added_overrides: Overrides,
    /// Graphics states naming a copy of a soft mask's group, which this stream's resources gain.
    added_masks: AddedMasks,
    /// The `/ExtGState` entry the last `gs` named, which a soft mask's group copy is restated from.
    current_state: Option<Object>,
    /// The transform the stream now running began under — the page's base transform, or a form's
    /// matrix at its `Do` — which a pattern's `/Matrix` is stated against (§8.7.3.1).
    stream_base: Transform,
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
            codes: None,
            added_states: Vec::new(),
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
            image_samples: ORDINARY_JPX_SAMPLES,
            form_outcomes: Vec::new(),
            split: HashMap::new(),
            placements: HashMap::new(),
            copies: HashMap::new(),
            added_forms: Vec::new(),
            located: Vec::new(),
            added_overrides: Vec::new(),
            added_masks: Vec::new(),
            current_state: None,
            stream_base: base_transform(page),
        }
    }

    /// Walks the page's content stream, returning the edited bytes and every form placement's
    /// outcome, or a refusal by name.
    fn run(mut self, content: &[u8]) -> Result<(PageEdit, FormOutcomes), String> {
        self.run_stream(content)?;
        if self.code_index != self.quads.len() {
            return Err(format!(
                "the content walk found {} code(s) where the interpreter placed {}; the page is \
                 refused rather than cut against a walk the interpreter does not confirm",
                self.code_index,
                self.quads.len()
            ));
        }
        let located = self.finish_located()?;
        Ok((
            PageEdit {
                content: apply_edits(content, self.edits),
                removed: self.removed,
                clears: self.clears,
                inline_images: self.inline_cleared,
                paths: self.paths_cut,
                forms: self.form_edits,
                states: self.added_states,
                names: Names {
                    forms: self.added_forms,
                    overrides: self.added_overrides,
                    located,
                    masks: self.added_masks,
                },
            },
            self.form_outcomes,
        ))
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
                        // §8.5.4's clipping operator makes the path a boundary as well as a
                        // mark, and the boundary has to survive the mark being cut out from
                        // under it. The producer's own bytes are what it is re-stated from, so
                        // that no coordinate of it is re-derived (ADR 1248). Taken at the
                        // painting operator rather than at the clipping one, because the clause
                        // only *permits* the order — "may appear after the last path
                        // construction operator" — and the boundary is "the newly constructed
                        // path", which is whole only here.
                        if is_painting_operator(keyword)
                            && let Some(path) = self.path.as_mut()
                            && path.clips
                            && let Some(bytes) = content.get(path.start..start)
                        {
                            path.clip_bytes = Some(bytes.to_vec());
                        }
                        if is_stroking_colour_operator(keyword)
                            && let Some((_, from)) = operands.first()
                        {
                            let bytes = content.get(*from..start).unwrap_or_default();
                            let bytes = bytes.trim_ascii_end();
                            if !bytes.is_empty() {
                                self.graphics.record_colour(keyword, bytes.to_vec());
                            }
                        }
                        if keyword == b"sh" {
                            if let Some(path) = self.path.as_mut() {
                                path.interrupted = true;
                            }
                            self.shade(content, &operands, start)?;
                        } else {
                            self.operator(keyword, start, &operands)?;
                        }
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
            // §8.6.8's colour operators, as far as whether the colour is a pattern: a pattern is
            // named by the operand of `scn` or `SCN`, and every other colour operator replaces it.
            b"scn" => self.graphics.fill_pattern = pattern_name(operands),
            b"cs" | b"sc" | b"g" | b"rg" | b"k" => self.graphics.fill_pattern = None,
            b"SCN" => self.graphics.stroke_pattern = pattern_name(operands),
            b"CS" | b"SC" | b"G" | b"RG" | b"K" => self.graphics.stroke_pattern = None,
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
            b"n" => {
                if let Some(path) = self.path.take() {
                    self.narrow_clip(&path);
                }
            }
            b"Do" => self.do_xobject(operands, keyword_start)?,
            b"gs" => self.ext_gstate(operands, keyword_start)?,
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
        self.codes = match name {
            Some(name) => Some(self.font_codes(&name)?),
            None => None,
        };
        Ok(())
    }

    /// How the named font's codes are read, or a refusal for a font the walk will not cut.
    fn font_codes(&self, name: &[u8]) -> Result<Codes, String> {
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
            b"Type1" | b"TrueType" | b"MMType1" => Ok(Codes::Simple),
            b"Type3" => self.type3_codes(font, name),
            b"Type0" => self.composite_codes(font, name),
            other => Err(format!(
                "the content shows a /{} font, whose code width this walk does not settle; the \
                 page is refused",
                String::from_utf8_lossy(other)
            )),
        }
    }

    /// A Type 3 font's codes (§9.6.4), with what locates each glyph's marks.
    fn type3_codes(&self, font: &Dictionary, name: &[u8]) -> Result<Codes, String> {
        let shown = String::from_utf8_lossy(name);
        let read =
            pdf_model::type3::Type3Font::read(self.document, font, &shown).map_err(|error| {
                format!(
                    "§9.6.4: the Type 3 font /{shown} cannot be read ({error}); the page is refused"
                )
            })?;
        let font_box = numbers(self.document, font, "FontBBox")
            .and_then(|values| match values.as_slice() {
                [llx, lly, urx, ury] => Some([
                    f64::from(*llx),
                    f64::from(*lly),
                    f64::from(*urx),
                    f64::from(*ury),
                ]),
                _ => None,
            })
            .filter(|values| values.iter().any(|value| *value != 0.0))
            .map(|[llx, lly, urx, ury]| [llx.min(urx), lly.min(ury), llx.max(urx), lly.max(ury)]);
        Ok(Codes::Type3(Arc::new(Type3Marks {
            font: read,
            font_box,
        })))
    }

    /// A Type 0 font's codes, as its `CMap`'s codespace ranges delimit them (§9.7.6.2).
    ///
    /// The `CMap` is resolved by the same function the font loader uses, so a predefined name, an
    /// embedded stream and its `/UseCMap` chain are all read one way. A vertical `CMap` carries the
    /// descendant's §9.7.4.3 displacements with it, because in writing mode 1 the gap a removal
    /// leaves is §9.4.4's `ty` ([`CompositeCodes::vertical`]).
    fn composite_codes(&self, font: &Dictionary, name: &[u8]) -> Result<Codes, String> {
        let shown = String::from_utf8_lossy(name);
        let cmap = pdf_font::composite_cmap(self.document, font, &shown).map_err(|error| {
            format!(
                "§9.7.6.2: the composite font /{shown}'s CMap cannot be resolved ({error}), so \
                 how many bytes each of its codes takes is unknown; the page is refused"
            )
        })?;
        let vertical = if cmap.wmode() == 0 {
            None
        } else {
            Some(
                pdf_font::VerticalDisplacements::of_composite(self.document, font).ok_or_else(
                    || {
                        format!(
                            "§9.7.4.3: the composite font /{shown} writes vertically and names no \
                             descendant CIDFont to state its vertical displacements; the page is \
                             refused"
                        )
                    },
                )?,
            )
        };
        Ok(Codes::Composite(Arc::new(CompositeCodes {
            cmap,
            vertical,
        })))
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
        let codes = self.require_codes()?;
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
            return if self.count_only(elements, &codes)? {
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
                    any |= self.rewrite_string(bytes, &codes, &mut out)?;
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
        codes: &Codes,
        out: &mut String,
    ) -> Result<bool, String> {
        let mut kept: Vec<u8> = Vec::new();
        let mut removed_first: Option<[f32; 8]> = None;
        let mut removed_last: [f32; 8] = [0.0; 8];
        // The removed run's §9.7.4.3 vertical displacement, in thousandths, in writing mode 1.
        let mut removed_w1y = 0.0f32;
        let vertical = match codes {
            Codes::Composite(composite) => composite.vertical.as_ref().map(|_| composite),
            Codes::Simple | Codes::Type3(_) => None,
        };
        let mut any = false;
        for (start, len) in split_codes(bytes, codes) {
            let quad = self.quad_at(self.code_index)?;
            let code = bytes
                .get(start..start.saturating_add(len))
                .unwrap_or_default();
            let marks = self.code_box(&quad, codes, code)?;
            let meets = self.regions.iter().any(|region| overlaps(*region, marks));
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
                if let Some(composite) = vertical
                    && let Some(displacements) = composite.vertical.as_ref()
                {
                    removed_w1y += displacements.w1y(composite.cid(code));
                }
                self.removed = self.removed.saturating_add(1);
                any = true;
            } else {
                if let Some(first) = removed_first.take() {
                    self.write_run_gap(out, (first, removed_last), vertical.map(|_| removed_w1y));
                    removed_w1y = 0.0;
                }
                kept.extend_from_slice(&bytes[start..start.saturating_add(len)]);
            }
        }
        if let Some(first) = removed_first.take() {
            self.write_run_gap(out, (first, removed_last), vertical.map(|_| removed_w1y));
        }
        if !kept.is_empty() {
            write_pdf_string(out, &kept);
        }
        Ok(any)
    }

    /// The `TJ` adjustment restoring a removed run's advance, in the writing mode the font writes
    /// in: `vertical_w1y` is the run's summed §9.7.4.3 vertical displacement in writing mode 1,
    /// `None` in writing mode 0.
    ///
    /// §9.4.4 computes `ty = (w1 − Tj/1000) × Tfs + Tc + Tw` in vertical writing, and the `TJ`
    /// number is "subtracted from the current horizontal or vertical coordinate, depending on the
    /// writing mode" (§9.4.3). With `Tc` and `Tw` zero — spacing in force refuses the page before
    /// this — the number that moves the pen by the removed glyphs' own `w1` is minus their sum,
    /// already in thousandths, and `Th` takes no part: it scales `tx` alone (ADR 1363).
    fn write_run_gap(
        &self,
        out: &mut String,
        run: ([f32; 8], [f32; 8]),
        vertical_w1y: Option<f32>,
    ) {
        match vertical_w1y {
            Some(w1y) => write_number(out, f64::from(-w1y)),
            None => self.write_gap(out, run.0, run.1),
        }
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
    fn count_only(&mut self, elements: &[ArrayElement], codes: &Codes) -> Result<bool, String> {
        let mut met = false;
        for element in elements {
            if let ArrayElement::Str(bytes) = element {
                met |= self.codes_meet_region(bytes, codes)?;
            }
        }
        Ok(met)
    }

    /// Tests every code of a string against the region without editing (the refused operators).
    fn string_meets_region(&mut self, bytes: &[u8]) -> Result<bool, String> {
        let codes = self.require_codes()?;
        self.codes_meet_region(bytes, &codes)
    }

    /// Consumes each code of a string, advancing the index, and answers whether any meets the
    /// region.
    fn codes_meet_region(&mut self, bytes: &[u8], codes: &Codes) -> Result<bool, String> {
        let mut met = false;
        for (start, len) in split_codes(bytes, codes) {
            let quad = self.quad_at(self.code_index)?;
            self.code_index = self.code_index.saturating_add(1);
            let code = bytes
                .get(start..start.saturating_add(len))
                .unwrap_or_default();
            let marks = self.code_box(&quad, codes, code)?;
            met |= self.regions.iter().any(|region| overlaps(*region, marks));
        }
        Ok(met)
    }

    /// The box in the display list's space a code is tested against: the quadrilateral the
    /// interpreter placed it in, widened for a Type 3 glyph to the box its description declares.
    ///
    /// A Type 3 glyph's description is a content stream that may mark anywhere the font's glyph
    /// space reaches, so the advance box alone would miss a glyph whose marks fall in the region
    /// while its advance does not. The declared box is carried by §9.6.4's own matrices: glyph
    /// space to text space by `/FontMatrix`, then §9.4.4's text rendering matrix — whose
    /// translation is the placed quadrilateral's first corner, glyph space's origin, because a
    /// Type 3 box's descent is zero — and whose linear part the walk tracks (ADR 1351).
    fn code_box(&self, quad: &[f32; 8], codes: &Codes, code: &[u8]) -> Result<[f32; 4], String> {
        let placed = bbox_of_points(quad);
        let Codes::Type3(marks) = codes else {
            return Ok(placed);
        };
        let Some(&byte) = code.first() else {
            return Ok(placed);
        };
        let stroke = (self.graphics.width, self.graphics.miter_limit);
        let Some([llx, lly, urx, ury]) = marks.glyph_box(self.document, self.page, stroke, byte)?
        else {
            return Ok(placed);
        };
        let text = Transform::new(
            self.font_size * self.horizontal,
            0.0,
            0.0,
            self.font_size,
            0.0,
            0.0,
        )
        .then(self.text_linear)
        .then(linear(self.ctm));
        let glyph_to_display = marks.font.font_matrix().then(text);
        let mut points: Vec<f32> = placed.to_vec();
        for (x, y) in [(llx, lly), (urx, lly), (urx, ury), (llx, ury)] {
            let point = glyph_to_display.apply(Point::new(narrow(x), narrow(y)));
            points.push(point.x + quad[0]);
            points.push(point.y + quad[1]);
        }
        Ok(bbox_of_points(&points))
    }

    /// The placed quadrilateral for a code index, or a refusal where the walk has outrun the
    /// interpreter (the code-count check, met early).
    fn quad_at(&self, index: usize) -> Result<[f32; 8], String> {
        self.quads.get(index).copied().ok_or_else(|| {
            "the content walk showed more codes than the interpreter placed; the page is refused"
                .to_owned()
        })
    }

    fn require_codes(&self) -> Result<Codes, String> {
        self.codes.clone().ok_or_else(|| {
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
        self.narrow_clip(&path);
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
        if fill.is_some()
            && let Some(name) = self.graphics.fill_pattern.clone()
        {
            self.pattern_admits_a_cut(&name)?;
        }
        if stroked && let Some(name) = self.graphics.stroke_pattern.clone() {
            self.pattern_admits_a_cut(&name)?;
        }
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
        if stroked && self.graphics.width <= 0.0 {
            replacement.extend_from_slice(&self.cut_hairline(&path, closes, &regions, to_display)?);
        } else if stroked {
            let outline = self.stroke_outline(&path, closes);
            let cut = cut_to_complement(&outline, &regions, to_display)?;
            if !cut.is_empty() {
                let colour = self.graphics.non_stroking_colour();
                let restated = self.restate_stroke();
                let mut text = String::new();
                paths::write_polygons(&mut text, &cut);
                // Balanced inside the replacement (§8.4.2), so the fill colour, alpha and
                // overprint the producer set for whatever comes next are the ones that come back.
                replacement.extend_from_slice(b"q\n");
                replacement.extend_from_slice(&restated);
                replacement.extend_from_slice(&colour);
                replacement.extend_from_slice(text.as_bytes());
                // §8.5.3.2's marks are the region the outline encloses, and `kurbo::stroke` winds
                // every contour of that outline the same way — so the nonzero rule is the one
                // that paints it, and `f*` would put holes where the outline overlaps itself.
                replacement.extend_from_slice(b"f\nQ\n");
            }
        }
        // §8.5.4's boundary, re-stated after the marks it used to accompany. The producer's own
        // bytes for the whole path object, then `n`: the marks above were painted under the clip
        // already in force, and this sets the clip to its intersection with the path the producer
        // constructed — which is the order the clause states and the same clip every later mark
        // on the page is held to (ADR 1248).
        if let Some(bytes) = path.clip_bytes.as_ref() {
            replacement.extend_from_slice(bytes);
            replacement.extend_from_slice(b"\nn\n");
        }
        self.paths_cut = self.paths_cut.saturating_add(1);
        self.edits.push((
            path.start,
            keyword_start.saturating_add(keyword.len()),
            replacement,
        ));
        Ok(())
    }

    /// The `gs` that gives a fill of a surviving outline the stroke's own alpha and overprint, or
    /// nothing where the fill's already are.
    ///
    /// A fill takes the non-stroking members of two pairs Table 51 keeps apart — §11.6.4.4's two
    /// alpha constants, one for strokes and one for everything else, and the two overprint
    /// parameters "one for stroking and one for all other painting operations" —
    /// and no operator sets either, so where a stroke's differ from the fill's they are restated
    /// by a graphics state parameter dictionary holding the stroke's values under the fill's keys,
    /// `/ca` and `/op`. The dictionary states the producer's own numbers and nothing else, so
    /// the outline composites exactly as the stroke did; the stream's resources gain it under a
    /// name no entry already has (ADR 1351).
    fn restate_stroke(&mut self) -> Vec<u8> {
        let mut state = Dictionary::new();
        if (self.graphics.stroking_alpha - self.graphics.fill_alpha).abs() > 0.0 {
            state.insert(
                Name::new(&b"ca"[..]),
                Object::Real(self.graphics.stroking_alpha),
            );
        }
        if self.graphics.stroke_overprint != self.graphics.fill_overprint {
            state.insert(
                Name::new(&b"op"[..]),
                Object::Boolean(self.graphics.stroke_overprint),
            );
        }
        if state.is_empty() {
            return Vec::new();
        }
        state.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"ExtGState"[..])),
        );
        let name = if let Some((name, _)) = self
            .added_states
            .iter()
            .find(|(_, earlier)| *earlier == state)
        {
            name.clone()
        } else {
            let taken = self.document.get_key(&self.resources, "ExtGState");
            let taken = taken.as_dict();
            let mut ordinal = self.added_states.len().saturating_add(1);
            loop {
                let candidate = format!("RedactStroke{ordinal}").into_bytes();
                let used = taken.is_some_and(|dict| {
                    dict.get_by_name(&Name::new(candidate.as_slice())).is_some()
                }) || self.added_states.iter().any(|(name, _)| *name == candidate);
                if !used {
                    break candidate;
                }
                ordinal = ordinal.saturating_add(1);
            }
        };
        if !self
            .added_states
            .iter()
            .any(|(earlier, _)| *earlier == name)
        {
            self.added_states.push((name.clone(), state));
        }
        let mut out = b"/".to_vec();
        out.extend_from_slice(&name);
        out.extend_from_slice(b" gs\n");
        out
    }

    /// A zero-width stroke cut as the path it is drawn along, and restroked at the same width
    /// (ADR 1363).
    ///
    /// §8.4.3.2 states the width in the device's terms — "[a] line width of 0 shall denote the
    /// thinnest line that can be rendered at device resolution: 1 device pixel wide" — so the
    /// marks are the path at whatever resolution draws it, and no outline in user space states
    /// them. The path is therefore cut as a line ([`paths::subtract_along`]): the stretches inside
    /// the region go, the rest are stroked again by the same `S` under the same graphics state,
    /// whose width is still 0. A dash pattern is applied first, as it is for a wide stroke, since
    /// it decides which stretches are marked at all; the dashes are written out as subpaths of
    /// their own and stroked solid, inside a §8.4.2-balanced `q`/`Q`, because a pattern restarted
    /// at each surviving piece would mark different stretches.
    fn cut_hairline(
        &self,
        path: &PathObject,
        closes: bool,
        regions: &[[f64; 4]],
        to_display: paths::Mapping,
    ) -> Result<Vec<u8>, String> {
        let source = path.stroked_path(closes);
        let subpaths = if self.graphics.dashes.is_empty() {
            split_open(&source)
        } else {
            let dashed: kurbo::BezPath = kurbo::dash(
                source.elements().iter().copied(),
                self.graphics.dash_phase,
                &self.graphics.dashes,
            )
            .collect();
            split_open(&dashed)
        };
        let cut = paths::subtract_along(&subpaths, regions, to_display).ok_or_else(|| {
            "§12.5.6.23: cutting a zero-width stroke against these regions exceeds this build's \
             bound on the surviving pieces; the page is refused rather than truncated"
                .to_owned()
        })?;
        if !cut.margin_holds() {
            return Err(
                "§7.3.3: a coordinate of the cut path is large enough that writing it back and \
                 reading it as a single-precision real could move the cut end inside the region; \
                 the page is refused rather than leave a sliver of the removed marks"
                    .to_owned(),
            );
        }
        if cut.polygons.is_empty() {
            return Ok(Vec::new());
        }
        let mut text = String::new();
        let dashed = !self.graphics.dashes.is_empty();
        if dashed {
            text.push_str("q\n[] 0 d\n");
        }
        paths::write_open(&mut text, &cut.polygons);
        text.push_str("S\n");
        if dashed {
            text.push_str("Q\n");
        }
        Ok(text.into_bytes())
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
    /// Offsetting a straight segment and closing it with a butt or projecting-square cap and a
    /// miter or bevel join is computed in closed form. A round cap, a round join and the offset of
    /// a curved segment are arcs no path can state, so they come back as cubics within
    /// [`ARC_TOLERANCE`] of the arc and are cut at their roots like any §8.5.2.2 curve: the
    /// outline outside the region is the producer's mark re-expressed to that bound, and the
    /// one inside it is gone. ADR 1324.
    fn stroke_outline(&self, path: &PathObject, closes: bool) -> Vec<paths::SubPath> {
        let source = path.stroked_path(closes);
        // `admits_a_cut` has refused a singular transform, so the norm is positive.
        let tolerance = ARC_TOLERANCE / mapping(path.ctm).norm();
        let outline = kurbo::stroke(
            source.elements().iter().copied(),
            &self.graphics.stroke_style(),
            &kurbo::StrokeOpts::default(),
            tolerance,
        );
        split_subpaths(&outline)
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
    fn do_xobject(
        &mut self,
        operands: &[(Operand, usize)],
        keyword_start: usize,
    ) -> Result<(), String> {
        let Some((name, name_start)) = operands.iter().find_map(|(operand, start)| match operand {
            Operand::Name(bytes) => Some((bytes.clone(), *start)),
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
                let clears = self.plan_image_clear(&name, &entry, &object)?;
                self.clears.extend(clears);
                Ok(())
            }
            b"Form" => self.run_form(
                &name,
                &entry,
                &object,
                Entered::Drawn {
                    name_start,
                    do_end: keyword_start.saturating_add(2),
                },
            ),
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
    fn run_form(
        &mut self,
        name: &[u8],
        entry: &Object,
        object: &Object,
        entered: Entered,
    ) -> Result<(), String> {
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

        let matrix = stated_matrix(self.document, &stream.dict);
        let resources = self
            .document
            .get_key(&stream.dict, "Resources")
            .as_dict()
            .cloned()
            .unwrap_or_else(|| self.page.resources.clone());

        let group = self
            .document
            .get_key(&stream.dict, "Group")
            .as_dict()
            .is_some_and(|group| {
                self.document
                    .get_key(group, "S")
                    .as_name()
                    .is_some_and(|name| name.as_bytes() == b"Transparency")
            });
        let saved = self.enter_frame(resources, matrix.then(self.ctm), entered, group);
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
        let (edits, states, (named, overrides, masks), resources) =
            self.leave_frame(saved, entered);
        walked?;

        let edited = (!edits.is_empty()).then(|| apply_edits(&data, edits));
        if let Some(id) = form_id {
            self.form_outcomes.push((id, edited.clone()));
        }
        let Some(content) = edited else {
            // Nothing of this form falls under the region: its stream crosses the output byte
            // for byte, and so does the `Do` that draws it.
            return match form_id {
                Some(id) if self.split.contains_key(&id) => self.place_split(id, None, entered),
                _ => Ok(()),
            };
        };
        let Some(id) = form_id else {
            return Err(format!(
                "§12.5.6.23: the form /{shown} draws marks under the region but is a direct \
                 object this removal cannot replace; the page is refused"
            ));
        };
        // A mask's group is reached through an `/ExtGState` entry, a graphics state dictionary
        // and a soft-mask dictionary, any of which another page may share; a copy for this page
        // is correct whichever of them is shared, because the original is carried into the
        // output only where something else still reaches it.
        let private = matches!(entered, Entered::MaskGroup { .. }) || !self.owns(id);
        let edit = FormEdit {
            id,
            content,
            private,
            resources,
            states,
            forms: named,
            overrides,
            masks,
            placement_copy: false,
        };
        if self.split.contains_key(&id) {
            return self.place_split(id, Some(edit), entered);
        }
        self.record_form_edit(edit)
    }

    /// Sets the page's per-stream state aside and starts a form's, returning what to put back.
    ///
    /// §8.10.1: a form inherits the graphics state, and its own `q`/`Q` nesting is its own — so
    /// the state carries in and the stack is set aside. Two things differ from inheritance: a
    /// transparency group starts from Table 51's initial alpha constants, which the table says a
    /// reader resets at the beginning of such a group's execution, and a soft mask's group is evaluated from the initial graphics state,
    /// text state included, under its own coordinate system (§11.6.5.1), which is what the
    /// interpreter runs it under.
    fn enter_frame(
        &mut self,
        resources: Dictionary,
        ctm: Transform,
        entered: Entered,
        group: bool,
    ) -> Frame {
        let saved = Frame {
            edits: std::mem::take(&mut self.edits),
            resources: std::mem::replace(&mut self.resources, resources),
            ctm: std::mem::replace(&mut self.ctm, ctm),
            ctm_stack: std::mem::take(&mut self.ctm_stack),
            path: self.path.take(),
            graphics: self.graphics.clone(),
            graphics_stack: std::mem::take(&mut self.graphics_stack),
            text: self.text_state(),
            added_states: std::mem::take(&mut self.added_states),
            added_forms: std::mem::take(&mut self.added_forms),
            added_overrides: std::mem::take(&mut self.added_overrides),
            added_masks: std::mem::take(&mut self.added_masks),
            stream_base: std::mem::replace(&mut self.stream_base, ctm),
        };
        if group {
            self.graphics.stroking_alpha = 1.0;
            self.graphics.fill_alpha = 1.0;
        }
        if matches!(entered, Entered::MaskGroup { .. }) {
            self.graphics = GraphicsState::default();
            self.set_text_state(TextState::default());
        }
        saved
    }

    /// Puts the enclosing stream's state back, returning the form's edits, the graphics states
    /// its edits name and the resources its names were resolved in.
    fn leave_frame(
        &mut self,
        saved: Frame,
        entered: Entered,
    ) -> (
        Edits,
        AddedStates,
        (AddedForms, Overrides, AddedMasks),
        Dictionary,
    ) {
        let edits = std::mem::replace(&mut self.edits, saved.edits);
        let states = std::mem::replace(&mut self.added_states, saved.added_states);
        let named = std::mem::replace(&mut self.added_forms, saved.added_forms);
        let overrides = std::mem::replace(&mut self.added_overrides, saved.added_overrides);
        let masks = std::mem::replace(&mut self.added_masks, saved.added_masks);
        self.stream_base = saved.stream_base;
        let resources = std::mem::replace(&mut self.resources, saved.resources);
        self.ctm = saved.ctm;
        self.ctm_stack = saved.ctm_stack;
        self.path = saved.path;
        self.graphics = saved.graphics;
        self.graphics_stack = saved.graphics_stack;
        if matches!(entered, Entered::MaskGroup { .. }) {
            self.set_text_state(saved.text);
        }
        (edits, states, (named, overrides, masks), resources)
    }

    /// Records one form's edited content in the form's own place.
    ///
    /// A form drawn twice on the page is one stream, so this place can hold it only one way: two
    /// placements asking for the same edit are one edit. Two asking for different ones are what
    /// the first walk of a page notes and the second splits ([`forms`], ADR 1363), so on the
    /// first walk the disagreement is left for [`forms::split`] to find.
    fn record_form_edit(&mut self, edit: FormEdit) -> Result<(), String> {
        if let Some(earlier) = self
            .form_edits
            .iter()
            .find(|earlier| earlier.id == edit.id && !earlier.placement_copy)
        {
            if earlier.content == edit.content {
                return Ok(());
            }
            if self.split.is_empty() {
                return Ok(());
            }
            return Err(format!(
                "§8.10.1: the second walk of the page asked form object {} for an edit the first \
                 did not; the page is refused rather than written from two readings",
                edit.id.number
            ));
        }
        self.form_edits.push(edit);
        Ok(())
    }

    /// The text state the walk tracks, as one value, for a stream that starts from its own.
    fn text_state(&self) -> TextState {
        TextState {
            linear: self.text_linear,
            size: self.font_size,
            horizontal: self.horizontal,
            char_spacing: self.char_spacing,
            word_spacing: self.word_spacing,
            codes: self.codes.clone(),
        }
    }

    /// Puts a text state back.
    fn set_text_state(&mut self, state: TextState) {
        self.text_linear = state.linear;
        self.font_size = state.size;
        self.horizontal = state.horizontal;
        self.char_spacing = state.char_spacing;
        self.word_spacing = state.word_spacing;
        self.codes = state.codes;
    }

    /// Plans clearing an image `XObject` that meets the region (§12.5.6.23), or refuses by name.
    ///
    /// The clearing itself is deferred to [`cleared_image_samples`], because a page may draw one
    /// image twice and the destroyed samples are the union across its placements. What this does
    /// is prove the image *can* be cleared without trace — its layout is known, and if it is
    /// behind a codec that codec is one this build runs as a filter and whose output a
    /// `FlateDecode` stream carries losslessly — so the refusal is decided here, at the page. The
    /// answer is the picture's clear and one for each mask it names ([`Walk::stated_masks`]),
    /// because a mask is image data of its own.
    fn plan_image_clear(
        &self,
        name: &[u8],
        entry: &Object,
        object: &Object,
    ) -> Result<Vec<ImageClear>, String> {
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
        let private = !self.owns(image_id);
        let image = self.document.image_stream(stream).ok_or_else(|| {
            format!("the image /{shown} did not decode to samples; the page is refused")
        })?;
        let codec = image.codec.clone();
        // §7.4.9 and §11.6.4.3: a non-zero `/SMaskInData` means the opacity came inside the
        // codestream, "the SMask entry shall not be present", and the embedded mask "shall
        // override any explicit or colour key mask specified by the image dictionary's Mask
        // entry". Such a picture's written dictionary names the opacity channel as its soft mask
        // and nothing else, so an entry it overrides is reached from nowhere and left behind.
        let opacity_in_data = codec.as_deref() == Some(&b"JPXDecode"[..])
            && self
                .document
                .get_key(&stream.dict, "SMaskInData")
                .as_integer()
                .is_some_and(|code| code != 0);
        let masks = if opacity_in_data {
            StatedMasks::default()
        } else {
            self.stated_masks(&shown, stream)?
        };
        let mut picture = if let Some(codec) = &codec {
            self.plan_codec_clear(
                &shown,
                image_id,
                stream,
                (codec, &image.data),
                Role::Picture,
            )?
        } else {
            self.plan_packed_clear(&shown, image_id, stream, &image.data, Role::Picture)?
        };
        picture.private = private;
        if let Some(matte) = &masks.matte {
            picture.fill = self.matte_fill(&shown, stream, &picture, matte)?;
        }
        let mut clears = vec![picture];
        for mask in masks.stated {
            clears.push(self.plan_mask_clear(&shown, &mask, private)?);
        }
        Ok(clears)
    }

    /// The sample a picture pre-blended with §11.6.5.2's matte colour takes where it is cleared,
    /// or the refusal by name where the matte does not describe the picture's components.
    ///
    /// Table 144's pre-blending is `c′ = m + α × (c − m)`, and a cleared soft mask states α = 0
    /// across the region, where the formula gives `c′ = m` whatever `c` was. So the picture's
    /// cleared samples are the matte, and the destroyed region is data the formula could have
    /// produced rather than a zero it could not: nothing of `c` survives in either image, and
    /// the two still describe one pre-blended picture (ADR 1333). The clause adds that the
    /// computation "shall use actual colour component values, with the effects of the Filter
    /// and Decode transformations already performed", so the matte is carried into samples by
    /// §8.9.5.2's map run backwards, component by component at each one's own depth.
    ///
    /// Every picture is written in the domain its dictionary states — a codec's output is its
    /// samples (Table 87), and a `JPXDecode` image's own integers are carried — so the matte,
    /// whose components are "valid colour components in that colour space" of the parent image
    /// (Table 144), is carried as the file stated it in every colour space (ADR 1371). An
    /// `Indexed` picture's matte is a colour of the base space, which no index is guaranteed to
    /// name, so its cleared indices stay the domain's zero: the region's opacity is zero either
    /// way, and §11.6.5.2 lets a reader choose any `c` there.
    fn matte_fill(
        &self,
        shown: &str,
        stream: &Stream,
        picture: &ImageClear,
        matte: &[f32],
    ) -> Result<Option<Vec<u32>>, String> {
        let refused = |why: &str| {
            format!(
                "§11.6.5.2: the image /{shown} is pre-blended with its soft mask's /Matte, and \
                 {why}; the page is refused"
            )
        };
        let space_entry = self.document.get_key(&stream.dict, "ColorSpace");
        let space = ColourSpace::parse(self.document, &space_entry, &self.resources);
        if matches!(space, Some(ColourSpace::Indexed { .. })) {
            return Ok(None);
        }
        let (pairs, depths) = match &picture.source {
            Source::Own(decoded) => (decoded.own.pairs.clone(), decoded.own.depths.clone()),
            Source::Packed | Source::Filtered(_) => {
                let space = space.ok_or_else(|| refused("its colour space cannot be read"))?;
                let bits = picture.layout.bits;
                let pairs = decode_pairs(self.document, &stream.dict, &space, bits);
                let depth = u8::try_from(bits).unwrap_or(u8::MAX);
                let depths = vec![depth; pairs.len()];
                (pairs, depths)
            }
        };
        matte_samples(matte, &pairs, &depths)
            .map(Some)
            .ok_or_else(|| refused("the matte does not name one value per component"))
    }

    /// The `/SMask` and `/Mask` streams an image names, each to be cleared on its own grid, or a
    /// refusal naming a mask the removal cannot clear.
    ///
    /// §12.5.6.23 forbids hiding what is removed — "clipping or image masks shall not be used to
    /// hide that data" — and a mask is also image data in its own right: §11.6.5.2's soft-mask
    /// image and §8.9.6.3's explicit mask each carry, sample by sample, the shape of the picture
    /// they belong to. So the portion under the region is destroyed in the mask as well as in the
    /// picture. Both are placed on the picture's unit square — "since all images shall be defined
    /// on the unit square in user space, their boundaries on the page will coincide" (§8.9.6.3)
    /// — so the picture's placement is the mask's, and the mask's own `/Width` and `/Height` are
    /// the grid its samples are cleared on.
    ///
    /// A §8.9.6.4 colour-key array is not image data: it is a test on the picture's samples,
    /// "colour values before decoding with the Decode array". Every picture is written as the
    /// integers its dictionary describes, so the ranges still test the samples they were written
    /// against and are carried with the dictionary (ADR 1371). A soft mask's §11.6.5.2 `/Matte`
    /// is read here and answered by [`Walk::matte_fill`].
    fn stated_masks(&self, shown: &str, stream: &Stream) -> Result<StatedMasks, String> {
        let mut masks = StatedMasks::default();
        for (key, role) in [("SMask", Role::SoftMask), ("Mask", Role::ExplicitMask)] {
            let Some(entry) = stream.dict.get(key) else {
                continue;
            };
            let target = self.document.resolve(entry);
            match (&target, entry.as_reference()) {
                (Object::Stream(mask), Some(id)) => {
                    if role == Role::SoftMask {
                        let matte = self.document.get_key(&mask.dict, "Matte");
                        if let Some(items) = matte.as_array() {
                            let values: Option<Vec<f32>> = items
                                .iter()
                                .map(|item| self.document.resolve(item).as_number().map(narrow))
                                .collect();
                            masks.matte = Some(values.ok_or_else(|| {
                                format!(
                                    "§11.6.5.2: the soft mask of the image /{shown} states a \
                                     /Matte that is not an array of numbers; the page is refused"
                                )
                            })?);
                        }
                    }
                    masks.stated.push(StatedMask { id, role });
                }
                (Object::Stream(_), None) => {
                    return Err(format!(
                        "§8.9.5: the image /{shown}'s /{key} is a direct stream this removal \
                         cannot replace; the page is refused"
                    ));
                }
                // Anything else names no mask image: nothing to clear.
                _ => {}
            }
        }
        Ok(masks)
    }

    /// Plans clearing one mask a picture names, on the mask's own grid and under the picture's
    /// placement, or refuses by name.
    ///
    /// The mask is copied rather than replaced wherever the picture is — a picture another page
    /// draws keeps its mask with it — and wherever the mask is reached from anything else as well
    /// (ADR 1196).
    fn plan_mask_clear(
        &self,
        picture: &str,
        mask: &StatedMask,
        picture_private: bool,
    ) -> Result<ImageClear, String> {
        let key = if mask.role == Role::SoftMask {
            "SMask"
        } else {
            "Mask"
        };
        let shown = format!("{picture}'s /{key}");
        let object = self.document.get(mask.id);
        let stream = object
            .as_stream()
            .ok_or_else(|| format!("the image /{shown} is not a stream; the page is refused"))?;
        let image = self.document.image_stream(stream).ok_or_else(|| {
            format!("the image /{shown} did not decode to samples; the page is refused")
        })?;
        let mut clear = if let Some(codec) = &image.codec {
            self.plan_codec_clear(&shown, mask.id, stream, (codec, &image.data), mask.role)?
        } else {
            self.plan_packed_clear(&shown, mask.id, stream, &image.data, mask.role)?
        };
        clear.private = picture_private || !self.owns(mask.id);
        Ok(clear)
    }

    /// Plans clearing a codec-free image in its own packed grid (§8.9.5.2), or refuses by name.
    fn plan_packed_clear(
        &self,
        shown: &str,
        image_id: ObjectId,
        stream: &Stream,
        data: &[u8],
        role: Role,
    ) -> Result<ImageClear, String> {
        let layout = self.image_layout(&stream.dict, shown)?;
        let stride = row_stride(layout)
            .ok_or_else(|| format!("the image /{shown}'s grid overflows; the page is refused"))?;
        let expected = stride
            .checked_mul(layout.height)
            .ok_or_else(|| format!("the image /{shown}'s grid overflows; the page is refused"))?;
        if data.len() < expected {
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
            role,
            source: Source::Packed,
            fill: None,
        })
    }

    /// Plans clearing an image behind a codec from the samples its filter delivers, or refuses
    /// by name (§8.9.5, §7.4.8, §7.4.6, §7.4.7, §7.4.9).
    ///
    /// A codec's stream bytes are its input, not samples, so the packed-grid destruction the
    /// codec-free path takes cannot address them. But a codec is a filter, and Table 87 says what
    /// it delivers: "a CCITTFaxDecode or JBIG2Decode filter shall always deliver 1-bit samples, a
    /// RunLengthDecode or DCTDecode filter shall always deliver 8-bit samples". Those samples are
    /// the image's own — the integers its `/ColorSpace`, `/Decode`, colour key and a mask's
    /// `/Matte` describe — so they are taken from the codec run as a filter
    /// ([`pdf_model::image::filter_samples`], under the process's isolation as the interpreter
    /// runs it: confined in the program, principle 3), cleared in the grid the dictionary states,
    /// and written under `FlateDecode` with the dictionary carried: a lossless, non-codec filter,
    /// so the cleared region is exactly the zero it was set to and cannot round-trip back through
    /// a codec that would leak it (ADR 1371).
    ///
    /// A `JPXDecode` image is written as its own integers ([`Walk::plan_jpx_clear`]), and one
    /// that is an image mask as the one-bit samples §7.4.9 requires of it
    /// ([`Walk::plan_jpx_stencil_clear`]).
    ///
    /// Refused, each for the clause it answers to (trap 5, principle 1): an image that does not
    /// decode; one whose filter stopped short of its grid or concealed damaged rows, whose missing
    /// rows are not the image's; one decoded off the grid the dictionary states; and one whose
    /// dictionary contradicts what the filter delivers ([`Walk::filtered_layout`]).
    #[expect(
        clippy::doc_markdown,
        reason = "the comment quotes Table 87 and §7.4.9 verbatim, and a quotation is not marked up"
    )]
    fn plan_codec_clear(
        &self,
        shown: &str,
        image_id: ObjectId,
        stream: &Stream,
        (codec, codestream): (&[u8], &[u8]),
        role: Role,
    ) -> Result<ImageClear, String> {
        let stencil = matches!(
            self.document.get_key(&stream.dict, "ImageMask"),
            Object::Boolean(true)
        );
        if codec == b"JPXDecode" {
            return if stencil {
                self.plan_jpx_stencil_clear(shown, image_id, stream, codestream, role)
            } else {
                self.plan_jpx_clear(shown, image_id, stream, codestream, role)
            };
        }
        let filtered =
            pdf_model::image::filter_samples(self.document, stream).map_err(|error| {
                if let ImageError::UnsupportedFilter { filter } = &error {
                    format!(
                        "§8.9.5: the image /{shown} is encoded with the {filter} codec, whose \
                     samples this removal does not re-encode; the page is refused"
                    )
                } else {
                    format!(
                        "§8.9.5: the codec image /{shown} did not decode to samples ({error}); the \
                     page is refused"
                    )
                }
            })?;
        if let Some(said) = &filtered.shortfall {
            return Err(format!(
                "§8.9.5: the codec image /{shown} decoded short of its grid ({said}); the page is \
                 refused rather than redact a partial decode"
            ));
        }
        self.plan_filtered_clear(
            shown,
            image_id,
            stream,
            (
                Arc::from(filtered.data),
                (filtered.width, filtered.height),
                filtered.components,
                filtered.bits,
            ),
            role,
        )
    }

    /// The clear of samples a filter delivered, once they are held to the grid and the shape the
    /// dictionary states ([`Walk::filtered_layout`]).
    fn plan_filtered_clear(
        &self,
        shown: &str,
        image_id: ObjectId,
        stream: &Stream,
        (data, grid, components, bits): (Arc<[u8]>, (u32, u32), usize, u8),
        role: Role,
    ) -> Result<ImageClear, String> {
        let layout = self.filtered_layout(shown, &stream.dict, (grid, components, bits))?;
        let expected = row_stride(layout)
            .and_then(|stride| stride.checked_mul(layout.height))
            .ok_or_else(|| format!("the image /{shown}'s grid overflows; the page is refused"))?;
        if data.len() < expected {
            return Err(format!(
                "§8.9.5: the codec image /{shown} delivered fewer samples than its {}×{} grid; \
                 the page is refused rather than clear samples that are not there",
                layout.width, layout.height
            ));
        }
        Ok(ImageClear {
            image_id,
            ctm: self.ctm,
            regions: self.regions.clone(),
            layout,
            private: !self.owns(image_id),
            role,
            source: Source::Filtered(data),
            fill: None,
        })
    }

    /// The layout of samples a filter delivered, held to what the dictionary states, or the
    /// refusal naming the contradiction.
    ///
    /// The raster has to be the image's own grid, which is what the dictionary states
    /// (§8.9.5.1 Table 87) and, for a `DCTDecode` frame, what §7.4.8 puts in the encoded data;
    /// where the two part, the samples written back would not be the image the dictionary
    /// describes. The components have to be the colour space's, which is what interprets them,
    /// and an image mask's samples one bit: Table 87 says "[i]f ImageMask is true , the value of
    /// BitsPerComponent , if present, shall be 1". The depth is the filter's, which Table 87 makes
    /// the dictionary's to be "consistent with", so the written dictionary states it
    /// ([`Described::Filtered`]).
    #[expect(
        clippy::doc_markdown,
        reason = "the comment quotes Table 87 and §7.4.9 verbatim, and a quotation is not marked up"
    )]
    fn filtered_layout(
        &self,
        shown: &str,
        dict: &Dictionary,
        (grid, components, bits): ((u32, u32), usize, u8),
    ) -> Result<ImageLayout, String> {
        let stated = (
            positive_dim(self.document, dict, "Width", shown)?,
            positive_dim(self.document, dict, "Height", shown)?,
        );
        let width = usize::try_from(grid.0).map_err(|_| {
            format!("the codec image /{shown}'s grid overflows; the page is refused")
        })?;
        let height = usize::try_from(grid.1).map_err(|_| {
            format!("the codec image /{shown}'s grid overflows; the page is refused")
        })?;
        if stated != (width, height) {
            return Err(format!(
                "§8.9.5: the codec image /{shown} states a {}×{} grid and decoded to {width}×\
                 {height}, so the samples this removal would write back are not the image's own; \
                 the page is refused",
                stated.0, stated.1
            ));
        }
        let stencil = matches!(
            self.document.get_key(dict, "ImageMask"),
            Object::Boolean(true)
        );
        if stencil {
            if (components, bits) != (1, 1) {
                return Err(format!(
                    "§8.9.5.1: the image mask /{shown}'s filter delivered {components} \
                     component(s) of {bits} bits, where Table 87 gives an image mask one bit a \
                     sample; the page is refused"
                ));
            }
        } else {
            let space = self.document.get_key(dict, "ColorSpace");
            let stated_components = ColourSpace::parse(self.document, &space, &self.resources)
                .map(|resolved| resolved.components())
                .filter(|count| *count > 0)
                .ok_or_else(|| {
                    format!(
                        "the image /{shown}'s colour space is one this removal cannot count the \
                         components of; the page is refused"
                    )
                })?;
            if stated_components != components {
                return Err(format!(
                    "§8.9.5.1: the image /{shown}'s colour space takes {stated_components} \
                     component(s) and its filter delivered {components}, so the samples are not \
                     the ones the dictionary describes; the page is refused"
                ));
            }
        }
        Ok(ImageLayout {
            width,
            height,
            components,
            bits: usize::from(bits),
        })
    }

    /// Plans clearing a `JPXDecode` image mask on the one-bit samples §7.4.9 requires of it, or
    /// refuses by name.
    ///
    /// §7.4.9: "If ImageMask is true , the JPEG 2000 data shall provide a single colour channel
    /// with 1-bit samples". Those are the samples §8.9.6.2's stencil is made of, so they are taken
    /// as the decoder's own integers at full resolution ([`pdf_model::image::jpx_samples`]),
    /// packed as §8.9.5.2 lays out one-bit samples, and cleared and carried as any filter's are
    /// ([`Walk::plan_filtered_clear`]).
    #[expect(
        clippy::doc_markdown,
        reason = "the comment quotes Table 87 and §7.4.9 verbatim, and a quotation is not marked up"
    )]
    fn plan_jpx_stencil_clear(
        &self,
        shown: &str,
        image_id: ObjectId,
        stream: &Stream,
        codestream: &[u8],
        role: Role,
    ) -> Result<ImageClear, String> {
        let samples = self.jpx_whole(shown, stream, codestream)?;
        if samples.components != 1 || samples.depths.as_slice() != [1] {
            return Err(format!(
                "§7.4.9: the JPEG 2000 image mask /{shown} decoded to {} channel(s) of {:?} bits, \
                 where the clause requires \"a single colour channel with 1-bit samples\"; the \
                 page is refused",
                samples.components, samples.depths
            ));
        }
        let width = usize::try_from(samples.width).unwrap_or(0);
        let stride = width.div_ceil(8);
        let mut packed =
            vec![0u8; stride.saturating_mul(usize::try_from(samples.height).unwrap_or(0))];
        if width > 0 {
            for (row, line) in samples.colour.chunks_exact(width).enumerate() {
                for (col, sample) in line.iter().enumerate() {
                    if *sample != 0
                        && let Some(cell) =
                            packed.get_mut(row.saturating_mul(stride).saturating_add(col / 8))
                    {
                        *cell |= 0x80u8 >> (col % 8);
                    }
                }
            }
        }
        self.plan_filtered_clear(
            shown,
            image_id,
            stream,
            (Arc::from(packed), (samples.width, samples.height), 1, 1),
            role,
        )
    }

    /// A `JPXDecode` image's own samples at full resolution, within the operator's budget, or
    /// the refusal naming the budget (ADR 1333).
    fn jpx_whole(
        &self,
        shown: &str,
        stream: &Stream,
        codestream: &[u8],
    ) -> Result<pdf_model::image::JpxSamples, String> {
        pdf_model::image::jpx_samples(
            self.document,
            &stream.dict,
            &self.resources,
            (codestream, self.image_samples),
        )
        .map_err(|error| {
            format!(
                "§7.4.9: the JPEG 2000 image /{shown} was not decoded to its own samples at \
                 full resolution ({error}); a reduced resolution level would resample every \
                 sample outside the region, so the page is refused — an operator whose machine \
                 can hold a larger decode states it with --image-samples"
            )
        })
    }

    /// Plans clearing a `JPXDecode` image on its own samples, or refuses by name (ADRs 1333,
    /// 1371).
    ///
    /// §12.5.6.23 destroys "that portion of the image data" and says nothing of the rest, which
    /// is therefore to be carried as the file had it. The decoder delivers the codestream's
    /// components as integers at a precision Table 87 leaves to the processor —
    /// "[t]he bit depth is determined by the PDF processor in the process of decoding the JPEG
    /// 2000 image" — each component at its own depth where §7.4.9's "[t]he colour components in
    /// an image may have different numbers of bits per sample" applies, and in the colour space
    /// §7.4.9's precedence names. Table 87 has one depth for an image — "the number of bits shall
    /// be the same for all colour components" — so the integers are written back under
    /// `FlateDecode` unchanged in the field of the widest ([`pdf_model::image::jpx_samples`]):
    /// eight bits where every component is eight or fewer, sixteen above, with each component's
    /// own `/Decode` pair widened from its own depth so §8.9.5.2's map gives every integer the
    /// value it had ([`widened_decode`]). Nothing outside the region is converted, requantised or
    /// resampled, so a colour key's ranges and a soft mask's `/Matte` still describe the samples
    /// they were written against and are carried with them. Table 87's `/SMaskInData` opacity
    /// becomes the soft-mask image the table has the processor "create", at the same depth.
    fn plan_jpx_clear(
        &self,
        shown: &str,
        image_id: ObjectId,
        stream: &Stream,
        codestream: &[u8],
        role: Role,
    ) -> Result<ImageClear, String> {
        self.jpx_admits_a_clear(shown, stream, codestream)?;
        let samples = self.jpx_whole(shown, stream, codestream)?;
        let stated = (
            positive_dim(self.document, &stream.dict, "Width", shown)?,
            positive_dim(self.document, &stream.dict, "Height", shown)?,
        );
        let width = usize::try_from(samples.width).unwrap_or(0);
        let height = usize::try_from(samples.height).unwrap_or(0);
        if stated != (width, height) {
            // §7.4.9 NOTE 3: a codestream over the decoder's sample budget "can select and
            // decode only the data making up a lower-resolution version", and a raster on that
            // grid is not the image — written back, it would resample everything outside the
            // region as well.
            return Err(format!(
                "§7.4.9: the JPEG 2000 image /{shown} states a {}×{} grid and the decoder's \
                 budget delivered a reduced resolution level of {width}×{height}, so the \
                 samples this removal would write back are not the image's own; the page is \
                 refused",
                stated.0, stated.1
            ));
        }
        if role == Role::SoftMask && samples.components != 1 {
            return Err(format!(
                "§11.6.5.2: the soft-mask image /{shown} decoded to {} colour components where \
                 Table 143 requires DeviceGray; the page is refused",
                samples.components
            ));
        }
        if samples.premultiplied && samples.space == pdf_model::image::JpxSpace::Stated {
            let entry = self.document.get_key(&stream.dict, "ColorSpace");
            if matches!(
                ColourSpace::parse(self.document, &entry, &self.resources),
                Some(ColourSpace::Indexed { .. })
            ) {
                return Err(format!(
                    "§8.9.5.1: the JPEG 2000 image /{shown} states /SMaskInData 2 over an \
                     Indexed space, whose indices no opacity can have multiplied; the page is \
                     refused"
                ));
            }
        }
        let bits = if samples.precision > 8 { 16 } else { 8 };
        let decode = samples
            .decode
            .iter()
            .zip(&samples.depths)
            .flat_map(|(pair, depth)| widened_decode(&[*pair], *depth, bits))
            .collect();
        let opacity_decode = widened_decode(&[(0.0, 1.0)], samples.opacity_depth, bits);
        let layout = ImageLayout {
            width,
            height,
            components: samples.components,
            bits,
        };
        let own = OwnRaster {
            space: samples.space.clone(),
            bits,
            decode,
            opacity_decode,
            premultiplied: samples.premultiplied,
            depths: samples.depths.clone(),
            pairs: samples.decode.clone(),
        };
        Ok(ImageClear {
            image_id,
            ctm: self.ctm,
            regions: self.regions.clone(),
            layout,
            private: !self.owns(image_id),
            role,
            source: Source::Own(Decoded {
                samples: Arc::from(samples.colour.as_slice()),
                opacity: samples.opacity.map(|alpha| Arc::from(alpha.as_slice())),
                own: Arc::new(own),
            }),
            fill: None,
        })
    }

    /// The conditions a `JPXDecode` image's codestream has to meet before its samples are
    /// cleared.
    ///
    /// §12.5.6.23 asks for one thing of an image — "that portion of the image data shall be
    /// destroyed; clipping or image masks shall not be used to hide that data" — and says nothing
    /// about the encoding the rest of it survives in. So decoding the codestream, zeroing the
    /// region's samples and writing the whole grid back under `FlateDecode` *is* the destruction
    /// the clause asks for: the removed samples are in the output in no form at all, and the ones
    /// outside the region are the values a reader decoded before, carried losslessly. That the
    /// codestream was lossy does not weaken it — the samples a reader sees are the decoder's
    /// output, and those are exactly what is re-encoded. What has to hold before that
    /// (ADRs 1248, 1333, 1371):
    ///
    /// - **Table 87's `/SMaskInData` is one of the three codes it defines.** Codes 1 and 2 say
    ///   the codestream carries an opacity channel from which "[a] PDF processor shall create a
    ///   soft-mask image", and [`Walk::plan_jpx_clear`] writes that image (ADR 1277).
    /// - **Every component fits a sample Table 87 can state.** §7.4.9 lets a component be "between
    ///   1 to 38 inclusive" bits; Table 87 lets an image's samples be at most sixteen — "[t]he
    ///   value shall be 1 , 2 , 4 , 8 , or (from PDF 1.5) 16" — so a deeper component's integers
    ///   have no field in any image this removal can write, and carrying them coarser would
    ///   change content the annotation did not identify.
    fn jpx_admits_a_clear(
        &self,
        shown: &str,
        stream: &Stream,
        codestream: &[u8],
    ) -> Result<(), String> {
        match self
            .document
            .get_key(&stream.dict, "SMaskInData")
            .as_integer()
        {
            None | Some(0..=2) => {}
            Some(other) => {
                return Err(format!(
                    "§8.9.5.1: the JPEG 2000 image /{shown} states /SMaskInData {other}, a code \
                     Table 87 does not define; the page is refused rather than guess what its \
                     samples carry"
                ));
            }
        }
        let headers = pdf_model::jpeg2000::Headers::parse(codestream).map_err(|error| {
            format!(
                "§7.4.9: the JPEG 2000 image /{shown} does not state its own precision \
                 ({error}); the page is refused rather than re-encode a grid this removal \
                 cannot show it preserves"
            )
        })?;
        let depths = headers.component_depths();
        if depths.is_empty() {
            return Err(format!(
                "§7.4.9: the JPEG 2000 image /{shown} states no component precision at all, so \
                 this removal cannot show its re-encode preserves the samples outside the \
                 region; the page is refused"
            ));
        }
        if let Some(deep) = depths
            .iter()
            .map(|depth| depth.bits)
            .find(|bits| *bits > 16)
        {
            return Err(format!(
                "§7.4.9: the JPEG 2000 image /{shown} states a component of {deep} bits, and \
                 Table 87 gives an image sample at most sixteen, so no image this removal writes \
                 can carry its integers; the page is refused rather than coarsen the samples \
                 outside the region"
            ));
        }
        Ok(())
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

    /// A soft mask a `gs` establishes (§11.6.4.3): its transparency group's content is walked
    /// and cut like any other content stream.
    ///
    /// §12.5.6.23 asks for "all traces of the specified content" to go, and a mask's group is
    /// content that marks the page — not in colour, but as the shape or opacity of what is
    /// painted under it — so its marks within the region are traces of what the region showed.
    /// A glyph drawn into a `/Luminosity` group is legible through every mark the mask applies
    /// to, and it would survive in the file with every one of those marks cut away. So the group
    /// is entered, in §11.6.5.1's coordinate system — "concatenating the transformation matrix
    /// specified by the Matrix entry in the transparency group's form dictionary … with the
    /// current transformation matrix at the moment the soft mask is established in the graphics
    /// state with the gs operator" — at the `gs`, which is where the interpreter evaluates it
    /// and so where its codes fall in the count this walk is held to. ADR 1352.
    fn enter_soft_mask(&mut self, state: &Dictionary, at: (usize, usize)) -> Result<(), String> {
        let mask = self.document.get_key(state, "SMask");
        let Some(mask) = mask.as_dict() else {
            // `/None`, or no entry: §11.6.4.3's absence of a mask, which removes nothing.
            return Ok(());
        };
        let Some(entry) = mask.get("G").cloned() else {
            return Err(
                "§11.6.5.1: a soft mask with no /G group is in force; the page is refused rather \
                 than walked past a mask whose content cannot be read"
                    .to_owned(),
            );
        };
        let object = self.document.resolve(&entry);
        self.run_form(
            b"SMask /G",
            &entry,
            &object,
            Entered::MaskGroup {
                name_start: at.0,
                gs_end: at.1,
            },
        )
    }

    /// Whether the page's `/XObject` resource subdictionary is not itself a shared object.
    fn xobject_private(&self) -> bool {
        let resources = self.document.get_key(&self.page.dict, "Resources");
        match resources.as_dict().and_then(|dict| dict.get("XObject")) {
            Some(Object::Reference(id)) => self.counts.get(&id.number).copied() == Some(1),
            _ => true,
        }
    }

    /// `gs`: the named graphics state's line parameters and alphas, and its soft mask entered.
    fn ext_gstate(
        &mut self,
        operands: &[(Operand, usize)],
        keyword_start: usize,
    ) -> Result<(), String> {
        let Some((name, name_start)) = operands.iter().find_map(|(operand, start)| match operand {
            Operand::Name(bytes) => Some((bytes.clone(), *start)),
            _ => None,
        }) else {
            return Ok(());
        };
        let states = self.document.get_key(&self.resources, "ExtGState");
        let entry = states
            .as_dict()
            .and_then(|dict| dict.get_by_name(&Name::new(name.as_slice())))
            .cloned();
        let state = entry.as_ref().map(|entry| self.document.resolve(entry));
        let Some(dict) = state.as_ref().and_then(Object::as_dict) else {
            return Ok(());
        };
        self.current_state = entry;
        self.enter_soft_mask(dict, (name_start, keyword_start.saturating_add(2)))?;
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
        // Table 58: "Specifying an OP entry shall set both parameters unless there is also an op
        // entry in the same graphics state parameter dictionary".
        let flag = |key: &str| match self.document.get_key(dict, key) {
            Object::Boolean(value) => Some(value),
            _ => None,
        };
        let (stroking, other) = (flag("OP"), flag("op"));
        if let Some(value) = stroking {
            self.graphics.stroke_overprint = value;
        }
        if let Some(value) = other.or(stroking) {
            self.graphics.fill_overprint = value;
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
    /// A codec-free image's samples come from [`Document::image_stream`] exactly as an image
    /// `XObject`'s do, are zeroed by [`clear_region`] under this placement, re-encoded
    /// `FlateDecode`, and written back under a dictionary carrying every §8.9.7 key the source
    /// stated (its `/Width`, `/Height`, `/BitsPerComponent`, colour space, `/Decode`,
    /// `/ImageMask`\u{2026}) with only the old encoding replaced. Zero is the sample domain's own
    /// constant: §8.9.5.2 maps it through any `/Decode` to that array's `Dmin`, carrying none of
    /// the original sample (ADR 1126). An image behind one of the two codecs Table 92 lets an
    /// inline image name, `DCTDecode` and `CCITTFaxDecode`, is decoded and re-expressed as an
    /// image `XObject` behind one is ([`inline_codec_raster`]).
    fn spliced_inline_image(&self, stream: &Stream) -> Result<Vec<u8>, String> {
        let image = self.document.image_stream(stream).ok_or_else(|| {
            "§8.9.7: an inline image meeting the region did not decode to samples; the page is \
             refused"
                .to_owned()
        })?;
        let mut dict = stream.dict.clone();
        let (mut samples, layout) = if let Some(codec) = &image.codec {
            let (samples, layout) = self.inline_filter_output(stream, codec)?;
            // Table 87: `/BitsPerComponent` "shall be consistent with the size of the data
            // samples that the filter delivers", and those are the samples now written.
            dict.insert(
                Name::new(&b"BitsPerComponent"[..]),
                Object::Integer(i64::try_from(layout.bits).unwrap_or(i64::MAX)),
            );
            (samples, layout)
        } else {
            let layout = self.image_layout(&stream.dict, "an inline image")?;
            (image.data.to_vec(), layout)
        };
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
        if samples.len() < expected {
            return Err(
                "§8.9.5: an inline image meeting the region has less sample data than its declared \
                 grid; the page is refused rather than clear samples that are not there"
                    .to_owned(),
            );
        }
        clear_region(
            &mut samples,
            stride,
            layout,
            (self.ctm, &self.regions),
            None,
        );
        let encoded = flate_encode(&samples, 6).ok_or_else(|| {
            "§8.9.7: an inline image's cleared samples could not be re-encoded; the page is refused"
                .to_owned()
        })?;
        build_inline_image(self.document, &dict, &self.resources, &encoded)
    }

    /// The samples an inline image's codec delivers as a filter, and their layout, or the
    /// refusal by name.
    ///
    /// §8.9.7 names the codecs an inline image may use by leaving the others out: "JBIG2Decode ,
    /// Crypt and JPXDecode are not listed … because those filters shall not be used with inline
    /// images". The two it leaves, `DCTDecode` and `CCITTFaxDecode`, deliver the image's samples
    /// as an image `XObject`'s codec does ([`pdf_model::image::filter_samples`], ADR 1371), held
    /// to what the inline dictionary states ([`Walk::filtered_layout`]), so the splice carries
    /// every key the producer wrote with only the encoding replaced. A filter the clause forbids
    /// here is refused by name rather than decoded on the file's word.
    #[expect(
        clippy::doc_markdown,
        reason = "the comment quotes §8.9.7 verbatim, and a quotation is not marked up"
    )]
    fn inline_filter_output(
        &self,
        stream: &Stream,
        codec: &[u8],
    ) -> Result<(Vec<u8>, ImageLayout), String> {
        if !matches!(codec, b"DCTDecode" | b"DCT" | b"CCITTFaxDecode" | b"CCF") {
            return Err(format!(
                "§8.9.7: an inline image meeting the region is encoded with {}, a filter that \
                 \"shall not be used with inline images\"; the page is refused",
                String::from_utf8_lossy(codec)
            ));
        }
        let shown = "an inline image";
        let filtered = pdf_model::image::filter_samples(self.document, stream).map_err(|error| {
            format!(
                "§8.9.7: an inline image meeting the region is encoded with the {} codec and did \
                 not decode to samples ({error}); the page is refused",
                String::from_utf8_lossy(codec)
            )
        })?;
        if let Some(said) = &filtered.shortfall {
            return Err(format!(
                "§8.9.7: an inline image meeting the region decoded short of its grid ({said}); \
                 the page is refused rather than redact a partial decode"
            ));
        }
        let layout = self.filtered_layout(
            shown,
            &stream.dict,
            (
                (filtered.width, filtered.height),
                filtered.components,
                filtered.bits,
            ),
        )?;
        Ok((filtered.data, layout))
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

/// A dictionary's `/Matrix` (Table 93's, a pattern's, Table 78's), or the identity it defaults to.
fn stated_matrix(document: &Document, dict: &Dictionary) -> Transform {
    numbers(document, dict, "Matrix")
        .and_then(|values| {
            matrix(
                &values
                    .iter()
                    .map(|value| f64::from(*value))
                    .collect::<Vec<_>>(),
            )
        })
        .unwrap_or(Transform::IDENTITY)
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

/// The pattern name a `scn` or `SCN` operator's operands end with, if they name one (§8.6.8).
fn pattern_name(operands: &[(Operand, usize)]) -> Option<Vec<u8>> {
    operands
        .iter()
        .rev()
        .find_map(|(operand, _)| match operand {
            Operand::Name(bytes) => Some(bytes.clone()),
            _ => None,
        })
}

/// The linear part of a transform (its translation dropped).
fn linear(transform: Transform) -> Transform {
    Transform::new(transform.a, transform.b, transform.c, transform.d, 0.0, 0.0)
}

/// One string's byte offsets and lengths per code.
///
/// A simple font's code is one byte (§9.6). A composite font's is what §9.7.6.2's matching
/// against the codespace ranges extracts, taken by [`pdf_font::cmap::CMap::next_code`] — the
/// function the reader decodes with, including §9.7.6.3's rule for how many bytes a code outside
/// every range consumes — so a removed code takes exactly its own bytes and no neighbour's.
fn split_codes(bytes: &[u8], codes: &Codes) -> Vec<(usize, usize)> {
    let Codes::Composite(composite) = codes else {
        return (0..bytes.len()).map(|start| (start, 1)).collect();
    };
    let cmap = &composite.cmap;
    let mut out = Vec::new();
    let mut start = 0;
    while let Some(rest) = bytes.get(start..).filter(|rest| !rest.is_empty()) {
        let length = usize::from(cmap.next_code(rest).length()).max(1);
        out.push((start, length.min(rest.len())));
        start = start.saturating_add(length);
    }
    out
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

/// `/Decode` pairs read at `precision` restated for samples written at `bits` with the same
/// integers, flattened as §8.9.5.2's array takes them.
///
/// §8.9.5.2 maps a sample `x` to `D min + x × (D max − D min) ÷ (2^n − 1)`, so an integer read
/// at `n = precision` and written at `n = bits` keeps its value when the pair's far end becomes
/// `D min + (D max − D min) × (2^bits − 1) ÷ (2^precision − 1)` — which is the pair itself where
/// the two depths agree.
pub(crate) fn widened_decode(pairs: &[(f32, f32)], precision: u8, bits: usize) -> Vec<f32> {
    let highest = |depth: u32| f64::from(1u32.checked_shl(depth).unwrap_or(0).saturating_sub(1));
    let from = highest(u32::from(precision)).max(1.0);
    let to = highest(u32::try_from(bits).unwrap_or(8));
    pairs
        .iter()
        .flat_map(|(low, high)| {
            let (low, high) = (f64::from(*low), f64::from(*high));
            [narrow(low), narrow((high - low).mul_add(to / from, low))]
        })
        .collect()
}

/// The `/Decode` pair a reader maps each component of a packed image through: the dictionary's
/// where it states one, Table 88's default for the space otherwise (§8.9.5.2).
fn decode_pairs(
    document: &Document,
    dict: &Dictionary,
    space: &ColourSpace,
    bits: usize,
) -> Vec<(f32, f32)> {
    let stated: Vec<f32> = document
        .get_key(dict, "Decode")
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| document.resolve(item).as_number().map(narrow))
                .collect()
        })
        .unwrap_or_default();
    let bits = u32::try_from(bits).unwrap_or(8);
    (0..space.components())
        .map(|component| {
            let at = component.saturating_mul(2);
            match (stated.get(at), stated.get(at.saturating_add(1))) {
                (Some(low), Some(high)) => (*low, *high),
                _ => space.default_decode(component, bits),
            }
        })
        .collect()
}

/// §11.6.5.2's matte colour as the samples §8.9.5.2's map takes nearest to it, one per
/// component at that component's own depth — the map run backwards — or `None` where the matte
/// does not name one value for each pair.
fn matte_samples(matte: &[f32], pairs: &[(f32, f32)], depths: &[u8]) -> Option<Vec<u32>> {
    if matte.len() != pairs.len() || depths.len() != pairs.len() {
        return None;
    }
    Some(
        matte
            .iter()
            .zip(pairs)
            .zip(depths)
            .map(|((value, (low, high)), depth)| {
                let highest = 1u32
                    .checked_shl(u32::from(*depth))
                    .unwrap_or(0)
                    .saturating_sub(1);
                let span = f64::from(*high) - f64::from(*low);
                if span == 0.0 {
                    return 0;
                }
                let at = (f64::from(*value) - f64::from(*low)) / span * f64::from(highest);
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "clamped into 0..=highest, which a u32 holds, before the cast"
                )]
                let sample = at.round().clamp(0.0, f64::from(highest)) as u32;
                sample
            })
            .collect(),
    )
}

/// What [`cleared_image_samples`] produced for one image object.
struct ClearedSamples {
    /// The cleared samples, re-encoded `FlateDecode`.
    encoded: Vec<u8>,
    /// How the written dictionary describes them.
    written: Described,
    /// §7.4.9's opacity channel, cleared on the same grid and re-encoded, where one was carried.
    opacity: Option<Vec<u8>>,
}

/// The image's samples with every region's samples zeroed, re-encoded with `FlateDecode`, and
/// how the written dictionary describes them.
///
/// §12.5.6.23: "that portion of the image data shall be destroyed". For a **codec-free** image the
/// samples come from [`Document::image_stream`], which runs every filter before the codec, so they
/// are the packed samples themselves; the codec-free precondition was proved at planning time
/// ([`Walk::plan_packed_clear`]) and is re-checked here rather than trusted across the two phases.
/// For a **codec** image ([`Walk::plan_codec_clear`]) the samples the filter delivered were taken
/// then and travel on the placement; the source stream's bytes are the codec's input, not samples,
/// so they are never read here. Either way every placement's region is cleared and the result
/// re-encoded `FlateDecode`. A §7.4.9 opacity channel is image data too, and it is cleared under
/// the same placements on the same grid, so the shape of what was removed does not survive in it.
fn cleared_image_samples(
    document: &Document,
    stream: &Stream,
    placements: &[ImageClear],
) -> Result<ClearedSamples, String> {
    let first = placements
        .first()
        .ok_or_else(|| "no placement to clear".to_owned())?;
    let layout = first.layout;
    // All placements of one object read the same image, so any one's samples are the object's;
    // a codec-free image reads its packed samples from the stream instead. Either way the
    // placements' regions are unioned onto the samples below.
    let (mut samples, written) = match &first.source {
        Source::Packed => {
            let image = document
                .image_stream(stream)
                .ok_or_else(|| "the image no longer decodes to samples".to_owned())?;
            if image.codec.is_some() {
                return Err(
                    "the image is behind a codec whose samples this removal does not re-encode"
                        .to_owned(),
                );
            }
            (image.data.to_vec(), Described::Carried)
        }
        Source::Filtered(data) => (data.to_vec(), Described::Filtered { bits: layout.bits }),
        Source::Own(decoded) => (
            decoded.samples.to_vec(),
            Described::Own {
                layout,
                own: Arc::clone(&decoded.own),
            },
        ),
    };
    let stride = row_stride(layout).ok_or_else(|| "the image grid overflows".to_owned())?;
    let expected = stride
        .checked_mul(layout.height)
        .ok_or_else(|| "the image grid overflows".to_owned())?;
    if samples.len() < expected {
        return Err("the image sample data is shorter than its declared grid".to_owned());
    }
    // The opacity channel is one component on the picture's grid, at the picture's own written
    // depth.
    let mut opacity = match &first.source {
        Source::Own(decoded) => decoded.opacity.as_ref().map(|alpha| {
            (
                alpha.to_vec(),
                ImageLayout {
                    components: 1,
                    bits: decoded.own.bits,
                    ..layout
                },
            )
        }),
        Source::Packed | Source::Filtered(_) => None,
    };
    for clear in placements {
        clear_region(
            &mut samples,
            stride,
            clear.layout,
            (clear.ctm, &clear.regions),
            clear.fill.as_deref(),
        );
        if let Some((channel, channel_layout)) = opacity.as_mut() {
            let stride =
                row_stride(*channel_layout).ok_or_else(|| "the image grid overflows".to_owned())?;
            clear_region(
                channel,
                stride,
                *channel_layout,
                (clear.ctm, &clear.regions),
                None,
            );
        }
    }
    let encoded = flate_encode(&samples, 6)
        .ok_or_else(|| "the cleared samples could not be re-encoded".to_owned())?;
    let opacity = opacity
        .map(|(channel, _)| {
            flate_encode(&channel, 6)
                .ok_or_else(|| "the cleared opacity channel could not be re-encoded".to_owned())
        })
        .transpose()?;
    Ok(ClearedSamples {
        encoded,
        written,
        opacity,
    })
}

/// Clears every sample whose centre lies in a region box under one placement: to the domain's
/// zero, or to `fill`'s integer per component where a matte decides the cleared value.
///
/// The region is inverse-mapped into image space to bound the work, then each sample centre in
/// that block is mapped forward and tested exactly — so a rotated placement clears only the
/// samples truly inside the region, and the destruction is the region's and no more.
fn clear_region(
    samples: &mut [u8],
    stride: usize,
    layout: ImageLayout,
    (ctm, regions): (Transform, &[[f32; 4]]),
    fill: Option<&[u32]>,
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
                if let Some(fill) = fill {
                    set_sample(samples, stride, layout, (row, col), fill);
                }
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

/// Writes one sample's components as `values`, each `layout.bits` wide, MSB first within the row
/// (§8.9.5.2), over a sample [`zero_sample`] has already cleared — so only the one bits are set.
fn set_sample(
    samples: &mut [u8],
    stride: usize,
    layout: ImageLayout,
    (row, col): (usize, usize),
    values: &[u32],
) {
    let per_sample = layout.components.saturating_mul(layout.bits);
    let row_base = row.saturating_mul(stride);
    for (component, value) in values.iter().enumerate().take(layout.components) {
        let first = col
            .saturating_mul(per_sample)
            .saturating_add(component.saturating_mul(layout.bits));
        for offset in 0..layout.bits {
            let shift = layout.bits.saturating_sub(1).saturating_sub(offset);
            if value
                .checked_shr(u32::try_from(shift).unwrap_or(u32::MAX))
                .unwrap_or(0)
                & 1
                == 0
            {
                continue;
            }
            let bit = first.saturating_add(offset);
            if let Some(cell) = samples.get_mut(row_base.saturating_add(bit / 8)) {
                *cell |= 0x80u8 >> (bit % 8);
            }
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
/// stream, where §7.3.8's indirect references cannot appear. A colour space the scan resolved
/// from a resource name (an `/ICCBased` space, a `/Separation` tint) is written as that name
/// again ([`colour_space_resource`]), which is how the producer stated it; a value that holds a
/// reference or a stream and that no resource name reaches is **refused by name** rather than
/// written with a reference no content stream can carry (principle 1).
fn build_inline_image(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
    encoded: &[u8],
) -> Result<Vec<u8>, String> {
    let mut out = Vec::from(&b"BI"[..]);
    for (key, value) in dict.iter() {
        match key.as_bytes() {
            // Dropped: the old encoding is replaced below. §8.9.7's `/L` and `/Length` are one
            // key, and `/DP` is `/DecodeParms`; the scan has already expanded both.
            b"Filter" | b"DecodeParms" | b"DP" | b"Length" | b"L" => continue,
            _ => {}
        }
        if key.as_bytes() == b"ColorSpace"
            && !inline_safe(value)
            && let Some(named) = colour_space_resource(document, resources, value)
        {
            // The scan resolved a `/CS` name into the resource it names; the name is what an
            // inline image may carry, and the same resources are in force where it is written.
            out.extend_from_slice(b" /ColorSpace ");
            pdf_syntax::write::object(&Object::Name(named), &mut out);
            continue;
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

/// The name under which the resources in force hold `value` as a colour space, if any.
///
/// §8.9.7 names two things an inline image's colour space may be — Table 92's device names, and
/// by a sentence of its own: "the value of the ColorSpace entry may also be the name of a colour
/// space in the ColorSpace subdictionary of the current resource dictionary".
/// [`pdf_model::inline_image::scan`] has resolved such a name into the resource's value so the
/// samples can be read, so writing the image back needs the name again: the key whose entry in
/// the current `/ColorSpace` subdictionary is that value.
#[expect(
    clippy::doc_markdown,
    reason = "the comment quotes §8.9.7 verbatim, and a quotation is not marked up"
)]
fn colour_space_resource(
    document: &Document,
    resources: &Dictionary,
    value: &Object,
) -> Option<Name> {
    let table = document.get_key(resources, "ColorSpace");
    table
        .as_dict()?
        .iter()
        .find(|(_name, entry)| *entry == value)
        .map(|(name, _entry)| name.clone())
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
    let Slots {
        replacements,
        private,
        forms: form_slots,
    } = reserve_slots(&mut assembly, applied)?;
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

    let placed_located = place_located(&mut assembly, document, applied, &private)?;
    for (index, page) in applied.iter().enumerate() {
        let copies = private.get(index).cloned().unwrap_or_default();
        let slots = form_slots.get(index);
        let carried = Carried {
            private: &copies,
            named: &slotted(&page.names.forms, slots),
            overrides: &resolved(&page.names.overrides, placed_located.get(index)),
            masks: &slotted_masks(&page.names.masks, slots),
        };
        let object = build_page(&mut assembly, document, page, &carried)?;
        assembly
            .place(page.placed, object)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
    }

    for (placed, source, index) in replacements {
        let copies = private.get(index).cloned().unwrap_or_default();
        let object = match source {
            Owned::Image(image) => build_cleared_image(&mut assembly, document, image, &copies)?,
            Owned::Form(form) => {
                let slots = form_slots.get(index);
                let carried = Carried {
                    private: &copies,
                    named: &slotted(&form.forms, slots),
                    overrides: &resolved(&form.overrides, placed_located.get(index)),
                    masks: &slotted_masks(&form.masks, slots),
                };
                build_form(&mut assembly, document, form, &carried)?
            }
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

/// The slots a redaction's replaced and copied objects take, taken before any object is built.
struct Slots<'a> {
    /// Each object to build, the slot it goes into, and the page it belongs to.
    replacements: Vec<(ObjectId, Owned<'a>, usize)>,
    /// Per page, the objects copied for it and the slots of their copies.
    private: Vec<HashMap<ObjectId, ObjectId>>,
    /// Per page, the slot each of its form edits was given.
    forms: Vec<Vec<ObjectId>>,
}

/// Takes a slot for every image and form the removal destroyed or edited, replacing an owned
/// object's own and reserving a fresh one for a copy.
fn reserve_slots<'a>(
    assembly: &mut Assembly<'_>,
    applied: &'a [AppliedPage],
) -> Result<Slots<'a>, Refusal> {
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
    let mut replacements: Vec<(ObjectId, Owned<'a>, usize)> = Vec::new();
    let mut private: Vec<HashMap<ObjectId, ObjectId>> = Vec::with_capacity(applied.len());
    // The slot each of a page's form edits was given, so a stream naming a placement's own copy
    // of a form can name the slot it went into (ADR 1363).
    let mut form_slots: Vec<Vec<ObjectId>> = Vec::with_capacity(applied.len());
    for (index, page) in applied.iter().enumerate() {
        let mut copies = HashMap::new();
        let mut slots = Vec::with_capacity(page.forms.len());
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
            // A placement's own copy stands in for the form nowhere else: only the placements
            // that name it draw it, and every other reference keeps reaching the form.
            if form.private && !form.placement_copy {
                copies.insert(form.id, slot);
            }
            slots.push(slot);
            replacements.push((slot, Owned::Form(form), index));
        }
        private.push(copies);
        form_slots.push(slots);
    }
    Ok(Slots {
        replacements,
        private,
        forms: form_slots,
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
    carried: &Carried<'_>,
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
    let restated = !carried.private.is_empty() || !page.states.is_empty() || carried.names();
    for (key, value) in source.iter() {
        let skipped = matches!(key.as_bytes(), b"Contents" | b"Annots")
            || (key.as_bytes() == b"Resources" && restated);
        if skipped {
            continue;
        }
        dict.insert(key.clone(), carry(assembly, document, value, 0));
    }
    dict.insert(Name::new(&b"Contents"[..]), Object::Reference(content_id));
    if restated {
        // §7.7.3.3 lets a page state its own `/Resources`, and §7.7.3.4's inheritance is what
        // `Page::resources` has already resolved — so writing the effective dictionary here is
        // the same resources the producer's page had, with the copied objects substituted and
        // any graphics state the edited content names added.
        let stated = with_states(document, &page.resources, &page.states);
        let resources = carried.resources(assembly, document, &stated);
        dict.insert(Name::new(&b"Resources"[..]), resources);
    }
    if let Some(annots) = surviving {
        dict.insert(Name::new(&b"Annots"[..]), annots);
    }
    Ok(Object::Dictionary(dict))
}

/// A resource dictionary with graphics state parameter dictionaries added to its `/ExtGState`.
///
/// Built in the source document's terms, before [`privatise`] carries it, so an `/ExtGState`
/// held by reference is read and restated as a direct dictionary holding every entry it had and
/// the added ones — the original object is left for whatever else names it.
fn with_states(
    document: &Document,
    resources: &Dictionary,
    states: &[(Vec<u8>, Dictionary)],
) -> Dictionary {
    if states.is_empty() {
        return resources.clone();
    }
    let mut table = document
        .get_key(resources, "ExtGState")
        .as_dict()
        .cloned()
        .unwrap_or_default();
    for (name, state) in states {
        table.insert(
            Name::new(name.as_slice()),
            Object::Dictionary(state.clone()),
        );
    }
    let mut out = resources.clone();
    out.insert(Name::new(&b"ExtGState"[..]), Object::Dictionary(table));
    out
}

/// What a replaced page or form carries its resources with: the objects copied for the page, and
/// the new objects its edited stream names (ADR 1363).
struct Carried<'a> {
    /// The objects copied for the page, by the original's number.
    private: &'a HashMap<ObjectId, ObjectId>,
    /// Form copies named in `/XObject`, with their slots.
    named: &'a [(Vec<u8>, ObjectId)],
    /// Shading and pattern entries given their placed replacements.
    overrides: &'a [(located::Category, Vec<u8>, Object)],
    /// Graphics states establishing a soft mask on a group's copy: the new name, the producer's
    /// `/ExtGState` entry, and the copy's slot.
    masks: &'a [(Vec<u8>, Object, ObjectId)],
}

impl Carried<'_> {
    /// Whether the edited stream names anything new, so its resources are written afresh.
    fn names(&self) -> bool {
        !self.named.is_empty() || !self.overrides.is_empty() || !self.masks.is_empty()
    }

    /// A stream's resources, carried into the output with the page's copies substituted, the
    /// replaced entries replaced and the new names added.
    ///
    /// `stated` is in the source document's terms; every subdictionary a name is added to or
    /// taken from is written direct first, so the change is made on this stream's own copy and
    /// the producer's subdictionary is left for whatever else names it.
    fn resources(
        &self,
        assembly: &mut Assembly<'_>,
        document: &Document,
        stated: &Dictionary,
    ) -> Object {
        let mut stated = without_overridden(document, stated, self.overrides);
        for (category, needed) in [
            ("XObject", !self.named.is_empty()),
            ("ExtGState", !self.masks.is_empty()),
        ] {
            if needed {
                let table = document
                    .get_key(&stated, category)
                    .as_dict()
                    .cloned()
                    .unwrap_or_default();
                stated.insert(Name::new(category.as_bytes()), Object::Dictionary(table));
            }
        }
        let resources = privatise(
            assembly,
            document,
            &Object::Dictionary(stated),
            self.private,
            0,
        );
        let resources = with_overrides(name_forms(resources, self.named), self.overrides);
        let Object::Dictionary(mut resources) = resources else {
            return resources;
        };
        if !self.masks.is_empty() {
            let mut table = match resources.get("ExtGState") {
                Some(Object::Dictionary(table)) => table.clone(),
                _ => Dictionary::new(),
            };
            for (name, state, slot) in self.masks {
                table.insert(
                    Name::new(name.as_slice()),
                    mask_state(assembly, document, state, *slot, self.private),
                );
            }
            resources.insert(Name::new(&b"ExtGState"[..]), Object::Dictionary(table));
        }
        Object::Dictionary(resources)
    }
}

/// The producer's graphics state dictionary restated with its soft mask's group the copy in
/// `slot`: every entry carried, and in `/SMask` every entry but `/G` (§11.6.5.1).
fn mask_state(
    assembly: &mut Assembly<'_>,
    document: &Document,
    state: &Object,
    slot: ObjectId,
    private: &HashMap<ObjectId, ObjectId>,
) -> Object {
    let resolved = document.resolve(state);
    let mut out = Dictionary::new();
    for (key, value) in resolved.as_dict().into_iter().flat_map(Dictionary::iter) {
        if key.as_bytes() == b"SMask" {
            let mask = document.resolve(value);
            let mut restated = Dictionary::new();
            for (entry, item) in mask.as_dict().into_iter().flat_map(Dictionary::iter) {
                let carried = if entry.as_bytes() == b"G" {
                    Object::Reference(slot)
                } else {
                    privatise(assembly, document, item, private, 0)
                };
                restated.insert(entry.clone(), carried);
            }
            out.insert(key.clone(), Object::Dictionary(restated));
        } else {
            out.insert(
                key.clone(),
                privatise(assembly, document, value, private, 0),
            );
        }
    }
    Object::Dictionary(out)
}

/// A stream's mask states paired with the slots their groups' copies were given.
fn slotted_masks(
    masks: &AddedMasks,
    slots: Option<&Vec<ObjectId>>,
) -> Vec<(Vec<u8>, Object, ObjectId)> {
    masks
        .iter()
        .filter_map(|(name, state, index)| {
            slots
                .and_then(|slots| slots.get(*index))
                .map(|slot| (name.clone(), state.clone(), *slot))
        })
        .collect()
}

/// Each page's destructions of located shading data, placed once and named by every resource
/// dictionary on the page that gives one of its entries the replacement (ADR 1363).
fn place_located(
    assembly: &mut Assembly<'_>,
    document: &Document,
    applied: &[AppliedPage],
    private: &[HashMap<ObjectId, ObjectId>],
) -> Result<Vec<HashMap<located::Key, Object>>, Refusal> {
    let mut out = Vec::with_capacity(applied.len());
    for (index, page) in applied.iter().enumerate() {
        let copies = private.get(index).cloned().unwrap_or_default();
        let mut placed = HashMap::new();
        for (key, built) in &page.names.located {
            placed.insert(
                key.clone(),
                place_built(assembly, document, built, &copies)?,
            );
        }
        out.push(placed);
    }
    Ok(out)
}

/// A stream's overridden entries paired with the placed replacements.
fn resolved(
    overrides: &Overrides,
    placed: Option<&HashMap<located::Key, Object>>,
) -> Vec<(located::Category, Vec<u8>, Object)> {
    overrides
        .iter()
        .filter_map(|(category, name, key)| {
            placed
                .and_then(|placed| placed.get(key))
                .map(|object| (*category, name.clone(), object.clone()))
        })
        .collect()
}

/// A resource dictionary with the overridden entries taken out of their categories, written
/// direct, in the source document's terms (like [`with_states`]), so that nothing carried names
/// the producer's object under them.
fn without_overridden(
    document: &Document,
    resources: &Dictionary,
    overrides: &[(located::Category, Vec<u8>, Object)],
) -> Dictionary {
    let mut out = resources.clone();
    for category in [located::Category::Shading, located::Category::Pattern] {
        let names: Vec<&[u8]> = overrides
            .iter()
            .filter(|(known, _, _)| *known == category)
            .map(|(_, name, _)| name.as_slice())
            .collect();
        if names.is_empty() {
            continue;
        }
        let stated = document.get_key(resources, category.key());
        let mut table = Dictionary::new();
        for (name, entry) in stated.as_dict().into_iter().flat_map(Dictionary::iter) {
            if !names.contains(&name.as_bytes()) {
                table.insert(name.clone(), entry.clone());
            }
        }
        out.insert(
            Name::new(category.key().as_bytes()),
            Object::Dictionary(table),
        );
    }
    out
}

/// A carried resource dictionary with each overridden entry given its placed replacement.
fn with_overrides(resources: Object, overrides: &[(located::Category, Vec<u8>, Object)]) -> Object {
    let Object::Dictionary(mut resources) = resources else {
        return resources;
    };
    for (category, name, object) in overrides {
        let key = Name::new(category.key().as_bytes());
        let mut table = match resources.get(category.key()) {
            Some(Object::Dictionary(table)) => table.clone(),
            _ => Dictionary::new(),
        };
        table.insert(Name::new(name.as_slice()), object.clone());
        resources.insert(key, Object::Dictionary(table));
    }
    Object::Dictionary(resources)
}

/// Places a replacement built for located shading data: the producer's values carried into the
/// output's numbering, and each object of its own added and named by its new number.
fn place_built(
    assembly: &mut Assembly<'_>,
    document: &Document,
    built: &located::Built,
    private: &HashMap<ObjectId, ObjectId>,
) -> Result<Object, Refusal> {
    let entries = |assembly: &mut Assembly<'_>, entries: &[(Name, located::Built)]| {
        let mut dict = Dictionary::new();
        for (key, value) in entries {
            dict.insert(
                key.clone(),
                place_built(assembly, document, value, private)?,
            );
        }
        Ok::<Dictionary, Refusal>(dict)
    };
    Ok(match built {
        located::Built::Source(value) => privatise(assembly, document, value, private, 0),
        located::Built::Dict(values) => Object::Dictionary(entries(assembly, values)?),
        located::Built::Array(items) => Object::Array(
            items
                .iter()
                .map(|item| place_built(assembly, document, item, private))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        located::Built::Stream(values, data) => {
            let mut dict = entries(assembly, values)?;
            dict.insert(
                Name::new(&b"Length"[..]),
                Object::Integer(i64::try_from(data.len()).unwrap_or(i64::MAX)),
            );
            Object::Stream(Arc::new(Stream {
                dict,
                data: Arc::from(data.as_slice()),
                decryption_failed: false,
            }))
        }
        located::Built::New(inner) => {
            let object = place_built(assembly, document, inner, private)?;
            Object::Reference(
                assembly
                    .add(object)
                    .map_err(|error| Refusal::Assembly(error.to_string()))?,
            )
        }
    })
}

/// A stream's added form names paired with the slots their copies were given.
fn slotted(named: &[(Vec<u8>, usize)], slots: Option<&Vec<ObjectId>>) -> Vec<(Vec<u8>, ObjectId)> {
    named
        .iter()
        .filter_map(|(name, index)| {
            slots
                .and_then(|slots| slots.get(*index))
                .map(|slot| (name.clone(), *slot))
        })
        .collect()
}

/// A carried resource dictionary with each form copy named in its `/XObject`, by the output slot
/// the copy was given (ADR 1363).
fn name_forms(resources: Object, named: &[(Vec<u8>, ObjectId)]) -> Object {
    let Object::Dictionary(mut resources) = resources else {
        return resources;
    };
    if named.is_empty() {
        return Object::Dictionary(resources);
    }
    let mut table = match resources.get("XObject") {
        Some(Object::Dictionary(table)) => table.clone(),
        _ => Dictionary::new(),
    };
    for (name, slot) in named {
        table.insert(Name::new(name.as_slice()), Object::Reference(*slot));
    }
    resources.insert(Name::new(&b"XObject"[..]), Object::Dictionary(table));
    Object::Dictionary(resources)
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
    carried: &Carried<'_>,
) -> Result<Object, Refusal> {
    let object = document.get(form.id);
    let source = object.as_stream().ok_or_else(|| {
        Refusal::Assembly(format!(
            "form object {} is not a stream where its marks must be removed",
            form.id.number
        ))
    })?;
    let mut dict = Dictionary::new();
    let restated = !form.states.is_empty() || carried.names();
    if restated {
        // The resources the form's names were resolved in, its own or the page's, gain the
        // graphics states, form copies and replaced shadings its edited content names; written
        // on the form itself, so a form that inherited the page's names now states them.
        let resources = with_states(document, &form.resources, &form.states);
        let resources = carried.resources(assembly, document, &resources);
        dict.insert(Name::new(&b"Resources"[..]), resources);
    }
    for (key, value) in source.dict.iter() {
        match key.as_bytes() {
            // The producer's encoding is dropped and §7.3.8.2's `/Length` restated: the content
            // written here is the decoded stream with the region's marks cut out of it.
            b"Filter" | b"DecodeParms" | b"DP" | b"Length" => {}
            b"Resources" if restated => {}
            _ => {
                dict.insert(
                    key.clone(),
                    privatise(assembly, document, value, carried.private, 0),
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
/// `/Decode`, colour key and masks — with references carried into the output's numbering (like
/// [`build_page`]'s entries), because the samples are still on the grid it describes; only the
/// encoding is replaced, and a mask it names is the cleared mask where the removal reached one.
/// An image whose samples a **codec's filter** delivered ([`Described::Filtered`]) keeps it the
/// same way, because those samples are the ones it describes (Table 87), with
/// `/BitsPerComponent` stated as the filter's depth. A `JPXDecode` image's own samples
/// ([`Described::Own`]) keep it too, restated where the codec had decided something
/// ([`own_dictionary`], ADRs 1333 and 1371).
fn build_cleared_image(
    assembly: &mut Assembly<'_>,
    document: &Document,
    image: &ClearedImage,
    private: &HashMap<ObjectId, ObjectId>,
) -> Result<Object, Refusal> {
    let mut dict = Dictionary::new();
    if let Described::Own { layout, own } = &image.written {
        own_dictionary(
            assembly,
            document,
            image,
            (*layout, own),
            private,
            &mut dict,
        )?;
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
                //
                // **§8.9.5.4's /Alternates goes with them** (ADR 1248). The entry is the only
                // route to a variant — "an array of alternate image dictionaries specifying
                // variant representations of the base image" — and §8.9.5.4's own selection runs
                // from the base image's array, so a base that states none is the image every
                // reader draws, printing included. Dropping the key leaves the alternates
                // reached from nowhere on this page, and the closure walk below copies only what
                // is reached, so a variant of the destroyed portion is not in the output at all:
                // §12.5.6.23's "remove all traces of the specified content", by removal rather
                // than by the re-encoding of somebody else's grid this writer cannot do. Where
                // the image is another page's too it is copied rather than replaced (ADR 1196),
                // and that page keeps its own base and its own variants — the same position that
                // page's un-destroyed samples are already in.
                b"Filter" | b"DecodeParms" | b"DP" | b"Length" | b"Alternates" => {}
                _ => {
                    dict.insert(
                        key.clone(),
                        privatise(assembly, document, value, private, 0),
                    );
                }
            }
        }
        if let Described::Filtered { bits } = image.written {
            // Table 87: `/BitsPerComponent` "shall be consistent with the size of the data
            // samples that the filter delivers", and those are the samples now written.
            dict.insert(
                Name::new(&b"BitsPerComponent"[..]),
                Object::Integer(i64::try_from(bits).unwrap_or(i64::MAX)),
            );
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

/// States a `DeviceGray` soft-mask image's grid in its dictionary (§8.9.5.1 Table 87, Table 143):
/// its type, its size, its colour space and its depth.
fn grey_dictionary(dict: &mut Dictionary, layout: ImageLayout) {
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
        Object::Name(Name::new(&b"DeviceGray"[..])),
    );
    dict.insert(
        Name::new(&b"BitsPerComponent"[..]),
        Object::Integer(i64::try_from(layout.bits).unwrap_or(i64::MAX)),
    );
}

/// A [`Described::Own`] image's dictionary: the source's, with what the codec decided restated
/// for the `FlateDecode` stream that now carries its samples (ADRs 1333, 1371).
///
/// Every entry that described the samples still does, because they are the same integers in
/// the same colour space, and is carried: a stated `/ColorSpace`, `/Intent`, `/Interpolate`, a
/// colour-key `/Mask` whose ranges are in their domain, the `/SMask` and `/Mask` streams (named
/// again in the output's numbering, the cleared copies where the removal reached them). What
/// the codec had decided and a `FlateDecode` stream must state is restated: the depth, the
/// `/Decode` pairs at that depth ([`widened_decode`]), the colour space where it was the
/// codestream's — an ICC profile carried into an `ICCBased` stream of its own — and Table 87's
/// opacity channel as the soft-mask image the table has the processor create, whose presence
/// makes the source's `/SMask` and `/Mask` entries ones §11.6.4.3 has it override, so they are
/// dropped.
fn own_dictionary(
    assembly: &mut Assembly<'_>,
    document: &Document,
    image: &ClearedImage,
    (layout, own): (ImageLayout, &OwnRaster),
    private: &HashMap<ObjectId, ObjectId>,
    dict: &mut Dictionary,
) -> Result<(), Refusal> {
    let object = document.get(image.id);
    let source = object.as_stream().ok_or_else(|| {
        Refusal::Assembly(format!(
            "image object {} is not a stream at clearing time",
            image.id.number
        ))
    })?;
    let stated = own.space == pdf_model::image::JpxSpace::Stated;
    for (key, value) in source.dict.iter() {
        let dropped = match key.as_bytes() {
            b"Filter" | b"DecodeParms" | b"DP" | b"Length" | b"Alternates"
            | b"BitsPerComponent" | b"Decode" | b"SMaskInData" => true,
            b"ColorSpace" => !stated,
            b"SMask" | b"Mask" => image.opacity.is_some(),
            _ => false,
        };
        if !dropped {
            dict.insert(
                key.clone(),
                privatise(assembly, document, value, private, 0),
            );
        }
    }
    let space = match &own.space {
        pdf_model::image::JpxSpace::Stated => None,
        pdf_model::image::JpxSpace::Gray => Some(Object::Name(Name::new(&b"DeviceGray"[..]))),
        pdf_model::image::JpxSpace::Rgb => Some(Object::Name(Name::new(&b"DeviceRGB"[..]))),
        pdf_model::image::JpxSpace::Cmyk => Some(Object::Name(Name::new(&b"DeviceCMYK"[..]))),
        pdf_model::image::JpxSpace::Lab => {
            // §8.6.5.4's `Lab`, under the D50 white point and the range the codestream's CIELab
            // samples were delivered over (ADR 1383).
            let mut parameters = Dictionary::new();
            parameters.insert(
                Name::new(&b"WhitePoint"[..]),
                reals(&[0.964_2, 1.0, 0.824_9]),
            );
            parameters.insert(
                Name::new(&b"Range"[..]),
                reals(&[-128.0, 127.0, -128.0, 127.0]),
            );
            Some(Object::Array(vec![
                Object::Name(Name::new(&b"Lab"[..])),
                Object::Dictionary(parameters),
            ]))
        }
        pdf_model::image::JpxSpace::Icc(profile) => {
            // §8.6.5.5: an `ICCBased` space is an array of the name and a stream whose `/N` is
            // the component count, holding the profile the codestream carried.
            let mut header = Dictionary::new();
            header.insert(
                Name::new(&b"N"[..]),
                Object::Integer(i64::try_from(layout.components).unwrap_or(i64::MAX)),
            );
            let placed = add_raw_stream(assembly, header, profile)?;
            Some(Object::Array(vec![
                Object::Name(Name::new(&b"ICCBased"[..])),
                Object::Reference(placed),
            ]))
        }
    };
    if let Some(space) = space {
        dict.insert(Name::new(&b"ColorSpace"[..]), space);
    }
    dict.insert(
        Name::new(&b"BitsPerComponent"[..]),
        Object::Integer(i64::try_from(own.bits).unwrap_or(i64::MAX)),
    );
    dict.insert(Name::new(&b"Decode"[..]), reals(&own.decode));
    if let Some(opacity) = &image.opacity {
        // `/SMaskInData` 2's multiplication is §11.6.5.2's pre-blending with the matte colour of
        // the space's zero: `c′ = 0 + α × (c − 0)`.
        let matte = own.premultiplied.then(|| vec![0.0; layout.components]);
        let placed = add_soft_mask(
            assembly,
            ImageLayout {
                components: 1,
                bits: own.bits,
                ..layout
            },
            opacity,
            &own.opacity_decode,
            matte.as_deref(),
        )?;
        dict.insert(Name::new(&b"SMask"[..]), Object::Reference(placed));
    }
    Ok(())
}

/// Numbers as a PDF array of reals.
fn reals(values: &[f32]) -> Object {
    Object::Array(
        values
            .iter()
            .map(|value| Object::Real(f64::from(*value)))
            .collect(),
    )
}

/// Adds §7.4.9's opacity channel as the soft-mask image Table 87 names: `DeviceGray` on the
/// `layout` it was cleared on, under `FlateDecode`, with Table 143's `/Decode` and, where the
/// colour was multiplied by it, Table 144's `/Matte`.
fn add_soft_mask(
    assembly: &mut Assembly<'_>,
    layout: ImageLayout,
    encoded: &[u8],
    decode: &[f32],
    matte: Option<&[f32]>,
) -> Result<ObjectId, Refusal> {
    let mut mask = Dictionary::new();
    grey_dictionary(&mut mask, layout);
    mask.insert(Name::new(&b"Decode"[..]), reals(decode));
    if let Some(matte) = matte {
        mask.insert(Name::new(&b"Matte"[..]), reals(matte));
    }
    add_stream(assembly, mask, encoded)
}

/// Adds a `FlateDecode` stream whose data is already encoded, stating its filter and length.
fn add_stream(
    assembly: &mut Assembly<'_>,
    mut dict: Dictionary,
    encoded: &[u8],
) -> Result<ObjectId, Refusal> {
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    add_raw_stream(assembly, dict, encoded)
}

/// Adds a stream holding `data` as it stands, stating its length.
fn add_raw_stream(
    assembly: &mut Assembly<'_>,
    mut dict: Dictionary,
    data: &[u8],
) -> Result<ObjectId, Refusal> {
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).unwrap_or(i64::MAX)),
    );
    assembly
        .add(Object::Stream(Arc::new(Stream {
            dict,
            data: Arc::from(data),
            decryption_failed: false,
        })))
        .map_err(|error| Refusal::Assembly(error.to_string()))
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
