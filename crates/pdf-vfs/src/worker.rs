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
use pdf_transform::attachments::{Action, AttachmentEntry, AttachmentsPlan, Payload};
use pdf_transform::images::ImagesPlan;
use pdf_transform::json::Value;
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::update::{self, UpdatePlan};
use pdf_transform::{
    Budget, Consulted, Level, MemorySinks, Operation, Plan, Policy, Refusal, Report, Secret,
    Source, apply_borrowed, consult, pattern,
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
    /// Another document's pages inserted before a position, counted from 1; one past the end
    /// appends. RFC 0003 section 5.2's first write verb.
    InsertPages {
        /// Where they go, counted from 1.
        at: usize,
        /// The document whose pages are being carried in, whole.
        document: Vec<u8>,
    },
    /// One page taken out of §7.7.3.2's tree.
    DeletePage {
        /// Which page, counted from 1.
        page: usize,
    },
    /// A file written into the document as a §7.11.4 embedded file.
    Attach {
        /// The name §7.7.4's tree files it under.
        name: String,
        /// Its bytes.
        bytes: Vec<u8>,
    },
    /// An embedded file taken out of §7.7.4's tree.
    Detach {
        /// The name the tree files it under.
        name: String,
    },
    /// §14.3.3's entries set to what this JSON states — `meta/info.json`'s own form, which is
    /// what [`Query::Information`] answers with.
    SetInformation {
        /// The file the caller wrote.
        json: Vec<u8>,
    },
    /// **Would this operation be restricted, and why** — the first of ADR 0874's two round
    /// trips, and the only query here that changes nothing and reads no page.
    ///
    /// RFC 0003 section 6 puts every byte of parsing in a process with no channel to a person,
    /// and `CLAUDE.md` principle 3's *ask* level needs one. So the question crosses as data and
    /// comes back as [`Answer::Consulted`]: the level's verdict, the operation and the document's
    /// own reasons, worded once by `pdf_transform::consult`. A face with somewhere to put the
    /// question puts it; a face with nowhere says so.
    Consult {
        /// What the caller is about to do.
        operation: Operation,
    },
    /// The second round trip: this query, with a person's *yes* behind it.
    ///
    /// The wrapper carries the **answer**, never a second copy of the policy — the inner query
    /// runs at `Level::Off`, which is the level `CLAUDE.md` says "shall always be possible" and
    /// is what a person consenting to one operation has chosen for it. A broker sends this only
    /// where a [`Query::Consult`] asked and was answered yes; nothing here can tell, which is
    /// why the answer is `crate::Vfs`'s to keep and this is only how it crosses.
    ///
    /// Boxed because `Query` would otherwise be infinitely sized; a `Consented` inside a
    /// `Consented` is not a shape the broker builds and needs no special case — running an inner
    /// `Consented` at `Off` gives the same answer as running it at whatever it held.
    Consented(Box<Query>),
}

impl Query {
    /// Which of `pdf_model::restriction`'s operations this question performs, if any.
    ///
    /// **The broker's copy of `pdf_transform::Plan::operation`**, and it exists because a
    /// consent is spent against an operation rather than against a query: a person who said yes
    /// to deleting a page said yes to `Operation::Assemble`, and the broker has to be able to
    /// tell that the query it is about to send is the one that was asked about.
    ///
    /// `tests/a_write.rs` holds it to the plan's own answer rather than to this list, because
    /// two mappings that must agree and are only *said* to agree is how they stop agreeing.
    #[must_use]
    pub fn operation(&self) -> Option<Operation> {
        match self {
            // Read-only questions no Table 22 bit names: a count, a listing, §14.3's metadata,
            // §12.3.3's outline. `Query::PageText` is here for a reason of its own —
            // `doc/todo/38` holds Table 22 bit 5's "copy or otherwise extract" for the day a
            // host can say *this is a copy*, and until then this crate does not invent one.
            Self::PageCount
            | Self::PageText { .. }
            | Self::AttachmentInventory
            | Self::Information
            | Self::MetadataStream
            | Self::Outline
            | Self::Consult { .. } => None,
            // Table 22 bit 11 names all three in as many words: "[a]ssemble the document
            // (insert, rotate, or delete pages …)", and a page taken *out* of the mount is
            // `pdf_transform split`'s own output, which is a file made of pages the source
            // stated.
            Self::ExtractPage { .. } | Self::InsertPages { .. } | Self::DeletePage { .. } => {
                Some(Operation::Assemble)
            }
            Self::RenderPage { .. } => Some(Operation::Print),
            Self::ExtractImages { .. } | Self::ExtractAttachment { .. } => Some(Operation::Extract),
            Self::Attach { .. } | Self::Detach { .. } | Self::SetInformation { .. } => {
                Some(Operation::Modify)
            }
            // The operation is the inner query's; the wrapper is an answer about it.
            Self::Consented(inner) => inner.operation(),
        }
    }
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
    /// The whole document, with ISO 32000-2 §7.5.6's update appended.
    ///
    /// **The whole file, not the suffix**, which is the shape RFC 0003 section 6 needs: "the
    /// confined worker *computes* the transform output (the §7.5.6 append bytes); the broker
    /// validates the frame … and performs the actual file append". A broker that received only
    /// a suffix would have to trust that it belonged to the file it holds; one that receives the
    /// whole document can *check* the clause's own property — "changes shall be appended to the
    /// end of the file, leaving its original contents intact" — against the bytes on disk,
    /// which is a comparison and not a parse.
    Written {
        /// The updated document.
        bytes: Vec<u8>,
        /// What the transform said on the way, including the levels of `CLAUDE.md` principle 3
        /// that proceed and speak.
        warnings: Vec<String>,
    },
    /// What [`Query::Consult`] came back with: the verdict, the operation and the reasons.
    ///
    /// Never a refusal, even when the verdict is one — a question answered is an answer. What a
    /// broker does with it is `crate::Vfs::consult`'s, and what a *face* does with it is the
    /// face's, which is the whole point of asking before acting.
    Consulted(Consulted),
}

/// Why a question could not be answered.
///
/// # Why a refusal is a sentence and a kind rather than `pdf_transform::Refusal`
///
/// **Because the confined worker has to be a transport change and nothing else** (ADR 0847). A
/// `Refusal` is a dozen structured variants, and re-encoding each of them on the wire would put a
/// second copy of that vocabulary in this crate — with a face that behaved differently depending
/// on which worker answered it, which is the one thing the seam exists to prevent. So both
/// implementations produce the same population here: the sentence the seam wrote, under the one
/// distinction a face actually acts on — [`Self::PasswordRequired`], which is the answer that
/// means *ask somebody for something* rather than *this cannot be done*.
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    /// The document asserts something over its reader and the host's level is `on`.
    ///
    /// Its own variant because a face turns it into an `errno` a file manager shows, and because
    /// it is the answer that means *the document said no* rather than *this cannot be done*. The
    /// sentence names every bit and clause that applied — `pdf_transform` words the list and
    /// `pdf_model::restriction` supplies it, once, at the seam inside `apply`.
    #[error("{0}")]
    Restricted(String),
    /// The host's level is `ask` and there is nobody to ask.
    ///
    /// A file system has no dialogue. RFC 0003 section 5.3 already records what that costs —
    /// FUSE "returns … `EPERM` with no message channel" — so the level is answered here the way
    /// `viewer_host::unanswerable` answers it for a host that cannot put the question: as a
    /// refusal, never as a silent proceed, because proceeding would be doing the very thing the
    /// person asked to be consulted about.
    #[error("{0}")]
    Unanswerable(String),
    /// ISO 32000-2 §7.6.4.1: the document is encrypted, and neither the empty password nor the
    /// one supplied opens it.
    ///
    /// Its own variant because it is the only refusal here a face can do something about: a mount
    /// that could ask would ask, and `crate::Vfs::shortfalls` already names the design that is
    /// missing for it.
    #[error("{0}")]
    PasswordRequired(String),
    /// The transform seam refused the whole operation, in its own words.
    #[error("{0}")]
    Refused(String),
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
    /// The confined worker could not be started, stopped without answering, or sent something
    /// this build cannot read.
    ///
    /// **A named error rather than a hang**, which is what a face shows: RFC 0003 section 6's
    /// worker is killable by design — by its own seccomp filter, by the address-space ceiling, by
    /// a panic — and every one of those has to arrive somewhere a file manager can print it.
    /// [`InProcess`] cannot produce it.
    #[error("{0}")]
    Transport(#[from] confined_transport::TransportError),
}

impl WorkerError {
    /// The refusal a `pdf_transform::Refusal` is, in this vocabulary.
    ///
    /// The one place the seam's structured population is narrowed, so that the in-process worker
    /// and the confined one cannot disagree about what a document that wants a password is.
    fn of(refusal: &Refusal) -> Self {
        match refusal {
            Refusal::PasswordRequired { .. } => Self::PasswordRequired(refusal.to_string()),
            Refusal::Restricted { .. } => Self::Restricted(refusal.to_string()),
            Refusal::Unanswered { .. } => Self::Unanswerable(refusal.to_string()),
            _ => Self::Refused(refusal.to_string()),
        }
    }
}

impl From<Refusal> for WorkerError {
    fn from(refusal: Refusal) -> Self {
        Self::of(&refusal)
    }
}

/// Something that can answer questions about one generation of one document.
pub trait Worker: Send + Sync + std::fmt::Debug {
    /// Answers one question.
    ///
    /// # Errors
    ///
    /// [`WorkerError`] for a refusal, a declined item, or a page the document does not have; and
    /// [`WorkerError::Transport`] where a confined worker is gone.
    fn ask(&self, query: &Query) -> Result<Answer, WorkerError>;

    /// The same question, with a person's *yes* behind it — `CLAUDE.md` principle 3's *ask*
    /// level, answered.
    ///
    /// **The second of ADR 0874's two round trips.** The first is [`Query::Consult`], which a
    /// broker puts to this worker and a face puts to a person; this is the operation issued
    /// afterwards, at the level a consent *is* — `Level::Off`, the one `CLAUDE.md` says "shall
    /// always be possible", for this one operation and no other.
    ///
    /// Defaulted to [`Worker::ask`], which is the safe direction rather than a convenience: an
    /// implementation that does not override it refuses the operation again instead of
    /// performing something nobody consented to. Both implementations in this crate override it.
    ///
    /// # Errors
    ///
    /// [`Worker::ask`]'s.
    fn ask_consented(&self, query: &Query) -> Result<Answer, WorkerError> {
        self.ask(query)
    }

    /// Whether this worker can still be asked anything.
    ///
    /// **What makes "the next query gets a fresh worker" a property rather than a hope.** A
    /// confined worker that the kernel killed answers `false` for ever after, and
    /// [`crate::Vfs`] throws the generation away rather than asking a corpse — so a face that
    /// showed the death and was asked again gets a new worker rather than a second, stranger
    /// error about a closed pipe. [`InProcess`] is always alive: there is nothing to die.
    fn is_alive(&self) -> bool {
        true
    }
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
        Ok(Box::new(InProcess::new(source, policy, budget, None)))
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
    /// How many strips a page's raster is cut into, or `None` to let `render-cpu` ask the machine.
    ///
    /// **A confined worker states it and an unconfined one does not**, and the reason is the
    /// kernel's rather than a preference: `std::thread::available_parallelism` reads
    /// `/proc/self/cgroup` on Linux, and a process with no filesystem is *killed* for it rather
    /// than told no (ADR 0218). `crate::serve` takes the number before its confinement and states
    /// it here; `InProcessWorkers` leaves it `None`, which is what this crate did before the
    /// confined implementation existed.
    strips: Option<u32>,
}

impl InProcess {
    /// A worker over one source, drawing with `strips` strips a page.
    ///
    /// Public because a confinement probe has to be able to do the work *inside* a confined
    /// process without a broker on the other end of a socket — which is what
    /// `tests/confined.rs`'s positive probe is, and what says `Profile::Interpreter` is wide
    /// enough for this worker.
    #[must_use]
    pub fn new(source: Source, policy: Policy, budget: Budget, strips: Option<u32>) -> Self {
        Self {
            source,
            policy,
            budget,
            strips,
        }
    }

    /// Opens the document for a reader that has no verb.
    fn document(&self) -> Result<Document, WorkerError> {
        Ok(self.source.document(self.budget.limits)?)
    }

    /// Runs one plan and hands back every output it wrote, by name.
    fn run(
        &self,
        plan: &Plan,
        level: Level,
    ) -> Result<(Report, BTreeMap<String, Vec<u8>>), WorkerError> {
        self.run_beside(plan, None, level)
    }

    /// [`InProcess::run`], with a second document opened beside the mounted one.
    ///
    /// `pdf_transform::apply_borrowed` rather than `apply`, and the reason is a deliberate
    /// property of `viewer_core::Secret`: this worker holds one [`Source`] for the generation's
    /// whole life, §7.6.4.1's password inside it, and a `Secret` is not `Clone`.
    fn run_beside(
        &self,
        plan: &Plan,
        beside: Option<&Source>,
        level: Level,
    ) -> Result<(Report, BTreeMap<String, Vec<u8>>), WorkerError> {
        let mut sources: Vec<&Source> = vec![&self.source];
        sources.extend(beside);
        let sinks = MemorySinks::new();
        // The whole of `Policy` is the level today, so this is a copy with one field replaced
        // rather than a struct update — and it is written out so that a second field added to
        // `Policy` fails to compile here rather than being silently dropped for a consented
        // operation.
        let policy = Policy {
            restrictions: level,
        };
        let report = apply_borrowed(plan, &sources, &sinks, &policy, &self.budget)?;
        let files = sinks.into_outputs().into_iter().collect();
        Ok((report, files))
    }

    /// One in-place edit, as the whole updated document and what was said on the way.
    ///
    /// The output is §7.5.6's: "the contents of a PDF file can be updated incrementally without
    /// rewriting the entire file … changes shall be appended to the end of the file, leaving its
    /// original contents intact", so what comes back is the source's bytes and then the update.
    fn amend(
        &self,
        edit: update::Edit,
        beside: Option<&Source>,
        level: Level,
    ) -> Result<Answer, WorkerError> {
        let plan = Plan::Update(UpdatePlan {
            source: 0,
            edit,
            names: page_pattern()?,
        });
        let (report, files) = self.run_beside(&plan, beside, level)?;
        if let Some(declined) = report.refused.first() {
            return Err(WorkerError::Declined {
                subject: declined.subject.clone(),
                detail: declined.detail.clone(),
            });
        }
        let bytes = files
            .into_values()
            .next()
            .ok_or_else(|| WorkerError::NotPresent(String::from("the update wrote no document")))?;
        Ok(Answer::Written {
            bytes,
            warnings: report
                .warnings
                .into_iter()
                .map(|warning| warning.detail)
                .collect(),
        })
    }

    /// The one output a single-file plan wrote, or the reason there is none.
    fn only(&self, plan: &Plan, level: Level) -> Result<Answer, WorkerError> {
        let (report, files) = self.run(plan, level)?;
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
            // **ADR 0874's second round trip, and the one place a level other than this
            // worker's own is used.** The wrapper says a person was asked about this operation
            // and said yes, so the operation runs at the level `CLAUDE.md` says "shall always be
            // possible" — which is what a yes *is*. Nothing here can check that the question was
            // put; that is `crate::Vfs`'s, which is the only thing that builds this wrapper.
            Query::Consented(inner) => self.answer(inner, Level::Off),
            other => self.answer(other, self.policy.restrictions),
        }
    }

    fn ask_consented(&self, query: &Query) -> Result<Answer, WorkerError> {
        self.answer(query, Level::Off)
    }
}

impl InProcess {
    /// One question, under the level that governs *this* question.
    ///
    /// The level is a parameter rather than `self.policy`'s because of
    /// [`Query::Consented`]: everything else about the worker is the same, and a second worker
    /// at a second level would be a second confinement, a second parse and a second copy of
    /// §7.6.4.1's password.
    fn answer(&self, query: &Query, level: Level) -> Result<Answer, WorkerError> {
        match query {
            // `CLAUDE.md` principle 3's two round trips, split off so that this function stays
            // under this tree's own line-count lint — and split at the line the round drew
            // anyway: neither of them is a question about what the document *contains*.
            Query::Consult { .. } | Query::Consented(_) => self.policy_answer(query, level),
            Query::PageCount => {
                let document = self.document()?;
                Ok(Answer::Count(Pages::new(&document).len()))
            }
            Query::ExtractPage { page } => self.only(
                &Plan::Split(SplitPlan {
                    source: 0,
                    pages: Self::one_page(*page)?,
                    pieces: Pieces::EachPage,
                    names: page_pattern()?,
                }),
                level,
            ),
            Query::RenderPage { page, dpi } => self.only(
                &Plan::Render(RenderPlan {
                    source: 0,
                    pages: Self::one_page(*page)?,
                    // Dots per inch over ISO 32000-2 §8.3.2.3's 72 units to the inch, which is
                    // `Sizing::Dpi`'s own statement of the conversion; nothing is computed here.
                    size: Sizing::Dpi(dpi_as_scale(*dpi)),
                    format: ImageFormat::Png,
                    page_box: None,
                    annotations: true,
                    names: page_pattern()?,
                    strips: self.strips,
                }),
                level,
            ),
            Query::ExtractImages { page } => {
                let (report, files) =
                    self.run(&Plan::Images(images_plan(Self::one_page(*page)?)?), level)?;
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
                let (report, _) = self.run(
                    &Plan::Attachments(AttachmentsPlan {
                        source: 0,
                        action: Action::List,
                    }),
                    level,
                )?;
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
            Query::ExtractAttachment { name } => self.only(
                &Plan::Attachments(AttachmentsPlan {
                    source: 0,
                    action: Action::Save {
                        name: name.clone(),
                        names: page_pattern()?,
                    },
                }),
                level,
            ),
            // Named rather than caught, so that a query added to this module fails to compile
            // here as well as on the wire.
            meta @ (Query::Information | Query::MetadataStream | Query::Outline) => {
                self.meta_answer(meta)
            }
            write @ (Query::InsertPages { .. }
            | Query::DeletePage { .. }
            | Query::Attach { .. }
            | Query::Detach { .. }
            | Query::SetInformation { .. }) => self.write_answer(write, level),
        }
    }

    /// `CLAUDE.md` principle 3's two round trips: the question, and the operation with a yes
    /// behind it.
    ///
    /// `pdf_transform::consult` is the same call `apply` makes, so a face that asks and then
    /// acts is answered by one reading rather than by two that could disagree (ADR 0874).
    fn policy_answer(&self, query: &Query, level: Level) -> Result<Answer, WorkerError> {
        match query {
            Query::Consult { operation } => Ok(Answer::Consulted(consult(
                level,
                &self.document()?,
                *operation,
            ))),
            // Unreachable through `Worker::ask`, which strips the wrapper before this is called;
            // named rather than caught so that the wire cannot smuggle one past.
            Query::Consented(inner) => self.answer(inner, Level::Off),
            // `InProcess::answer` is this function's one caller and names the two it sends.
            Query::PageCount
            | Query::ExtractPage { .. }
            | Query::RenderPage { .. }
            | Query::ExtractImages { .. }
            | Query::PageText { .. }
            | Query::AttachmentInventory
            | Query::ExtractAttachment { .. }
            | Query::Information
            | Query::MetadataStream
            | Query::Outline
            | Query::InsertPages { .. }
            | Query::DeletePage { .. }
            | Query::Attach { .. }
            | Query::Detach { .. }
            | Query::SetInformation { .. } => Err(WorkerError::Mismatched {
                got: "a question about the document",
                wanted: "a consultation or a consent",
            }),
        }
    }

    /// The three answers `meta/` holds, which are the ones no transform verb covers: §14.3.3's
    /// information dictionary, §14.3.2's stream and §12.3.3's outline.
    fn meta_answer(&self, query: &Query) -> Result<Answer, WorkerError> {
        let document = self.document()?;
        match query {
            Query::Information => Ok(Answer::Bytes(
                information_json(&Information::read(&document)).into_bytes(),
            )),
            Query::MetadataStream => {
                Ok(metadata_stream(&document).map_or(Answer::Absent, Answer::Bytes))
            }
            Query::Outline => {
                let pages = Pages::new(&document);
                let outline = Outline::read(&document, &pages);
                let labels = PageLabels::read(&document);
                Ok(Answer::Bytes(
                    outline_json(&document, &pages, &labels, &outline).into_bytes(),
                ))
            }
            // `Worker::ask` is this function's one caller and names the three it sends.
            Query::PageCount
            | Query::ExtractPage { .. }
            | Query::RenderPage { .. }
            | Query::ExtractImages { .. }
            | Query::PageText { .. }
            | Query::AttachmentInventory
            | Query::ExtractAttachment { .. }
            | Query::InsertPages { .. }
            | Query::DeletePage { .. }
            | Query::Attach { .. }
            | Query::Detach { .. }
            | Query::SetInformation { .. }
            | Query::Consult { .. }
            | Query::Consented(_) => Err(WorkerError::Mismatched {
                got: "a question about the document",
                wanted: "one of meta/'s three",
            }),
        }
    }

    /// The five write queries, split off so that `Worker::ask` stays under this tree's own
    /// line-count lint — and split at the line the round drew anyway: everything above answers a
    /// question about the document, everything here changes it.
    fn write_answer(&self, query: &Query, level: Level) -> Result<Answer, WorkerError> {
        match query {
            Query::InsertPages { at, document } => {
                // The incoming document is opened for this one operation and let go of again:
                // a mount holds the document it is over, and a `cp` names its second file once.
                let beside = Source::new(document.clone());
                self.amend(
                    update::Edit::InsertPages { from: 1, at: *at },
                    Some(&beside),
                    level,
                )
            }
            Query::DeletePage { page } => {
                self.amend(update::Edit::DeletePage { page: *page }, None, level)
            }
            Query::Attach { name, bytes } => {
                let (report, files) = self.run(
                    &Plan::Attachments(AttachmentsPlan {
                        source: 0,
                        action: Action::Attach {
                            payload: Payload::new(bytes.clone()),
                            name: name.clone(),
                            description: None,
                            // No clock in this tree, and none here: the same file attached twice is
                            // the same bytes (RFC 0002 section 9's first layer). A face that wants
                            // Table 45's dates states them, and none does yet.
                            date: None,
                            names: page_pattern()?,
                            on_page: None,
                        },
                    }),
                    level,
                )?;
                written(report, files)
            }
            Query::Detach { name } => {
                let (report, files) = self.run(
                    &Plan::Attachments(AttachmentsPlan {
                        source: 0,
                        action: Action::Remove {
                            name: name.clone(),
                            names: page_pattern()?,
                        },
                    }),
                    level,
                )?;
                written(report, files)
            }
            Query::SetInformation { json } => self.amend(
                update::Edit::SetInformation {
                    entries: information_entries(json)?,
                },
                None,
                level,
            ),
            // Every question about the document is `Worker::ask`'s, which is this function's one
            // caller; naming them keeps the wire's own property — nothing is dropped in silence.
            Query::PageCount
            | Query::ExtractPage { .. }
            | Query::RenderPage { .. }
            | Query::ExtractImages { .. }
            | Query::PageText { .. }
            | Query::AttachmentInventory
            | Query::ExtractAttachment { .. }
            | Query::Information
            | Query::MetadataStream
            | Query::Outline
            | Query::Consult { .. }
            | Query::Consented(_) => Err(WorkerError::Mismatched {
                got: "a question",
                wanted: "a write",
            }),
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
        .map_err(|error: pattern::PatternError| WorkerError::Refused(error.to_string()))
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
            .map_err(|error: pattern::PatternError| WorkerError::Refused(error.to_string()))?,
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
                    .map(|item| item_json(document, labels, &indices, item))
                    .collect(),
            ),
        ),
    ])
    .render()
}

/// One outline item, and its children under it.
fn item_json(
    document: &Document,
    labels: &PageLabels,
    indices: &BTreeMap<pdf_syntax::ObjectId, usize>,
    item: &Item,
) -> Value {
    let index = item
        .destination
        .and_then(|destination| destination.page_index_with(document, indices));
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
                    .map(|child| item_json(document, labels, indices, child))
                    .collect(),
            ),
        ),
    ])
}

/// The one output an update wrote, with what the transform said on the way.
fn written(report: Report, files: BTreeMap<String, Vec<u8>>) -> Result<Answer, WorkerError> {
    if let Some(declined) = report.refused.first() {
        return Err(WorkerError::Declined {
            subject: declined.subject.clone(),
            detail: declined.detail.clone(),
        });
    }
    let bytes = files
        .into_values()
        .next()
        .ok_or_else(|| WorkerError::NotPresent(String::from("the update wrote no document")))?;
    Ok(Answer::Written {
        bytes,
        warnings: report
            .warnings
            .into_iter()
            .map(|warning| warning.detail)
            .collect(),
    })
}

/// `meta/info.json`'s own keys, and the Table 349 key each names.
///
/// The one place the two spellings meet, in both directions: [`information_json`] writes the
/// left column and this reads it. A file that named a key on neither side would be a file this
/// program invented an entry from, so the list is closed and a key outside it is refused.
const INFORMATION_NAMES: [(&str, &str); 9] = [
    ("title", "Title"),
    ("author", "Author"),
    ("subject", "Subject"),
    ("keywords", "Keywords"),
    ("creator", "Creator"),
    ("producer", "Producer"),
    ("created", "CreationDate"),
    ("modified", "ModDate"),
    ("trapped", "Trapped"),
];

/// The §14.3.3 entries a written `meta/info.json` states.
///
/// **The file is the whole of Table 349 and nothing else.** A key the file omits is an entry the
/// document shall no longer state, which is what makes `cat info.json > info.json` a no-op and
/// therefore what makes the read and the write one thing rather than two. A key the dictionary
/// holds that Table 349 does not define is untouched: §14.3.3 permits such a key —"[w]here a
/// document information dictionary contains keys other than `CreationDate` and `ModDate` , the
/// value associated with any such key shall be a text string" — and this file does not show it,
/// so this file is not the place it would be deleted from.
fn information_entries(json: &[u8]) -> Result<Vec<update::InfoEntry>, WorkerError> {
    let text = std::str::from_utf8(json)
        .map_err(|_| WorkerError::Refused(String::from("meta/info.json: not UTF-8")))?;
    let stated = read_flat_object(text).map_err(WorkerError::Refused)?;
    let mut entries = Vec::with_capacity(INFORMATION_NAMES.len());
    for (name, _) in &stated {
        if !INFORMATION_NAMES.iter().any(|(known, _)| known == name) {
            return Err(WorkerError::Refused(format!(
                "meta/info.json: {name:?} is not one of its keys ({})",
                INFORMATION_NAMES
                    .iter()
                    .map(|(known, _)| *known)
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    }
    for (name, key) in INFORMATION_NAMES {
        let value = stated
            .iter()
            .find(|(stated, _)| stated == name)
            .and_then(|(_, value)| value.clone());
        entries.push(update::InfoEntry {
            key: key.to_owned(),
            value,
        });
    }
    Ok(entries)
}

/// RFC 8259's object, restricted to what `meta/info.json` is: string keys, string or null
/// values, no nesting.
///
/// A reader for one file's grammar rather than a JSON library, and the restriction is the point:
/// everything this file can hold is a Table 349 entry, so a nested value, a number or a boolean
/// is a file that means something this edit has no way to write — refused by name rather than
/// coerced (trap 5). The escapes are exactly the ones `pdf_transform::json` writes, so the two
/// are inverses.
fn read_flat_object(text: &str) -> Result<Vec<(String, Option<String>)>, String> {
    let mut chars = text.chars().peekable();
    let mut out: Vec<(String, Option<String>)> = Vec::new();
    let skip = |chars: &mut std::iter::Peekable<std::str::Chars<'_>>| {
        while chars.peek().is_some_and(char::is_ascii_whitespace) {
            chars.next();
        }
    };
    skip(&mut chars);
    if chars.next() != Some('{') {
        return Err(String::from("meta/info.json: it is not a JSON object"));
    }
    loop {
        skip(&mut chars);
        match chars.peek() {
            Some('}') => {
                chars.next();
                break;
            }
            Some(',') if !out.is_empty() => {
                chars.next();
                continue;
            }
            Some('"') => {}
            other => {
                return Err(format!(
                    "meta/info.json: expected a key and found {}",
                    other.map_or_else(|| String::from("the end of the file"), |c| format!("{c:?}"))
                ));
            }
        }
        let key = read_string(&mut chars)?;
        skip(&mut chars);
        if chars.next() != Some(':') {
            return Err(format!(
                "meta/info.json: {key:?} is not followed by a colon"
            ));
        }
        skip(&mut chars);
        let value = match chars.peek() {
            Some('"') => Some(read_string(&mut chars)?),
            Some('n') => {
                for expected in "null".chars() {
                    if chars.next() != Some(expected) {
                        return Err(format!("meta/info.json: {key:?} is not a string or null"));
                    }
                }
                None
            }
            _ => {
                return Err(format!(
                    "meta/info.json: {key:?} is not a string or null, and every entry §14.3.3's \
                     Table 349 defines is one of those two"
                ));
            }
        };
        if out.iter().any(|(known, _)| *known == key) {
            return Err(format!("meta/info.json: {key:?} is stated twice"));
        }
        out.push((key, value));
    }
    skip(&mut chars);
    match chars.next() {
        None => Ok(out),
        Some(extra) => Err(format!(
            "meta/info.json: {extra:?} after the object, and the file is one object"
        )),
    }
}

/// One RFC 8259 string, with the escapes `pdf_transform::json` writes.
fn read_string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<String, String> {
    if chars.next() != Some('"') {
        return Err(String::from(
            "meta/info.json: a string does not start with a quote",
        ));
    }
    let mut out = String::new();
    loop {
        let Some(character) = chars.next() else {
            return Err(String::from("meta/info.json: a string is not closed"));
        };
        match character {
            '"' => return Ok(out),
            '\\' => {
                let Some(escape) = chars.next() else {
                    return Err(String::from("meta/info.json: a string ends in a backslash"));
                };
                match escape {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'u' => {
                        let mut digits = String::new();
                        for _ in 0..4 {
                            digits.push(chars.next().ok_or_else(|| {
                                String::from("meta/info.json: a \\u escape is cut short")
                            })?);
                        }
                        let code = u32::from_str_radix(&digits, 16).map_err(|_| {
                            format!("meta/info.json: \\u{digits} is not four hexadecimal digits")
                        })?;
                        // A lone surrogate is not a character, and this file's writer emits
                        // `\u` only for the controls below U+0020; a pair would be a file
                        // somebody else wrote, and refusing is better than inventing U+FFFD.
                        out.push(char::from_u32(code).ok_or_else(|| {
                            format!("meta/info.json: \\u{digits} is not a character")
                        })?);
                    }
                    other => {
                        return Err(format!("meta/info.json: \\{other} is not an escape"));
                    }
                }
            }
            other => out.push(other),
        }
    }
}
