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
//! 0002 section 12). One font cache and one rasteriser per worker thread rather than one shared, because
//! `pdf_model::content::FontCache` is behind a mutex nothing in this workspace contends for from
//! two threads, and a transform is the first thing that would.
//!
//! # What is a warning and what is a refusal
//!
//! A page whose interpretation reports something it could not draw is **written and warned
//! about** (exit 3): the output is usable and every missing mark is named, per page, which is
//! what trap 5 asks. A page the rasteriser will not draw at all, or that the budget will not
//! admit, is **refused by name** (exit 4) and the other pages are still written. ADR 0800.

use std::io::Write as _;

use pdf_model::content::FontCache;
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
    /// How the outputs are named.
    pub names: Pattern,
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
}

impl ImageFormat {
    /// The conventional file extension.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Ppm => "ppm",
        }
    }

    /// Parses the CLI's word.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "png" => Some(Self::Png),
            "ppm" => Some(Self::Ppm),
            _ => None,
        }
    }
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

/// One page rendered: the raster and what the interpreter could not draw.
#[derive(Debug)]
pub struct Rendered {
    /// The pixels.
    pub raster: Raster,
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
    size: Sizing,
    budget: &Budget,
) -> Result<Rendered, PageRefusal> {
    let interpretation = pdf_model::content::interpret_with_fonts(document, page, view, fonts);
    let list = interpretation.display_list;
    let scale = size.scale(list.page_size);
    let target =
        TargetSpec::for_page(&list, scale, budget.max_pixels).map_err(PageRefusal::Target)?;
    let raster = rasterizer
        .rasterize(&list, target)
        .map_err(PageRefusal::Raster)?;
    Ok(Rendered {
        raster,
        unsupported: interpretation
            .unsupported
            .iter()
            .map(crate::describe)
            .collect(),
    })
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
        budget,
        sinks,
        count: selected.len(),
    };

    // Every page is independent, so this is the embarrassingly parallel shape RFC 0002 section 12 asks for.
    // The results come back in selection order whatever order the threads finished in, which
    // is what keeps the report deterministic.
    let done: Vec<Done> = selected
        .par_iter()
        .enumerate()
        .map_init(
            || (FontCache::new(), CpuRasterizer::new()),
            |(fonts, rasterizer), (at, &index)| job.page(fonts, rasterizer, at, index),
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
    fn page(
        &self,
        fonts: &FontCache,
        rasterizer: &mut CpuRasterizer,
        at: usize,
        index: usize,
    ) -> Done {
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
            fonts,
            rasterizer,
            self.plan.size,
            self.budget,
        ) {
            Ok(rendered) => rendered,
            Err(refusal) => return declined(refusal.to_string()),
        };
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
