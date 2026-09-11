//! The middle stage: what the converter decides about a requirement the document failed.
//!
//! **The spine of the verb.** [`REMEDIES`] is the decision table, [`Decision`] is what one row
//! becomes once the caller's authorisations and the document's own preparations are known, and
//! everything in the other four files hangs off that answer: [`super::prepare`] builds what a
//! decision needs, [`super::rewrite`] carries it out, [`super::report`] says what it was.
//!
//! Nothing here reads the standard and nothing here touches the document. `pdf_archive::check` is
//! the reading and [`super::prepare`] is the document; this file is a pure function from a failed
//! requirement's identifier to one of five answers, which is what makes the table reviewable by
//! somebody who is not reading Rust.
use pdf_archive::survey::DeviceFamily;
use pdf_archive::{Judgement, Outcome};

use super::prepare::Prepared;
use super::rewrite::Rewrite;

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
    /// section 3.9: a metadata property its own predefined schema does not define, removed.
    ///
    /// ISO 19005-2 section 6.6.2.3.1 requires every property to *use* the schema whose namespace
    /// it names, and a packet can name a predefined namespace without using it — a value of one
    /// type where the schema defines another. Three routes exist and two are closed: correcting
    /// the value invents content, because a schema says what shape a value has and not what this
    /// document meant, and describing a predefined schema in section 6.6.2.3.2's extension
    /// container would misrepresent it in the file itself. What is left is removing the property,
    /// which throws away what its producer wrote — so the report names every one, its namespace
    /// and the value that was there, and a user can restore it by hand or supply a corrected
    /// source.
    MetadataProperty,
    /// section 3.7: an annotation that stated no flags is made printable.
    ///
    /// ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2 require every annotation but a
    /// `Popup` to state an `/F`, and require its `Print` bit to be set. §12.5.2's Table 166 gives
    /// `/F` a default of 0, so an annotation stating none has every flag clear — and §12.5.3's
    /// Table 167 says what a clear `Print` bit means:
    ///
    /// > If clear, never print the annotation, regardless of whether it is rendered on the screen.
    ///
    /// So the file, read as the standard defines it, says this annotation is never printed, and
    /// the only `/F` that satisfies the requirement says the opposite. Nothing is deleted and no
    /// mark moves on screen; what is lost is the producer's statement about the printed page.
    ///
    /// **How much it costs depends on whether the annotation has an appearance**, and the same
    /// table says so: "If the annotation does not contain any appearance streams this flag shall
    /// be ignored." An annotation with no `/AP` therefore prints no differently for this; one
    /// with an `/AP` now appears on paper where it did not.
    AnnotationPrinting,
}

impl Loss {
    /// Every loss this converter knows how to ask about.
    pub const ALL: [Self; 3] = [
        Self::ImageSmoothing,
        Self::MetadataProperty,
        Self::AnnotationPrinting,
    ];

    /// The word a caller authorises it by.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::ImageSmoothing => "image-smoothing",
            Self::MetadataProperty => "metadata-property",
            Self::AnnotationPrinting => "annotation-printing",
        }
    }

    /// What is lost, in one sentence for a person.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::ImageSmoothing => {
                "image smoothing is turned off, so a low-resolution image will look blockier"
            }
            Self::MetadataProperty => {
                "a metadata property whose predefined schema does not define the value it holds \
                 is removed from the packet, and what its producer wrote there is gone"
            }
            Self::AnnotationPrinting => {
                "an annotation that stated no flags is given the Print flag ISO 19005 requires, \
                 so one whose appearance never printed now prints"
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
    /// Whether [`Loss::MetadataProperty`] was authorised.
    pub metadata_property: bool,
    /// Whether [`Loss::AnnotationPrinting`] was authorised.
    pub annotation_printing: bool,
}

impl Authorisations {
    /// Whether this loss was authorised.
    #[must_use]
    pub const fn grants(self, loss: Loss) -> bool {
        match loss {
            Loss::ImageSmoothing => self.image_smoothing,
            Loss::MetadataProperty => self.metadata_property,
            Loss::AnnotationPrinting => self.annotation_printing,
        }
    }

    /// Authorises one loss.
    pub const fn authorise(&mut self, loss: Loss) {
        match loss {
            Loss::ImageSmoothing => self.image_smoothing = true,
            Loss::MetadataProperty => self.metadata_property = true,
            Loss::AnnotationPrinting => self.annotation_printing = true,
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
    /// The fix is on the far side of ADR 0816's fence: it would edit a content stream, invent a
    /// mark, or author content the source does not carry. Section 5.2 of
    /// `doc/pdf-a-conversion-limits.md` is the first of those and section 5.1 the third — the
    /// converter does not rewrite the producer's page to reach conformance, and it does not
    /// write down semantics nobody produced.
    ///
    /// **Not a gap.** Nothing later in this project's life closes one of these, which is what
    /// separates it from [`Self::NotBuiltYet`]: a user told this has been told the final
    /// answer, and `doc/pdf-a-conversion-limits.md` section 1.1's table is what they do next.
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
    /// The conversion could have been carried out and the **caller asked that it not be**.
    ///
    /// The fourth kind of *no*, and the one that is not a fact about the document or about this
    /// program at all: `doc/pdf-a-conversion-limits.md` section 4.9 offers `--no-substitute` to
    /// the user who would rather a font be refused than replaced, and a report that said "not
    /// built yet" about a flag they had just typed would be wrong in the most confusing
    /// direction available. Unlike [`Self::NotBuiltYet`] the answer changes the moment the flag
    /// comes off, and unlike [`Self::NotThisTarget`] no other target helps.
    Declined(&'static str),
}

impl Because {
    /// The reason, in one sentence.
    #[must_use]
    pub const fn sentence(self) -> &'static str {
        match self {
            Self::TheFence(why)
            | Self::NotBuiltYet(why)
            | Self::NotThisTarget(why)
            | Self::Declined(why) => why,
        }
    }

    /// A stable word for the report's machine-readable form.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::TheFence(_) => "the-fence",
            Self::NotBuiltYet(_) => "not-built-yet",
            Self::NotThisTarget(_) => "not-this-target",
            Self::Declined(_) => "declined",
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
pub(super) enum Answer {
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
    /// ISO 19005-2 section 6.2.4.3's `DeviceCMYK` sentence, which admits two licences this verb
    /// can reach and picks between them on the profile in hand.
    ///
    /// The only row of [`REMEDIES`] whose answer is not a rewrite, because the sentence itself
    /// offers two: a PDF/A output intent holding a **CMYK** destination profile, which is the
    /// right answer and needs a profile only the document's owner can supply, and a
    /// **`DeviceN`-based** `/DefaultCMYK`, which the subclause's NOTE 2 makes device independent
    /// and which needs nothing but the standard. [`decide`] takes the first where the profile in
    /// hand is a CMYK one and the second otherwise; `doc/pdf-a-conversion-limits.md` section 10.1
    /// is the reading, and `doc/questions/A48` the permission.
    ///
    /// **Part 4 states this sentence without the `DeviceN` half**, so its own row is an ordinary
    /// [`Self::Stated`] naming the output intent and a document whose only failure is
    /// `DeviceCMYK` under PDF/A-4 is refused unless a CMYK profile is supplied. That is the
    /// standard's difference rather than this converter's.
    CmykUnderPartTwo,
}

impl Answer {
    /// Every rewrite this answer might perform, whatever the caller has authorised.
    ///
    /// Two rather than one because [`Self::CmykUnderPartTwo`] can be answered two ways and
    /// [`Prepared`] has to build whichever [`decide`] turns out to choose — which it cannot know
    /// before the constructions exist.
    pub(super) const fn rewrites(self) -> [Option<Rewrite>; 2] {
        match self {
            Self::Mechanical(rewrite) | Self::Stated(_, rewrite, _) | Self::Loses(_, rewrite) => {
                [Some(rewrite), None]
            }
            Self::CmykUnderPartTwo => [Some(Rewrite::OutputIntent), Some(Rewrite::DefaultCmyk)],
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
pub(super) struct Remedy {
    /// The `pdf_archive` requirement identifier this answers.
    pub(super) requirement: &'static str,
    /// What is done about it.
    pub(super) answer: Answer,
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

/// What the `DeviceN` `/DefaultCMYK` asserts, which is `doc/questions/A48`'s condition on it.
///
/// `doc/adr/0927`: the sentence travels with the decision. What a reader is told afterwards is
/// §8.6.5.6's remapping — "if such an entry is present, its value shall be used as the colour
/// space for the operation currently being performed" — through a transform §10.4.2.5 states and
/// §10.4.2.1 ranks below §10.3's ICC route in the same breath: "These algorithms are, however,
/// very simple and as perceived by a human viewer they produce only crude approximations of the
/// original colours."
const DEFAULT_CMYK_REINTERPRETS: &str = "every DeviceCMYK value in this file is now read through \
     ISO 32000-2 §10.4.2.5's conversion into this file's sRGB, because ISO 19005-2 section \
     6.2.4.3's NOTE 2 makes a DeviceN-based DefaultCMYK device independent and that is the one \
     licence the standard offers a file with no CMYK profile. The content stream still says what \
     its producer wrote and nothing on any page moves; what changes is how the four numbers are \
     read. §10.4.2.1 calls this family of conversions a crude approximation, which for a \
     photograph separated for a press is a visible loss of fidelity and for a rule or a logo \
     drawn in k is imperceptible — supply that press's profile with --output-intent-profile \
     and the output intent answers the clause instead";

/// What a constructed appearance asserts, which is `doc/questions/A21`'s condition on allowing it.
///
/// `doc/adr/0927`: the sentence travels with the decision. What changes is not *whether* the
/// annotation is drawn — a conforming reader constructs one either way, §12.7.4.3 says so — but
/// that the construction is now fixed, in this program's version of it, and that §12.5.2 has a
/// reader ignore the appearance characteristics the annotation states in favour of these bytes.
const APPEARANCE_REINTERPRETS: &str = "an appearance stream this program constructed is now what \
     every reader draws for these annotations. ISO 32000-2 Table 166 requires a writer to include \
     an appearance dictionary and each subtype's own clause says what its marks are, so nothing \
     here is invented — but the detail is this renderer's, and another reader constructing from \
     the same entries would differ in line joins, in text metrics and in where a caption sits. \
     §12.5.2 then has a reader ignore C, IC, Border, BS and the rest in favour of the stream, so \
     an annotation whose look used to be recomputed from those entries is fixed as it is here. \
     The report names every appearance written, with the page it is on";

/// What embedding a substitute face asserts, which is `doc/questions/A47`'s condition on it.
///
/// `doc/adr/0927`: the sentence travels with the decision. What changes here is larger than what
/// any other `Stated` row changes and the argument for it is `doc/pdf-a-conversion-limits.md`
/// section 4.9's: a file that names a font and does not carry it **has no appearance of its
/// own** — every reader picks a face at display time and they pick different ones — so writing
/// one down settles a question the file left open rather than overriding an answer it gave.
const SUBSTITUTION_REINTERPRETS: &str = "a face this program ships is now embedded for a font \
     this file names and never carried, so the shapes every reader draws for it are this \
     program's choice rather than each reader's. That is a change, and it is the one an archival \
     format asks for: the file had no appearance of its own before, because a non-embedded font \
     is drawn from whatever the machine opening it happens to have. Nothing moves — the glyph \
     widths the font dictionary states are untouched, so every line breaks and every word sits \
     where its producer put it — and the report names, per font, what was asked for, what was \
     embedded and whether the face's own advances were used or restated. Supply the real font \
     with --font, or ask for --no-substitute and the font is refused by name instead";

/// The one requirement identifier that will not fit beside its key inside 100 columns.
const CMYK_UNDER_PART_FOUR: &str =
    "graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent";

/// Why a colour requirement an output intent of another family answers is refused.
const WRONG_FAMILY: &str = "this requirement is licensed by a destination profile of its own \
     colour family, and the profile this conversion has is of another. Supply the right one with \
     --output-intent-profile. For DeviceCMYK with no CMYK profile to hand there is a second \
     licence, and ISO 19005-2 states it where ISO 19005-4 does not: part 2's section 6.2.4.3 \
     admits a DeviceN-based DefaultCMYK, and part 4's requires a device independent one, which \
     is an ICC CMYK profile or nothing";

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
pub(super) const REMEDIES: &[Remedy] = &[
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
    // The one sentence of ISO 19005-2 section 6.2.4.3 that offers two licences rather than one,
    // and the reason `Answer::CmykUnderPartTwo` exists: which of them this verb takes is decided
    // per document, on the colour family of the profile in hand.
    Remedy {
        requirement: "graphics/device-cmyk-needs-a-default-or-a-cmyk-output-intent",
        answer: Answer::CmykUnderPartTwo,
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
    // ISO 19005-2 section 6.2.11.7.2, and Level U and above only: a CMap on every non-exempt
    // font, and usable values in the ones a file already has. Both are answered by the same
    // derivation, because a CMap written over a font's referenced codes states a value for each
    // of them — so a placeholder the producer left is replaced by the same act that supplies a
    // missing entry. Part 4 states the first rule as a `should` and its own row is the second
    // one, which is why the identifier is shared and the clause is not.
    Remedy {
        requirement: "fonts/to-unicode-present",
        answer: Answer::Mechanical(Rewrite::ToUnicode),
    },
    Remedy {
        requirement: "fonts/to-unicode-values-are-usable",
        answer: Answer::Mechanical(Rewrite::ToUnicode),
    },
    // ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1, first requirement:
    // the program of every font a content stream renders is in the file.
    // `doc/pdf-a-conversion-limits.md` section 4.9 makes embedding a face the default rather than
    // a refusal, `doc/questions/A47` settled it, and the condition A47 attaches — refuse rather
    // than guess where no shipped face covers the characters — is `super::fonts`' own gate.
    Remedy {
        requirement: "fonts/font-programs-embedded",
        answer: Answer::Stated(
            None,
            Rewrite::SubstituteFontProgram,
            SUBSTITUTION_REINTERPRETS,
        ),
    },
    // ISO 19005-2 section 6.2.11.5, ISO 19005-4 section 6.2.10.5: the widths the dictionary
    // states and the ones the program states agree. Section 4.9 sets out three ways to reach
    // that and ranks them; this is the second, and the third — restating `/Widths` — is the one
    // it calls **never**, because `/Widths` is what positions the glyphs.
    Remedy {
        requirement: "fonts/widths-agree-with-the-program",
        answer: Answer::Mechanical(Rewrite::RestateFontMetrics),
    },
    // ISO 19005-2 section 6.7.2.2, and Level A only. Answered where the file already carries the
    // structure tree the flag is a claim about, and refused with `NO_STRUCTURE_TREE` where it
    // does not — which is `Prepared::obstacle`'s gate rather than a second row.
    Remedy {
        requirement: "logical-structure/mark-info-marked",
        answer: Answer::Mechanical(Rewrite::MarkInfo),
    },
    // ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3, whose NOTE 1 attributes the rule
    // to §12.5.2's Table 166 rather than restating it — hence the two identifiers for one
    // construction, and the longer exempt list part 4's row reads off that table.
    //
    // `doc/pdf-a-conversion-limits.md` section 4.4's *interesting case* — a `NeedAppearances`
    // of true, where the producer deliberately left the appearances to the reader — is an **Ask**
    // and no interface exists to ask it. Nothing here has to guard it: that document also fails
    // `forms/need-appearances-absent-or-false`, which this table answers with nothing, so the
    // conversion is refused before an appearance is written.
    Remedy {
        requirement: "annotations/appearance-dictionary-present",
        answer: Answer::Stated(None, Rewrite::AppearanceDictionary, APPEARANCE_REINTERPRETS),
    },
    Remedy {
        requirement: "annotations/appearance-dictionary-present-from-base-standard",
        answer: Answer::Stated(None, Rewrite::AppearanceDictionary, APPEARANCE_REINTERPRETS),
    },
    // ISO 19005-2 section 6.3.2, ISO 19005-4 section 6.3.2, first sentence. The second sentence
    // — the Print bit set and four others clear — is a separate row over annotations that *do*
    // state flags, and `doc/pdf-a-conversion-limits.md` section 3.7's other future for those is
    // removing the annotation, which this converter does not offer.
    Remedy {
        requirement: "annotations/flags-entry-present",
        answer: Answer::Loses(Loss::AnnotationPrinting, Rewrite::AnnotationFlags),
    },
    // ISO 19005-2 section 6.6.2.3.1, and the only route of the three
    // `doc/pdf-a-conversion-limits.md` section 3.9 leaves open — which is why it is a loss rather
    // than a default.
    Remedy {
        requirement: "metadata/properties-use-known-schemas",
        answer: Answer::Loses(Loss::MetadataProperty, Rewrite::PropertyOutsideItsSchema),
    },
    // ISO 19005-2 section 6.8 and ISO 19005-4 section 6.9: the two file name keys, each written
    // from the other where the one that is there is ASCII.
    Remedy {
        requirement: "embedded-files/file-and-unicode-names",
        answer: Answer::Mechanical(Rewrite::AssociatedFileNames),
    },
    // ISO 19005-4 section 6.9, which Annex A and Annex B keep for PDF/A-4f and PDF/A-4e.
    Remedy {
        requirement: "embedded-files/relationship-stated",
        answer: Answer::Mechanical(Rewrite::AssociatedFileRelationship),
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

/// Every requirement identifier this converter refuses with a sentence of its own.
///
/// Public for the same test and for the same reason as [`answered`]: a typo in one of these keys
/// does not fail to compile, it quietly demotes a refusal that had an argument behind it to
/// [`NOT_BUILT_YET`]'s "a later slice owes this" — which is the one sentence every row of
/// [`REFUSED_BY_NAME`] exists to avoid saying.
#[must_use]
pub fn refused_by_name() -> Vec<&'static str> {
    REFUSED_BY_NAME.iter().map(|(id, _)| *id).collect()
}

/// Every requirement this converter refuses *by argument* rather than for want of a slice.
///
/// **A third table beside [`REMEDIES`] and [`WRITER_EMITS`], and the reason it exists is that
/// [`NOT_BUILT_YET`] would be a lie for every row of it.** That sentence says the gap is this
/// program's and that a later slice will close it; each row here is a requirement no slice of
/// this converter will ever answer, or one whose answer needs something the file does not
/// contain. Telling a user "not yet" about a document that will never convert sends them back
/// tomorrow for the same answer.
///
/// A requirement in none of the three tables is still refused — by [`decide`], with
/// [`NOT_BUILT_YET`] — so nothing here is load-bearing for safety. What it is load-bearing for
/// is the report saying something true.
const REFUSED_BY_NAME: &[(&str, Because)] = &[
    // `doc/pdf-a-conversion-limits.md` section 5.1's refusal, at the three clauses that are the
    // structure tree itself. `CLAUDE.md`'s "authoring content from nothing" is the fence, and
    // ISO 19005-2 section 6.7.1 advises writers against exactly this in its own words.
    (
        "logical-structure/tagged-pdf",
        Because::TheFence(WILL_NOT_INVENT_STRUCTURE),
    ),
    (
        "logical-structure/structure-tree-root",
        Because::TheFence(WILL_NOT_INVENT_STRUCTURE),
    ),
    // ISO 19005-2 section 6.7.3.2 asks for space characters *inside show strings*, which is the
    // producer's page and ADR 0816's fence.
    (
        "logical-structure/word-boundaries",
        Because::TheFence(WORD_BOUNDARIES_ARE_ON_THE_PAGE),
    ),
    // ISO 19005-2 section 6.2.11.7.3's `/ActualText` is a marked-content property list or a
    // structure element's entry over a *span of a content stream*, so supplying one means
    // deciding where the span starts and ends inside the producer's page.
    (
        "fonts/actual-text-covers-private-use-characters",
        Because::TheFence(ACTUAL_TEXT_IS_ON_THE_PAGE),
    ),
    // ISO 19005-2 section 6.2.11.4.1's last requirement and both parts' `.notdef` clause
    // (section 6.2.11.8, section 6.2.10.9). `doc/pdf-a-conversion-limits.md` section 2.2 splits
    // this by where the missing glyph is, and **this predicate's population is the embedded
    // half**: `pdf_archive` asks the question only of a font whose own program the file carries
    // and this tree reads. A font the file does *not* embed reaches neither row, because
    // section 4.9's substitution builds the program and chooses what each code maps to.
    (
        "fonts/embedded-programs-define-every-glyph-shown",
        Because::TheFence(THE_GLYPH_IS_NOT_IN_THE_PROGRAM),
    ),
    (
        "fonts/no-notdef-glyph-shown",
        Because::TheFence(THE_GLYPH_IS_NOT_IN_THE_PROGRAM),
    ),
    // ISO 19005-2 section 6.7.3.4 needs a judgement about what a producer's own structure type
    // *meant*, which `doc/pdf-a-conversion-limits.md` section 5.1 calls an **Ask** and no
    // interface exists to ask.
    (
        "logical-structure/role-map-terminates-at-a-standard-type",
        Because::NotBuiltYet(ROLE_MAP_NEEDS_A_JUDGEMENT),
    ),
    // ISO 19005-4 Annex A.2 requires the key to be *present*, which is the one requirement in
    // either part that a document can fail by holding nothing at all.
    (
        "embedded-files/pdfa-4f-carries-embedded-files",
        Because::NotThisTarget(FOUR_F_NEEDS_AN_EMBEDDED_FILE),
    ),
    // ISO 19005-2 section 6.8 and ISO 19005-4 section 6.9's first sentence: the embedded file
    // shall itself conform. Converting it means running this whole verb over the file inside the
    // file, which is a slice of its own and is named as one rather than half-done.
    (
        "embedded-files/embedded-file-is-itself-pdfa",
        Because::NotBuiltYet(EMBEDDED_FILE_NOT_CONVERTED),
    ),
    (
        "embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile",
        Because::NotBuiltYet(EMBEDDED_FILE_NOT_CONVERTED),
    ),
    // ISO 19005-4 section 6.9 wants a MIME media type, and nothing in a file specification says
    // what one is: a file name's extension is a convention rather than a statement.
    (
        "embedded-files/associated-file-media-type",
        Because::NotBuiltYet(NO_MEDIA_TYPE_TO_DERIVE),
    ),
    // ISO 19005-2 section 6.6.2.3.3's four tables, each of which names the fields an extension
    // schema container's descriptions have to state. A description that states one wrongly is a
    // different case from one that states nothing, and the corpus's witnesses are all the second:
    // a schema with no `pdfaSchema:schema`, a property with no `pdfaProperty:category`.
    (
        "metadata/extension-schema-container-fields",
        Because::NotBuiltYet(EXTENSION_FIELDS_ARE_NOT_DERIVABLE),
    ),
    // ISO 19005-4 Annex B.2.2, the one requirement of the engineering annex that binds a file
    // rather than a processor and that a document can fail on its content.
    (
        "annotations/three-dimensional-stream-format",
        Because::NotThisTarget(THREE_DIMENSIONAL_FORMAT),
    ),
];

/// Why a page that draws a glyph its own embedded program has not got is refused.
///
/// `doc/pdf-a-conversion-limits.md` section 2.2 calls this a true refusal and says why in one
/// line: the mapping is fixed by a program the file carries, so the only ways out are editing the
/// content stream to stop showing the code or editing the font program to give it a glyph — the
/// first takes a mark off the page and the second invents one. ADR 0816's fence is where both
/// stop, and `CLAUDE.md`'s "authoring content from nothing" is what the second would be.
const THE_GLYPH_IS_NOT_IN_THE_PROGRAM: &str = "this file shows a character code whose glyph its \
     own embedded font program does not define, so the code reaches the .notdef glyph — which \
     ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing \
     operator to reference in any rendering mode. The file carries the program, so the mapping \
     is the producer's and fixed: the only remedies are taking the code off the page or drawing \
     a glyph for it, and this converter does neither — a mark removed is content lost and a \
     glyph drawn is content invented. Where the font is *not* embedded this is not a refusal at \
     all, because doc/pdf-a-conversion-limits.md section 4.9 then builds the program and chooses \
     what each code maps to; supplying the intended font with --font moves this document into \
     that case";

/// `doc/pdf-a-conversion-limits.md` section 5.1's sentence, at the clauses that are the tree.
const WILL_NOT_INVENT_STRUCTURE: &str = "this converter will not invent a structure tree. \
     Deciding that this run of glyphs is a heading and that one a table cell, what the reading \
     order of a two-column page is and what an image depicts, is authoring rather than \
     converting — and a wrong reading order is worse than none, because it misleads \
     confidently. ISO 19005-2 section 6.7.1 advises writers against adding structural \
     information not present in the source solely to achieve conformance. A document that \
     already carries a tree converts to PDF/A-2a; this one does not, and PDF/A-2u and PDF/A-2b \
     ask nothing of logical structure";

/// Why show strings with no spaces between words are not repaired.
const WORD_BOUNDARIES_ARE_ON_THE_PAGE: &str = "word boundaries are asked for inside the show \
     strings themselves, so supplying them means editing the producer's content stream and \
     re-deciding the glyph positioning that goes with it. ADR 0816's fence is where that stops. \
     PDF/A-2u and PDF/A-2b do not state this requirement";

/// Why a Private Use character's `/ActualText` is not supplied.
const ACTUAL_TEXT_IS_ON_THE_PAGE: &str = "an ActualText entry covers a span of a content stream, \
     so writing one means deciding where inside the producer's page that span begins and ends — \
     and its value means deciding what a character in the Private Use Area was for, which is the \
     evidence Level A exists to require rather than something to be manufactured";

/// Why an unmapped non-standard structure type is not role-mapped.
const ROLE_MAP_NEEDS_A_JUDGEMENT: &str = "mapping a non-standard structure type to the nearest \
     standard one is a statement about what the producer's own type meant, and this converter \
     does not guess it: Chapter to Sect is a guess that happens to be right and Sidebar to Note \
     is one that may not be. It is a question for the document's owner, and the interface that \
     asks it — the four levels doc/pdf-a-conversion-limits.md section 9.4 describes — is not \
     built";

/// Why a document with no embedded file cannot be made PDF/A-4f.
const FOUR_F_NEEDS_AN_EMBEDDED_FILE: &str = "ISO 19005-4 Annex A.2 makes an EmbeddedFiles key \
     required of a PDF/A-4f file, and this document holds no embedded file. Attaching one would \
     be adding content no source states, which is not converting. Ask for PDF/A-4 instead: the \
     plain profile is what a document with nothing embedded in it is for";

/// Why an embedded file is not itself converted.
const EMBEDDED_FILE_NOT_CONVERTED: &str = "this file embeds another PDF, and the clause requires \
     that one to conform to ISO 19005 as well. Converting it means running this whole verb over \
     the embedded bytes under a budget of their own, which is not built; PDF/A-4f and PDF/A-4e \
     lift the requirement altogether and admit an embedded file of any type";

/// Why an embedded file's `/Subtype` media type is not supplied.
const NO_MEDIA_TYPE_TO_DERIVE: &str = "ISO 19005-4 section 6.9 asks the embedded file stream for \
     a Subtype that is a MIME media type, and nothing in a file specification states one: a file \
     name's extension is a convention rather than a declaration, and reading it as one would be \
     this converter asserting what the bytes are";

/// Why a missing field of an extension schema container's description is not supplied.
///
/// **Deliberately [`Because::NotBuiltYet`] rather than [`Because::TheFence`]**, and the
/// distinction is the one `doc/adr/0948` insists on. Three of the four fields are prose or a
/// claim about a property that only its producer holds — `pdfaSchema:schema` is what the schema
/// is called, `pdfaProperty:description` says what a property means, and
/// `pdfaProperty:category` asserts whether a value is derived from the document or supplied from
/// outside it — so filling them in is `doc/questions/A48`'s forbidden half. But
/// `doc/pdf-a-conversion-limits.md` section 4.2 already calls emitting an extension schema
/// container a **Default** for the neighbouring row, "authoring in a small way", with an **Ask**
/// where a value type cannot be determined; and `pdfaSchema:prefix` is *derivable*, because the
/// packet itself binds that namespace to a prefix. So this is a question about how far that
/// Default reaches, and a question nobody has answered is a gap rather than a fence.
const EXTENSION_FIELDS_ARE_NOT_DERIVABLE: &str = "this document describes an extension schema \
     whose description leaves out a field ISO 19005-2 section 6.6.2.3.3's tables require of it. \
     What is missing is a name for the schema, a description of what a property means, or the \
     category saying whether a property's value is derived from the document or supplied from \
     outside it — none of which the file states anywhere, so supplying one would be this \
     converter writing metadata about metadata that nobody produced. \
     doc/pdf-a-conversion-limits.md section 4.2 permits emitting such a container for a property \
     that has no description at all and calls it authoring in a small way; whether that \
     permission reaches a description a producer wrote and left incomplete is a question for the \
     document's owner, and no interface exists to ask it";

/// Why a 3D stream in a format Annex B does not name is not converted.
const THREE_DIMENSIONAL_FORMAT: &str = "ISO 19005-4 Annex B.2.2 admits a 3D stream whose Subtype \
     is U3D or PRC, and this document's is neither. Translating 3D artwork between formats is a \
     media engine rather than a rendering question, and CLAUDE.md excludes ISO 32000-2 clause 13 \
     by name. Removing the annotation would reach plain PDF/A-4 at the cost of the artwork, and \
     this converter does not yet offer that loss";

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

/// Decides what to do about every requirement the document failed.
///
/// The middle stage, as a pure function of the validator's report and the caller's
/// authorisations. **A requirement absent from [`REMEDIES`] and [`WRITER_EMITS`] is refused by
/// name** — never passed over, and never answered by a rewrite invented here.
pub(super) fn decide(
    judgement: &Judgement,
    authorised: Authorisations,
    prepared: &Prepared,
) -> Decision {
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
        return Decision::Refused(
            REFUSED_BY_NAME
                .iter()
                .find(|(id, _)| *id == judgement.id)
                .map_or(Because::NotBuiltYet(NOT_BUILT_YET), |(_, because)| *because),
        );
    };
    match remedy.answer {
        // A rewrite is an answer only where the document can take it, and the reason it cannot
        // is the reason the requirement is refused with — never this converter's own paraphrase
        // of it. `Prepared::obstacle` is where that question is asked, once.
        Answer::Mechanical(rewrite) => prepared
            .obstacle(rewrite)
            .map_or(Decision::Mechanical(rewrite), Decision::Refused),
        // ISO 19005-2 section 6.2.4.3's `DeviceCMYK` sentence, whose two licences `Prepared` has
        // already chosen between: a CMYK destination profile answers it outright, and where the
        // profile in hand is of another family the DeviceN default is what NOTE 2 leaves.
        Answer::CmykUnderPartTwo => match (&prepared.intent, &prepared.default_cmyk) {
            (Ok(intent), _) if intent.family == DeviceFamily::Cmyk => Decision::Stated {
                rewrite: Rewrite::OutputIntent,
                reinterprets: OUTPUT_INTENT_REINTERPRETS,
            },
            (_, Ok(_)) => Decision::Stated {
                rewrite: Rewrite::DefaultCmyk,
                reinterprets: DEFAULT_CMYK_REINTERPRETS,
            },
            (_, Err(because)) => Decision::Refused(*because),
        },
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
        // Every other `Stated` row is decided by the requirement and the standard, and then by
        // whether *this* document can take the rewrite — an appearance whose subtype clause
        // states no artwork is the standing case, and the reason is the preparation's own.
        Answer::Stated(_, rewrite, reinterprets) => prepared.obstacle(rewrite).map_or(
            Decision::Stated {
                rewrite,
                reinterprets,
            },
            Decision::Refused,
        ),
        // A loss the document cannot take is refused rather than offered: telling a user that
        // `--authorise` would allow something this file cannot have is worse than saying why.
        Answer::Loses(loss, rewrite) => match prepared.obstacle(rewrite) {
            Some(because) => Decision::Refused(because),
            None if authorised.grants(loss) => Decision::Authorised { loss, rewrite },
            None => Decision::Unauthorised { loss, rewrite },
        },
    }
}
