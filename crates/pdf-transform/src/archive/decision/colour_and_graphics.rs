//! The colour and graphics family of the decision table: the rows whose requirement identifier
//! begins `graphics/`, the sentences those rows quote, and the answers for device colour that
//! read what the preparation found. `doc/pdfa-blocking-issues.md` section 4 is the same family.
//!
//! Whichever of [`super::REMEDIES`], [`super::CONDITIONAL`], [`super::SHAPES`] and
//! [`super::REFUSED_BY_NAME`] this family has rows for reads them beside every other family's, and
//! the dispatch in [`super`] reads only those four tables, so where a row is written decides
//! nothing about its answer (`doc/adr/2634`).

use pdf_archive::survey::DeviceFamily;

use super::super::prepare::{Intent, Prepared, ProfileSource};
use super::super::rewrite::Rewrite;
use super::{
    Answer, Authorisations, Because, Conditional, Decision, Loss, Losses, Remedy, Shape, answer_of,
    losing,
};

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

/// What adding an output intent asserts where its destination profile is the CMYK press this
/// program ships, which the file did not already state (`doc/adr/2691`).
///
/// [`OUTPUT_INTENT_REINTERPRETS`]'s second sentence is true of an RGB or a grey profile, which
/// draw a device colour where this renderer drew it to within a level of 255, and false of a
/// CMYK one: ISO 32000-2 §8.6.4.1 has device colours "map directly (or by simple conversions)"
/// to a device's colourants, and with no profile in the file this renderer drew `DeviceCMYK`
/// through its own assumed process inks (`pdf_colour::assumed_press`, ADR 0263), which no ICC
/// profile states. So the output draws those colours through a different press than the source
/// did, and the sentence that is the condition of the rewrite (`doc/adr/0927`) says so.
const OUTPUT_INTENT_STATES_THE_SHIPPED_PRESS: &str = "a PDF/A output intent is what a \
     conforming reader colour-manages device colours through, so every DeviceGray, DeviceRGB and \
     DeviceCMYK value in this file now means what the destination profile says it means, and the \
     same profile becomes the default blending space for transparency. That profile is the CMYK \
     press this program ships, and this file named no press: this renderer drew its DeviceCMYK \
     colours through its own assumed process inks, which no ICC profile states, so they now draw \
     differently here as well as in other readers, most where the press's inks lie outside what \
     a screen shows. For content separated for a particular press, supply that press's profile \
     with --output-intent-profile instead";

/// [`OUTPUT_INTENT_STATES_THE_SHIPPED_PRESS`] where the CMYK press is the one the operator's
/// configuration names (`doc/adr/2705`): the same move of the drawing, and the press is theirs,
/// named once for every document whose press nobody stated rather than for this one.
const OUTPUT_INTENT_STATES_THE_CONFIGURED_PRESS: &str = "a PDF/A output intent is what a \
     conforming reader colour-manages device colours through, so every DeviceGray, DeviceRGB and \
     DeviceCMYK value in this file now means what the destination profile says it means, and the \
     same profile becomes the default blending space for transparency. That profile is the CMYK \
     press your configuration names, and this file named no press: this renderer drew its \
     DeviceCMYK colours through its own assumed process inks, which no ICC profile states, so \
     they now draw differently here as well as in other readers, most where the press's inks lie \
     outside what a screen shows. For content separated for another press, supply that press's \
     profile with --output-intent-profile instead";

/// [`OUTPUT_INTENT_STATES_THE_SHIPPED_PRESS`] where the CMYK profile is the operator's
/// (`doc/adr/2691`): the same move of the drawing, and the press is a statement the operator made.
const OUTPUT_INTENT_STATES_THE_SUPPLIED_PRESS: &str = "a PDF/A output intent is what a \
     conforming reader colour-manages device colours through, so every DeviceGray, DeviceRGB and \
     DeviceCMYK value in this file now means what the destination profile says it means, and the \
     same profile becomes the default blending space for transparency. That profile is the CMYK \
     one you supplied with --output-intent-profile, your statement of the press this document \
     was made for; this file named none, and this renderer drew its DeviceCMYK colours through \
     its own assumed process inks, which no ICC profile states, so they now draw differently \
     here as well as in other readers";

/// What adding an output intent asserts where its destination profile is the one this program
/// writes of its own assumed process inks (`doc/adr/2700`, the owner's answer A780).
///
/// [`OUTPUT_INTENT_REINTERPRETS`]'s claim that this renderer already showed the colours that way
/// is true here: the profile is a sampling of the ink cube the source's `DeviceCMYK` was drawn
/// through, fine enough that the tests of `pdf-colour`'s `inks_profile` hold every fill and every
/// press sample within one level of 255.
///
/// What the claim does not cover is a grey inside a soft mask (`doc/adr/2708`). A `/Luminosity`
/// group stating `/CS /DeviceCMYK` composites in the profile's four components once the profile
/// is the intent, and a `DeviceGray` colour painted into it is separated into them by the
/// profile's own `B2A` (ADR 0796) where the source took ISO 32000-2 §10.4.2.3's black alone; the
/// other direction, an isolated `/DeviceCMYK` group drawn into a `/DeviceGray` mask group, leaves
/// the profile colorimetrically where the source took §10.4.2.3's grey. §11.5.3's device branch, which this reader takes in every space (ADR 2552), weighs the two
/// separations differently, and in the six documents measured the masks moved by up to 25
/// levels while the pages' own blending moved by at most one. ISO 19005-2 and ISO 19005-4
/// section 6.2.4.3 have a conforming reader convert a grey under a CMYK intent by the classic
/// method, which would draw those masks as the source did; whether this reader does is `Q788`. The sentence
/// also says what the profile is not, because its description does: a characterisation of a
/// renderer, not of a press.
const OUTPUT_INTENT_STATES_THE_ASSUMED_INKS: &str = "a PDF/A output intent is what a conforming \
     reader colour-manages device colours through, so every DeviceGray, DeviceRGB and DeviceCMYK \
     value in this file now means what the destination profile says it means, and the same \
     profile becomes the default blending space for transparency. This file named no press, and \
     that profile states the process inks this renderer assumed for its DeviceCMYK colours, so \
     this renderer already showed those colours that way, and another reader following the \
     profile shows them the same way. Where a soft mask mixes grey and DeviceCMYK \
     content, this renderer now converts between them through the profile's inks rather than \
     through black alone, so such a mask can draw differently here. The profile characterises this \
     renderer and not a physical press, so for content separated for a particular press, \
     supply that press's profile with --output-intent-profile instead";

/// The sentence a stated output intent carries: `row`'s own, unless the intent is a CMYK press
/// this conversion adds. Where that press is a real one this renderer did not draw the source
/// through, the drawing moves with it (`doc/adr/2691`); where it is the assumed inks it does not
/// (`doc/adr/2700`).
///
/// A profile the file already held is the one this renderer drew the source through, so the row's
/// sentence stays true of it whatever its family.
pub(super) fn output_intent_sentence(intent: &Intent, row: &'static str) -> &'static str {
    if row != OUTPUT_INTENT_REINTERPRETS || intent.family != DeviceFamily::Cmyk {
        return row;
    }
    match intent.reported.source {
        ProfileSource::Shipped if intent.assumed_inks => OUTPUT_INTENT_STATES_THE_ASSUMED_INKS,
        ProfileSource::Shipped => OUTPUT_INTENT_STATES_THE_SHIPPED_PRESS,
        ProfileSource::Supplied => OUTPUT_INTENT_STATES_THE_SUPPLIED_PRESS,
        ProfileSource::Configured => OUTPUT_INTENT_STATES_THE_CONFIGURED_PRESS,
        ProfileSource::AlreadyInTheFile => row,
    }
}

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

/// What restating a rendering intent asserts, which is `doc/adr/0948`'s condition on it.
///
/// Nothing is lost and something is all the same different: the file used to state a name whose
/// meaning §8.6.5.8 settles for a *processor*, and now states the name itself. A reader that had
/// recognised the producer's private intent — the one case the standard does not govern — sees
/// the substitute instead.
const RENDERING_INTENT_REINTERPRETS: &str = "a rendering intent this file states is not one of \
     the four the base standard defines, and the entry now names RelativeColorimetric — which is \
     the intent that subclause has every conforming processor use for a name it does not \
     recognise. The requirement binds a PDF/A-2 target alone, so the base standard it has to \
     hold under is ISO 32000-1:2008, whose 8.6.5.8 states the same rule of a conforming reader \
     as ISO 32000-2's does of a processor. \
     No colour value changes and no mark moves; what changes is that a reader which happened to \
     know the producer's own name for an intent is no longer told it. The report names every \
     entry restated";

/// What removing an identity transfer function asserts (`doc/adr/1952` section 3).
const IDENTITY_TRANSFER_REINTERPRETS: &str = "a graphics state in this file states TR \
     Identity, which ISO 19005 forbids, and no graphics state in it states any other transfer \
     function, so the entry is removed. ISO 32000-2 8.4.1's Table 52 leaves the initial transfer \
     function device dependent; on this reader's device it is the identity, so every colour is \
     drawn where it was. On a device whose own default is not the identity, the producer's \
     reset to the identity no longer happens. The report names every graphics state rewritten";

/// What restating a blend mode array asserts, which is `doc/adr/0948`'s condition on it.
const BLEND_MODE_REINTERPRETS: &str = "a BM entry in this file is an array naming no blend mode \
     ISO 32000-2 defines, and it now names Normal — which is what §8.4.1's Table 57 has every \
     reader take from such an array. Nothing composites differently for any conforming reader; \
     what changes is that a reader which recognised one of the producer's own names would have \
     used it, and now uses Normal like everybody else";

/// The one requirement identifier that will not fit beside its key inside 100 columns.
const CMYK_UNDER_PART_FOUR: &str =
    "graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent";

/// This family's rows of [`super::REMEDIES`].
pub(super) const REMEDIES: &[Remedy] = &[
    // ISO 19005-2 section 6.2.8.1, ISO 19005-4 section 6.2.7.1 — the one row of section 4.7's table
    // marked *not quite mechanical*, and therefore the one that has to be authorised.
    Remedy {
        requirement: "graphics/image-interpolation-is-off",
        answer: Answer::Loses(Loss::ImageSmoothing, Rewrite::InterpolationOff),
    },
    // ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3. The two colour-box rows, answered
    // by removing specifications rather than by writing one: the sentence after the method's says
    // a conforming processor shall use only the selected colour space and shall ignore all the
    // others, so what goes is what the part itself directs a processor to ignore. `super::super::jpeg2000`
    // has the whole reading, including which box the standards' own sentences select and which
    // shapes stay refused because nothing selects one.
    Remedy {
        requirement: "graphics/jpeg2000-one-best-colour-space-specification",
        answer: Answer::Loses(
            Loss::Jpeg2000ColourFallback,
            Rewrite::Jpeg2000ColourSpecifications,
        ),
    },
    Remedy {
        requirement: "graphics/jpeg2000-colour-specification-method",
        answer: Answer::Loses(
            Loss::Jpeg2000ColourFallback,
            Rewrite::Jpeg2000ColourSpecifications,
        ),
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
    // ISO 19005-2 section 6.2.3, ISO 19005-4 section 6.2.3: the entries of one `OutputIntents`
    // array name one profile object. Mechanical because the preparation performs it only where
    // the objects carry the same profile bytes under the same stream dictionary — an array whose
    // entries name genuinely different destinations keeps its refusal, in `super::super::sites`.
    Remedy {
        requirement: "graphics/one-destination-profile-per-output-intents-array",
        answer: Answer::Mechanical(Rewrite::SharedDestinationProfile),
    },
    // ISO 19005-2 section 6.2.4.4, ISO 19005-4 section 6.2.4.4: a spot colourant's own entry in
    // the `/Colorants` dictionary. Mechanical because the entry written is either the
    // `Separation` array this file already states for that colourant, which the same subclause
    // requires every Separation of the name to agree with, or the `DeviceN` space's own
    // transform restricted exactly to that colourant alone (`super::super::spot_separation`,
    // `doc/adr/2690`); a transform that could only be resampled is refused there by name.
    Remedy {
        requirement: "graphics/spot-colourants-appear-in-the-colorants-dictionary",
        answer: Answer::Mechanical(Rewrite::SpotColorantEntry),
    },
    // ISO 19005-2 section 6.2.4.3, ISO 19005-4 section 6.2.4.3, and the two transparency
    // subclauses that turn on the same sentence — ISO 19005-2 section 6.2.10 and ISO 19005-4
    // section 6.2.9. Every one of them licenses a device colour space through a PDF/A output
    // intent, so one intent answers all of them at once; which of them it actually answers turns
    // on the destination profile's own colour family, which is what `Answer::Stated` carries.
    // `DeviceRGB`'s rows admit a device independent `/DefaultRGB` as well, which is the licence
    // left where the profile in hand is of another family (`doc/adr/2114`).
    Remedy {
        requirement: "graphics/device-rgb-needs-a-default-or-an-rgb-output-intent",
        answer: Answer::RgbUnderPartTwoOrFour(Some(DeviceFamily::Rgb)),
    },
    Remedy {
        requirement: "graphics/device-rgb-needs-a-default-a-blending-space-or-an-rgb-output-intent",
        answer: Answer::RgbUnderPartTwoOrFour(Some(DeviceFamily::Rgb)),
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
        answer: Answer::CmykUnderPartFour,
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
    // PDF/A output intent, so a document-level one answers it outright. Page-level intents are
    // therefore a facility this converter declines rather than one it has no occasion for, and
    // `doc/adr/1153` section 4 is the argument: taking them means writing no document-level
    // intent at all, an `OutputIntents` array on *every* device-dependent page, and a refusal
    // still left for any single page that draws in both RGB and CMYK.
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
    // case `doc/adr/0947`'s fourth decision built the net for. Each sends a space to section
    // 6.2.4.3, so each takes the `/DefaultRGB` where the document draws in `DeviceRGB` beside a
    // profile of another family: §11.6.6's Table 145 remaps a group's device space through the
    // defaults of the current resource dictionary (`doc/adr/2114`).
    Remedy {
        requirement: "graphics/transparency-group-colour-spaces-obey-the-colour-rules",
        answer: Answer::RgbUnderPartTwoOrFour(None),
    },
    Remedy {
        requirement: "graphics/transparency-group-colour-spaces-obey-the-colour-rules-of-part-four",
        answer: Answer::RgbUnderPartTwoOrFour(None),
    },
    Remedy {
        requirement: "graphics/indexed-and-pattern-base-spaces-obey-the-colour-rules",
        answer: Answer::RgbUnderPartTwoOrFour(None),
    },
    Remedy {
        requirement: "graphics/indexed-and-pattern-base-spaces-obey-the-colour-rules-of-part-four",
        answer: Answer::RgbUnderPartTwoOrFour(None),
    },
    Remedy {
        requirement: "graphics/separation-alternate-spaces-obey-the-colour-rules",
        answer: Answer::RgbUnderPartTwoOrFour(None),
    },
    Remedy {
        requirement: "graphics/separation-alternate-spaces-obey-the-colour-rules-of-part-four",
        answer: Answer::RgbUnderPartTwoOrFour(None),
    },
    // -----------------------------------------------------------------------------------------
    // `doc/pdf-a-mitigations.md` section 13.3, *owed, not optional*: the refusals whose right
    // answer loses nothing and which were waiting on code rather than on a decision. Each row's
    // clause is beside it; `super::super::sites` is where each works out which objects it reaches, and
    // is also what refuses a document failing in the shape the standard leaves unanswered.
    // -----------------------------------------------------------------------------------------

    // ISO 19005-2 section 6.2.3, ISO 19005-4 section 6.2.3: a destination profile of a class or a
    // colour space neither part admits, where this reader does not draw through it and the slot is
    // RGB; `super::super::prepare::ReplacedDestination` names each other case's refusal (`doc/adr/2007`).
    Remedy {
        requirement: "graphics/destination-profile-class-and-colour-space",
        answer: Answer::Stated(
            None,
            Rewrite::DestinationProfileReplaced,
            DESTINATION_REPLACED_REINTERPRETS,
        ),
    },
    // ISO 19005-2 section 6.2.3, ISO 19005-4 section 6.2.3: the tag and identifier rows, where
    // the destination profile's header alone fails them — a version 5 header on a profile written
    // in ICC.1:1998-09's tags and types, or a stale Profile ID. One rewrite answers both rows, and
    // `super::super::destination` names each other case's refusal (`doc/adr/2021`).
    // The same subclauses' first sentence, where a PDF/A entry identified as `Custom` states no
    // profile at all: it is given the one this conversion's own entry would name
    // (`super::super::prepare::SuppliedDestination`, `doc/adr/2035`).
    Remedy {
        requirement: "graphics/pdfa-output-intent-states-a-destination-profile",
        answer: Answer::Stated(
            None,
            Rewrite::DestinationProfileSupplied,
            DESTINATION_SUPPLIED_REINTERPRETS,
        ),
    },
    Remedy {
        requirement: "graphics/destination-profile-carries-the-tags-its-class-requires",
        answer: Answer::Stated(
            None,
            Rewrite::DestinationProfileRestated,
            DESTINATION_RESTATED_REINTERPRETS,
        ),
    },
    Remedy {
        requirement: "graphics/destination-profile-states-a-correct-profile-id",
        answer: Answer::Stated(
            None,
            Rewrite::DestinationProfileRestated,
            DESTINATION_RESTATED_REINTERPRETS,
        ),
    },
    // ISO 19005-2 section 6.2.5, ISO 19005-4 section 6.2.5. Stated where every `/TR` is the
    // identity (`doc/adr/1952` section 3), and a loss where one is not (`doc/adr/1964`):
    // `answer_of` routes on what `super::super::transfer` found, as `CONDITIONAL` says.
    Remedy {
        requirement: "graphics/no-transfer-function-in-a-graphics-state",
        answer: Answer::Stated(
            None,
            Rewrite::IdentityTransferFunctionRemoved,
            IDENTITY_TRANSFER_REINTERPRETS,
        ),
    },
    // The `/TR2` row beside it, the same removal's two answers over Table 57's `Default`
    // (`doc/adr/1978` section 6).
    Remedy {
        requirement: "graphics/second-transfer-function-is-default",
        answer: Answer::Stated(
            None,
            Rewrite::IdentityTransferFunctionRemoved,
            SECOND_IDENTITY_REINTERPRETS,
        ),
    },
    // A halftone's `TransferFunction` where §10.6.5.6 requires none: the same removal's two
    // answers, its `Identity` free wherever it stands (`doc/adr/2007`).
    Remedy {
        requirement: "graphics/halftone-transfer-function-only-where-required",
        answer: Answer::Stated(
            None,
            Rewrite::IdentityTransferFunctionRemoved,
            HALFTONE_IDENTITY_REINTERPRETS,
        ),
    },
    // ISO 19005-4 section 6.2.3 in every output intent, ISO 19005-2 section 6.2.3 in a PDF/X one:
    // `DestOutputProfileRef` goes where the same intent embeds its profile, and
    // `super::super::sites::profile_references` names each other shape's refusal (`doc/adr/2083`).
    Remedy {
        requirement: "graphics/no-destination-profile-reference",
        answer: Answer::Stated(
            None,
            Rewrite::DestinationProfileReferenceRemoved,
            PROFILE_REFERENCE_REINTERPRETS,
        ),
    },
    Remedy {
        requirement: "graphics/no-destination-profile-reference-in-a-pdfx-output-intent",
        answer: Answer::Stated(
            None,
            Rewrite::DestinationProfileReferenceRemoved,
            PROFILE_REFERENCE_REINTERPRETS,
        ),
    },
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
    // ISO 19005-2 section 6.2.5, ISO 19005-4 section 6.2.5: a halftone's device pointer, whose
    // removal moves nothing on a screen and changes what a press does (`doc/adr/1965`).
    Remedy {
        requirement: "graphics/no-halftone-name",
        answer: Answer::Loses(Loss::HalftoneName, Rewrite::HalftoneNameRemoved),
    },
    // The same subclauses' halftone type and part 4's origin: a press's screen, replaced by its
    // own or placed at its own origin, and nothing on a screen moved (`doc/adr/2034`).
    Remedy {
        requirement: "graphics/halftone-type-is-one-or-five",
        answer: Answer::Loses(Loss::Halftone, Rewrite::HalftoneWrittenDefault),
    },
    Remedy {
        requirement: "graphics/no-halftone-origin-in-a-graphics-state",
        answer: Answer::Loses(Loss::Halftone, Rewrite::HalftoneOriginRemoved),
    },
    // Part 2's halftone phase, an entry neither ISO 32000-1 nor ISO 32000-2 defines, so that no
    // processor of the base standard reads it (`doc/adr/2035`).
    Remedy {
        requirement: "graphics/no-halftone-phase-in-a-graphics-state",
        answer: Answer::Stated(
            None,
            Rewrite::HalftonePhaseRemoved,
            HALFTONE_PHASE_REINTERPRETS,
        ),
    },
    // ISO 19005-2 section 6.2.4.2, ISO 19005-4 section 6.2.4.2. Stated where the states setting
    // the mode govern no `DeviceCMYK` mark, and a loss where one does (`doc/adr/1978`):
    // `answer_of` routes on what `super::super::overprint` found, as `CONDITIONAL` says.
    Remedy {
        requirement: "graphics/no-overprint-mode-one-under-icc-cmyk",
        answer: Answer::Stated(
            None,
            Rewrite::OverprintModeRestated,
            OVERPRINT_MODE_REINTERPRETS,
        ),
    },
    // ISO 19005-2 section 6.2.4.2's tag row, where every short profile is one this reader does
    // not draw through: §8.6.5.5's Table 65 alternate is what the page showed. A profile it does
    // draw through and that lacks only a text tag is the operator's `supply`, and one lacking a
    // transform tag is refused; `super::super::icc` says which (`doc/adr/1979`, `doc/adr/1990`).
    Remedy {
        requirement: "graphics/icc-profiles-carry-the-tags-a-permitted-edition-requires",
        answer: Answer::Stated(
            None,
            Rewrite::ProfileAlternateWritten,
            PROFILE_ALTERNATE_REINTERPRETS,
        ),
    },
    // ISO 19005-4 section 6.2.4.2's tag row, which holds a profile to the edition its own header
    // names: the same three answers, read by the part's own text (`doc/adr/2006`).
    Remedy {
        requirement: "graphics/icc-profiles-carry-the-tags-their-version-requires",
        answer: Answer::Stated(
            None,
            Rewrite::ProfileAlternateWritten,
            PROFILE_ALTERNATE_REINTERPRETS,
        ),
    },
    // ISO 19005-4 section 6.2.4.2's first sentence through ISO 32000-2 §8.6.5.5's Table 66: an
    // `N` that is missing or is not the profile's own count states the profile's, which this
    // reader drew with; `super::super::component_counts` refuses a profile it does not draw through or
    // whose header the clause does not admit (`doc/adr/2099`).
    Remedy {
        requirement: "graphics/icc-profiles-conform-to-the-base-standard",
        answer: Answer::Stated(
            None,
            Rewrite::IccComponentCountRestated,
            COMPONENT_COUNT_REINTERPRETS,
        ),
    },
    // ISO 19005-4 section 6.2.4.2's last requirement, and section 6.2.4.4 sending it to an
    // alternate space: an `ICCBased` array whose profile is the one in force at every use is
    // written `DeviceCMYK`, and `super::super::duplicate_profiles` names each refusal (`doc/adr/2098`).
    Remedy {
        requirement: "graphics/no-icc-space-duplicating-the-output-intent-profile",
        answer: Answer::Stated(
            None,
            Rewrite::DuplicateProfileWrittenDeviceCmyk,
            DUPLICATE_PROFILE_REINTERPRETS,
        ),
    },
    Remedy {
        requirement: "graphics/separation-alternate-space-does-not-duplicate-a-current-profile",
        answer: Answer::Stated(
            None,
            Rewrite::DuplicateProfileWrittenDeviceCmyk,
            DUPLICATE_PROFILE_REINTERPRETS,
        ),
    },
    // ISO 19005-2 section 6.2.2, ISO 19005-4 section 6.2.2 — the page half of A003's reading.
    Remedy {
        requirement: "graphics/content-streams-have-an-explicit-resources-dictionary",
        answer: Answer::Mechanical(Rewrite::PageResources),
    },
    // ISO 19005-2 section 6.2.2, ISO 19005-4 section 6.2.2 — the second sentence, which
    // `TechNote 0010` A002 reads into part 2 (`doc/adr/2500`).
    Remedy {
        requirement: "graphics/named-resources-are-defined",
        answer: Answer::Mechanical(Rewrite::EmptyResourceDefined),
    },
];

/// This family's rows of [`super::CONDITIONAL`].
pub(super) const CONDITIONAL: &[(&str, Conditional)] = &[
    (
        "graphics/no-transfer-function-in-a-graphics-state",
        Conditional::PerDocument(Losses::one(Loss::TransferFunction)),
    ),
    (
        "graphics/second-transfer-function-is-default",
        Conditional::PerDocument(Losses::one(Loss::TransferFunction)),
    ),
    (
        "graphics/halftone-transfer-function-only-where-required",
        Conditional::PerDocument(Losses::one(Loss::TransferFunction)),
    ),
    (
        "graphics/no-overprint-mode-one-under-icc-cmyk",
        Conditional::PerDocument(Losses::one(Loss::OverprintMode)),
    ),
    // ISO 19005-4 section 6.2.4.3's `DeviceCMYK` row: a conversion states the press the operator
    // supplied at no cost, and the one this program ships as the loss `press-assumed`.
    (
        CMYK_UNDER_PART_FOUR,
        Conditional::PerDocument(Losses::one(Loss::PressAssumed)),
    ),
];

/// This family's rows of [`super::SHAPES`].
pub(super) const SHAPES: &[(&str, &[Shape])] = &[
    // ISO 19005-2 section 6.2.10, ISO 19005-4 section 6.2.9. §11.6.3's Table 136 entry has a
    // reader take the first mode of an array that it recognises, or `Normal` if it recognises
    // none — so an array reduces to a name every reader already used. A bare name the standard
    // does not define states a compositing function nothing defines, and writing one in its
    // place chooses how the marks under it are composited.
    (
        "graphics/graphics-state-blend-modes-are-defined",
        &[
            Shape {
                name: "array",
                answered: true,
            },
            Shape {
                name: "name",
                answered: false,
            },
        ],
    ),
    (
        "graphics/annotation-blend-modes-are-defined",
        &[
            Shape {
                name: "array",
                answered: true,
            },
            Shape {
                name: "name",
                answered: false,
            },
        ],
    ),
    // ISO 19005-2 section 6.2.2, ISO 19005-4 section 6.2.2.
    (
        "graphics/content-streams-have-an-explicit-resources-dictionary",
        &[
            Shape {
                name: "page",
                answered: true,
            },
            Shape {
                name: "form-xobject",
                answered: true,
            },
        ],
    ),
    // ISO 19005-2 section 6.2.2, ISO 19005-4 section 6.2.2. The two halves fail for different
    // reasons: §7.8.2's `BX`/`EX` pair brackets "a compatibility section, a portion of a content
    // stream within which unrecognised operators shall be ignored without error", so an operator
    // inside one is already ignored by every reader; outside one, §7.8.2 has an error occur.
    (
        "graphics/only-operators-the-base-standard-defines",
        &[
            Shape {
                name: "compatibility-section",
                answered: false,
            },
            Shape {
                name: "plain",
                answered: false,
            },
        ],
    ),
    // ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3. §7.4.9 has a stated `/ColorSpace`
    // decide what a JPEG 2000 image's samples mean, so a Flate copy in that space keeps them;
    // where the dictionary states none, the data's specification is what they mean and a copy
    // would need a space this converter chose (ADR 1400).
    (
        "graphics/jpeg2000-uses-the-baseline-feature-set",
        &[
            Shape {
                name: "stated-colour-space",
                answered: true,
            },
            Shape {
                name: "data-colour-space",
                answered: false,
            },
        ],
    ),
    // The same subclauses' channel count, bit depth and CIEJab rows: the same transcode answers
    // the same half of each, because their subject is JPEG 2000 data too (ADR 1412).
    (
        "graphics/jpeg2000-bit-depth",
        &[
            Shape {
                name: "stated-colour-space",
                answered: true,
            },
            Shape {
                name: "data-colour-space",
                answered: false,
            },
        ],
    ),
    (
        "graphics/jpeg2000-channel-count",
        &[
            Shape {
                name: "stated-colour-space",
                answered: true,
            },
            Shape {
                name: "data-colour-space",
                answered: false,
            },
        ],
    ),
    (
        "graphics/jpeg2000-no-ciejab-colour-space",
        &[
            Shape {
                name: "stated-colour-space",
                answered: true,
            },
            Shape {
                name: "data-colour-space",
                answered: false,
            },
        ],
    ),
];

/// This family's rows of [`super::REFUSED_BY_NAME`].
pub(super) const REFUSED_BY_NAME: &[(&str, Because)] = &[
    // ISO 19005-2 section 6.4.2, ISO 19005-4 section 6.4.2.
    // ISO 19005-2 section 6.2.3, ISO 19005-4 section 6.2.3: the destination profile the file
    // already holds.
    (
        "graphics/destination-profile-carries-the-tags-its-class-requires",
        Because::NotBuiltYet(DESTINATION_PROFILE_NOT_REPLACED),
    ),
    (
        "graphics/destination-profile-states-a-correct-profile-id",
        Because::NotBuiltYet(DESTINATION_PROFILE_NOT_REPLACED),
    ),
    // ISO 19005-2 section 6.2.4.2, ISO 19005-4 section 6.2.4.2: an `ICCBased` space's own. Both
    // parts' tag rows are answered in `REMEDIES` (`doc/adr/1990`, `doc/adr/2006`).
    (
        "graphics/icc-profiles-claim-a-permitted-edition",
        Because::NotBuiltYet(ICC_SPACE_PROFILE_NOT_REPLACED),
    ),
    // ISO 19005-2 section 6.2.3, ISO 19005-4 section 6.2.3: the shape of the array itself.
    (
        "graphics/page-output-intents-have-the-same-shape",
        Because::NotBuiltYet(PAGE_OUTPUT_INTENTS_NOT_TIDIED),
    ),
    // ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3: section 4.10's table. The two
    // rows whose field is in the JP2 wrapper are `REMEDIES`' now — a colour specification box can
    // be removed where it cannot be written — and what stays here is the three whose subject is
    // the codestream's own samples.
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
    (
        "graphics/jpeg2000-uses-the-baseline-feature-set",
        Because::NotBuiltYet(JPEG2000_BASELINE_NOT_REACHED),
    ),
    // ISO 19005-2 section 6.2.4.4, ISO 19005-4 section 6.2.4.4: section 4.5's two rows.
    (
        "graphics/separations-of-one-name-agree",
        Because::NotBuiltYet(SEPARATIONS_NOT_RECONCILED),
    ),
    // ISO 19005-2 section 6.2.9.2, ISO 19005-4 section 6.2.8.2: section 2.3's one workaround.
    (
        "graphics/no-reference-xobjects",
        Because::NotBuiltYet(REFERENCE_XOBJECT_NOT_PROXIED),
    ),
];

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

/// Why a page's own `/OutputIntents` array is not reconciled.
const PAGE_OUTPUT_INTENTS_NOT_TIDIED: &str = "a page of this file states an OutputIntents array \
     of its own, which ISO 19005-4 section 6.2.3 holds to the same three rules as the document's \
     — a PDF/A entry with a destination profile stream, no DestOutputProfileRef, and one profile \
     object across the array. The document-level array's third rule is built and this one is not: \
     a page-level intent is ISO 19005-4's facility for a document mixing colour destinations, so \
     which of a page's intents wins cannot be settled by the same proof that settles the \
     catalog's — the pages may legitimately differ from each other and from the document";

/// Why a `/DestOutputProfileRef` is not removed.
/// What removing an output intent's profile reference asserts (`doc/adr/2083`).
const PROFILE_REFERENCE_REINTERPRETS: &str = "an output intent in this file embeds its \
     DestOutputProfile and also states a DestOutputProfileRef naming profiles outside the file, \
     which ISO 19005 forbids, so the reference is removed. ISO 32000-2 14.11.5 makes an output \
     intent's data informational, Table 401 asks nothing of a processor about the reference, and \
     the profile the intent's colours are referred to is the embedded one, which stays; this \
     reader reads no reference, so every page is drawn where it was. What goes is the producer's \
     description of where further copies of a profile could be found";

/// What writing a `/TR2 /Identity` as `Default` asserts (`doc/adr/1978` section 6).
const SECOND_IDENTITY_REINTERPRETS: &str = "a graphics state in this file states TR2 Identity, \
     which ISO 19005 forbids, and no graphics state in it states any other transfer function, so \
     the entry is written Default, ISO 32000-2 8.4.5's Table 57 name for the transfer function in \
     effect at the start of the page. Table 52 leaves that initial function device dependent; on \
     this reader's device it is the identity, so every colour is drawn where it was. On a device \
     whose own default is not the identity, the producer's reset to the identity no longer \
     happens. The report names every graphics state rewritten";

/// What giving a PDF/A output intent stating no profile its profile asserts (`doc/adr/2035`).
const DESTINATION_SUPPLIED_REINTERPRETS: &str = "a PDF/A output intent in this file \
     identifies its output condition as Custom and states no destination profile, so this reader \
     drew the page as though no intent were stated. The entry is given the profile this \
     conversion's own PDF/A output intent would name - one another entry already names, or else \
     the profile a file stating no intent is given - which is the entry this conversion writes, \
     Custom included, with the producer's own Info kept. The report names the profile written";

/// What removing `HTP` asserts (`doc/adr/2035`).
const HALFTONE_PHASE_REINTERPRETS: &str = "a graphics state in this file states HTP, a halftone \
     phase that ISO 19005-2 section 6.2.5 forbids and that neither ISO 32000-1 nor ISO 32000-2 \
     defines: PDF 1.3 removed it. ISO 32000-2 Annex I.3 makes an entry the specification does not \
     describe change nothing a processor does, so the entry is removed and no processor of the \
     base standard, press or screen, behaves otherwise";

/// What writing a profile over an unreadable destination profile asserts (`doc/adr/2007`).
const DESTINATION_REPLACED_REINTERPRETS: &str = "the destination profile this file's own output \
     intent names is not an output or a monitor profile over grey, RGB or CMYK, and this reader \
     cannot draw through it, so it drew the page as though no intent were stated - which ISO \
     32000-2 14.11.5 permits, the intent being for informational purposes only. The profile a \
     file stating no intent would be given is written at the object every entry names, so the \
     device colours are stated as this reader showed them. What changes is for a processor that \
     could read the producer's profile, and the producer's own identification of the output \
     condition is kept beside a profile it did not choose. The report names the profile written";

/// What restating a destination profile's header asserts (`doc/adr/2021`).
const DESTINATION_RESTATED_REINTERPRETS: &str = "the destination profile this file's own output \
     intent names says in its header something its own bytes contradict: an ICC edition whose \
     tags it does not carry, where every tag it does carry is written as ICC.1:1998-09 defines \
     it, or a Profile ID that is not the digest of the profile. The header is restated as what \
     the bytes are - version 2.2.0, the reserved field that edition has where later ones put the \
     identifier cleared, or the digest written - and no tag changes, so this reader draws every \
     colour through the profile exactly as before. What changes is for a processor that read the \
     producer's header as the producer wrote it";

/// What removing a halftone's `TransferFunction /Identity` asserts (`doc/adr/2007`).
const HALFTONE_IDENTITY_REINTERPRETS: &str = "a halftone in this file states TransferFunction \
     Identity where ISO 32000-2 10.6.5.6 requires none, which ISO 19005 forbids, and no graphics \
     state or halftone in it states any other transfer function, so the entry is removed. By \
     ISO 32000-2 10.5 it overrode the graphics state's transfer function with the identity, and \
     this conversion leaves no transfer function in any graphics state, so on this reader's \
     device every colour is drawn where it was. On a device whose own default is not the \
     identity, the producer's reset to the identity for that colourant no longer happens. The \
     report names every halftone rewritten";

/// Why JPEG 2000 samples are not re-encoded.
const JPEG2000_SAMPLES_NOT_RE_ENCODED: &str = "doc/pdf-a-conversion-limits.md section 4.10: the \
     bit depth and the channel count are stated in the codestream's own SIZ marker, and the \
     enumerated CIEJab colour space is what the samples mean — so meeting these means decoding \
     the image, or relabelling what its numbers are. Where the image dictionary states a \
     ColorSpace the samples are already in its domain, and `remedy = \"preserve\"` at this site \
     transcodes them to FlateDecode in that space (doc/adr/1400, doc/adr/1412) — at a cost in \
     size, and with this program's decoder's output kept as the picture, which is why it is the \
     operator's answer rather than the default. Where it states none, what the data says is what \
     the samples mean, and a copy would need a ColorSpace this converter chose. One channel-count \
     case is cheaper and is unbuilt: a cdef box declaring the second channel as opacity leaves \
     one colour channel without a sample being touched";

/// Why JPEG 2000 data outside the JPX baseline is not brought into it without a configuration.
///
/// ADR 1383 reads each shape against ITU-T T.801 M.9.2 and ADR 1371's rule that a codec's output
/// is the image's samples. Where the dictionary states a `ColorSpace`, the data's colour
/// specifications are ones §7.4.9 has a reader ignore, and the samples are already in the
/// dictionary's domain: `preserve` gives a bare codestream the JP2 file structure, its bytes kept
/// (ADR 2695), and transcodes any other shape to `FlateDecode` (ADR 1400); a profile that answers
/// nothing, `refuse-any-loss` among them, reaches this sentence (ADR 2694). Where it states none, a
/// specification off M.9.2.4's list is what the samples mean, and any baseline space written in
/// its place relabels them.
const JPEG2000_BASELINE_NOT_REACHED: &str = "ITU-T T.801 M.9.2 defines the JPX baseline and \
     each way out of it is the samples' own. Where the image dictionary states a ColorSpace the \
     samples are already in its domain, and `remedy = \"preserve\"` at this site, which the \
     as-if-printed, as-shown, only-metadata-loss, no-data-loss and state-every-loss profiles \
     give, keeps them: a bare codestream is given the JP2 file structure with its bytes kept \
     (doc/adr/2695), and any other shape is transcoded to FlateDecode in that space at a cost in \
     size, with this program's decoder's output kept as the picture (doc/adr/1400, \
     doc/adr/2694). Where the dictionary states none, a colour specification off M.9.2.4's list is \
     what the samples mean, so writing a baseline one in its place, or a ColorSpace beside a Flate \
     copy, relabels the picture";

/// What restating an `ICCBased` profile stream's `N` asserts (`doc/adr/2099`).
const COMPONENT_COUNT_REINTERPRETS: &str = "an ICCBased colour space in this file states an N \
     that is missing or is not the number of components its profile has, which ISO 32000-2 \
     8.6.5.5's Table 66 requires it to match, so N is written as the count the profile's own \
     header gives. This reader draws such a space through its profile and reads no N there, so \
     every page is drawn where it was; a processor that took the stated N over the profile no \
     longer has a stated N to take. The report counts every profile restated";

/// What naming `DeviceCMYK` for a duplicated profile asserts (`doc/adr/2098`).
const DUPLICATE_PROFILE_REINTERPRETS: &str = "an ICCBased CMYK space in this file, or the \
     alternate of a Separation or DeviceN space, carries the profile of the PDF/A output intent or \
     of the transparency blending space in force wherever it is used, which ISO 19005-4 section \
     6.2.4.2 forbids, so every ICCBased array naming that profile is written DeviceCMYK. ISO \
     19005-4 section 6.2.4.3 renders DeviceCMYK through the output intent's profile, and ISO \
     32000-2 11.7.2 composites it as the blending space's, so each colour is the same colour \
     through the same profile; and no graphics state in the file turns overprinting on, so \
     8.6.7's overprint mode, the semantics DeviceCMYK keeps, acts on no mark. What changes is that \
     the space no longer depends on whether a processor makes 8.6.5.7's implicit conversion. The \
     report counts every array rewritten";

/// What setting an `ICCBased` mark's overprint mode to 0 asserts (`doc/adr/1978`).
const OVERPRINT_MODE_REINTERPRETS: &str = "a graphics state in this file sets overprint mode 1 \
     where an ICCBased CMYK space is painted with overprinting on, which ISO 19005 forbids, and \
     none of those states governs a DeviceCMYK mark, so each one's mode is set to 0. ISO 32000-2 \
     8.6.7 applies a non-zero mode only to DeviceCMYK marks, and 8.6.5.7 extends it to an \
     ICCBased one only where a processor treats that space as its output device's own; this \
     reader draws these pages in the screen's components and applies the mode to no ICCBased \
     mark, so every mark is drawn where it was. A processor that treated the space as its press's \
     and overprinted no longer leaves the colour under the marks' zero components, which is the \
     dependence on the processor the requirement removes. The report names every graphics state \
     rewritten";

/// What writing an unreadable profile's alternate asserts (`doc/adr/1990`).
const PROFILE_ALTERNATE_REINTERPRETS: &str = "an ICCBased colour space in this file carries a \
     profile short of the tags the ICC edition section 6.2.4.2 holds it to requires, and this reader \
     cannot draw through it, so it drew the space in ISO 32000-2 8.6.5.5's Table 65 alternate - \
     the Alternate entry, or the device space of N - which that table makes the space used where \
     the profile is not supported. That alternate is written where the ICCBased array was, so \
     every colour is stated as this reader showed it. What changes is for a processor that could \
     read the profile: its colours there were managed, and now they are the alternate's. The \
     report counts every array rewritten";

/// Why two `Separation` arrays of one name are not reconciled.
const SEPARATIONS_NOT_RECONCILED: &str = "two Separation arrays here name the same colourant and \
     define it differently. doc/pdf-a-conversion-limits.md section 4.5: making them agree means \
     choosing one definition and rewriting the others, and the two may genuinely render \
     differently — so the converter is to report the disagreement, show both, and rewrite only \
     when told which one wins. The report and the rewrite are both owed, and a document \
     assembled from several producers routinely lands here";

/// Why a reference `XObject`'s `/Ref` is not dropped in favour of its proxy.
const REFERENCE_XOBJECT_NOT_PROXIED: &str = "doc/pdf-a-conversion-limits.md section 2.3's one \
     case with a real workaround: §8.10.4 makes a reference XObject's containing form serve as a \
     proxy — what a processor draws when the referenced content is not available, and what one \
     that does not implement Ref draws unconditionally. An archived file is exactly the case \
     where the target will not be available, so dropping the Ref entry and keeping the proxy \
     writes down what the file was going to show. It is an Ask rather than a mechanical rewrite, \
     because a reader that could have reached the imported content now sees the placeholder \
     instead, and neither half is built";

/// ISO 19005 section 6.2.4.3's `DeviceRGB` sentence: the `/DefaultRGB` where the preparation could
/// build one, which it does only where the profile in hand cannot answer the clause and something
/// is drawn in `DeviceRGB`, and the intent's answer otherwise (`doc/adr/2114`).
pub(super) fn rgb_answer(
    family: Option<DeviceFamily>,
    authorised: Authorisations,
    prepared: &Prepared,
) -> Decision {
    match (&prepared.default_rgb, &prepared.supplied_cmyk) {
        (Ok(_), _) => Decision::Stated {
            rewrite: Rewrite::DefaultRgb,
            reinterprets: super::super::default_rgb::REINTERPRETS,
        },
        // A row whose family is the document's, where what it draws in is `DeviceCMYK` beside a
        // profile of another family and the operator supplied the press (`doc/adr/2130`). Without
        // a supply the row takes the intent's answer and the output's own verdict settles it:
        // which of the document's spaces failed the row is the finding's, not the survey's, and
        // a sentence about `DeviceCMYK` would be wrong for a row an RGB alternate failed.
        (Err(_), Ok(supplied))
            if family.is_none()
                && supplied.press == super::super::supplied_cmyk::Press::Supplied =>
        {
            Decision::Stated {
                rewrite: Rewrite::SuppliedDefaultCmyk,
                reinterprets: super::super::supplied_cmyk::REINTERPRETS,
            }
        }
        _ => answer_of(
            Answer::Stated(family, Rewrite::OutputIntent, OUTPUT_INTENT_REINTERPRETS),
            authorised,
            prepared,
        ),
    }
}

/// ISO 19005-2 section 6.2.4.3's `DeviceCMYK` sentence, whose licences `Prepared` has already
/// chosen between: a CMYK destination profile answers it outright; where the profile in hand is of
/// another family, the press profile the operator supplied is the default (`doc/adr/2130`), and
/// without one the `DeviceN` default is what NOTE 2 leaves.
pub(super) fn cmyk_answer_under_part_two(prepared: &Prepared) -> Decision {
    match (&prepared.intent, &prepared.default_cmyk) {
        (Ok(intent), _) if intent.family == DeviceFamily::Cmyk => Decision::Stated {
            rewrite: Rewrite::OutputIntent,
            reinterprets: output_intent_sentence(intent, OUTPUT_INTENT_REINTERPRETS),
        },
        _ if prepared.supplied_cmyk.is_ok() => Decision::Stated {
            rewrite: Rewrite::SuppliedDefaultCmyk,
            reinterprets: super::super::supplied_cmyk::REINTERPRETS,
        },
        (_, Ok(_)) => Decision::Stated {
            rewrite: Rewrite::DefaultCmyk,
            reinterprets: DEFAULT_CMYK_REINTERPRETS,
        },
        (_, Err(because)) => Decision::Refused(*because),
    }
}

/// ISO 19005-4 section 6.2.4.3's, which has no `DeviceN` half: the intent where it is a CMYK one,
/// the operator's press profile as the default otherwise, and the reason that could not be built —
/// which says what the operator can do — where neither is in hand (`doc/adr/2130`).
///
/// **The press this program ships is a loss and not a statement** (`doc/adr/2518`): where the
/// operator supplied none and the caller authorised [`Loss::PressAssumed`], the default over the
/// shipped profile is an authorised loss, and without that authorisation the preparation built
/// nothing and the refusal above stands.
pub(super) fn cmyk_answer_under_part_four(
    prepared: &Prepared,
    authorised: Authorisations,
) -> Decision {
    match (&prepared.intent, &prepared.supplied_cmyk) {
        (Ok(intent), _) if intent.family == DeviceFamily::Cmyk => Decision::Stated {
            rewrite: Rewrite::OutputIntent,
            reinterprets: output_intent_sentence(intent, OUTPUT_INTENT_REINTERPRETS),
        },
        (Ok(_), Ok(supplied)) if supplied.press.is_assumed() => losing(
            Losses::one(Loss::PressAssumed),
            Rewrite::SuppliedDefaultCmyk,
            authorised,
        ),
        (Ok(_), Ok(_)) => Decision::Stated {
            rewrite: Rewrite::SuppliedDefaultCmyk,
            reinterprets: super::super::supplied_cmyk::REINTERPRETS,
        },
        (Err(because), _) | (Ok(_), Err(because)) => Decision::Refused(*because),
    }
}
