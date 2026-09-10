//! `archive` — one document converted to a stated part and level of ISO 19005 (PDF/A).
//!
//! # Provenance, and the one rule about quotation
//!
//! ISO 19005-2:2011 and ISO 19005-4:2020 are licensed to a single reader
//! (`doc/questions/A16`), so **nothing here quotes them**: a requirement is named by the
//! identifier `pdf-archive` gives it and cited by clause, and a reader who needs the sentence
//! opens `doc/pdfa/` at the clause. ISO 32000-2 is quoted normally, because `doc/md/` carries it
//! and `tools/conformance` checks the quotation. A section sign marks a clause of ISO 32000-2 and
//! nothing else.
//!
//! # Three stages, and the middle one is the product
//!
//! `doc/pdf-a-conversion-limits.md` is this verb's specification of behaviour, and its shape is
//! why converting is not one operation:
//!
//! 1. **Validate.** `pdf_archive::check` holds the document to the target and answers a
//!    requirement at a time. Nothing here re-reads the standard: the validator is the reading.
//! 2. **Decide.** Each requirement the document *failed* gets one of three answers, and they are
//!    three different things rather than three degrees of one thing — sections 2, 3 and 4 of
//!    `doc/pdf-a-conversion-limits.md` are a refusal, an authorised loss and a default.
//!    [`Decision`] is that answer, [`REMEDIES`] is the table it comes from, and everything else in
//!    this module hangs off it.
//! 3. **Apply.** The rewrites the decisions call for, through the serializer this tree already
//!    has, and then **the output is validated again**. A file leaves this verb only if it
//!    conforms; anything less is refused, with the requirements it would still have failed
//!    named, and a requirement its *source* met is named again as the worse case. That is what
//!    makes the report's claim about the output a measurement rather than a promise, and it is
//!    what catches the cases the decision table cannot see.
//!
//! ADR 0947 has the argument for all three, and for the two rules the whole verb rests on:
//! nothing is changed that no failed requirement asked for, and no file is written that does not
//! conform.
//!
//! # What this verb does and what it refuses
//!
//! The mechanical rewrites of `doc/pdf-a-conversion-limits.md` section 4.7, and the two section 4
//! *defaults* without which almost no real document can be made to conform at all: **the output
//! intent** (section 4.1) and **the identification schema** (section 4.2). **Everything else is
//! refused by name**: fonts (section 4.9), the structure tree (section 5.1), encryption
//! (section 3.5), attachments (section 3.1), `/Info` reconciliation and the extension schemas a
//! producer's private XMP property needs (section 4.2's other halves). A document needing one of
//! those is told which requirement, at which clause, this converter cannot yet meet — and no file
//! is written. A stub that wrote one anyway would be producing a file wearing a conformance claim
//! it had not earned, which is the failure section 7 of that document exists to prevent.
//!
//! # The report is an output, not a log
//!
//! `doc/adr/0927`: the owner's four permissions to write something a producer did not — `A18`,
//! `A21`, `A48`, `A50` — all carry one condition, that **what was written is reported**, named
//! per document rather than inferable from a diff. `A18` is the first of the four this verb
//! exercises, and its condition is why [`Decision::Stated`] exists as a variant of its own: an
//! output intent loses nothing and all the same changes what every device colour in the file
//! means to a conforming reader, so the sentence saying so travels with the decision rather than
//! being left to a caller to remember. [`Conversion`] is the report, and it reaches a caller
//! through [`crate::Report::archive`] whether or not a file was written.
//!
//! # One cost, stated
//!
//! The output is assembled in memory before it reaches the sink, because stage 3 validates what
//! it wrote and a validator needs a document rather than a stream of bytes. So this verb's peak
//! is the whole output file, where every other writer in this crate hands the sink an `Arc` and
//! keeps nothing. That is the price of the re-validation, and the re-validation is what the
//! verdict rests on.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::Write as _;

use pdf_archive::survey::DeviceFamily;
use pdf_archive::{Flavour, Judgement, Level, Outcome, Target, Verdict};
use pdf_model::Pages;
use pdf_model::icc::Identification;
use pdf_model::xmp::{self, Schema};
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::{Assembly, Form, ObjectStreams, Options, Streams, flate_encode};
use pdf_syntax::{Document, Version, serialize::serialize};

use crate::json::Value;
use crate::pattern::{Fill, Pattern};
use crate::{Declined, Origin, Output, Refusal, Report, Sinks};

/// The deepest a rewritten object's value tree is walked for references.
///
/// [`crate::optimize`]'s number, for its reason: one more than `pdf_syntax::Limits::DEFAULT`'s
/// `max_depth`, so that every object the parser admitted has its references reached.
const MAX_WALK_DEPTH: usize = 257;

/// zlib's effort for a stream this verb re-encodes.
///
/// The serializer's own default, measured in ADR 0842 over the pdf.js corpus: level 9 saves
/// 13.30% of the whole against level 6's 12.60%. A conversion is not on a latency path, so the
/// same choice is made here and there is no flag, because a caller cannot choose the filter
/// either — ISO 19005 chose it.
const COMPRESSION_LEVEL: u32 = 9;

/// The ICC profile this program ships, and the default destination profile of an output intent
/// it adds.
///
/// `doc/questions/A18`: ship the standard sRGB profile, with a flag to override it.
/// `data/icc/PROVENANCE.md` records which of the ICC's four sRGB profiles this is and why — the
/// short answer being that ISO 19005-2 section 6.2.4.2 names the ICC editions a profile may
/// conform to and the ICC's headline v4 download conforms to none of them. It is `static` data,
/// so it costs no parse time until something asks for it.
const SRGB: &[u8] = include_bytes!("../../../data/icc/sRGB2014.icc");

/// One document converted to a stated part and level of ISO 19005.
#[derive(Debug, Clone, PartialEq)]
pub struct ArchivePlan {
    /// Which source.
    pub source: usize,
    /// How the one output is named.
    pub names: Pattern,
    /// The part, level or flavour to convert to.
    ///
    /// All six of `pdf_archive`'s targets are selectable, and none is a default: section 9 of
    /// `doc/pdf-a-conversion-limits.md` is the case of a user whose deposit rule names one and
    /// for whom "use PDF/A-4f instead" is a restatement of their problem rather than an answer.
    pub target: Target,
    /// What the caller has authorised this conversion to lose.
    pub authorised: Authorisations,
    /// The ICC profile an output intent this conversion adds states as its destination profile.
    ///
    /// `None` is the shipped sRGB profile, which is `doc/questions/A18`'s answer and
    /// [`SRGB`]'s reason for existing. A caller supplies one — `--output-intent-profile` —
    /// for a document produced for a press, which is the case
    /// `doc/pdf-a-conversion-limits.md` section 10.1 says nobody but the document's owner can
    /// decide. A supplied profile's own `cprt` tag is named in the report, because a user
    /// embedding somebody else's profile is entitled to be told whose it is.
    pub profile: Option<std::sync::Arc<[u8]>>,
}

/// Something a conversion can throw away, which a user has to authorise first.
///
/// section 3 of `doc/pdf-a-conversion-limits.md`: "Each of these can be done. Each throws something
/// away. None of them may happen silently." A batch tool has nobody to ask — the same wall
/// `crate::Refusal::Unanswered` meets for Table 22's restrictions — so the question is asked
/// *before* the run, on the command line, and a loss nobody authorised stops the conversion
/// instead of happening quietly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Loss {
    /// section 4.7: an image's `/Interpolate` turned off.
    ///
    /// The one row of that table marked *not quite mechanical*, and the reason is in the row:
    /// interpolation is image smoothing, so a low-resolution image that had it on will look
    /// blockier afterwards. Nothing is deleted and no mark moves; what changes is how a
    /// conforming reader is told to sample the image it already has.
    ImageSmoothing,
}

impl Loss {
    /// Every loss this converter knows how to ask about.
    pub const ALL: [Self; 1] = [Self::ImageSmoothing];

    /// The word a caller authorises it by.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::ImageSmoothing => "image-smoothing",
        }
    }

    /// What is lost, in one sentence for a person.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::ImageSmoothing => {
                "image smoothing is turned off, so a low-resolution image will look blockier"
            }
        }
    }

    /// The loss a caller's word names.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|loss| loss.word() == word)
    }
}

/// What the caller has authorised this conversion to lose.
///
/// One field per [`Loss`] rather than a set, so that adding a loss to the table is a compile
/// error everywhere it has to be answered rather than a word nobody matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Authorisations {
    /// Whether [`Loss::ImageSmoothing`] was authorised.
    pub image_smoothing: bool,
}

impl Authorisations {
    /// Whether this loss was authorised.
    #[must_use]
    pub const fn grants(self, loss: Loss) -> bool {
        match loss {
            Loss::ImageSmoothing => self.image_smoothing,
        }
    }

    /// Authorises one loss.
    pub const fn authorise(&mut self, loss: Loss) {
        match loss {
            Loss::ImageSmoothing => self.image_smoothing = true,
        }
    }
}

/// One rewrite this converter performs, named so that a report can say what was done.
///
/// Each is a *mechanism*; which clause required it comes from the requirement it answers, so
/// nothing here restates a clause number the validator already carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rewrite {
    /// The file header states the version the target's part admits.
    ///
    /// Not an object rewrite at all: every output of this verb is a whole new file, so the
    /// header is written fresh. What is decided is the version it states — [`version_for`].
    FileHeader,
    /// The catalog's `/Version` is restated in the shape the target requires.
    CatalogVersion,
    /// The whole file is written afresh, in the shape `pdf_syntax::write` emits.
    ///
    /// The remedy for every row of [`WRITER_EMITS`]: nothing is decided about the document, and
    /// there is no place to count, because the rewrite is the file. What it fixes is the
    /// *syntax* a producer wrote — an odd hexadecimal digit count, a `/Length` that disagrees
    /// with the bytes, a keyword's line endings — and it fixes it everywhere at once.
    WholeFileRewritten,
    /// Every stream whose filter chain names `LZWDecode` is decoded and re-encoded as one
    /// §7.4.4's `FlateDecode`.
    ///
    /// The decoded bytes are identical, so every mark the producer specified is the same mark;
    /// what changes is §7.4's encoding of them, which is a statement about the file rather than
    /// about the page. §7.4.4's NOTE 1 is why the file usually shrinks:
    ///
    /// > Because of its cascaded adaptive Huffman coding, Flate-encoded output is usually much
    /// > more compact than LZW-encoded output for the same input.
    FlateInsteadOfLzw,
    /// Every image `XObject` that states an `/Interpolate` other than false has it set false.
    InterpolationOff,
    /// `/Alternates` and `/OPI` are removed from every image `XObject`.
    ImageAlternatesAndOpi,
    /// `/OPI` is removed from every form `XObject`.
    FormOpi,
    /// A form `XObject`'s `/PS`, and a `/Subtype2` whose value is `PS`, are removed.
    FormPostScript,
    /// Every PostScript `XObject` is dropped, and with it the resource entries naming it.
    PostScriptXObject,
    /// The catalog's `/Requirements` is removed.
    RequirementsDictionary,
    /// The name dictionary's `/AlternatePresentations` is removed.
    AlternatePresentations,
    /// Every page's `/PresSteps` is removed.
    PresentationSteps,
    /// The catalog's `/OutputIntents` gains a PDF/A entry naming a destination profile.
    ///
    /// **The one rewrite of this verb that changes what the file *means*** rather than only what
    /// it holds, which is why it is a [`Decision::Stated`] and not a [`Decision::Mechanical`].
    OutputIntent,
    /// The document's XMP packet states the identification schema the target's part requires,
    /// and is created where the document had none.
    IdentificationSchema,
}

impl Rewrite {
    /// What the rewrite does, in one sentence for a person.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::FileHeader => "the file header states the version this part admits",
            Self::CatalogVersion => "the catalog's /Version is restated in the required shape",
            Self::WholeFileRewritten => {
                "the file is written afresh, in the syntax ISO 19005 requires"
            }
            Self::FlateInsteadOfLzw => {
                "an LZWDecode stream is decoded and re-encoded as one FlateDecode, byte for byte"
            }
            Self::InterpolationOff => "an image's /Interpolate is set false",
            Self::ImageAlternatesAndOpi => "an image's /Alternates and /OPI are removed",
            Self::FormOpi => "a form XObject's /OPI is removed",
            Self::FormPostScript => "a form XObject's /PS and PostScript /Subtype2 are removed",
            Self::PostScriptXObject => {
                "a PostScript XObject is dropped, and the resource entries naming it with it"
            }
            Self::RequirementsDictionary => "the catalog's /Requirements is removed",
            Self::AlternatePresentations => {
                "the name dictionary's /AlternatePresentations is removed"
            }
            Self::PresentationSteps => "a page's /PresSteps is removed",
            Self::OutputIntent => {
                "the catalog states a PDF/A output intent whose destination profile says what \
                 this file's device colours mean"
            }
            Self::IdentificationSchema => {
                "the document's XMP packet states this part's identification schema, and is \
                 created where the document had none"
            }
        }
    }

    /// A stable word for the report's machine-readable form.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::FileHeader => "file-header",
            Self::CatalogVersion => "catalog-version",
            Self::WholeFileRewritten => "whole-file-rewritten",
            Self::FlateInsteadOfLzw => "flate-instead-of-lzw",
            Self::InterpolationOff => "interpolation-off",
            Self::ImageAlternatesAndOpi => "image-alternates-and-opi",
            Self::FormOpi => "form-opi",
            Self::FormPostScript => "form-postscript",
            Self::PostScriptXObject => "postscript-xobject",
            Self::RequirementsDictionary => "requirements-dictionary",
            Self::AlternatePresentations => "alternate-presentations",
            Self::PresentationSteps => "presentation-steps",
            Self::OutputIntent => "output-intent",
            Self::IdentificationSchema => "identification-schema",
        }
    }
}

/// Why a requirement the document failed cannot be answered by this conversion.
///
/// Three reasons, and they are three different facts about the world rather than three shades
/// of *no*. section 2.0 of `doc/pdf-a-conversion-limits.md` is the argument for separating them: a
/// user who is told "not this target" has somewhere to go, and one who is told "not yet" knows
/// the answer will change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Because {
    /// The fix is on the far side of ADR 0816's fence: it would edit a content stream or invent
    /// a mark. Section 5.2 of that document: the converter does not rewrite the producer's page to
    /// reach conformance.
    TheFence(&'static str),
    /// This converter does not implement the fix yet, and the sentence names which slice owes
    /// it.
    ///
    /// Honest rather than a stub: `doc/pdf-a-conversion-limits.md` names the resource or the
    /// decision each of these waits on, and a converter that wrote the file anyway would be
    /// asserting a conformance it had not reached.
    NotBuiltYet(&'static str),
    /// No conforming file can be made from this document *for the target asked for*, whatever
    /// is implemented. Section 2.5's implementation limits and section 2.3's external data are the
    /// standing cases, and section 1.1's table is what a user does about it.
    NotThisTarget(&'static str),
}

impl Because {
    /// The reason, in one sentence.
    #[must_use]
    pub const fn sentence(self) -> &'static str {
        match self {
            Self::TheFence(why) | Self::NotBuiltYet(why) | Self::NotThisTarget(why) => why,
        }
    }

    /// A stable word for the report's machine-readable form.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::TheFence(_) => "the-fence",
            Self::NotBuiltYet(_) => "not-built-yet",
            Self::NotThisTarget(_) => "not-this-target",
        }
    }
}

/// What the converter decided about one requirement the document did not meet.
///
/// **The middle stage's whole product.** section 2, section 3 and section 4 of
/// `doc/pdf-a-conversion-limits.md` are three different answers to a failed requirement — a
/// refusal, a loss somebody has to authorise, and a default that is right — and a converter
/// that ran them together would be unable to say which one a given file got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The *Mechanical* class: the requirement is met by a rewrite that loses nothing, every time.
    Mechanical(Rewrite),
    /// section 3: it can be done, it loses something, and the caller authorised it.
    Authorised {
        /// What is lost.
        loss: Loss,
        /// The rewrite that loses it.
        rewrite: Rewrite,
    },
    /// section 3: the same, and nobody authorised it. The conversion stops.
    Unauthorised {
        /// What would have been lost.
        loss: Loss,
        /// The rewrite that would have lost it.
        rewrite: Rewrite,
    },
    /// section 4: the requirement is met by writing down an interpretation the standard defines,
    /// which changes what the file *asserts* without changing what it holds.
    ///
    /// **The class `doc/adr/0927`'s four permissions belong to**, and the reason it is not
    /// [`Self::Mechanical`]: nothing is lost, and something all the same is different afterwards.
    /// `doc/questions/A48` draws the line these sit on — *state an interpretation the standard
    /// defines; never fill in an absence* — and each of the four is conditional on the same
    /// thing, that what was written is reported. So the sentence saying what a different reader
    /// may now do differently rides in the decision itself rather than being left to a caller to
    /// remember.
    Stated {
        /// The rewrite that states it.
        rewrite: Rewrite,
        /// What is now asserted that was not before, in one sentence for a person.
        reinterprets: &'static str,
    },
    /// section 2: no file is written, and the reason says which of three kinds of *no* this is.
    Refused(Because),
}

impl Decision {
    /// Whether this decision lets the conversion proceed.
    #[must_use]
    pub const fn proceeds(self) -> bool {
        matches!(
            self,
            Self::Mechanical(_) | Self::Stated { .. } | Self::Authorised { .. }
        )
    }

    /// The rewrite this decision performs, where it performs one.
    #[must_use]
    pub const fn rewrite(self) -> Option<Rewrite> {
        match self {
            Self::Mechanical(rewrite)
            | Self::Stated { rewrite, .. }
            | Self::Authorised { rewrite, .. } => Some(rewrite),
            Self::Unauthorised { .. } | Self::Refused(_) => None,
        }
    }

    /// A stable word for the report's machine-readable form.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Mechanical(_) => "mechanical",
            Self::Stated { .. } => "stated",
            Self::Authorised { .. } => "authorised",
            Self::Unauthorised { .. } => "unauthorised",
            Self::Refused(_) => "refused",
        }
    }
}

/// What the converter knows how to do about one of the validator's requirements.
///
/// The answer half of [`Remedy`], before the caller's authorisations are known. `Decision` is
/// what it becomes once they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    /// A rewrite that loses nothing.
    Mechanical(Rewrite),
    /// A rewrite that states an interpretation, with the sentence [`Decision::Stated`] carries.
    ///
    /// The `Option<DeviceFamily>` is the one thing a static table cannot answer about an output
    /// intent: ISO 19005 section 6.2.4.3 licenses `DeviceRGB` through an **RGB** destination
    /// profile and `DeviceCMYK` through a **CMYK** one, so which of those rows an intent answers
    /// depends on the profile in hand rather than on the requirement alone. `None` is a row any
    /// PDF/A output intent answers whatever its profile is.
    Stated(Option<DeviceFamily>, Rewrite, &'static str),
    /// A rewrite that loses something, which the caller must authorise.
    Loses(Loss, Rewrite),
}

impl Answer {
    /// The rewrite this answer performs, whatever the caller has authorised.
    const fn rewrite(self) -> Rewrite {
        match self {
            Self::Mechanical(rewrite) | Self::Stated(_, rewrite, _) | Self::Loses(_, rewrite) => {
                rewrite
            }
        }
    }
}

/// One row of the converter's decision table: what to do about one of the validator's
/// requirements.
///
/// **Data for `pdf_archive::Requirement`'s reason.** The remedies are a few dozen, most are
/// one line, and every one has to be checkable against the requirement it answers by somebody
/// who is not reading Rust. A requirement absent from this table is not forgotten: it is
/// refused by name, with its own clause and its own sentence, by [`decide`].
#[derive(Debug, Clone, Copy)]
struct Remedy {
    /// The `pdf_archive` requirement identifier this answers.
    requirement: &'static str,
    /// What is done about it.
    answer: Answer,
}

/// What adding an output intent asserts, which is `doc/questions/A18`'s condition on allowing it.
///
/// The answer attaches this to the permission rather than to the code: the difference has to be
/// **visible in the report, not only in the bytes**. `doc/adr/0927` has the argument — an output
/// intent is what a conforming reader colour-manages device colours *through*, so writing one
/// down records the interpretation this renderer was applying anyway (§10.4.2) and at the same
/// time changes what a *different* reader is told those colours mean.
const OUTPUT_INTENT_REINTERPRETS: &str = "a PDF/A output intent is what a conforming reader \
     colour-manages device colours through, so every DeviceGray, DeviceRGB and DeviceCMYK value \
     in this file now means what the destination \
     profile says it means, and the same profile becomes the default blending space for \
     transparency. This renderer already showed those colours that way; another reader may not \
     have, and for content separated for a particular press the difference is real rather than \
     imperceptible — supply that press's profile with --output-intent-profile instead";

/// The one requirement identifier that will not fit beside its key inside 100 columns.
const CMYK_UNDER_PART_FOUR: &str =
    "graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent";

/// Why a colour requirement an output intent of another family answers is refused.
const WRONG_FAMILY: &str = "this requirement is licensed by a destination profile of its own \
     colour family, and the profile this conversion has is of another. Supply the right one with \
     --output-intent-profile; for CMYK with no profile to hand, \
     doc/pdf-a-conversion-limits.md section 10.1 states the DeviceN /DefaultCMYK construction the \
     standard's own §10.4.2.5 transform makes conforming, which this converter does not write yet";

/// Every requirement this converter can answer, and how.
///
/// `doc/pdf-a-conversion-limits.md` section 4.7's table is the list this slice takes, less the rows
/// that need something shipped. Three of that table's rows are absent on purpose and each has
/// its reason written where it belongs rather than here:
///
/// - **The `Crypt` filter.** The row says its removal "follows from section 3.5" — encryption — and
///   the requirement itself permits an `Identity` `Crypt` filter to stay. So a filter chain
///   this converter re-encodes for the `LZWDecode` rewrite drops its `Crypt` stages, and a
///   conforming `Identity` one is left alone: **nothing is changed that no failed requirement
///   asked for**, which is the rule that makes "a document already conforming is not rewritten"
///   true by construction rather than by a special case.
/// - **Hexadecimal string padding, `endstream`/`endobj` whitespace, `/Length`, the indirect
///   object syntax and the cross-reference keyword's line endings.** The row's own note says
///   it: "the serializer emits this shape anyway". Every one of them is a property of
///   `pdf_syntax::write`, so the remedy is the writer and the rewrite is the whole-file
///   rewrite. They are answered by [`Answer::Mechanical`] with [`Rewrite::FileHeader`]'s
///   sibling — see [`WRITER_EMITS`], which is the list of requirements the writer satisfies by
///   construction and which therefore need no rewrite of their own.
/// - **Names that are not valid UTF-8.** The row calls it a **Refusal**, not a rewrite, and
///   says why: renaming a font or a colourant would break the references that use it.
const REMEDIES: &[Remedy] = &[
    // ISO 19005-2 section 6.1.2, ISO 19005-4 section 6.1.2. Every output of this verb is a new
    // file whose header the serializer writes; `version_for` decides what it states.
    Remedy {
        requirement: "file-structure/file-header",
        answer: Answer::Mechanical(Rewrite::FileHeader),
    },
    // ISO 19005-4 section 6.1.12.
    Remedy {
        requirement: "file-structure/catalog-version-key",
        answer: Answer::Mechanical(Rewrite::CatalogVersion),
    },
    // ISO 19005-2 section 6.1.7.2, ISO 19005-4 section 6.1.6.2.
    Remedy {
        requirement: "file-structure/no-lzw-filter",
        answer: Answer::Mechanical(Rewrite::FlateInsteadOfLzw),
    },
    // ISO 19005-2 section 6.2.8.1, ISO 19005-4 section 6.2.7.1 — the one row of section 4.7's table
    // marked *not quite mechanical*, and therefore the one that has to be authorised.
    Remedy {
        requirement: "graphics/image-interpolation-is-off",
        answer: Answer::Loses(Loss::ImageSmoothing, Rewrite::InterpolationOff),
    },
    Remedy {
        requirement: "graphics/no-image-alternates-or-opi",
        answer: Answer::Mechanical(Rewrite::ImageAlternatesAndOpi),
    },
    // ISO 19005-2 section 6.2.9.1, ISO 19005-4 section 6.2.8.1.
    Remedy {
        requirement: "graphics/no-form-xobject-opi",
        answer: Answer::Mechanical(Rewrite::FormOpi),
    },
    // ISO 19005-2 section 6.2.9.1's other two keys, which part 4 does not state because PDF 2.0
    // defines neither.
    Remedy {
        requirement: "graphics/no-postscript-passthrough-in-a-form-xobject",
        answer: Answer::Mechanical(Rewrite::FormPostScript),
    },
    // ISO 19005-2 section 6.2.9.3.
    Remedy {
        requirement: "graphics/no-postscript-xobjects",
        answer: Answer::Mechanical(Rewrite::PostScriptXObject),
    },
    // ISO 19005-2 section 6.11, ISO 19005-4 section 6.12.
    Remedy {
        requirement: "document-requirements/no-requirements-dictionary",
        answer: Answer::Mechanical(Rewrite::RequirementsDictionary),
    },
    // ISO 19005-2 section 6.10, ISO 19005-4 section 6.11.
    Remedy {
        requirement: "alternate-presentations/none-in-the-name-dictionary",
        answer: Answer::Mechanical(Rewrite::AlternatePresentations),
    },
    Remedy {
        requirement: "alternate-presentations/no-presentation-steps",
        answer: Answer::Mechanical(Rewrite::PresentationSteps),
    },
    // ISO 19005-2 section 6.2.4.3, ISO 19005-4 section 6.2.4.3, and the two transparency
    // subclauses that turn on the same sentence — ISO 19005-2 section 6.2.10 and ISO 19005-4
    // section 6.2.9. Every one of them licenses a device colour space through a PDF/A output
    // intent, so one intent answers all of them at once; which of them it actually answers turns
    // on the destination profile's own colour family, which is what `Answer::Stated` carries.
    Remedy {
        requirement: "graphics/device-rgb-needs-a-default-or-an-rgb-output-intent",
        answer: Answer::Stated(
            Some(DeviceFamily::Rgb),
            Rewrite::OutputIntent,
            OUTPUT_INTENT_REINTERPRETS,
        ),
    },
    Remedy {
        requirement: "graphics/device-rgb-needs-a-default-a-blending-space-or-an-rgb-output-intent",
        answer: Answer::Stated(
            Some(DeviceFamily::Rgb),
            Rewrite::OutputIntent,
            OUTPUT_INTENT_REINTERPRETS,
        ),
    },
    Remedy {
        requirement: "graphics/device-cmyk-needs-a-default-or-a-cmyk-output-intent",
        answer: Answer::Stated(
            Some(DeviceFamily::Cmyk),
            Rewrite::OutputIntent,
            OUTPUT_INTENT_REINTERPRETS,
        ),
    },
    Remedy {
        requirement: CMYK_UNDER_PART_FOUR,
        answer: Answer::Stated(
            Some(DeviceFamily::Cmyk),
            Rewrite::OutputIntent,
            OUTPUT_INTENT_REINTERPRETS,
        ),
    },
    // Both parts license `DeviceGray` through a PDF/A output intent of *any* family, and neither
    // page-level rule below asks anything of the profile either.
    Remedy {
        requirement: "graphics/device-gray-needs-a-default-or-an-output-intent",
        answer: Answer::Stated(None, Rewrite::OutputIntent, OUTPUT_INTENT_REINTERPRETS),
    },
    Remedy {
        requirement: "graphics/device-gray-needs-a-default-or-a-current-output-intent",
        answer: Answer::Stated(None, Rewrite::OutputIntent, OUTPUT_INTENT_REINTERPRETS),
    },
    // ISO 19005-4 section 6.2.3's page-level rule binds only where the *document* states no
    // PDF/A output intent, so a document-level one answers it outright and PDF/A-4's page-level
    // intents are a facility this converter has no occasion to use. That is the clause read
    // rather than the feature declined: a page-level intent is for a document mixing an RGB body
    // with CMYK inserts, and choosing which pages get which profile is not a decision that can be
    // taken from the file.
    Remedy {
        requirement: "graphics/a-device-dependent-page-carries-an-output-intent",
        answer: Answer::Stated(None, Rewrite::OutputIntent, OUTPUT_INTENT_REINTERPRETS),
    },
    Remedy {
        requirement: "graphics/a-transparent-page-has-a-blending-space",
        answer: Answer::Stated(None, Rewrite::OutputIntent, OUTPUT_INTENT_REINTERPRETS),
    },
    Remedy {
        requirement: "graphics/a-transparent-page-has-a-blending-space-or-an-output-intent",
        answer: Answer::Stated(None, Rewrite::OutputIntent, OUTPUT_INTENT_REINTERPRETS),
    },
    // ISO 19005-2 section 6.2.10 and ISO 19005-4 section 6.2.9 make a transparency group's `CS`
    // subject to the colour subclauses, and ISO 19005-2 section 6.2.4.5 and ISO 19005-4 section
    // 6.2.4.5 do the same for what underlies an `Indexed` or a `Pattern`, section 6.2.4.4 for a
    // `Separation` or `DeviceN` alternate. Which family each of those turns out to need is a fact
    // about the document rather than about the requirement, so these rows state no family and the
    // output's own verdict is what decides whether the intent answered them — which is exactly the
    // case `doc/adr/0947`'s fourth decision built the net for.
    Remedy {
        requirement: "graphics/transparency-group-colour-spaces-obey-the-colour-rules",
        answer: Answer::Stated(None, Rewrite::OutputIntent, OUTPUT_INTENT_REINTERPRETS),
    },
    Remedy {
        requirement: "graphics/transparency-group-colour-spaces-obey-the-colour-rules-of-part-four",
        answer: Answer::Stated(None, Rewrite::OutputIntent, OUTPUT_INTENT_REINTERPRETS),
    },
    Remedy {
        requirement: "graphics/indexed-and-pattern-base-spaces-obey-the-colour-rules",
        answer: Answer::Stated(None, Rewrite::OutputIntent, OUTPUT_INTENT_REINTERPRETS),
    },
    Remedy {
        requirement: "graphics/indexed-and-pattern-base-spaces-obey-the-colour-rules-of-part-four",
        answer: Answer::Stated(None, Rewrite::OutputIntent, OUTPUT_INTENT_REINTERPRETS),
    },
    Remedy {
        requirement: "graphics/separation-alternate-spaces-obey-the-colour-rules",
        answer: Answer::Stated(None, Rewrite::OutputIntent, OUTPUT_INTENT_REINTERPRETS),
    },
    Remedy {
        requirement: "graphics/separation-alternate-spaces-obey-the-colour-rules-of-part-four",
        answer: Answer::Stated(None, Rewrite::OutputIntent, OUTPUT_INTENT_REINTERPRETS),
    },
    // ISO 19005-2 section 6.6.2.1 and ISO 19005-4 section 6.7.2.1: the catalog states a metadata
    // stream. Answered by the same rewrite as the schema itself, because a document with no packet
    // gets one that states the schema and a document with one keeps every other property in it.
    Remedy {
        requirement: "metadata/catalog-metadata-stream",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    // ISO 19005-2 section 6.6.4 and ISO 19005-4 section 6.7.3: the identification schema, which
    // is the file's own claim to be PDF/A. `doc/pdf-a-conversion-limits.md` section 4.2 calls it
    // **Mechanical**, "and the one place the converter states a claim about its own output" — the
    // claim being safe because `doc/adr/0947`'s third stage holds the output to the target again
    // before the file is written.
    Remedy {
        requirement: "metadata/identification-schema-prefix",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-part-number",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-conformance-level",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-declares-level-a",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-part-number-four",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-revision-year",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-states-no-flavour",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-declares-flavour-e",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-declares-flavour-f",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
];

/// The requirements the *writer* satisfies, because every output of this verb is a new file.
///
/// `doc/pdf-a-conversion-limits.md` section 4.7's note for each of them is "the serializer emits
/// this shape anyway", and that is exactly what makes them a separate list from [`REMEDIES`]:
/// nothing decides anything about the document, and there is no rewrite to report per place. A
/// whole-file rewrite is the remedy, and the report says so once.
///
/// Each is still *reported*, because a requirement the input failed is a fact about the input
/// whether the fix cost anything or not.
const WRITER_EMITS: &[&str] = &[
    // ISO 19005-2 section 6.1.6, ISO 19005-4 section 6.1.5: `pdf_syntax::write` writes every
    // string in §7.3.4.3's hexadecimal form, two digits a byte.
    "file-structure/hexadecimal-string-digits",
    "file-structure/hexadecimal-string-holds-only-digits",
    // ISO 19005-2 section 6.1.7.1 and section 6.1.9, ISO 19005-4 section 6.1.6.1 and section
    // 6.1.8: the keyword line endings and the object syntax the serializer emits.
    "file-structure/stream-keyword-line-endings",
    "file-structure/indirect-object-syntax",
    "file-structure/cross-reference-keyword-line-endings",
    // Both parts: `/Length` is re-derived as a direct integer from the bytes actually written.
    "file-structure/stream-length-matches-the-data",
    // ISO 19005-2 section 6.1.3, ISO 19005-4 section 6.1.3: nothing follows the last `%%EOF`
    // this writer emits.
    "file-structure/nothing-after-the-last-end-of-file-marker",
];

/// Every requirement identifier this converter answers, from both tables.
///
/// Public for one test — `tests/archive.rs` compares it with the identifiers `pdf_archive`
/// actually states — because a typo in a table key does not fail to compile. It quietly becomes
/// a requirement nobody answers, which this verb then refuses for a reason no reader could see.
#[must_use]
pub fn answered() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = REMEDIES.iter().map(|remedy| remedy.requirement).collect();
    out.extend_from_slice(WRITER_EMITS);
    out
}

/// The sentence a requirement absent from both tables is refused with.
///
/// Deliberately not per-requirement prose. The requirement's own `asks` and its clause are
/// already in the report beside this, and a second sentence restating them would be this
/// converter's paraphrase of a paraphrase. What this adds is the only thing the reader does not
/// already have: that the gap is *this program's*, not the document's.
const NOT_BUILT_YET: &str = "this converter does not yet meet this requirement; \
     doc/pdf-a-conversion-limits.md names the resource or the decision each remaining one waits \
     on, and no file is written rather than one wearing a claim it has not earned";

/// A syntax rule the whole-file rewrite cannot reach, because the syntax is on a page.
const SYNTAX_INSIDE_CONTENT: &str = "this failure is inside a content stream, which this verb \
     carries byte for byte; correcting \
     it would edit the producer's page, and ADR 0816's fence is where that stops — \
     doc/pdf-a-conversion-limits.md section 5.2";

/// Whether any place the validator kept names a page rather than an object.
///
/// `pdf_archive` records an object's own syntax at `Where::object` and a content stream's at
/// `Where::page`, so this is the question "was it on a page" asked of the report rather than of
/// the document. The list is capped, so a document failing in both kinds past the cap could be
/// read the other way; stage 3's own verdict is what catches that, and it refuses to write a
/// file that does not conform.
fn in_a_content_stream(judgement: &Judgement) -> bool {
    let Outcome::Failed { places, .. } = &judgement.outcome else {
        return false;
    };
    places
        .iter()
        .any(|finding| finding.place.page.is_some() && finding.place.object.is_none())
}

/// The document does not conform and cannot be made to without editing what it says.
const UTF8_NAMES: &str = "ISO 19005 binds these names to valid UTF-8, and renaming one would \
     break every reference that uses it — doc/pdf-a-conversion-limits.md section 4.7 records this \
     row \
     as a refusal rather than a rewrite";

/// The two spellings the two parts print for the identification schema's namespace.
///
/// ISO 19005-2 section 6.6.4 gives it with an `http` scheme and ISO 19005-4 section 6.7.3 with an
/// `https` one. A conversion writes the one its target's part prints and **removes both**, because
/// a property left behind under the other spelling would be a second claim standing beside the one
/// just written.
const IDENTIFICATION_URIS: [&str; 2] = [
    "http://www.aiim.org/pdfa/ns/id/",
    "https://www.aiim.org/pdfa/ns/id/",
];

/// The prefix both parts make required for every property of the identification schema.
const IDENTIFICATION_PREFIX: &str = "pdfaid";

/// The publication year ISO 19005-4's own revision property names.
const REVISION_YEAR: &str = "2020";

/// Why no output intent could be prepared for this document.
const NO_USABLE_PROFILE: &str = "an output intent needs a destination profile that is an ICC \
     profile of an output or monitor class over grey, RGB or CMYK, and this conversion has none: \
     either the profile supplied with --output-intent-profile is not one, or the file already \
     holds a destination profile that is not — and ISO 19005 requires every entry of an \
     OutputIntents array to name the same profile object, so a second one cannot be added beside \
     it";

/// Why a producer's XMP packet could not be given the identification schema.
const PACKET_NOT_EDITABLE: &str = "this document's XMP packet cannot be edited in place, and \
     replacing it would throw away metadata its producer wrote — which is a loss nobody has been \
     asked to authorise. doc/pdf-a-conversion-limits.md section 4.2 is where that question \
     belongs, and this converter does not put it yet";

/// The placeholder for a construction no failed requirement asked for.
///
/// Never reported: `decide` reads a preparation's reason only for a requirement the table answers
/// with the rewrite that preparation builds, and such a requirement is exactly what makes the
/// preparation happen. It says so rather than borrowing another reason's sentence.
const NOT_ASKED_FOR: &str = "no requirement this document failed asked for this construction, so \
     none was prepared";

/// Why neither construction could be prepared for a document with no readable catalog.
///
/// Unreachable through [`run`], which refuses such a document before stage 1; it exists so that
/// the reason a requirement is refused with is never a reason that is not the actual one.
const NO_CATALOG: &str = "this document has no readable catalog, so there is nowhere to state an \
     output intent or a metadata stream";

/// Why an object could not be added to the output.
const NO_SPARE_OBJECT: &str = "this document uses every object number a conversion could give to \
     the stream it has to add";

/// The properties the identification schema states for one target.
///
/// ISO 19005-2 section 6.6.4 asks for a part number of 2 and a conformance level of A, B or U;
/// ISO 19005-4 section 6.7.3 asks for a part number of 4 and a revision year, and reserves a
/// conformance property for the two annexes — a file that is neither PDF/A-4e nor PDF/A-4f states
/// none at all. **Part 4's own table spells that property with a `pdfa` prefix** in a schema whose
/// required prefix the same table gives as `pdfaid`; `pdf_archive`'s metadata tranche records why
/// it reads the property in the identification namespace whatever prefix spells it, and this
/// writes `pdfaid` because that is the prefix the subclause makes required and the one
/// `metadata/identification-schema-prefix` holds a file to.
fn identification_properties(target: Target) -> Vec<(&'static str, String)> {
    match target {
        Target::Two(level) => vec![
            ("part", "2".to_owned()),
            (
                "conformance",
                match level {
                    Level::A => "A",
                    Level::B => "B",
                    Level::U => "U",
                }
                .to_owned(),
            ),
        ],
        Target::Four(flavour) => {
            let mut out = vec![("part", "4".to_owned()), ("rev", REVISION_YEAR.to_owned())];
            match flavour {
                Flavour::Plain => {}
                Flavour::E => out.push(("conformance", "E".to_owned())),
                Flavour::F => out.push(("conformance", "F".to_owned())),
            }
            out
        }
    }
}

/// The namespace URI the target's own part prints for the identification schema.
const fn identification_uri(target: Target) -> &'static str {
    match target.part() {
        pdf_archive::Part::Two => IDENTIFICATION_URIS[0],
        pdf_archive::Part::Four => IDENTIFICATION_URIS[1],
    }
}

/// Where an output intent's destination profile came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSource {
    /// The sRGB profile this program ships — `doc/questions/A18`'s default.
    Shipped,
    /// One the caller supplied.
    Supplied,
    /// One the document already held.
    ///
    /// Not a choice this conversion made: ISO 19005 requires every entry of an `OutputIntents`
    /// array that states a destination profile to state the *same* object, so a file that already
    /// holds one decides what a new entry names.
    AlreadyInTheFile,
}

impl ProfileSource {
    /// A stable word for the report's machine-readable form.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Shipped => "shipped",
            Self::Supplied => "supplied",
            Self::AlreadyInTheFile => "already-in-the-file",
        }
    }

    /// Where the profile came from, in one clause for a person.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Shipped => "the sRGB profile this program ships",
            Self::Supplied => "the profile you supplied",
            Self::AlreadyInTheFile => "the destination profile this file already held",
        }
    }
}

/// The destination profile a conversion's output intent names, as the report states it.
///
/// **The `cprt` tag is the load-bearing field.** `doc/pdf-a-conversion-limits.md` section 10.1
/// records the ICC's own guidance that a profile's copyright owner and terms of use live in its
/// header's creator field and its `cprt` tag, and turns that into a rule: a user who embeds
/// somebody else's press profile is told whose it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestinationProfile {
    /// Where it came from.
    pub source: ProfileSource,
    /// The profile's `desc` tag: what it calls itself.
    pub describes: Option<String>,
    /// Its `cprt` tag: whose profile it is, and on what terms.
    pub copyright: Option<String>,
    /// The colour space its own header states.
    pub space: String,
}

impl DestinationProfile {
    /// The profile as JSON.
    fn to_json(&self) -> Value {
        Value::Object(vec![
            ("source".to_owned(), Value::text(self.source.word())),
            (
                "describes".to_owned(),
                self.describes
                    .as_ref()
                    .map_or(Value::Null, |text| Value::text(text.clone())),
            ),
            (
                "copyright".to_owned(),
                self.copyright
                    .as_ref()
                    .map_or(Value::Null, |text| Value::text(text.clone())),
            ),
            ("space".to_owned(), Value::text(self.space.clone())),
        ])
    }
}

/// The output intent a conversion is in a position to add.
struct Intent {
    /// The object the destination profile is, in the *source's* numbering.
    destination: ObjectId,
    /// The profile stream this conversion adds, where it adds one.
    written: Option<Object>,
    /// The colour family the profile's own header states.
    family: DeviceFamily,
    /// What the report says about the profile.
    reported: DestinationProfile,
}

/// The metadata stream a conversion is in a position to write.
struct Metadata {
    /// The object the packet goes in, in the *source's* numbering.
    at: ObjectId,
    /// The stream this conversion adds, where the document had no packet to edit.
    written: Option<Object>,
    /// The packet, for the stream that carries it.
    packet: Vec<u8>,
}

/// What this slice's two constructions need, worked out before any decision is taken.
///
/// Both depend on the *document* rather than on the requirement alone — whether a profile can be
/// shared with one the file already holds, whether the packet a producer wrote can be edited in
/// place — and **a decision that cannot be carried out is a refusal rather than a plan**, which
/// is the rule [`version_for`] already answers to. So both are settled here, in stage 2, and the
/// reason each failed is the reason the requirements it would have answered are refused with.
///
/// Nothing is prepared that no failed requirement asked for: a document that already conforms
/// does not have its packet read, which is what keeps `doc/adr/0947`'s first rule true of this
/// slice as well.
struct Prepared {
    /// The output intent to add, or why one cannot be.
    intent: Result<Intent, Because>,
    /// The metadata stream to write, or why one cannot be.
    metadata: Result<Metadata, Because>,
}

impl Prepared {
    /// Works out what can be built for this document, and only what a failed requirement asks for.
    fn of(plan: &ArchivePlan, document: &Document, failed: &BTreeSet<&'static str>) -> Self {
        let wanted = |rewrite: Rewrite| {
            REMEDIES.iter().any(|remedy| {
                failed.contains(remedy.requirement) && remedy.answer.rewrite() == rewrite
            })
        };
        let mut spare = Spare::of(document);
        let catalog = document.catalog().ok();
        Self {
            intent: match (wanted(Rewrite::OutputIntent), catalog.as_ref()) {
                (true, Some(catalog)) => prepare_intent(plan, document, catalog, &mut spare),
                (true, None) => Err(Because::NotBuiltYet(NO_CATALOG)),
                // Nothing failed that an output intent answers, so nothing is prepared and the
                // reason is never read: `decide` consults this only for a requirement `wanted`
                // has already found in the table.
                (false, _) => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
            },
            metadata: match (wanted(Rewrite::IdentificationSchema), catalog.as_ref()) {
                (true, Some(catalog)) => {
                    prepare_metadata(plan.target, document, catalog, &mut spare)
                }
                (true, None) => Err(Because::NotBuiltYet(NO_CATALOG)),
                (false, _) => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
            },
        }
    }

    /// The objects the conversion adds, in the source's numbering, for [`enter`] to find.
    fn added(&self) -> BTreeMap<ObjectId, Object> {
        let mut out = BTreeMap::new();
        if let Ok(intent) = &self.intent
            && let Some(object) = &intent.written
        {
            out.insert(intent.destination, object.clone());
        }
        if let Ok(metadata) = &self.metadata
            && let Some(object) = &metadata.written
        {
            out.insert(metadata.at, object.clone());
        }
        out
    }
}

/// Object numbers nothing in the source resolves to, for the objects a conversion adds.
///
/// Past the highest number any cross-reference section names, and then checked one at a time:
/// **the objects a conversion adds are built in the source's numbering** like every other object
/// this verb rewrites, so a number that collided with a source object would silently replace it.
struct Spare {
    /// The next number to try.
    next: u32,
}

impl Spare {
    /// Begins past the highest object number the document's cross-reference sections name.
    fn of(document: &Document) -> Self {
        let highest = document.xref().object_numbers().max().unwrap_or(0);
        Self {
            next: highest.saturating_add(1),
        }
    }

    /// The next number the document resolves to nothing.
    fn take(&mut self, document: &Document) -> Option<ObjectId> {
        for _ in 0..MAX_SPARE_NUMBERS {
            let id = ObjectId::new(self.next, 0);
            self.next = self.next.checked_add(1)?;
            if document.get(id) == Object::Null {
                return Some(id);
            }
        }
        None
    }
}

/// How many object numbers [`Spare`] tries before giving up.
const MAX_SPARE_NUMBERS: usize = 64;

/// The `/OutputIntents` array the catalog states, resolved to its entries.
fn output_intent_entries(document: &Document, catalog: &Dictionary) -> Vec<Object> {
    document
        .get_key(catalog, "OutputIntents")
        .as_array()
        .map(<[Object]>::to_vec)
        .unwrap_or_default()
}

/// The colour family a profile's header names, where it is one of ISO 19005's three.
fn family_of(stated: &Identification) -> Option<DeviceFamily> {
    // ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3 admit an output or a monitor
    // profile and no other class, and grey, RGB or CMYK and no other space. `pdf_archive` judges
    // both of a file's own profile; this asks the same question of one about to be written, so
    // that a conversion cannot add a profile the validator would then reject.
    if stated.class != *b"prtr" && stated.class != *b"mntr" {
        return None;
    }
    match &stated.space {
        b"GRAY" => Some(DeviceFamily::Gray),
        b"RGB " => Some(DeviceFamily::Rgb),
        b"CMYK" => Some(DeviceFamily::Cmyk),
        _ => None,
    }
}

/// Prepares the output intent: the profile it names, and where that profile comes from.
fn prepare_intent(
    plan: &ArchivePlan,
    document: &Document,
    catalog: &Dictionary,
    spare: &mut Spare,
) -> Result<Intent, Because> {
    let wrong = Because::NotBuiltYet(NO_USABLE_PROFILE);
    // ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3: where an OutputIntents array holds
    // more than one entry, every entry stating a destination profile states the *same* object. So
    // a file that already holds one decides what a new entry may name, and adding a second
    // profile beside it is not open to this conversion at all.
    if let Some(destination) = held_destination_profile(document, catalog) {
        let Object::Stream(stream) = document.get(destination) else {
            return Err(wrong);
        };
        let data = document.decoded_stream_data(&stream).ok_or(wrong)?;
        let stated = Identification::read(&data).ok_or(wrong)?;
        let family = family_of(&stated).ok_or(wrong)?;
        return Ok(Intent {
            destination,
            written: None,
            family,
            reported: DestinationProfile {
                source: ProfileSource::AlreadyInTheFile,
                space: stated.space_name(),
                describes: stated.description,
                copyright: stated.copyright,
            },
        });
    }

    let (source, bytes) = match &plan.profile {
        Some(supplied) => (ProfileSource::Supplied, supplied.to_vec()),
        None => (ProfileSource::Shipped, SRGB.to_vec()),
    };
    let stated = Identification::read(&bytes).ok_or(wrong)?;
    let family = family_of(&stated).ok_or(wrong)?;
    let destination = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    Ok(Intent {
        destination,
        written: Some(profile_stream(&bytes, family)),
        family,
        reported: DestinationProfile {
            source,
            space: stated.space_name(),
            describes: stated.description,
            copyright: stated.copyright,
        },
    })
}

/// The object an entry of the catalog's `OutputIntents` array already names as its destination
/// profile, where one does.
fn held_destination_profile(document: &Document, catalog: &Dictionary) -> Option<ObjectId> {
    output_intent_entries(document, catalog)
        .iter()
        .filter_map(|entry| document.resolve(entry).as_dict().cloned())
        .find_map(|entry| {
            entry
                .get("DestOutputProfile")
                .and_then(Object::as_reference)
        })
}

/// The ICC profile stream an output intent names.
///
/// §14.11.5's Table 401 makes `DestOutputProfile` "[a]n ICC profile stream defining the
/// transformation from the PDF document's source colours to output device colourants" and says
/// that "[t]he format of the profile stream is the same as that used in specifying an `ICCBased`
/// colour space", which is §8.6.5.5's — hence the `/N`.
fn profile_stream(bytes: &[u8], family: DeviceFamily) -> Object {
    let components = match family {
        DeviceFamily::Gray => 1,
        DeviceFamily::Rgb => 3,
        DeviceFamily::Cmyk => 4,
    };
    let mut dict = Dictionary::new();
    dict.insert(Name::new(&b"N"[..]), Object::Integer(components));
    let data = match flate_encode(bytes, COMPRESSION_LEVEL) {
        Some(encoded) => {
            dict.insert(
                Name::new(&b"Filter"[..]),
                Object::Name(Name::new(&b"FlateDecode"[..])),
            );
            encoded
        }
        None => bytes.to_vec(),
    };
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).unwrap_or(i64::MAX)),
    );
    Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    }))
}

/// The output intent dictionary a conversion adds.
///
/// §14.11.5's Table 400 and Table 401 decide every entry, and one of them is a choice this file
/// records rather than derives. `OutputConditionIdentifier` is required and the table says that
/// "[i]f the intended production condition is not a recognised standard, the value of this entry
/// may be `Custom` or an application-specific, machine-readable name" — so `Custom` is the
/// standard's own word for exactly this case, and no convention is being copied from anywhere.
/// The same sentence then says that "[t]he `DestOutputProfile` entry defines the ICC profile, and
/// the `Info` entry shall be used for further human-readable identification", which is why `Info`
/// carries the profile's own `desc` tag: it is read out of the profile rather than written about
/// it. `S` is `GTS_PDFA1`, which both parts' section 6.2.3 requires of a PDF/A output intent.
fn intent_dictionary(intent: &Intent) -> Dictionary {
    let mut out = Dictionary::new();
    out.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"OutputIntent"[..])),
    );
    out.insert(
        Name::new(&b"S"[..]),
        Object::Name(Name::new(&b"GTS_PDFA1"[..])),
    );
    out.insert(
        Name::new(&b"OutputConditionIdentifier"[..]),
        Object::String(b"Custom".to_vec().into()),
    );
    let info = intent
        .reported
        .describes
        .clone()
        .unwrap_or_else(|| "the ICC profile embedded beside this entry".to_owned());
    out.insert(
        Name::new(&b"Info"[..]),
        Object::String(info.into_bytes().into()),
    );
    out.insert(
        Name::new(&b"DestOutputProfile"[..]),
        Object::Reference(intent.destination),
    );
    out
}

/// Prepares the metadata stream: the packet, and the object it goes in.
///
/// Two cases, and the difference between them is the whole of what makes this safe. A document
/// with a packet has it **edited in place** — `pdf_model::xmp::restate` cuts the identification
/// schema's properties out of the producer's own bytes and puts this target's in, leaving every
/// other byte alone. A document with none gets a fresh packet stating the schema and nothing
/// else. What is never done is reading a producer's packet to a value and printing it again: this
/// tree's reader keeps neither an `rdf:about` subject nor a qualifier other than `xml:lang`, so a
/// packet round-tripped through it would come back quietly poorer.
fn prepare_metadata(
    target: Target,
    document: &Document,
    catalog: &Dictionary,
    spare: &mut Spare,
) -> Result<Metadata, Because> {
    let properties = identification_properties(target);
    let schema = Schema {
        namespace: identification_uri(target),
        prefix: IDENTIFICATION_PREFIX,
        properties: &properties,
    };
    if let Some(at) = catalog.get("Metadata").and_then(Object::as_reference)
        && let Object::Stream(stream) = document.get(at)
        && let Some(bytes) = document.decoded_stream_data(&stream)
    {
        let packet = xmp::restate(&bytes, &IDENTIFICATION_URIS, &schema)
            .map_err(|_| Because::NotBuiltYet(PACKET_NOT_EDITABLE))?;
        return Ok(Metadata {
            at,
            written: None,
            packet,
        });
    }
    let at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    let packet = xmp::packet(&schema);
    Ok(Metadata {
        at,
        written: Some(metadata_stream(&Dictionary::new(), &packet)),
        packet,
    })
}

/// One metadata stream, with the two entries §14.3.2's Table 347 requires of it.
///
/// `/Type`:
///
/// > ( Required ) The type of PDF object that this dictionary describes; shall be Metadata for a
/// > metadata stream.
///
/// `/Subtype`:
///
/// > ( Required ) The type of metadata stream that this dictionary describes; shall be XML .
///
/// The packet is written uncompressed, which is what §14.3.2's own EXAMPLE shows and what leaves
/// a document's metadata legible to a reader that is not a PDF parser. Any filter the source's
/// stream stated goes with the bytes it decoded.
fn metadata_stream(from: &Dictionary, packet: &[u8]) -> Object {
    let mut dict = from.clone();
    dict.remove("Filter");
    dict.remove("DecodeParms");
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Metadata"[..])),
    );
    dict.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(&b"XML"[..])),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(packet.len()).unwrap_or(i64::MAX)),
    );
    Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: packet.to_vec().into(),
        decryption_failed: false,
    }))
}

/// Decides what to do about every requirement the document failed.
///
/// The middle stage, as a pure function of the validator's report and the caller's
/// authorisations. **A requirement absent from [`REMEDIES`] and [`WRITER_EMITS`] is refused by
/// name** — never passed over, and never answered by a rewrite invented here.
fn decide(judgement: &Judgement, authorised: Authorisations, prepared: &Prepared) -> Decision {
    if judgement.id == "file-structure/bound-names-are-valid-utf8" {
        return Decision::Refused(Because::TheFence(UTF8_NAMES));
    }
    if WRITER_EMITS.contains(&judgement.id) {
        // **Not unconditionally.** Some of these rules reach inside a content stream — ISO
        // 19005's hexadecimal string rule is stated of the file's syntax and a content stream
        // is syntax — and a content stream crosses this verb byte for byte. So a failure a
        // page names is one the whole-file rewrite does not reach, and saying otherwise would
        // be promising a fix that never arrives.
        if in_a_content_stream(judgement) {
            return Decision::Refused(Because::TheFence(SYNTAX_INSIDE_CONTENT));
        }
        return Decision::Mechanical(Rewrite::WholeFileRewritten);
    }
    let Some(remedy) = REMEDIES
        .iter()
        .find(|remedy| remedy.requirement == judgement.id)
    else {
        return Decision::Refused(Because::NotBuiltYet(NOT_BUILT_YET));
    };
    match remedy.answer {
        // The two rewrites this slice added are answers only where the construction they need
        // could be prepared, and the reason it could not is the reason the requirement is
        // refused with — never this converter's own paraphrase of it.
        Answer::Mechanical(Rewrite::IdentificationSchema) => match &prepared.metadata {
            Ok(_) => Decision::Mechanical(Rewrite::IdentificationSchema),
            Err(because) => Decision::Refused(*because),
        },
        Answer::Mechanical(rewrite) => Decision::Mechanical(rewrite),
        Answer::Stated(family, Rewrite::OutputIntent, reinterprets) => match &prepared.intent {
            Err(because) => Decision::Refused(*because),
            // ISO 19005 section 6.2.4.3 licenses a device colour space through a destination
            // profile **of its own family**, so a row naming one is answered by this intent only
            // where the profile in hand is that family's. A row naming none is answered by any
            // PDF/A output intent, and a row whose family is the *document's* rather than the
            // requirement's names none — the output's own verdict is what settles those.
            Ok(intent) if family.is_none_or(|wanted| wanted == intent.family) => Decision::Stated {
                rewrite: Rewrite::OutputIntent,
                reinterprets,
            },
            Ok(_) => Decision::Refused(Because::NotBuiltYet(WRONG_FAMILY)),
        },
        Answer::Stated(_, rewrite, reinterprets) => Decision::Stated {
            rewrite,
            reinterprets,
        },
        Answer::Loses(loss, rewrite) if authorised.grants(loss) => {
            Decision::Authorised { loss, rewrite }
        }
        Answer::Loses(loss, rewrite) => Decision::Unauthorised { loss, rewrite },
    }
}

/// One requirement the input failed, with what was decided and what was done about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decided {
    /// The `pdf_archive` requirement identifier.
    pub requirement: &'static str,
    /// Its clause, as cited for the target that was asked for.
    pub citation: String,
    /// What it asks, in `pdf_archive`'s own words.
    pub asks: &'static str,
    /// How many places the input failed it, including any past the validator's own bound.
    pub places: usize,
    /// What the converter decided.
    pub decision: Decision,
    /// How many places the rewrite actually touched.
    ///
    /// Zero for a decision that performed none, and for a requirement the writer met by
    /// construction — there is no place to count when the whole file is the rewrite.
    pub changed: usize,
}

/// One requirement the validator did not check, carried into the conversion's report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotChecked {
    /// The requirement identifier.
    pub requirement: &'static str,
    /// Its clause.
    pub citation: String,
    /// Why it was not checked, in the validator's own words.
    ///
    /// **Never shortened and never softened.** `doc/questions/A20`: the reason *is* the report,
    /// and a conversion that trimmed it would be making its own verdict look better than the
    /// evidence for it.
    pub because: &'static str,
}

/// What holding the *output* to the target found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Achieved {
    /// Whether every requirement the validator checks was met by the output.
    pub conforms: bool,
    /// Every requirement the output still fails, by identifier.
    pub still_failing: Vec<&'static str>,
    /// Every requirement the *input* met and the output does not.
    ///
    /// A regression is what stops the file being written at all: a conversion that broke a
    /// requirement its source satisfied has not converted the document, it has damaged it.
    pub regressions: Vec<&'static str>,
    /// How many requirements the validator checked, of those that bind the target and are
    /// about a file at all.
    pub checked: usize,
}

/// What one conversion did, per document.
///
/// The report `doc/adr/0927` makes the condition of the owner's four permissions, and section 7 of
/// `doc/pdf-a-conversion-limits.md` the promise it keeps: what conformed already, what was
/// changed and under which clause, what was refused and why, what was lost with authorisation
/// — and the list of requirements the verdict is a verdict *over*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversion {
    /// Which source.
    pub source: usize,
    /// The target asked for.
    pub target: Target,
    /// Every requirement the input already met, by identifier.
    pub conformed: Vec<&'static str>,
    /// Every requirement the input failed, with the decision taken about it.
    pub decided: Vec<Decided>,
    /// Every requirement the validator did not check.
    pub not_checked: Vec<NotChecked>,
    /// What the output was found to be, where one was written.
    ///
    /// `None` means no file was written: some decision refused, and [`Conversion::decided`]
    /// says which.
    pub achieved: Option<Achieved>,
    /// The destination profile the output intent this conversion added names.
    ///
    /// `None` where no output intent was added. `doc/questions/A18` makes this half of the
    /// report the condition on the permission: a conversion that adds an output intent has
    /// changed what every device colour in the file means to a conforming reader, and a user is
    /// entitled to be told which profile decided that and whose profile it is.
    pub profile: Option<DestinationProfile>,
}

impl Conversion {
    /// Whether every decision lets the conversion proceed.
    #[must_use]
    pub fn proceeds(&self) -> bool {
        self.decided
            .iter()
            .all(|decided| decided.decision.proceeds())
    }

    /// The report as RFC 0002 section 4.5's JSON.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Value::Object(vec![
            ("source".to_owned(), Value::count(self.source)),
            ("target".to_owned(), Value::text(self.target.to_string())),
            (
                "conformed".to_owned(),
                Value::Array(self.conformed.iter().map(|id| Value::text(*id)).collect()),
            ),
            (
                "decided".to_owned(),
                Value::Array(self.decided.iter().map(Decided::to_json).collect()),
            ),
            (
                "not_checked".to_owned(),
                Value::Array(self.not_checked.iter().map(NotChecked::to_json).collect()),
            ),
            (
                "achieved".to_owned(),
                self.achieved
                    .as_ref()
                    .map_or(Value::Null, Achieved::to_json),
            ),
            (
                "output_intent_profile".to_owned(),
                self.profile
                    .as_ref()
                    .map_or(Value::Null, DestinationProfile::to_json),
            ),
        ])
    }

    /// The report as text, for a person.
    ///
    /// The shape is the one section 7 of that document asks for: what the target was, what
    /// conformed, what was done and under which clause, what was refused and why — and the
    /// not-checked list last, never
    /// omitted, so that a reader who has learned to look for it can see it is empty rather than
    /// absent.
    #[must_use]
    pub fn render(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{}: {} of the requirements this file was held to were already met",
            self.target,
            self.conformed.len()
        );
        // One rewrite can answer a dozen requirements — an output intent answers most of the
        // colour subclause at once — and its sentence about what the file now asserts is the same
        // sentence every time. Said once, so that a reader meets it rather than skims past it;
        // the JSON keeps it against each decision, where a machine wants it.
        let mut said: BTreeSet<&'static str> = BTreeSet::new();
        for decided in &self.decided {
            let repeated = match decided.decision {
                Decision::Stated { reinterprets, .. } => !said.insert(reinterprets),
                _ => false,
            };
            let _ = writeln!(
                out,
                "  {} ({}) — {} place(s)\n      {}",
                decided.requirement,
                decided.citation,
                decided.places,
                describe_decision(decided, repeated)
            );
        }
        if let Some(profile) = &self.profile {
            let _ = writeln!(
                out,
                "  the output intent's destination profile is {}{}, over {}",
                profile.source.describe(),
                profile
                    .describes
                    .as_ref()
                    .map_or_else(String::new, |name| format!(" ({name})")),
                profile.space
            );
            if let Some(copyright) = &profile.copyright {
                let _ = writeln!(out, "      its copyright tag says: {copyright}");
            }
        }
        if let Some(achieved) = &self.achieved {
            let verdict = if achieved.conforms {
                "conforms"
            } else {
                "does not conform"
            };
            let _ = writeln!(
                out,
                "  the output was held to {} again and {verdict}, over {} requirements",
                self.target, achieved.checked
            );
            for id in &achieved.still_failing {
                let sharper = if achieved.regressions.contains(id) {
                    " — which this document met, so the conversion broke it"
                } else {
                    ""
                };
                let _ = writeln!(out, "      still failed: {id}{sharper}");
            }
            if !achieved.conforms {
                let _ = writeln!(
                    out,
                    "  no file was written, because a conversion whose result is not {} has not \
                     converted the document",
                    self.target
                );
            }
        } else {
            let _ = writeln!(out, "  no file was written");
        }
        let _ = writeln!(out, "  not checked ({}):", self.not_checked.len());
        for row in &self.not_checked {
            let _ = writeln!(out, "      {} ({})", row.requirement, row.citation);
        }
        out
    }
}

/// One decision, worded for a person.
fn describe_decision(decided: &Decided, repeated: bool) -> String {
    match decided.decision {
        Decision::Mechanical(rewrite) => {
            format!(
                "changed, losing nothing: {} ({} done)",
                rewrite.describe(),
                decided.changed
            )
        }
        Decision::Stated {
            rewrite,
            reinterprets,
        } => {
            let sentence = if repeated {
                "the same interpretation as above"
            } else {
                reinterprets
            };
            format!(
                "changed, stating an interpretation the standard defines: {} ({} done)\n      {}",
                rewrite.describe(),
                decided.changed,
                sentence
            )
        }
        Decision::Authorised { loss, rewrite } => format!(
            "changed with your authorisation: {} — {} ({} done)",
            rewrite.describe(),
            loss.describe(),
            decided.changed
        ),
        Decision::Unauthorised { loss, rewrite } => format!(
            "not done, because it loses something nobody authorised: {} — {}; \
             --authorise {} allows it",
            rewrite.describe(),
            loss.describe(),
            loss.word()
        ),
        Decision::Refused(because) => format!("refused: {}", because.sentence()),
    }
}

impl Decided {
    /// One decision as JSON.
    fn to_json(&self) -> Value {
        let mut fields = vec![
            ("requirement".to_owned(), Value::text(self.requirement)),
            ("clause".to_owned(), Value::text(self.citation.clone())),
            ("asks".to_owned(), Value::text(self.asks)),
            ("places".to_owned(), Value::count(self.places)),
            ("decision".to_owned(), Value::text(self.decision.word())),
            ("changed".to_owned(), Value::count(self.changed)),
        ];
        match self.decision {
            Decision::Mechanical(rewrite) => {
                fields.push(("rewrite".to_owned(), Value::text(rewrite.word())));
            }
            Decision::Stated {
                rewrite,
                reinterprets,
            } => {
                fields.push(("rewrite".to_owned(), Value::text(rewrite.word())));
                fields.push(("reinterprets".to_owned(), Value::text(reinterprets)));
            }
            Decision::Authorised { loss, rewrite } | Decision::Unauthorised { loss, rewrite } => {
                fields.push(("rewrite".to_owned(), Value::text(rewrite.word())));
                fields.push(("loss".to_owned(), Value::text(loss.word())));
                fields.push(("loses".to_owned(), Value::text(loss.describe())));
            }
            Decision::Refused(because) => {
                fields.push(("because".to_owned(), Value::text(because.word())));
                fields.push(("reason".to_owned(), Value::text(because.sentence())));
            }
        }
        Value::Object(fields)
    }
}

impl NotChecked {
    /// One of the validator's unchecked rows, carried across.
    fn of(judgement: &Judgement) -> Self {
        Self {
            requirement: judgement.id,
            citation: judgement.citation.clone(),
            because: match judgement.outcome {
                Outcome::Unchecked(why) => why,
                // `Report::unchecked` yields only `Outcome::Unchecked`, so this arm is
                // unreachable; it names the bug rather than panicking.
                _ => "the validator reported no reason",
            },
        }
    }

    /// One unchecked requirement as JSON.
    fn to_json(&self) -> Value {
        Value::Object(vec![
            ("requirement".to_owned(), Value::text(self.requirement)),
            ("clause".to_owned(), Value::text(self.citation.clone())),
            ("not_checked_because".to_owned(), Value::text(self.because)),
        ])
    }
}

impl Achieved {
    /// The output's verdict as JSON.
    fn to_json(&self) -> Value {
        Value::Object(vec![
            ("conforms".to_owned(), Value::Bool(self.conforms)),
            ("checked".to_owned(), Value::count(self.checked)),
            (
                "still_failing".to_owned(),
                Value::Array(
                    self.still_failing
                        .iter()
                        .map(|id| Value::text(*id))
                        .collect(),
                ),
            ),
            (
                "regressions".to_owned(),
                Value::Array(self.regressions.iter().map(|id| Value::text(*id)).collect()),
            ),
        ])
    }
}

/// Converts one document to the target and writes it.
///
/// `at` is the document's position among the opened ones — one, for this verb.
///
/// # Errors
///
/// [`Refusal::NoSuchSource`], [`Refusal::Reconstructed`] where the source states its structure
/// only through §C.4's recovery, [`Refusal::Assembly`] where the document cannot be rewritten at
/// all, and [`Refusal::Sink`] where the output cannot be written. A document that cannot be
/// *converted* is not an error: it is [`Report::refused`] beside the conversion's own report,
/// which is where a caller reads which requirement stopped it.
pub(crate) fn run(
    plan: &ArchivePlan,
    at: usize,
    documents: &[Document],
    sinks: &dyn Sinks,
    report: &mut Report,
) -> Result<(), Refusal> {
    let document = documents.get(at).ok_or(Refusal::NoSuchSource {
        at: plan.source,
        count: documents.len(),
    })?;
    let root = crate::optimize::catalog_of(document)?;
    crate::optimize::refuse_a_document_only_recovery_reads(document, root)?;

    // Stage 1: the validator is the reading. Nothing below re-reads ISO 19005.
    let input = pdf_archive::check(document, plan.target);
    // Stage 2: one decision per failed requirement.
    let (mut conversion, version, prepared) = decide_every_failure(plan, document, &input);
    if !conversion.proceeds() {
        for decided in &conversion.decided {
            if decided.decision.proceeds() {
                continue;
            }
            report.refused.push(Declined {
                source: plan.source,
                page: None,
                subject: format!("{} ({})", decided.requirement, decided.citation),
                detail: describe_decision(decided, false),
            });
        }
        report.archive = Some(conversion);
        return Ok(());
    }
    // Stage 3: apply, hold the output to the same target, and write it only if it stands.
    let outcome = apply_the_decisions(
        plan,
        document,
        &input,
        &mut conversion,
        version,
        &prepared,
        sinks,
    );
    let written = match outcome {
        Ok(written) => written,
        Err(refusal) => {
            report.archive = Some(conversion);
            return Err(refusal);
        }
    };
    match written {
        Written::Refused(declined) => report.refused.push(declined),
        Written::File(output) => report.outputs.push(output),
    }
    report.archive = Some(conversion);
    Ok(())
}

/// Stage 2: what to do about every requirement the input failed, and the header's version.
///
/// A pure function of the validator's report, the document's own version and the caller's
/// authorisations. The version is answered here rather than in the rewrite because the
/// *feasibility* of the header rewrite depends on the document — [`version_for`] — and a
/// decision that cannot be carried out is a refusal rather than a plan.
fn decide_every_failure(
    plan: &ArchivePlan,
    document: &Document,
    input: &pdf_archive::Report,
) -> (Conversion, Option<Version>, Prepared) {
    let mut conversion = Conversion {
        source: plan.source,
        target: plan.target,
        conformed: input
            .judgements
            .iter()
            .filter(|judgement| judgement.outcome == Outcome::Met)
            .map(|judgement| judgement.id)
            .collect(),
        decided: Vec::new(),
        not_checked: input.unchecked().map(NotChecked::of).collect(),
        achieved: None,
        profile: None,
    };
    let failed: BTreeSet<&'static str> = input.failures().map(|judgement| judgement.id).collect();
    let prepared = Prepared::of(plan, document, &failed);
    let mut version = None;
    for judgement in input.failures() {
        let places = match &judgement.outcome {
            Outcome::Failed { total, .. } => *total,
            // `Report::failures` yields only `Outcome::Failed`, so this arm is unreachable; it
            // reports no places rather than panicking.
            _ => 0,
        };
        let mut decision = decide(judgement, plan.authorised, &prepared);
        if decision.rewrite() == Some(Rewrite::FileHeader) {
            match version_for(document, plan.target) {
                Ok(stated) => version = Some(stated),
                Err(because) => decision = Decision::Refused(because),
            }
        }
        conversion.decided.push(Decided {
            requirement: judgement.id,
            citation: judgement.citation.clone(),
            asks: judgement.asks,
            places,
            decision,
            changed: 0,
        });
    }
    (conversion, version, prepared)
}

/// What stage 3 produced: a file, or a refusal to write one.
enum Written {
    /// The converted document, written to the sink.
    File(Output),
    /// The conversion was carried out and its result would not stand. Nothing was written.
    Refused(Declined),
}

/// Stage 3: the rewrites, the output's own verdict, and the sink.
fn apply_the_decisions(
    plan: &ArchivePlan,
    document: &Document,
    input: &pdf_archive::Report,
    conversion: &mut Conversion,
    version: Option<Version>,
    prepared: &Prepared,
    sinks: &dyn Sinks,
) -> Result<Written, Refusal> {
    // Asked here for a document whose header already conformed, which is every document that
    // reaches this line without a `FileHeader` decision.
    let version = match version {
        Some(version) => version,
        None => version_for(document, plan.target).map_err(|because| {
            Refusal::Assembly(format!(
                "the header's version cannot be stated for {}: {}",
                plan.target,
                because.sentence()
            ))
        })?,
    };
    let wanted: BTreeSet<Rewrite> = conversion
        .decided
        .iter()
        .filter_map(|decided| decided.decision.rewrite())
        .collect();
    if wanted.contains(&Rewrite::OutputIntent)
        && let Ok(intent) = &prepared.intent
    {
        conversion.profile = Some(intent.reported.clone());
    }
    let converted = convert(document, plan.target, &wanted, version, prepared)?;
    for decided in &mut conversion.decided {
        if let Some(rewrite) = decided.decision.rewrite() {
            decided.changed = converted.applied.get(&rewrite).copied().unwrap_or(0);
        }
    }

    let achieved = hold_the_output_to_the_target(&converted.bytes, input, plan)?;
    let stands = achieved.conforms;
    let declined = declined_output(plan, &achieved);
    conversion.achieved = Some(achieved);
    if !stands {
        return Ok(Written::Refused(declined));
    }

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
    let sank = |error| Refusal::Sink {
        name: expanded.name.clone(),
        error,
    };
    writer.write_all(&converted.bytes).map_err(sank)?;
    writer.flush().map_err(sank)?;
    drop(writer);

    Ok(Written::File(Output {
        name: expanded.name,
        bytes: u64::try_from(converted.bytes.len()).unwrap_or(u64::MAX),
        sanitised: expanded.sanitised,
        origin: Origin::Archived {
            source: plan.source,
            target: plan.target.to_string(),
            pages: Pages::new(document).len(),
            changed: conversion
                .decided
                .iter()
                .filter(|decided| decided.decision.rewrite().is_some())
                .count(),
        },
    }))
}

/// Why an assembled output was not written.
///
/// **The rule is that a file leaves this verb only if it conforms**, and it is stronger than
/// checking that nothing regressed: a conversion whose output still fails a requirement has not
/// converted the document, and writing it would put a file into an archive under a claim
/// nothing had established. A *regression* — a requirement the source met and the output does
/// not — is named separately inside that, because it is the worse of the two failures and a
/// reader should not have to diff two lists to find it.
fn declined_output(plan: &ArchivePlan, achieved: &Achieved) -> Declined {
    use std::fmt::Write as _;

    let mut detail = format!(
        "the output would still fail {} requirement(s): {}. No file is written, because a \
         conversion whose result is not {} has not converted the document",
        achieved.still_failing.len(),
        achieved.still_failing.join(", "),
        plan.target,
    );
    if !achieved.regressions.is_empty() {
        let _ = write!(
            detail,
            ". {} of them the source met, which is worse: {}",
            achieved.regressions.len(),
            achieved.regressions.join(", ")
        );
    }
    Declined {
        source: plan.source,
        page: None,
        subject: format!("the converted file would not be {}", plan.target),
        detail,
    }
}

/// Holds the bytes just written to the same target, and says how they differ from the input.
fn hold_the_output_to_the_target(
    bytes: &[u8],
    input: &pdf_archive::Report,
    plan: &ArchivePlan,
) -> Result<Achieved, Refusal> {
    let output = Document::open(bytes.to_vec()).map_err(|error| {
        Refusal::Assembly(format!("the converted document does not re-open: {error}"))
    })?;
    let held = pdf_archive::check(&output, plan.target);
    let met: BTreeSet<&'static str> = input
        .judgements
        .iter()
        .filter(|judgement| judgement.outcome == Outcome::Met)
        .map(|judgement| judgement.id)
        .collect();
    let still_failing: Vec<&'static str> = held.failures().map(|judgement| judgement.id).collect();
    let regressions = still_failing
        .iter()
        .filter(|id| met.contains(*id))
        .copied()
        .collect();
    Ok(Achieved {
        conforms: held.verdict() == Verdict::Conforms,
        still_failing,
        regressions,
        checked: held.checked(),
    })
}

/// The version the output's header states, or why it cannot be stated.
///
/// **Raising a version and lowering one are not the same act**, and this is where that is
/// decided rather than in the rewrite:
///
/// - The source's major already matches the part's: the minor is kept where the part admits it,
///   and clamped to the part's own floor where it does not. Nothing is asserted that the source
///   did not.
/// - **Raising** (a PDF 1.x source to a part-4 target): allowed. Every construct ISO 32000-1
///   defines, ISO 32000-2 still defines; what changes is that some are deprecated, and ISO
///   19005-4 section 5.1's prohibition on deprecated features is a requirement the validator
///   reports as not checked either way, so the raise asserts nothing the report conceals.
/// - **Lowering** (a PDF 2.x source to a part-2 target): refused. section 6 of
///   `doc/pdf-a-conversion-limits.md` states the cost — every PDF 2.0-only construct is
///   translated or refused — and this converter translates none of them, so a `%PDF-1.7` header
///   over 2.0 constructs would be a file whose own header disowned its contents.
/// - A source whose header states no version this tree can read: refused, because choosing one
///   would be inventing what the producer wrote.
fn version_for(document: &Document, target: Target) -> Result<Version, Because> {
    // ISO 19005-2 section 6.1.2 admits 1.0 to 1.7; ISO 19005-4 section 6.1.2 admits 2.0 to 2.9.
    let (major, top) = match target.part() {
        pdf_archive::Part::Two => (1, 7),
        pdf_archive::Part::Four => (2, 9),
    };
    let Some(stated) = document.version() else {
        return Err(Because::NotBuiltYet(
            "this document's header states no version this tree reads, and choosing one for it \
             would be inventing what its producer wrote",
        ));
    };
    if stated.major == major {
        return Ok(Version {
            major,
            minor: stated.minor.min(top),
        });
    }
    if stated.major < major {
        return Ok(Version { major, minor: 0 });
    }
    Err(Because::NotThisTarget(
        "this document is PDF 2.0 or later and the target's part requires a %PDF-1.n header, so \
         every construct PDF 2.0 added would have to be translated or refused one at a time — \
         doc/pdf-a-conversion-limits.md section 6 — and this converter translates none of them",
    ))
}

/// The bytes of a converted document, and what the rewrites touched.
struct Converted {
    /// The whole output file.
    bytes: Vec<u8>,
    /// How many places each rewrite touched.
    applied: BTreeMap<Rewrite, usize>,
}

/// Rewrites the document and serializes it.
///
/// The walk is [`crate::optimize`]'s closure walk with one difference, and the difference is
/// this verb: an object the conversion changes is `replace`d rather than `copied`, so that
/// every reference to it lands on the new one without the rest of the closure knowing. The
/// replacement is built in the *source's* numbering and renumbered when it is placed, which is
/// what lets the walk follow the references the **rewritten** object holds rather than the ones
/// the source held — so a key the conversion removed cannot leave an object in the file that
/// nothing refers to.
fn convert(
    document: &Document,
    target: Target,
    wanted: &BTreeSet<Rewrite>,
    version: Version,
    prepared: &Prepared,
) -> Result<Converted, Refusal> {
    let root = crate::optimize::catalog_of(document)?;
    let sites = Sites::of(document, root);
    let rewriter = Rewriter {
        document,
        target,
        wanted,
        sites,
        added: prepared.added(),
        intent: prepared.intent.as_ref().ok(),
        metadata: prepared.metadata.as_ref().ok(),
    };
    let mut applied = BTreeMap::new();

    let mut assembly = Assembly::new(vec![document]);
    let mut replaced: Vec<(ObjectId, Object)> = Vec::new();
    let mapped = walk(&rewriter, &mut assembly, root, &mut replaced, &mut applied)
        .map_err(|error| Refusal::Assembly(error.to_string()))?;
    assembly.set_root(mapped);
    // §14.3.3's document information dictionary is the trailer's second root: nothing in the
    // catalog reaches it, so a walk from `/Root` alone would drop a document's title and author.
    if let Some(info) = document
        .trailer()
        .get("Info")
        .and_then(Object::as_reference)
    {
        let carried = walk(&rewriter, &mut assembly, info, &mut replaced, &mut applied)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
        assembly.set_info(Some(carried));
    }

    for (id, value) in replaced {
        let Some(slot) = assembly.copied(0, id) else {
            continue;
        };
        let renumbered = renumber(&assembly, &value, 0);
        assembly
            .place(slot, renumbered)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
    }

    let options = Options {
        // The source's own cross-reference form, and no object streams: this verb changes what
        // ISO 19005 requires changed and nothing else, so a file whose producer wrote §7.5.4's
        // classic table gets one back.
        form: Form::of(document),
        object_streams: ObjectStreams::Disable,
        streams: Streams::Carry,
    };
    let mut bytes = Vec::new();
    serialize(&assembly, version, options, &mut bytes)
        .map_err(|error| Refusal::Assembly(error.to_string()))?;
    // The rewrites there is no place to count: the file's own shape, and the two constructions
    // that are one per document by definition — a document gains one output intent and states one
    // identification schema.
    for whole in [
        Rewrite::FileHeader,
        Rewrite::WholeFileRewritten,
        Rewrite::OutputIntent,
        Rewrite::IdentificationSchema,
    ] {
        if wanted.contains(&whole) {
            applied.insert(whole, 1);
        }
    }
    Ok(Converted { bytes, applied })
}

/// Copies `start` and everything the *converted* document reaches into the assembly.
fn walk(
    rewriter: &Rewriter<'_>,
    assembly: &mut Assembly<'_>,
    start: ObjectId,
    replaced: &mut Vec<(ObjectId, Object)>,
    applied: &mut BTreeMap<Rewrite, usize>,
) -> Result<ObjectId, pdf_syntax::AssemblyError> {
    let mut queue: VecDeque<ObjectId> = VecDeque::new();
    let first = enter(rewriter, assembly, start, replaced, applied, &mut queue)?
        .ok_or(pdf_syntax::AssemblyError::TooManyObjects)?;
    while let Some(id) = queue.pop_front() {
        let value = replaced
            .iter()
            .find(|(placed, _)| *placed == id)
            .map_or_else(|| rewriter.document.get(id), |(_, value)| value.clone());
        reach(rewriter, assembly, &value, 0, replaced, applied, &mut queue)?;
    }
    Ok(first)
}

/// Puts one object into the assembly, rewritten where the conversion changes it.
///
/// `Ok(None)` for an object the conversion **drops** — a PostScript `XObject` — which is not an
/// error: §7.3.10 makes a reference to an object the file does not hold "a reference to the null
/// object", the serializer writes that null, and §7.3.7 makes a dictionary entry whose value is
/// null the same as an absent one. So the resource entry that named it disappears with it,
/// without anything here having to find the dictionary it sat in.
fn enter(
    rewriter: &Rewriter<'_>,
    assembly: &mut Assembly<'_>,
    id: ObjectId,
    replaced: &mut Vec<(ObjectId, Object)>,
    applied: &mut BTreeMap<Rewrite, usize>,
    queue: &mut VecDeque<ObjectId>,
) -> Result<Option<ObjectId>, pdf_syntax::AssemblyError> {
    if let Some(already) = assembly.copied(0, id) {
        return Ok(Some(already));
    }
    // An object this conversion *adds* is built in the source's numbering like every object it
    // rewrites, so that the walk maps its references exactly as it maps a rewritten object's.
    // [`Spare`] is what keeps its number from colliding with one the source uses.
    if let Some(added) = rewriter.added.get(&id) {
        let placed = assembly.replace(0, id)?;
        replaced.push((id, added.clone()));
        queue.push_back(id);
        return Ok(Some(placed));
    }
    let value = rewriter.document.get(id);
    if value == Object::Null {
        return Ok(None);
    }
    match rewriter.rewrite(id, &value, applied) {
        Rewritten::Dropped => Ok(None),
        Rewritten::Carried => {
            let placed = assembly.copy(0, id)?;
            queue.push_back(id);
            Ok(Some(placed))
        }
        Rewritten::Changed(changed) => {
            let placed = assembly.replace(0, id)?;
            replaced.push((id, changed));
            queue.push_back(id);
            Ok(Some(placed))
        }
    }
}

/// Every reference in one value, entered and queued.
fn reach(
    rewriter: &Rewriter<'_>,
    assembly: &mut Assembly<'_>,
    value: &Object,
    depth: usize,
    replaced: &mut Vec<(ObjectId, Object)>,
    applied: &mut BTreeMap<Rewrite, usize>,
    queue: &mut VecDeque<ObjectId>,
) -> Result<(), pdf_syntax::AssemblyError> {
    if depth >= MAX_WALK_DEPTH {
        return Ok(());
    }
    let deeper = depth.saturating_add(1);
    match value {
        Object::Reference(id) => {
            enter(rewriter, assembly, *id, replaced, applied, queue)?;
        }
        Object::Array(items) => {
            for item in items {
                reach(rewriter, assembly, item, deeper, replaced, applied, queue)?;
            }
        }
        Object::Dictionary(dict) => {
            for (_, item) in dict.iter() {
                reach(rewriter, assembly, item, deeper, replaced, applied, queue)?;
            }
        }
        Object::Stream(stream) => {
            for (key, item) in stream.dict.iter() {
                // §7.3.8.2's `/Length` is re-derived by the writer as a direct integer, so an
                // object the source stated it in is referred to by nothing in the output.
                // [`crate::optimize`] has the whole argument.
                if key.as_bytes() == b"Length" {
                    continue;
                }
                reach(rewriter, assembly, item, deeper, replaced, applied, queue)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// One value with every reference mapped into the output's numbering.
///
/// The serializer does this for a *copied* object and deliberately does not for a synthesised
/// one, whose references are the output's by construction. A replaced object is neither: it is
/// built from the source's value, so its references are the source's and this is where they are
/// mapped. A reference the assembly does not hold becomes §7.3.10's null, and a dictionary
/// entry that became null is dropped for §7.3.7's sentence — the same two rules the serializer
/// applies, applied here for the same reason.
fn renumber(assembly: &Assembly<'_>, value: &Object, depth: usize) -> Object {
    if depth >= MAX_WALK_DEPTH {
        return Object::Null;
    }
    let deeper = depth.saturating_add(1);
    match value {
        Object::Reference(id) => assembly
            .copied(0, *id)
            .map_or(Object::Null, Object::Reference),
        Object::Array(items) => Object::Array(
            items
                .iter()
                .map(|item| renumber(assembly, item, deeper))
                .collect(),
        ),
        Object::Dictionary(dict) => Object::Dictionary(renumber_dictionary(assembly, dict, depth)),
        Object::Stream(stream) => Object::Stream(std::sync::Arc::new(Stream {
            dict: renumber_dictionary(assembly, &stream.dict, depth),
            data: std::sync::Arc::clone(&stream.data),
            decryption_failed: stream.decryption_failed,
        })),
        other => other.clone(),
    }
}

/// [`renumber`] over a dictionary's values, dropping the entries that became null.
fn renumber_dictionary(assembly: &Assembly<'_>, dict: &Dictionary, depth: usize) -> Dictionary {
    let mut out = Dictionary::new();
    for (key, value) in dict.iter() {
        let value = renumber(assembly, value, depth.saturating_add(1));
        if matches!(value, Object::Null) {
            continue;
        }
        out.insert(key.clone(), value);
    }
    out
}

/// The objects a rewrite has to be able to name, found once.
///
/// Three of the rewrites are about a dictionary's *position* rather than its contents — the
/// catalog, the name dictionary and a page — and a dictionary carries nothing that says it is
/// the name dictionary. So the positions are resolved from the document's own structure before
/// the walk starts.
struct Sites {
    /// §7.5.5's `/Root`.
    catalog: ObjectId,
    /// §7.7.2's `/Names`, where the catalog states it indirectly.
    names: Option<ObjectId>,
    /// Every page object §7.7.3's tree reaches.
    pages: BTreeSet<ObjectId>,
}

impl Sites {
    /// The positions, read off the document.
    fn of(document: &Document, catalog: ObjectId) -> Self {
        let names = document
            .get(catalog)
            .as_dict()
            .and_then(|dict| dict.get("Names").and_then(Object::as_reference));
        let tree = Pages::new(document);
        let pages = (0..tree.len())
            .filter_map(|index| tree.get(index).and_then(|page| page.id))
            .collect();
        Self {
            catalog,
            names,
            pages,
        }
    }
}

/// What became of one object on the way into the output.
enum Rewritten {
    /// Unchanged: the source's bytes cross to the sink.
    Carried,
    /// Changed, in the source's numbering.
    Changed(Object),
    /// Not written at all.
    Dropped,
}

/// The rewrites, applied one object at a time.
struct Rewriter<'a> {
    /// The document being converted.
    document: &'a Document,
    /// The target, which decides the catalog's `/Version`.
    target: Target,
    /// Which rewrites this conversion performs.
    wanted: &'a BTreeSet<Rewrite>,
    /// The positions three of them turn on.
    sites: Sites,
    /// The objects this conversion adds, in the source's numbering.
    added: BTreeMap<ObjectId, Object>,
    /// The output intent to write into the catalog, where one is being added.
    intent: Option<&'a Intent>,
    /// The metadata stream to write, where one is being written.
    metadata: Option<&'a Metadata>,
}

impl Rewriter<'_> {
    /// What becomes of one object.
    fn rewrite(
        &self,
        id: ObjectId,
        value: &Object,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> Rewritten {
        match value {
            Object::Dictionary(dict) => self
                .rewrite_dictionary(id, dict, applied)
                .map_or(Rewritten::Carried, |dict| {
                    Rewritten::Changed(Object::Dictionary(dict))
                }),
            Object::Stream(stream) => self.rewrite_stream(id, stream, applied),
            _ => Rewritten::Carried,
        }
    }

    /// A dictionary object, rewritten where its position asks for it.
    fn rewrite_dictionary(
        &self,
        id: ObjectId,
        dict: &Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> Option<Dictionary> {
        let mut out = dict.clone();
        let mut changed = false;
        if id == self.sites.catalog {
            changed |= self.rewrite_catalog(&mut out, applied);
        }
        if Some(id) == self.sites.names
            && self.wants(Rewrite::AlternatePresentations)
            && out.remove("AlternatePresentations").is_some()
        {
            count(applied, Rewrite::AlternatePresentations);
            changed = true;
        }
        if self.sites.pages.contains(&id)
            && self.wants(Rewrite::PresentationSteps)
            && out.remove("PresSteps").is_some()
        {
            count(applied, Rewrite::PresentationSteps);
            changed = true;
        }
        changed.then_some(out)
    }

    /// The catalog: §7.7.2's `/Requirements`, `/Version` and a direct `/Names`.
    fn rewrite_catalog(
        &self,
        catalog: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if self.wants(Rewrite::RequirementsDictionary) && catalog.remove("Requirements").is_some() {
            count(applied, Rewrite::RequirementsDictionary);
            changed = true;
        }
        if self.wants(Rewrite::CatalogVersion) && catalog.get("Version").is_some() {
            // ISO 19005-4 section 6.1.12 fixes the shape of this value; §7.7.2's Table 28 gives
            // the entry its meaning, "[t]he version of the PDF specification to which this
            // document conforms". Restating it as the header's own version is the smallest
            // value that satisfies both, and it is the one the file already asserts.
            let (major, minor) = match self.target.part() {
                pdf_archive::Part::Two => (1, 7),
                pdf_archive::Part::Four => (2, 0),
            };
            catalog.insert(
                Name::new(&b"Version"[..]),
                Object::Name(Name::new(format!("{major}.{minor}").as_bytes())),
            );
            count(applied, Rewrite::CatalogVersion);
            changed = true;
        }
        if self.wants(Rewrite::OutputIntent)
            && let Some(intent) = self.intent
        {
            // Written as a direct array whatever the source stated it as: an array object the
            // source held indirectly is not carried, because nothing in the rewritten catalog
            // refers to it any more. Its entries are — a reference among them is renumbered like
            // any other reference this verb rewrites.
            let mut entries = output_intent_entries(self.document, catalog);
            entries.push(Object::Dictionary(intent_dictionary(intent)));
            catalog.insert(Name::new(&b"OutputIntents"[..]), Object::Array(entries));
            changed = true;
        }
        if self.wants(Rewrite::IdentificationSchema)
            && let Some(metadata) = self.metadata
            && metadata.written.is_some()
        {
            catalog.insert(Name::new(&b"Metadata"[..]), Object::Reference(metadata.at));
            changed = true;
        }
        if self.wants(Rewrite::AlternatePresentations)
            && let Some(Object::Dictionary(names)) = catalog.get("Names")
        {
            let mut names = names.clone();
            if names.remove("AlternatePresentations").is_some() {
                catalog.insert(Name::new(&b"Names"[..]), Object::Dictionary(names));
                count(applied, Rewrite::AlternatePresentations);
                changed = true;
            }
        }
        changed
    }

    /// A stream object: the `XObject` rules, and the filter chain.
    fn rewrite_stream(
        &self,
        id: ObjectId,
        stream: &Stream,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> Rewritten {
        // ISO 32000-1's 8.8.2 defines a PostScript XObject as an XObject stream whose
        // `/Subtype` is `PS`, and says that its fragments have no effect when the document is
        // viewed on screen or printed to a non-PostScript device. So dropping one takes nothing
        // off any page this program can draw. ISO 32000-2 defines no such object at all.
        if self.wants(Rewrite::PostScriptXObject) && self.subtype_is(&stream.dict, b"PS") {
            count(applied, Rewrite::PostScriptXObject);
            return Rewritten::Dropped;
        }
        // The producer's own packet, with the identification schema's properties cut out of it
        // and this target's put in — every other byte of it the producer's.
        if self.wants(Rewrite::IdentificationSchema)
            && let Some(metadata) = self.metadata
            && metadata.written.is_none()
            && metadata.at == id
        {
            return Rewritten::Changed(metadata_stream(&stream.dict, &metadata.packet));
        }
        let mut changed = false;
        let mut dict = match self.rewrite_dictionary(id, &stream.dict, applied) {
            Some(rewritten) => {
                changed = true;
                rewritten
            }
            None => stream.dict.clone(),
        };
        changed |= self.rewrite_xobject(&mut dict, applied);
        if self.wants(Rewrite::FlateInsteadOfLzw)
            && let Some(reencoded) = self.reencode(stream, &dict)
        {
            count(applied, Rewrite::FlateInsteadOfLzw);
            return Rewritten::Changed(reencoded);
        }
        if changed {
            return Rewritten::Changed(Object::Stream(std::sync::Arc::new(Stream {
                dict,
                data: std::sync::Arc::clone(&stream.data),
                decryption_failed: stream.decryption_failed,
            })));
        }
        Rewritten::Carried
    }

    /// The image and form `XObject` keys ISO 19005 forbids.
    fn rewrite_xobject(
        &self,
        dict: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if self.subtype_is(dict, b"Image") {
            if self.wants(Rewrite::ImageAlternatesAndOpi) {
                for key in ["Alternates", "OPI"] {
                    if dict.remove(key).is_some() {
                        count(applied, Rewrite::ImageAlternatesAndOpi);
                        changed = true;
                    }
                }
            }
            // §8.9.5.1's Table 87 makes `/Interpolate` "[a] flag indicating whether image
            // interpolation shall be performed by a PDF processor", default false. Stating it
            // false says what ISO 19005 requires in the entry the file already has.
            if self.wants(Rewrite::InterpolationOff)
                && dict.get("Interpolate").is_some()
                && self.document.get_key(dict, "Interpolate") != Object::Boolean(false)
            {
                dict.insert(Name::new(&b"Interpolate"[..]), Object::Boolean(false));
                count(applied, Rewrite::InterpolationOff);
                changed = true;
            }
        }
        if self.subtype_is(dict, b"Form") {
            if self.wants(Rewrite::FormOpi) && dict.remove("OPI").is_some() {
                count(applied, Rewrite::FormOpi);
                changed = true;
            }
            if self.wants(Rewrite::FormPostScript) {
                if dict.remove("PS").is_some() {
                    count(applied, Rewrite::FormPostScript);
                    changed = true;
                }
                let is_postscript = self.subtype_is_named(dict, "Subtype2", b"PS");
                if is_postscript && dict.remove("Subtype2").is_some() {
                    count(applied, Rewrite::FormPostScript);
                    changed = true;
                }
            }
        }
        changed
    }

    /// One stream decoded through the filters this tree reads and re-encoded as one
    /// `FlateDecode`, where its chain names `LZWDecode`.
    ///
    /// `None` where the stream does not name that filter, or where the re-encoding cannot be
    /// done without changing what the file refers to — a chain whose `/Filter` is not a name or
    /// an array of names, a `/DecodeParms` entry that names another object, a stage this tree
    /// does not decode. Every one of those leaves the requirement failed, which the output's
    /// own verdict then reports rather than this function guessing.
    fn reencode(&self, stream: &Stream, dict: &Dictionary) -> Option<Object> {
        let chain = chain_of(self.document, &stream.dict);
        if !chain
            .iter()
            .any(|filter| filter.as_slice() == b"LZWDecode" || filter.as_slice() == b"LZW")
        {
            return None;
        }
        if Document::is_external(stream) || stream.decryption_failed {
            return None;
        }
        let names = filter_names(dict)?;
        if names.len() != chain.len() {
            return None;
        }
        let mut data: std::sync::Arc<[u8]> = std::sync::Arc::clone(&stream.data);
        let mut stop = chain.len();
        for (index, filter) in chain.iter().enumerate() {
            if pdf_syntax::filter::is_image_codec(filter) {
                stop = index;
                break;
            }
            let stage = pdf_syntax::filter::decode_with_parms_reported(
                filter,
                &data,
                parms_at(self.document, &stream.dict, index).as_ref(),
                self.document.limits(),
            )
            .ok()?;
            if stage.damage.is_some() {
                return None;
            }
            data = stage.data;
        }
        // An image codec after the LZW stage would leave its bytes inside the new outer filter,
        // which is right — but the LZW stage has to be one of the ones that was decoded, or
        // nothing has been fixed.
        if !chain
            .get(..stop)?
            .iter()
            .any(|filter| filter.as_slice() == b"LZWDecode" || filter.as_slice() == b"LZW")
        {
            return None;
        }
        let parms = decode_parms(dict, names.len())?;
        if parms
            .get(..stop)
            .unwrap_or_default()
            .iter()
            .any(|value| holds_reference(value, 0))
        {
            return None;
        }
        let encoded = flate_encode(&data, COMPRESSION_LEVEL)?;
        Some(Object::Stream(std::sync::Arc::new(Stream {
            dict: with_flate(dict, &names, &parms, stop, encoded.len()),
            data: encoded.into(),
            decryption_failed: false,
        })))
    }

    /// Whether this dictionary's `/Subtype` is the given name.
    fn subtype_is(&self, dict: &Dictionary, subtype: &[u8]) -> bool {
        self.subtype_is_named(dict, "Subtype", subtype)
    }

    /// Whether the named key resolves to the given name.
    fn subtype_is_named(&self, dict: &Dictionary, key: &str, value: &[u8]) -> bool {
        self.document
            .get_key(dict, key)
            .as_name()
            .is_some_and(|name| name.as_bytes() == value)
    }

    /// Whether this conversion performs the rewrite.
    fn wants(&self, rewrite: Rewrite) -> bool {
        self.wanted.contains(&rewrite)
    }
}

/// Counts one place a rewrite touched.
fn count(applied: &mut BTreeMap<Rewrite, usize>, rewrite: Rewrite) {
    let entry = applied.entry(rewrite).or_insert(0);
    *entry = entry.saturating_add(1);
}

/// A stream's filter chain, in application order, with an indirect entry resolved.
///
/// `pdf_syntax::Document` keeps its own copy of this reading crate-private, so this is the
/// same rule read again through the public API rather than a second reading of §7.4.1: the
/// value is a name or an array of names, and anything else names no filter.
fn chain_of(document: &Document, dict: &Dictionary) -> Vec<Vec<u8>> {
    match document.get_key(dict, "Filter") {
        Object::Name(name) => vec![name.as_bytes().to_vec()],
        Object::Array(items) => items
            .iter()
            .map(|item| document.resolve(item))
            .filter_map(|item| item.as_name().map(|name| name.as_bytes().to_vec()))
            .collect(),
        _ => Vec::new(),
    }
}

/// The `/DecodeParms` entry for the filter at `index`, with an indirect entry resolved.
fn parms_at(document: &Document, dict: &Dictionary, index: usize) -> Option<Dictionary> {
    match document.get_key(dict, "DecodeParms") {
        Object::Dictionary(parms) => Some(parms),
        Object::Array(items) => items
            .get(index)
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_dict().cloned()),
        _ => None,
    }
}

/// A stream's `/Filter` names as its own dictionary states them, one per stage.
///
/// §7.4.1's Table 5 makes `/Filter` "[t]he name of a filter that shall be applied in processing
/// the stream data found between the keywords stream and endstream , or an array of zero, one
/// or several names". Anything else — an indirect entry, an array element that is not a name —
/// answers `None`, and the stream is left as its producer wrote it.
fn filter_names(dict: &Dictionary) -> Option<Vec<Object>> {
    match dict.get("Filter") {
        None => Some(Vec::new()),
        Some(Object::Name(name)) => Some(vec![Object::Name(name.clone())]),
        Some(Object::Array(items)) => items
            .iter()
            .map(|item| match item {
                Object::Name(name) => Some(Object::Name(name.clone())),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

/// A stream's `/DecodeParms` as its own dictionary states them, one per filter.
fn decode_parms(dict: &Dictionary, filters: usize) -> Option<Vec<Object>> {
    match dict.get("DecodeParms") {
        None => Some(vec![Object::Null; filters]),
        Some(Object::Dictionary(parms)) if filters <= 1 => {
            Some(vec![Object::Dictionary(parms.clone())])
        }
        Some(Object::Array(items)) if items.len() == filters => Some(items.clone()),
        _ => None,
    }
}

/// Whether a value names another object anywhere inside it.
///
/// Asked of the `/DecodeParms` entries a re-encoding discards, so that discarding them cannot
/// leave an object in the file that nothing refers to.
fn holds_reference(value: &Object, depth: usize) -> bool {
    if depth >= MAX_WALK_DEPTH {
        return true;
    }
    let deeper = depth.saturating_add(1);
    match value {
        Object::Reference(_) => true,
        Object::Array(items) => items.iter().any(|item| holds_reference(item, deeper)),
        Object::Dictionary(dict) => dict.iter().any(|(_, item)| holds_reference(item, deeper)),
        Object::Stream(stream) => stream
            .dict
            .iter()
            .any(|(_, item)| holds_reference(item, deeper)),
        _ => false,
    }
}

/// The stream dictionary a re-encoded stream gets: one `FlateDecode` in place of the stages
/// that were decoded, and whatever image codec followed them.
fn with_flate(
    dict: &Dictionary,
    names: &[Object],
    parms: &[Object],
    stop: usize,
    length: usize,
) -> Dictionary {
    let mut out = dict.clone();
    let mut filters = vec![Object::Name(Name::new(&b"FlateDecode"[..]))];
    filters.extend(names.get(stop..).unwrap_or_default().iter().cloned());
    let mut kept = vec![Object::Null];
    kept.extend(parms.get(stop..).unwrap_or_default().iter().cloned());
    if let (1, Some(only)) = (filters.len(), filters.first()) {
        out.insert(Name::new(&b"Filter"[..]), only.clone());
    } else {
        out.insert(Name::new(&b"Filter"[..]), Object::Array(filters));
    }
    // Table 5: `/DecodeParms` holds "either the parameter dictionary for that filter, or the
    // null object if that filter has no parameters", and is absent where no filter has any. The
    // `FlateDecode` written here never has parameters, because the predictor the source may
    // have used was reversed by the decode.
    if kept.iter().any(|value| *value != Object::Null) {
        out.insert(Name::new(&b"DecodeParms"[..]), Object::Array(kept));
    } else {
        out.remove("DecodeParms");
    }
    out.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(length).unwrap_or(i64::MAX)),
    );
    out
}
