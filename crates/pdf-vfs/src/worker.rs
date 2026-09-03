//! The confined side of RFC 0003 section 6, and the broker's half of the wire.
//!
//! # The posture, stated where the code is
//!
//! RFC 0003 section 6's diagram puts two thin, privileged frontends over one core over one
//! confined worker, and it says what each may hold: "[t]he frontends and the core never parse PDF
//! bytes. They hold paths, verbs, caches and the wire protocol. The confined worker parses,
//! renders, extracts". This module is the seam that makes that true of *this* crate: every
//! question that requires looking at a PDF is a [`Query`], every answer is an [`Answer`], and
//! [`crate::Vfs`] — the broker — holds no `Document` and calls no reader.
//!
//! [`InProcess`] is the one implementation this round, and it is deliberately the *unconfined*
//! one: it answers in the calling process, which is what a test harness and a first face need
//! and what `pdf-transform` itself defaulted to (RFC 0002 section 13 question 3, ADR 0800 section
//! 6). What makes the confined one a **transport** change rather than a redesign is that
//! [`Query`] and [`Answer`] are plain data with no borrow, no path and no descriptor in them, and
//! that a worker is created once per generation — which is exactly the moment a broker would
//! open the file and pass the descriptor across with `SCM_RIGHTS`, admitting the two syscalls
//! ADR 0812 admitted and not one more. `pdf_syntax::FileBytes::from_handle` is the receiving end
//! and already exists.
//!
//! # Why the readers are reached through `pdf-transform` and not directly
//!
//! RFC 0003 section 7: the core "[c]onsumes the transform layer (RFC 0002) for every write and
//! for page extraction; consumes the existing readers … through the confined worker". Six of the
//! eight generators below are a [`pdf_transform::Plan`] and nothing else, so a page taken out of
//! the mount is byte-for-byte a page taken out by `pdf-transform split`, and there is no second
//! implementation of extraction, rendering, image decoding or attachment saving anywhere in this
//! crate. The two that are not — a page's text and §14.3's metadata — have no verb, so they call
//! the `pdf-model` reader directly and add nothing to it.

use std::collections::BTreeMap;

use pdf_model::metadata::{Information, Trapped};
use pdf_model::outline::{Item, Outline};
use pdf_model::page_label::PageLabels;
use pdf_model::{Pages, interpret};
use pdf_syntax::{Document, FileBytes};
use pdf_transform::attachments::{Action, AttachmentEntry, AttachmentsPlan};
use pdf_transform::images::ImagesPlan;
use pdf_transform::json::Value;
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::{
    Budget, MemorySinks, Plan, Policy, Refusal, Report, Secret, Source, apply, pattern,
};

/// One question about the document, as data.
///
/// Every page number here is counted from 1, because that is what the layout's names are and a
/// second convention across one boundary is a defect waiting for a fencepost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    /// How many pages ISO 32000-2 §7.7.3.2's tree holds.
    PageCount,
    /// One page as a complete single-page PDF.
    ExtractPage {
        /// Which page.
        page: usize,
    },
    /// One page drawn at a resolution.
    RenderPage {
        /// Which page.
        page: usize,
        /// Dots per inch over §8.3.2.3's 72 units to the inch.
        dpi: u32,
    },
    /// Every image one page places, extracted, keyed by the name its output took.
    ///
    /// One query rather than an inventory *and* an extraction, which is what makes the
    /// `images/NNNN/` listing and a read out of it incapable of disagreeing: the directory's
    /// entries are this answer's keys (`crate::layout`'s departure note).
    ExtractImages {
        /// Which page.
        page: usize,
    },
    /// One page's text readback.
    PageText {
        /// Which page.
        page: usize,
    },
    /// §7.11.4's embedded files, listed.
    AttachmentInventory,
    /// One embedded file's bytes, by the name the document files it under.
    ExtractAttachment {
        /// The document's own name for it.
        name: String,
    },
    /// §14.3.3's document information dictionary.
    Information,
    /// §14.3.2's document-level metadata stream, undecoded of nothing but its filters.
    MetadataStream,
    /// §12.3.3's outline.
    Outline,
}

/// One answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// A count.
    Count(usize),
    /// A file's bytes.
    Bytes(Vec<u8>),
    /// Several files, by the name the transform seam's sinks were opened with.
    ///
    /// A `BTreeMap` rather than a `Vec` because trap 30 is about exactly this: `MemorySinks`
    /// hands its outputs back **in the order they were opened**, which is not the order a plan
    /// names them in, so anything that keys them by position is keying them by a race. The
    /// [`pdf_transform::Report`]'s own output names are the key here.
    Files(BTreeMap<String, Vec<u8>>),
    /// An attachment inventory.
    Attachments(Vec<AttachmentEntry>),
    /// The document states nothing here — no `/Metadata`, no such attachment.
    Absent,
}

/// Why a question could not be answered.
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    /// The transform seam refused the whole operation, by name.
    #[error("{0}")]
    Refused(#[from] Refusal),
    /// The transform seam produced the output but declined an item on the way — a codec the
    /// confined worker does not have, a page the rasteriser would not draw. Kept as its own
    /// variant so that a face can tell "this file cannot be made" from "this document cannot be
    /// read at all" (trap 5: it stays loud either way).
    #[error("{subject}: {detail}")]
    Declined {
        /// What was declined.
        subject: String,
        /// Why.
        detail: String,
    },
    /// The document has no such page, image or file.
    #[error("{0}")]
    NotPresent(String),
    /// The worker answered with something the broker did not ask for. Unreachable in
    /// [`InProcess`] and not unreachable across a pipe, which is why it is a variant rather than
    /// an assertion.
    #[error("the worker answered a {got} where a {wanted} was asked for")]
    Mismatched {
        /// What came back.
        got: &'static str,
        /// What was asked for.
        wanted: &'static str,
    },
}

/// Something that can answer questions about one generation of one document.
pub trait Worker: Send + Sync + std::fmt::Debug {
    /// Answers one question.
    ///
    /// # Errors
    ///
    /// [`WorkerError`] for a refusal, a declined item, or a page the document does not have.
    fn ask(&self, query: &Query) -> Result<Answer, WorkerError>;
}

/// What creates a worker for one generation of a document.
///
/// The broker's factory, and the place a confined implementation would fork, hand over the
/// descriptor and keep the pipe. Separate from [`Worker`] because the *lifetime* is the design:
/// one worker per generation, so a document that changed under the mount is a new worker rather
/// than a worker asked to change its mind.
pub trait Workers: Send + Sync + std::fmt::Debug {
    /// A worker over these bytes.
    ///
    /// # Errors
    ///
    /// Whatever starting one costs; [`InProcess`] cannot fail, and a confined one can.
    fn spawn(
        &self,
        bytes: FileBytes,
        password: Option<Secret>,
        policy: Policy,
        budget: Budget,
    ) -> Result<Box<dyn Worker>, WorkerError>;
}

/// Workers that answer in this process.
#[derive(Debug, Default, Clone, Copy)]
pub struct InProcessWorkers;

impl Workers for InProcessWorkers {
    fn spawn(
        &self,
        bytes: FileBytes,
        password: Option<Secret>,
        policy: Policy,
        budget: Budget,
    ) -> Result<Box<dyn Worker>, WorkerError> {
        let source = match password {
            Some(password) => Source::with_password(bytes, password),
            None => Source::new(bytes),
        };
        Ok(Box::new(InProcess {
            source,
            policy,
            budget,
        }))
    }
}

/// One document, answered in the calling process.
///
/// **One [`Source`], held for the worker's whole life**, because `viewer_core::Secret` is not
/// `Clone` on purpose — "[a] copy is a second buffer to clear and a second lifetime to reason
/// about" — so §7.6.4.1's password is held once here and both the verbs and the readers reach the
/// document through it (`pdf_transform::Source::document`).
#[derive(Debug)]
pub struct InProcess {
    /// The file and its password.
    source: Source,
    /// What the host decided about the document's assertions over its reader — asked once per
    /// `pdf_transform::apply`, which is where `CLAUDE.md` principle 3's four levels are honoured.
    policy: Policy,
    /// The ceilings.
    budget: Budget,
}

impl InProcess {
    /// Opens the document for a reader that has no verb.
    fn document(&self) -> Result<Document, WorkerError> {
        Ok(self.source.document(self.budget.limits)?)
    }

    /// Runs one plan and hands back every output it wrote, by name.
    fn run(&self, plan: &Plan) -> Result<(Report, BTreeMap<String, Vec<u8>>), WorkerError> {
        let sinks = MemorySinks::new();
        let report = apply(
            plan,
            std::slice::from_ref(&self.source),
            &sinks,
            &self.policy,
            &self.budget,
        )?;
        let files = sinks.into_outputs().into_iter().collect();
        Ok((report, files))
    }

    /// The one output a single-file plan wrote, or the reason there is none.
    fn only(&self, plan: &Plan) -> Result<Answer, WorkerError> {
        let (report, files) = self.run(plan)?;
        if let Some(declined) = report.refused.first() {
            return Err(WorkerError::Declined {
                subject: declined.subject.clone(),
                detail: declined.detail.clone(),
            });
        }
        match files.into_values().next() {
            Some(bytes) => Ok(Answer::Bytes(bytes)),
            None => Err(WorkerError::NotPresent(String::from(
                "the transform wrote no output for this request",
            ))),
        }
    }

    /// One page as a `Selection`, refused where the document does not have it.
    fn one_page(page: usize) -> Result<Selection, WorkerError> {
        page.to_string()
            .parse()
            .map_err(|_| WorkerError::NotPresent(format!("page {page} is not a page number")))
    }
}

impl Worker for InProcess {
    fn ask(&self, query: &Query) -> Result<Answer, WorkerError> {
        match query {
            Query::PageCount => {
                let document = self.document()?;
                Ok(Answer::Count(Pages::new(&document).len()))
            }
            Query::ExtractPage { page } => self.only(&Plan::Split(SplitPlan {
                source: 0,
                pages: Self::one_page(*page)?,
                pieces: Pieces::EachPage,
                names: page_pattern()?,
            })),
            Query::RenderPage { page, dpi } => self.only(&Plan::Render(RenderPlan {
                source: 0,
                pages: Self::one_page(*page)?,
                // Dots per inch over ISO 32000-2 §8.3.2.3's 72 units to the inch, which is
                // `Sizing::Dpi`'s own statement of the conversion; nothing is computed here.
                size: Sizing::Dpi(dpi_as_scale(*dpi)),
                format: ImageFormat::Png,
                page_box: None,
                annotations: true,
                names: page_pattern()?,
            })),
            Query::ExtractImages { page } => {
                let (report, files) =
                    self.run(&Plan::Images(images_plan(Self::one_page(*page)?)?))?;
                if files.is_empty()
                    && let Some(declined) = report.refused.first()
                {
                    return Err(WorkerError::Declined {
                        subject: declined.subject.clone(),
                        detail: declined.detail.clone(),
                    });
                }
                Ok(Answer::Files(files))
            }
            Query::PageText { page } => {
                let document = self.document()?;
                let pages = Pages::new(&document);
                let index = page
                    .checked_sub(1)
                    .filter(|index| *index < pages.len())
                    .ok_or_else(|| {
                        WorkerError::NotPresent(format!(
                            "page {page}: the document has {}",
                            pages.len()
                        ))
                    })?;
                let found = pages.get(index).ok_or_else(|| {
                    WorkerError::NotPresent(format!("page {page} could not be read"))
                })?;
                Ok(Answer::Bytes(
                    interpret(&document, &found).text.into_bytes(),
                ))
            }
            Query::AttachmentInventory => {
                let (report, _) = self.run(&Plan::Attachments(AttachmentsPlan {
                    source: 0,
                    action: Action::List,
                }))?;
                Ok(Answer::Attachments(
                    report
                        .listed
                        .into_iter()
                        .filter_map(|listed| match listed {
                            pdf_transform::Listed::Attachment(entry) => Some(entry),
                            pdf_transform::Listed::Image(_) => None,
                        })
                        .collect(),
                ))
            }
            Query::ExtractAttachment { name } => self.only(&Plan::Attachments(AttachmentsPlan {
                source: 0,
                action: Action::Save {
                    name: name.clone(),
                    names: page_pattern()?,
                },
            })),
            Query::Information => {
                let document = self.document()?;
                Ok(Answer::Bytes(
                    information_json(&Information::read(&document)).into_bytes(),
                ))
            }
            Query::MetadataStream => {
                let document = self.document()?;
                Ok(metadata_stream(&document).map_or(Answer::Absent, Answer::Bytes))
            }
            Query::Outline => {
                let document = self.document()?;
                let pages = Pages::new(&document);
                let outline = Outline::read(&document, &pages);
                let labels = PageLabels::read(&document);
                Ok(Answer::Bytes(
                    outline_json(&document, &pages, &labels, &outline).into_bytes(),
                ))
            }
        }
    }
}

/// The output-name pattern every single-file query uses.
///
/// One output, so the name carries nothing: [`Answer::Bytes`] is keyed by the query rather than
/// by a name, and `Pattern::distinguishes` is satisfied for a count of one.
fn page_pattern() -> Result<pattern::Pattern, WorkerError> {
    // `%d` is the ordinal within the run, which for a one-output run is always `1` — and for the
    // images route is what makes an output's name state its index within the page. A pattern
    // this crate wrote and never shows anybody, so its literal text is not a decision; the error
    // arm is unreachable and is written rather than asserted, because a literal that stopped
    // parsing is a change to the grammar and should be a message rather than a panic.
    "%d".parse::<pattern::Pattern>()
        .map_err(|error: pattern::PatternError| {
            WorkerError::Refused(Refusal::Pattern(error.to_string()))
        })
}

/// The images plan for one page: the codec's own stream where the codec has a file form.
///
/// `native` is on because RFC 0003 section 4 states it as a decision — "pass the original stream
/// through untouched where it is already a complete image file (`DCTDecode`, `JPXDecode` —
/// re-encoding would be a lie about the bytes), decode to PNG where it is not" — and `min_pixels`
/// is zero because a mount inventories what the document holds rather than what is worth looking
/// at.
fn images_plan(pages: Selection) -> Result<ImagesPlan, WorkerError> {
    Ok(ImagesPlan {
        source: 0,
        pages,
        min_pixels: 0,
        list_only: false,
        native: true,
        no_mask: false,
        format: ImageFormat::Png,
        // `%02d` rather than `%d`, so an output's name sorts the way a listing reads and the
        // index within the page is two digits as RFC 0003 section 4's own example spells it.
        names: "%02d"
            .parse::<pattern::Pattern>()
            .map_err(|error: pattern::PatternError| {
                WorkerError::Refused(Refusal::Pattern(error.to_string()))
            })?,
    })
}

/// Dots per inch as the scale `Sizing::Dpi` takes.
fn dpi_as_scale(dpi: u32) -> f32 {
    // A resolution a caller typed, far below `f32`'s exact integer range; the layout's own
    // resolutions are 150 and 300.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a dots-per-inch figure the layout table states, two of them, both small"
    )]
    let scale = dpi as f32;
    scale
}

/// The catalog's `/Metadata` stream, decoded, or `None` where the document states none.
///
/// ISO 32000-2 §14.3.2:
///
/// > The contents of a metadata stream shall be the metadata represented in Extensible Markup
/// > Language (XML) and the grammar of the XML representing the metadata shall be defined
/// > according to the extensible metadata platform specification (ISO 16684-1).
///
/// So the file at `meta/xmp.xml` is the stream's own bytes and nothing else: the clause says
/// they are XML, `pdf_model::xmp` is what *parses* them for a properties panel, and a mount that
/// re-serialised a parse would be handing back this program's reading of the packet under a name
/// that claims to be the packet. The only thing undone is §7.4's filters, because a `/Filter` is
/// how the bytes are stored rather than what they are.
fn metadata_stream(document: &Document) -> Option<Vec<u8>> {
    let catalog = document.catalog().ok()?;
    let object = document.get_key(&catalog, "Metadata");
    let stream = object.as_stream()?;
    document
        .decoded_stream_data(stream)
        .map(|bytes| bytes.to_vec())
}

/// §14.3.3's document information dictionary, as JSON.
///
/// ISO 32000-2 §14.3.3:
///
/// > Where a document information dictionary contains keys other than CreationDate and ModDate ,
/// > the value associated with any such key shall be a text string.
///
/// Which is why every entry below is a JSON string or `null` and none is a number: the clause
/// makes them text, `pdf_model::metadata` has already decoded §7.9.2.2's encodings, and the two
/// date entries are handed back **as the file spells them** rather than reformatted — a §7.9.4
/// date string is what the document said, and a mount that normalised it would be answering a
/// question about this program.
fn information_json(information: &Information) -> String {
    Value::Object(vec![
        (
            "title".to_owned(),
            Value::optional(information.title.clone()),
        ),
        (
            "author".to_owned(),
            Value::optional(information.author.clone()),
        ),
        (
            "subject".to_owned(),
            Value::optional(information.subject.clone()),
        ),
        (
            "keywords".to_owned(),
            Value::optional(information.keywords.clone()),
        ),
        (
            "creator".to_owned(),
            Value::optional(information.creator.clone()),
        ),
        (
            "producer".to_owned(),
            Value::optional(information.producer.clone()),
        ),
        (
            "created".to_owned(),
            Value::optional(information.created.clone()),
        ),
        (
            "modified".to_owned(),
            Value::optional(information.modified.clone()),
        ),
        (
            "trapped".to_owned(),
            Value::text(match information.trapped {
                Trapped::Fully => "True",
                Trapped::NotYet => "False",
                Trapped::Unknown => "Unknown",
            }),
        ),
    ])
    .render()
}

/// §12.3.3's outline, as JSON, with each item's page ordinal beside it.
///
/// ISO 32000-2 §12.3.3:
///
/// > The outline consists of a tree-structured hierarchy of outline items (sometimes called
/// > bookmarks ), which serve as a visual table of contents to display the document's structure
/// > to the user.
///
/// A tree, so the JSON is a tree: nothing here flattens it, because the clause's own structure is
/// what a consumer of the mount is reading it for. The page is the ordinal the `pages/` and
/// `text/` directories use, so an outline entry names a file that exists — which is the join this
/// file is for — and §12.4.2's label is beside it, because a label is what a person calls the
/// page and is not usable as a name (`crate::layout`).
fn outline_json(
    document: &Document,
    pages: &Pages<'_>,
    labels: &PageLabels,
    outline: &Outline,
) -> String {
    let indices = pages.indices();
    Value::Object(vec![
        (
            "stated_count".to_owned(),
            outline.stated_count.map_or(Value::Null, Value::Integer),
        ),
        (
            "items".to_owned(),
            Value::Array(
                outline
                    .items
                    .iter()
                    .map(|item| item_json(document, pages, labels, &indices, item))
                    .collect(),
            ),
        ),
    ])
    .render()
}

/// One outline item, and its children under it.
fn item_json(
    document: &Document,
    pages: &Pages<'_>,
    labels: &PageLabels,
    indices: &BTreeMap<pdf_syntax::ObjectId, usize>,
    item: &Item,
) -> Value {
    let index = item
        .destination
        .and_then(|destination| destination.page_index_with(document, pages, indices));
    Value::Object(vec![
        ("title".to_owned(), Value::text(item.title.clone())),
        (
            "page".to_owned(),
            Value::optional_count(index.map(|index| index.saturating_add(1))),
        ),
        (
            "label".to_owned(),
            Value::optional(index.and_then(|index| labels.label(index))),
        ),
        ("open".to_owned(), Value::Bool(item.open)),
        ("bold".to_owned(), Value::Bool(item.bold)),
        ("italic".to_owned(), Value::Bool(item.italic)),
        (
            "children".to_owned(),
            Value::Array(
                item.children
                    .iter()
                    .map(|child| item_json(document, pages, labels, indices, child))
                    .collect(),
            ),
        ),
    ])
}
