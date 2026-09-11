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
    /// The requirement asks other requirements' rules again of a narrower population, and its
    /// answer is theirs.
    ///
    /// **Routing rather than a rewrite**, and `doc/pdf-a-mitigations.md` section 7 named the
    /// absence of it as a refusal the conversion would in fact have fixed: ISO 19005-2 section
    /// 6.4.3 and ISO 19005-4 section 6.5.1 say that a signature field's widget meets the
    /// annotation flag and appearance rules, which are three rows of this table already — so a
    /// decision taken per requirement identifier refused a document every one of whose actual
    /// failures had an answer.
    ///
    /// The rows named here are never themselves [`Self::AsUnderlying`]: a compound of a compound
    /// would need an order this table does not have, and [`decide_as_underlying`] passes over
    /// one rather than following it.
    AsUnderlying(&'static [&'static str]),
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
            // None of its own: the rewrites are the underlying rows', and those rows are in this
            // same table, so a preparation asking "did any failed requirement want this rewrite"
            // has already been answered by them.
            Self::AsUnderlying(_) => [None, None],
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

/// What restating a rendering intent asserts, which is `doc/adr/0948`'s condition on it.
///
/// Nothing is lost and something is all the same different: the file used to state a name whose
/// meaning §8.6.5.8 settles for a *processor*, and now states the name itself. A reader that had
/// recognised the producer's private intent — the one case the standard does not govern — sees
/// the substitute instead.
const RENDERING_INTENT_REINTERPRETS: &str = "a rendering intent this file states is not one of \
     ISO 32000-2 §8.6.5.8's four, and the entry now names RelativeColorimetric — which is the \
     intent that subclause has every conforming processor use for a name it does not recognise. \
     No colour value changes and no mark moves; what changes is that a reader which happened to \
     know the producer's own name for an intent is no longer told it. The report names every \
     entry restated";

/// What restating a blend mode array asserts, which is `doc/adr/0948`'s condition on it.
const BLEND_MODE_REINTERPRETS: &str = "a BM entry in this file is an array naming no blend mode \
     ISO 32000-2 defines, and it now names Normal — which is what §8.4.1's Table 57 has every \
     reader take from such an array. Nothing composites differently for any conforming reader; \
     what changes is that a reader which recognised one of the producer's own names would have \
     used it, and now uses Normal like everybody else";

/// What collapsing an appearance subdictionary asserts, which is `doc/adr/0948`'s condition.
const APPEARANCE_STATE_REINTERPRETS: &str = "an annotation's normal appearance was a \
     subdictionary of appearance states and is now the single stream its own AS entry selected. \
     §12.5.2's Table 166 makes AS what selects the applicable stream, so the page draws exactly \
     what it drew before — and the other states go with the subdictionary, so an annotation a \
     reader could have switched (a check box, a trap network) is fixed in the state the file was \
     saved in. The report names every one";

/// What writing `/CIDToGIDMap` `/Identity` asserts, which is `doc/adr/0948`'s condition on it.
const CID_TO_GID_REINTERPRETS: &str = "an embedded Type 2 CIDFont in this file stated no \
     CIDToGIDMap, and now states Identity. ISO 19005-2 section 5.1 makes a PDF/A-2 file one that \
     adheres to ISO 32000-1, whose table gives Identity as that entry's own default, so every \
     reader of this file was already mapping CIDs to glyphs that way and no glyph changes. What \
     changes is that the file now says so, which is what makes it readable the same way under \
     ISO 32000-2 — where the entry is required and has no default";

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
    // -----------------------------------------------------------------------------------------
    // `doc/pdf-a-mitigations.md` section 13.3, *owed, not optional*: the refusals whose right
    // answer loses nothing and which were waiting on code rather than on a decision. Each row's
    // clause is beside it; `super::sites` is where each works out which objects it reaches, and
    // is also what refuses a document failing in the shape the standard leaves unanswered.
    // -----------------------------------------------------------------------------------------

    // ISO 19005-2 section 6.2.6, which part 4 does not state.
    Remedy {
        requirement: "graphics/rendering-intent-entries-name-one-of-four",
        answer: Answer::Stated(
            None,
            Rewrite::RenderingIntent,
            RENDERING_INTENT_REINTERPRETS,
        ),
    },
    // ISO 19005-2 section 6.2.10, ISO 19005-4 section 6.2.9 — the array half of each.
    Remedy {
        requirement: "graphics/graphics-state-blend-modes-are-defined",
        answer: Answer::Stated(None, Rewrite::BlendModeNormal, BLEND_MODE_REINTERPRETS),
    },
    Remedy {
        requirement: "graphics/annotation-blend-modes-are-defined",
        answer: Answer::Stated(None, Rewrite::BlendModeNormal, BLEND_MODE_REINTERPRETS),
    },
    // ISO 19005-2 section 6.3.3, ISO 19005-4 section 6.3.3 — the half an `/AS` entry answers.
    Remedy {
        requirement: "annotations/normal-appearance-shape",
        answer: Answer::Stated(
            None,
            Rewrite::NormalAppearanceFromState,
            APPEARANCE_STATE_REINTERPRETS,
        ),
    },
    // ISO 19005-2 section 6.9, ISO 19005-4 section 6.10 — the `/Order` half of that subclause.
    Remedy {
        requirement: "optional-content/order-lists-every-group",
        answer: Answer::Mechanical(Rewrite::OptionalContentOrder),
    },
    // ISO 19005-2 section 6.2.2, ISO 19005-4 section 6.2.2 — the page half of A003's reading.
    Remedy {
        requirement: "graphics/content-streams-have-an-explicit-resources-dictionary",
        answer: Answer::Mechanical(Rewrite::PageResources),
    },
    // ISO 19005-2 section 6.2.11.4.2, which part 4 does not state.
    Remedy {
        requirement: "fonts/charset-lists-every-glyph-in-the-program",
        answer: Answer::Mechanical(Rewrite::DescriptorSetRemoved),
    },
    Remedy {
        requirement: "fonts/cidset-lists-every-cid-in-the-program",
        answer: Answer::Mechanical(Rewrite::DescriptorSetRemoved),
    },
    // ISO 19005-2 section 6.2.11.3.2, ISO 19005-4 section 6.2.10.3.2 — written at a part 2
    // target, where `super::sites` refuses it at a part 4 one and says why.
    Remedy {
        requirement: "fonts/cid-to-gid-map-present",
        answer: Answer::Stated(None, Rewrite::CidToGidIdentity, CID_TO_GID_REINTERPRETS),
    },
    // ISO 19005-2 section 6.6.2.1, ISO 19005-4 section 6.7.2.1.
    Remedy {
        requirement: "metadata/xmp-packet-header-attributes",
        answer: Answer::Mechanical(Rewrite::PacketHeaderAttributes),
    },
    // ISO 19005-2 section 6.4.2, ISO 19005-4 section 6.4.2 — the second sentence of the pair.
    // The first, `forms/no-xfa-key`, is still refused, which is what makes this one lossless:
    // a document whose form dictionary still states an `/XFA` never reaches a written file.
    Remedy {
        requirement: "forms/no-needs-rendering",
        answer: Answer::Mechanical(Rewrite::NeedsRendering),
    },
    // ISO 19005-2 section 6.2.11.6, ISO 19005-4 section 6.2.10.6 — the two rows of that
    // subclause whose subject is the font dictionary rather than the program. Each is mechanical
    // only where `super::sites` has proved the glyph every code reaches is unchanged, and refused
    // by that preparation where it is not.
    Remedy {
        requirement: "fonts/symbolic-truetype-states-no-encoding",
        answer: Answer::Mechanical(Rewrite::SymbolicTrueTypeEncodingRemoved),
    },
    Remedy {
        requirement: "fonts/non-symbolic-truetype-uses-a-standard-encoding",
        answer: Answer::Mechanical(Rewrite::StandardTrueTypeEncoding),
    },
    // ISO 19005-2 section 6.4.3, ISO 19005-4 section 6.5.1: the annotation rules, asked again of
    // a signature field's widget. Nothing of its own to do — the three rows it names are what
    // answer a widget as they answer any other annotation.
    Remedy {
        requirement: "signatures/signature-widgets-meet-the-annotation-rules",
        answer: Answer::AsUnderlying(ANNOTATION_RULES),
    },
];

/// The three rows ISO 19005-2 section 6.4.3 and ISO 19005-4 section 6.5.1 ask again of a
/// signature field's widget.
///
/// One per sentence of the predicate that judges it: the flags entry it may not omit, the flags
/// it may not set, and the appearance dictionary that may hold nothing but `/N`.
const ANNOTATION_RULES: &[&str] = &[
    "annotations/flags-entry-present",
    "annotations/printable-and-visible",
    "annotations/appearance-dictionary-holds-only-normal",
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
pub(super) const WRITER_EMITS: &[&str] = &[
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
    // ISO 19005-2 section 6.1.3, ISO 19005-4 section 5.1: §14.4's pair of identifiers, which
    // `pdf_syntax::serialize` writes into the trailer of every file it creates — the source's
    // own permanent identifier where it had one, and a digest of the bytes written for the
    // changing half. Found unanswered by `super::census` in session 954, and answered by
    // reading the serializer rather than by building anything.
    "file-structure/file-identifier",
    // ISO 19005-2 section 6.1.13's object-count limit, and the one row of that subclause with a
    // remedy — `doc/pdf-a-mitigations.md` section 13.3. The serializer writes the objects the
    // *converted* document reaches and nothing else, so every object no reference reaches is
    // gone from the output before the count is taken. A file over the limit because it
    // accumulated orphans across twenty years of incremental updates is therefore converted with
    // nothing lost and nobody asked to authorise anything.
    //
    // **Where the objects are genuinely reachable this does not help**, and nothing would: the
    // count is then a fact about what the document holds, and `doc/adr/0947`'s third stage
    // refuses the file rather than writing one wearing a claim it has not earned. The nine other
    // rows of the subclause stay refused with `IMPLEMENTATION_LIMITS`, whose sentence says what
    // this one no longer needs to — ask for PDF/A-4, which states no implementation limits.
    "implementation-limits/indirect-object-count",
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

/// Every requirement this converter refuses with a sentence written for that requirement.
///
/// **A third table beside [`REMEDIES`] and [`WRITER_EMITS`], and the reason it exists is that
/// [`NOT_BUILT_YET`]'s sentence tells a reader almost nothing.** Each row here says which of
/// four things the refusal is: ADR 0816's fence, which no slice of this converter will ever
/// cross; a target that conforms where the one asked for cannot; a caller's own `--no-substitute`;
/// or a rewrite this converter owes, *named*, so that "not yet" comes with what it is waiting on.
/// Telling a user "not yet" about a document that will never convert sends them back tomorrow
/// for the same answer, and telling them "not yet" without saying what for tells them nothing at
/// all.
///
/// A requirement in none of the three tables is still refused — by [`decide`], with
/// [`NOT_BUILT_YET`] — so nothing here is load-bearing for safety. What it is load-bearing for
/// is the report saying something true. `super::census` is what counts the requirements that
/// have reached none of the three, and `tests/archive_unconsidered.txt` holds that count where
/// it is.
pub(super) const REFUSED_BY_NAME: &[(&str, Because)] = &[
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
    // ISO 19005-2 section 6.1.8 and ISO 19005-4 section 6.1.7. A row of this table rather
    // than a branch of [`decide`] since session 954: a refusal `census` cannot see is a
    // refusal that counts as a gap, and this one is an argument rather than a gap.
    (
        "file-structure/bound-names-are-valid-utf8",
        Because::TheFence(UTF8_NAMES),
    ),
    // ---------------------------------------------------------------------------------------
    // Session 954: the requirements the census found with no considered answer at all. Each is
    // one of four things and the sentence says which — a fence nothing closes, a target that
    // conforms where this one cannot, a caller's own refusal, or a rewrite named as owed. The
    // census that found them is `super::census`, and `tests/archive_unconsidered.txt` is the
    // ratchet that keeps the list from growing back.
    // ---------------------------------------------------------------------------------------

    // ISO 19005-2 section 6.1.13's ten limits. Part 4 states no implementation-limits subclause
    // at all, which is what makes every one of these a target question rather than a gap.
    (
        "implementation-limits/character-identifiers",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/devicen-colourants",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/graphics-state-nesting",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/integer-values",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/name-lengths",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/page-boundary-sizes",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/real-values",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/string-lengths",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/values-written-in-content-streams",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    // ISO 19005-2 section 6.1.7.1, ISO 19005-4 section 6.1.6.1:
    // `doc/pdf-a-conversion-limits.md` section 2.3's standing case.
    (
        "file-structure/no-external-stream-data",
        Because::NotThisTarget(EXTERNAL_STREAM_DATA),
    ),
    // The rules a content stream's own bytes fail, which this verb carries byte for byte.
    (
        "graphics/inline-image-interpolation-is-off",
        Because::TheFence(CONTENT_STREAM_IS_THE_PRODUCERS),
    ),
    (
        "file-structure/inline-image-filters",
        Because::TheFence(CONTENT_STREAM_IS_THE_PRODUCERS),
    ),
    (
        "graphics/only-operators-the-base-standard-defines",
        Because::TheFence(CONTENT_STREAM_IS_THE_PRODUCERS),
    ),
    (
        "graphics/rendering-intent-operator-names-one-of-four",
        Because::TheFence(RENDERING_INTENT_OPERAND),
    ),
    (
        "graphics/named-resources-are-defined",
        Because::TheFence(NAMED_RESOURCE_IS_NOT_IN_THE_FILE),
    ),
    // ISO 19005-4 section 6.2.10.5 and section 6.2.10.8: both are the producer's page.
    (
        "fonts/type3-glyph-procedures-state-their-width",
        Because::TheFence(TYPE3_WIDTH_IS_IN_THE_PROCEDURE),
    ),
    (
        "fonts/actual-text-states-no-private-use",
        Because::TheFence(ACTUAL_TEXT_STATES_THE_PRODUCERS_WORDS),
    ),
    // ISO 19005-2 section 6.2.11.6, ISO 19005-4 section 6.2.10.6: the embedded program and the
    // encoding held to each other, where changing either moves what a code draws.
    (
        "fonts/non-symbolic-truetype-differences-are-listed-names",
        Because::TheFence(THE_FONT_PROGRAM_IS_THE_PRODUCERS),
    ),
    (
        "fonts/non-symbolic-truetype-differences-need-the-unicode-cmap",
        Because::TheFence(THE_FONT_PROGRAM_IS_THE_PRODUCERS),
    ),
    (
        "fonts/non-symbolic-truetype-program-maps-every-code",
        Because::TheFence(THE_FONT_PROGRAM_IS_THE_PRODUCERS),
    ),
    (
        "fonts/symbolic-truetype-program-has-a-usable-cmap",
        Because::TheFence(THE_FONT_PROGRAM_IS_THE_PRODUCERS),
    ),
    (
        "fonts/truetype-codes-reach-glyphs-by-the-standard-route",
        Because::TheFence(THE_FONT_PROGRAM_IS_THE_PRODUCERS),
    ),
    // ISO 19005-2 section 6.2.11.3.3, ISO 19005-4 section 6.2.10.3.3.
    (
        "fonts/cmap-embedded-or-predefined",
        Because::TheFence(A_CMAP_IS_THE_ENCODING),
    ),
    (
        "fonts/cmap-uses-only-predefined-cmaps",
        Because::TheFence(A_CMAP_IS_THE_ENCODING),
    ),
    // ISO 19005-2 section 6.2.11.3.1, ISO 19005-4 section 6.2.10.3.1.
    (
        "fonts/cid-system-info-agrees-with-the-cmap",
        Because::TheFence(A_CHARACTER_COLLECTION_IS_A_CLAIM),
    ),
    // ISO 19005-4 section 6.7.5's history entries.
    (
        "metadata/provenance-recorded-action-fields-four",
        Because::TheFence(HISTORY_IS_WHAT_HAPPENED),
    ),
    // ISO 19005-2 section 6.5.1 and section 6.5.2, ISO 19005-4 section 6.6.1 and section 6.6.3,
    // and both parts' section 6.4.1 for the widget's own `/A`: nine rows, one loss.
    (
        "actions/named-action-is-page-navigation",
        Because::NotBuiltYet(ACTION_REMOVAL_NOT_BUILT),
    ),
    (
        "actions/no-additional-actions-dictionary",
        Because::NotBuiltYet(ACTION_REMOVAL_NOT_BUILT),
    ),
    (
        "actions/no-deprecated-set-state-or-no-op-actions",
        Because::NotBuiltYet(ACTION_REMOVAL_NOT_BUILT),
    ),
    (
        "actions/no-javascript-action",
        Because::NotBuiltYet(ACTION_REMOVAL_NOT_BUILT),
    ),
    (
        "actions/no-launch-multimedia-or-form-actions",
        Because::NotBuiltYet(ACTION_REMOVAL_NOT_BUILT),
    ),
    (
        "actions/no-optional-content-or-view-action",
        Because::NotBuiltYet(ACTION_REMOVAL_NOT_BUILT),
    ),
    (
        "actions/optional-content-or-view-action-only-in-engineering-files",
        Because::NotBuiltYet(ACTION_REMOVAL_NOT_BUILT),
    ),
    (
        "actions/additional-actions-outside-widgets-hold-only-annotation-triggers",
        Because::NotBuiltYet(ACTION_REMOVAL_NOT_BUILT),
    ),
    (
        "forms/no-action-on-widget-or-field",
        Because::NotBuiltYet(ACTION_REMOVAL_NOT_BUILT),
    ),
    // ISO 19005-2 section 6.3.1, ISO 19005-4 section 6.3.1.
    (
        "annotations/subtype-defined-in-iso-32000-1",
        Because::NotBuiltYet(ANNOTATION_REMOVAL_NOT_BUILT),
    ),
    (
        "annotations/subtype-defined-in-iso-32000-2",
        Because::NotBuiltYet(ANNOTATION_REMOVAL_NOT_BUILT),
    ),
    (
        "annotations/three-dimensional-only-in-engineering-files",
        Because::NotBuiltYet(ANNOTATION_REMOVAL_NOT_BUILT),
    ),
    (
        "annotations/file-attachment-only-in-embedded-file-files",
        Because::NotBuiltYet(ANNOTATION_REMOVAL_NOT_BUILT),
    ),
    // ISO 19005-2 section 6.3.2, ISO 19005-4 section 6.3.2: the half of section 3.7 that is not
    // an annotation stating no flags at all.
    (
        "annotations/printable-and-visible",
        Because::NotBuiltYet(HIDDEN_ANNOTATION_NOT_BUILT),
    ),
    // ISO 19005-2 section 6.3.3, ISO 19005-4 section 6.3.3. The neighbouring row of this
    // subclause — a normal appearance that is a subdictionary of states — is answered by
    // `REMEDIES` where the annotation's own `/AS` says which state it is in.
    (
        "annotations/appearance-dictionary-holds-only-normal",
        Because::NotBuiltYet(EXTRA_APPEARANCE_STATES_NOT_BUILT),
    ),
    // ISO 19005-2 section 6.1.3 and section 6.1.7.2, ISO 19005-4 section 6.1.3 and section
    // 6.1.6.2: `doc/pdf-a-conversion-limits.md` section 3.5.
    (
        "file-structure/no-encryption",
        Because::NotBuiltYet(ENCRYPTION_REMOVAL_NOT_BUILT),
    ),
    (
        "file-structure/crypt-filter-is-identity",
        Because::NotBuiltYet(ENCRYPTION_REMOVAL_NOT_BUILT),
    ),
    // ISO 19005-2 section 6.1.12, ISO 19005-4 section 6.1.11: section 3.6.
    (
        "file-structure/document-signature-states-no-digest",
        Because::NotBuiltYet(SIGNATURE_STRUCTURE_NOT_BUILT),
    ),
    (
        "file-structure/permissions-dictionary-keys",
        Because::NotBuiltYet(SIGNATURE_STRUCTURE_NOT_BUILT),
    ),
    // ISO 19005-4 section 6.1.3's two sentences about the document information dictionary.
    (
        "file-structure/document-information-dictionary-holds-only-a-modification-date",
        Because::NotBuiltYet(INFO_DICTIONARY_NOT_RECONCILED),
    ),
    (
        "file-structure/document-information-dictionary-needs-piece-info",
        Because::NotBuiltYet(INFO_DICTIONARY_NOT_RECONCILED),
    ),
    // ISO 19005-2 section 6.1.7.2, ISO 19005-4 section 6.1.6.2.
    (
        "file-structure/stream-filters-are-standard",
        Because::NotBuiltYet(NON_STANDARD_FILTER_NOT_BUILT),
    ),
    // ISO 19005-2 section 6.2.11.3.3, ISO 19005-4 section 6.2.10.3.3.
    (
        "fonts/embedded-cmap-states-its-own-write-mode",
        Because::NotBuiltYet(WRITE_MODE_DISAGREEMENT),
    ),
    // ISO 19005-4 section 6.2.10.5.
    (
        "fonts/vertical-metrics-agree-with-the-program",
        Because::NotBuiltYet(VERTICAL_METRICS_NOT_RESTATED),
    ),
    // ISO 19005-2 section 6.4.2, ISO 19005-4 section 6.4.2.
    (
        "forms/no-xfa-key",
        Because::NotBuiltYet(XFA_REMOVAL_NOT_BUILT),
    ),
    // ISO 19005-2 section 6.4.1, ISO 19005-4 section 6.4.1.
    (
        "forms/need-appearances-absent-or-false",
        Because::NotBuiltYet(NEED_APPEARANCES_NOT_BUILT),
    ),
    // ISO 19005-2 section 6.2.3, ISO 19005-4 section 6.2.3: the destination profile the file
    // already holds.
    (
        "graphics/destination-profile-carries-the-tags-its-class-requires",
        Because::NotBuiltYet(DESTINATION_PROFILE_NOT_REPLACED),
    ),
    (
        "graphics/destination-profile-class-and-colour-space",
        Because::NotBuiltYet(DESTINATION_PROFILE_NOT_REPLACED),
    ),
    (
        "graphics/destination-profile-states-a-correct-profile-id",
        Because::NotBuiltYet(DESTINATION_PROFILE_NOT_REPLACED),
    ),
    // ISO 19005-2 section 6.2.4.2, ISO 19005-4 section 6.2.4.2: an `ICCBased` space's own.
    (
        "graphics/icc-profiles-carry-the-tags-a-permitted-edition-requires",
        Because::NotBuiltYet(ICC_SPACE_PROFILE_NOT_REPLACED),
    ),
    (
        "graphics/icc-profiles-carry-the-tags-their-version-requires",
        Because::NotBuiltYet(ICC_SPACE_PROFILE_NOT_REPLACED),
    ),
    (
        "graphics/icc-profiles-claim-a-permitted-edition",
        Because::NotBuiltYet(ICC_SPACE_PROFILE_NOT_REPLACED),
    ),
    (
        "graphics/icc-profiles-conform-to-the-base-standard",
        Because::NotBuiltYet(ICC_SPACE_PROFILE_NOT_REPLACED),
    ),
    // ISO 19005-2 section 6.2.3, ISO 19005-4 section 6.2.3: the shape of the array itself.
    (
        "graphics/one-destination-profile-per-output-intents-array",
        Because::NotBuiltYet(OUTPUT_INTENT_ARRAY_NOT_TIDIED),
    ),
    (
        "graphics/pdfa-output-intent-states-a-destination-profile",
        Because::NotBuiltYet(OUTPUT_INTENT_ARRAY_NOT_TIDIED),
    ),
    (
        "graphics/page-output-intents-have-the-same-shape",
        Because::NotBuiltYet(OUTPUT_INTENT_ARRAY_NOT_TIDIED),
    ),
    (
        "graphics/no-destination-profile-reference",
        Because::NotBuiltYet(PROFILE_REFERENCE_NOT_REMOVED),
    ),
    (
        "graphics/no-destination-profile-reference-in-a-pdfx-output-intent",
        Because::NotBuiltYet(PROFILE_REFERENCE_NOT_REMOVED),
    ),
    // ISO 19005-2 section 6.2.5, ISO 19005-4 section 6.2.5: section 4.8's two halves, which are
    // two different facts about what a reader sees.
    (
        "graphics/halftone-type-is-one-or-five",
        Because::NotBuiltYet(HALFTONE_NOT_REMOVED),
    ),
    (
        "graphics/no-halftone-name",
        Because::NotBuiltYet(HALFTONE_NOT_REMOVED),
    ),
    (
        "graphics/no-halftone-origin-in-a-graphics-state",
        Because::NotBuiltYet(HALFTONE_NOT_REMOVED),
    ),
    (
        "graphics/no-halftone-phase-in-a-graphics-state",
        Because::NotBuiltYet(HALFTONE_NOT_REMOVED),
    ),
    (
        "graphics/halftone-transfer-function-only-where-required",
        Because::NotBuiltYet(HALFTONE_NOT_REMOVED),
    ),
    (
        "graphics/no-transfer-function-in-a-graphics-state",
        Because::NotBuiltYet(TRANSFER_FUNCTION_NOT_REMOVED),
    ),
    (
        "graphics/second-transfer-function-is-default",
        Because::NotBuiltYet(TRANSFER_FUNCTION_NOT_REMOVED),
    ),
    // ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3: section 4.10's table, split by
    // whether the offending field is in the JP2 wrapper or in the codestream.
    (
        "graphics/jpeg2000-one-best-colour-space-specification",
        Because::NotBuiltYet(JPEG2000_BOX_NOT_REWRITTEN),
    ),
    (
        "graphics/jpeg2000-colour-specification-method",
        Because::NotBuiltYet(JPEG2000_BOX_NOT_REWRITTEN),
    ),
    (
        "graphics/jpeg2000-bit-depth",
        Because::NotBuiltYet(JPEG2000_SAMPLES_NOT_RE_ENCODED),
    ),
    (
        "graphics/jpeg2000-channel-count",
        Because::NotBuiltYet(JPEG2000_SAMPLES_NOT_RE_ENCODED),
    ),
    (
        "graphics/jpeg2000-no-ciejab-colour-space",
        Because::NotBuiltYet(JPEG2000_SAMPLES_NOT_RE_ENCODED),
    ),
    // ISO 19005-4 section 6.2.4.2 and section 6.2.4.4.
    (
        "graphics/no-icc-space-duplicating-the-output-intent-profile",
        Because::NotBuiltYet(DUPLICATE_PROFILE_NOT_COLLAPSED),
    ),
    (
        "graphics/separation-alternate-space-does-not-duplicate-a-current-profile",
        Because::NotBuiltYet(DUPLICATE_PROFILE_NOT_COLLAPSED),
    ),
    // ISO 19005-2 section 6.2.4.2, ISO 19005-4 section 6.2.4.2.
    (
        "graphics/no-overprint-mode-one-under-icc-cmyk",
        Because::NotBuiltYet(OVERPRINT_MODE_NOT_CHANGED),
    ),
    // ISO 19005-2 section 6.2.4.4, ISO 19005-4 section 6.2.4.4: section 4.5's two rows.
    (
        "graphics/separations-of-one-name-agree",
        Because::NotBuiltYet(SEPARATIONS_NOT_RECONCILED),
    ),
    (
        "graphics/spot-colourants-appear-in-the-colorants-dictionary",
        Because::NotBuiltYet(COLORANTS_NOT_SYNTHESISED),
    ),
    // ISO 19005-2 section 6.2.9.2, ISO 19005-4 section 6.2.8.2: section 2.3's one workaround.
    (
        "graphics/no-reference-xobjects",
        Because::NotBuiltYet(REFERENCE_XOBJECT_NOT_PROXIED),
    ),
    // ISO 19005-2 section 6.7.4, which binds PDF/A-2a alone.
    (
        "logical-structure/catalog-language-identifier",
        Because::NotBuiltYet(LANGUAGE_IDENTIFIER_NOT_REMOVED),
    ),
    (
        "logical-structure/element-and-property-list-language-identifiers",
        Because::NotBuiltYet(LANGUAGE_IDENTIFIER_NOT_REMOVED),
    ),
    // ISO 19005-2 section 6.6.2.1, ISO 19005-4 section 6.7.2.1.
    (
        "metadata/xmp-packets-well-formed",
        Because::NotBuiltYet(XMP_PACKET_NOT_REBUILT),
    ),
    (
        "metadata/xmp-packets-state-one-rdf-element",
        Because::NotBuiltYet(XMP_PACKET_NOT_REBUILT),
    ),
    (
        "metadata/xmp-packets-meet-the-xmp-data-model",
        Because::NotBuiltYet(XMP_PACKET_NOT_REBUILT),
    ),
    (
        "metadata/xmp-character-data-only-in-simple-values",
        Because::NotBuiltYet(XMP_STRAY_CHARACTER_DATA_NOT_REMOVED),
    ),
    // ISO 19005-2 section 6.6.2.3.2 and section 6.6.4.
    (
        "metadata/extension-schemas-embedded",
        Because::NotBuiltYet(EXTENSION_CONTAINER_NOT_EMITTED),
    ),
    (
        "metadata/identification-amendment-form",
        Because::NotBuiltYet(AMENDMENT_IDENTIFIER_NOT_REMOVED),
    ),
    // ISO 19005-2 section 6.9, ISO 19005-4 section 6.10: the two of section 3.8's three rules
    // that are still refused. The `/Order` rule is a row of `REMEDIES`.
    (
        "optional-content/configuration-names",
        Because::NotBuiltYet(CONFIGURATION_NAME_NOT_WRITTEN),
    ),
    (
        "optional-content/no-automatic-states",
        Because::NotBuiltYet(AUTOMATIC_STATES_NOT_REMOVED),
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

/// Why a document outside ISO 19005-2's implementation limits is a PDF/A-4 document.
///
/// `doc/pdf-a-conversion-limits.md` section 2.5. Ten requirements share this: every one of them
/// is a limit on what the file may contain rather than on what it may say, so every fix is an
/// edit to the document's own values — and part 4 states no implementation-limits subclause at
/// all, which turns the whole family into a question about the target.
const IMPLEMENTATION_LIMITS: &str = "ISO 19005-2 section 6.1.13 sets a hard limit this document \
     exceeds, and every way of meeting one is an edit to what the document says: re-nesting q \
     and Q, rescaling a page, re-encoding a colour space, renaming an object, or dropping \
     objects until the count falls. ISO 19005-4 states no implementation-limits subclause at all \
     — its section 6.1 runs to 6.1.12 — so a large-format drawing, a deeply nested content \
     stream or a 40-colourant DeviceN space can be PDF/A-4 and cannot be PDF/A-2. Ask for \
     PDF/A-4";

/// Why a stream whose bytes live outside the file is refused.
///
/// `doc/pdf-a-conversion-limits.md` section 2.3, and one of the two places where "cannot view
/// it either" and "cannot archive it" agree.
const EXTERNAL_STREAM_DATA: &str = "a stream in this file states F, FFilter or FDecodeParams, \
     which puts its data on somebody else's disk or server. Fetching it is a network operation \
     this program does not have and CLAUDE.md principle 3 will not acquire, and a file assembled \
     from an unverifiable fetch is not an archival object. Both parts forbid the keys, so no \
     other target helps; what does is obtaining the referenced data and having the producer \
     embed it. Where nothing draws the stream, ISO 19005-2 section 6.2.2's exemption for a named \
     resource the content stream never references would free it, and this tree does not yet \
     state that exemption — doc/todo/62";

/// Why a rule a content stream's own bytes fail is not repaired.
const CONTENT_STREAM_IS_THE_PRODUCERS: &str = "this requirement is failed by bytes inside a \
     content stream, which this verb carries from the source byte for byte. Meeting it means \
     editing the producer's page, and ADR 0816's fence is where that stops — \
     doc/pdf-a-conversion-limits.md section 5.2. The one exception doc/questions/A50 grants is a \
     closed list of operator spellings the standard itself documents as equivalent, applied only \
     where a deprecation rule names one, and this is not that";

/// Why an unrecognised `ri` operand is not restated, where an `/RI` entry's answer differs.
const RENDERING_INTENT_OPERAND: &str = "the rendering intent operator's operand names an intent \
     ISO 32000-2 does not define. §8.6.5.8 says what a processor does with such a name — it uses \
     RelativeColorimetric — so the answer is known, and the operand is inside a content stream \
     this verb carries byte for byte, which is ADR 0816's fence. The same name in a graphics \
     state's RI entry or an image dictionary's Intent entry is not refused for this reason: that \
     is an object, and doc/pdf-a-conversion-limits.md's own answer for it is a rewrite nobody \
     has built";

/// Why a name a resources dictionary does not define is not supplied.
const NAMED_RESOURCE_IS_NOT_IN_THE_FILE: &str = "a content stream names a resource its resources \
     dictionary does not define, and the two ways out are opposite fences: supplying the \
     resource invents an object the producer never wrote, and taking the reference off the page \
     edits the producer's content stream. §7.8.3 makes the resources dictionary what a name is \
     resolved through, so nothing in the file says what the missing one was meant to be";

/// Why a Type 3 glyph procedure's width is not reconciled with the font dictionary's.
const TYPE3_WIDTH_IS_IN_THE_PROCEDURE: &str = "a Type 3 glyph procedure's d0 or d1 operands \
     disagree with the width its font dictionary states, and both halves are closed. The \
     operands are inside the glyph procedure, which is a content stream this verb carries byte \
     for byte; and §9.2.4 makes the dictionary's widths what positions every glyph on the line, \
     so doc/pdf-a-conversion-limits.md section 4.9 restates those never. Changing either moves \
     marks on the page";

/// Why an `/ActualText` holding a Private Use character is neither corrected nor removed.
const ACTUAL_TEXT_STATES_THE_PRODUCERS_WORDS: &str = "ISO 19005-4 section 6.2.10.8 forbids a \
     Private Use character inside an ActualText entry, and that entry is a marked-content \
     property list or a structure element's entry over a span of the producer's page. Changing \
     its value means deciding what the producer meant that span to say, which doc/questions/A48 \
     forbids; removing it takes away the only statement the file makes about what those glyphs \
     spell, which at Level A is the evidence the level exists to require";

/// Why a TrueType font's program and encoding are not made to agree.
const THE_FONT_PROGRAM_IS_THE_PRODUCERS: &str = "this rule holds an embedded font program and \
     the font dictionary's encoding to each other, and meeting it means changing which glyph a \
     character code selects: adding a cmap subtable to the program, or rewriting the Differences \
     array that names the glyphs. The first invents outlines the producer never shipped and the \
     second moves the marks on the page, so ADR 0816's fence closes both. Where the file embeds \
     no program at all this is a different case: doc/pdf-a-conversion-limits.md section 4.9 \
     builds one, and --font supplies the real face";

/// Why a `CMap` this file neither embeds nor names from the predefined set is not written.
const A_CMAP_IS_THE_ENCODING: &str = "a CMap is what maps a composite font's character codes to \
     CIDs, so supplying one this file neither embeds nor takes from the predefined set means \
     writing the encoding its producer did not — every code would then select whatever this \
     program chose, which is doc/questions/A48's forbidden half. The same holds of an embedded \
     CMap that builds on one the base standard does not predefine: what it builds on is not here \
     to be carried, and inventing it invents the mapping";

/// Why a `CIDSystemInfo` and a `CMap` that disagree are not reconciled.
const A_CHARACTER_COLLECTION_IS_A_CLAIM: &str = "the CIDFont's registry, ordering and supplement \
     say which character collection its CIDs are numbered in, and the CMap's say the same of the \
     codes it produces. Making them agree means restating one of the two, which relabels what \
     every CID in this font means — the file states two collections and nothing in it says which \
     its producer meant";

/// Why a producer's incomplete XMP history entry is not completed.
const HISTORY_IS_WHAT_HAPPENED: &str = "ISO 19005-4 section 6.7.5 requires every action recorded \
     in the XMP history to state what was done and when. An entry missing either records \
     something this converter did not witness, so filling it in would be writing down a \
     provenance nobody has — doc/questions/A48's line, and the opposite of what an audit trail \
     is for. This converter adds its own history entries for what it does \
     (doc/pdf-a-conversion-limits.md section 4.2); it does not complete a producer's";

/// Why removing an action is not built, and which targets permit which actions.
const ACTION_REMOVAL_NOT_BUILT: &str = "an action carries behaviour, and the only way to meet \
     this clause is to remove it. doc/pdf-a-conversion-limits.md section 3.3 calls that an Ask, \
     because a form that computed its own fields stops computing them, and neither the \
     authorisation word nor the rewrite that takes an action out of a dictionary is built. Two \
     target facts belong beside it: ISO 19005-4 section 6.6.2 permits a JavaScript action \
     outright and its section 6.6.3 permits an AA entry on a widget annotation, so a document \
     refused here for PDF/A-2 may convert to PDF/A-4 untouched; and PDF/A-4e admits a \
     SetOCGState or GoTo3DView action that plain PDF/A-4 does not";

/// Why removing an annotation of a forbidden subtype is not built.
const ANNOTATION_REMOVAL_NOT_BUILT: &str = "this annotation's subtype is one ISO 19005 does not \
     admit for the target asked for, and removal is the only remedy. \
     doc/pdf-a-conversion-limits.md section 3.2 makes it an Ask — a removed annotation takes its \
     normal appearance off the page with it, and for a Screen, Movie or Sound one the media \
     stream goes too — and neither the authorisation word nor the rewrite is built. Re-badging \
     it as a Stamp to keep the mark is ADR 0816's fence rather than a fix. PDF/A-4e admits a 3D \
     or RichMedia annotation and PDF/A-4f a FileAttachment, so for those two the target is the \
     shorter route";

/// Why an annotation the producer hid is neither shown nor removed.
const HIDDEN_ANNOTATION_NOT_BUILT: &str = "this annotation states flags ISO 19005 forbids — \
     Hidden, Invisible, NoView or ToggleNoView set, or Print clear. \
     doc/pdf-a-conversion-limits.md section 3.7: an annotation somebody hid has two futures and \
     no third, becoming visible and printable or being removed, and both change the document. \
     What is built is the smaller half, an annotation stating no F entry at all, which is not \
     one anybody hid: --authorise annotation-printing writes the Print flag for that case. \
     Un-hiding one that states a flag, and removing it, are the two rewrites owed, and section \
     3.7 makes removal the default of the two";

/// Why a rollover or down appearance is not dropped.
const EXTRA_APPEARANCE_STATES_NOT_BUILT: &str = "this annotation's appearance dictionary states \
     a rollover or a down appearance beside its normal one, and ISO 19005 admits only N. \
     Dropping R and D loses what §12.5.5 has a reader draw while the pointer is over the \
     annotation or the mouse button is down, which is a loss doc/pdf-a-conversion-limits.md \
     section 3 would have a caller authorise before it happened. Neither the word nor the \
     rewrite exists";

/// Why a compound row is refused where none of the rules it names failed.
///
/// The one case [`Answer::AsUnderlying`] cannot route: the compound's own predicate found
/// something at a population the rules it names did not reach. A signature field's widget that no
/// page's `/Annots` array holds is the standing shape — `pdf_archive` walks the annotation rules
/// over the pages and the signature rule over the form's field tree, and a widget in the second
/// and not the first is a document whose annotation this conversion never sees.
const NOTHING_UNDERLYING_FAILED: &str = "this requirement asks other rules of this same table \
     again, of a narrower population, and every one of those rules passed. So the failure is at \
     something the rules themselves did not reach — for a signature field's widget, an \
     annotation the interactive form names and no page's Annots array holds. This conversion \
     rewrites annotations the pages reach, and one reachable only through the field tree is not \
     among them";

/// Why encryption is not removed.
const ENCRYPTION_REMOVAL_NOT_BUILT: &str = "both parts forbid an Encrypt key in the trailer \
     outright, and with it a Crypt filter whose name is not Identity. Removing encryption is \
     mechanically small and doc/pdf-a-conversion-limits.md section 3.5 makes it an Ask and never \
     a default: the archived copy becomes readable by anyone holding it, and Table 22's \
     permission flags stop being asserted with it. The authorisation word and the \
     decrypt-then-write path are not built, and a document whose password is not to hand cannot \
     be read at all (section 2.4)";

/// Why the keys a signature structure keeps are not stripped.
const SIGNATURE_STRUCTURE_NOT_BUILT: &str = "this key belongs to a permissions dictionary or a \
     signature reference that the conversion has already invalidated: \
     doc/pdf-a-conversion-limits.md section 3.6 — a signature covers a byte range of one file, a \
     conversion rewrites every offset, and no converted document carries its source's \
     signatures. Removing the key is a small rewrite and it belongs with the report section 3.6 \
     asks for, which names each signature, its signer and whether it validated before the \
     conversion. Neither is built";

/// Why `/Info` is not reconciled with the XMP packet.
const INFO_DICTIONARY_NOT_RECONCILED: &str = "ISO 19005-4 section 6.1.3 leaves a document \
     information dictionary no entry but ModDate, and only where the catalog states a PieceInfo. \
     §14.3.3 deprecates the dictionary, so the answer is doc/pdf-a-conversion-limits.md section \
     4.2's: what it holds moves into the XMP packet and the dictionary goes. This converter \
     writes the identification schema into that packet and does not reconcile Info with it, \
     which is the rewrite this needs — and a value moved wrongly would be metadata this \
     converter had asserted rather than carried";

/// Why a stream in a filter this tree cannot decode is refused.
const NON_STANDARD_FILTER_NOT_BUILT: &str = "this stream names a filter outside the set the base \
     standard defines, so no conforming reader can decode it and this program cannot either. \
     Re-encoding needs the decoded bytes, which needs a decoder nobody specifies; what is left \
     is dropping the stream, which is doc/pdf-a-conversion-limits.md section 3's kind of loss \
     and is not offered. Where nothing draws the stream, ISO 19005-2 section 6.2.2's exemption \
     for an unreferenced named resource would free it, and this tree does not yet state that \
     exemption — doc/todo/62";

/// Why a `CMap` stream and its program disagreeing about `/WMode` is not settled.
const WRITE_MODE_DISAGREEMENT: &str = "this CMap stream's WMode entry and the write mode the \
     CMap program itself states disagree, and between them they decide whether the text runs \
     down the page or across it. Making them agree means choosing which of the file's two \
     statements its producer meant, which changes either the writing direction a reader lays the \
     text out in or the program's own bytes. That is a question for the document's owner, and no \
     interface exists to ask it";

/// Why vertical metrics are not restated the way horizontal advances already are.
///
/// **The direction matters and `doc/pdf-a-mitigations.md` had it the wrong way round.** That
/// entry proposed restating `/DW2` and `/W2` from the program, "on the same argument that
/// already justifies the horizontal case" — but the horizontal case restates the *program*, and
/// for exactly the reason that makes the other direction unsafe: §9.2.4 makes the font
/// dictionary's numbers what a processor positions glyphs by without looking inside the program,
/// and §9.7.4.3 gives `/DW2` and `/W2` that role in vertical writing. Restating them would move
/// every glyph on a vertical line; restating the program's `vmtx` moves nothing.
///
/// What this waits on is therefore the *writer* rather than the reader:
/// `pdf_font::LoadedFont::program_vertical_advance` already hands the program's number back, and
/// `pdf_font::restate` rewrites an sfnt's `hmtx` and a charstring's leading width and nothing
/// vertical.
const VERTICAL_METRICS_NOT_RESTATED: &str = "doc/pdf-a-conversion-limits.md section 4.9's \
     restatement, in the other writing direction, and it is the program that would be restated \
     rather than the dictionary. §9.2.4 makes the font dictionary's numbers what positions a \
     glyph without looking inside the program, and §9.7.4.3 gives DW2 and W2 that role going \
     down the page — so rewriting them would move every glyph on a vertical line, and rewriting \
     the program's own vmtx moves nothing. This tree's font reader already states the program's \
     vertical advance; what it cannot yet do is write one, which is where pdf_font::restate \
     rewrites an sfnt's hmtx and nothing vertical. That writer is what this requirement waits \
     on";

/// Why `/XFA` is not removed.
///
/// **The pair this used to answer is now one row.** ISO 19005-2 section 6.4.2 forbids two
/// entries and the catalog's `/NeedsRendering` is the half whose answer the base standard prints
/// — §7.7.2's Table 29 deprecates it and gives it a default of `false` — so it is a
/// `Rewrite::NeedsRendering` and no longer waits on this. What is left here is the half that
/// genuinely needs a judgement about the producer's pipeline.
const XFA_REMOVAL_NOT_BUILT: &str = "both parts forbid an XFA entry in the interactive form \
     dictionary. doc/pdf-a-conversion-limits.md \
     section 3.4's default is to keep the AcroForm's data and drop the XFA key — ISO 32000-2 \
     Annex K requires a conforming hybrid file's AcroForm entries to be consistent with the XFA \
     information, so for a static form the AcroForm is the form — and to refuse a dynamic one \
     outright, because there the AcroForm is not the document and the output would be a \
     placeholder page wearing a conformance claim. Neither the key removal nor the test that \
     tells the two apart is built, and CLAUDE.md excludes rendering XFA, so flattening one is \
     not available";

/// Why `/NeedAppearances` is not simply written `false`.
const NEED_APPEARANCES_NOT_BUILT: &str = "this form asks a reader to build its field \
     appearances, which ISO 32000-2 Table 224 deprecates and ISO 19005 forbids. The honest \
     answer is a pair: construct every field's appearance — doc/pdf-a-conversion-limits.md \
     section 4.4, which this converter does for annotations whose own subtype clause states what \
     to draw — and then write the flag false, so the file says what it shows. Writing the flag \
     alone would assert appearances nobody built, and the field construction for every widget in \
     the form is what this needs";

/// Why the destination profile a file already holds is not replaced.
const DESTINATION_PROFILE_NOT_REPLACED: &str = "the destination profile this file's own output \
     intent carries is not one ISO 19005 admits: the wrong device class or colour space, missing \
     the tags the ICC edition its header names requires, or stating a profile identifier that is \
     not the digest of its own bytes. An ICC profile is opaque data — this converter can write a \
     profile it has, sRGB or the one --output-intent-profile supplies, but replacing the profile \
     a document already holds changes what every device colour in the file means, which is \
     doc/adr/0927's statement rather than a repair. That replacement path is not built";

/// Why an `ICCBased` space's own profile is not replaced.
const ICC_SPACE_PROFILE_NOT_REPLACED: &str = "an ICCBased colour space in this file carries a \
     profile ISO 19005 does not admit. Unlike an output intent's, this profile is what the \
     page's colours are given in: replacing it recolours everything drawn through the space, and \
     falling back on §8.6.5.5's alternate space puts a device space where a managed one was. \
     Both are doc/pdf-a-conversion-limits.md section 3's kind of loss, neither is built, and \
     nothing in the file supplies a corrected profile";

/// Why an `/OutputIntents` array that is already the wrong shape is not reconciled.
const OUTPUT_INTENT_ARRAY_NOT_TIDIED: &str = "this file's OutputIntents array is not the shape \
     ISO 19005 section 6.2.3 requires — a PDF/A entry naming no destination profile, or several \
     entries naming different profile objects, or a page's own array doing either. This \
     conversion writes a PDF/A output intent by appending one to the array it found \
     (doc/pdf-a-conversion-limits.md section 4.1), which is the right answer only where the \
     array was silent; reconciling one that already states intents — dropping a PDF/A entry that \
     names nothing, pointing every entry at one profile object — is the rewrite this needs";

/// Why a `/DestOutputProfileRef` is not removed.
const PROFILE_REFERENCE_NOT_REMOVED: &str = "an output intent here names its destination profile \
     through DestOutputProfileRef, which puts the profile outside the file. Where the same \
     intent also embeds a DestOutputProfile, removing the reference is mechanical and loses \
     nothing; where it does not, the profile is on somebody else's disk and \
     doc/pdf-a-conversion-limits.md section 2.3's refusal applies unless --output-intent-profile \
     supplies one. Neither branch is built";

/// Why a halftone is not removed.
///
/// The pair with [`TRANSFER_FUNCTION_NOT_REMOVED`], and they are deliberately two sentences:
/// `CLAUDE.md` records §10.6's halftones as inapplicable on the standard's own condition and
/// §10.5's transfer functions as emphatically not, so what removing each costs is different.
const HALFTONE_NOT_REMOVED: &str = "ISO 19005 admits halftones only of type 1 or 5, without a \
     HalftoneName, and forbids a halftone phase or origin key in a graphics state. Removing one \
     changes nothing this renderer draws — CLAUDE.md records §10.6's halftones as inapplicable \
     on the standard's own condition, because a halftone describes how a marking device renders \
     continuous tone — and it does change what a press does with the file, which is exactly what \
     an archived print master is kept for. doc/pdf-a-conversion-limits.md section 4.8 therefore \
     makes it an Ask, and neither the authorisation word nor the rewrite is built";

/// Why a transfer function is not removed.
const TRANSFER_FUNCTION_NOT_REMOVED: &str = "ISO 19005 forbids a TR entry in a graphics state \
     and admits TR2 only with the value Default. This is not a print-only key: CLAUDE.md records \
     that this project called transfer functions inapplicable and was wrong, because §10.5's \
     transfer functions decide what a screen shows and an inverting one is a photographic \
     negative. So removing one can change the rendered page, and \
     doc/pdf-a-conversion-limits.md section 4.8's default is to render the page both ways, show \
     whether anything changed, and remove it only with that shown. The comparison and the \
     rewrite are both owed; where the function is the identity the removal is silent and safe, \
     and even that is not built";

/// Why a JP2 wrapper box is not rewritten.
const JPEG2000_BOX_NOT_REWRITTEN: &str = "doc/pdf-a-conversion-limits.md section 4.10: this \
     field is in the JP2 wrapper rather than the codestream — the colour specification box's \
     method, and which specification is marked best available — so meeting the clause is byte \
     surgery on a hundred-odd bytes and touches no sample. **What the cost does not settle is \
     the value.** A method outside the three the part admits describes this image's colour in a \
     way the part does not read, so writing one of the three in its place states a colour space \
     the box did not; and marking exactly one specification as the best available, where the \
     file marks none, ranks two of the producer's own specifications against each other on \
     evidence the file does not carry. Dropping the others instead throws one of them away. \
     Every route is a choice rather than a restatement, which is why this is refused rather \
     than merely unwritten";

/// Why JPEG 2000 samples are not re-encoded.
const JPEG2000_SAMPLES_NOT_RE_ENCODED: &str = "doc/pdf-a-conversion-limits.md section 4.10: the \
     bit depth and the channel count are stated in the codestream's own SIZ marker, and the \
     enumerated CIEJab colour space is what the samples mean — so meeting these means decoding \
     and re-encoding the image, or relabelling what its numbers are. The universal fallback is \
     transcoding the samples to FlateDecode, which loses nothing visible at a large cost in size \
     and puts this tree's own JPEG 2000 decoder's output into the archive permanently. One \
     channel-count case is cheaper and is unbuilt too: a cdef box declaring the second channel \
     as opacity leaves one colour channel without a sample being touched";

/// Why an `ICCBased` space duplicating the output intent's profile is not collapsed.
const DUPLICATE_PROFILE_NOT_COLLAPSED: &str = "ISO 19005-4 forbids an ICCBased space, or a \
     Separation's alternate space, from carrying a CMYK destination profile identical to the one \
     the output intent or the blending space already supplies, and naming DeviceCMYK in its \
     place looks mechanical: the identical profile is already the file's, and ISO 19005-4 \
     section 6.2.4.3 licenses that device space through that very intent. **Two things stop it, \
     and the first is decisive.** The rule binds a space that is *used*, so the failure is \
     reported where the content stream selected it — a page, with no object — and the array to \
     rewrite sits in a resource dictionary no finding names; siting the rewrite would mean \
     walking the content streams a second time to decide which space was used, which is the \
     validator's reading made again in this crate. And even sited it would not be a restatement: \
     §8.6.7 applies non-zero overprint mode only where the current space is DeviceCMYK \
     or is implicitly converted to it, so the substitution can decide a composite that \
     §8.6.5.7 left open — which is the ambiguity section 6.2.4.2's own NOTE 2 names as the \
     reason for the prohibition";

/// Why overprint mode is not changed.
const OVERPRINT_MODE_NOT_CHANGED: &str = "this file sets overprint mode 1 while an ICCBased CMYK \
     space is in use and overprinting is on. doc/pdf-a-conversion-limits.md section 4.5 makes it \
     an Ask: the mode decides whether a zero component leaves the backdrop alone or paints it, \
     so changing it changes how overlapping CMYK marks composite. The key is in a graphics state \
     parameter dictionary rather than on a page, so no fence stands in the way; what is missing \
     is the authorisation word and the rewrite";

/// Why two `Separation` arrays of one name are not reconciled.
const SEPARATIONS_NOT_RECONCILED: &str = "two Separation arrays here name the same colourant and \
     define it differently. doc/pdf-a-conversion-limits.md section 4.5: making them agree means \
     choosing one definition and rewriting the others, and the two may genuinely render \
     differently — so the converter is to report the disagreement, show both, and rewrite only \
     when told which one wins. The report and the rewrite are both owed, and a document \
     assembled from several producers routinely lands here";

/// Why a `/Colorants` entry is not synthesised.
const COLORANTS_NOT_SYNTHESISED: &str = "every spot colour a DeviceN or NChannel space uses \
     needs an entry in that space's Colorants dictionary, which the base standard leaves \
     optional. doc/pdf-a-conversion-limits.md section 4.5 calls this a Default, synthesised from \
     the space's own alternate space and tint transform so that nothing is invented and no mark \
     changes — and the entry it would write is a Separation, whose tint transform §8.6.6.4 makes \
     a function of *one* input where the DeviceN's is a function of N. **That derivation is not \
     arithmetic, because §7.10 gives a PDF function no way to call another.** The general route \
     is to sample the producer's function along the one axis, which is an approximation of it \
     rather than a restatement, and an approximation written into an archive as though it were \
     the producer's definition is a loss wearing a mechanical's clothes. One shape could be \
     exact and is the thing to build first: a §7.10.2 sampled transform already states its \
     values on a grid, and the samples along one axis are the producer's own numbers rather \
     than a re-approximation of them";

/// Why a reference `XObject`'s `/Ref` is not dropped in favour of its proxy.
const REFERENCE_XOBJECT_NOT_PROXIED: &str = "doc/pdf-a-conversion-limits.md section 2.3's one \
     case with a real workaround: §8.10.4 makes a reference XObject's containing form serve as a \
     proxy — what a processor draws when the referenced content is not available, and what one \
     that does not implement Ref draws unconditionally. An archived file is exactly the case \
     where the target will not be available, so dropping the Ref entry and keeping the proxy \
     writes down what the file was going to show. It is an Ask rather than a mechanical rewrite, \
     because a reader that could have reached the imported content now sees the placeholder \
     instead, and neither half is built";

/// Why a malformed `/Lang` is neither corrected nor removed.
const LANGUAGE_IDENTIFIER_NOT_REMOVED: &str = "a Lang entry here is not a language identifier the \
     base standard defines. Correcting it means deciding what language the producer meant, which \
     doc/questions/A48 forbids; removing it drops the only statement the file makes about the \
     text's language, which §14.9.2 has a reader use and which PDF/A-2a exists partly to \
     require. Removal is doc/pdf-a-conversion-limits.md section 3's kind of answer and is not \
     offered";

/// Why a packet this converter cannot parse is not rebuilt.
const XMP_PACKET_NOT_REBUILT: &str = "this file's metadata packet does not parse, states more \
     than one rdf:RDF element, or breaks the XMP data model. This converter writes into the \
     producer's own packet by span — doc/pdf-a-conversion-limits.md section 4.2's identification \
     schema, and section 3.9's property removals — and a packet it cannot parse has no spans to \
     write into. Replacing it with one this program composes would throw away everything the \
     producer recorded, which is section 3's kind of loss; repairing it would be this converter \
     deciding what a malformed packet meant, which is doc/questions/A48's forbidden half. \
     Neither is built, and the first is the one a later slice can offer";

/// Why character data outside a simple value is left where it is.
///
/// The requirement arrived in session 958 and the census ratchet caught this the same day — a
/// requirement `pdf-archive` learns to check is a requirement this verb owes an answer to, which
/// is the ratchet's whole purpose.
const XMP_STRAY_CHARACTER_DATA_NOT_REMOVED: &str = "this packet writes non-white character data \
     somewhere the XMP standard's serialisation admits none — that standard allows white space \
     anywhere the RDF syntax does and confines everything else to the element content of a leaf \
     representing a simple value, so what is here expresses no XMP value at all. Cutting it is \
     the same span surgery doc/pdf-a-conversion-limits.md section 3.9 already describes for a \
     property, but the remover in pdf_model::xmp takes a property name and this text belongs to \
     no property — it needs a second entry point keyed on the span the reader already records. \
     Whether taking it out is mechanical or an authorised loss is a question this row does not \
     settle: it expresses nothing a reader of the packet can use, and it is still bytes somebody \
     wrote, which is section 3.9's shape exactly";

/// Why an extension schema container is not emitted for an undescribed schema.
const EXTENSION_CONTAINER_NOT_EMITTED: &str = "this packet uses a schema outside the predefined \
     ones and describes it nowhere. doc/pdf-a-conversion-limits.md section 4.2 permits emitting \
     an extension schema container for such a property and calls it authoring in a small way, \
     with an Ask where a value's type cannot be determined from what is there — that is the \
     rewrite this needs. Its sibling, a container the producer wrote and left a required field \
     out of, is refused separately and for a different reason: there the missing field is a \
     sentence only its producer holds";

/// Why a malformed amendment identifier is not removed.
const AMENDMENT_IDENTIFIER_NOT_REMOVED: &str = "the identification schema here states an \
     amendment or corrigendum identifier that is not the number and the year separated by a \
     colon. Neither half can be recovered from a malformed one, so correcting it is \
     doc/questions/A48's forbidden half; the entry is optional, so removing it is the available \
     remedy and it drops the producer's claim about which amendment the file was made to. This \
     converter writes the identification schema (doc/pdf-a-conversion-limits.md section 4.2) and \
     does not touch the amendment entry";

/// Why a configuration's `/Name` is not written.
///
/// The `/Order` half of section 3.8's pair is built — `Rewrite::OptionalContentOrder` — and this
/// is the half that is not, because the two are different acts. Completing an array with groups
/// the file already lists invents nothing; a name does not exist anywhere to be found.
const CONFIGURATION_NAME_NOT_WRITTEN: &str = "an optional content configuration here states no \
     Name, or states one another configuration already uses. doc/pdf-a-conversion-limits.md \
     section 3.8 calls a name synthesised uniquely within the file Default work, and this is \
     where that class meets doc/questions/A48's line: §8.11.4.3 makes Name a label for a user \
     interface, so a synthesised one is text no producer wrote, and the answer has to be argued \
     rather than assumed. The neighbouring rule of the same subclause — an Order array that does \
     not reference every group in the file — is answered, because the groups and their order are \
     both the file's own";

/// Why an automatic optional-content state is not removed.
const AUTOMATIC_STATES_NOT_REMOVED: &str = "ISO 19005-2 section 6.9 forbids an AS entry in an \
     optional content configuration; ISO 19005-4 section 6.10 permits it and has a conforming \
     processor ignore it instead. AS is what switches layers by zoom, by print-versus-view or by \
     user event, so removing it freezes the document into one state — \
     doc/pdf-a-conversion-limits.md section 3.8's Ask, and not built. PDF/A-4 is the shorter \
     route here, because it keeps the key and ignores it";

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
    input: &pdf_archive::Report,
    judgement: &Judgement,
    authorised: Authorisations,
    prepared: &Prepared,
) -> Decision {
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
    if let Answer::AsUnderlying(rules) = remedy.answer {
        return decide_as_underlying(input, rules, authorised, prepared);
    }
    answer_of(remedy.answer, authorised, prepared)
}

/// A compound requirement's decision, taken from the rules it names.
///
/// The answer is the **most constraining** of theirs, in the order a conversion is constrained: a
/// refusal stops the file, then an unauthorised loss, then one the caller authorised, then a
/// statement, then a rewrite that loses nothing. So a widget failing two rules, one of which this
/// converter refuses, is refused — which is what the conversion is going to do anyway, said in
/// the row a reader is looking at.
///
/// A rule the document did not fail contributes nothing, and where **none** of them failed the
/// compound is refused: its predicate found something at a population the rules did not reach,
/// and [`NOTHING_UNDERLYING_FAILED`] is what that is.
fn decide_as_underlying(
    input: &pdf_archive::Report,
    rules: &[&str],
    authorised: Authorisations,
    prepared: &Prepared,
) -> Decision {
    let mut best: Option<Decision> = None;
    for judgement in input.failures() {
        if !rules.contains(&judgement.id) {
            continue;
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
        // A compound naming a compound would need an order this table does not have; the rows
        // `Answer::AsUnderlying` names are ordinary ones, and one that were not is passed over
        // rather than followed.
        if matches!(remedy.answer, Answer::AsUnderlying(_)) {
            continue;
        }
        let decision = answer_of(remedy.answer, authorised, prepared);
        if best.is_none_or(|held| constraint(decision) > constraint(held)) {
            best = Some(decision);
        }
    }
    best.unwrap_or(Decision::Refused(Because::NotBuiltYet(
        NOTHING_UNDERLYING_FAILED,
    )))
}

/// How much one decision constrains the conversion, for [`decide_as_underlying`]'s ordering.
const fn constraint(decision: Decision) -> u8 {
    match decision {
        Decision::Mechanical(_) => 0,
        Decision::Stated { .. } => 1,
        Decision::Authorised { .. } => 2,
        Decision::Unauthorised { .. } => 3,
        Decision::Refused(_) => 4,
    }
}

/// One row's answer, once the caller's authorisations and the document's preparations are known.
fn answer_of(answer: Answer, authorised: Authorisations, prepared: &Prepared) -> Decision {
    match answer {
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
        // Resolved by `decide` before this function is reached, so that the routing is one place
        // rather than two. A row reaching here is one `decide_as_underlying` passed over.
        Answer::AsUnderlying(_) => Decision::Refused(Because::NotBuiltYet(NOT_BUILT_YET)),
    }
}
