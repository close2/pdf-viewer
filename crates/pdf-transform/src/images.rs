//! `images` — the images a document embeds, as files, RFC 0002 section 6.3.
//!
//! # What is enumerated
//!
//! Two populations, and the report says which each image came from.
//!
//! The image `XObject`s reachable from a selected page's resources (ISO 32000-2 §7.8.3, the
//! `/XObject` sub-dictionary of Table 34), descending through form `XObject`s' own resources
//! (§8.10.2) because a scanned page very often wraps its one image in a form. An image placed on
//! forty pages is one object and is extracted **once**, on the first selected page that reaches
//! it — RFC 0002 section 6.3's proposal — and the report says which page that was. A form is
//! walked once for the same reason.
//!
//! **Inline images** (§8.9.7), which are not objects at all: "an inline image object … shall be
//! delimited in the content stream by the operators `BI` (begin image), `ID` (image data) and
//! `EI` (end image)". The page's content and each reached form's content are lexed for `BI`,
//! and what follows is read by `pdf_model::inline_image::scan` — the interpreter's own reader,
//! which hands back the `Stream` the same image would have been as an `XObject`, key for key
//! (Table 91's abbreviations expanded) — so there is one inline-image reader in the tree and
//! this is not a second. An inline image is listed at every placement, because each `BI` is its
//! own data; it has no object number, and the entry says `inline`. An image `XObject` that is a
//! stencil mask (`/ImageMask true`, §8.9.6.2) is decoded painting black through its set bits,
//! which is what a stencil is with no fill colour to take from.
//!
//! # What is written
//!
//! Decoded PNG by default, because it is the answer that always works: `pdf_model::image::decode`
//! produces straight-alpha RGBA8 for every image an interpreted page can place — JBIG2, JPX and
//! CCITT through the confined `pdf-sandbox` worker, exactly as in the viewer — with the image's
//! soft mask composited into the alpha. A build without the worker beside it refuses those images
//! **by name** rather than falling back in-process (RFC 0002 section 8).
//!
//! [`ImagesPlan::format`] chooses the file form of every image that is *decoded*: PNG, or the
//! two netpbm forms `render` also writes — PPM, the RGB with no alpha, and PGM, the grey of
//! §10.4.2.2's conversion over the decoded RGB, which is [`crate::render::ImageFormat::Pgm`]'s
//! stated choice and is one conversion for a page and an image alike. A format is a statement
//! about decoded samples and never about a native stream: under `--native` a JPEG is the JPEG
//! whatever its own colour model — a grey one is grey already, a CMYK one stays CMYK — and a
//! person who wants the grey of a CMYK JPEG asks for it decoded, where the interpreter's own
//! conversion takes it to RGB first. Neither netpbm form holds an alpha, so under either the
//! mask is written beside the image, as the native route already does.
//!
//! **`--native`** ([`ImagesPlan::native`]) writes the embedded codec stream as it is where the
//! codec has a standalone file form, and decoded PNG otherwise, per image:
//!
//! - `DCTDecode` (§7.4.8) as `.jpg`: the filter's data is "encoded in the JPEG baseline format"
//!   and is a JPEG interchange stream on its own.
//! - `JPXDecode` (§7.4.9) as `.jp2`: the filter "shall expect to read a full JPX file
//!   structure", so the bytes are a file already. `.jp2` rather than `.jpx` is a choice:
//!   JP2 is the subset every reader opens, and a JPX file opens under it too.
//! - `JBIG2Decode` and `CCITTFaxDecode` are decoded to PNG **and the report says so**, because
//!   neither is a file: a JBIG2 embedded stream has its globals in a second stream (Table 12's
//!   `/JBIG2Globals`) and a CCITT stream has its parameters in the dictionary, and inventing
//!   sidecar formats for them is what RFC 0002 section 6.3 declined.
//!
//! The bytes are those after every filter *in front of* the codec has run —
//! `pdf_syntax::Document::image_stream`, the same call the interpreter makes — so a
//! `[/FlateDecode /DCTDecode]` chain yields the JPEG. What the native form loses is stated
//! rather than hidden: the dictionary's `/Decode` array is not in the codestream, so a JPEG
//! written natively is the JPEG, not the image as the page draws it. Under `--native` the
//! format's extension is appended to the expanded name, because the caller cannot know which
//! of three it will be.
//!
//! # The mask, beside the image or inside it
//!
//! ISO 32000-2 §8.9.6.1 names the ways an image is masked, and two of them are a second image:
//! §8.9.6.3's explicit mask, "a separate image `XObject` which shall be used as an explicit mask
//! specifying which areas of the image to paint and which to mask out", and §11.6.5.2's soft
//! mask through `/SMask`. §8.9.6.3 says what the two images are to each other — "[t]he base
//! image and the image mask need not have the same resolution (Width and Height values), but
//! since all images shall be defined on the unit square in user space, their boundaries on the
//! page will coincide" — so a mask is an image on its own grid, and the page's picture is the
//! base seen through it. Three outputs follow, and the plan decides between them:
//!
//! - **The composite**, a PNG whose alpha is the mask resampled onto the base's grid: the
//!   default on the decoded route, because it is the image as the page draws it, and the one
//!   file form that can carry it.
//! - **The base as it is, with the mask beside it** as `<name>.mask.png`, an 8-bit grey image
//!   on the mask's own grid whose value is the opacity it gives the base — a soft mask's
//!   samples, or an explicit mask's painted places as 255. This is what [`ImagesPlan::no_mask`]
//!   asks for on the decoded route, and it is what the native route **always** does, because a
//!   JPEG or a JP2 without opacity in its codestream has nowhere to put a mask and dropping it
//!   was the one silent loss this verb had (trap 5).
//! - **Nothing beside it** where the mask is not an image: §8.9.6.4's colour key is a range of
//!   sample values, and §11.6.5.2's `/SMaskInData` is opacity that travelled in the codestream.
//!   Both are said in the report rather than written as a file that would have to be invented.
//!
//! The base written without its mask is the same stream decoded as if it stated no `/SMask`
//! and no `/Mask`: `pdf_model::image::decode` over a copy of the dictionary with the two
//! entries removed, which is one decoder rather than a second route through it. ADR 0802.

use std::collections::BTreeSet;
use std::io::Write as _;
use std::sync::Arc;

use pdf_model::page_label::PageLabels;
use pdf_model::{Page, Pages};
use pdf_syntax::{Dictionary, Document, Lexer, Object, ObjectId, Stream, Token};
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::json::Value;
use crate::pattern::{Fill, Pattern};
use crate::range::Selection;
use crate::render::ImageFormat;
use crate::{Declined, Listed, Origin, Output, Refusal, Report, Sinks, Warning};

/// The images a document embeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagesPlan {
    /// Which source.
    pub source: usize,
    /// Which pages to look on, in which order.
    pub pages: Selection,
    /// Images with fewer samples than this are left out — the icon and the rule, in a
    /// document whose pictures are what was asked for. Zero keeps everything.
    pub min_pixels: u64,
    /// Inventory only: nothing is decoded, nothing is written.
    pub list_only: bool,
    /// Write the embedded codec stream as it is where it is a file on its own — DCT as
    /// `.jpg`, JPX as `.jp2` — and decoded PNG otherwise, saying so. The module comment has
    /// what the native form does and does not carry.
    pub native: bool,
    /// Keep a mask out of the image's alpha and write it beside the image as its own file
    /// instead. The module comment has the three outputs this chooses between.
    pub no_mask: bool,
    /// The file form of every image that is decoded. Says nothing about a native stream, which
    /// is written as it is; the module comment says why.
    pub format: ImageFormat,
    /// How the outputs are named.
    pub names: Pattern,
}

/// One image, as the inventory describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageEntry {
    /// Which source.
    pub source: usize,
    /// The first selected page that reaches it, counted from 1.
    pub page: usize,
    /// That page's §12.4.2 label, where the document states one.
    pub label: Option<String>,
    /// The object it is, where it is an indirect object.
    pub object: Option<String>,
    /// Whether it is §8.9.7's inline image, written into the content stream itself.
    pub inline: bool,
    /// Table 87's `/Width`.
    pub width: u32,
    /// Table 87's `/Height`.
    pub height: u32,
    /// Table 87's `/BitsPerComponent`, where stated.
    pub bits_per_component: Option<i64>,
    /// The family name of Table 87's `/ColorSpace`, where stated: a name, or an array's first
    /// element.
    pub colour_space: Option<String>,
    /// Table 5's `/Filter` names, in order.
    pub filters: Vec<String>,
    /// Whether it is §8.9.6.2's stencil mask.
    pub stencil: bool,
    /// Whether it carries §11.6.5.2's `/SMask` or §8.9.6.4's `/Mask`.
    pub masked: bool,
}

impl ImageEntry {
    /// The entry as JSON.
    pub(crate) fn to_json(&self) -> Value {
        Value::Object(vec![
            ("kind".to_owned(), Value::text("image")),
            ("source".to_owned(), Value::count(self.source)),
            ("page".to_owned(), Value::count(self.page)),
            ("label".to_owned(), Value::optional(self.label.clone())),
            ("object".to_owned(), Value::optional(self.object.clone())),
            ("inline".to_owned(), Value::Bool(self.inline)),
            ("width".to_owned(), Value::Integer(i64::from(self.width))),
            ("height".to_owned(), Value::Integer(i64::from(self.height))),
            (
                "bits_per_component".to_owned(),
                self.bits_per_component.map_or(Value::Null, Value::Integer),
            ),
            (
                "colour_space".to_owned(),
                Value::optional(self.colour_space.clone()),
            ),
            (
                "filters".to_owned(),
                Value::Array(self.filters.iter().cloned().map(Value::Text).collect()),
            ),
            ("stencil".to_owned(), Value::Bool(self.stencil)),
            ("masked".to_owned(), Value::Bool(self.masked)),
        ])
    }
}

/// The file format one image is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFile {
    /// Decoded, RGBA8 with the soft mask in the alpha.
    Png,
    /// Decoded, the RGB with no alpha — `P6`.
    Ppm,
    /// Decoded, §10.4.2.2's grey of the RGB — `P5`.
    Pgm,
    /// §7.4.8's stream as it is.
    Jpeg,
    /// §7.4.9's stream as it is.
    Jp2,
    /// An image's mask on its own grid, as an 8-bit grey PNG: the opacity each sample gives
    /// the base image it is written beside.
    Mask,
    /// The same mask as a `P5` PGM, beside an image written in a netpbm form: a mask is one
    /// channel, so its netpbm form is the grey one whether the image beside it is grey or RGB.
    MaskPgm,
}

impl ImageFile {
    /// The conventional file extension.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Ppm => "ppm",
            Self::Pgm => "pgm",
            Self::Jpeg => "jpg",
            Self::Jp2 => "jp2",
            Self::Mask => "mask.png",
            Self::MaskPgm => "mask.pgm",
        }
    }

    /// The file a decoded image is written as, in this format.
    #[must_use]
    pub fn decoded(format: ImageFormat) -> Self {
        match format {
            ImageFormat::Png => Self::Png,
            ImageFormat::Ppm => Self::Ppm,
            ImageFormat::Pgm => Self::Pgm,
        }
    }

    /// The file a mask is written as, beside an image written in this format.
    #[must_use]
    pub fn mask(format: ImageFormat) -> Self {
        match format {
            ImageFormat::Png => Self::Mask,
            ImageFormat::Ppm | ImageFormat::Pgm => Self::MaskPgm,
        }
    }
}

/// Which route one image's bytes take to its file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    /// Through `pdf_model::image::decode`, then the encoder for the format.
    Decoded(ImageFormat),
    /// §7.4.8's stream as it is.
    Jpeg,
    /// §7.4.9's stream as it is.
    Jp2,
}

impl Route {
    /// The file the route ends in.
    fn file(self) -> ImageFile {
        match self {
            Self::Decoded(format) => ImageFile::decoded(format),
            Self::Jpeg => ImageFile::Jpeg,
            Self::Jp2 => ImageFile::Jp2,
        }
    }

    /// Whether the file can carry the mask in its alpha; only a decoded PNG can.
    fn holds_alpha(self) -> bool {
        match self {
            Self::Decoded(format) => format.holds_alpha(),
            Self::Jpeg | Self::Jp2 => false,
        }
    }
}

/// The name a mask is written under beside an image called `name`: the name with its file
/// extension, if it has one, replaced by the mask's — so `img-3.png` and `img-3.jpg` both put
/// their mask at `img-3.mask.png`, and `img-3` does too; beside a netpbm image it is
/// `img-3.mask.pgm`.
#[must_use]
pub fn mask_name(name: &str, format: ImageFormat) -> String {
    let file_starts = name.rfind('/').map_or(0, |slash| slash.saturating_add(1));
    let stem = match name[file_starts..].rfind('.') {
        Some(dot) if dot > 0 => &name[..file_starts.saturating_add(dot)],
        _ => name,
    };
    format!("{stem}.{}", ImageFile::mask(format).extension())
}

/// One image found on a page: what decoding it needs.
#[derive(Debug)]
struct Found {
    /// The inventory entry.
    entry: ImageEntry,
    /// The stream — an object's, or the one §8.9.7's reader built from the content.
    stream: Arc<Stream>,
    /// The resources it was found under, which is what a colour space named by resource name
    /// resolves against (§8.6.3's `/ColorSpace` sub-dictionary): the page's for an `XObject`,
    /// and for an inline image the resources of the content it was written in.
    resources: Dictionary,
}

/// Runs the verb.
pub(crate) fn run(
    plan: &ImagesPlan,
    document: &Document,
    sinks: &dyn Sinks,
    report: &mut Report,
) -> Result<(), Refusal> {
    let pages = Pages::new(document);
    let labels = PageLabels::read(document);
    let selected = plan
        .pages
        .resolve(pages.len(), |index| labels.label(index))
        .map_err(|error| Refusal::Selection {
            at: plan.source,
            error,
        })?;
    if plan.names.names_a_title() {
        return Err(Refusal::Pattern(
            "%t names a title, and an image has none".to_owned(),
        ));
    }

    let found = enumerate(
        document,
        &pages,
        &labels,
        &selected,
        plan,
        &mut report.warnings,
    );
    if plan.list_only {
        report
            .listed
            .extend(found.into_iter().map(|found| Listed::Image(found.entry)));
        return Ok(());
    }
    if !plan.names.distinguishes(found.len()) {
        return Err(Refusal::Pattern(format!(
            "{} images would be written and the output name {:?} has no %d to tell them apart",
            found.len(),
            plan.names.to_string()
        )));
    }

    let count = found.len();
    let outcomes: Vec<Written> = found
        .par_iter()
        .enumerate()
        .map(|(at, found)| write_one(plan, document, sinks, at, count, found))
        .collect();
    for Written { outcomes, warnings } in outcomes {
        report.warnings.extend(warnings);
        for outcome in outcomes {
            match outcome {
                Ok(output) => report.outputs.push(output),
                Err(Problem::Declined(declined)) => report.refused.push(declined),
                Err(Problem::Sink(name, error)) => return Err(Refusal::Sink { name, error }),
            }
        }
    }
    Ok(())
}

/// What writing one image produced: the image, and its mask where one is written beside it.
struct Written {
    /// Each output, or why not — the image first, then its mask.
    outcomes: Vec<Result<Output, Problem>>,
    /// What could not be done for it, where something could not.
    warnings: Vec<Warning>,
}

/// Why an image was not written: this program declined it (exit 4, the others still written),
/// or the sink failed (exit 2, the run ends).
enum Problem {
    /// Refused by name — the codec worker missing, the image malformed.
    Declined(Declined),
    /// The sink could not be opened or written.
    Sink(String, std::io::Error),
}

/// Decides the route for one image under the plan, with the reason where native was asked
/// for and cannot be given.
fn route_for(plan: &ImagesPlan, document: &Document, found: &Found) -> (Route, Option<String>) {
    let decoded = Route::Decoded(plan.format);
    if !plan.native {
        return (decoded, None);
    }
    match document.image_codec(&found.stream).as_deref() {
        Some(b"DCTDecode" | b"DCT") => (Route::Jpeg, None),
        Some(b"JPXDecode") => (Route::Jp2, None),
        Some(codec) => (
            decoded,
            Some(format!(
                "{} has no standalone file form, so the image was decoded to {}",
                String::from_utf8_lossy(codec),
                plan.format.extension().to_ascii_uppercase()
            )),
        ),
        None => (decoded, None),
    }
}

/// Writes the `at`-th of `count` found images, as the plan says, and its mask beside it
/// where the plan or the file form keeps the two apart.
fn write_one(
    plan: &ImagesPlan,
    document: &Document,
    sinks: &dyn Sinks,
    at: usize,
    count: usize,
    found: &Found,
) -> Written {
    let (route, reason) = route_for(plan, document, found);
    let expanded = plan.names.expand(&Fill {
        ordinal: at.saturating_add(1),
        count,
        page: Some(found.entry.page),
        label: found.entry.label.as_deref(),
        title: None,
    });
    let name = if plan.native {
        format!("{}.{}", expanded.name, route.file().extension())
    } else {
        expanded.name
    };
    let page = Some(found.entry.page);
    let mut warnings: Vec<Warning> = reason
        .map(|reason| Warning {
            source: plan.source,
            page,
            detail: format!("{name}: {reason}"),
        })
        .into_iter()
        .collect();
    // The mask stays out of the image where the plan says so, and where the file form could
    // not hold it anyway: a JPEG has no alpha, a JP2 without §11.6.5.2's `/SMaskInData` has
    // none either, and a netpbm file is its samples. The module comment has the three outputs.
    let separate = plan.no_mask || !route.holds_alpha();
    let mut outcomes = vec![write_as(
        plan,
        document,
        sinks,
        found,
        &name,
        route,
        separate,
        expanded.sanitised,
        &mut warnings,
    )];
    if separate {
        match mask_of(document, found) {
            Ok(Some(mask)) => {
                if let Some(detail) = &mask.shortfall {
                    warnings.push(Warning {
                        source: plan.source,
                        page,
                        detail: format!("{}: {detail}", mask_name(&name, plan.format)),
                    });
                }
                outcomes.push(write_mask(
                    plan,
                    sinks,
                    found,
                    &mask_name(&name, plan.format),
                    &mask,
                    expanded.sanitised,
                ));
            }
            Ok(None) => {}
            Err(reason) => warnings.push(Warning {
                source: plan.source,
                page,
                detail: format!("{name}: {reason}"),
            }),
        }
    }
    if separate && !plan.native && smask_in_data(document, &found.stream) {
        warnings.push(Warning {
            source: plan.source,
            page,
            detail: format!(
                "{name}: the opacity travelled in the codestream (§11.6.5.2's /SMaskInData) and \
                 stays in the PNG's alpha, since it has no existence apart from the samples"
            ),
        });
    }
    Written { outcomes, warnings }
}

/// Produces the bytes of one image in one file form and writes them.
///
/// `unmasked` decodes the PNG route's image as if its dictionary stated no `/SMask` and no
/// `/Mask`, which is what "the base as it is" means; the native route never applies one.
#[expect(
    clippy::too_many_arguments,
    reason = "one call site, and a struct for nine things used once would name the same nine"
)]
fn write_as(
    plan: &ImagesPlan,
    document: &Document,
    sinks: &dyn Sinks,
    found: &Found,
    name: &str,
    route: Route,
    unmasked: bool,
    sanitised: bool,
    warnings: &mut Vec<Warning>,
) -> Result<Output, Problem> {
    let declined = |detail: String| {
        Problem::Declined(Declined {
            source: plan.source,
            page: Some(found.entry.page),
            subject: name.to_owned(),
            detail,
        })
    };
    let sink_failed = |error: std::io::Error| Problem::Sink(name.to_owned(), error);
    let (bytes, width, height): (Arc<[u8]>, u32, u32) = match route {
        Route::Decoded(format) => {
            let unmasked_stream;
            let stream: &Stream = if unmasked {
                unmasked_stream = without_masks(&found.stream);
                &unmasked_stream
            } else {
                &found.stream
            };
            let pdf_model::image::Flattened { image, shortfall } =
                decode_image(document, stream, &found.resources)
                    .map_err(|error| declined(error.to_string()))?;
            // The filter stopped on damaged data and delivered the rows before it: the file
            // written holds those rows over an unpainted remainder, which the page would draw
            // the same way, and the report says so where the page's own report would (ADR 0794).
            if let Some(detail) = shortfall {
                warnings.push(Warning {
                    source: plan.source,
                    page: Some(found.entry.page),
                    detail: format!("{name}: {detail}"),
                });
            }
            let raster = pdf_render::Raster {
                width: image.width,
                height: image.height,
                format: pdf_render::RasterFormat::Rgba8,
                data: image.data.to_vec(),
            };
            let encoded = crate::render::encode(&raster, format)
                .map_err(|error| sink_failed(std::io::Error::other(error)))?;
            (Arc::from(encoded), image.width, image.height)
        }
        Route::Jpeg | Route::Jp2 => {
            // The chain in front of the codec is run and the codec is not: `image_stream`
            // answers `None` only where a filter before it is one this tree does not decode,
            // which is the same refusal the page would draw.
            let native = document.image_stream(&found.stream).ok_or_else(|| {
                declined("a filter before the codec could not be decoded".to_owned())
            })?;
            (native.data, found.entry.width, found.entry.height)
        }
    };
    let mut sink = sinks.open(name).map_err(sink_failed)?;
    sink.write_all(&bytes)
        .and_then(|()| sink.flush())
        .map_err(sink_failed)?;
    Ok(Output {
        name: name.to_owned(),
        bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        sanitised,
        origin: Origin::Image {
            source: plan.source,
            page: found.entry.page,
            object: found.entry.object.clone(),
            inline: found.entry.inline,
            file: route.file(),
            width,
            height,
        },
    })
}

/// One decode, the interpreter's own: straight-alpha RGBA8 on the grid the samples are on,
/// a stencil painted black through its set bits, colour converted as a device would.
fn decode_image(
    document: &Document,
    stream: &Stream,
    resources: &Dictionary,
) -> Result<pdf_model::image::Flattened, pdf_model::image::ImageError> {
    pdf_model::image::decode(
        document,
        stream,
        resources,
        pdf_render::Color::BLACK,
        &pdf_model::colour::Conversion::device(),
    )
}

/// The same stream stating no `/SMask` and no `/Mask`: the base image alone.
fn without_masks(stream: &Stream) -> Stream {
    let mut dict = stream.dict.clone();
    dict.remove("SMask");
    dict.remove("Mask");
    Stream {
        dict,
        data: Arc::clone(&stream.data),
        decryption_failed: stream.decryption_failed,
    }
}

/// Whether Table 87's `/SMaskInData` says the opacity came with the samples.
fn smask_in_data(document: &Document, stream: &Stream) -> bool {
    document
        .get_key(&stream.dict, "SMaskInData")
        .as_integer()
        .is_some_and(|code| code != 0)
}

/// An image's mask as the grey it gives the base: one byte per sample on the mask's own grid.
struct MaskImage {
    /// The opacity per sample, row by row.
    opacity: Vec<u8>,
    /// The mask's own `/Width`.
    width: u32,
    /// Its `/Height`.
    height: u32,
    /// The filter's sentence where the mask's decode stopped short on damaged data, for the
    /// caller to report beside the file it writes; see `pdf_model::image::Parts::shortfall`.
    shortfall: Option<String>,
}

/// Reads the mask an image states as an image of its own, or says why there is no such image.
///
/// `Ok(None)` is an image that states no mask. `Err` is a mask that exists and is not an
/// image — §8.9.6.4's colour key — or one that this tree could not decode, worded for the
/// report.
fn mask_of(document: &Document, found: &Found) -> Result<Option<MaskImage>, String> {
    let dict = &found.stream.dict;
    // §11.6.5.2's soft mask first, because §11.6.4.3 makes it override an explicit or colour
    // key mask where both are stated — the same order `pdf_model::image::decode` applies them.
    let soft = document.get_key(dict, "SMask");
    if let Some(mask) = soft.as_stream() {
        // A soft-mask image is a `DeviceGray` image XObject, and its decoded samples *are* the
        // opacity: the grey is read off the red channel because the three are one value.
        let pdf_model::image::Flattened { image, shortfall } =
            decode_image(document, mask, &found.resources)
                .map_err(|error| format!("the /SMask could not be decoded ({error})"))?;
        return Ok(Some(MaskImage {
            opacity: image.data.chunks_exact(4).map(|px| px[0]).collect(),
            width: image.width,
            height: image.height,
            shortfall,
        }));
    }
    let explicit = document.get_key(dict, "Mask");
    if let Some(stencil) = explicit.as_stream() {
        // §8.9.6.3: "[u]nmasked areas shall be painted with the corresponding portions of the
        // base image; masked areas shall not be", and §8.9.6.2 gives the stencil's sample that
        // paints. Decoded as the stencil it is, the painted places are the opaque ones, so the
        // alpha channel is the opacity the base receives.
        let pdf_model::image::Flattened { image, shortfall } =
            decode_image(document, stencil, &found.resources)
                .map_err(|error| format!("the /Mask could not be decoded ({error})"))?;
        return Ok(Some(MaskImage {
            opacity: image.data.chunks_exact(4).map(|px| px[3]).collect(),
            width: image.width,
            height: image.height,
            shortfall,
        }));
    }
    if explicit.as_array().is_some() {
        return Err(
            "§8.9.6.4's colour key mask is a range of sample values rather than an image, so \
             there is no mask file; the image is written with no mask applied"
                .to_owned(),
        );
    }
    Ok(None)
}

/// Writes one mask as an 8-bit grey image: a PNG beside a PNG, a PGM beside a netpbm image.
fn write_mask(
    plan: &ImagesPlan,
    sinks: &dyn Sinks,
    found: &Found,
    name: &str,
    mask: &MaskImage,
    sanitised: bool,
) -> Result<Output, Problem> {
    let sink_failed = |error: std::io::Error| Problem::Sink(name.to_owned(), error);
    let file = ImageFile::mask(plan.format);
    let bytes = match file {
        ImageFile::MaskPgm => crate::render::pgm(mask.width, mask.height, &mask.opacity),
        _ => encode_grey(mask.width, mask.height, &mask.opacity)
            .map_err(|error| sink_failed(std::io::Error::other(error)))?,
    };
    let mut sink = sinks.open(name).map_err(sink_failed)?;
    sink.write_all(&bytes)
        .and_then(|()| sink.flush())
        .map_err(sink_failed)?;
    Ok(Output {
        name: name.to_owned(),
        bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        sanitised,
        origin: Origin::Image {
            source: plan.source,
            page: found.entry.page,
            object: found.entry.object.clone(),
            inline: found.entry.inline,
            file,
            width: mask.width,
            height: mask.height,
        },
    })
}

/// Encodes one byte per sample as an 8-bit greyscale PNG, deterministically, as
/// [`crate::render::encode`] does for RGBA.
///
/// # Errors
///
/// The encoder's, which for a buffer whose length is width × height does not happen.
pub fn encode_grey(width: u32, height: u32, samples: &[u8]) -> Result<Vec<u8>, png::EncodingError> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(samples)?;
    }
    Ok(bytes)
}

/// Every image the selected pages reach, first reach first, each object once.
///
/// Per page: the `XObject`s its resources reach, forms descended, then the inline images in
/// its content, then the inline images in the content of each form the page reached for the
/// first time — so an image's ordinal is stable across runs and forms are read once.
fn enumerate(
    document: &Document,
    pages: &Pages<'_>,
    labels: &PageLabels,
    selected: &[usize],
    plan: &ImagesPlan,
    warnings: &mut Vec<Warning>,
) -> Vec<Found> {
    let mut walk = Walk {
        document,
        labels,
        plan,
        seen: BTreeSet::new(),
        forms: BTreeSet::new(),
        found: Vec::new(),
        warnings,
    };
    for &index in selected {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let mut reached_forms = Vec::new();
        walk.collect(&page, &page.resources, index, &mut reached_forms, 0);
        let content = page.content(document);
        walk.inline_images(&content, &page.resources, index);
        for (form, resources) in reached_forms {
            if let Some(content) = document.decoded_stream_data(&form) {
                walk.inline_images(&content, &resources, index);
            }
        }
    }
    walk.found
}

/// How deep a form may nest forms before the walk stops: a cycle is caught by `forms`, so this
/// bounds only a genuinely deep tree, at a depth no producer reaches.
const MAX_FORM_DEPTH: usize = 16;

/// The state one enumeration carries across pages.
struct Walk<'a> {
    /// The document.
    document: &'a Document,
    /// Its §12.4.2 labels.
    labels: &'a PageLabels,
    /// The plan, for its floor and its source.
    plan: &'a ImagesPlan,
    /// Image objects already listed.
    seen: BTreeSet<ObjectId>,
    /// Forms already walked.
    forms: BTreeSet<ObjectId>,
    /// What has been found, in order.
    found: Vec<Found>,
    /// Inline images that could not be read, by page.
    warnings: &'a mut Vec<Warning>,
}

impl Walk<'_> {
    /// Walks one resource dictionary's `/XObject`s, descending into forms and noting each form
    /// reached for the first time — with the resources its content is read against — in
    /// `reached`.
    fn collect(
        &mut self,
        page: &Page,
        resources: &Dictionary,
        index: usize,
        reached: &mut Vec<(Arc<Stream>, Dictionary)>,
        depth: usize,
    ) {
        let document = self.document;
        let Some(xobjects) = resources
            .get("XObject")
            .map(|object| document.resolve(object))
        else {
            return;
        };
        let Some(xobjects) = xobjects.as_dict() else {
            return;
        };
        for (_name, entry) in xobjects.iter() {
            let id = match entry {
                Object::Reference(id) => Some(*id),
                _ => None,
            };
            let resolved = document.resolve(entry);
            let Some(stream) = resolved.as_stream() else {
                continue;
            };
            let subtype = stream
                .dict
                .get("Subtype")
                .map(|object| document.resolve(object));
            let subtype = subtype
                .as_ref()
                .and_then(Object::as_name)
                .and_then(|n| n.as_str());
            match subtype {
                Some("Image") => {
                    if let Some(id) = id
                        && !self.seen.insert(id)
                    {
                        continue;
                    }
                    if let Some(entry) = self.describe(stream, id, index, false) {
                        self.found.push(Found {
                            entry,
                            stream: Arc::clone(stream),
                            resources: page.resources.clone(),
                        });
                    }
                }
                Some("Form") if depth < MAX_FORM_DEPTH => {
                    if let Some(id) = id
                        && !self.forms.insert(id)
                    {
                        continue;
                    }
                    // §8.10.2: a form's own resources, and where an older file states none,
                    // the page's — which is what the interpreter reads it against too.
                    let inner = stream
                        .dict
                        .get("Resources")
                        .map(|object| document.resolve(object));
                    let inner = inner
                        .as_ref()
                        .and_then(Object::as_dict)
                        .cloned()
                        .unwrap_or_else(|| resources.clone());
                    reached.push((Arc::clone(stream), inner.clone()));
                    self.collect(page, &inner, index, reached, depth.saturating_add(1));
                }
                _ => {}
            }
        }
    }

    /// Lexes one content stream for §8.9.7's `BI` and reads each inline image after it.
    ///
    /// The lexer is `pdf_syntax`'s own, so a `BI` inside a string or a name is a string or a
    /// name; and once `BI` is met the interpreter's reader takes over from the lexer's position
    /// and says where interpretation resumes — past the `EI` — so image data is never lexed as
    /// operators, which is the whole difficulty of the clause.
    fn inline_images(&mut self, content: &[u8], resources: &Dictionary, index: usize) {
        let mut lexer = Lexer::new(content);
        while let Some(token) = lexer.next_token() {
            if !matches!(token, Token::Keyword(b"BI")) {
                continue;
            }
            let scan = pdf_model::inline_image::scan(
                self.document,
                content,
                lexer.position(),
                resources,
                true,
            );
            match scan.image {
                Ok(stream) => {
                    if let Some(entry) = self.describe(&stream, None, index, true) {
                        self.found.push(Found {
                            entry,
                            stream: Arc::new(stream),
                            resources: resources.clone(),
                        });
                    }
                }
                Err(error) => self.warnings.push(Warning {
                    source: self.plan.source,
                    page: Some(index.saturating_add(1)),
                    detail: format!("an inline image could not be read: {error:?}"),
                }),
            }
            // `resume` is past the `EI` or at the end; it is never behind the lexer, and the
            // `max` makes that a fact about this loop rather than about another module.
            lexer.seek(scan.resume.max(lexer.position()));
        }
    }

    /// The inventory entry for one image, or `None` where it is under the plan's size floor or
    /// states no usable size.
    fn describe(
        &self,
        stream: &Stream,
        id: Option<ObjectId>,
        index: usize,
        inline: bool,
    ) -> Option<ImageEntry> {
        let document = self.document;
        let dict = &stream.dict;
        let integer = |key: &str| {
            dict.get(key)
                .map(|object| document.resolve(object))
                .and_then(|object| object.as_integer())
        };
        let width = u32::try_from(integer("Width")?).ok()?;
        let height = u32::try_from(integer("Height")?).ok()?;
        if u64::from(width).saturating_mul(u64::from(height)) < self.plan.min_pixels {
            return None;
        }
        let name_of = |object: &Object| -> Option<String> {
            match document.resolve(object) {
                Object::Name(name) => name.as_str().map(str::to_owned),
                Object::Array(items) => items.first().and_then(|first| {
                    document
                        .resolve(first)
                        .as_name()?
                        .as_str()
                        .map(str::to_owned)
                }),
                _ => None,
            }
        };
        let filters = dict
            .get("Filter")
            .map(|object| match document.resolve(object) {
                Object::Name(name) => name.as_str().map(str::to_owned).into_iter().collect(),
                Object::Array(items) => items
                    .iter()
                    .filter_map(|item| {
                        document
                            .resolve(item)
                            .as_name()?
                            .as_str()
                            .map(str::to_owned)
                    })
                    .collect(),
                _ => Vec::new(),
            })
            .unwrap_or_default();
        let stencil = dict
            .get("ImageMask")
            .map(|object| document.resolve(object))
            .is_some_and(|object| matches!(object, Object::Boolean(true)));
        Some(ImageEntry {
            source: self.plan.source,
            page: index.saturating_add(1),
            label: self.labels.label(index),
            object: id.map(|id| id.to_string()),
            inline,
            width,
            height,
            bits_per_component: integer("BitsPerComponent"),
            colour_space: dict.get("ColorSpace").and_then(name_of),
            filters,
            stencil,
            masked: dict.get("SMask").is_some() || dict.get("Mask").is_some(),
        })
    }
}
