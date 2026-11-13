//! The annotations and attachments family of the decision table: the rows whose requirement
//! identifier begins `annotations/`, `forms/`, `actions/`, `signatures/`, `embedded-files/` or
//! `alternate-presentations/`, and the sentences those rows quote.
//! `doc/pdfa-blocking-issues.md` section 6 holds them with the files and structure.
//!
//! Whichever of [`super::REMEDIES`], [`super::CONDITIONAL`], [`super::SHAPES`] and
//! [`super::REFUSED_BY_NAME`] this family has rows for reads them beside every other family's, and
//! the dispatch in [`super`] reads only those four tables, so where a row is written decides
//! nothing about its answer (`doc/adr/2634`).

use super::super::rewrite::Rewrite;
use super::{Answer, Because, Conditional, Loss, Losses, Remedy, Shape};

/// What a constructed appearance asserts, which is `doc/questions/A21`'s condition on allowing it.
///
/// `doc/adr/0927`: the sentence travels with the decision. What changes is not *whether* the
/// annotation is drawn — a conforming reader constructs one either way, §12.7.4.3 says so — but
/// that the construction is now fixed, in this program's version of it, and that §12.5.2 has a
/// reader ignore the appearance characteristics the annotation states in favour of these bytes.
const APPEARANCE_REINTERPRETS: &str = "an appearance stream this program constructed is now what \
     every reader draws for these annotations. What asks for the dictionary is the part's own \
     requirement — ISO 19005-2 section 6.3.3 at a part 2 target, where ISO 32000-1:2008's Table \
     164 leaves the entry optional, and ISO 32000-2's Table 166 writer obligation at a part 4 \
     one — and what goes inside it is each subtype's own clause, which both editions state alike, \
     so nothing here is invented — but the detail is this renderer's, and another reader \
     constructing from \
     the same entries would differ in line joins, in text metrics and in where a caption sits. \
     §12.5.2 then has a reader ignore C, IC, Border, BS and the rest in favour of the stream, so \
     an annotation whose look used to be recomputed from those entries is fixed as it is here. \
     The report names every appearance written, with the page it is on";

/// What clearing `/NeedAppearances` asserts, which is `doc/questions/A21`'s condition again.
///
/// `doc/adr/0927`: the sentence travels with the decision. §12.7.3's Table 224 makes the flag
/// "\[a] flag specifying whether to construct appearance streams and appearance dictionaries for
/// all widget annotations in the document", and its writer obligation makes an absent one a claim
/// that streams have been provided for every visible widget. So what changes is the same thing
/// `APPEARANCE_REINTERPRETS` names, at the widgets: the construction stops being every reader's
/// own and becomes this program's, frozen in the file.
const FIELD_APPEARANCE_REINTERPRETS: &str = "this file asked every reader to build its form \
     fields' appearances for itself. Instead, each visible widget the file provides no normal \
     appearance for is given one this program constructed by ISO 32000-2 \u{a7}12.7.4.3's \
     algorithm, from the widget's own MK characteristics and its field's value, and the flag \
     asking is removed. A widget whose AP already states an N keeps its producer's bytes \
     untouched. What the clause hands the processor \u{2014} where a baseline sits, how far \
     apart two lines are, what size auto-sizing picks \u{2014} is this renderer's choice, so a \
     field that every reader used to lay out slightly differently now looks the same everywhere \
     and looks the way this program draws it. The report names every appearance written, with \
     the page it is on";

/// What collapsing an appearance subdictionary asserts, which is `doc/adr/0948`'s condition.
const APPEARANCE_STATE_REINTERPRETS: &str = "an annotation's normal appearance is now in the \
     shape ISO 19005 asks of it, and draws the stream it drew. Where it was a subdictionary of \
     appearance states it is the single stream its own AS entry selected: §12.5.2's Table 166 \
     makes AS what selects the applicable stream, so the page draws exactly what it drew before \
     — and the other states go with the subdictionary, so an annotation a reader could have \
     switched (a check box, a trap network) is fixed in the state the file was saved in. Where \
     it was a push button's single stream it is the one entry of a subdictionary its new AS \
     entry selects, and the state's name decides nothing a reader draws. Where the appearance \
     dictionary stated no entry, or only entries whose value is null, it is gone: it described no \
     appearance, as an absent one does (ISO 32000-2 §7.3.7, Table 170). The report names every \
     one";

/// This family's rows of [`super::REMEDIES`].
pub(super) const REMEDIES: &[Remedy] = &[
    // ISO 19005-2 Annex B.1: a signature whose range does not cover the file it was made over.
    // Only the signer could redo the range and the digest as one act (ADR 1003), and no output
    // of this verb carries the signature anyway; what the row asked for was that the range the
    // source got wrong be *stated*, which the report's line per signature now does.
    Remedy {
        requirement: "signatures/digest-covers-the-whole-file",
        answer: Answer::Loses(Loss::SignatureAssertion, Rewrite::SignatureValueRemoved),
    },
    // ISO 19005-2 section 6.5.1 and ISO 19005-4 section 6.6.1: six rows naming action *types*,
    // answered by taking the action out of the tree it sits in and promoting §12.6.2's `/Next`
    // into its place. One loss for all nine rows below, because it is one thing a person is
    // being asked to give up — `doc/pdf-a-conversion-limits.md` section 3.3's *Ask*.
    Remedy {
        requirement: "actions/no-launch-multimedia-or-form-actions",
        answer: Answer::Loses(Loss::InteractiveBehaviour, Rewrite::ForbiddenActionRemoved),
    },
    Remedy {
        requirement: "actions/no-deprecated-set-state-or-no-op-actions",
        answer: Answer::Loses(Loss::InteractiveBehaviour, Rewrite::ForbiddenActionRemoved),
    },
    // ISO 19005-4 section 6.6.2 permits a JavaScript action outright, so this row binds part 2
    // alone and a document refused here for PDF/A-2 converts to PDF/A-4 untouched.
    Remedy {
        requirement: "actions/no-javascript-action",
        answer: Answer::Loses(Loss::InteractiveBehaviour, Rewrite::ForbiddenActionRemoved),
    },
    Remedy {
        requirement: "actions/no-optional-content-or-view-action",
        answer: Answer::Loses(Loss::InteractiveBehaviour, Rewrite::ForbiddenActionRemoved),
    },
    Remedy {
        requirement: "actions/optional-content-or-view-action-only-in-engineering-files",
        answer: Answer::Loses(Loss::InteractiveBehaviour, Rewrite::ForbiddenActionRemoved),
    },
    Remedy {
        requirement: "actions/named-action-is-page-navigation",
        answer: Answer::Loses(Loss::InteractiveBehaviour, Rewrite::ForbiddenActionRemoved),
    },
    // ISO 19005-2 section 6.5.2 forbids the dictionary in four places; ISO 19005-4 section 6.6.3
    // permits it on a widget and admits only Table 197's triggers elsewhere. Two rows, two sizes
    // of one act.
    Remedy {
        requirement: "actions/no-additional-actions-dictionary",
        answer: Answer::Loses(
            Loss::InteractiveBehaviour,
            Rewrite::AdditionalActionsRemoved,
        ),
    },
    Remedy {
        requirement: "actions/additional-actions-outside-widgets-hold-only-annotation-triggers",
        answer: Answer::Loses(
            Loss::InteractiveBehaviour,
            Rewrite::AdditionalActionsRemoved,
        ),
    },
    // ISO 19005-2 section 6.4.1 and ISO 19005-4 section 6.4.1: the entry itself, so the chain
    // behind it goes rather than the actions of a forbidden type in it.
    Remedy {
        requirement: "forms/no-action-on-widget-or-field",
        answer: Answer::Loses(
            Loss::InteractiveBehaviour,
            Rewrite::WidgetActionEntryRemoved,
        ),
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
    // removing the annotation, which this converter does not offer. A loss to authorise rather
    // than a statement, by the owner's word: a run with no profile states no policy, so it does
    // not decide what a document says about paper (`doc/questions/A448`, `doc/adr/1942`).
    Remedy {
        requirement: "annotations/flags-entry-present",
        answer: Answer::Loses(Loss::AnnotationPrinting, Rewrite::AnnotationFlags),
    },
    // ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2, second sentence: the annotations
    // whose producer *did* state flags and stated ones the parts forbid. The clause offers
    // nothing to put in their place, so `doc/pdf-a-conversion-limits.md` section 3.7's two
    // futures are the whole set, and it makes removal the default of the two — showing an
    // annotation somebody hid puts a mark on a page its producer kept off, which is a larger act
    // than taking the annotation out. `doc/adr/1234`.
    Remedy {
        requirement: "annotations/printable-and-visible",
        answer: Answer::Loses(Loss::HiddenAnnotation, Rewrite::HiddenAnnotationRemoved),
    },
    // ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3: an appearance dictionary holds
    // `/N` and nothing else. §12.5.5's Table 170 makes `/R` and `/D` optional and gives each the
    // default "the value of the N entry", so the removal leaves a reader drawing in those states
    // what the standard already has it draw for a dictionary that states neither — which is why
    // this is the cheapest authorised loss in the catalogue rather than a `Mechanical` one: the
    // artwork itself is gone from the file. `doc/adr/1234`.
    Remedy {
        requirement: "annotations/appearance-dictionary-holds-only-normal",
        answer: Answer::Loses(
            Loss::AppearanceStates,
            Rewrite::ExtraAppearanceStatesRemoved,
        ),
    },
    // ISO 19005-2 section 6.3.1 and ISO 19005-4 section 6.3.1, each clause's first sentence and
    // its struck names. The clause states a prohibition and no alternative, so the one rewrite
    // that meets it takes the annotation off the page — `doc/pdf-a-conversion-limits.md` section
    // 3.2's *Ask*, built as one (`doc/adr/1099`). **The flavour rows below are not these two**:
    // ISO 19005-4's later paragraphs confine `3D`, `RichMedia` and `FileAttachment` to a flavour
    // rather than forbidding them, so the first answer there is the flavour that admits the
    // subtype and the `FileAttachment` case keeps the file it names; both stay refused by name.
    Remedy {
        requirement: "annotations/subtype-defined-in-iso-32000-1",
        answer: Answer::Loses(
            Loss::ForbiddenAnnotation,
            Rewrite::ForbiddenAnnotationRemoved,
        ),
    },
    Remedy {
        requirement: "annotations/subtype-defined-in-iso-32000-2",
        answer: Answer::Loses(
            Loss::ForbiddenAnnotation,
            Rewrite::ForbiddenAnnotationRemoved,
        ),
    },
    // ISO 19005-4 section 6.3.1 confines `3D` and `RichMedia` to PDF/A-4e: at PDF/A-4 and
    // PDF/A-4f the subtype is not admitted, which is the loss above exactly — the artwork goes
    // with the annotation, and the report's sentence names PDF/A-4e as the target that keeps it
    // (`doc/adr/2051` section 4).
    Remedy {
        requirement: "annotations/three-dimensional-only-in-engineering-files",
        answer: Answer::Loses(
            Loss::ForbiddenAnnotation,
            Rewrite::ForbiddenAnnotationRemoved,
        ),
    },
    // The same subclause confines `FileAttachment` to PDF/A-4f, and at PDF/A-4 and PDF/A-4e the
    // annotation goes in the same removal — but the file its `/FS` names is filed in the document's
    // `/EmbeddedFiles` tree, which every part 4 flavour admits, so what is lost is the marker on
    // the page and not the attachment (`doc/adr/2067` section 4).
    Remedy {
        requirement: "annotations/file-attachment-only-in-embedded-file-files",
        answer: Answer::Loses(
            Loss::ForbiddenAnnotation,
            Rewrite::ForbiddenAnnotationRemoved,
        ),
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
    // ISO 19005-2 section 6.3.3, ISO 19005-4 section 6.3.3 — the half an `/AS` entry answers.
    Remedy {
        requirement: "annotations/normal-appearance-shape",
        answer: Answer::Stated(
            None,
            Rewrite::NormalAppearanceFromState,
            APPEARANCE_STATE_REINTERPRETS,
        ),
    },
    // ISO 19005-2 section 6.4.2, ISO 19005-4 section 6.4.2 — the second sentence of the pair.
    // The entry is deprecated, its subject is the XFA form, and §7.7.2's Table 29 gives it a
    // default of `false` — so removing it states what an absent entry states. What it is *not* is
    // a statement about the form beside it: the `/XFA` row's own answer removes the resource, and
    // `doc/adr/1257` reads this same entry as the document's claim that its pages are generated.
    Remedy {
        requirement: "forms/no-needs-rendering",
        answer: Answer::Mechanical(Rewrite::NeedsRendering),
    },
    // ISO 19005-2 section 6.4.1, ISO 19005-4 section 6.4.1. §12.7.3's Table 224 makes clearing
    // the flag a claim about the file — that appearance streams have been provided for all
    // visible widget annotations — so the construction beside it is what makes the claim true.
    // With `construct = false` the operator asks for the flag alone, and `answer_of` routes that
    // to `Loss::FieldAppearances`. `doc/adr/1257`.
    Remedy {
        requirement: "forms/need-appearances-absent-or-false",
        answer: Answer::Stated(
            None,
            Rewrite::NeedAppearancesCleared,
            FIELD_APPEARANCE_REINTERPRETS,
        ),
    },
    // ISO 19005-2 section 6.4.2, ISO 19005-4 section 6.4.2, first sentence. ISO 32000-2 Annex K
    // requires a file stating the key to keep its interactive form dictionary consistent with the
    // resource, so for a static form the AcroForm is the form and what goes is the behaviour the
    // template carried. `doc/adr/1257` is what refuses a dynamic one.
    Remedy {
        requirement: "forms/no-xfa-key",
        answer: Answer::Loses(Loss::XfaForm, Rewrite::XfaRemoved),
    },
    // ISO 19005-2 section 6.8, ISO 19005-4 section 6.9: an embedded file that is not itself a
    // conforming PDF/A, with nowhere in either clause to put it. The last of `doc/rfc/0007`
    // section 4.6's four answers, and the three above it are the configuration's — `derive` with
    // a declared tool, the owner's XML departure, and a 4f or 4e target where the rule does not
    // bind at all. `doc/adr/1258`.
    Remedy {
        requirement: "embedded-files/embedded-file-is-itself-pdfa",
        answer: Answer::Loses(Loss::EmbeddedFile, Rewrite::EmbeddedFileRemoved),
    },
    Remedy {
        requirement: "embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile",
        answer: Answer::Loses(Loss::EmbeddedFile, Rewrite::EmbeddedFileRemoved),
    },
    // ISO 19005-2 section 6.4.3, ISO 19005-4 section 6.5.1: the annotation rules, asked again of
    // a signature field's widget. Nothing of its own to do — the three rows it names are what
    // answer a widget as they answer any other annotation.
    Remedy {
        requirement: "signatures/signature-widgets-meet-the-annotation-rules",
        answer: Answer::AsUnderlying(ANNOTATION_RULES),
    },
];

/// This family's rows of [`super::CONDITIONAL`].
pub(super) const CONDITIONAL: &[(&str, Conditional)] = &[
    (
        "forms/need-appearances-absent-or-false",
        Conditional::PerConfiguration(Losses::one(Loss::FieldAppearances)),
    ),
    // A construction is a statement where the clauses state the appearance, and the loss
    // `stand-in-face` where its text is set in ADR 0112's stand-in for a `/DA` font `/DR` does not
    // define: which is a fact about the file (`doc/adr/2858`).
    (
        "annotations/appearance-dictionary-present",
        Conditional::PerDocument(Losses::one(Loss::StandInFace)),
    ),
    (
        "annotations/appearance-dictionary-present-from-base-standard",
        Conditional::PerDocument(Losses::one(Loss::StandInFace)),
    ),
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

/// This family's rows of [`super::SHAPES`].
pub(super) const SHAPES: &[(&str, &[Shape])] = &[
    (
        "annotations/printable-and-visible",
        &[
            Shape {
                name: "hidden-at-rest",
                answered: true,
            },
            Shape {
                name: "visible-nonprinting",
                answered: true,
            },
        ],
    ),
    // ISO 19005-2 section 6.3.3, ISO 19005-4 section 6.3.3. §12.5.2's Table 166 makes `/AS` what
    // selects the applicable stream, so an appearance subdictionary with one collapses to the
    // stream the file was saved showing; with none, nothing in the file says which state the
    // producer meant. A push button's single stream becomes a subdictionary holding it alone
    // (`doc/adr/2319`).
    (
        "annotations/normal-appearance-shape",
        &[
            Shape {
                name: "appearance-state",
                answered: true,
            },
            Shape {
                name: "push-button-stream",
                answered: true,
            },
            Shape {
                name: "no-appearance-state",
                answered: false,
            },
        ],
    ),
    // ISO 19005-2 section 6.4.2, ISO 19005-4 section 6.4.2.
    (
        "forms/no-xfa-key",
        &[
            Shape {
                name: "static",
                answered: false,
            },
            Shape {
                name: "dynamic",
                answered: false,
            },
        ],
    ),
];

/// This family's rows of [`super::REFUSED_BY_NAME`].
pub(super) const REFUSED_BY_NAME: &[(&str, Because)] = &[
    // ISO 19005-4 Annex A.2 requires the key to be *present*, which is the one requirement in
    // either part that a document can fail by holding nothing at all.
    (
        "embedded-files/pdfa-4f-carries-embedded-files",
        Because::NotThisTarget(FOUR_F_NEEDS_AN_EMBEDDED_FILE),
    ),
    // ISO 19005-2 section 6.8 and ISO 19005-4 section 6.9's first sentence: the embedded file
    // shall itself conform. Converting it means running this whole verb over the file inside the
    // file, which is a slice of its own and is named as one rather than half-done.
    // ISO 19005-4 section 6.9 wants a MIME media type, and nothing in a file specification says
    // what one is: a file name's extension is a convention rather than a statement.
    (
        "embedded-files/associated-file-media-type",
        Because::NotBuiltYet(NO_MEDIA_TYPE_TO_DERIVE),
    ),
    // ISO 19005-4 Annex B.2.2, the one requirement of the engineering annex that binds a file
    // rather than a processor and that a document can fail on its content.
    (
        "annotations/three-dimensional-stream-format",
        Because::NotThisTarget(THREE_DIMENSIONAL_FORMAT),
    ),
];

/// Why a document with no embedded file cannot be made PDF/A-4f.
const FOUR_F_NEEDS_AN_EMBEDDED_FILE: &str = "ISO 19005-4 Annex A.2 makes an EmbeddedFiles key \
     required of a PDF/A-4f file, and this document holds no embedded file. Attaching one would \
     be adding content no source states, which is not converting. Ask for PDF/A-4 instead: the \
     plain profile is what a document with nothing embedded in it is for";

/// Why an embedded file's `/Subtype` media type is not supplied.
const NO_MEDIA_TYPE_TO_DERIVE: &str = "ISO 19005-4 section 6.9 asks the embedded file stream for \
     a Subtype that is a MIME media type, and nothing in a file specification states one: a file \
     name's extension is a convention rather than a declaration, and reading it as one would be \
     this converter asserting what the bytes are";

/// Why a 3D stream in a format Annex B does not name is not converted.
const THREE_DIMENSIONAL_FORMAT: &str = "ISO 19005-4 Annex B.2.2 admits a 3D stream whose Subtype \
     is U3D or PRC, and this document's is neither. Translating 3D artwork between formats is a \
     media engine rather than a rendering question, and CLAUDE.md excludes ISO 32000-2 clause 13 \
     by name. Removing the annotation would reach plain PDF/A-4 at the cost of the artwork, and \
     this converter does not yet offer that loss";
