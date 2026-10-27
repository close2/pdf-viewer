//! A page whose `Adobe-Identity` text no reader can name, photographed as the window draws it
//! (owner answer A570's second remedy, ADR 2394).
//!
//! # What is missing, and why no font answers it
//!
//! ISO 32000-2 §9.7.5.2:
//!
//! > The Identity-H and Identity-V CMaps shall not be used with a non-embedded font.
//!
//! Under `Identity` ordering a CID indexes the glyphs of the program that defined it (§9.7.4.2), and
//! that program is not in the file. Re-keying (`super::rekeyed`) answers the half whose `/ToUnicode`
//! states a character for every code; this module is the other half — no map, or a map that names
//! nothing the page shows. **It chooses no font**, because the font is not the missing piece: the
//! per-CID correspondence is, and assigning glyphs to the numbers without it would invent which
//! character each number names. So the page is photographed instead: the window's own rendering,
//! the marks it cannot draw left blank exactly as the window leaves them (ADR 0270), written as one
//! image covering the page. Its worth is salvage — the graphics, the images and every other mark
//! the window draws survive as pixels, where a refusal archives nothing.
//!
//! # The owner's gate, and how it is decided
//!
//! A570: the image is taken only where the input's text was never machine-readable anywhere, so
//! that nothing extractable is lost. The page is interpreted once, with its annotations left out,
//! because they stay annotations and keep whatever they carry; that one interpretation is both the
//! evidence for the gate and the picture that is written, so what was judged is what was drawn.
//! The page is refused by name where:
//!
//! - **its readback names any character** — §9.10.2's mapping, §14.9.4's replacement text, any font
//!   on the page: the image would turn text a reader can extract into pixels;
//! - **the interpreter reports anything but the fonts it could not draw**: a mark the window did
//!   not draw is one another reader might, and the photograph would fix its absence into the file;
//! - **the document states a structure tree** (§14.7), whose content items would be left pointing
//!   at marked content no longer on the page, **or optional content** (§8.11), whose other
//!   configurations a photograph of one would discard.
//!
//! # Where it is offered
//!
//! At Level B of ISO 19005-2 and at every flavour of ISO 19005-4, which ask no Unicode of a page;
//! Level U and Level A exist to keep a page's text, so there the font keeps the refusal it had.
//!
//! # The picture
//!
//! At [`DOTS_PER_INCH`], over §8.3.2.3's 72 units to the inch — a documented choice, since no clause
//! states a resolution for this, made so that a ten-point line of the text that does draw stays
//! legible on paper. The samples are the oracle backend's, composited onto white by §11.4.7 before
//! they leave it, and are written as `ICCBased` over the shipped sRGB profile: the colour the
//! window showed, which `super::super::default_rgb` explains is what that profile states on this
//! reader. The image is placed by the inverse of the page's own transform from default user space
//! (`pdf_model::content::base_transform`), so `/Rotate`, the crop box's origin and `/UserUnit` stay
//! the page's and the annotations keep their places over it.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::survey::SelectedFont;
use pdf_archive::{Level, Target};
use pdf_model::Pages;
use pdf_model::content::Unsupported;
use pdf_model::content::ledger::{Ledger, Outcome};
use pdf_render::{Point, Rasterizer as _, TargetSpec, Transform};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;
use render_cpu::CpuRasterizer;

use super::super::COMPRESSION_LEVEL;
use super::super::decision::Because;
use super::super::prepare::{SRGB, Spare, profile_stream};
use super::Substitutes;

/// The resolution a page is photographed at, in dots per inch.
pub const DOTS_PER_INCH: u16 = 300;

/// The name the one image goes under in the photographed page's resources.
const IMAGE: &[u8] = b"Photograph";

/// A font the window cannot draw, held until the pages that show it are known.
#[derive(Debug)]
pub(super) struct Deferred {
    /// How a content stream's lookup finds it: its object, or its dictionary as a direct entry.
    selected: Outcome,
    /// The `/BaseFont` the document states, for the report.
    requested: String,
    /// The refusal the font keeps where no page that shows it can be photographed.
    refusal: Because,
    /// Whether the font states a `/ToUnicode` stream, so that re-keying was tried and refused.
    ///
    /// Such a font's map names characters for some codes as a rule, and where those make the
    /// page's text extractable the re-keying route's own sentence says more than the gate's.
    mapped: bool,
}

/// One page this conversion photographed: what the report and the file's history name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotographedPage {
    /// The page's number, from one.
    pub page: usize,
    /// The fonts on it the window could not draw, by the `/BaseFont` each states.
    pub fonts: Vec<String>,
    /// The resolution it was photographed at.
    pub dots_per_inch: u16,
    /// The picture's width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
}

/// What a photographed page's dictionary is given in place of its content.
#[derive(Debug)]
pub(in super::super) struct Photograph {
    /// The content stream that paints the image.
    pub(in super::super) contents: ObjectId,
    /// The resource dictionary naming the image and nothing else.
    pub(in super::super) resources: Dictionary,
}

/// Whether a target admits a page photographed with no text on it: Level B of ISO 19005-2 and
/// every flavour of ISO 19005-4 (the module comment's *Where it is offered*).
pub(super) const fn offered(target: Target) -> bool {
    matches!(target, Target::Two(Level::B) | Target::Four(_))
}

/// Holds a font whose glyphs no face can be keyed to until its pages are known.
///
/// Nothing is refused here: the decision is [`photograph`]'s, once every font is in.
pub(super) fn defer(
    document: &Document,
    used: &SelectedFont,
    refusal: Because,
    substitutes: &mut Substitutes,
) {
    substitutes.deferred.push(Deferred {
        selected: super::selected(used),
        requested: super::base_font(document, &used.dict),
        refusal,
        mapped: matches!(document.get_key(&used.dict, "ToUnicode"), Object::Stream(_)),
    });
}

/// Photographs every page that shows a deferred font, or refuses the first that may not be.
///
/// `offered` is whether the target admits a page with no text at all (the module comment's
/// *Where it is offered*); where it does not, the first deferred font's own refusal stands.
///
/// # Errors
///
/// The gate's sentence for a page it refuses, a deferred font's own refusal where no page's content
/// shows it, and a sentence for a page that cannot be drawn within the budget.
pub(super) fn photograph(
    document: &Document,
    offered: bool,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<(), Because> {
    let Some(first) = substitutes.deferred.first() else {
        return Ok(());
    };
    if !offered {
        return Err(first.refusal);
    }
    let catalog = document.catalog().unwrap_or_default();
    if catalog.get("StructTreeRoot").is_some() {
        return Err(Because::NotBuiltYet(STRUCTURED));
    }
    if catalog.get("OCProperties").is_some() {
        return Err(Because::NotBuiltYet(LAYERED));
    }
    let mut reached = vec![false; substitutes.deferred.len()];
    let mut profile = None;
    for (index, page) in Pages::new(document).every().into_iter().enumerate() {
        let Some(page) = page else {
            continue;
        };
        let content = crate::render::page_to_draw(&page, None, false);
        let (interpretation, ledger) = pdf_model::content::interpret_ledgered(document, &content);
        if ledger.truncated() {
            return Err(Because::NotBuiltYet(LEDGER_TRUNCATED));
        }
        let shown = shown_here(&ledger, &substitutes.deferred);
        if shown.is_empty() {
            continue;
        }
        admitted(&interpretation, &shown, &substitutes.deferred)?;
        let id = page.id.ok_or(Because::NotBuiltYet(PAGE_NOT_AN_OBJECT))?;
        let raster = taken(&interpretation.display_list)?;
        let placed = placement(
            &content,
            interpretation.display_list.page_size.height,
            &raster,
        )?;
        let missing = Because::NotBuiltYet(NO_SPARE_OBJECT);
        let icc = if let Some(icc) = profile {
            icc
        } else {
            let at = spare.take(document).ok_or(missing)?;
            substitutes.written.insert(
                at,
                profile_stream(SRGB, pdf_archive::survey::DeviceFamily::Rgb),
            );
            profile = Some(at);
            at
        };
        let photograph = written(document, &raster, icc, placed, spare, substitutes)?;
        substitutes.photographs.insert(id, photograph);
        for &font in &shown {
            if let Some(seen) = reached.get_mut(font) {
                *seen = true;
            }
        }
        substitutes.photographed.push(PhotographedPage {
            page: index.saturating_add(1),
            fonts: shown
                .iter()
                .filter_map(|&font| substitutes.deferred.get(font))
                .map(|font| font.requested.clone())
                .collect(),
            dots_per_inch: DOTS_PER_INCH,
            width: raster.width,
            height: raster.height,
        });
    }
    // A font no page's content showed was reached some other way — an annotation's appearance is
    // the one the content-only interpretation leaves out — and a photograph of a page cannot take
    // it out of the file, so it keeps the refusal it had.
    if let Some((font, _)) = substitutes
        .deferred
        .iter()
        .zip(&reached)
        .find(|(_, reached)| !**reached)
    {
        return Err(font.refusal);
    }
    // A substituted font only the photographed content selected went with that content, so the
    // report and the file's history name only the fonts the output still holds (`doc/adr/2454`).
    if !substitutes.photographs.is_empty() {
        let held = still_held(document, &substitutes.photographs, &substitutes.done);
        substitutes.done.retain(|font| held.contains(&font.font));
    }
    Ok(())
}

/// The keys a photographed page's dictionary loses, whose values the output therefore does not
/// reach through it: what [`Photograph`] replaces, and the group `rewrite` removes with them.
const PHOTOGRAPHED_AWAY: [&str; 3] = ["Contents", "Resources", "Group"];

/// Which of the substituted fonts an object the output holds still names, once every
/// photographed page has lost its content and its resources.
///
/// **Reached, the way the writer reaches what it copies**: `super::super::rewrite::convert` writes
/// what the catalog and the information dictionary lead to, so a font whose every route ran
/// through a photographed page's own resources is in no object of the output, while one an
/// annotation's appearance, another page or an inherited resource dictionary names is still
/// there, its face with it. The walk is over the source's objects, each read once, with an
/// explicit stack rather than recursion, because a document's graph is the file's to shape.
fn still_held(
    document: &Document,
    photographs: &BTreeMap<ObjectId, Photograph>,
    fonts: &[super::SubstitutedFont],
) -> BTreeSet<Outcome> {
    let direct: BTreeSet<&String> = fonts
        .iter()
        .filter_map(|font| match &font.font {
            Outcome::Direct(spelled) => Some(spelled),
            _ => None,
        })
        .collect();
    let wanted: BTreeSet<&Outcome> = fonts.iter().map(|font| &font.font).collect();
    let trailer = document.trailer();
    let mut pending: Vec<Object> = ["Root", "Info"]
        .into_iter()
        .filter_map(|key| trailer.get(key).cloned())
        .collect();
    let mut seen = BTreeSet::new();
    let mut held = BTreeSet::new();
    while let Some(object) = pending.pop() {
        match object {
            Object::Reference(id) => {
                if !seen.insert(id) {
                    continue;
                }
                let reached = Outcome::Reference(id);
                if wanted.contains(&reached) {
                    held.insert(reached);
                }
                let mut value = document.get(id);
                if photographs.contains_key(&id)
                    && let Object::Dictionary(page) = &mut value
                {
                    for key in PHOTOGRAPHED_AWAY {
                        page.remove(key);
                    }
                }
                pending.push(value);
            }
            Object::Dictionary(dict) => {
                // A direct font has no identity but its value (`Outcome::Direct`), so a dictionary
                // that could be one is spelled out only while a direct font is still looked for.
                if !direct.is_empty() && dict.get("Subtype").is_some() {
                    let spelled = format!("{:?}", Object::Dictionary(dict.clone()));
                    if direct.contains(&spelled) {
                        held.insert(Outcome::Direct(spelled));
                    }
                }
                pending.extend(dict.iter().map(|(_, value)| value.clone()));
            }
            Object::Array(items) => pending.extend(items),
            Object::Stream(stream) => pending.push(Object::Dictionary(stream.dict.clone())),
            _ => {}
        }
    }
    held
}

/// The owner's gate over one page's content-only interpretation: nothing a reader can extract,
/// and nothing the window left undrawn but the fonts it could not draw.
fn admitted(
    interpretation: &pdf_model::content::Interpretation,
    shown: &[usize],
    deferred: &[Deferred],
) -> Result<(), Because> {
    if interpretation.text.chars().any(|c| !c.is_whitespace()) {
        let keyed = shown
            .iter()
            .filter_map(|&font| deferred.get(font))
            .find(|font| font.mapped);
        return Err(keyed.map_or(Because::NotBuiltYet(TEXT_IS_READABLE), |font| font.refusal));
    }
    if interpretation
        .unsupported
        .iter()
        .any(|report| !matches!(report, Unsupported::Font { .. } | Unsupported::Text { .. }))
    {
        return Err(Because::NotBuiltYet(NOT_DRAWN_WHOLE));
    }
    Ok(())
}

/// The page's display list drawn by the oracle backend at [`DOTS_PER_INCH`].
fn taken(list: &pdf_render::DisplayList) -> Result<pdf_render::Raster, Because> {
    let target = TargetSpec::for_page(list, scale(), crate::Budget::default().max_pixels)
        .map_err(|_| Because::NotBuiltYet(PAGE_TOO_LARGE))?;
    CpuRasterizer::new()
        .rasterize(list, target)
        .map_err(|_| Because::NotBuiltYet(PAGE_NOT_DRAWN))
}

/// Device pixels to a unit of the display list's space, §8.3.2.3's 72 to the inch.
fn scale() -> f32 {
    f32::from(DOTS_PER_INCH) / 72.0
}

/// The image and the content stream painting it, written as new objects, and the resources that
/// name the image.
fn written(
    document: &Document,
    raster: &pdf_render::Raster,
    icc: ObjectId,
    placed: Transform,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<Photograph, Because> {
    let missing = Because::NotBuiltYet(NO_SPARE_OBJECT);
    let image = spare.take(document).ok_or(missing)?;
    let contents = spare.take(document).ok_or(missing)?;
    substitutes
        .written
        .insert(image, image_stream(raster, icc).ok_or(missing)?);
    substitutes
        .written
        .insert(contents, contents_stream(placed).ok_or(missing)?);
    let mut images = Dictionary::new();
    images.insert(Name::new(IMAGE), Object::Reference(image));
    let mut resources = Dictionary::new();
    resources.insert(Name::new(&b"XObject"[..]), Object::Dictionary(images));
    Ok(Photograph {
        contents,
        resources,
    })
}

/// The deferred fonts a page's content selected, by their place in the list.
fn shown_here(ledger: &Ledger, deferred: &[Deferred]) -> Vec<usize> {
    deferred
        .iter()
        .enumerate()
        .filter(|(_, font)| {
            ledger.selections().any(|(selection, _)| {
                selection.category == "Font" && selection.outcome == font.selected
            })
        })
        .map(|(index, _)| index)
        .collect()
}

/// The matrix that puts the image's unit square where the raster's pixels were, in default user
/// space.
///
/// `TargetSpec::for_page` anchors the raster at the top-left corner of the display list's page and
/// lets the fraction of a pixel it rounds up to fall off the right and the bottom edges, so the
/// image is `width / scale` by `height / scale` units of the list's space with its top edge at the
/// page's: §8.9.4's image space then reaches the list through `[w 0 0 h 0 top − h]`, and default
/// user space through the inverse of the page's own transform.
fn placement(
    page: &pdf_model::Page,
    top: f32,
    raster: &pdf_render::Raster,
) -> Result<Transform, Because> {
    let scale = scale();
    #[expect(
        clippy::cast_precision_loss,
        reason = "a raster's extent is bounded by the pixel budget, far inside f32's exact integers"
    )]
    let (across, down) = (raster.width as f32 / scale, raster.height as f32 / scale);
    let into_list = Transform::new(across, 0.0, 0.0, down, 0.0, top - down);
    let into_user = pdf_model::content::base_transform(page)
        .invert()
        .ok_or(Because::NotBuiltYet(PAGE_NOT_DRAWN))?;
    let placed = into_list.then(into_user);
    // A quarter turn's inverse is exact in principle and a few ulps off in `f32`; the corner is
    // checked to be finite so that no NaN reaches the content stream.
    let corner = placed.apply(Point::new(1.0, 1.0));
    if corner.x.is_finite() && corner.y.is_finite() {
        Ok(placed)
    } else {
        Err(Because::NotBuiltYet(PAGE_NOT_DRAWN))
    }
}

/// The image `XObject`: the raster's colour samples, deflated, in the sRGB `ICCBased` space.
///
/// The alpha is dropped because every sample's is 255: §11.4.7 composited the page onto white
/// before the raster left the backend (`crate::render::encode` relies on the same fact).
fn image_stream(raster: &pdf_render::Raster, space: ObjectId) -> Option<Object> {
    let samples: Vec<u8> = raster
        .data
        .chunks_exact(4)
        .flat_map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    let data = flate_encode(&samples, COMPRESSION_LEVEL)?;
    let mut dict = Dictionary::new();
    let name = |word: &[u8]| Object::Name(Name::new(word));
    dict.insert(Name::new(&b"Type"[..]), name(b"XObject"));
    dict.insert(Name::new(&b"Subtype"[..]), name(b"Image"));
    dict.insert(
        Name::new(&b"Width"[..]),
        Object::Integer(i64::from(raster.width)),
    );
    dict.insert(
        Name::new(&b"Height"[..]),
        Object::Integer(i64::from(raster.height)),
    );
    dict.insert(
        Name::new(&b"ColorSpace"[..]),
        Object::Array(vec![name(b"ICCBased"), Object::Reference(space)]),
    );
    dict.insert(Name::new(&b"BitsPerComponent"[..]), Object::Integer(8));
    dict.insert(Name::new(&b"Filter"[..]), name(b"FlateDecode"));
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).ok()?),
    );
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    })))
}

/// The content stream that paints the image where [`placement`] puts it, and nothing else.
fn contents_stream(placed: Transform) -> Option<Object> {
    let operands = [placed.a, placed.b, placed.c, placed.d, placed.e, placed.f]
        .map(number)
        .join(" ");
    let image = std::str::from_utf8(IMAGE).ok()?;
    let text = format!("q {operands} cm /{image} Do Q\n");
    let data = flate_encode(text.as_bytes(), COMPRESSION_LEVEL)?;
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).ok()?),
    );
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    })))
}

/// One operand as §7.3.3 writes a real: four places, which is under a thousandth of a pixel at
/// [`DOTS_PER_INCH`], with the trailing zeros and a negative zero taken off.
fn number(value: f32) -> String {
    let written = format!("{value:.4}");
    let trimmed = written.trim_end_matches('0').trim_end_matches('.');
    match trimmed {
        "-0" | "" => "0".to_owned(),
        other => other.to_owned(),
    }
}

/// Every photographed page, as the file's `xmpMM:History` states it.
pub(super) fn recorded(pages: &[PhotographedPage]) -> String {
    pages
        .iter()
        .map(|page| {
            format!(
                "page {} was replaced by a photograph of itself at {} dpi, the text of {} that no \
                 reader can name left blank as this viewer drew it, losing text-as-image",
                page.page,
                page.dots_per_inch,
                page.fonts.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The gate: a page whose text a reader can already extract.
const TEXT_IS_READABLE: &str = "this document renders an Adobe-Identity composite font it does not \
     embed, whose codes no ToUnicode CMap names, on a page that also shows text a reader can \
     extract; a photograph of the page would turn that text into pixels, so the page is not \
     photographed. Name the producer's font program with --font to keep the page as text";

/// The gate: a page the window does not draw whole.
const NOT_DRAWN_WHOLE: &str = "this document renders an Adobe-Identity composite font it does not \
     embed, whose codes no ToUnicode CMap names, on a page whose content this reader does not draw \
     whole; a photograph would fix what the window could not draw into the file, so the page is not \
     photographed. Name the producer's font program with --font";

/// The gate: a structure tree a photograph would leave pointing at nothing.
const STRUCTURED: &str = "this document renders an Adobe-Identity composite font it does not \
     embed, whose codes no ToUnicode CMap names, and states a structure tree (ISO 32000-2 \
     section 14.7) whose content items a photographed page would leave pointing at marked content \
     no longer on it. Name the producer's font program with --font";

/// The gate: optional content a photograph of one configuration would discard.
const LAYERED: &str = "this document renders an Adobe-Identity composite font it does not embed, \
     whose codes no ToUnicode CMap names, and states optional content (ISO 32000-2 section 8.11), \
     whose other configurations a photograph of the one this viewer shows would discard. Name the \
     producer's font program with --font";

/// The ledger stopped before the page's lookups were all kept.
const LEDGER_TRUNCATED: &str = "a page of this document names so many resources that the record of \
     which fonts it shows stopped short, so whether it shows the Adobe-Identity composite font no \
     reader can name is not known and no page is photographed";

/// The page has no object to give new content to.
const PAGE_NOT_AN_OBJECT: &str = "a page showing an Adobe-Identity composite font no reader can \
     name is one this reader recovered rather than one the page tree states as an object, so there \
     is no page object to give the photograph to";

/// The picture would pass the pixel budget.
const PAGE_TOO_LARGE: &str = "a page showing an Adobe-Identity composite font no reader can name \
     is too large to be photographed at the resolution a page is photographed at within the pixel \
     budget a rendered page is held to";

/// The backend refused the page, or its transform has no inverse.
const PAGE_NOT_DRAWN: &str = "a page showing an Adobe-Identity composite font no reader can name \
     could not be drawn to be photographed: the oracle backend refused it, or the page's own \
     transform from user space has no inverse";

/// No object number was free.
const NO_SPARE_OBJECT: &str = "no unused object number could be found for the photograph of a page \
     this conversion would have written";

#[cfg(test)]
mod tests {
    use super::number;

    #[test]
    fn an_operand_is_written_without_its_trailing_zeros() {
        assert_eq!(number(612.0), "612");
        assert_eq!(number(0.25), "0.25");
        assert_eq!(number(-0.000_01), "0");
        assert_eq!(number(-1.5), "-1.5");
    }
}
