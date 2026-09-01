//! `images` — the images a document embeds, as files, RFC 0002 section 6.3.
//!
//! # What is enumerated
//!
//! The image `XObject`s reachable from a selected page's resources (ISO 32000-2 §7.8.3, the
//! `/XObject` sub-dictionary of Table 34), descending through form `XObject`s' own resources
//! (§8.10.2) because a scanned page very often wraps its one image in a form. An image placed on
//! forty pages is one object and is extracted **once**, on the first selected page that reaches
//! it — RFC 0002 section 6.3's proposal — and the report says which page that was.
//!
//! **Inline images (§8.9.7) are not enumerated this round.** They are a content-stream construct
//! the interpreter already parses, and enumerating them means one interpreter touch this round
//! did not take; `doc/todo/57` names it. An image `XObject` that is a stencil mask
//! (`/ImageMask true`, §8.9.6.2) is decoded painting black through its set bits, which is what a
//! stencil is with no fill colour to take from.
//!
//! # What is written
//!
//! Decoded PNG, because it is the answer that always works: `pdf_model::image::decode` produces
//! straight-alpha RGBA8 for every image an interpreted page can place — JBIG2, JPX and CCITT
//! through the confined `pdf-sandbox` worker, exactly as in the viewer — with the image's soft
//! mask composited into the alpha. A build without the worker beside it refuses those images
//! **by name** rather than falling back in-process (RFC 0002 section 8). `--native` pass-through of
//! DCT and JPX bytes is the next round's; `doc/todo/57`.

use std::collections::BTreeSet;
use std::io::Write as _;
use std::sync::Arc;

use pdf_model::page_label::PageLabels;
use pdf_model::{Page, Pages};
use pdf_syntax::{Dictionary, Document, Object, ObjectId, Stream};
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::json::Value;
use crate::pattern::{Fill, Pattern};
use crate::range::Selection;
use crate::{Declined, Listed, Origin, Output, Refusal, Report, Sinks};

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
    /// How the outputs are named.
    pub names: Pattern,
}

/// One image `XObject`, as the inventory describes it.
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

/// One image found on a page: what decoding it needs.
#[derive(Debug)]
struct Found {
    /// The inventory entry.
    entry: ImageEntry,
    /// The stream.
    stream: Arc<Stream>,
    /// The resources it was found under — the page's, which is what a colour space named by
    /// resource name resolves against (§8.6.3's `/ColorSpace` sub-dictionary).
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

    let found = enumerate(document, &pages, &labels, &selected, plan);
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
    let outcomes: Vec<Result<Output, Problem>> = found
        .par_iter()
        .enumerate()
        .map(|(at, found)| {
            let expanded = plan.names.expand(&Fill {
                ordinal: at.saturating_add(1),
                count,
                page: Some(found.entry.page),
                label: found.entry.label.as_deref(),
                title: None,
            });
            let declined = |detail: String| Declined {
                source: plan.source,
                page: Some(found.entry.page),
                subject: expanded.name.clone(),
                detail,
            };
            let image = pdf_model::image::decode(
                document,
                &found.stream,
                &found.resources,
                pdf_render::Color::BLACK,
                &pdf_model::colour::Conversion::device(),
            )
            .map_err(|error| Problem::Declined(declined(error.to_string())))?;
            let raster = pdf_render::Raster {
                width: image.width,
                height: image.height,
                format: pdf_render::RasterFormat::Rgba8,
                data: image.data.to_vec(),
            };
            let sink_failed = |error: std::io::Error| Problem::Sink(expanded.name.clone(), error);
            let bytes = crate::render::encode(&raster, crate::render::ImageFormat::Png)
                .map_err(|error| sink_failed(std::io::Error::other(error)))?;
            let mut sink = sinks.open(&expanded.name).map_err(sink_failed)?;
            sink.write_all(&bytes)
                .and_then(|()| sink.flush())
                .map_err(sink_failed)?;
            Ok(Output {
                name: expanded.name.clone(),
                bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
                sanitised: expanded.sanitised,
                origin: Origin::Image {
                    source: plan.source,
                    page: found.entry.page,
                    object: found.entry.object.clone(),
                    width: image.width,
                    height: image.height,
                },
            })
        })
        .collect();
    for outcome in outcomes {
        match outcome {
            Ok(output) => report.outputs.push(output),
            Err(Problem::Declined(declined)) => report.refused.push(declined),
            Err(Problem::Sink(name, error)) => return Err(Refusal::Sink { name, error }),
        }
    }
    Ok(())
}

/// Why an image was not written: this program declined it (exit 4, the others still written),
/// or the sink failed (exit 2, the run ends).
enum Problem {
    /// Refused by name — the codec worker missing, the image malformed.
    Declined(Declined),
    /// The sink could not be opened or written.
    Sink(String, std::io::Error),
}

/// Every image the selected pages reach, first reach first, each object once.
fn enumerate(
    document: &Document,
    pages: &Pages<'_>,
    labels: &PageLabels,
    selected: &[usize],
    plan: &ImagesPlan,
) -> Vec<Found> {
    let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
    let mut found = Vec::new();
    for &index in selected {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let mut forms: BTreeSet<ObjectId> = BTreeSet::new();
        collect(
            document,
            &page,
            &page.resources,
            index,
            labels,
            plan,
            &mut seen,
            &mut forms,
            &mut found,
            0,
        );
    }
    found
}

/// How deep a form may nest forms before the walk stops: a cycle is caught by `forms`, so this
/// bounds only a genuinely deep tree, at a depth no producer reaches.
const MAX_FORM_DEPTH: usize = 16;

/// Walks one resource dictionary's `/XObject`s, descending into forms.
#[expect(
    clippy::too_many_arguments,
    reason = "a recursive walk whose state is five named things; a struct for them would name \
              the same five"
)]
fn collect(
    document: &Document,
    page: &Page,
    resources: &Dictionary,
    index: usize,
    labels: &PageLabels,
    plan: &ImagesPlan,
    seen: &mut BTreeSet<ObjectId>,
    forms: &mut BTreeSet<ObjectId>,
    found: &mut Vec<Found>,
    depth: usize,
) {
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
                    && !seen.insert(id)
                {
                    continue;
                }
                if let Some(entry) = describe(document, stream, id, index, labels, plan) {
                    found.push(Found {
                        entry,
                        stream: Arc::clone(stream),
                        resources: page.resources.clone(),
                    });
                }
            }
            Some("Form") if depth < MAX_FORM_DEPTH => {
                if let Some(id) = id
                    && !forms.insert(id)
                {
                    continue;
                }
                let inner = stream
                    .dict
                    .get("Resources")
                    .map(|object| document.resolve(object));
                if let Some(inner) = inner.as_ref().and_then(Object::as_dict) {
                    collect(
                        document,
                        page,
                        inner,
                        index,
                        labels,
                        plan,
                        seen,
                        forms,
                        found,
                        depth.saturating_add(1),
                    );
                }
            }
            _ => {}
        }
    }
}

/// The inventory entry for one image, or `None` where it is under the plan's size floor or
/// states no usable size.
fn describe(
    document: &Document,
    stream: &Stream,
    id: Option<ObjectId>,
    index: usize,
    labels: &PageLabels,
    plan: &ImagesPlan,
) -> Option<ImageEntry> {
    let dict = &stream.dict;
    let integer = |key: &str| {
        dict.get(key)
            .map(|object| document.resolve(object))
            .and_then(|object| object.as_integer())
    };
    let width = u32::try_from(integer("Width")?).ok()?;
    let height = u32::try_from(integer("Height")?).ok()?;
    if u64::from(width).saturating_mul(u64::from(height)) < plan.min_pixels {
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
        source: plan.source,
        page: index.saturating_add(1),
        label: labels.label(index),
        object: id.map(|id| id.to_string()),
        width,
        height,
        bits_per_component: integer("BitsPerComponent"),
        colour_space: dict.get("ColorSpace").and_then(name_of),
        filters,
        stencil,
        masked: dict.get("SMask").is_some() || dict.get("Mask").is_some(),
    })
}
