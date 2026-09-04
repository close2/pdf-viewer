//! `render` — pages to raster images, RFC 0002 section 6.4.
//!
//! Packaging, not construction: `interpret` → `render-cpu` at a scale derived from the dpi, PNG
//! out through the in-tree encoder. `crates/pdf-model/examples/render_at.rs` is this verb as an
//! example, and the integration test holds the verb to that pipeline byte for byte, so the CLI
//! cannot become a fourth rasteriser.
//!
//! # The scale
//!
//! ISO 32000-2 §8.3.2.3: "[t]he default for the size of the unit in default user space (1 / 72
//! inch) is approximately the same as a point" — so a page of *w* × *h* user-space units at
//! *d* dots per inch is *w* · *d* / 72 pixels wide. That mapping is the one every raster gate
//! already performs; how a fractional extent becomes a whole number of pixels is
//! [`pdf_render::TargetSpec::for_page`]'s decision, shared with the viewer, and not this crate's.
//!
//! # CPU only, and parallel across pages
//!
//! The oracle backend is the correctness reference and a batch tool wants no device dependency;
//! what throughput there is to be had comes from rayon across pages, which are independent (RFC
//! 0002 section 12).
//!
//! **One font cache, shared by every page.** The first landing gave each rayon job its own
//! `FontCache` through `map_init`, on the argument that the cache's mutex had never been
//! contended from two threads. That was the wrong unit as well as the wrong argument:
//! `map_init` runs its constructor once per *split* of the iterator rather than once per thread,
//! and a split is what a steal makes, so the number of caches grew with the stealing rather
//! than with the pool — and every one of them parsed the document's fonts again. Measured in the
//! eight-hundred-and-sixty-eighth session (ADR 0801), pages 1–200 of ISO 32000-2 at 150 dpi,
//! the `gates` profile, one sitting:
//!
//! | threads | one cache per job | one cache shared |
//! |---|---|---|
//! | 1 | 8.17 s wall, 7.96 s CPU | 8.09 s wall, 7.88 s CPU |
//! | 2 | 7.92 s wall, 14.35 s CPU | 4.12 s wall, 7.91 s CPU |
//! | 4 | 2.93 s wall, 11.26 s CPU | 2.43 s wall, 9.31 s CPU |
//! | 24 | 1.06 s wall, 20.38 s CPU | 1.08 s wall, 19.86 s CPU |
//!
//! Two threads did the work of one at twice the processor time; sharing the cache halves the
//! wall clock there and takes a fifth off it at four. What it *cannot* take off is the
//! 24-thread row, and that row is the machine's rather than this crate's: twenty-four separate
//! single-threaded processes over disjoint page ranges, sharing nothing, cost **20.6 s** of CPU
//! between them for the same pages — twelve cores with two hardware threads each, at an all-core
//! clock below the single-core one. So the "2.4× CPU gap" the first landing recorded is a
//! property of CPU-seconds as a unit under simultaneous multithreading, and not a cost this
//! code pays. The cache is `Sync` already — `pdf_font::LoadedFont`'s header says what that
//! cost, and ADR 0710 measured it — so sharing it is one field moved. The rasteriser stays per
//! job: it is two words and an empty memo.
//!
//! # Which box, and whether the annotations
//!
//! ISO 32000-2 §7.7.3.3 gives a page five rectangles and the defaults that chain them: the
//! crop box's "[d]efault value: the value of `MediaBox`", and for the bleed, trim and art boxes
//! "[d]efault value: the value of `CropBox`". §14.11.2.1 adds the one rule that binds a processor
//! on all four — "[i]f the bounds of the crop, trim, bleed or art box extends outside of the
//! bounds of the media box, a processor shall treat the box as its intersection with the media
//! box" — and `pdf_model::Page` has applied both by the time a page is handed over, so
//! [`RenderPlan::page_box`] chooses among rectangles already defaulted and intersected.
//!
//! **The box asked for is both the raster's extent and the clip, and that is a choice.** The
//! clause defines each box as a clipping region for a purpose — the crop box "the region to
//! which the contents of the page shall be clipped (cropped) when displayed or printed", the
//! bleed box the same "when output in a production environment" — so asking for a box is
//! asking for that purpose's view of the page, and the marks outside it are not shown. The
//! other construction, a larger extent with a smaller clip inside it and a blank margin
//! between, is §12.2's `/ViewArea` against `/ViewClip`, which is the document's to state and
//! not a flag's to invent; a document that states it is honoured under the default, which is
//! the viewer's own `display_box` and `clip_box`. Under a named box the two are that box.
//!
//! **Annotations draw by default because §6.3.2.2 requires it** of a rendering processor — it
//! "shall also render the appropriate appearance stream for all annotations" whose flags
//! designate one (§12.5.3, §12.5.5) — and [`RenderPlan::annotations`] `false` opts out: the page

//! is interpreted as a page that states no `/Annots`, so §12.5.3's pass has nothing to draw and
//! the raster is the content stream alone. Neither knob touches `interpret`: a `Page` is a
//! value whose fields are the interpreter's inputs, and `render` states the page it wants drawn
//! — the same move `Pages::detached` makes for §12.7.7's templates. ADR 0802.
//!
//! # What is a warning and what is a refusal

//!
//! A page whose interpretation reports something it could not draw is **written and warned
//! about** (exit 3): the output is usable and every missing mark is named, per page, which is
//! what trap 5 asks. A page the rasteriser will not draw at all, or that the budget will not
//! admit, is **refused by name** (exit 4) and the other pages are still written. ADR 0800.

use std::io::Write as _;

use pdf_model::content::FontCache;
pub use pdf_model::page::Boundary;
use pdf_model::page_label::PageLabels;
use pdf_model::view::ViewState;
use pdf_model::{Page, Pages};

use pdf_render::{Raster, Rasterizer, Size, TargetSpec};
use pdf_syntax::Document;
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use render_cpu::CpuRasterizer;

use crate::pattern::{Fill, Pattern};
use crate::range::Selection;
use crate::{Budget, Declined, Origin, Output, Refusal, Report, Sinks, Warning};

/// Pages to raster images.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderPlan {
    /// Which source.
    pub source: usize,
    /// Which pages, in which order.
    pub pages: Selection,
    /// How big.
    pub size: Sizing,
    /// Which file format.
    pub format: ImageFormat,
    /// Which of §7.7.3.3's boxes is the raster, or `None` for the viewer's own display
    /// boundary — the crop box unless §12.2's `/ViewArea` names another.
    pub page_box: Option<Boundary>,
    /// Whether §12.5.3's annotation pass draws. `true` is §6.3.2.2's obligation.
    pub annotations: bool,
    /// How the outputs are named.
    pub names: Pattern,
    /// How many strips one page's raster may be cut into, or `None` to ask the machine.
    ///
    /// **`None` is the batch answer and a confined caller may not use it.** `render-cpu` asks
    /// `std::thread::available_parallelism` where a caller says nothing, and on Linux that reads
    /// `/proc/self/cgroup` — an `openat` a process with no filesystem is *killed* for rather than
    /// told no (ADR 0218). `pdf-vfs`'s confined worker draws through this plan, so the number has
    /// to be sayable here; it states one, taken before its confinement (ADR 0847).
    ///
    /// It is `None` everywhere else, which is what this crate did before the field existed: a
    /// batch render is already parallel across pages, and what the strips add on top of that is
    /// the rasteriser's own judgement about one page.
    pub strips: Option<u32>,
}

impl RenderPlan {
    /// The three choices about how a page is drawn, together.
    #[must_use]
    pub fn drawing(&self) -> Drawing {
        Drawing {
            size: self.size,
            page_box: self.page_box,
            annotations: self.annotations,
        }
    }
}

/// How one page is drawn: how big, which box, whether the annotations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drawing {
    /// How big.
    pub size: Sizing,
    /// Which of §7.7.3.3's boxes, or the viewer's own display boundary.
    pub page_box: Option<Boundary>,
    /// Whether §12.5.3's annotation pass draws.
    pub annotations: bool,
}

/// Parses a `--page-box` word into a boundary.
#[must_use]
pub fn parse_boundary(word: &str) -> Option<Boundary> {
    match word {
        "media" => Some(Boundary::Media),
        "crop" => Some(Boundary::Crop),
        "bleed" => Some(Boundary::Bleed),
        "trim" => Some(Boundary::Trim),
        "art" => Some(Boundary::Art),
        _ => None,
    }
}

/// The page as the plan wants it drawn: the named box as both the displayed region and the
/// clip, and no `/Annots` where the annotations are opted out of.
///
/// A copy, because `pdf_syntax::Document` and what is read from it stay as the file said —
/// the document is the oracle's input and is not edited to be rendered differently.
#[must_use]
pub fn page_to_draw(page: &Page, page_box: Option<Boundary>, annotations: bool) -> Page {
    let mut page = page.clone();
    if let Some(boundary) = page_box {
        let rectangle = page.boundary(boundary);
        page.display_box = rectangle;
        page.clip_box = rectangle;
    }
    if !annotations {
        page.dict.remove("Annots");
    }
    page
}

/// How a page's size in user space becomes a size in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sizing {
    /// Dots per inch over §8.3.2.3's 72 units to the inch. 150 is the CLI's default —
    /// poppler's, not mutool's 72, because it is the modern-screen answer.
    Dpi(f32),
    /// Fit the page's longer side to this many pixels.
    Longest(u32),
    /// Fit the page inside this box, keeping its aspect.
    Within {
        /// The box's width in pixels.
        width: u32,
        /// Its height.
        height: u32,
    },
}

impl Sizing {
    /// The scale from user-space units to pixels for a page of this size.
    ///
    /// A page of zero extent gets scale 1, which [`TargetSpec::for_page`] then refuses as a
    /// zero-sized target — the refusal is its, and this does not pre-empt it.
    #[must_use]
    pub fn scale(self, page: Size) -> f32 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a pixel count a caller typed is far below f32's exact integer range"
        )]
        match self {
            Self::Dpi(dpi) => dpi / 72.0,
            Self::Longest(pixels) => {
                let longest = page.width.max(page.height);
                if longest > 0.0 {
                    pixels as f32 / longest
                } else {
                    1.0
                }
            }
            Self::Within { width, height } => {
                let across = if page.width > 0.0 {
                    width as f32 / page.width
                } else {
                    f32::INFINITY
                };
                let down = if page.height > 0.0 {
                    height as f32 / page.height
                } else {
                    f32::INFINITY
                };
                let scale = across.min(down);
                if scale.is_finite() { scale } else { 1.0 }
            }
        }
    }
}

/// The file format a raster is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// PNG, RGBA8, through the `png` crate the tree already ships. The default.
    Png,
    /// Binary PPM (`P6`), RGB8: the raster with its alpha dropped, which loses nothing because
    /// §11.4.7 has already composited the page onto 𝑊 white before the raster leaves the
    /// backend (`pdf_render::Medium::PAGE_ONLY`), so every alpha is 255.
    Ppm,
    /// Binary PGM (`P5`), one byte a pixel: the raster's RGB made grey by ISO 32000-2
    /// §10.4.2.2's rule, which is the one conversion from an RGB colour to a grey that the
    /// standard states (see [`grey_of`]), and its alpha dropped as PPM drops it.
    ///
    /// **What a colour in any other space becomes is decided before this format sees it**,
    /// and is a choice stated here: the raster handed to the encoder is already RGB — a page
    /// through `render-cpu`, an image through `pdf_model::image::decode` — with every
    /// `DeviceCMYK`, `ICCBased`, `Indexed` or `Separation` colour taken to RGB by the
    /// interpreter's own conversion, which §10.4.2.1 ranks above this clause's family for an
    /// ICC-enabled processor. So a grey file is the grey of the picture as this tree draws it,
    /// and a `DeviceGray` source comes through unchanged, because the clause's other direction
    /// — "[a] gray level shall be equivalent to an RGB value with all three components the
    /// same" — makes the round trip the identity (the three weights sum to 1.0). There is no
    /// second route that reads a grey image's samples directly: one conversion (trap 6), and
    /// a `/Decode` array, a soft mask or a colour key applied on the way to RGB stays applied.
    Pgm,
}

impl ImageFormat {
    /// The conventional file extension.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Ppm => "ppm",
            Self::Pgm => "pgm",
        }
    }

    /// Parses the CLI's word.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "png" => Some(Self::Png),
            "ppm" => Some(Self::Ppm),
            "pgm" => Some(Self::Pgm),
            _ => None,
        }
    }

    /// Whether the format carries an alpha channel. Only PNG does: a netpbm file is its
    /// samples and nothing else, so a mask that would have been the alpha is written beside
    /// the image instead (`crate::images`).
    #[must_use]
    pub fn holds_alpha(self) -> bool {
        match self {
            Self::Png => true,
            Self::Ppm | Self::Pgm => false,
        }
    }
}

/// One RGBA8 pixel's grey, ISO 32000-2 §10.4.2.2.
///
/// > The gray value for a given RGB value shall be computed according to the NTSC video
/// > standard, which determines how a colour television signal is rendered on a
/// > black-and-white television set:
///
/// and the formula the clause then sets out is `gray = 0.3 × red + 0.59 × green + 0.11 × blue`.
/// The arithmetic is [`pdf_render::Color::grey_level`]'s — the one place this tree states those
/// three weights, which a `/Luminosity` soft mask and `pdf_model`'s ink also take from there —
/// so that a grey file and a grey mask cannot disagree about what grey is (trap 6). The clause
/// works on components in `0.0..=1.0`; a byte is taken there, converted, and rounded to the
/// nearest byte on the way back. The alpha is ignored: what calls this has either composited
/// the page onto white already (§11.4.7) or written the mask beside the image.
#[must_use]
pub fn grey_of(pixel: [u8; 4]) -> u8 {
    let level = |byte: u8| f32::from(byte) / 255.0;
    let grey =
        pdf_render::Color::rgb(level(pixel[0]), level(pixel[1]), level(pixel[2])).grey_level();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=255 on the line above the cast"
    )]
    {
        (grey * 255.0).round().clamp(0.0, 255.0) as u8
    }
}

/// A binary PGM (`P5`) of one byte a sample, deterministically: the header and the samples
/// and nothing else.
#[must_use]
pub fn pgm(width: u32, height: u32, samples: &[u8]) -> Vec<u8> {
    let header = format!("P5\n{width} {height}\n255\n");
    let mut bytes = Vec::with_capacity(header.len().saturating_add(samples.len()));
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(samples);
    bytes
}

/// Why one page produced no raster.
#[derive(Debug, thiserror::Error)]
pub enum PageRefusal {
    /// The page is not in the document — a selection resolved against a different one.
    #[error("page {page} is past the end")]
    NoSuchPage {
        /// The page, counted from 1.
        page: usize,
    },
    /// The target would need more pixels than the budget admits, or has no extent.
    #[error("{0}")]
    Target(pdf_render::BackendError),
    /// The rasteriser would not draw it.
    #[error("{0}")]
    Raster(render_cpu::CpuRasterError),
}

/// How much of the raster the page does not reach, on each axis, in pixels.
///
/// A page `W` units wide at scale `s` is `ceil(W × s)` pixels wide, and the strip between
/// `W × s` and that ceiling is raster the page does not cover. `TargetSpec::for_page` anchors
/// the page at the raster's **top-left** corner on both axes (ADR 0064), so that strip is on the
/// right of the raster across and at the bottom of it down, and each is less than one whole pixel
/// because the ceiling is taken of the same number.
///
/// ISO 32000-2 states none of this — §10.7 leaves scan conversion to the device and says nothing
/// about a page whose size is not a whole number of pixels — so this is the renderer's documented
/// choice reported rather than a requirement met. It is reported because a caller comparing two
/// rasters of *different* page geometry cannot otherwise know where the page sits inside either:
/// turn a page a quarter turn and the strip moves from one edge to another, which is worth up to a
/// whole pixel of placement and which ADR 0831 section 1 had to search for rather than derive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Overrun {
    /// Columns of raster to the right of the page's right edge, in `0.0..1.0`.
    pub across: f64,
    /// Rows of raster below the page's bottom edge, in `0.0..1.0`.
    pub down: f64,
}

/// One page rendered: the raster, where the page sits inside it, and what the interpreter could
/// not draw.
#[derive(Debug)]
pub struct Rendered {
    /// The pixels.
    pub raster: Raster,
    /// The sub-pixel strip of raster the page does not reach, on each axis.
    pub overrun: Overrun,
    /// What the interpreter met and could not draw, in its own words.
    pub unsupported: Vec<String>,
}

/// Renders one page exactly as the viewer's oracle backend draws it.
///
/// Public so that a test — or a consumer that wants pixels rather than a file — can hold the
/// verb to `interpret` + `render-cpu` directly. `fonts` and `rasterizer` are the caller's so
/// that a loop over pages reuses them.
///
/// # Errors
///
/// A [`PageRefusal`] naming why: no such page, a target the budget refuses, a raster the
/// backend refuses.
pub fn render_page(
    document: &Document,
    page: &Page,
    view: &ViewState,
    fonts: &FontCache,
    rasterizer: &mut CpuRasterizer,
    drawing: Drawing,
    budget: &Budget,
) -> Result<Rendered, PageRefusal> {
    let page = page_to_draw(page, drawing.page_box, drawing.annotations);
    let interpretation = pdf_model::content::interpret_with_fonts(document, &page, view, fonts);
    let list = interpretation.display_list;
    let scale = drawing.size.scale(list.page_size);
    let target =
        TargetSpec::for_page(&list, scale, budget.max_pixels).map_err(PageRefusal::Target)?;
    let raster = rasterizer
        .rasterize(&list, target)
        .map_err(PageRefusal::Raster)?;
    let overrun = overrun_of(list.page_size, scale, target.width, target.height);
    Ok(Rendered {
        raster,
        overrun,
        unsupported: interpretation
            .unsupported
            .iter()
            .map(crate::describe)
            .collect(),
    })
}

/// The sub-pixel strip of raster the page does not reach, on each axis.
///
/// Derived from the same three numbers `TargetSpec::for_page` builds the target out of — the
/// page's extent in user space, the scale, and the raster's integer size — so it says where the
/// renderer put the page rather than where a second implementation of the rounding would have.
/// `pixel_extent` takes the ceiling, so each value is in `0.0..1.0` for any raster that was
/// built; a caller holding one built some other way gets the arithmetic clamped to that range
/// rather than a negative number.
fn overrun_of(page: Size, scale: f32, width: u32, height: u32) -> Overrun {
    let strip = |extent: f32, pixels: u32| {
        let exact = f64::from(extent) * f64::from(scale);
        (f64::from(pixels) - exact).clamp(0.0, 1.0)
    };
    Overrun {
        across: strip(page.width, width),
        down: strip(page.height, height),
    }
}

/// Encodes a raster in the format, deterministically: no timestamp chunk, no text chunk, the
/// encoder's default compression, so the same raster is the same bytes on every run.
///
/// # Errors
///
/// The encoder's, which for an in-memory buffer means a raster whose dimensions and data
/// disagree — and [`Raster`] is constructed by the backend, so that does not happen.
pub fn encode(raster: &Raster, format: ImageFormat) -> Result<Vec<u8>, png::EncodingError> {
    match format {
        ImageFormat::Png => {
            let mut bytes = Vec::new();
            {
                let mut encoder = png::Encoder::new(&mut bytes, raster.width, raster.height);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                let mut writer = encoder.write_header()?;
                writer.write_image_data(&raster.data)?;
            }
            Ok(bytes)
        }
        ImageFormat::Ppm => {
            let header = format!("P6\n{} {}\n255\n", raster.width, raster.height);
            let mut bytes = Vec::with_capacity(
                header
                    .len()
                    .saturating_add(raster.data.len().saturating_mul(3).saturating_div(4)),
            );
            bytes.extend_from_slice(header.as_bytes());
            for pixel in raster.data.chunks_exact(4) {
                bytes.extend_from_slice(&pixel[..3]);
            }
            Ok(bytes)
        }
        ImageFormat::Pgm => {
            let samples: Vec<u8> = raster
                .data
                .chunks_exact(4)
                .map(|pixel| grey_of([pixel[0], pixel[1], pixel[2], pixel[3]]))
                .collect();
            Ok(pgm(raster.width, raster.height, &samples))
        }
    }
}

/// What one page's job produced.
struct Done {
    /// The output written, or why not.
    outcome: Result<Output, Problem>,
    /// The interpreter's reports for the page.
    warnings: Vec<Warning>,
}

/// Why a page was not written: this program declined it, or the sink failed.
///
/// Two kinds because they are two exit statuses: a refusal is 4 and the other pages are still
/// written, a sink that fails is the machine's and ends the run with 2.
enum Problem {
    /// Refused by name.
    Declined(Declined),
    /// The sink could not be opened or written.
    Sink(String, std::io::Error),
}

/// Runs the verb.
pub(crate) fn run(
    plan: &RenderPlan,
    document: &Document,
    sinks: &dyn Sinks,
    budget: &Budget,
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
    if !plan.names.distinguishes(selected.len()) {
        return Err(Refusal::Pattern(format!(
            "{} pages would be written and the output name {:?} has no %d to tell them apart",
            selected.len(),
            plan.names.to_string()
        )));
    }
    if plan.names.names_a_title() {
        return Err(Refusal::Pattern(
            "%t names a title, and a rendered page has none".to_owned(),
        ));
    }
    let job = Job {
        plan,
        document,
        pages: &pages,
        labels: &labels,
        view: ViewState::of(document),
        fonts: FontCache::new(),
        budget,
        sinks,
        count: selected.len(),
    };

    // Every page is independent, so this is the embarrassingly parallel shape RFC 0002 section 12
    // asks for. The results come back in selection order whatever order the threads finished
    // in, which is what keeps the report deterministic. The font cache is the job's, shared —
    // `map_init` would make one per split, and the module comment has what that cost.
    let done: Vec<Done> = selected
        .par_iter()
        .enumerate()
        .map_init(
            || match plan.strips {
                Some(strips) => CpuRasterizer::new().with_strips(strips),
                None => CpuRasterizer::new(),
            },
            |rasterizer, (at, &index)| job.page(rasterizer, at, index),
        )
        .collect();

    for Done { outcome, warnings } in done {
        report.warnings.extend(warnings);
        match outcome {
            Ok(output) => report.outputs.push(output),
            Err(Problem::Declined(declined)) => report.refused.push(declined),
            Err(Problem::Sink(name, error)) => return Err(Refusal::Sink { name, error }),
        }
    }
    Ok(())
}

/// Everything one page's job needs that is shared between the pages.
struct Job<'a> {
    /// The plan.
    plan: &'a RenderPlan,
    /// The document.
    document: &'a Document,
    /// Its page tree.
    pages: &'a Pages<'a>,
    /// Its §12.4.2 labels.
    labels: &'a PageLabels,
    /// The state the pages are interpreted against: the document's own defaults.
    view: ViewState,
    /// The one font cache every page shares: a font parsed once is a font parsed once, whichever
    /// thread meets it first. The module comment has the measurement.
    fonts: FontCache,
    /// The ceilings.
    budget: &'a Budget,
    /// Where the outputs go.
    sinks: &'a dyn Sinks,
    /// How many pages are being written, for `%d`'s width.
    count: usize,
}

impl Job<'_> {
    /// Renders, encodes and writes the page at zero-based `index`, the `at`-th of the
    /// selection.
    fn page(&self, rasterizer: &mut CpuRasterizer, at: usize, index: usize) -> Done {
        let page_number = index.saturating_add(1);
        let label = self.labels.label(index);
        let expanded = self.plan.names.expand(&Fill {
            ordinal: at.saturating_add(1),
            count: self.count,
            page: Some(page_number),
            label: label.as_deref(),
            title: None,
        });
        let declined = |detail: String| Done {
            outcome: Err(Problem::Declined(Declined {
                source: self.plan.source,
                page: Some(page_number),
                subject: expanded.name.clone(),
                detail,
            })),
            warnings: Vec::new(),
        };
        let Some(page) = self.pages.get(index) else {
            return declined(PageRefusal::NoSuchPage { page: page_number }.to_string());
        };
        let rendered = match render_page(
            self.document,
            &page,
            &self.view,
            &self.fonts,
            rasterizer,
            self.plan.drawing(),
            self.budget,
        ) {
            Ok(rendered) => rendered,
            Err(refusal) => return declined(refusal.to_string()),
        };
        let overrun = rendered.overrun;
        let warnings = rendered
            .unsupported
            .into_iter()
            .map(|detail| Warning {
                source: self.plan.source,
                page: Some(page_number),
                detail,
            })
            .collect();
        let outcome = write(
            self.sinks,
            &expanded.name,
            &rendered.raster,
            self.plan.format,
        )
        .map(|bytes| Output {
            name: expanded.name.clone(),
            bytes,
            sanitised: expanded.sanitised,
            origin: Origin::Page {
                source: self.plan.source,
                page: page_number,
                label,
                width: rendered.raster.width,
                height: rendered.raster.height,
                overrun,
            },
        })
        .map_err(|error| Problem::Sink(expanded.name, error));
        Done { outcome, warnings }
    }
}

/// Encodes and writes one raster, answering the byte count.
fn write(
    sinks: &dyn Sinks,
    name: &str,
    raster: &Raster,
    format: ImageFormat,
) -> std::io::Result<u64> {
    let bytes = encode(raster, format).map_err(std::io::Error::other)?;
    let mut sink = sinks.open(name)?;
    sink.write_all(&bytes)?;
    sink.flush()?;
    Ok(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::grey_of;

    /// §10.4.2.2's two directions meet on a grey: "[a] gray level shall be equivalent to an RGB
    /// value with all three components the same", and the NTSC weights sum to 1.0, so a grey
    /// pixel's grey is its own value — every byte, exactly.
    #[test]
    fn a_grey_pixel_is_its_own_grey() {
        for level in 0..=255_u8 {
            assert_eq!(grey_of([level, level, level, 255]), level);
        }
    }

    /// The weights, on the primaries where the clause's arithmetic does not land on a half:
    /// `0.59 × 255 = 150.45` and `0.11 × 255 = 28.05`. Pure red is `76.5` exactly, a tie that
    /// floating-point arithmetic settles one way or the other, so it is not pinned.
    #[test]
    fn the_primaries_weigh_what_the_clause_says() {
        assert_eq!(grey_of([0, 255, 0, 255]), 150);
        assert_eq!(grey_of([0, 0, 255, 255]), 28);
        assert_eq!(grey_of([255, 0, 255, 255]), 105);
        assert_eq!(grey_of([255, 255, 0, 0]), 227, "the alpha is ignored");
    }
}
