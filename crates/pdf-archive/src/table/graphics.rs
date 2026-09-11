//! Clause 6.2 up to the fonts: what a page may be painted with.
//!
//! The largest tranche of clause 6 and the one whose rules divide most sharply into two kinds.
//! Some are **dictionary shape**: an image may not state `Alternates`, a graphics state may not
//! state `TR`, a halftone is of type 1 or 5. Those are decided by reading the objects a
//! cross-reference section names, and they are the ones implemented here.
//!
//! The rest turn on **what a page's content actually does** — which colour space is in force when a
//! colour is set, whether a page contains transparency, what a rendering intent operator's operand
//! was. Those read [`crate::survey`], which walks the content streams once and reports the answers
//! with the resource dictionary and the blending space that were in force at each point. ISO
//! 19005-2 section 6.2.4.3's device colour rules are the largest of them and the reason the survey
//! exists.
//!
//! A rule the survey still cannot decide stays a [`Check::Unchecked`] row naming what a reader
//! would have to answer, which is `doc/questions/Q20`'s discipline rather than a gap.
//!
//! # Why the colour rows are split by part where the other rows are not
//!
//! ISO 19005-2 section 6.2.4.3 and ISO 19005-4 section 6.2.4.3 license the same use differently,
//! and a predicate is not told which target it is running for. So each of the three device colour
//! sentences is two rows, one per part, and the same is true of the two subclauses that point at
//! them (section 6.2.4.4's alternate spaces, section 6.2.4.5's underlying ones) and of the
//! transparency group colour space. Part 4's extra licences — the current blending space, and a
//! page-level output intent — are exactly what would over-report if part 2's rule were applied to a
//! part 4 file, and under-report the other way round.
//!
//! A third kind is here for completeness and can never be otherwise: a clause addressed to the
//! *conforming processor* — ignore flatness, never substitute a thumbnail, respect overprint —
//! states nothing a document can be held to, and a row that quietly passed one would be claiming
//! to have judged the file. Nine rows are of that kind and they carry [`Check::Processor`],
//! which is neither a pass nor a debt: they are obligations on *this program*, and
//! `doc/PLAN.md` section 5a's conformance ledger is where a claim about its own rendering belongs.
//! Reading them as unchecked requirements overstated this crate's gap by nine rows on clause
//! 6.2 alone.
//!
//! # Where the two parts differ, and it is more than renumbering
//!
//! - ISO 19005-2 section 6.2.6 restricts rendering intent names; **ISO 19005-4 states no such
//!   subclause at all**, which is why the rendering-intent rows are [`Clauses::only_two`].
//! - Part 2 forbids `HTP` in a graphics state and part 4 forbids `HTO` — different keys, so
//!   different rows.
//! - Part 2 forbids `DestOutputProfileRef` in a *PDF/X* output intent; part 4 forbids it in
//!   *any* output intent.
//! - Part 4 admits a **page-level** PDF/A output intent, and requires one on any page whose
//!   contents are not fully device-independent. Part 2 knows only the document's array.
//! - Part 4's section 6.2.4.3 admits the **current transparency blending space** as a third way to
//!   license a device colour space, where part 2 admits only a default space or the output
//!   intent. Part 2 in turn admits a **DeviceN-based `DefaultCMYK`**, which part 4 dropped.
//! - Part 4 forbids an `ICCBased` space whose profile duplicates the output intent's CMYK
//!   profile; part 2 states no such rule. It is **two** rows in part 4, because section 6.2.4.4
//!   sends a `Separation`'s or `DeviceN`'s alternate space to section 6.2.4.2 as well, and a
//!   verdict has to cite the sentence that put the restriction where it found the fault.
//! - Both parts require an `ICCBased` space's profile to conform to something, and the
//!   somethings are different documents: part 2 names four ICC editions, part 4 defers to
//!   ISO 32000-2 §8.6.5.5. Only the second is a text this tree holds, so they are two rows
//!   and only one of them is checked.
//! - Part 2's transparency clause requires a page `Group` with a `CS` where the document has no
//!   PDF/A output intent; part 4 accepts a page-level output intent instead.
//! - Part 4 extends the blend-mode restriction to an **annotation** dictionary's `BM`.
//! - Part 2 forbids PostScript `XObject`s and the `Subtype2`/`PS` passthrough in a form `XObject`;
//!   part 4 dropped both, keeping only `OPI`.
//!
//! ISO 19005-4's Annex A does not touch clause 6.2. **Annex B does**, which this file said it did
//! not: section B.2.3 states how a PDF/A-4e processor colour manages 3D artwork, and sends it to
//! Section 6.2.4.2. So one row here is an [`Applies::Flavours`] row citing `B.2.3`, and every other
//! is [`Applies::Always`]. Nothing in Annex B binds a *file's* colour, which is why that row is a
//! processor obligation rather than a check.
//!
//! # Recommendations are not rows
//!
//! ISO 19005-2 section 6.2.4.4's closing sentence about `Colorants` consistency is a *should*. This
//! table is of requirements, and admitting a recommendation would make a failed verdict say
//! something the standard does not.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use pdf_model::Pages;
use pdf_model::icc;
use pdf_model::jpeg2000::{Channel, ColourSpecification, Headers};
use pdf_syntax::{Dictionary, Document, Object, ObjectId, Stream};

use crate::Examination;
use crate::finding::{Findings, Where};
use crate::requirement::{Applies, Check, Clauses, Requirement};
use crate::survey::{DefaultSpace, DeviceColour, DeviceFamily, IccProfile, Route, SpaceKind};
use crate::table::{name_of, states, states_name};
use crate::target::Flavour;

/// The rows this module contributes, which `super::TRANCHES` concatenates.
pub(super) static REQUIREMENTS: &[Requirement] = &[
    Requirement {
        id: "graphics/graphical-elements-rendered-as-the-base-standard-defines",
        asks: "A conforming processor shall render a page's graphical elements as the base \
               standard requires, as modified by ISO 19005.",
        clauses: Clauses::both("6.2.1", "6.2.1"),
        applies: Applies::Always,
        check: Check::Processor(
            "the clause binds a conforming processor's rendering rather than the file, so no \
             document can be judged against it; the rendering it asks for is this project's \
             own subject and `doc/conformance` is where that is tracked",
        ),
    },
    Requirement {
        id: "graphics/only-operators-the-base-standard-defines",
        asks: "A content stream shall use no operator the base standard does not define, even \
               inside the BX and EX compatibility brackets.",
        clauses: Clauses::both("6.2.2", "6.2.2"),
        applies: Applies::Always,
        check: Check::Implemented(only_operators_the_base_standard_defines),
    },
    Requirement {
        id: "graphics/content-streams-have-an-explicit-resources-dictionary",
        asks: "A content stream that names other objects shall have a resources dictionary \
               explicitly associated with it.",
        clauses: Clauses::both("6.2.2", "6.2.2"),
        applies: Applies::Always,
        check: Check::Implemented(content_streams_carry_their_own_resources),
    },
    Requirement {
        id: "graphics/named-resources-are-defined",
        asks: "A resources dictionary shall define every named resource the content stream it \
               belongs to references.",
        clauses: Clauses::both("6.2.2", "6.2.2"),
        applies: Applies::Always,
        check: Check::Implemented(named_resources_are_defined),
    },
    Requirement {
        id: "graphics/pdfa-output-intent-states-a-destination-profile",
        asks: "An output intent identified as a PDF/A one by the value GTS_PDFA1 in its S key \
               shall state a DestOutputProfile whose value is an ICC profile stream.",
        clauses: Clauses::both("6.2.3", "6.2.3"),
        applies: Applies::Always,
        check: Check::Implemented(pdfa_output_intent_states_a_destination_profile),
    },
    Requirement {
        id: "graphics/destination-profile-conforms-to-an-icc-edition",
        asks: "A PDF/A output intent's destination profile shall be a valid ICC profile — one \
               that conforms to the edition of the ICC specification its own header names.",
        clauses: Clauses::both("6.2.3", "6.2.3"),
        applies: Applies::Always,
        check: Check::Unchecked(DESTINATION_PROFILE_VALIDITY_NEEDS_AN_ICC_TEXT),
    },
    Requirement {
        id: "graphics/destination-profile-states-a-correct-profile-id",
        asks: "A destination profile that states a Profile ID shall state the MD5 the ICC \
               edition its header names defines, taken over its own bytes.",
        clauses: Clauses::both("6.2.3", "6.2.3"),
        applies: Applies::Always,
        check: Check::Implemented(destination_profile_states_a_correct_profile_id),
    },
    Requirement {
        id: "graphics/destination-profile-carries-the-tags-its-class-requires",
        asks: "A destination profile shall carry the tags the ICC edition its header names \
               requires of a profile of its device class.",
        clauses: Clauses::both("6.2.3", "6.2.3"),
        applies: Applies::Always,
        check: Check::Implemented(destination_profile_carries_the_tags_its_class_requires),
    },
    Requirement {
        id: "graphics/destination-profile-class-and-colour-space",
        asks: "A destination profile shall be an output or a monitor profile, and its colour \
               space shall be grey, RGB or CMYK.",
        clauses: Clauses::both("6.2.3", "6.2.3"),
        applies: Applies::Always,
        check: Check::Implemented(destination_profile_class_and_colour_space),
    },
    Requirement {
        id: "graphics/no-destination-profile-reference-in-a-pdfx-output-intent",
        asks: "An output intent identified as a PDF/X one shall not state the \
               DestOutputProfileRef key, which would put its profile outside the file.",
        clauses: Clauses::only_two("6.2.3"),
        applies: Applies::Always,
        check: Check::Implemented(no_destination_profile_reference_in_a_pdfx_output_intent),
    },
    Requirement {
        id: "graphics/no-destination-profile-reference",
        asks: "No output intent dictionary shall state the DestOutputProfileRef key, which \
               would put its profile outside the file.",
        clauses: Clauses::only_four("6.2.3"),
        applies: Applies::Always,
        check: Check::Implemented(no_destination_profile_reference),
    },
    Requirement {
        id: "graphics/one-destination-profile-per-output-intents-array",
        asks: "Where an OutputIntents array holds more than one entry, every entry that states \
               a DestOutputProfile shall state the same indirect object as its value.",
        clauses: Clauses::both("6.2.3", "6.2.3"),
        applies: Applies::Always,
        check: Check::Implemented(one_destination_profile_per_output_intents_array),
    },
    Requirement {
        id: "graphics/page-output-intents-have-the-same-shape",
        asks: "A page's own OutputIntents array shall obey the same requirements as the \
               document's: a PDF/A entry with a valid destination profile, no \
               DestOutputProfileRef, and one shared profile object across the array.",
        clauses: Clauses::only_four("6.2.3"),
        applies: Applies::Always,
        check: Check::Implemented(page_output_intents_have_the_same_shape),
    },
    Requirement {
        id: "graphics/a-device-dependent-page-carries-an-output-intent",
        asks: "Where the document states no document-level PDF/A output intent, every page \
               whose contents are not wholly device-independent shall state one of its own.",
        clauses: Clauses::only_four("6.2.3"),
        applies: Applies::Always,
        check: Check::Implemented(a_device_dependent_page_carries_an_output_intent),
    },
    Requirement {
        id: "graphics/colour-is-specified-device-independently",
        asks: "Every colour shall be specified device-independently, either by a \
               device-independent colour space or through the PDF/A output intent's profile.",
        clauses: Clauses::both("6.2.4.1", "6.2.4.1"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "the general statement the subclauses below make specific, and every colour this \
             crate can see is judged by the section 6.2.4.3 to section 6.2.4.5 rows. A predicate \
             here would \
             report those same failures under a clause number that adds nothing to them, and \
             the part of the sentence it would *not* cover — a colour specified indirectly \
             through the output intent's profile — is the licence those rows already apply",
        ),
    },
    Requirement {
        id: "graphics/icc-profiles-conform-to-a-permitted-edition",
        asks: "The profile forming an ICCBased colour space's stream shall conform to one of \
               the four ICC editions ISO 19005-2 names.",
        clauses: Clauses::only_two("6.2.4.2"),
        applies: Applies::Always,
        check: Check::Unchecked(ICC_PERMITTED_EDITION_NEEDS_THE_TEXTS_IT_NAMES),
    },
    Requirement {
        id: "graphics/icc-profiles-carry-the-tags-a-permitted-edition-requires",
        asks: "The profile forming an ICCBased colour space's stream shall carry the tags one of \
               the ICC editions ISO 19005-2 names requires of a profile of its device class.",
        clauses: Clauses::only_two("6.2.4.2"),
        applies: Applies::Always,
        check: Check::Implemented(icc_profiles_carry_the_tags_a_permitted_edition_requires),
    },
    Requirement {
        id: "graphics/icc-profiles-claim-a-permitted-edition",
        asks: "The profile forming an ICCBased colour space's stream shall not state a profile \
               version number that names a specification outside the four ISO 19005-2 permits.",
        clauses: Clauses::only_two("6.2.4.2"),
        applies: Applies::Always,
        check: Check::Implemented(icc_profiles_claim_a_permitted_edition),
    },
    Requirement {
        id: "graphics/icc-profiles-conform-to-the-version-they-name",
        asks: "The profile forming an ICCBased colour space's stream shall conform to the \
               specification version its own header states.",
        clauses: Clauses::only_four("6.2.4.2"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "part 4 sends this profile to ISO 32000 section 8.6.5.5 whole, and that clause closes \
             by requiring a profile to conform to the specification version its header names — so \
             which text decides is the profile's own choice. This project holds four of those \
             texts, one for each of the versions 2.2.0, 4.0.0, 4.4.0.0 and 5.0.0.0, and the row \
             below checks a profile naming any of them against that edition's required tags. What \
             stays here is what that leaves, and it is most of the population and most of each \
             document: a profile naming 2.1.0 — the commonest version there is — or 2.0.0, 2.3.0, \
             2.4.0 or 4.1.0 to 4.3.0 names a text that is not here, and for the four that are, \
             nothing beyond the tag lists is read. A `spac` profile is not judged even under a \
             held edition, because its required-tag table is the one class this crate has not \
             transcribed",
        ),
    },
    Requirement {
        id: "graphics/icc-profiles-carry-the-tags-their-version-requires",
        asks: "The profile forming an ICCBased colour space's stream shall carry the tags the ICC \
               edition its own header names requires of a profile of its device class.",
        clauses: Clauses::only_four("6.2.4.2"),
        applies: Applies::Always,
        check: Check::Implemented(icc_profiles_carry_the_tags_their_version_requires),
    },
    Requirement {
        id: "graphics/icc-profiles-conform-to-the-base-standard",
        asks: "The profile forming an ICCBased colour space's stream shall conform to what the \
               base standard requires of one: a component count of 1, 3 or 4 that matches the \
               profile, and a device class and colour space the base standard admits.",
        clauses: Clauses::only_four("6.2.4.2"),
        applies: Applies::Always,
        check: Check::Implemented(icc_profiles_conform_to_the_base_standard),
    },
    Requirement {
        id: "graphics/icc-alternate-space-not-used-for-rendering",
        asks: "A conforming processor shall render an ICCBased colour space through its profile \
               and not through the Alternate space the profile's stream dictionary names.",
        clauses: Clauses::both("6.2.4.2", "6.2.4.2"),
        applies: Applies::Always,
        check: Check::Processor(
            "binds a conforming processor's rendering rather than the file, so no document can \
             be judged against it",
        ),
    },
    Requirement {
        id: "graphics/no-overprint-mode-one-under-icc-cmyk",
        asks: "Overprint mode shall not be 1 while an ICCBased CMYK colour space is in use and \
               stroking or filling overprint is on.",
        clauses: Clauses::both("6.2.4.2", "6.2.4.2"),
        applies: Applies::Always,
        check: Check::Implemented(no_overprint_mode_one_under_icc_cmyk),
    },
    Requirement {
        id: "graphics/no-icc-space-duplicating-the-output-intent-profile",
        asks: "An ICCBased colour space shall not carry a CMYK destination profile identical to \
               the one in the output intent or the blending space in force.",
        clauses: Clauses::only_four("6.2.4.2"),
        applies: Applies::Always,
        check: Check::Implemented(no_icc_space_duplicating_a_current_profile),
    },
    Requirement {
        id: "graphics/device-rgb-needs-a-default-or-an-rgb-output-intent",
        asks: "DeviceRGB shall be used only where a device-independent DefaultRGB is in force, \
               or the file states a PDF/A output intent holding an RGB destination profile.",
        clauses: Clauses::only_two("6.2.4.3"),
        applies: Applies::Always,
        check: Check::Implemented(device_rgb_under_part_two),
    },
    Requirement {
        id: "graphics/device-rgb-needs-a-default-a-blending-space-or-an-rgb-output-intent",
        asks: "DeviceRGB shall be used only where a device-independent DefaultRGB is in force, \
               or the transparency blending space then in force is a device-independent \
               RGB-based space, or the PDF/A output intent then current holds an RGB \
               destination profile.",
        clauses: Clauses::only_four("6.2.4.3"),
        applies: Applies::Always,
        check: Check::Implemented(device_rgb_under_part_four),
    },
    Requirement {
        id: "graphics/device-cmyk-needs-a-default-or-a-cmyk-output-intent",
        asks: "DeviceCMYK shall be used only where a device-independent or DeviceN-based \
               DefaultCMYK is in force, or the file states a PDF/A output intent holding a \
               CMYK destination profile.",
        clauses: Clauses::only_two("6.2.4.3"),
        applies: Applies::Always,
        check: Check::Implemented(device_cmyk_under_part_two),
    },
    Requirement {
        id: "graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent",
        asks: "DeviceCMYK shall be used only where a device-independent DefaultCMYK is in \
               force, or the transparency blending space then in force is a device-independent \
               CMYK-based space, or the PDF/A output intent then current holds a CMYK \
               destination profile.",
        clauses: Clauses::only_four("6.2.4.3"),
        applies: Applies::Always,
        check: Check::Implemented(device_cmyk_under_part_four),
    },
    Requirement {
        id: "graphics/device-gray-needs-a-default-or-an-output-intent",
        asks: "DeviceGray shall be used only where a device-independent DefaultGray is in \
               force, or the file states a PDF/A output intent.",
        clauses: Clauses::only_two("6.2.4.3"),
        applies: Applies::Always,
        check: Check::Implemented(device_gray_under_part_two),
    },
    Requirement {
        id: "graphics/device-gray-needs-a-default-or-a-current-output-intent",
        asks: "DeviceGray shall be used only where a device-independent DefaultGray is in \
               force, or a PDF/A output intent — the page's own, or failing that the \
               document's — is then in effect.",
        clauses: Clauses::only_four("6.2.4.3"),
        applies: Applies::Always,
        check: Check::Implemented(device_gray_under_part_four),
    },
    Requirement {
        id: "graphics/device-colours-render-through-the-output-intent",
        asks: "A conforming processor shall render a device colour that no default space \
               replaces through the profile in the PDF/A output intent then in effect.",
        clauses: Clauses::both("6.2.4.3", "6.2.4.3"),
        applies: Applies::Always,
        check: Check::Processor(
            "binds a conforming processor's rendering rather than the file, so no document can \
             be judged against it",
        ),
    },
    Requirement {
        id: "graphics/process-colourants-render-through-the-output-intent",
        asks: "A conforming processor shall treat a Separation or DeviceN space named only for \
               process colourants as components of the output intent's CMYK profile.",
        clauses: Clauses::both("6.2.4.4", "6.2.4.4"),
        applies: Applies::Always,
        check: Check::Processor(
            "binds a conforming processor's rendering rather than the file, so no document can \
             be judged against it",
        ),
    },
    Requirement {
        id: "graphics/separation-alternate-spaces-obey-the-colour-rules",
        asks: "The alternate space of a Separation or DeviceN colour space shall obey the \
               device colour space restrictions, so a device space there needs a \
               device-independent default or a PDF/A output intent of its own family.",
        clauses: Clauses::only_two("6.2.4.4"),
        applies: Applies::Always,
        check: Check::Implemented(separation_alternate_spaces_under_part_two),
    },
    Requirement {
        id: "graphics/3d-artwork-colour-management",
        asks: "A processor that colour manages 3D artwork shall handle it as an ICCBased colour \
               built from the 3D stream's ColorSpace key, or from sRGB where it states none, \
               and shall do so after the artwork is rendered.",
        clauses: Clauses::only_four("B.2.3"),
        applies: Applies::Flavours(&[Flavour::E]),
        check: Check::Processor(
            "every sentence of the subclause is addressed to a conforming processor, and the \
             first of them relieves one of colour managing 3D artwork at all — so no property of \
             a document satisfies or breaks it. The rules it points at are section 6.2.4.2's, \
             applied \
             to a profile the processor builds rather than to a colour space the file selects; \
             this project's own answer to it belongs in `doc/PLAN.md` section 5a's ledger beside \
             the \
             clause 13 exclusion that keeps 3D artwork unrendered here",
        ),
    },
    Requirement {
        id: "graphics/separation-alternate-space-does-not-duplicate-a-current-profile",
        asks: "The alternate space of a Separation or DeviceN colour space shall not be an \
               ICCBased space carrying a CMYK destination profile identical to the one in the \
               output intent or the blending space in force.",
        clauses: Clauses::only_four("6.2.4.4"),
        applies: Applies::Always,
        check: Check::Implemented(separation_alternates_duplicating_a_current_profile),
    },
    Requirement {
        id: "graphics/separation-alternate-spaces-obey-the-colour-rules-of-part-four",
        asks: "The alternate space of a Separation or DeviceN colour space shall obey the \
               device colour space restrictions, so a device space there needs a \
               device-independent default, a blending space of its own family, or a current \
               PDF/A output intent of its own family.",
        clauses: Clauses::only_four("6.2.4.4"),
        applies: Applies::Always,
        check: Check::Implemented(separation_alternate_spaces_under_part_four),
    },
    Requirement {
        id: "graphics/spot-colourants-appear-in-the-colorants-dictionary",
        asks: "Every spot colour a DeviceN or NChannel colour space uses shall have an entry in \
               that space's Colorants dictionary.",
        clauses: Clauses::both("6.2.4.4", "6.2.4.4"),
        applies: Applies::Always,
        check: Check::Implemented(spot_colourants_appear_in_the_colorants_dictionary),
    },
    Requirement {
        id: "graphics/separations-of-one-name-agree",
        asks: "Every Separation array in the file that names the same colourant shall state the \
               same alternate space and the same tint transform.",
        clauses: Clauses::both("6.2.4.4", "6.2.4.4"),
        applies: Applies::Always,
        check: Check::Implemented(separations_of_one_name_agree),
    },
    Requirement {
        id: "graphics/indexed-and-pattern-base-spaces-obey-the-colour-rules",
        asks: "The colour space underlying an Indexed or Pattern colour space shall obey every \
               requirement the colour space subclauses state, so a device space there needs a \
               device-independent default or a PDF/A output intent of its own family.",
        clauses: Clauses::only_two("6.2.4.5"),
        applies: Applies::Always,
        check: Check::Implemented(underlying_spaces_under_part_two),
    },
    Requirement {
        id: "graphics/indexed-and-pattern-base-spaces-obey-the-colour-rules-of-part-four",
        asks: "The colour space underlying an Indexed or Pattern colour space shall obey every \
               requirement the colour space subclauses state, so a device space there needs a \
               device-independent default, a blending space of its own family, or a current \
               PDF/A output intent of its own family.",
        clauses: Clauses::only_four("6.2.4.5"),
        applies: Applies::Always,
        check: Check::Implemented(underlying_spaces_under_part_four),
    },
    Requirement {
        id: "graphics/no-transfer-function-in-a-graphics-state",
        asks: "A graphics state parameter dictionary shall not state the TR key.",
        clauses: Clauses::both("6.2.5", "6.2.5"),
        applies: Applies::Always,
        check: Check::Implemented(no_transfer_function_in_a_graphics_state),
    },
    Requirement {
        id: "graphics/no-halftone-phase-in-a-graphics-state",
        asks: "A graphics state parameter dictionary shall not state the HTP key.",
        clauses: Clauses::only_two("6.2.5"),
        applies: Applies::Always,
        check: Check::Implemented(no_halftone_phase_in_a_graphics_state),
    },
    Requirement {
        id: "graphics/no-halftone-origin-in-a-graphics-state",
        asks: "A graphics state parameter dictionary shall not state the HTO key.",
        clauses: Clauses::only_four("6.2.5"),
        applies: Applies::Always,
        check: Check::Implemented(no_halftone_origin_in_a_graphics_state),
    },
    Requirement {
        id: "graphics/second-transfer-function-is-default",
        asks: "Where a graphics state parameter dictionary states the TR2 key, its value shall \
               be the name Default.",
        clauses: Clauses::both("6.2.5", "6.2.5"),
        applies: Applies::Always,
        check: Check::Implemented(second_transfer_function_is_default),
    },
    Requirement {
        id: "graphics/halftone-transfer-function-only-where-required",
        asks: "A halftone dictionary shall state a TransferFunction only where the base \
               standard requires one.",
        clauses: Clauses::both("6.2.5", "6.2.5"),
        applies: Applies::Always,
        check: Check::Implemented(halftone_transfer_function_only_where_required),
    },
    Requirement {
        id: "graphics/halftone-type-is-one-or-five",
        asks: "Every halftone shall state a HalftoneType of 1 or 5.",
        clauses: Clauses::both("6.2.5", "6.2.5"),
        applies: Applies::Always,
        check: Check::Implemented(halftone_type_is_one_or_five),
    },
    Requirement {
        id: "graphics/no-halftone-name",
        asks: "A halftone shall not state the HalftoneName key.",
        clauses: Clauses::both("6.2.5", "6.2.5"),
        applies: Applies::Always,
        check: Check::Implemented(no_halftone_name),
    },
    Requirement {
        id: "graphics/black-generation-and-undercolour-removal-ignored",
        asks: "A conforming processor shall ignore the BG, BG2, UCR and UCR2 functions when it \
               renders.",
        clauses: Clauses::both("6.2.5", "6.2.5"),
        applies: Applies::Always,
        check: Check::Processor(
            "binds a conforming processor's rendering rather than the file, so no document can \
             be judged against it",
        ),
    },
    Requirement {
        id: "graphics/overprint-entries-respected",
        asks: "A conforming processor shall respect OP, op and OPM, simulating overprint on a \
               device that does not support every colourant natively.",
        clauses: Clauses::both("6.2.5", "6.2.5"),
        applies: Applies::Always,
        check: Check::Processor(
            "binds a conforming processor's rendering rather than the file, so no document can \
             be judged against it",
        ),
    },
    Requirement {
        id: "graphics/rendering-intent-entries-name-one-of-four",
        asks: "A graphics state's RI entry and an image dictionary's Intent entry shall name \
               one of the four rendering intents the base standard defines.",
        clauses: Clauses::only_two("6.2.6"),
        applies: Applies::Always,
        check: Check::Implemented(rendering_intent_entries_name_one_of_four),
    },
    Requirement {
        id: "graphics/rendering-intent-operator-names-one-of-four",
        asks: "The operand of the rendering intent operator shall name one of the four \
               rendering intents the base standard defines.",
        clauses: Clauses::only_two("6.2.6"),
        applies: Applies::Always,
        check: Check::Implemented(rendering_intent_operator_names_one_of_four),
    },
    Requirement {
        id: "graphics/flatness-value-ignored",
        asks: "A conforming processor shall ignore the flatness a file states and choose its \
               own value instead.",
        clauses: Clauses::both("6.2.7", "6.2.6"),
        applies: Applies::Always,
        check: Check::Processor(
            "binds a conforming processor's rendering rather than the file, so no document can \
             be judged against it",
        ),
    },
    Requirement {
        id: "graphics/no-image-alternates-or-opi",
        asks: "An image dictionary shall state neither the Alternates key nor the OPI key.",
        clauses: Clauses::both("6.2.8.1", "6.2.7.1"),
        applies: Applies::Always,
        check: Check::Implemented(no_image_alternates_or_opi),
    },
    Requirement {
        id: "graphics/image-interpolation-is-off",
        asks: "Where an image dictionary states the Interpolate key, its value shall be false.",
        clauses: Clauses::both("6.2.8.1", "6.2.7.1"),
        applies: Applies::Always,
        check: Check::Implemented(image_interpolation_is_off),
    },
    Requirement {
        id: "graphics/inline-image-interpolation-is-off",
        asks: "Where an inline image states the I key, its value shall be false.",
        clauses: Clauses::both("6.2.8.1", "6.2.7.1"),
        applies: Applies::Always,
        check: Check::Implemented(inline_image_interpolation_is_off),
    },
    Requirement {
        id: "graphics/thumbnails-never-stand-in-for-a-page",
        asks: "A conforming processor shall never render a page from a thumbnail image, from \
               whatever part of the file that thumbnail came.",
        clauses: Clauses::both("6.2.8.2", "6.2.7.2"),
        applies: Applies::Always,
        check: Check::Processor(
            "binds a conforming processor's rendering rather than the file, so no document can \
             be judged against it",
        ),
    },
    Requirement {
        id: "graphics/jpeg2000-uses-the-baseline-feature-set",
        asks: "JPEG 2000 data shall use only the JPX baseline feature set, as the base standard \
               and ISO 19005 restrict and extend it, and shall be created and read as the \
               extensions part describes.",
        clauses: Clauses::both("6.2.8.3", "6.2.7.3"),
        applies: Applies::Always,
        check: Check::Unchecked(JPEG2000_BASELINE_IS_IN_THE_PART_NOT_HELD),
    },
    Requirement {
        id: "graphics/jpeg2000-channel-count",
        asks: "JPEG 2000 data shall have 1, 3 or 4 colour channels.",
        clauses: Clauses::both("6.2.8.3", "6.2.7.3"),
        applies: Applies::Always,
        check: Check::Implemented(jpeg2000_channel_count),
    },
    Requirement {
        id: "graphics/jpeg2000-one-best-colour-space-specification",
        asks: "Where JPEG 2000 data states more than one colour space specification, exactly \
               one shall be marked as the best available, and any ICC profile it names shall \
               conform to the base standard.",
        clauses: Clauses::both("6.2.8.3", "6.2.7.3"),
        applies: Applies::Always,
        check: Check::Implemented(jpeg2000_one_best_colour_space_specification),
    },
    Requirement {
        id: "graphics/jpeg2000-colour-specification-method",
        asks: "The colour specification method a JPEG 2000 colour box states shall be one of \
               the three the part permits.",
        clauses: Clauses::both("6.2.8.3", "6.2.7.3"),
        applies: Applies::Always,
        check: Check::Implemented(jpeg2000_colour_specification_method),
    },
    Requirement {
        id: "graphics/jpeg2000-no-ciejab-colour-space",
        asks: "JPEG 2000 data shall not use the enumerated CIEJab colour space.",
        clauses: Clauses::both("6.2.8.3", "6.2.7.3"),
        applies: Applies::Always,
        check: Check::Implemented(jpeg2000_no_ciejab_colour_space),
    },
    Requirement {
        id: "graphics/jpeg2000-bit-depth",
        asks: "JPEG 2000 data shall have a bit depth between 1 and 38, the same for every \
               colour channel.",
        clauses: Clauses::both("6.2.8.3", "6.2.7.3"),
        applies: Applies::Always,
        check: Check::Implemented(jpeg2000_bit_depth),
    },
    Requirement {
        id: "graphics/jpeg2000-device-colour-the-image-dictionary-states",
        asks: "Where a JPEG 2000 image's ColorSpace entry names a device colour space, the \
               device colour space requirements shall apply to it.",
        clauses: Clauses::both("6.2.8.3", "6.2.7.3"),
        applies: Applies::Always,
        check: Check::Unchecked(JPEG2000_DEVICE_COLOUR_IS_DELEGATED),
    },
    Requirement {
        id: "graphics/jpeg2000-device-colour-the-codestream-defines",
        asks: "Where a JPEG 2000 image states no ColorSpace entry and the colour space defined \
               in its data is effectively a device one, the device colour space requirements \
               shall apply to it.",
        clauses: Clauses::both("6.2.8.3", "6.2.7.3"),
        applies: Applies::Always,
        check: Check::Unchecked(JPEG2000_DEVICE_COLOUR_IN_THE_CODESTREAM),
    },
    Requirement {
        id: "graphics/no-form-xobject-opi",
        asks: "A form XObject dictionary shall not state the OPI key.",
        clauses: Clauses::both("6.2.9.1", "6.2.8.1"),
        applies: Applies::Always,
        check: Check::Implemented(no_form_xobject_opi),
    },
    Requirement {
        id: "graphics/no-postscript-passthrough-in-a-form-xobject",
        asks: "A form XObject dictionary shall state neither the PS key nor a Subtype2 of PS, \
               both of which carry executable PostScript.",
        clauses: Clauses::only_two("6.2.9.1"),
        applies: Applies::Always,
        check: Check::Implemented(no_postscript_passthrough_in_a_form_xobject),
    },
    Requirement {
        id: "graphics/no-reference-xobjects",
        asks: "The file shall contain no reference XObject, which is a form XObject that names \
               content in another file through a Ref key.",
        clauses: Clauses::both("6.2.9.2", "6.2.8.2"),
        applies: Applies::Always,
        check: Check::Implemented(no_reference_xobjects),
    },
    Requirement {
        id: "graphics/no-postscript-xobjects",
        asks: "The file shall contain no PostScript XObject.",
        clauses: Clauses::only_two("6.2.9.3"),
        applies: Applies::Always,
        check: Check::Implemented(no_postscript_xobjects),
    },
    Requirement {
        id: "graphics/a-transparent-page-has-a-blending-space",
        asks: "Where the file states no PDF/A output intent, every page containing transparency \
               shall state a Group whose attribute dictionary names a CS to blend in.",
        clauses: Clauses::only_two("6.2.10"),
        applies: Applies::Always,
        check: Check::Implemented(a_transparent_page_has_a_blending_space),
    },
    Requirement {
        id: "graphics/a-transparent-page-has-a-blending-space-or-an-output-intent",
        asks: "Where the file states no document-level PDF/A output intent, every page \
               containing transparency shall state either a page-level output intent or a \
               Group whose attribute dictionary names a CS to blend in.",
        clauses: Clauses::only_four("6.2.9"),
        applies: Applies::Always,
        check: Check::Implemented(a_transparent_page_has_a_blending_space_or_an_output_intent),
    },
    Requirement {
        id: "graphics/transparency-group-colour-spaces-obey-the-colour-rules",
        asks: "The CS entry of any transparency group's attribute dictionary shall obey every \
               requirement the colour space subclauses state, so a device space there needs a \
               device-independent default or a PDF/A output intent of its own family.",
        clauses: Clauses::only_two("6.2.10"),
        applies: Applies::Always,
        check: Check::Implemented(group_colour_spaces_obey_the_colour_rules_under_part_two),
    },
    Requirement {
        id: "graphics/transparency-group-colour-spaces-obey-the-colour-rules-of-part-four",
        asks: "The CS entry of any transparency group's attribute dictionary shall obey every \
               requirement the colour space subclauses state, so a device space there needs a \
               device-independent default or a current PDF/A output intent of its own family.",
        clauses: Clauses::only_four("6.2.9"),
        applies: Applies::Always,
        check: Check::Implemented(group_colour_spaces_obey_the_colour_rules_under_part_four),
    },
    Requirement {
        id: "graphics/graphics-state-blend-modes-are-defined",
        asks: "The BM key of a graphics state parameter dictionary shall name a blend mode the \
               base standard defines.",
        clauses: Clauses::both("6.2.10", "6.2.9"),
        applies: Applies::Always,
        check: Check::Implemented(graphics_state_blend_modes_are_defined),
    },
    Requirement {
        id: "graphics/annotation-blend-modes-are-defined",
        asks: "The BM key of an annotation dictionary shall name a blend mode the base standard \
               defines.",
        clauses: Clauses::only_four("6.2.9"),
        applies: Applies::Always,
        check: Check::Implemented(annotation_blend_modes_are_defined),
    },
];

/// What is left of the destination profile's *validity* once the two rows beside it are subtracted.
///
/// Both parts require the `DestOutputProfile` value to be a valid ICC profile stream, and neither
/// says what makes one valid; the base standard hands the format to §8.6.5.5, which hands it to
/// the ICC specification itself. That sentence bottoms out in a text, and **which** text is the
/// profile's own choice: §8.6.5.5 closes with "Profiles shall conform to the specification version
/// indicated by the Profile version number in its header", so a profile stating 4.4.0.0 is judged
/// by ICC.1:2022 and one stating 5.0.0.0 by ICC.2:2023 — the two this project now holds — while
/// one stating 2.x, or 4.0 to 4.3, names a text that is still not here.
///
/// Four editions are now held and each states the version consistent with itself, so a profile
/// stating 2.2.0, 4.0.0, 4.4.0.0 or 5.0.0.0 is judged by the text it names; one stating 2.0.0,
/// 2.1.0, 2.3.0, 2.4.0 or 4.1.0 to 4.3.0 names a text that is not here.
///
/// So this row is the remainder rather than the whole question, and it is a real remainder in two
/// directions:
///
/// - **A profile naming an edition this project does not hold** is not judged at all — and the
///   commonest profile version in the wild, 2.1.0, is one of them.
/// - **Even for a held edition, only two requirements are checked**: the `Profile ID` and the
///   required tags of the profile's class. No edition's tag-type restrictions or encodings are, nor
///   the rest of its header fields, and three slivers are worth naming because they *are* readable
///   and are deliberately left out: ICC.2 section 8.2's `spectralWhitePointTag`, required where the
///   spectral PCS field is not zero; ICC.2's `mediaWhitePointTag`, required where the PCS field is
///   not zero; and the `chromaticAdaptationTag` the v4 texts require where the measurement
///   illuminant was not D50. The first two would need this crate to read header bytes it does not;
///   the third turns on a fact no byte of the profile states.
///
/// The corpus's `6-2-3-t01-fail-d` is what made the gap concrete, and it is the same file under
/// both parts: an Adobe RGB (1998) profile whose header states version 5.0.0.0, which by ICC.2
/// section 7.2.6 is iccMAX's own version. **ICC.2 decides it, and against the file.** ICC.2 section
/// 8.4 requires a display profile to carry one or more of `AToB0Tag` to `AToB3Tag` or `DToB0Tag` to
/// `DToB3Tag` and one or more of `BToA0Tag` to `BToA3Tag` or `BToD0Tag` to `BToD3Tag`; this profile
/// carries none of the sixteen, only the matrix and curve tags of the v2 form, which ICC.2 does not
/// define. Its `Profile ID` field states a value that is not the one section 7.2.20's method gives
/// either. Both are reported by the rows beside this one, so the file is no longer a miss —
/// what stays here is everything else validity means.
const DESTINATION_PROFILE_VALIDITY_NEEDS_AN_ICC_TEXT: &str = "both parts require the destination profile to be a *valid* ICC profile stream and neither \
     defines validity; the base standard sends the format to ISO 32000 section 8.6.5.5, which \
     sends it to the ICC specification — and to a particular edition of it, since that clause \
     requires a profile to conform to the version its own header states. This project holds four \
     of those editions and each states its own version number: ICC.1:1998-09 (2.2.0), \
     ICC.1:2001-12 (4.0.0), ICC.1:2022 (4.4.0.0) and ICC.2:2023 (5.0.0.0). It holds \
     ISO 15076-1:2010 (4.3.0.0) only as a front-matter preview that stops before clause 7, and \
     holds ICC.1:2003-09 (4.1.0) not at all. So a profile stating any other version — 2.1.0 \
     above all, which is most of the profiles that exist — is judged by no text that is here. \
     For the four that are, the required tags of the profile's class are checked by the row \
     beside this one, and the `Profile ID` by the row beside that where the edition states a \
     method (ICC.1:1998-09 has no such field; ICC.1:2001-12 has the field and sends the method \
     to a web note). The rest of what those editions require of a profile is not checked, \
     including ICC.2's two conditional white points and the v4 chromatic adaptation tag";

/// Why conformance to one of ISO 19005-2's four ICC editions stays unchecked, and what replaced it.
///
/// ISO 19005-2 section 6.2.4.2 names ICC.1:1998-09, ICC.1:2001-12, ICC.1:2003-09 and ISO 15076-1.
/// The last is undated in that part's clause 2, whose normative-reference boilerplate makes the
/// latest edition apply, and its title line there pins the part to the one based on ICC.1:2010.
/// This project holds two of the four, both since the nine-hundred-and-fiftieth session:
/// ICC.1:1998-09 and ICC.1:2001-12. ICC.1:2003-09 is not here — the ICC supplies its past
/// specifications on request only — and ISO 15076-1:2010 is a front-matter preview. Holding
/// ICC.1:2022 does not substitute for either of the two that are missing: it is a fifth text,
/// profile version 4.4.0.0, and part 2 names no edition later than 4.3.0.0.
///
/// **A version number identifies which edition a profile claims**, and that was worth establishing
/// rather than assuming. All four held editions say the major and minor versions are set by the ICC
/// and each states the number consistent with its own text — 2.2.0, 4.0.0, 4.4.0.0, 5.0.0.0.
/// ICC.1:2022's foreword walks the chain: ICC.1:2004-08 was proposed as ISO 15076-1:2005 at
/// revision 4.2, ISO 15076-1:2010 and ICC.1:2010 are technically identical at 4.3.0.0, and 4.4 is
/// that edition. ISO 19005-2's own NOTE 1 makes the same identification from the other side, calling
/// ISO 15076-1 technically identical to the earlier texts in every respect relevant here *other than
/// the value of the profile version number*.
///
/// **But a claim is not a conformance, and this clause asks for the second.** Part 2's sentence is a
/// disjunction over documents and requires no profile to state any particular number; neither held
/// text requires its own number of a profile either. So the row below judges every `ICCBased`
/// profile against the two texts held of the four, whatever version it states, and this row keeps
/// what that leaves: the rest of those two documents, and the two documents that are not here.
const ICC_PERMITTED_EDITION_NEEDS_THE_TEXTS_IT_NAMES: &str = "part 2 names four ICC texts and this project now holds two of them: ICC.1:1998-09 and \
     ICC.1:2001-12 arrived in the nine-hundred-and-fiftieth session, and the clause 6.3 \
     required-tag lists of both are checked by the row below. ICC.1:2003-09 is not here — the \
     ICC supplies its past specifications on request only — and ISO 15076-1:2010 is a preview of \
     the front matter that stops before clause 7. What stays unchecked is therefore two things \
     rather than one: everything the two held texts require beyond those tag lists, which is most \
     of each document; and, for a profile that satisfies neither, whether one of the two texts \
     not held would admit it all the same. `CLAUDE.md` principle 5 forbids closing either from \
     somebody else's reading";

/// Why the first half of the device-colour sentence is delegated rather than predicated.
///
/// The sentence names two routes by which a JPEG 2000 image can come to use `DeviceGray`,
/// `DeviceRGB` or `DeviceCMYK`, and puts them in an order: the `ColorSpace` entry of the image
/// `XObject`, and — *in the absence thereof* — the colour space defined in the JPEG 2000 data.
/// The first route is not ambiguous in any degree: an entry naming `/DeviceRGB` is a use of
/// `DeviceRGB`, and section 6.2.4.3 binds every use of one whatever compression carries the
/// samples.
///
/// **That row used to be one row, and its reason said the whole rule turned on the word
/// *effectively*.** It does not: only the second route does. `crate::survey`'s `image_space`
/// records the `ColorSpace` of every image `XObject` a content stream draws, in the resource
/// dictionary that was in force, and the six section 6.2.4.3 rows judge what it records — so
/// the first route is already answered under the clause that states the rule, and a predicate
/// here would report the same failure twice under a clause number that adds nothing.
///
/// The limits are the survey's rather than this row's, and they are the ones its own rows carry:
/// an image on no page is not drawn and so not recorded, and a soft mask's image is not descended
/// into.
const JPEG2000_DEVICE_COLOUR_IS_DELEGATED: &str = "delegated, and the delegation is real rather than promised: the sentence's first route is \
     the image XObject's own ColorSpace entry, which `crate::survey` records for every image a \
     content stream draws and the six section 6.2.4.3 rows then judge. A predicate here would \
     report those same failures under a clause number that adds nothing to them. What the \
     sentence adds beyond them is its second route, which is the row beside this one";

/// Why the second half of the device-colour sentence needs a part this project does not hold.
///
/// `pdf_model::jpeg2000` reads the `colr` boxes, so what the codestream *declares* is legible.
/// What is not is the word the clause turns on. Neither part says which of the enumerated colour
/// spaces makes an image *effectively* `DeviceGray`, `DeviceRGB` or `DeviceCMYK`: 16 and 17 are
/// sRGB and an sRGB-nonlinearity greyscale, which are calibrated rather than device, and 12
/// (CMYK) carries no such definition at all. Both parts' NOTE 3 sends the question to
/// ISO/IEC 15444-2, which `doc/questions/A51` settles will not be bought.
const JPEG2000_DEVICE_COLOUR_IN_THE_CODESTREAM: &str = "the codestream is readable — `pdf_model::jpeg2000` reports the `colr` boxes — and the word \
     the rule turns on is not: neither part says which enumerated colour space makes an image \
     *effectively* a device one, and both send the question to ISO/IEC 15444-2 in their NOTE 3. \
     Numbers 16 and 17 are sRGB and an sRGB-nonlinearity greyscale, which are calibrated rather \
     than device; 12 (CMYK) has no such definition attached. `doc/questions/A51` rules that the \
     extensions part will not be bought, so this is settled rather than outstanding, and a later \
     round should neither reconstruct the reading from a secondary source nor soften this reason";

/// Why the baseline-feature row is unchecked, where five rows beside it no longer are.
///
/// The project holds ISO/IEC 15444-1:2000, which is what the other five needed; it does not hold
/// ISO/IEC 15444-2:2004, which is what this one needs and the only place either sentence of it is
/// defined. **`doc/questions/A51` closed that**: the owner will not buy the extensions part, so
/// this row is settled rather than pending, and `doc/adr/0928` records the argument.
///
/// The two later editions of part 1 in `doc/` cannot substitute for it, and a round that has not
/// opened them will assume they can: they are iTeh STANDARD PREVIEW extracts, fifteen pages of
/// front matter apiece, and neither contains a single occurrence of `colr` or `EnumCS`.
const JPEG2000_BASELINE_IS_IN_THE_PART_NOT_HELD: &str = "both sentences of this rule name the extensions part rather than the core one. Its NOTE 1 \
     says the JPX baseline set of features is defined in ISO/IEC 15444-2:2004 M.9.2, and the \
     subclause closes by requiring the image to be created and read as that document describes. \
     This project holds ISO/IEC 15444-1:2000 — enough for the channel count, the colour \
     specification boxes and the bit depth, which are checked — and does not hold part 2, so \
     there is no list of baseline features to judge an image against and `CLAUDE.md` principle 5 \
     forbids reconstructing one from another implementation. This is settled rather than \
     outstanding: `doc/questions/A51` rules that the extensions part will not be bought, so the \
     row stays unchecked deliberately and a later round should neither reconstruct the list from \
     a secondary source nor soften this reason (`doc/adr/0928`)";

/// How deep into one cross-referenced object's own structure the walk below goes.
///
/// A second bound rather than the only one — `pdf_syntax::Limits` already caps how deeply a
/// parsed object nests — so that the recursion here stays finite whatever that limit becomes.
/// Nothing a resource dictionary nests comes close to it.
const MAX_DEPTH: usize = 32;

/// The blend modes ISO 32000 defines, from its separable and non-separable tables.
///
/// `Compatible` is among them: the base standard lists it, deprecated in PDF 2.0 and still
/// defined, and a rule about what the standard *defines* is not a rule about what it prefers.
/// The same list drives `pdf_model::content::ext_gstate`, which is where a name reaching this
/// program is turned into a mode.
const BLEND_MODES: [&[u8]; 17] = [
    b"Normal",
    b"Compatible",
    b"Multiply",
    b"Screen",
    b"Overlay",
    b"Darken",
    b"Lighten",
    b"ColorDodge",
    b"ColorBurn",
    b"HardLight",
    b"SoftLight",
    b"Difference",
    b"Exclusion",
    b"Hue",
    b"Saturation",
    b"Color",
    b"Luminosity",
];

/// The four rendering intents ISO 32000 defines, which ISO 19005-2 section 6.2.6 restricts a file
/// to.
const RENDERING_INTENTS: [&[u8]; 4] = [
    b"RelativeColorimetric",
    b"AbsoluteColorimetric",
    b"Perceptual",
    b"Saturation",
];

/// What a dictionary's *position* in the file says it is, where its own entries need not.
///
/// Two of ISO 32000's dictionaries carry no required `Type`: a graphics state parameter
/// dictionary and an annotation. Both are reached through a fixed key — a resource
/// dictionary's `ExtGState` table and a page's `Annots` array — so the position is what
/// identifies them, and the walk carries it down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Position {
    /// Nowhere in particular: only the dictionary's own entries say what it is.
    Anywhere,
    /// A resource dictionary's `ExtGState` table, whose entries are graphics states.
    GraphicsStateTable,
    /// One entry of such a table.
    GraphicsState,
    /// A page's `Annots` array, whose entries are annotations.
    AnnotationArray,
    /// One entry of such an array.
    Annotation,
}

/// One dictionary the walk reached.
#[derive(Debug)]
struct Site<'a> {
    /// The cross-referenced object it was found in, which is the witness a report prints.
    id: ObjectId,
    /// What its position says it is.
    position: Position,
    /// The dictionary — a stream's dictionary, where the object it sits in is a stream.
    dict: &'a Dictionary,
}

/// Visits every dictionary the file's cross-referenced objects state, direct ones included.
///
/// The population is `file_structure`'s and for its reason: ISO 19005-2 section 6.1.4 and
/// ISO 19005-4 section 6.1.4 exempt an indirect object no cross-reference section names, so what a
/// requirement binds is exactly the objects that table reaches. References are *not* followed —
/// every indirect object is visited once by the loop itself — which is what makes the walk
/// terminate without a set of visited objects and visit each dictionary exactly once.
fn for_each_dictionary(exam: &Examination<'_>, mut visit: impl FnMut(&Site<'_>)) {
    let positions = positions(exam);
    for (id, object) in exam.objects() {
        let at = positions.get(id).copied().unwrap_or(Position::Anywhere);
        descend(object, *id, at, 0, &mut visit);
    }
}

/// One object's own structure, and the dictionaries inside it.
fn descend(
    object: &Object,
    id: ObjectId,
    at: Position,
    depth: usize,
    visit: &mut impl FnMut(&Site<'_>),
) {
    if depth > MAX_DEPTH {
        return;
    }
    let deeper = depth.saturating_add(1);
    let dict = match object {
        Object::Array(items) => {
            let each = if at == Position::AnnotationArray {
                Position::Annotation
            } else {
                Position::Anywhere
            };
            for item in items {
                descend(item, id, each, deeper, visit);
            }
            return;
        }
        Object::Dictionary(dict) => dict,
        Object::Stream(stream) => &stream.dict,
        _ => return,
    };
    // A table is a position for its entries and is nothing itself.
    let table = at == Position::GraphicsStateTable;
    let position = if table { Position::Anywhere } else { at };
    visit(&Site { id, position, dict });
    for (key, value) in dict.iter() {
        let child = if table {
            Position::GraphicsState
        } else {
            match key.as_bytes() {
                b"ExtGState" => Position::GraphicsStateTable,
                b"Annots" => Position::AnnotationArray,
                _ => Position::Anywhere,
            }
        };
        descend(value, id, child, deeper, visit);
    }
}

/// Which cross-referenced objects a resource dictionary's `ExtGState` table or a page's `Annots`
/// array names.
///
/// A pass of its own, because [`descend`] does not follow references and an indirect graphics
/// state is therefore reached at the top level, where the key its parent filed it under is out
/// of sight. Resolving the table itself is what makes an indirect *table* work too: the parent's
/// key is read through [`Document::get_key`], so a table written as its own object still hands
/// back its entries.
fn positions(exam: &Examination<'_>) -> BTreeMap<ObjectId, Position> {
    let document = exam.document;
    let mut found = BTreeMap::new();
    for number in document.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        collect_positions(document, &document.get(id), 0, &mut found);
    }
    found
}

/// One object's contribution to [`positions`].
fn collect_positions(
    document: &Document,
    object: &Object,
    depth: usize,
    found: &mut BTreeMap<ObjectId, Position>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let deeper = depth.saturating_add(1);
    let dict = match object {
        Object::Array(items) => {
            for item in items {
                collect_positions(document, item, deeper, found);
            }
            return;
        }
        Object::Dictionary(dict) => dict,
        Object::Stream(stream) => &stream.dict,
        _ => return,
    };
    if let Some(reference) = dict.get("ExtGState").and_then(Object::as_reference) {
        found.insert(reference, Position::GraphicsStateTable);
    }
    if let Object::Dictionary(table) = document.get_key(dict, "ExtGState") {
        for (_, value) in table.iter() {
            if let Some(reference) = value.as_reference() {
                found.insert(reference, Position::GraphicsState);
            }
        }
    }
    if let Some(reference) = dict.get("Annots").and_then(Object::as_reference) {
        found.insert(reference, Position::AnnotationArray);
    }
    if let Object::Array(annotations) = document.get_key(dict, "Annots") {
        for value in &annotations {
            if let Some(reference) = value.as_reference() {
                found.insert(reference, Position::Annotation);
            }
        }
    }
    for (_, value) in dict.iter() {
        collect_positions(document, value, deeper, found);
    }
}

/// Whether this dictionary describes an `XObject` of the given subtype.
fn is_xobject(document: &Document, dict: &Dictionary, subtype: &[u8]) -> bool {
    states_name(document, dict, "Subtype", subtype)
}

/// Whether this site is a graphics state parameter dictionary: ISO 32000-2 §8.4.5, Table 57.
///
/// Two routes because the `Type` entry is optional there — a dictionary that says so is one,
/// and so is anything a resource dictionary's `ExtGState` table names, which is the only way
/// the `gs` operator can reach one.
fn is_graphics_state(document: &Document, site: &Site<'_>) -> bool {
    site.position == Position::GraphicsState
        || states_name(document, site.dict, "Type", b"ExtGState")
}

/// Whether this dictionary is a halftone: ISO 32000-2 §10.6.5.
///
/// Identified by its own entries rather than by position, because a halftone is reached through
/// a graphics state's `HT`, through a type 5 halftone's colourant entries and through the
/// `Colorants` of neither — one required entry it always has is `HalftoneType`, and `Type` names
/// it where a producer wrote that instead.
fn is_halftone(document: &Document, dict: &Dictionary) -> bool {
    states(document, dict, "HalftoneType") || states_name(document, dict, "Type", b"Halftone")
}

/// The output intent dictionaries one `OutputIntents` array holds.
fn output_intents(document: &Document, holder: &Dictionary) -> Vec<(Where, Dictionary)> {
    let Object::Array(entries) = document.get_key(holder, "OutputIntents") else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| {
            let place = entry.as_reference().map_or_else(Where::file, Where::object);
            match document.resolve(entry) {
                Object::Dictionary(dict) => Some((place, dict)),
                _ => None,
            }
        })
        .collect()
}

/// Every `OutputIntents` array the document states, the catalog's first and then each page's.
///
/// ISO 32000-1 defines the entry only on the catalog, so for an ISO 19005-2 target the pages
/// contribute nothing; ISO 19005-4 section 6.2.3 is what adds the page-level array, and the two
/// rows that read pages are `only_four` for that reason.
fn output_intent_arrays(document: &Document) -> Vec<Vec<(Where, Dictionary)>> {
    let mut arrays = Vec::new();
    if let Ok(catalog) = document.catalog() {
        arrays.push(output_intents(document, &catalog));
    }
    arrays
}

/// Every page's own `OutputIntents` array.
fn page_output_intent_arrays(document: &Document) -> Vec<Vec<(Where, Dictionary)>> {
    let pages = Pages::new(document);
    (0..pages.len())
        .filter_map(|index| pages.get(index))
        .map(|page| output_intents(document, &page.dict))
        .filter(|array| !array.is_empty())
        .collect()
}

/// Whether an output intent is a PDF/A one: ISO 19005-2 section 6.2.3, ISO 19005-4 section 6.2.3.
fn is_pdfa_output_intent(document: &Document, intent: &Dictionary) -> bool {
    states_name(document, intent, "S", b"GTS_PDFA1")
}

/// ISO 19005-2 section 6.2.3, ISO 19005-4 section 6.2.3.
fn pdfa_output_intent_states_a_destination_profile(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    for array in output_intent_arrays(document) {
        for (place, intent) in array {
            if !is_pdfa_output_intent(document, &intent) {
                continue;
            }
            if !matches!(
                document.get_key(&intent, "DestOutputProfile"),
                Object::Stream(_)
            ) {
                findings.record(
                    place.named("DestOutputProfile"),
                    "a PDF/A output intent states no destination profile stream",
                );
            }
        }
    }
}

/// What an ICC profile's header says it is: its device class and its data colour space.
///
/// The two four-character signatures at offsets 12 and 16 of the 128-byte header.
/// `pdf_model::icc` reads the second of them at that offset to decide a profile's channel
/// count, and `pdf-model`'s own tests and its `press_census` example write and read the first
/// there — so this crate reads the header rather than inventing a parser, and the offsets are
/// the ones this tree already depends on.
fn profile_header(data: &[u8]) -> Option<(&[u8], &[u8])> {
    let class = data.get(12..16)?;
    let space = data.get(16..20)?;
    // A profile that does not carry the signature at offset 36 is not one at all, and saying
    // so here keeps a stream of arbitrary bytes from being judged on four of them.
    if data.get(36..40)? != b"acsp" {
        return None;
    }
    Some((class, space))
}

/// ISO 19005-2 section 6.2.3, ISO 19005-4 section 6.2.3.
///
/// The clause states the restriction of the profile that *is* a `DestOutputProfile` value,
/// without qualifying which output intent stated it — and the paragraph before it requires every
/// entry that states one to state the same object — so this reads every one it finds.
fn destination_profile_class_and_colour_space(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for array in output_intent_arrays(document) {
        check_destination_profiles(document, &array, findings);
    }
}

/// One array's destination profiles, for the row above and for the page-level row.
fn check_destination_profiles(
    document: &Document,
    array: &[(Where, Dictionary)],
    findings: &mut Findings,
) {
    for (place, intent) in array {
        let Object::Stream(stream) = document.get_key(intent, "DestOutputProfile") else {
            continue;
        };
        let place = place.clone().named("DestOutputProfile");
        let Some(data) = document.decoded_stream_data(&stream) else {
            findings.record(
                place,
                "a destination profile stream could not be decoded, so it is not a valid ICC \
                 profile",
            );
            continue;
        };
        let Some((class, space)) = profile_header(&data) else {
            findings.record(place, "a destination profile is not an ICC profile");
            continue;
        };
        if class != b"prtr" && class != b"mntr" {
            findings.record(
                place.clone(),
                "a destination profile is neither an output nor a monitor profile",
            );
        }
        if space != b"GRAY" && space != b"RGB " && space != b"CMYK" {
            findings.record(
                place,
                "a destination profile's colour space is not grey, RGB or CMYK",
            );
        }
    }
}

/// An edition of the ICC specification this project holds, identified by the profile version it
/// states as its own.
///
/// ISO 32000-2 §8.6.5.5 is what makes this the right key:
///
/// > Profiles shall conform to the specification version indicated by the Profile version number
/// > in its header.
///
/// So the header's version at bytes 8 to 11 names the text a profile is to be judged by, and this
/// enumerates the four texts that are here to judge with, each stating the version consistent with
/// itself: ICC.1:1998-09 section 6.1.3 gives 2.2.0, ICC.1:2001-12 section 6.1.3 gives 4.0.0,
/// ICC.1:2022 section 7.2.4 gives 4.4.0.0 and ICC.2:2023 section 7.2.6 gives 5.0.0.0. All four
/// clauses put the major version in byte 8 and the minor and bug-fix versions in the two halves of
/// byte 9, which is what `pdf_model::icc::Identification` reads.
///
/// **The match is exact rather than "4.4 or later" or "any version 2".** Each edition says which
/// number is consistent with *it*, and nothing held says what a 2.1.0 or a 4.1.0 profile would have
/// to satisfy — so those fall to the unchecked remainder rather than being judged by a neighbour.
/// `icc_profiles_carry_the_tags_a_permitted_edition_requires` asks the *other* question, which is
/// ISO 19005-2's own and not conditioned on the version at all.
///
/// The two bytes after the version are read by none of the four: the v2 and v4 texts reserve them
/// and ICC.2 gives them to a profile sub-class whose own specification is a separate document, and
/// a sub-class profile still owes everything ICC.2 requires, so ignoring them can only under-report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IccEdition {
    /// ICC.1:1998-09, profile version 2.2.0.
    Nineteen98,
    /// ICC.1:2001-12, profile version 4.0.0.
    TwoThousand1,
    /// ICC.1:2022, profile version 4.4.0.0.
    TwentyTwo,
    /// ICC.2:2023 — iccMAX — profile version 5.0.0.0.
    TwentyThree,
}

impl IccEdition {
    /// The edition a header's version names, where this project holds it.
    fn of(version: (u8, u8, u8)) -> Option<Self> {
        match version {
            (2, 2, 0) => Some(Self::Nineteen98),
            (4, 0, 0) => Some(Self::TwoThousand1),
            (4, 4, 0) => Some(Self::TwentyTwo),
            (5, 0, 0) => Some(Self::TwentyThree),
            _ => None,
        }
    }

    /// How a finding names the edition, so that a reader can check the verdict against a copy.
    fn name(self) -> &'static str {
        match self {
            Self::Nineteen98 => "ICC.1:1998-09",
            Self::TwoThousand1 => "ICC.1:2001-12",
            Self::TwentyTwo => "ICC.1:2022",
            Self::TwentyThree => "ICC.2:2023",
        }
    }

    /// Which clause of that edition names the required tags of a profile class.
    fn required_tag_clause(self) -> &'static str {
        match self {
            Self::Nineteen98 | Self::TwoThousand1 => "6.3",
            Self::TwentyTwo | Self::TwentyThree => "8",
        }
    }

    /// Which clause states the `Profile ID` field, where the edition states one this can compute.
    ///
    /// `None` for the two older texts, and for two different reasons that are both worth keeping:
    ///
    /// - **ICC.1:1998-09 has no such field.** Its Table 9 gives bytes 84 to 127 to "44 bytes
    ///   reserved for future expansion", and — unlike the reserved fields elsewhere in that
    ///   document, which say they must be set to zero — states no requirement on their contents at
    ///   all. So a v2 profile carrying sixteen non-zero bytes there breaks nothing that text says.
    /// - **ICC.1:2001-12 has the field and not the method.** Its section 6.1.13 puts the Profile ID
    ///   at bytes 84 to 99, but hands the details of the MD5 fingerprinting method to a technical
    ///   note on the ICC's web site rather than stating them, and the fields it names as zeroed for
    ///   the calculation are not the ones the later editions name: the rendering intent, the
    ///   *header attributes* and the Profile ID, where ICC.1:2022 and ICC.2:2023 zero the profile
    ///   *flags* instead of the attributes. `pdf_model::icc::computed_id` implements the later
    ///   method, which is the one ISO 19005-4 section 6.2.4.2 names — so applying it to a profile
    ///   claiming 4.0.0 would be judging that profile by a calculation its own edition does not
    ///   describe.
    fn profile_id_clause(self) -> Option<&'static str> {
        match self {
            Self::Nineteen98 | Self::TwoThousand1 => None,
            Self::TwentyTwo => Some("7.2.18"),
            Self::TwentyThree => Some("7.2.20"),
        }
    }
}

/// ISO 19005-2 section 6.2.3, ISO 19005-4 section 6.2.3, the `Profile ID` half of validity.
///
/// Both parts require the destination profile to be a valid ICC profile stream; ISO 32000-2
/// §8.6.5.5 sends validity to the edition the profile's own header names; and two of the four
/// editions held state the same requirement of the same sixteen bytes. ICC.1 section 7.2.18 and
/// ICC.2 section 7.2.20 each say that the field, if it is not zero, shall hold the Profile ID, and
/// that the Profile ID is RFC 1321's MD5 taken over the whole profile with three header fields
/// zeroed — which is `pdf_model::icc::computed_id`. A field that states something else is a profile
/// asserting an identity its own bytes do not give.
///
/// The other two editions state no such requirement this crate can apply, and
/// [`IccEdition::profile_id_clause`] says why for each.
///
/// A profile whose size field claims more bytes than the stream holds computes no identifier, and
/// is not reported here: what that profile violates is section 7.2.2's requirement on the size
/// field rather than this one, and section 7.2.2 belongs to the unchecked remainder.
fn destination_profile_states_a_correct_profile_id(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    for_each_destination_profile(exam.document, |place, edition, data| {
        let Some(clause) = edition.profile_id_clause() else {
            return;
        };
        let (Some(stated), Some(computed)) = (icc::stated_id(data), icc::computed_id(data)) else {
            return;
        };
        if stated != computed {
            findings.record(
                place,
                format!(
                    "a destination profile states a Profile ID that is not the MD5 of its own \
                     bytes, which {} section {clause} requires of a non-zero field",
                    edition.name(),
                ),
            );
        }
    });
}

/// ISO 19005-2 section 6.2.3, ISO 19005-4 section 6.2.3, the required-tag half of validity.
///
/// Each held edition lists the tags a profile of a given class shall contain, and
/// [`missing_required_tags`] is that list for the two classes a destination profile may be.
fn destination_profile_carries_the_tags_its_class_requires(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    for_each_destination_profile(exam.document, |place, edition, data| {
        let Some((class, _)) = profile_header(data) else {
            return;
        };
        for missing in missing_required_tags(edition, data, class) {
            findings.record(
                place.clone(),
                format!(
                    "a destination profile carries no {missing}, which {} clause {} requires of \
                     a profile of its class",
                    edition.name(),
                    edition.required_tag_clause(),
                ),
            );
        }
    });
}

/// The six tags a three-component matrix-and-curve profile is built from.
///
/// Named `redMatrixColumnTag`, `greenMatrixColumnTag` and `blueMatrixColumnTag` from ICC.1:2001-12
/// onward and `redColorantTag`, `greenColorantTag` and `blueColorantTag` in ICC.1:1998-09 — the
/// same four-character signatures under two names, which is why one array serves both.
const MATRIX_AND_CURVE_TAGS: [&[u8; 4]; 6] = [b"rXYZ", b"gXYZ", b"bXYZ", b"rTRC", b"gTRC", b"bTRC"];

/// What `edition` requires of a profile of `class` and this one does not carry.
///
/// The classes handled are the two ISO 19005 admits for a destination profile — `mntr` and `prtr`
/// — and the input class an `ICCBased` colour space may also be; a profile of any other class is
/// left to the row that reports the class itself.
///
/// # ICC.1:1998-09 and ICC.1:2001-12
///
/// Both state their required tags per class *and per form*, as a set of tables in clause 6.3, and
/// both begin from the same three: `profileDescriptionTag`, `mediaWhitePointTag` and
/// `copyrightTag` are in every device-class table of both documents. What the forms then add is
/// the transform, and a profile that is none of the forms its class offers is not a profile of
/// that class at all. ICC.1:2001-12 adds a `chromaticAdaptationTag` to each table on a condition —
/// that the actual illumination source is not D50 — which the bytes do not state, so it is not
/// among what is checked.
///
/// **The disjunction implemented is deliberately the weaker of the two documents' at each point**,
/// so that a profile conforming to either passes: ICC.1:1998-09's Table 25 gives a display profile
/// an `AToB0Tag` alone where ICC.1:2001-12's Table 26 wants a `BToA0Tag` beside it, and
/// ICC.1:1998-09's Table 29 and ICC.1:2001-12's Table 28 both want seven lookup tags of a colour
/// output profile where this asks for the first. Under-reporting is the standing direction of
/// error here, and the row that asks *which* of the two a profile conforms to would have to be a
/// different one.
///
/// # ICC.1:2022
///
/// Section 8.2 requires a `profileDescriptionTag`, a `copyrightTag` and a `mediaWhitePointTag` of
/// every profile but a `DeviceLink`, and a `chromaticAdaptationTag` on the same unstatable
/// condition. Section 8.4 then gives a display profile three forms and section 8.5 gives an output
/// profile two, each with its own required tags. Section 8.5.2's `colorantTableTag` is required
/// only for an xCLR data colour space, which a conforming destination profile cannot have, so it
/// is not checked either.
///
/// # ICC.2:2023
///
/// Section 8.2 requires the same first two tags, a `mediaWhitePointTag` where the header's PCS
/// field is not zero, and a `spectralWhitePointTag` where its spectral PCS field is not zero.
/// Sections 8.4 and 8.5 state one requirement for both classes rather than a set of forms: one or
/// more of the four `AToBx` or four `DToBx` tags, and one or more of the four `BToAx` or four
/// `BToDx` tags. The v2 and v4 matrix-and-curve form is not among them and iccMAX defines no such
/// tags, which is what the corpus's `6-2-3-t01-fail-d` turns on.
fn missing_required_tags(edition: IccEdition, data: &[u8], class: &[u8]) -> Vec<&'static str> {
    let has = |signature: &[u8; 4]| icc::has_tag(data, *signature);
    let any = |signatures: &[[u8; 4]]| signatures.iter().any(&has);
    let matrix = || MATRIX_AND_CURVE_TAGS.iter().all(|signature| has(signature));
    let mut missing = Vec::new();
    if !has(b"desc") {
        missing.push("profileDescriptionTag");
    }
    if !has(b"cprt") {
        missing.push("copyrightTag");
    }
    // Required outright by the three ICC.1 editions and conditionally by ICC.2, whose section 8.2
    // asks for it only where the header's PCS field is not zero — a field this crate does not
    // read, so a version 5 profile is not judged on it and the unchecked row says so.
    if edition != IccEdition::TwentyThree && !has(b"wtpt") {
        missing.push("mediaWhitePointTag");
    }
    match edition {
        IccEdition::Nineteen98 | IccEdition::TwoThousand1 => {
            // A monochrome form, a three-component matrix form, or a lookup-table form — the
            // three shapes clause 6.3 gives an input or a display profile, and the two it gives
            // an output profile.
            let curves_or_table = has(b"kTRC") || has(b"A2B0");
            match class {
                b"scnr" | b"mntr" if !(curves_or_table || matrix()) => missing.push(
                    "transform its class admits — a grayTRCTag, the six matrix and curve tags, \
                     or an AToB0Tag",
                ),
                b"prtr" if !curves_or_table => {
                    missing.push("transform its class admits — a grayTRCTag or an AToB0Tag");
                }
                _ => {}
            }
        }
        IccEdition::TwentyTwo => {
            // Section 8.4.2's N-component form, 8.4.3's three-component matrix form, 8.4.4's
            // monochrome form.
            let display = (has(b"A2B0") && has(b"B2A0")) || matrix() || has(b"kTRC");
            // Section 8.5.2's N-component form and 8.5.3's monochrome form.
            let output = [
                b"A2B0", b"A2B1", b"A2B2", b"B2A0", b"B2A1", b"B2A2", b"gamt",
            ]
            .iter()
            .all(|signature| has(signature))
                || has(b"kTRC");
            match class {
                b"mntr" if !display => missing.push(
                    "set of tags for any of section 8.4's three display profile forms — an \
                     AToB0Tag with a BToA0Tag, the six matrix and curve tags, or a grayTRCTag",
                ),
                b"prtr" if !output => missing.push(
                    "set of tags for either of section 8.5's two output profile forms — the six \
                     AToBx and BToAx tags with a gamutTag, or a grayTRCTag",
                ),
                _ => {}
            }
        }
        IccEdition::TwentyThree => {
            if !any(&[*b"A2B0", *b"A2B1", *b"A2B2", *b"A2B3"])
                && !any(&[*b"D2B0", *b"D2B1", *b"D2B2", *b"D2B3"])
            {
                missing.push(
                    "transform to the connection space — one of AToB0Tag to AToB3Tag or \
                     DToB0Tag to DToB3Tag",
                );
            }
            if !any(&[*b"B2A0", *b"B2A1", *b"B2A2", *b"B2A3"])
                && !any(&[*b"B2D0", *b"B2D1", *b"B2D2", *b"B2D3"])
            {
                missing.push(
                    "transform from the connection space — one of BToA0Tag to BToA3Tag or \
                     BToD0Tag to BToD3Tag",
                );
            }
        }
    }
    missing
}

/// Every destination profile the document's own output intents state, judged by a held edition.
///
/// The population is `output_intent_arrays`', which is the document catalog's array — the same
/// one `destination_profile_class_and_colour_space` reads, and for the same reason. A page's own
/// array is the business of `graphics/page-output-intents-have-the-same-shape`.
///
/// The visitor is handed only the profiles whose header names an edition this project holds; the
/// rest are the unchecked remainder and are not judged by a neighbouring text.
fn for_each_destination_profile(
    document: &Document,
    mut visit: impl FnMut(Where, IccEdition, &[u8]),
) {
    for array in output_intent_arrays(document) {
        for (place, intent) in array {
            let Object::Stream(stream) = document.get_key(&intent, "DestOutputProfile") else {
                continue;
            };
            let Some(data) = document.decoded_stream_data(&stream) else {
                continue;
            };
            let Some(stated) = icc::Identification::read(&data) else {
                continue;
            };
            let Some(edition) = IccEdition::of(stated.version) else {
                continue;
            };
            visit(place.named("DestOutputProfile"), edition, &data);
        }
    }
}

/// The device classes ISO 32000-2 §8.6.5.5's Table 67 admits for an `ICCBased` colour space.
///
/// An input, a display, an output and a colour space conversion profile. The classes it leaves
/// out are the ones that describe a transformation rather than a space — a device link, an
/// abstract profile, a named colour profile — and none of those can define the source colours
/// of a graphics object.
const ICC_DEVICE_CLASSES: [&[u8]; 4] = [b"scnr", b"mntr", b"prtr", b"spac"];

/// The data colour spaces that table admits, with the number of components each has.
///
/// The counts are the base standard's own: §8.6.5.5's table of ranges for typical ICC colour
/// spaces gives one component to grey, three to RGB and to L\*a\*b\*, and four to CMYK. They are
/// here because Table 65 requires `N` to match what the profile actually has, and the signature
/// in the profile's header is what says so.
const ICC_COLOUR_SPACES: [(&[u8], i64); 4] =
    [(b"GRAY", 1), (b"RGB ", 3), (b"CMYK", 4), (b"Lab ", 3)];

/// ISO 19005-4 section 6.2.4.2, first sentence.
///
/// # Why part 4's version of this rule is readable here and part 2's is not
///
/// ISO 19005-2 names four ICC texts and asks the profile to conform to one of them; this project
/// holds two of the four, which is enough for their required-tag lists and not for the rest of
/// either document, so that row stays unchecked with a reason saying which half is which.
/// **Part 4 states the same rule by deferring to ISO 32000-2 §8.6.5.5 instead**, and that clause
/// is in `doc/md/`. So the half a reader here can actually check is checked, and the halves are
/// separate rows rather than one row applying part 4's clause to a part 2 file.
///
/// Three things §8.6.5.5 states about the *file*:
///
/// - its Table 65 makes `N` required and gives it three valid values, 1, 3 or 4;
/// - the same entry requires that number to match the number of components the profile itself
///   has, which its header's data colour space signature says;
/// - its Table 67 lists the profile types a writer may use, by device class and data colour
///   space, and requires each field to hold one of the values listed for it.
///
/// What is deliberately *not* read here is the sentence asking a profile to conform to the
/// specification version its own header names. That sentence has a row of its own —
/// `graphics/icc-profiles-conform-to-the-version-they-name` — because answering it means opening
/// whichever ICC edition the profile happens to name, and this row would otherwise half-answer it.
///
/// A stream that does not decode, or that carries no ICC signature at offset 36, is passed over
/// rather than reported. Whether a stream decodes is section 6.1.7's subject, and a filter this
/// tree cannot yet decode would otherwise be announced to a user as a colour fault.
/// ISO 19005-2 section 6.2.4.2, first sentence, the half a version number settles.
///
/// The clause admits four texts: ICC.1:1998-09, ICC.1:2001-12, ICC.1:2003-09 and ISO 15076-1.
/// Two of them are here and two are not, so *conformance* to one is still the unchecked row
/// above — what the two held texts settle is their required-tag lists and the row beside this
/// one is where that is done. What a profile's header does settle is which text it claims, and
/// one claim is decidable against the texts that are here.
///
/// **A profile stating major version 5 claims ICC.2, and ICC.2 is not one of the four.** Its
/// section 7.2.6 states that 5.0.0.0 is the version consistent with iccMAX, and its section 1
/// places it beside ISO 15076-1 rather than inside it: a document based on ISO 15076-1 that
/// describes an expanded profile specification permitting greater flexibility and functionality
/// than ISO 15076-1, from which some types have been removed and to which others have been added.
/// A profile conforming to that document is therefore not a profile conforming to ISO 15076-1,
/// whatever edition of it part 2's undated reference reaches, nor to any of the three ICC.1 texts
/// that precede it.
///
/// # Why 4.4.0.0 is not reported, though ICC.1:2022 is equally absent from the four
///
/// It would be one clause too many. Part 2's reference to ISO 15076-1 is undated, and its clause 2
/// says an undated reference takes the latest edition of the referenced document; whether an
/// edition of ISO 15076-1 based on ICC.1:2022 exists is a fact about ISO's catalogue rather than
/// about any document this project holds. Version 5 needs no such fact, because ICC.2 is a
/// different document rather than a later edition of the same one. The rest belongs to the
/// unchecked row, which says so.
/// ISO 19005-2 section 6.2.4.2, first sentence, the half two held editions decide.
///
/// The clause is a disjunction over four documents — ICC.1:1998-09, ICC.1:2001-12, ICC.1:2003-09
/// and ISO 15076-1 — so a profile fails it only by conforming to *none* of them. Two of the four
/// are held, and each states, in its clause 6.3, the tags a profile of a given class shall carry.
/// A profile that carries neither text's list conforms to neither, and that is what is reported.
///
/// **The version number the profile states does not enter this row**, and that is the difference
/// between it and the section 6.2.3 rows beside it. Part 2 asks for conformance to one of four
/// documents; it does not ask a profile to *say* which. Nothing in either held text requires the
/// version field to carry that text's own number — ICC.1:1998-09 section 6.1.3 and ICC.1:2001-12
/// section 6.1.3 each state the number consistent with themselves and require nothing of a
/// profile's field — so a profile labelled 2.0.0 or 2.1.0 that carries what ICC.1:1998-09 requires
/// conforms to ICC.1:1998-09. That reading is what makes the shipped `data/icc/sRGB2014.icc`
/// admissible, and `doc/adr/0950` records it as a finding rather than leaving it implicit.
///
/// What this therefore does *not* answer is whether such a profile conforms in every other
/// respect, nor whether a profile failing both held texts might still conform to one of the two
/// that are not held. The unchecked row above carries both.
fn icc_profiles_carry_the_tags_a_permitted_edition_requires(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    for_each_colour_space(exam, b"ICCBased", |id, items| {
        let Some(entry) = items.get(1) else {
            return;
        };
        let Object::Stream(stream) = document.resolve(entry) else {
            return;
        };
        let Some(data) = document.decoded_stream_data(&stream) else {
            return;
        };
        let Some((class, _)) = profile_header(&data) else {
            return;
        };
        let under_1998 = missing_required_tags(IccEdition::Nineteen98, &data, class);
        if under_1998.is_empty()
            || missing_required_tags(IccEdition::TwoThousand1, &data, class).is_empty()
        {
            return;
        }
        for missing in under_1998 {
            findings.record(
                Where::object(id).named("ICCBased"),
                format!(
                    "an ICCBased colour space's profile carries no {missing}, so it conforms to \
                     neither ICC.1:1998-09 nor ICC.1:2001-12 — the two of this clause's four \
                     editions this crate can read",
                ),
            );
        }
    });
}

/// ISO 19005-4 section 6.2.4.2, by way of ISO 32000-2 §8.6.5.5's closing sentence.
///
/// Part 4 states no list of admitted editions; it requires the profile to conform to
/// ISO 32000-2 §8.6.5.5, and that clause requires a profile to conform to the specification version
/// its own header names. So the version *selects* the text here, where under part 2 it selects
/// nothing — which is why this row and the part 2 one beside it are two rows rather than one, and
/// why they disagree about a profile stating 2.1.0: part 2 asks whether it conforms to one of four
/// documents and this asks what its own edition requires, which for 2.1.0 is a text not held.
fn icc_profiles_carry_the_tags_their_version_requires(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    for_each_colour_space(exam, b"ICCBased", |id, items| {
        let Some(entry) = items.get(1) else {
            return;
        };
        let Object::Stream(stream) = document.resolve(entry) else {
            return;
        };
        let Some(data) = document.decoded_stream_data(&stream) else {
            return;
        };
        let Some(stated) = icc::Identification::read(&data) else {
            return;
        };
        let (Some(edition), Some((class, _))) =
            (IccEdition::of(stated.version), profile_header(&data))
        else {
            return;
        };
        for missing in missing_required_tags(edition, &data, class) {
            findings.record(
                Where::object(id).named("ICCBased"),
                format!(
                    "an ICCBased colour space's profile carries no {missing}, which {} clause {} \
                     requires of a profile of its class",
                    edition.name(),
                    edition.required_tag_clause(),
                ),
            );
        }
    });
}

fn icc_profiles_claim_a_permitted_edition(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_colour_space(exam, b"ICCBased", |id, items| {
        let Some(entry) = items.get(1) else {
            return;
        };
        let Object::Stream(stream) = document.resolve(entry) else {
            return;
        };
        let Some(data) = document.decoded_stream_data(&stream) else {
            return;
        };
        let Some(stated) = icc::Identification::read(&data) else {
            return;
        };
        let (major, minor, bug_fix) = stated.version;
        if major >= 5 {
            findings.record(
                Where::object(id).named("ICCBased"),
                format!(
                    "an ICCBased colour space's profile states version {major}.{minor}.{bug_fix}, \
                     which names ICC.2's iccMAX — a specification beside the four this part \
                     admits, not an edition of one of them",
                ),
            );
        }
    });
}

fn icc_profiles_conform_to_the_base_standard(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_colour_space(exam, b"ICCBased", |id, items| {
        let Some(entry) = items.get(1) else {
            return;
        };
        let Object::Stream(stream) = document.resolve(entry) else {
            return;
        };
        let place = Where::object(id).named("ICCBased");
        let stated = document.get_key(&stream.dict, "N").as_integer();
        if !matches!(stated, Some(1 | 3 | 4)) {
            findings.record(
                place.clone(),
                "an ICCBased colour space states no N of 1, 3 or 4",
            );
        }
        let Some(data) = document.decoded_stream_data(&stream) else {
            return;
        };
        let Some((class, space)) = profile_header(&data) else {
            return;
        };
        if !ICC_DEVICE_CLASSES.contains(&class) {
            findings.record(
                place.clone(),
                "an ICCBased colour space's profile is of a device class the base standard does \
                 not admit for one",
            );
        }
        let Some(components) = ICC_COLOUR_SPACES
            .iter()
            .find(|(signature, _)| *signature == space)
            .map(|(_, count)| *count)
        else {
            findings.record(
                place,
                "an ICCBased colour space's profile states a data colour space the base standard \
                 does not admit for one",
            );
            return;
        };
        if stated.is_some_and(|count| count != components) {
            findings.record(
                place,
                "an ICCBased colour space's N is not the number of components its profile has",
            );
        }
    });
}

/// Visits every colour space array of one family the file's cross-referenced objects state.
///
/// The population is [`crate::Examination::objects`]'s and for its reason: ISO 19005-2 section
/// 6.1.4 and ISO 19005-4 section 6.1.4 exempt an indirect object no cross-reference section names,
/// so the objects that table reaches are what a requirement binds. References are not followed,
/// because the loop visits every one of those objects itself.
///
/// **A structural population rather than the content survey's**, which is a choice the two rules
/// reading this share with [`separations_of_one_name_agree`] beside them: each is about how a
/// colour space is *defined* rather than about a painting operator, so the space is judged
/// wherever the file states it.
fn for_each_colour_space(
    exam: &Examination<'_>,
    family: &[u8],
    mut visit: impl FnMut(ObjectId, &[Object]),
) {
    let document = exam.document;
    for (id, object) in exam.objects() {
        descend_for_family(document, object, *id, family, 0, &mut visit);
    }
}

/// One object's contribution to [`for_each_colour_space`].
fn descend_for_family(
    document: &Document,
    object: &Object,
    id: ObjectId,
    family: &[u8],
    depth: usize,
    visit: &mut impl FnMut(ObjectId, &[Object]),
) {
    if depth > MAX_DEPTH {
        return;
    }
    let deeper = depth.saturating_add(1);
    match object {
        Object::Array(items) => {
            let first = items.first().map(|item| document.resolve(item));
            if first.is_some_and(|first| {
                first
                    .as_name()
                    .is_some_and(|name| name.as_bytes() == family)
            }) {
                visit(id, items);
            }
            for item in items {
                descend_for_family(document, item, id, family, deeper, visit);
            }
        }
        Object::Dictionary(dict) => {
            for (_, value) in dict.iter() {
                descend_for_family(document, value, id, family, deeper, visit);
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter() {
                descend_for_family(document, value, id, family, deeper, visit);
            }
        }
        _ => {}
    }
}

/// ISO 19005-2 section 6.2.3.
///
/// Part 2 forbids the key in a *PDF/X* output intent, which is the one identified by a
/// `GTS_PDFX` subtype; ISO 19005-4 widened it to every output intent, and that is the row below.
fn no_destination_profile_reference_in_a_pdfx_output_intent(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    for array in output_intent_arrays(document) {
        for (place, intent) in array {
            if states_name(document, &intent, "S", b"GTS_PDFX")
                && states(document, &intent, "DestOutputProfileRef")
            {
                findings.record(
                    place.named("DestOutputProfileRef"),
                    "a PDF/X output intent names a profile outside the file",
                );
            }
        }
    }
}

/// ISO 19005-4 section 6.2.3.
fn no_destination_profile_reference(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let arrays = output_intent_arrays(document)
        .into_iter()
        .chain(page_output_intent_arrays(document));
    for array in arrays {
        for (place, intent) in array {
            if states(document, &intent, "DestOutputProfileRef") {
                findings.record(
                    place.named("DestOutputProfileRef"),
                    "an output intent names a profile outside the file",
                );
            }
        }
    }
}

/// ISO 19005-2 section 6.2.3, ISO 19005-4 section 6.2.3.
fn one_destination_profile_per_output_intents_array(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    for array in output_intent_arrays(document) {
        check_one_destination_profile(&array, findings);
    }
}

/// One array's destination profile references, for the row above and for the page-level row.
///
/// The clause requires the *same indirect object*, so a destination profile written as a direct
/// stream fails it as surely as two different objects do — there is no object for a second entry
/// to share.
fn check_one_destination_profile(array: &[(Where, Dictionary)], findings: &mut Findings) {
    if array.len() < 2 {
        return;
    }
    let mut first: Option<ObjectId> = None;
    for (place, intent) in array {
        let Some(stated) = intent.get("DestOutputProfile") else {
            continue;
        };
        let place = place.clone().named("DestOutputProfile");
        let Some(id) = stated.as_reference() else {
            findings.record(
                place,
                "an output intent states its destination profile directly, so the other entries \
                 in the array cannot share the object",
            );
            continue;
        };
        match first {
            None => first = Some(id),
            Some(shared) if shared == id => {}
            Some(_) => findings.record(
                place,
                "two entries of one OutputIntents array state different destination profile \
                 objects",
            ),
        }
    }
}

/// ISO 19005-4 section 6.2.3.
fn page_output_intents_have_the_same_shape(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for array in page_output_intent_arrays(document) {
        for (place, intent) in &array {
            if is_pdfa_output_intent(document, intent)
                && !matches!(
                    document.get_key(intent, "DestOutputProfile"),
                    Object::Stream(_)
                )
            {
                findings.record(
                    place.clone().named("DestOutputProfile"),
                    "a page-level PDF/A output intent states no destination profile stream",
                );
            }
        }
        check_destination_profiles(document, &array, findings);
        check_one_destination_profile(&array, findings);
    }
}

/// A Separation colour space array, as ISO 32000-2 §8.6.6.4 shapes it.
///
/// The four elements are the family name, the colourant name, the alternate space and the tint
/// transform; this keeps the last three, since the first is what identified it.
#[derive(Debug)]
struct Separation {
    /// Where the array was found.
    place: Where,
    /// The colourant name, which is what ISO 19005 groups these by.
    colourant: Vec<u8>,
    /// The alternate space.
    alternate: Object,
    /// The tint transform.
    transform: Object,
}

/// Every Separation colour space array the file's cross-referenced objects state.
fn separations(exam: &Examination<'_>) -> Vec<Separation> {
    let document = exam.document;
    let mut found = Vec::new();
    for number in document.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        collect_separations(document, &document.get(id), id, 0, &mut found);
    }
    found
}

/// One object's contribution to [`separations`].
fn collect_separations(
    document: &Document,
    object: &Object,
    id: ObjectId,
    depth: usize,
    found: &mut Vec<Separation>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let deeper = depth.saturating_add(1);
    match object {
        Object::Array(items) => {
            if let Some(separation) = as_separation(document, items, id) {
                found.push(separation);
            }
            for item in items {
                collect_separations(document, item, id, deeper, found);
            }
        }
        Object::Dictionary(dict) => {
            for (_, value) in dict.iter() {
                collect_separations(document, value, id, deeper, found);
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter() {
                collect_separations(document, value, id, deeper, found);
            }
        }
        _ => {}
    }
}

/// Reads an array as a Separation colour space, or `None` where it is not one.
fn as_separation(document: &Document, items: &[Object], id: ObjectId) -> Option<Separation> {
    let family = document.resolve(items.first()?);
    if family.as_name()?.as_bytes() != b"Separation" {
        return None;
    }
    let colourant = document.resolve(items.get(1)?);
    let colourant = colourant.as_name()?.as_bytes().to_vec();
    Some(Separation {
        place: Where::object(id).named(String::from_utf8_lossy(&colourant).into_owned()),
        colourant,
        alternate: items.get(2)?.clone(),
        transform: items.get(3)?.clone(),
    })
}

/// ISO 19005-2 section 6.2.4.4, ISO 19005-4 section 6.2.4.4.
///
/// The clause is explicit about how the comparison is made: the PDF objects are compared rather
/// than what evaluating them would produce, and neither compression nor indirection counts. So
/// [`equivalent`] resolves references, decodes stream data and ignores the stream dictionary
/// entries that describe the encoding rather than the value.
fn separations_of_one_name_agree(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let mut first: BTreeMap<Vec<u8>, Separation> = BTreeMap::new();
    for separation in separations(exam) {
        match first.get(&separation.colourant) {
            None => {
                first.insert(separation.colourant.clone(), separation);
            }
            Some(earlier) => {
                if !equivalent(document, &earlier.alternate, &separation.alternate, 0) {
                    findings.record(
                        separation.place.clone(),
                        "two Separation arrays name one colourant and state different alternate \
                         spaces",
                    );
                } else if !equivalent(document, &earlier.transform, &separation.transform, 0) {
                    findings.record(
                        separation.place.clone(),
                        "two Separation arrays name one colourant and state different tint \
                         transforms",
                    );
                }
            }
        }
    }
}

/// The `DeviceN` component names that are never a spot colourant.
///
/// Four of them because ISO 32000-2 §8.6.6.5 reserves the names `Cyan`, `Magenta`, `Yellow` and
/// `Black` to name the subtractive process colourants of a CMYK device, and a reserved name is a
/// process component wherever it appears rather than only inside an `NChannel` space.
///
/// `None` is the fifth, and it is here for a different reason: that clause gives the name to a
/// component that shall never be painted on the page, so there is no colourant for a `Colorants`
/// entry to describe. `All` cannot appear at all — §8.6.6.5 forbids it in a `DeviceN` space.
const NEVER_SPOT: [&[u8]; 5] = [b"Cyan", b"Magenta", b"Yellow", b"Black", b"None"];

/// ISO 19005-2 section 6.2.4.4, ISO 19005-4 section 6.2.4.4.
///
/// # Which components are spot, which was thought to be undecidable and is not
///
/// Neither part of ISO 19005 defines a spot colourant, and this row was unchecked on the reading
/// that ISO 32000-2 §8.6.6.5 draws the line only through an `NChannel` space's process
/// dictionary. It draws it twice more, and both are general:
///
/// - it *reserves* four names — `Cyan`, `Magenta`, `Yellow`, `Black` — to the subtractive
///   process colourants of a CMYK device, in the general discussion of the `names` array rather
///   than in the `NChannel` restrictions;
/// - and where a process dictionary is present it settles the rest by exclusion:
///
/// > Any component not specified in the process dictionary shall be considered to be a spot
/// > colourant.
///
/// So a component is a spot colourant unless it is one of the four reserved process names, or is
/// listed in the process dictionary's `Components`, or is `None` — which §8.6.6.5 gives to a
/// component that is never painted and which therefore names no colourant. That is the standard
/// deciding it, not this crate choosing.
///
/// The `Process` dictionary is read whether or not the `Subtype` is `NChannel`. The base
/// standard requires it only there, but a plain `DeviceN` that states one has said which of its
/// components are process components, and believing it can only reduce what is reported.
fn spot_colourants_appear_in_the_colorants_dictionary(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    for_each_colour_space(exam, b"DeviceN", |id, items| {
        let Some(names) = items.get(1).map(|entry| document.resolve(entry)) else {
            return;
        };
        let Some(names) = names.as_array() else {
            return;
        };
        let attributes = items.get(4).map(|entry| document.resolve(entry));
        let attributes = attributes.as_ref().and_then(Object::as_dict);
        let colorants = attributes.map(|dict| document.get_key(dict, "Colorants"));
        let colorants = colorants.as_ref().and_then(Object::as_dict);
        let process = attributes.map_or_else(Vec::new, |dict| process_components(document, dict));
        for entry in names {
            let colourant = document.resolve(entry);
            let Some(colourant) = colourant.as_name().map(|name| name.as_bytes().to_vec()) else {
                continue;
            };
            if NEVER_SPOT.contains(&colourant.as_slice()) || process.contains(&colourant) {
                continue;
            }
            let defined = colorants.is_some_and(|dict| {
                dict.get_by_name(&pdf_syntax::Name::new(colourant.clone()))
                    .is_some()
            });
            if !defined {
                findings.record(
                    Where::object(id).named(String::from_utf8_lossy(&colourant).into_owned()),
                    "a DeviceN colour space uses a spot colourant its Colorants dictionary does \
                     not define",
                );
            }
        }
    });
}

/// The component names a `DeviceN` attributes dictionary's process dictionary claims.
///
/// ISO 32000-2 §8.6.6.5's Table 71 makes `Components` an array of names that correspond, in
/// order, to the components of the process colour space — and the names may be arbitrary, which
/// is exactly why they have to be read rather than guessed at.
fn process_components(document: &Document, attributes: &Dictionary) -> Vec<Vec<u8>> {
    let process = document.get_key(attributes, "Process");
    let Some(process) = process.as_dict() else {
        return Vec::new();
    };
    let listed = document.get_key(process, "Components");
    let Some(listed) = listed.as_array() else {
        return Vec::new();
    };
    listed
        .iter()
        .filter_map(|entry| {
            document
                .resolve(entry)
                .as_name()
                .map(|name| name.as_bytes().to_vec())
        })
        .collect()
}

/// Whether two objects are the same PDF object once indirection and compression are set aside.
///
/// Numbers compare by value, so a producer writing `0` where another wrote `0.0` is not a
/// difference — a deliberate reading of a clause that contrasts comparing the objects with
/// comparing what using them computes, and the reading that cannot manufacture a failure.
fn equivalent(document: &Document, left: &Object, right: &Object, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return true;
    }
    let deeper = depth.saturating_add(1);
    let (left, right) = (document.resolve(left), document.resolve(right));
    if let (Some(one), Some(other)) = (left.as_number(), right.as_number()) {
        return one.to_bits() == other.to_bits();
    }
    match (&left, &right) {
        (Object::Array(one), Object::Array(other)) => {
            one.len() == other.len()
                && one
                    .iter()
                    .zip(other.iter())
                    .all(|(a, b)| equivalent(document, a, b, deeper))
        }
        (Object::Dictionary(one), Object::Dictionary(other)) => {
            equivalent_dictionaries(document, one, other, deeper, &[])
        }
        (Object::Stream(one), Object::Stream(other)) => {
            equivalent_dictionaries(document, &one.dict, &other.dict, deeper, ENCODING_KEYS)
                && document.decoded_stream_data(one) == document.decoded_stream_data(other)
        }
        _ => left == right,
    }
}

/// The stream dictionary entries that describe the encoding rather than the value.
///
/// Ignored when two tint transforms are compared, because ISO 19005 says compression does not
/// count and these are what a different compression changes.
const ENCODING_KEYS: &[&str] = &["Filter", "DecodeParms", "DecodeParams", "Length", "DL"];

/// Whether two dictionaries agree, ignoring the named keys.
fn equivalent_dictionaries(
    document: &Document,
    left: &Dictionary,
    right: &Dictionary,
    depth: usize,
    ignore: &[&str],
) -> bool {
    let kept = |dict: &Dictionary| -> Vec<Vec<u8>> {
        dict.iter()
            .map(|(key, _)| key.as_bytes().to_vec())
            .filter(|key| !ignore.iter().any(|skip| skip.as_bytes() == key.as_slice()))
            .collect()
    };
    let keys = kept(left);
    if keys != kept(right) {
        return false;
    }
    keys.iter().all(|key| {
        let name = pdf_syntax::Name::new(key.clone());
        match (left.get_by_name(&name), right.get_by_name(&name)) {
            (Some(one), Some(other)) => equivalent(document, one, other, depth),
            _ => false,
        }
    })
}

/// ISO 19005-2 section 6.2.5, ISO 19005-4 section 6.2.5.
fn no_transfer_function_in_a_graphics_state(exam: &Examination<'_>, findings: &mut Findings) {
    forbidden_graphics_state_key(exam, findings, "TR");
}

/// ISO 19005-2 section 6.2.5.
fn no_halftone_phase_in_a_graphics_state(exam: &Examination<'_>, findings: &mut Findings) {
    forbidden_graphics_state_key(exam, findings, "HTP");
}

/// ISO 19005-4 section 6.2.5.
fn no_halftone_origin_in_a_graphics_state(exam: &Examination<'_>, findings: &mut Findings) {
    forbidden_graphics_state_key(exam, findings, "HTO");
}

/// One key no graphics state parameter dictionary may state.
fn forbidden_graphics_state_key(
    exam: &Examination<'_>,
    findings: &mut Findings,
    key: &'static str,
) {
    let document = exam.document;
    for_each_dictionary(exam, |site| {
        if is_graphics_state(document, site) && states(document, site.dict, key) {
            findings.record(
                Where::object(site.id).named(key),
                "a graphics state parameter dictionary states a key ISO 19005 forbids",
            );
        }
    });
}

/// ISO 19005-2 section 6.2.5, ISO 19005-4 section 6.2.5.
fn second_transfer_function_is_default(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_dictionary(exam, |site| {
        if !is_graphics_state(document, site) || !states(document, site.dict, "TR2") {
            return;
        }
        if !states_name(document, site.dict, "TR2", b"Default") {
            findings.record(
                Where::object(site.id).named("TR2"),
                "a graphics state parameter dictionary states a TR2 other than Default",
            );
        }
    });
}

/// ISO 19005-2 section 6.2.5, ISO 19005-4 section 6.2.5.
fn halftone_type_is_one_or_five(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_dictionary(exam, |site| {
        if !is_halftone(document, site.dict) {
            return;
        }
        let stated = document.get_key(site.dict, "HalftoneType").as_integer();
        if !matches!(stated, Some(1 | 5)) {
            findings.record(
                Where::object(site.id).named("HalftoneType"),
                "a halftone is of a type other than 1 or 5",
            );
        }
    });
}

/// ISO 19005-2 section 6.2.5, ISO 19005-4 section 6.2.5.
fn no_halftone_name(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_dictionary(exam, |site| {
        if is_halftone(document, site.dict) && states(document, site.dict, "HalftoneName") {
            findings.record(
                Where::object(site.id).named("HalftoneName"),
                "a halftone states a HalftoneName",
            );
        }
    });
}

/// The colourant names ISO 32000-2 §10.6.5.6 makes primary components of a standard device space.
///
/// That clause divides a type 5 halftone's keys into exactly two categories and lists the first
/// of them outright: grey for `DeviceGray`, red, green and blue for `DeviceRGB`, and cyan,
/// magenta, yellow and black for `DeviceCMYK`. Everything else it puts in the second category,
/// nonstandard components used as spot colourants — which is the division the transfer function
/// requirement turns on, and the reason this row is answerable at all.
const STANDARD_PRIMARY_COLOURANTS: [&[u8]; 8] = [
    b"Gray", b"Red", b"Green", b"Blue", b"Cyan", b"Magenta", b"Yellow", b"Black",
];

/// The keys of a type 5 halftone that name something other than one of its colourants.
///
/// ISO 32000-2 §10.6.5.6's Table 132: the three that describe the halftone itself, and
/// `Default`, which the clause treats separately and this rule does not judge.
const TYPE_FIVE_NON_COLOURANT_KEYS: [&[u8]; 4] =
    [b"Type", b"HalftoneType", b"HalftoneName", b"Default"];

/// ISO 19005-2 section 6.2.5, ISO 19005-4 section 6.2.5.
///
/// # What "only as required" resolves to
///
/// ISO 19005 says the entry shall be used only as the base standard requires one, so the whole
/// rule is where ISO 32000-2 requires it. Its halftone tables require it of a dictionary that is
/// a component of a type 5 halftone and represents a nonprimary or nonstandard primary colour
/// component, and nowhere else — so the entry is *forbidden* everywhere else, which is the half
/// of the rule that catches a transfer function in a halftone that is the current halftone
/// parameter rather than one colourant's.
///
/// §10.6.5.6 supplies the missing list, which is why this row moved off `Unchecked`: it names
/// the primary components of the standard device colour spaces, and puts every other key of a
/// type 5 halftone in the other category.
///
/// # Two things this rule deliberately does not judge
///
/// **The `Default` entry.** §10.6.5.6 requires it to have a transfer function where the halftone
/// has any nonprimary colourant, and says nothing about the case where it has none. Reading
/// ISO 19005's "only as required" onto that silence would manufacture a failure out of an entry
/// the base standard permits, and the direction of error this crate keeps is to under-report.
///
/// **A halftone reached by neither route.** The requirement is about a halftone's *position* —
/// a type 5 component, or the current halftone parameter — and a dictionary that looks like a
/// halftone somewhere else is not in a position to require anything.
///
/// # Where this reads the standard against the corpus
///
/// veraPDF expects a type 5 halftone naming `Red`, `Green` and `Blue` to carry transfer
/// functions and fails one that does not. §10.6.5.6 lists those three among the primary
/// components of `DeviceRGB`, so under the base standard they are exactly the case where the
/// entry is *not* required — and ISO 19005 permits it only where it is. The disagreement is
/// recorded rather than tuned away; it does not reach a PDF/A-4 verdict at all, because
/// `crate::errata`'s issue #314 withdrew this provision from part 4.
fn halftone_transfer_function_only_where_required(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_dictionary(exam, |site| {
        if is_halftone(document, site.dict)
            && document.get_key(site.dict, "HalftoneType").as_integer() == Some(5)
        {
            for (key, value) in site.dict.iter() {
                let key = key.as_bytes();
                if TYPE_FIVE_NON_COLOURANT_KEYS.contains(&key) {
                    continue;
                }
                let component = document.resolve(value);
                let Some(component) = halftone_dictionary(&component) else {
                    continue;
                };
                let stated = states(document, component, "TransferFunction");
                let place =
                    || Where::object(site.id).named(String::from_utf8_lossy(key).into_owned());
                if STANDARD_PRIMARY_COLOURANTS.contains(&key) {
                    if stated {
                        findings.record(
                            place(),
                            "a type 5 halftone states a TransferFunction for a colourant that is \
                             a primary component of a standard device colour space, where the \
                             base standard requires none",
                        );
                    }
                } else if !stated {
                    findings.record(
                        place(),
                        "a type 5 halftone omits the TransferFunction the base standard requires \
                         of a component representing a nonstandard colourant",
                    );
                }
            }
        }
        if is_graphics_state(document, site) {
            let current = document.get_key(site.dict, "HT");
            if halftone_dictionary(&current)
                .is_some_and(|halftone| states(document, halftone, "TransferFunction"))
            {
                findings.record(
                    Where::object(site.id).named("HT"),
                    "a halftone set as the current halftone parameter states a TransferFunction, \
                     which the base standard requires only of a type 5 halftone's component",
                );
            }
        }
    });
}

/// A halftone as its dictionary, whether it was written as one or as a stream.
///
/// ISO 32000-2 §10.6.5 gives types 6, 10 and 16 a stream of threshold values, so a type 5
/// halftone's component and a graphics state's `HT` are each reached as either shape.
fn halftone_dictionary(object: &Object) -> Option<&Dictionary> {
    match object {
        Object::Dictionary(dict) => Some(dict),
        Object::Stream(stream) => Some(&stream.dict),
        _ => None,
    }
}

/// ISO 19005-2 section 6.2.6.
///
/// The two places a *dictionary* states an intent: a graphics state's `RI` and an image's
/// `Intent`. The `ri` operator's operand is the third, and it is a row of its own because it
/// lives in the content stream.
///
/// An inline image is an image dictionary that is not an object, so `crate::survey` supplies
/// those: §8.9.7 gives `Intent` no abbreviation, so the key is spelled the one way.
fn rendering_intent_entries_name_one_of_four(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for (page, image) in exam.survey().inline_images() {
        if !states(document, image, "Intent") {
            continue;
        }
        if !name_of(document, image, "Intent")
            .is_some_and(|name| RENDERING_INTENTS.contains(&name.as_slice()))
        {
            findings.record(
                Where::page(*page).named("Intent"),
                "an inline image states a rendering intent that is not one of the four \
                 ISO 32000 defines",
            );
        }
    }
    for_each_dictionary(exam, |site| {
        let key = if is_graphics_state(document, site) {
            "RI"
        } else if is_xobject(document, site.dict, b"Image") {
            "Intent"
        } else {
            return;
        };
        if site.dict.get(key).is_none() {
            return;
        }
        let stated = name_of(document, site.dict, key);
        if !stated.is_some_and(|name| RENDERING_INTENTS.contains(&name.as_slice())) {
            findings.record(
                Where::object(site.id).named(key),
                "a rendering intent is not one of the four ISO 32000 defines",
            );
        }
    });
}

/// ISO 19005-2 section 6.2.8.1, ISO 19005-4 section 6.2.7.1.
fn no_image_alternates_or_opi(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_dictionary(exam, |site| {
        if !is_xobject(document, site.dict, b"Image") {
            return;
        }
        for key in ["Alternates", "OPI"] {
            if states(document, site.dict, key) {
                findings.record(
                    Where::object(site.id).named(key),
                    "an image dictionary states a key ISO 19005 forbids",
                );
            }
        }
    });
}

/// ISO 19005-2 section 6.2.8.1, ISO 19005-4 section 6.2.7.1.
fn image_interpolation_is_off(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_dictionary(exam, |site| {
        if !is_xobject(document, site.dict, b"Image") || !states(document, site.dict, "Interpolate")
        {
            return;
        }
        if document.get_key(site.dict, "Interpolate") != Object::Boolean(false) {
            findings.record(
                Where::object(site.id).named("Interpolate"),
                "an image dictionary states an Interpolate other than false",
            );
        }
    });
}

/// ISO 19005-2 section 6.2.9.1, ISO 19005-4 section 6.2.8.1.
fn no_form_xobject_opi(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_dictionary(exam, |site| {
        if is_xobject(document, site.dict, b"Form") && states(document, site.dict, "OPI") {
            findings.record(
                Where::object(site.id).named("OPI"),
                "a form XObject states the OPI key",
            );
        }
    });
}

/// ISO 19005-2 section 6.2.9.1.
fn no_postscript_passthrough_in_a_form_xobject(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_dictionary(exam, |site| {
        if !is_xobject(document, site.dict, b"Form") {
            return;
        }
        if states(document, site.dict, "PS") {
            findings.record(
                Where::object(site.id).named("PS"),
                "a form XObject carries a PostScript stream",
            );
        }
        if states_name(document, site.dict, "Subtype2", b"PS") {
            findings.record(
                Where::object(site.id).named("Subtype2"),
                "a form XObject states a Subtype2 of PS",
            );
        }
    });
}

/// ISO 19005-2 section 6.2.9.2, ISO 19005-4 section 6.2.8.2.
///
/// ISO 32000-2 §8.10.4 makes a reference `XObject` a form `XObject` carrying a `Ref` entry, so that
/// entry is what the file is searched for.
fn no_reference_xobjects(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_dictionary(exam, |site| {
        if is_xobject(document, site.dict, b"Form") && states(document, site.dict, "Ref") {
            findings.record(
                Where::object(site.id).named("Ref"),
                "a form XObject names content in another file",
            );
        }
    });
}

/// ISO 19005-2 section 6.2.9.3.
fn no_postscript_xobjects(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_dictionary(exam, |site| {
        if is_xobject(document, site.dict, b"PS") {
            findings.record(
                Where::object(site.id).named("PS"),
                "the file contains a PostScript XObject",
            );
        }
    });
}

/// ISO 19005-2 section 6.2.10, ISO 19005-4 section 6.2.9.
fn graphics_state_blend_modes_are_defined(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_dictionary(exam, |site| {
        if is_graphics_state(document, site) {
            check_blend_mode(
                document,
                site,
                findings,
                "a graphics state parameter dictionary sets a blend mode ISO 32000 does not \
                 define",
            );
        }
    });
}

/// ISO 19005-4 section 6.2.9.
fn annotation_blend_modes_are_defined(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_dictionary(exam, |site| {
        if site.position == Position::Annotation {
            check_blend_mode(
                document,
                site,
                findings,
                "an annotation sets a blend mode ISO 32000 does not define",
            );
        }
    });
}

/// One dictionary's `BM` entry.
///
/// A name is judged against the list. An *array* is ISO 32000's older form, whose whole purpose
/// is to offer a processor several names to pick a supported one from — so an unrecognised name
/// inside one is what the form is for, and only an array offering no defined mode at all is a
/// file that has set a blend mode the standard does not define.
fn check_blend_mode(
    document: &Document,
    site: &Site<'_>,
    findings: &mut Findings,
    what: &'static str,
) {
    let defined = |object: &Object| {
        object
            .as_name()
            .is_some_and(|name| BLEND_MODES.contains(&name.as_bytes()))
    };
    let stated = match site.dict.get("BM") {
        None => return,
        Some(_) => document.get_key(site.dict, "BM"),
    };
    let good = match &stated {
        Object::Array(names) => names.iter().any(|name| defined(&document.resolve(name))),
        other => defined(other),
    };
    if !good {
        findings.record(Where::object(site.id).named("BM"), what);
    }
}

// --------------------------------------------------------------------------------------------
// 6.2.4.3 / 6.2.9 / 6.2.10 — the rules that turn on what a page's content actually does.
//
// Every one of these reads `crate::survey`, which walks the content streams once and reports
// which device colour spaces they selected, under which resource dictionary, with which
// blending space in force, and which pages Annex Q's method finds transparency on.
// --------------------------------------------------------------------------------------------

/// What the PDF/A output intent in force says, as the colour rules need it.
///
/// Three answers rather than two, because "there is an output intent whose destination profile this
/// crate could not read" is not the same fact as "there is none" and must not license the same
/// conclusion. An unreadable profile licenses every family, so that a file whose profile is broken
/// is reported once — by the section 6.2.3 row that is about the profile — rather than again on
/// every colour it sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intent {
    /// No output intent identified as a PDF/A one.
    Absent,
    /// One is in effect, but what its destination profile is could not be established.
    Unknown,
    /// One is in effect, holding a destination profile of this colour space family.
    Profile(DeviceFamily),
}

impl Intent {
    /// Whether a PDF/A output intent is in effect at all, which is what section 6.2.4.3's
    /// `DeviceGray` sentence and section 6.2.9's transparency sentence each turn on.
    const fn present(self) -> bool {
        !matches!(self, Self::Absent)
    }

    /// Whether the intent holds a destination profile of this family.
    fn holds(self, family: DeviceFamily) -> bool {
        match self {
            Self::Absent => false,
            Self::Unknown => true,
            Self::Profile(found) => found == family,
        }
    }

    /// Whether this intent licenses a use of one device colour space.
    ///
    /// `DeviceGray` needs only that an intent is in effect; the other two need one of their own
    /// family. That asymmetry is the clause's, and its reason is spelled out in the same
    /// subclause: a grey colour has a defined conversion into whichever of the three profiles is
    /// there, and an RGB or CMYK colour does not.
    fn licenses(self, family: DeviceFamily) -> bool {
        match family {
            DeviceFamily::Gray => self.present(),
            other => self.holds(other),
        }
    }
}

/// What one `OutputIntents` array's PDF/A entry says.
fn intent_of(document: &Document, array: &[(Where, Dictionary)]) -> Intent {
    for (_, intent) in array {
        if !is_pdfa_output_intent(document, intent) {
            continue;
        }
        return destination_profile_family(document, intent)
            .map_or(Intent::Unknown, Intent::Profile);
    }
    Intent::Absent
}

/// Which of the three families a PDF/A output intent's destination profile is of.
fn destination_profile_family(document: &Document, intent: &Dictionary) -> Option<DeviceFamily> {
    let Object::Stream(stream) = document.get_key(intent, "DestOutputProfile") else {
        return None;
    };
    let data = document.decoded_stream_data(&stream)?;
    let (_, space) = profile_header(&data)?;
    [DeviceFamily::Gray, DeviceFamily::Rgb, DeviceFamily::Cmyk]
        .into_iter()
        .find(|family| family.profile_signature() == space)
}

/// The PDF/A output intent the document catalog states, which is the only one ISO 19005-2 knows.
fn document_intent(document: &Document) -> Intent {
    output_intent_arrays(document)
        .first()
        .map_or(Intent::Absent, |array| intent_of(document, array))
}

/// The PDF/A output intent each page states, which ISO 19005-4 section 6.2.3 adds.
fn page_intents(document: &Document) -> Vec<Intent> {
    let pages = Pages::new(document);
    (0..pages.len())
        .map(|index| {
            pages.get(index).map_or(Intent::Absent, |page| {
                intent_of(document, &output_intents(document, &page.dict))
            })
        })
        .collect()
}

/// ISO 19005-4 section 6.2.3's current PDF/A output intent for one page: the page's own where it
/// states one, and otherwise the document's.
fn current_intent(document_level: Intent, page_level: Option<&Intent>) -> Intent {
    match page_level {
        Some(&stated) if stated.present() => stated,
        _ => document_level,
    }
}

/// Whether §8.6.5.6's default colour space licenses a device colour under ISO 19005-2.
///
/// The `DeviceCMYK` sentence of ISO 19005-2 section 6.2.4.3 admits a `DeviceN`-based `DefaultCMYK`
/// beside a device-independent one, and its NOTE 2 explains why: such a space is subject to
/// Section 6.2.4.4, which is what makes it device independent. ISO 19005-4 dropped that half of the
/// sentence, so [`licensed_by_default_under_part_four`] does not carry it.
///
/// # `TechNote 0010` A028, which is why the *explicit* default is the one read here
///
/// The published section 6.2.2 requires a content stream that references other objects to have an
/// explicitly associated resources dictionary, and does not say whether §8.6.5.6's defaults are
/// among the objects that sentence reaches. A028 is the ISO working group resolving that they are:
/// parts 2 and 3 are read as if a default colour space had itself to be defined in the explicitly
/// associated dictionary, and as if a processor ignored any resource that dictionary does not
/// define. A003 fixes what the term *explicitly associated* names — a `Resources` entry in a
/// page, a tiling pattern, a form `XObject` or a Type 3 font dictionary — so a form `XObject`
/// that states none has no explicitly associated dictionary at all, and the page's `DefaultGray`
/// no longer reaches into it.
///
/// So this reads [`DefaultSpace::explicit`] where [`licensed_by_default_under_part_four`] reads
/// [`DefaultSpace::in_force`]: A028 names ISO 19005-2 and ISO 19005-3 and does not name part 4,
/// whose own section 6.2.2 states the resource sentences in its own words and says nothing about
/// defaults. Printing the resolution's reach wider than the working group drew it is what
/// ADR 0933 forbids.
fn licensed_by_default_under_part_two(default: DefaultSpace, family: DeviceFamily) -> bool {
    default.explicit.is_some_and(|kind| {
        kind.is_independent() || (family == DeviceFamily::Cmyk && kind == SpaceKind::Colourant)
    })
}

/// Whether §8.6.5.6's default colour space in force licenses a device colour under ISO 19005-4.
fn licensed_by_default_under_part_four(default: DefaultSpace) -> bool {
    default.in_force.is_some_and(SpaceKind::is_independent)
}

/// The sentence a failed device colour row prints.
fn unlicensed(colour: &DeviceColour, licences: &str) -> String {
    format!(
        "{} is selected by {} on page {}, where {licences}",
        colour.family.name(),
        colour.what,
        colour.page.saturating_add(1),
    )
}

/// ISO 19005-2 section 6.2.4.3, first sentence.
fn device_rgb_under_part_two(exam: &Examination<'_>, findings: &mut Findings) {
    device_colour_under_part_two(exam, findings, DeviceFamily::Rgb);
}

/// ISO 19005-2 section 6.2.4.3, second sentence.
fn device_cmyk_under_part_two(exam: &Examination<'_>, findings: &mut Findings) {
    device_colour_under_part_two(exam, findings, DeviceFamily::Cmyk);
}

/// ISO 19005-2 section 6.2.4.3, third sentence.
///
/// # The soft-mask exemption, and why it costs this predicate nothing
///
/// `TechNote 0010` A026 resolves that parts 2 and 3 are read as if the sentence admitted a third
/// licence: `DeviceGray` as the `ColorSpace` of a **soft-mask image dictionary**, needing neither
/// a default space nor an output intent, because there the samples describe shape rather than
/// colour and no device dependency is introduced. Session 941 flagged it as the item of the note
/// most likely to be costing a conforming file a false failure here. **It is not, and the reason
/// is a boundary rather than a rule**: `crate::survey` records an image's `ColorSpace` only for an
/// image a content stream draws — an `XObject` reached through `Do`, or §8.9.7's inline image —
/// and a soft mask is reached from the `SMask` entry of an image dictionary, which the walk reads
/// for Annex Q's transparency question and never descends into. So no soft-mask image's colour
/// space is ever a [`DeviceColour`], and there is nothing here to exempt.
///
/// What the resolution binds is any later round that widens the walk. A soft-mask image's
/// `ColorSpace` reaches this predicate only if somebody makes it, and on that day the exemption
/// has to arrive with it; that is why this is written where the rule is rather than where the walk
/// stops. The other direction is already right: an image that a content stream *draws* is judged
/// whether or not it also serves as some other image's soft mask, because drawing it is what makes
/// its samples colour.
///
/// A026's resolution names parts 2 and 3. ISO 19005-4 was published afterwards and its section
/// 6.2.4.3 states the `DeviceGray` sentence with no soft-mask licence, so
/// [`device_gray_under_part_four`] stands on part 4's own text — the reading ADR 0931 gives at
/// A021 and part 4's provenance subclause.
fn device_gray_under_part_two(exam: &Examination<'_>, findings: &mut Findings) {
    device_colour_under_part_two(exam, findings, DeviceFamily::Gray);
}

/// One family's worth of ISO 19005-2 section 6.2.4.3, which admits two licences.
fn device_colour_under_part_two(
    exam: &Examination<'_>,
    findings: &mut Findings,
    family: DeviceFamily,
) {
    unlicensed_under_part_two(exam, findings, |colour| colour.family == family);
}

/// Every device colour use a filter keeps that ISO 19005-2 section 6.2.4.3 does not license.
///
/// The filter is what makes the three subclauses that share this rule three rows: section 6.2.4.3
/// selects by family, section 6.2.4.4 selects the alternate spaces and section 6.2.4.5 the
/// underlying ones, and the licence they are judged against is one sentence written once.
fn unlicensed_under_part_two(
    exam: &Examination<'_>,
    findings: &mut Findings,
    wanted: impl Fn(&DeviceColour) -> bool,
) {
    let document = exam.document;
    let intent = document_intent(document);
    for colour in exam.survey().device_colours() {
        let family = colour.family;
        if !wanted(colour)
            || licensed_by_default_under_part_two(colour.default, family)
            || intent.licenses(family)
        {
            continue;
        }
        findings.record(
            colour.place.clone(),
            unlicensed(
                colour,
                &format!(
                    "no device-independent {} is in force and the file states no PDF/A output \
                     intent {}",
                    family.default_key(),
                    intent_wanted(family),
                ),
            ),
        );
    }
}

/// ISO 19005-4 section 6.2.4.3, first sentence.
fn device_rgb_under_part_four(exam: &Examination<'_>, findings: &mut Findings) {
    device_colour_under_part_four(exam, findings, DeviceFamily::Rgb);
}

/// ISO 19005-4 section 6.2.4.3, second sentence.
fn device_cmyk_under_part_four(exam: &Examination<'_>, findings: &mut Findings) {
    device_colour_under_part_four(exam, findings, DeviceFamily::Cmyk);
}

/// ISO 19005-4 section 6.2.4.3, third sentence.
fn device_gray_under_part_four(exam: &Examination<'_>, findings: &mut Findings) {
    device_colour_under_part_four(exam, findings, DeviceFamily::Gray);
}

/// One family's worth of ISO 19005-4 section 6.2.4.3.
///
/// Three licences rather than part 2's two, and the added one is the reason this is a separate
/// predicate: the transparency blending space then in force licenses a device colour of its own
/// family. `DeviceGray` does not get that third licence — the clause states it only for the
/// other two — which is why the family is tested rather than the blending space alone.
fn device_colour_under_part_four(
    exam: &Examination<'_>,
    findings: &mut Findings,
    family: DeviceFamily,
) {
    unlicensed_under_part_four(exam, findings, |colour| colour.family == family);
}

/// Every device colour use a filter keeps that ISO 19005-4 section 6.2.4.3 does not license.
fn unlicensed_under_part_four(
    exam: &Examination<'_>,
    findings: &mut Findings,
    wanted: impl Fn(&DeviceColour) -> bool,
) {
    let document = exam.document;
    let document_level = document_intent(document);
    let pages = page_intents(document);
    for colour in exam.survey().device_colours() {
        let family = colour.family;
        if !wanted(colour) || licensed_by_default_under_part_four(colour.default) {
            continue;
        }
        if licensed_by_blending(colour) {
            continue;
        }
        if current_intent(document_level, pages.get(colour.page)).licenses(family) {
            continue;
        }
        findings.record(
            colour.place.clone(),
            unlicensed(
                colour,
                &format!(
                    "no device-independent {} is in force, {}and no PDF/A output intent {} is \
                     current",
                    family.default_key(),
                    blending_wanted(family),
                    intent_wanted(family),
                ),
            ),
        );
    }
}

/// What the message says the output intent would have had to hold.
fn intent_wanted(family: DeviceFamily) -> String {
    match family {
        DeviceFamily::Gray => "at all".to_owned(),
        other => format!(
            "holding a {} destination profile",
            String::from_utf8_lossy(other.profile_signature()).trim()
        ),
    }
}

/// The clause of the message about the blending space, which `DeviceGray` has no licence from.
fn blending_wanted(family: DeviceFamily) -> String {
    match family {
        DeviceFamily::Gray => String::new(),
        other => format!(
            "the transparency blending space in force is not a device-independent {}-based \
             space, ",
            String::from_utf8_lossy(other.profile_signature()).trim()
        ),
    }
}

/// Whether ISO 19005-4 section 6.2.4.3's third licence covers this use.
///
/// The clause states it for `DeviceRGB` and `DeviceCMYK` and not for `DeviceGray`, which is why
/// the family is tested rather than the blending space alone: a grey colour under an ICC grey
/// blending space is still a grey colour the clause asks an output intent for.
fn licensed_by_blending(colour: &DeviceColour) -> bool {
    colour.family != DeviceFamily::Gray
        && colour
            .blending
            .is_some_and(|kind| kind.is_independent_family(colour.family))
}

/// ISO 19005-2 section 6.2.4.4, third paragraph.
fn separation_alternate_spaces_under_part_two(exam: &Examination<'_>, findings: &mut Findings) {
    unlicensed_under_part_two(exam, findings, |colour| colour.via == Route::Alternate);
}

/// ISO 19005-4 section 6.2.4.4, third paragraph.
fn separation_alternate_spaces_under_part_four(exam: &Examination<'_>, findings: &mut Findings) {
    unlicensed_under_part_four(exam, findings, |colour| colour.via == Route::Alternate);
}

/// ISO 19005-2 section 6.2.4.5.
fn underlying_spaces_under_part_two(exam: &Examination<'_>, findings: &mut Findings) {
    unlicensed_under_part_two(exam, findings, |colour| colour.via == Route::Underlying);
}

/// ISO 19005-4 section 6.2.4.5.
fn underlying_spaces_under_part_four(exam: &Examination<'_>, findings: &mut Findings) {
    unlicensed_under_part_four(exam, findings, |colour| colour.via == Route::Underlying);
}

/// ISO 19005-4 section 6.2.3, fifth paragraph.
///
/// The condition is a page whose contents are not fully specified in device-independent colour, and
/// Section 6.2.4.1 defines that as colour given either by a device-independent space or through the
/// output intent's profile. So a page is device-dependent here exactly where it sets a device
/// colour that neither §8.6.5.6's default nor section 6.2.4.3's blending-space licence makes
/// independent — the output intent itself is excluded from the test, because it is what the clause
/// is asking the page to supply.
fn a_device_dependent_page_carries_an_output_intent(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    if document_intent(document).present() {
        return;
    }
    let pages = page_intents(document);
    let mut reported: BTreeSet<usize> = BTreeSet::new();
    for colour in exam.survey().device_colours() {
        if licensed_by_default_under_part_four(colour.default) || licensed_by_blending(colour) {
            continue;
        }
        if pages
            .get(colour.page)
            .is_some_and(|intent| intent.present())
        {
            continue;
        }
        if !reported.insert(colour.page) {
            continue;
        }
        findings.record(
            Where::page(colour.page).named("OutputIntents"),
            format!(
                "the page sets {} through {}, so its contents are not fully specified in \
                 device-independent colour, and neither it nor the document states a PDF/A \
                 output intent",
                colour.family.name(),
                colour.what,
            ),
        );
    }
}

/// ISO 19005-4 section 6.2.2, third paragraph — and ISO 19005-2 section 6.2.2 under `TechNote
/// 0010` A002.
///
/// # Why this binds part 2, whose text does not say it
///
/// **A reader with ISO 19005-2 open will not find this sentence, and the row binds part 2 anyway.**
/// Part 2's section 6.2.2 requires a content stream that references other objects to have an
/// explicitly associated resources dictionary and stops there; part 4 adds, in its own words, that
/// the dictionary shall define the names the stream uses. `TechNote 0010` A002 is the ISO working
/// group resolving that parts 2 and 3 are to be read as if that second sentence stood in their
/// section 6.2.2 as well — the ambiguity it was asked about being precisely that the published
/// sentence does not say the names have to be *defined* there.
///
/// This row therefore cites part 2's own clause number and reports the clarification beside the
/// verdict, which `crate::clarification` carries and `crate::report` prints. Until session 942 the
/// row was part 4's alone, so a PDF/A-2 file naming a resource its associated dictionary does not
/// define passed here.
///
/// The population is what `crate::survey` reached: a name is reported only where a content
/// stream the walk actually ran used it, which is what part 4's same paragraph's closing sentence
/// asks for — a named resource nothing references is exempt, and so is a name in a stream
/// nothing invokes.
fn named_resources_are_defined(exam: &Examination<'_>, findings: &mut Findings) {
    for missing in exam.survey().missing_resources() {
        findings.record(
            Where::page(missing.page).named(missing.name.clone()),
            format!(
                "{} names /{} in its {} resources, and the resource dictionary associated with \
                 it defines no such entry",
                missing.what, missing.name, missing.category,
            ),
        );
    }
}

/// ISO 19005-2 section 6.2.2, ISO 19005-4 section 6.2.2, the first sentence.
///
/// # What "defined in the base standard" is decided against
///
/// Each part points at its own base standard's operator summary — part 2 at ISO 32000-1:2008
/// Annex A, part 4 at ISO 32000-2 Annex A — and **the two tables hold the same 73 operators**,
/// compared entry by entry, differing only in how they annotate `F`. So the two parts state one
/// rule with one answer, and this row is a single predicate rather than one per part. The table
/// itself lives in `crate::survey`'s `keyword`, beside the walk that reads the operators, with
/// the measurement that put it there; were a later edition to add or drop an operator, that is
/// where the split would have to be made.
///
/// # Two things the sentence says that a narrower reading would miss
///
/// **`BX` and `EX` exempt nothing.** Both parts say so in as many words, and ISO 32000-2 §7.8.2's
/// compatibility operators are the reason they had to: the base standard lets a processor ignore
/// an unrecognised operator between them, and ISO 19005 withdraws that. So the walk reports what
/// it finds without regard to the brackets, and the corpus's `6-2-2-t01-fail-c` is exactly that
/// case.
///
/// **A resource nothing invokes is exempt.** The same clause's last paragraph says a named
/// resource the content stream does not reference is not used for rendering and is exempt from
/// the part's requirements; `crate::survey` reaches a form `XObject` only through the `Do` that
/// names it, so an unreferenced one is never read. That is the corpus's `6-2-2-t01-pass-a`, and
/// it passes because of how the walk is built rather than by an exception written here.
fn only_operators_the_base_standard_defines(exam: &Examination<'_>, findings: &mut Findings) {
    for unlisted in exam.survey().unlisted_operators() {
        findings.record(
            unlisted
                .place
                .clone()
                .named(String::from_utf8_lossy(&unlisted.spelling).into_owned()),
            format!(
                "{} uses an operator the base standard's operator summary does not list",
                unlisted.what
            ),
        );
    }
}

/// ISO 19005-2 section 6.2.2, ISO 19005-4 section 6.2.2, the paragraph about associated resources.
///
/// Both parts require a content stream that references other objects to have a resource
/// dictionary *explicitly* associated with it, and ISO 32000-2 §7.8.3 is what "associated"
/// means. Its bulleted list gives a page's content two ways of arriving at one — the page
/// dictionary's own `Resources` entry, or inheritance from an ancestor of the page tree — and
/// says of the rest:
///
/// > Content streams that define the glyph descriptions of a Type 3 font shall include a
/// > Resources entry in the Type 3 font dictionary specifying all the resources used by all the
/// > content streams in the CharProcs dictionary of a Type 3 font.
///
/// So *explicitly* is the word that decides this rule: inheritance is the one route §7.8.3
/// offers that no entry on the stream itself states, and the same clause's last bullet — the
/// licence for older files to omit `Resources` from forms and Type 3 fonts and inherit from the
/// page — is exactly what ISO 19005 withdraws here.
///
/// The condition is read as narrowly as it is written. A stream that names no resource at all
/// is not a stream that "references other objects", so a form with no `/Resources` that only
/// paints paths breaks nothing; `crate::survey` reports the two facts per stream it opened, and
/// a stream nothing invokes is not opened.
///
/// # The committee read it the same way, and said which four dictionaries count
///
/// `TechNote 0010` A003 resolves that parts 2 and 3 use "explicitly associated Resources
/// dictionary" for exactly one thing: the `Resources` entry of a page dictionary, a tiling
/// pattern dictionary, a form `XObject` dictionary — annotation appearance streams included — or
/// a Type 3 font dictionary. Nothing inherited through the page tree is one. That is the reading
/// this row already had from §7.8.3's word *explicitly*, and those four dictionaries are exactly
/// the streams `crate::survey` opens with a record, so the resolution changes no verdict; it is
/// cited because a validator that summed the page tree's inheritance into "associated" would
/// disagree with every part-2 verdict here, and a reader is owed the reason.
fn content_streams_carry_their_own_resources(exam: &Examination<'_>, findings: &mut Findings) {
    for opened in exam.survey().opened_streams() {
        if !opened.referenced || opened.own_resources {
            continue;
        }
        findings.record(
            opened.place.clone().named("Resources"),
            format!(
                "{} names resources of its own and carries no Resources entry, so the names it \
                 uses resolve only through a dictionary it inherits",
                opened.what,
            ),
        );
    }
}

/// ISO 19005-2 section 6.2.4.2, ISO 19005-4 section 6.2.4.2, the overprint sentence.
///
/// Both parts forbid overprint mode 1 while an `ICCBased` CMYK colour space is in use and
/// overprinting is on. The population is a painting operator rather than a colour space
/// selection, and ISO 32000-2 §8.6.7 is why: it is the *mark* that non-zero overprint mode acts
/// on, and only for the side of the operator that uses the current colour.
///
/// > Non-zero overprint mode shall apply only to painting operations that use the current colour
/// > in the graphics state when the current colour space is DeviceCMYK (or is implicitly
/// > converted to DeviceCMYK ; see (8.6.5.7, "Implicit conversion of CIE-Based colour spaces").
///
/// That is also what makes the rule per-side: `crate::survey` records each of fill and stroke
/// separately, with the overprint parameter that governs that side, so a stream whose stroking
/// space is `ICCBased` CMYK with `OP` true but which never strokes has not used it.
///
/// # The committee settled the per-side reading, and it is the one open question of the sentence
///
/// `TechNote 0010` A024 is the working group resolving that parts 2 and 3 are read as if the
/// sentence paired each side with its own overprint parameter: an `ICCBased` CMYK space used for
/// stroking while stroke overprinting is on, or used for filling while fill overprinting is on, or
/// both. The question it was asked is the one this predicate would otherwise have to guess —
/// whether an `ICCBased` CMYK *stroke* is forbidden because *fill* overprinting happens to be on
/// — and the answer is no. So the resolution changes no verdict here and is cited because the
/// looser reading is available to anyone reading the published sentence alone.
///
/// **A024's own pertaining line numbers this section 6.2.4.3 in both parts, and the sentence
/// stands at section 6.2.4.2 in the copy this project holds.** The rule is identified by its
/// sentence rather than by a note's clause number, so the citation above follows the standard.
fn no_overprint_mode_one_under_icc_cmyk(exam: &Examination<'_>, findings: &mut Findings) {
    for paint in exam.survey().icc_cmyk_paints() {
        if !paint.overprinting || paint.mode != 1 {
            continue;
        }
        let side = if paint.stroking { "stroke" } else { "fill" };
        findings.record(
            paint.place.clone().named("OPM"),
            format!(
                "{} paints a {side} in an ICCBased CMYK colour space with overprinting on for \
                 that operation and the overprint mode set to 1",
                paint.what,
            ),
        );
    }
}

/// A profile an `ICCBased` colour space may be compared against: how it is reached, and what it
/// is.
///
/// Both halves matter, and for different tests of ISO 19005-4 section 6.2.4.2: the reference
/// decides the first, and the bytes the second.
struct Candidate {
    /// The object the profile stream is, where the entry that names it is a reference.
    id: Option<ObjectId>,
    /// The stream itself.
    stream: Arc<Stream>,
}

impl Candidate {
    /// The profile a survey's `ICCBased` selection is formed from.
    fn of(profile: &IccProfile) -> Self {
        Self {
            id: profile.id,
            stream: Arc::clone(&profile.stream),
        }
    }
}

/// The decoded profiles one run of the rule below has already read.
///
/// A CMYK destination profile is half a megabyte, and the same one is compared once per
/// selection that reaches it, so decoding it again each time would be the whole cost of the
/// rule. Only a profile that is an indirect object is remembered, because that is the only one
/// with a name to remember it by.
#[derive(Default)]
struct Profiles {
    /// What each profile object decoded to, including the ones that refused to decode.
    read: BTreeMap<ObjectId, Option<Arc<[u8]>>>,
    /// The MD5 each profile object is identified by, for the same reason and at a higher price:
    /// the digest is taken over the whole profile, so a half-megabyte press profile compared
    /// against three selections would otherwise be hashed three times.
    identified: BTreeMap<ObjectId, Option<[u8; 16]>>,
}

impl Profiles {
    /// One profile's decoded bytes.
    fn bytes(&mut self, document: &Document, profile: &Candidate) -> Option<Arc<[u8]>> {
        let Some(id) = profile.id else {
            return document.decoded_stream_data(&profile.stream);
        };
        self.read
            .entry(id)
            .or_insert_with(|| document.decoded_stream_data(&profile.stream))
            .clone()
    }

    /// The sixteen bytes ISO 19005-4 section 6.2.4.2 compares two profiles by.
    fn identity(&mut self, bytes: &[u8], profile: &Candidate) -> Option<[u8; 16]> {
        let Some(id) = profile.id else {
            return icc::profile_id(bytes);
        };
        *self
            .identified
            .entry(id)
            .or_insert_with(|| icc::profile_id(bytes))
    }
}

/// Whether ISO 19005-4 section 6.2.4.2's two stated tests make two profiles the same one.
///
/// The clause states both. The first is the colour space and the other holder reaching one
/// embedded stream by indirect reference. The second is equal MD5 values, each read from that
/// profile's own `Profile ID` field where it is present and not zero, and otherwise calculated by
/// the methodology of ISO 15076-1:2010 section 7.2.18 — which is `pdf_model::icc::profile_id`,
/// and that function's own comment says where the method is read from, since the document part 4
/// names is held here only as a front-matter preview.
///
/// **The clause conditions neither route on the profile's own version.** It says *each profile*,
/// and the field it names sits at the same bytes of the header in every ICC edition this project
/// holds — so a v2 profile stating a non-zero identifier is taken at its word exactly as a v4 one
/// is. That is the reading the corpus's `6-2-4-2-t03-fail-e` turns on: two copies of one v2.1
/// press profile, identical but for those sixteen bytes, one stating the identifier and the other
/// stating zero. Reading the field alone would answer *different*, and computing alone would
/// answer *same*; the clause's order — stated, else computed — answers *same* for both, which is
/// what makes the pair one profile.
///
/// Equal bytes remain equal profiles, and that case is reached before either hash is taken: it is
/// the cheap half, and an MD5 is a function of the bytes it is taken over.
fn same_profile(
    document: &Document,
    read: &mut Profiles,
    left: &Candidate,
    right: &Candidate,
) -> bool {
    if let (Some(left), Some(right)) = (left.id, right.id)
        && left == right
    {
        return true;
    }
    let Some(left_bytes) = read.bytes(document, left) else {
        return false;
    };
    let Some(right_bytes) = read.bytes(document, right) else {
        return false;
    };
    if left_bytes == right_bytes {
        return true;
    }
    // Two profiles with no identifier between them are not made identical by both failing to
    // state one, which is why an absent value on either side is an answer of *no* rather than a
    // comparison of `None` with `None`.
    match (
        read.identity(&left_bytes, left),
        read.identity(&right_bytes, right),
    ) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

/// The `DestOutputProfile` of one `OutputIntents` array's PDF/A entry.
///
/// The entry is read unresolved, because ISO 19005-4 section 6.2.4.2's first test is about the
/// reference rather than about what it reaches.
fn pdfa_destination_profile(
    document: &Document,
    array: &[(Where, Dictionary)],
) -> Option<Candidate> {
    array.iter().find_map(|(_, intent)| {
        if !is_pdfa_output_intent(document, intent) {
            return None;
        }
        let entry = intent.get("DestOutputProfile")?;
        let id = entry.as_reference();
        match document.resolve(entry) {
            Object::Stream(stream) => Some(Candidate { id, stream }),
            _ => None,
        }
    })
}

/// ISO 19005-4 section 6.2.4.2, the last requirement.
///
/// An `ICCBased` colour space carrying a CMYK destination profile identical to the one in the
/// current PDF/A output intent, or in the transparency blending colour space then in force, is
/// forbidden — and the clause's NOTE 2 says why: which of the two profiles a renderer applies
/// would decide the output, since §8.6.5.7's implicit conversion may or may not happen.
///
/// The rule binds a space that is *used*, so the population is `crate::survey`'s selections
/// rather than every `ICCBased` array in the file. That distinction is the whole of the
/// corpus's `6-2-4-2-t03-pass-b`: it states the very profile the output intent states, under the
/// same object, in the page's `/ColorSpace` — and its content stream never names it.
///
/// ISO 19005-2 states no such sentence, which is why this row cites part 4 alone.
fn no_icc_space_duplicating_a_current_profile(exam: &Examination<'_>, findings: &mut Findings) {
    duplicating_a_current_profile(exam, findings, |via| via != Route::Alternate);
}

/// ISO 19005-4 section 6.2.4.2's last requirement, reached through section 6.2.4.4's third
/// paragraph.
///
/// Split from the row above rather than folded into it, because the two cite different clauses: an
/// `ICCBased` space the content selects is section 6.2.4.2's own business, and the alternate space
/// of a `Separation` or `DeviceN` is subject to section 6.2.4.2 only because section 6.2.4.4 says
/// so. A reader checking a verdict against their own copy has to be sent to the sentence that put
/// the restriction where this found it — the same reason the device colour rules are three rows
/// over one walk rather than one row.
fn separation_alternates_duplicating_a_current_profile(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    duplicating_a_current_profile(exam, findings, |via| via == Route::Alternate);
}

/// The shared body of the two rows above, over the routes each of them owns.
fn duplicating_a_current_profile(
    exam: &Examination<'_>,
    findings: &mut Findings,
    wanted: impl Fn(Route) -> bool,
) {
    let document = exam.document;
    let selections = exam.survey().icc_selections();
    if selections.is_empty() {
        return;
    }
    let of_document = output_intent_arrays(document)
        .first()
        .and_then(|array| pdfa_destination_profile(document, array));
    let pages = Pages::new(document);
    let of_page: Vec<Option<Candidate>> = (0..pages.len())
        .map(|index| {
            pages.get(index).and_then(|page| {
                pdfa_destination_profile(document, &output_intents(document, &page.dict))
            })
        })
        .collect();
    let mut read = Profiles::default();
    for selection in selections {
        if !wanted(selection.via) {
            continue;
        }
        // The clause binds a CMYK destination profile, which `N` is what tells: §8.6.5.5's
        // Table 66 makes the component count the profile's own, so a three-component profile
        // duplicating an RGB output intent is not what this forbids.
        if selection.profile.family != Some(DeviceFamily::Cmyk) {
            continue;
        }
        let used = Candidate::of(&selection.profile);
        // Section 6.2.3: a page's own PDF/A output intent is the current one where it states one,
        // and the document's otherwise.
        let current = of_page
            .get(selection.page)
            .and_then(Option::as_ref)
            .or(of_document.as_ref());
        if let Some(current) = current
            && same_profile(document, &mut read, &used, current)
        {
            findings.record(
                selection.place.clone(),
                format!(
                    "{} uses {} whose profile is the profile in the PDF/A output intent then \
                     current",
                    selection.what,
                    icc_space_reached(selection.via),
                ),
            );
        }
        if let Some(blending) = &selection.blending
            && same_profile(document, &mut read, &used, &Candidate::of(blending))
        {
            findings.record(
                selection.place.clone(),
                format!(
                    "{} uses {} whose profile is the profile of the transparency blending \
                     colour space then in force",
                    selection.what,
                    icc_space_reached(selection.via),
                ),
            );
        }
    }
}

/// How a report names the `ICCBased` space a finding is about, given how it was reached.
fn icc_space_reached(via: Route) -> &'static str {
    match via {
        Route::Direct => "an ICCBased CMYK colour space",
        Route::Underlying => "an ICCBased CMYK colour space underlying the one it selected",
        Route::Alternate => {
            "a Separation or DeviceN colour space whose alternate is an \
                             ICCBased CMYK space"
        }
    }
}

/// ISO 19005-2 section 6.2.6, the `ri` operator.
///
/// The operator's operand is a name in the content stream rather than an entry in a dictionary,
/// so it is `crate::survey` that reports it. ISO 19005-4 states no rendering intent subclause at
/// all, which is why this row cites part 2 alone.
fn rendering_intent_operator_names_one_of_four(exam: &Examination<'_>, findings: &mut Findings) {
    for (page, intent) in exam.survey().rendering_intents() {
        if RENDERING_INTENTS.contains(&intent.as_slice()) {
            continue;
        }
        findings.record(
            Where::page(*page).named(String::from_utf8_lossy(intent).into_owned()),
            "the rendering intent operator was given a name that is not one of the four \
             ISO 32000 defines",
        );
    }
}

/// ISO 19005-2 section 6.2.8.1, ISO 19005-4 section 6.2.7.1, the inline image half.
///
/// §8.9.7's `/I` is the abbreviation of `Interpolate`, and `pdf_model::inline_image` expands it
/// before the dictionary reaches here, so one spelling is read rather than two.
fn inline_image_interpolation_is_off(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for (page, image) in exam.survey().inline_images() {
        if !states(document, image, "Interpolate") {
            continue;
        }
        if document.get_key(image, "Interpolate") != Object::Boolean(false) {
            findings.record(
                Where::page(*page).named("Interpolate"),
                "an inline image states an Interpolate other than false",
            );
        }
    }
}

/// The bit depths ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 admit, inclusive.
///
/// The same range ISO 32000-2 §7.4.9 states of the filter — "bits per sample shall be between 1
/// to 38 inclusive" — and the same one ISO/IEC 15444-1:2000's Tables I-6 and A-11 encode, whose
/// defined values run from one bit to thirty-eight and reserve the rest.
const JPEG2000_DEPTHS: std::ops::RangeInclusive<u8> = 1..=38;

/// The colour specification methods both parts admit in a `colr` box.
///
/// **Three of them, and ISO/IEC 15444-1:2000 Table I-9 defines only two.** The third comes from
/// the later work the extensions part carries, so a validator that judged `METH` against the
/// core part alone would reject a value ISO 19005 permits. The rule implemented here is
/// ISO 19005's, not part 1's.
const JPEG2000_METHODS: [u8; 3] = [0x01, 0x02, 0x03];

/// The enumerated colour space both parts forbid, `CIEJab`.
///
/// Named by number rather than by definition, which is all either part gives it and all this
/// rule needs. ISO 32000-2 §7.4.9 excludes the same number from what a PDF may carry.
const JPEG2000_CIEJAB: u32 = 19;

/// The `APPROX` value that marks the colour specification with the best colour fidelity.
///
/// Both parts' NOTE 2 says so in as many words. ISO/IEC 15444-1:2000 I.5.3.3 reserves the field
/// and sets it to zero, which is the first edition's reading of a field ISO 19005 gives a
/// meaning — so this constant is ISO 19005's requirement, not part 1's.
const JPEG2000_BEST_APPROXIMATION: u8 = 0x01;

/// Visits the headers of every `JPXDecode` stream the file's cross-referenced objects state.
///
/// **The codec is the test, not the `Subtype`.** ISO 19005-2 section 6.2.8.3 and ISO 19005-4
/// section 6.2.7.3 bind "the JPEG2000 data" rather than a dictionary, and ISO 32000-2 §7.4.9
/// confines the filter to image `XObject`s — so a stream carrying it either is an image or is
/// already breaking that clause, and neither is a reason to leave its data unread.
///
/// **Data that does not parse is passed over rather than reported**, which is the same choice
/// [`icc_profiles_conform_to_the_base_standard`] makes about a stream that does not decode: what
/// a malformed codestream breaks is §7.4.9 and the closing sentence of these two subclauses, and
/// that sentence is the row this file leaves [`Check::Unchecked`]. Announcing it under the
/// channel-count rule would put a true finding under a false clause.
///
/// The headers are parsed once per requirement rather than once per report, and deliberately: a
/// parse here reads a hundred-odd bytes of boxes already in memory, decodes nothing, and starts
/// no process — unlike the `/Annots` walk that put [`crate::Examination`]'s shared work there.
fn for_each_jpeg2000(exam: &Examination<'_>, mut visit: impl FnMut(ObjectId, &Headers<'_>)) {
    let document = exam.document;
    for (id, object) in exam.objects() {
        let Object::Stream(stream) = object else {
            continue;
        };
        let Some(image) = document.image_stream(stream) else {
            continue;
        };
        if image.codec.as_deref() != Some(b"JPXDecode".as_slice()) {
            continue;
        }
        let Ok(headers) = Headers::parse(&image.data) else {
            continue;
        };
        visit(*id, &headers);
    }
}

/// Where a finding about JPEG 2000 data is reported: the stream that carries it.
fn jpeg2000_site(id: ObjectId) -> Where {
    Where::object(id).named("JPXDecode")
}

/// ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3: 1, 3 or 4 colour channels.
///
/// A *colour* channel, which is not the same as a component: ISO/IEC 15444-1:2000 I.5.3.6 gives
/// each channel a type, and only type 0 is colour, so an RGB image with an opacity channel has
/// four components and three colour channels. `pdf_model::jpeg2000` makes that distinction and
/// this rule takes it.
///
/// **Two statements of the count are checked where the file makes two.** I.5.3.6 says a file with
/// no channel definition box holds colour channels only, so for such a file the image header's
/// `NC` and the codestream's `Csiz` are both statements of this number — and I.5.3.1 says a file
/// whose header contradicts its codestream is not conforming. Judging only one of them would let
/// a file put the wrong count in the half a reader did not look at, which is exactly what the
/// corpus witness for this rule does.
fn jpeg2000_channel_count(exam: &Examination<'_>, findings: &mut Findings) {
    for_each_jpeg2000(exam, |id, headers| {
        let mut counts: Vec<u32> = headers.colour_channels().into_iter().collect();
        if headers.channels.is_empty() {
            counts.extend(
                headers
                    .codestream
                    .as_ref()
                    .map(|codestream| u32::from(codestream.components())),
            );
        }
        counts.sort_unstable();
        counts.dedup();
        for count in counts {
            if !matches!(count, 1 | 3 | 4) {
                findings.record(
                    jpeg2000_site(id),
                    format!("JPEG 2000 data states {count} colour channels, not 1, 3 or 4"),
                );
            }
        }
    });
}

/// ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3: one specification marked best, and its
/// profile.
///
/// Two sentences, and the second depends on the first. Where the data states more than one
/// colour space specification, exactly one shall carry [`JPEG2000_BEST_APPROXIMATION`] in its
/// `APPROX` field; and where *that* specification — the selected one — uses an ICC profile, the
/// profile shall meet what the base standard requires of one.
///
/// **The ICC half is read as far as ISO 32000-2 §8.6.5.5's Table 67 goes and no further**, which
/// is the boundary [`icc_profiles_conform_to_the_base_standard`] already draws for the same
/// clause: the profile's device class and data colour space are checked against that table,
/// while Table 65's `N` has no counterpart here — a profile inside a `colr` box sits in no PDF
/// dictionary to disagree with — and the sentence asking a profile to conform to the ICC
/// specification its own header names needs the ICC texts, which this project does not hold.
///
/// Only a `METH` of 2 yields profile bytes to read. ISO/IEC 15444-1:2000 Table I-9 defines the
/// embedded profile for that method alone and reserves every other value, so a `METH` of 3 —
/// which ISO 19005 permits and part 1 does not describe — carries bytes this tree cannot claim
/// to be reading correctly, and they are left alone rather than guessed at.
fn jpeg2000_one_best_colour_space_specification(exam: &Examination<'_>, findings: &mut Findings) {
    for_each_jpeg2000(exam, |id, headers| {
        let best: Vec<&ColourSpecification<'_>> = headers
            .colour
            .iter()
            .filter(|colour| colour.approximation == JPEG2000_BEST_APPROXIMATION)
            .collect();

        let selected = if headers.colour.len() > 1 {
            if best.len() == 1 {
                best.first().copied()
            } else {
                findings.record(
                    jpeg2000_site(id),
                    format!(
                        "JPEG 2000 data states {} colour space specifications of which {} is \
                         marked as the best available, where exactly one shall be",
                        headers.colour.len(),
                        best.len()
                    ),
                );
                None
            }
        } else {
            headers.colour.first()
        };

        let Some(profile) = selected.and_then(|colour| colour.profile) else {
            return;
        };
        let Some((class, space)) = profile_header(profile) else {
            return;
        };
        if !ICC_DEVICE_CLASSES.contains(&class) {
            findings.record(
                jpeg2000_site(id),
                "the ICC profile in the selected JPEG 2000 colour specification is of a device \
                 class the base standard does not admit for a colour space",
            );
        }
        if !ICC_COLOUR_SPACES
            .iter()
            .any(|(signature, _)| *signature == space)
        {
            findings.record(
                jpeg2000_site(id),
                "the ICC profile in the selected JPEG 2000 colour specification states a data \
                 colour space the base standard does not admit for a colour space",
            );
        }
    });
}

/// ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3: `METH` shall be 0x01, 0x02 or 0x03.
///
/// Every `colr` box, not only the first. Both parts write the sentence about "its `colr` box" in
/// the singular, and ISO/IEC 15444-1:2000 I.5.3.3 permits several — a file may carry one per
/// method — so the requirement is read as binding each of them. The alternative reading, that a
/// file may hide an undefined method in a box after the first, would make the sentence say less
/// the more boxes a file states.
fn jpeg2000_colour_specification_method(exam: &Examination<'_>, findings: &mut Findings) {
    for_each_jpeg2000(exam, |id, headers| {
        for colour in &headers.colour {
            if !JPEG2000_METHODS.contains(&colour.method) {
                findings.record(
                    jpeg2000_site(id),
                    format!(
                        "a JPEG 2000 colour specification box states a METH of {:#04x}, which is \
                         none of 0x01, 0x02 and 0x03",
                        colour.method
                    ),
                );
            }
        }
    });
}

/// ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3: enumerated colour space 19 shall not
/// be used.
///
/// ISO/IEC 15444-1:2000 I.5.3.3 puts `EnumCS` in a `colr` box only where `METH` is 1, which is
/// why `pdf_model::jpeg2000` reports it only there and this rule asks no more.
///
/// The neighbouring sentence — that enumerated colour space 12 (CMYK) *may* be used — states a
/// permission rather than a requirement, so there is nothing for a file to fail and no row for
/// it. It is written down here because its absence from the table is a decision.
fn jpeg2000_no_ciejab_colour_space(exam: &Examination<'_>, findings: &mut Findings) {
    for_each_jpeg2000(exam, |id, headers| {
        for colour in &headers.colour {
            if colour.enumerated == Some(JPEG2000_CIEJAB) {
                findings.record(
                    jpeg2000_site(id),
                    "a JPEG 2000 colour specification box uses enumerated colour space 19, \
                     CIEJab, which ISO 19005 forbids",
                );
            }
        }
    });
}

/// ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3: 1 to 38 bits, the same on every colour
/// channel.
///
/// Two sentences with different subjects, and they are checked over different populations.
///
/// - The *range* binds the bit depth of the JPEG 2000 data, so every component's declared depth
///   is judged: an opacity channel's samples are as much the data as a colour channel's, and
///   ISO 32000-2 §7.4.9 states the same range without narrowing it to colour either.
/// - The *equality* binds the colour channels by its own words, so a channel that
///   ISO/IEC 15444-1:2000 I.5.3.6 types as opacity is excluded from it — an RGB image with an
///   eight-bit opacity channel over twelve-bit colour breaks no sentence of this rule.
///
/// Where the file states a channel definition box, its `Cn` indices are read as component
/// indices, which I.5.3.6 makes them wherever there is no component mapping box. Where a file
/// states one of *those*, they are not, and `pdf_model::jpeg2000` reads only its presence — so
/// the equality check is skipped there rather than made over the wrong components. The range
/// check above is unaffected, binding every component's declared depth however the channels are
/// mapped.
fn jpeg2000_bit_depth(exam: &Examination<'_>, findings: &mut Findings) {
    for_each_jpeg2000(exam, |id, headers| {
        let depths = headers.component_depths();
        let mut reported: Vec<u8> = Vec::new();
        for depth in &depths {
            if !JPEG2000_DEPTHS.contains(&depth.bits) && !reported.contains(&depth.bits) {
                reported.push(depth.bits);
                findings.record(
                    jpeg2000_site(id),
                    format!(
                        "JPEG 2000 data states a bit depth of {}, outside the range 1 to 38",
                        depth.bits
                    ),
                );
            }
        }

        if headers.component_mapping {
            return;
        }
        let colour: Vec<u8> = if headers.channels.is_empty() {
            depths.iter().map(|depth| depth.bits).collect()
        } else {
            headers
                .channels
                .iter()
                .filter(|channel| channel.kind == Channel::COLOUR)
                .filter_map(|channel| depths.get(usize::from(channel.channel)))
                .map(|depth| depth.bits)
                .collect()
        };
        if colour.windows(2).any(|pair| pair[0] != pair[1]) {
            findings.record(
                jpeg2000_site(id),
                "the colour channels of JPEG 2000 data do not all have the same bit depth",
            );
        }
    });
}

/// ISO 19005-2 section 6.2.10, second paragraph.
fn a_transparent_page_has_a_blending_space(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    if document_intent(document).present() {
        return;
    }
    for page in transparent_pages_without_a_blending_space(exam) {
        findings.record(
            Where::page(page).named("Group"),
            "the page contains transparency, the file states no PDF/A output intent, and the \
             page states no Group whose attribute dictionary names a CS to blend in",
        );
    }
}

/// ISO 19005-4 section 6.2.9, second paragraph.
fn a_transparent_page_has_a_blending_space_or_an_output_intent(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    if document_intent(document).present() {
        return;
    }
    let pages = page_intents(document);
    for page in transparent_pages_without_a_blending_space(exam) {
        if pages.get(page).is_some_and(|intent| intent.present()) {
            continue;
        }
        findings.record(
            Where::page(page).named("Group"),
            "the page contains transparency, the document states no PDF/A output intent, and \
             the page states neither one of its own nor a Group whose attribute dictionary \
             names a CS to blend in",
        );
    }
}

/// The pages ISO 32000-2 Annex Q finds transparency on that state no blending colour space.
///
/// The `Group` test is deliberately the clause's own words and not more: the entry is asked for,
/// and its attribute dictionary is asked for a `CS`. A page that states a group of some other
/// subtype with a `CS` is not reported here, because the sentence this implements does not
/// mention `S` at all and inventing the condition could only manufacture failures.
fn transparent_pages_without_a_blending_space(exam: &Examination<'_>) -> Vec<usize> {
    let document = exam.document;
    let survey = exam.survey();
    let pages = Pages::new(document);
    (0..survey.pages())
        .filter(|index| survey.page_is_transparent(*index))
        .filter(|index| {
            pages.get(*index).is_none_or(|page| {
                document
                    .get_key(&page.dict, "Group")
                    .as_dict()
                    .is_none_or(|group| !states(document, group, "CS"))
            })
        })
        .collect()
}

/// ISO 19005-2 section 6.2.10, third paragraph.
fn group_colour_spaces_obey_the_colour_rules_under_part_two(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    let intent = document_intent(document);
    for group in exam.survey().group_spaces() {
        let SpaceKind::Device(family) = group.kind else {
            continue;
        };
        if licensed_by_default_under_part_two(group.default, family) || intent.licenses(family) {
            continue;
        }
        findings.record(
            group.place.clone(),
            format!(
                "a transparency group blends in {}, and neither a device-independent {} nor a \
                 PDF/A output intent {} licenses it",
                family.name(),
                family.default_key(),
                intent_wanted(family),
            ),
        );
    }
}

/// ISO 19005-4 section 6.2.9, third paragraph.
///
/// The one difference from part 2's row is which output intent counts: ISO 19005-4 admits a
/// page-level one, so a group on a page that states its own is judged against that.
fn group_colour_spaces_obey_the_colour_rules_under_part_four(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    let document_level = document_intent(document);
    let pages = page_intents(document);
    for group in exam.survey().group_spaces() {
        let SpaceKind::Device(family) = group.kind else {
            continue;
        };
        if licensed_by_default_under_part_four(group.default) {
            continue;
        }
        if current_intent(document_level, pages.get(group.page)).licenses(family) {
            continue;
        }
        findings.record(
            group.place.clone(),
            format!(
                "a transparency group blends in {}, and neither a device-independent {} nor a \
                 current PDF/A output intent {} licenses it",
                family.name(),
                family.default_key(),
                intent_wanted(family),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::Examination;
    use crate::{Flavour, Target};
    use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

    use super::{
        BLEND_MODES, IccEdition, Position, Site, equivalent, is_halftone, missing_required_tags,
        profile_header,
    };
    use crate::finding::Findings;
    use crate::table::requirements;

    /// A dictionary from `(key, value)` pairs, for the predicates that read one directly.
    fn dictionary(entries: &[(&str, Object)]) -> Dictionary {
        let mut dict = Dictionary::new();
        for (key, value) in entries {
            dict.insert(Name::new(key.as_bytes().to_vec()), value.clone());
        }
        dict
    }

    /// A name object, which is most of what these rules are about.
    fn name(text: &str) -> Object {
        Object::Name(Name::new(text.as_bytes().to_vec()))
    }

    #[test]
    fn every_row_this_module_states_cites_a_graphics_clause() {
        let mine: Vec<_> = requirements()
            .filter(|requirement| requirement.id.starts_with("graphics/"))
            .collect();
        assert!(
            mine.len() >= 40,
            "the tranche should cover clause 6.2 up to the fonts, and has {} rows",
            mine.len()
        );
        for requirement in mine {
            for clause in [requirement.clauses.two, requirement.clauses.four]
                .into_iter()
                .flatten()
            {
                assert!(
                    // Annex B modifies clause 6.2 for PDF/A-4e rather than restating it
                    // elsewhere, so a row of this tranche may cite it — and only it.
                    clause.starts_with("6.2.") || clause.starts_with("B.2."),
                    "{} cites {clause}, which is not in this tranche's range",
                    requirement.id
                );
                assert!(
                    !clause.starts_with("6.2.11") && !clause.starts_with("6.2.10."),
                    "{} cites {clause}, which belongs to the fonts tranche",
                    requirement.id
                );
            }
        }
    }

    /// A halftone is recognised by its own required entry, because `Type` is optional there.
    #[test]
    fn a_halftone_is_recognised_without_a_type() {
        let document = Document::empty();
        let by_entry = dictionary(&[("HalftoneType", Object::Integer(1))]);
        let by_type = dictionary(&[("Type", name("Halftone"))]);
        let neither = dictionary(&[("Frequency", Object::Integer(60))]);
        assert!(is_halftone(&document, &by_entry));
        assert!(is_halftone(&document, &by_type));
        assert!(!is_halftone(&document, &neither));
    }

    /// The three signatures the destination profile rule turns on.
    #[test]
    fn a_profile_header_is_read_at_the_offsets_the_tree_already_uses() {
        let mut header = vec![0_u8; 128];
        header[12..16].copy_from_slice(b"prtr");
        header[16..20].copy_from_slice(b"CMYK");
        header[36..40].copy_from_slice(b"acsp");
        assert_eq!(
            profile_header(&header),
            Some((b"prtr".as_slice(), b"CMYK".as_slice()))
        );
        header[36..40].copy_from_slice(b"junk");
        assert_eq!(
            profile_header(&header),
            None,
            "bytes that are not an ICC profile are not judged as one"
        );
        assert_eq!(profile_header(&header[..20]), None, "a truncated header");
    }

    /// A profile carrying `tags` and nothing else, for the required-tag rule.
    ///
    /// Positional, so it doubles as a statement of the layout: the 128-byte header, the tag count,
    /// a twelve-byte entry per tag, and empty tag data.
    fn profile_of(class: [u8; 4], tags: &[&[u8; 4]]) -> Vec<u8> {
        let mut out = vec![0_u8; 128];
        out[12..16].copy_from_slice(&class);
        out[16..20].copy_from_slice(b"RGB ");
        out[36..40].copy_from_slice(b"acsp");
        out.extend_from_slice(&u32::try_from(tags.len()).expect("small").to_be_bytes());
        for tag in tags {
            out.extend_from_slice(*tag);
            out.extend_from_slice(&0_u32.to_be_bytes());
            out.extend_from_slice(&0_u32.to_be_bytes());
        }
        out
    }

    /// Each held edition states the one version number consistent with itself, and nothing states
    /// what a version between or beyond them would owe.
    #[test]
    fn an_edition_is_the_version_number_its_own_text_states() {
        assert_eq!(IccEdition::of((2, 2, 0)), Some(IccEdition::Nineteen98));
        assert_eq!(IccEdition::of((4, 0, 0)), Some(IccEdition::TwoThousand1));
        assert_eq!(IccEdition::of((4, 4, 0)), Some(IccEdition::TwentyTwo));
        assert_eq!(IccEdition::of((5, 0, 0)), Some(IccEdition::TwentyThree));
        assert_eq!(IccEdition::of((2, 1, 0)), None, "the commonest v2 revision");
        assert_eq!(IccEdition::of((4, 1, 0)), None, "ICC.1:2003-09's version");
        assert_eq!(
            IccEdition::of((4, 3, 0)),
            None,
            "ISO 15076-1:2010's version"
        );
        assert_eq!(IccEdition::of((4, 4, 1)), None, "a bug-fix revision of 4.4");
        assert_eq!(IccEdition::of((5, 1, 0)), None, "a later iccMAX");
    }

    /// The two v2-era texts state the field or the method or neither, and the row that reads a
    /// `Profile ID` has to know which.
    #[test]
    fn only_the_two_later_editions_state_a_profile_id_this_can_compute() {
        assert_eq!(IccEdition::Nineteen98.profile_id_clause(), None);
        assert_eq!(IccEdition::TwoThousand1.profile_id_clause(), None);
        assert_eq!(IccEdition::TwentyTwo.profile_id_clause(), Some("7.2.18"));
        assert_eq!(IccEdition::TwentyThree.profile_id_clause(), Some("7.2.20"));
    }

    /// The profile this project ships is a version 2.0.0 RGB display profile, and ICC.1:1998-09 —
    /// one of the four editions ISO 19005-2 names — requires of one exactly the nine tags it
    /// carries. `doc/adr/0950` has why the version number it states is not the question.
    #[test]
    fn the_shipped_profile_carries_what_a_permitted_edition_requires() {
        let profile: &[u8] = include_bytes!("../../../../data/icc/sRGB2014.icc");
        assert!(
            missing_required_tags(IccEdition::Nineteen98, profile, b"mntr").is_empty(),
            "ICC.1:1998-09 Table 27's tags for an RGB display profile"
        );
        assert!(
            missing_required_tags(IccEdition::TwoThousand1, profile, b"mntr").is_empty(),
            "and ICC.1:2001-12 Table 25's, which are the same six plus the same three"
        );
    }

    /// ICC.2 section 8.4 asks a display profile for a transform in each direction, and the v2
    /// matrix-and-curve tags are not one — which is the corpus's `6-2-3-t01-fail-d`.
    #[test]
    fn a_matrix_profile_is_not_a_version_five_display_profile() {
        let matrix = profile_of(
            *b"mntr",
            &[b"desc", b"cprt", b"wtpt", b"rXYZ", b"gXYZ", b"bXYZ"],
        );
        assert_eq!(
            missing_required_tags(IccEdition::TwentyThree, &matrix, b"mntr").len(),
            2,
            "neither direction of transform is present"
        );
        let lut = profile_of(*b"mntr", &[b"desc", b"cprt", b"wtpt", b"A2B1", b"B2D3"]);
        assert!(
            missing_required_tags(IccEdition::TwentyThree, &lut, b"mntr").is_empty(),
            "any one of each list satisfies the clause"
        );
    }

    /// ICC.1 section 8.4 gives a display profile three forms, and the same profile that fails
    /// ICC.2 satisfies the second of them.
    #[test]
    fn the_same_matrix_profile_is_one_of_icc_ones_three_display_forms() {
        let matrix = profile_of(
            *b"mntr",
            &[
                b"desc", b"cprt", b"wtpt", b"rXYZ", b"gXYZ", b"bXYZ", b"rTRC", b"gTRC", b"bTRC",
            ],
        );
        assert!(missing_required_tags(IccEdition::TwentyTwo, &matrix, b"mntr").is_empty());
        assert_eq!(
            missing_required_tags(IccEdition::TwentyTwo, &matrix, b"prtr").len(),
            1,
            "section 8.5 offers an output profile neither form"
        );
        let bare = profile_of(*b"mntr", &[]);
        assert_eq!(
            missing_required_tags(IccEdition::TwentyTwo, &bare, b"mntr").len(),
            4,
            "section 8.2's three tags and section 8.4's form"
        );
    }

    /// `0` and `0.0` are one value, which is what keeps the Separation rule from inventing a
    /// disagreement out of how a producer spelled a number.
    #[test]
    fn numbers_compare_by_value_and_names_by_bytes() {
        let document = Document::empty();
        assert!(equivalent(
            &document,
            &Object::Integer(0),
            &Object::Real(0.0),
            0
        ));
        assert!(!equivalent(
            &document,
            &Object::Integer(0),
            &Object::Integer(1),
            0
        ));
        assert!(equivalent(
            &document,
            &name("DeviceRGB"),
            &name("DeviceRGB"),
            0
        ));
        assert!(!equivalent(
            &document,
            &name("DeviceRGB"),
            &name("DeviceGray"),
            0
        ));
    }

    /// Two dictionaries that differ only in an entry the clause ignores are the same object for
    /// this purpose — the check is on arrays, which is where a tint transform's dictionary sits.
    #[test]
    fn dictionaries_compare_entry_by_entry() {
        let document = Document::empty();
        let one = Object::Dictionary(dictionary(&[("N", Object::Integer(1))]));
        let same = Object::Dictionary(dictionary(&[("N", Object::Real(1.0))]));
        let other = Object::Dictionary(dictionary(&[("N", Object::Integer(2))]));
        let extra = Object::Dictionary(dictionary(&[
            ("N", Object::Integer(1)),
            ("Domain", Object::Integer(0)),
        ]));
        assert!(equivalent(&document, &one, &same, 0));
        assert!(!equivalent(&document, &one, &other, 0));
        assert!(!equivalent(&document, &one, &extra, 0));
    }

    /// Every name the blend mode rule accepts is one ISO 32000 lists, and nothing else is.
    #[test]
    fn the_blend_modes_are_the_ones_iso_32000_defines() {
        assert!(BLEND_MODES.contains(&b"Luminosity".as_slice()));
        assert!(BLEND_MODES.contains(&b"Compatible".as_slice()));
        assert!(
            !BLEND_MODES.contains(&b"Subtract".as_slice()),
            "a name from another imaging model is not one of them"
        );
    }

    /// A graphics state is recognised by its position as well as by its `Type`, which is what
    /// the walk carries the position down for.
    #[test]
    fn a_graphics_state_is_recognised_by_position_or_by_type() {
        let document = Document::empty();
        let plain = dictionary(&[("BM", name("Multiply"))]);
        let typed = dictionary(&[("Type", name("ExtGState"))]);
        let id = ObjectId::new(1, 0);
        assert!(super::is_graphics_state(
            &document,
            &Site {
                id,
                position: Position::GraphicsState,
                dict: &plain,
            }
        ));
        assert!(!super::is_graphics_state(
            &document,
            &Site {
                id,
                position: Position::Anywhere,
                dict: &plain,
            }
        ));
        assert!(super::is_graphics_state(
            &document,
            &Site {
                id,
                position: Position::Anywhere,
                dict: &typed,
            }
        ));
    }

    /// A file assembled from object bodies, with a cross-reference table over them.
    ///
    /// The rules here are about objects rather than about bytes, so the shortest honest way to
    /// exercise them is a real file the real parser opens — which is also what checks the one
    /// piece of machinery no hand-built dictionary reaches: a graphics state written as its own
    /// indirect object, recognised only because a resource dictionary's `ExtGState` table names
    /// it.
    fn document_of(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let number = index.saturating_add(1);
            let _ = writeln!(out, "{number} 0 obj {body} endobj");
        }
        let start = out.len();
        let size = objects.len().saturating_add(1);
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer << /Size {size} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).unwrap_or_else(|_| Document::empty())
    }

    /// What one predicate found, as a count a test can assert on.
    fn found(document: &Document, predicate: fn(&Examination<'_>, &mut Findings)) -> usize {
        let mut findings = Findings::default();
        let exam = Examination::new(document, Target::Four(Flavour::Plain));
        predicate(&exam, &mut findings);
        findings.seen()
    }

    /// A JP2 box: `LBox`, `TBox`, payload. ISO/IEC 15444-1:2000 I.4.
    fn jp2_box(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
        let length = payload.len().saturating_add(8);
        let mut bytes = u32::try_from(length)
            .unwrap_or(u32::MAX)
            .to_be_bytes()
            .to_vec();
        bytes.extend_from_slice(&kind);
        bytes.extend_from_slice(payload);
        bytes
    }

    /// An `ihdr` payload of `components` components at `bits`, I.5.3.1 Table I-5.
    fn jp2_image_header(components: u16, bits: u8) -> Vec<u8> {
        let mut payload = 1u32.to_be_bytes().to_vec();
        payload.extend_from_slice(&1u32.to_be_bytes());
        payload.extend_from_slice(&components.to_be_bytes());
        payload.extend_from_slice(&[bits, 7, 0, 0]);
        payload
    }

    /// A `colr` payload stating `METH` 1 and an enumerated space, I.5.3.3 Table I-11.
    fn jp2_enumerated_colour(approximation: u8, space: u32) -> Vec<u8> {
        let mut payload = vec![1, 0, approximation];
        payload.extend_from_slice(&space.to_be_bytes());
        payload
    }

    /// A `cdef` payload from `(Cn, Typ, Asoc)` triples, I.5.3.6.
    fn jp2_channels(entries: &[(u16, u16, u16)]) -> Vec<u8> {
        let mut payload = u16::try_from(entries.len())
            .unwrap_or(u16::MAX)
            .to_be_bytes()
            .to_vec();
        for (channel, kind, association) in entries {
            payload.extend_from_slice(&channel.to_be_bytes());
            payload.extend_from_slice(&kind.to_be_bytes());
            payload.extend_from_slice(&association.to_be_bytes());
        }
        payload
    }

    /// A codestream of `SOC` and a `SIZ` segment for `components` components, A.5.1 Table A-9.
    fn jp2_codestream(components: u16) -> Vec<u8> {
        let mut parameters = 0u16.to_be_bytes().to_vec();
        for value in [1u32, 1, 0, 0, 1, 1, 0, 0] {
            parameters.extend_from_slice(&value.to_be_bytes());
        }
        parameters.extend_from_slice(&components.to_be_bytes());
        for _ in 0..components {
            parameters.extend_from_slice(&[7, 1, 1]);
        }
        let length = u16::try_from(parameters.len().saturating_add(2)).unwrap_or(u16::MAX);
        let mut bytes = vec![0xFF, 0x4F, 0xFF, 0x51];
        bytes.extend_from_slice(&length.to_be_bytes());
        bytes.extend_from_slice(&parameters);
        bytes
    }

    /// A whole JP2 file: signature, file type, the given header boxes, a codestream. I.5.
    fn jp2_file(header: &[Vec<u8>], components: u16) -> Vec<u8> {
        let mut inner = Vec::new();
        for part in header {
            inner.extend_from_slice(part);
        }
        let mut bytes = jp2_box(*b"jP  ", &[0x0D, 0x0A, 0x87, 0x0A]);
        bytes.extend_from_slice(&jp2_box(*b"ftyp", b"jp2 \0\0\0\0jp2 "));
        bytes.extend_from_slice(&jp2_box(*b"jp2h", &inner));
        bytes.extend_from_slice(&jp2_box(*b"jp2c", &jp2_codestream(components)));
        bytes
    }

    /// A file holding one `JPXDecode` image `XObject` over `data`.
    ///
    /// Assembled from bytes rather than through [`document_of`] for the obvious reason: JPEG 2000
    /// data is not text, and these rules are about exactly those bytes.
    fn jpx_document(data: &[u8]) -> Document {
        let dict = format!(
            "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /Filter /JPXDecode /Length {} >>",
            data.len()
        );
        let mut image = dict.into_bytes();
        image.extend_from_slice(b" stream\n");
        image.extend_from_slice(data);
        image.extend_from_slice(b"\nendstream");
        let bodies = [
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [] /Count 0 >>".to_vec(),
            image,
        ];

        let mut out = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (index, body) in bodies.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj ", index.saturating_add(1)).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b" endobj\n");
        }
        let start = out.len();
        let size = bodies.len().saturating_add(1);
        out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
        for offset in &offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!("trailer << /Size {size} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n")
                .as_bytes(),
        );
        Document::open(out).unwrap_or_else(|_| Document::empty())
    }

    /// ISO/IEC 15444-1:2000 I.5.3.1 makes `NC` and `Csiz` two statements of one number, and says
    /// a file whose two disagree is not conforming — so a bad count in either is a bad count.
    #[test]
    fn the_channel_count_is_judged_from_both_places_that_state_it() {
        let header = |components| {
            vec![
                jp2_box(*b"ihdr", &jp2_image_header(components, 7)),
                jp2_box(*b"colr", &jp2_enumerated_colour(0, 16)),
            ]
        };
        let agreeing = jpx_document(&jp2_file(&header(3), 3));
        assert_eq!(found(&agreeing, super::jpeg2000_channel_count), 0);

        // The corpus witness's arrangement: a header claiming five, a codestream carrying three.
        let by_the_header = jpx_document(&jp2_file(&header(5), 3));
        assert_eq!(found(&by_the_header, super::jpeg2000_channel_count), 1);

        let by_the_codestream = jpx_document(&jp2_file(&header(3), 5));
        assert_eq!(found(&by_the_codestream, super::jpeg2000_channel_count), 1);
    }

    /// I.5.3.6's `Typ` separates colour channels from opacity, and only the first kind is counted.
    #[test]
    fn an_opacity_channel_is_not_a_colour_channel() {
        let header = vec![
            jp2_box(*b"ihdr", &jp2_image_header(4, 7)),
            jp2_box(*b"colr", &jp2_enumerated_colour(0, 16)),
            jp2_box(
                *b"cdef",
                &jp2_channels(&[(0, 0, 1), (1, 0, 2), (2, 0, 3), (3, 1, 0)]),
            ),
        ];
        let document = jpx_document(&jp2_file(&header, 4));
        assert_eq!(found(&document, super::jpeg2000_channel_count), 0);
    }

    /// The edition trap: ISO 19005 permits a `METH` of 3, which ISO/IEC 15444-1:2000 Table I-9
    /// does not define. The rule implemented is ISO 19005's, so 3 passes and 4 does not.
    #[test]
    fn a_method_of_three_passes_and_a_method_of_four_does_not() {
        let with = |method: u8| {
            let header = vec![
                jp2_box(*b"ihdr", &jp2_image_header(3, 7)),
                jp2_box(*b"colr", &[method, 0, 0]),
            ];
            jpx_document(&jp2_file(&header, 3))
        };
        for method in [1, 2, 3] {
            assert_eq!(
                found(&with(method), super::jpeg2000_colour_specification_method),
                0,
                "METH {method} is one of the three ISO 19005 permits"
            );
        }
        assert_eq!(
            found(&with(4), super::jpeg2000_colour_specification_method),
            1
        );
    }

    /// The rule binds only data stating more than one specification, and asks for exactly one
    /// marked best — which the corpus witness, stating two and marking neither, does not do.
    #[test]
    fn one_specification_needs_no_approximation_and_two_need_exactly_one() {
        let with = |approximations: &[u8]| {
            let mut header = vec![jp2_box(*b"ihdr", &jp2_image_header(3, 7))];
            for approximation in approximations {
                header.push(jp2_box(
                    *b"colr",
                    &jp2_enumerated_colour(*approximation, 16),
                ));
            }
            jpx_document(&jp2_file(&header, 3))
        };
        let rule = super::jpeg2000_one_best_colour_space_specification;
        assert_eq!(found(&with(&[0]), rule), 0);
        assert_eq!(found(&with(&[1, 0]), rule), 0);
        assert_eq!(found(&with(&[0, 0]), rule), 1);
        assert_eq!(found(&with(&[1, 1]), rule), 1);
    }

    /// Enumerated space 19 is forbidden by name; 12 is permitted by name in the sentence beside
    /// it, and 16 is what a conforming witness states.
    #[test]
    fn only_the_cie_jab_enumerated_space_is_refused() {
        let with = |space: u32| {
            let header = vec![
                jp2_box(*b"ihdr", &jp2_image_header(3, 7)),
                jp2_box(*b"colr", &jp2_enumerated_colour(0, space)),
            ];
            jpx_document(&jp2_file(&header, 3))
        };
        assert_eq!(found(&with(16), super::jpeg2000_no_ciejab_colour_space), 0);
        assert_eq!(found(&with(12), super::jpeg2000_no_ciejab_colour_space), 0);
        assert_eq!(found(&with(19), super::jpeg2000_no_ciejab_colour_space), 1);
    }

    /// The two sentences of the bit-depth rule bind different channels, and the tests separate
    /// them: the range every component, the equality the colour channels alone.
    #[test]
    fn the_bit_depth_range_and_the_equality_bind_different_channels() {
        let rule = super::jpeg2000_bit_depth;

        // The corpus witness: a single `BPC` of 0x28, which Table I-6 reads as 41 bits.
        let header = vec![
            jp2_box(*b"ihdr", &jp2_image_header(3, 0x28)),
            jp2_box(*b"colr", &jp2_enumerated_colour(0, 16)),
        ];
        assert_eq!(found(&jpx_document(&jp2_file(&header, 3)), rule), 1);

        // Three colour channels of unequal depth, which the second sentence forbids.
        let header = vec![
            jp2_box(*b"ihdr", &jp2_image_header(3, 0xFF)),
            jp2_box(*b"bpcc", &[7, 7, 0x0B]),
            jp2_box(*b"colr", &jp2_enumerated_colour(0, 16)),
        ];
        assert_eq!(found(&jpx_document(&jp2_file(&header, 3)), rule), 1);

        // The same depths, with the odd one typed as opacity: no sentence is broken.
        let header = vec![
            jp2_box(*b"ihdr", &jp2_image_header(3, 0xFF)),
            jp2_box(*b"bpcc", &[7, 7, 0x0B]),
            jp2_box(*b"colr", &jp2_enumerated_colour(0, 16)),
            jp2_box(*b"cdef", &jp2_channels(&[(0, 0, 1), (1, 0, 2), (2, 1, 0)])),
        ];
        assert_eq!(found(&jpx_document(&jp2_file(&header, 3)), rule), 0);
    }

    /// Data that is not JPEG 2000 at all fails no JPEG 2000 rule here.
    ///
    /// What it breaks is ISO 32000-2 §7.4.9 and the closing sentence of both subclauses, and that
    /// sentence is the row this file leaves unchecked — so a finding here would sit under a
    /// clause that does not state it.
    #[test]
    fn unreadable_jpeg_2000_data_is_passed_over_rather_than_misreported() {
        let document = jpx_document(b"this is not a JP2 file");
        for rule in [
            super::jpeg2000_channel_count,
            super::jpeg2000_colour_specification_method,
            super::jpeg2000_no_ciejab_colour_space,
            super::jpeg2000_bit_depth,
            super::jpeg2000_one_best_colour_space_specification,
        ] {
            assert_eq!(found(&document, rule), 0);
        }
    }

    /// One file breaking one rule in each of the four places the walk has to reach: an output
    /// intent, an indirect graphics state a table names, an image `XObject` and an annotation.
    #[test]
    fn each_kind_of_object_is_reached_and_judged() {
        let document = document_of(&[
            "<< /Type /Catalog /Pages 2 0 R /OutputIntents [4 0 R] >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
              /Resources << /ExtGState << /GS0 5 0 R >> /XObject << /Im0 6 0 R >> >> \
              /Annots [7 0 R] >>",
            "<< /Type /OutputIntent /S /GTS_PDFA1 >>",
            "<< /TR 8 0 R /TR2 /Wobble /BM /Nope /RI /Bogus /HTP [0 0] >>",
            "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /Interpolate true \
              /Intent /Bogus /OPI << >> /Alternates [] /Length 0 >> stream\n\nendstream",
            "<< /Type /Annot /Subtype /Square /Rect [0 0 1 1] /BM /Nope >>",
            "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>",
        ]);
        assert!(
            document.catalog().is_ok(),
            "the test file has to open before it can be judged"
        );

        assert_eq!(
            found(
                &document,
                super::pdfa_output_intent_states_a_destination_profile
            ),
            1,
            "a PDF/A output intent with no destination profile"
        );
        assert_eq!(
            found(&document, super::no_transfer_function_in_a_graphics_state),
            1,
            "the graphics state states no Type, so only the ExtGState table identifies it"
        );
        assert_eq!(
            found(&document, super::no_halftone_phase_in_a_graphics_state),
            1
        );
        assert_eq!(
            found(&document, super::second_transfer_function_is_default),
            1
        );
        assert_eq!(
            found(&document, super::graphics_state_blend_modes_are_defined),
            1
        );
        assert_eq!(
            found(&document, super::annotation_blend_modes_are_defined),
            1
        );
        assert_eq!(
            found(&document, super::rendering_intent_entries_name_one_of_four),
            2,
            "the graphics state's RI and the image's Intent"
        );
        assert_eq!(
            found(&document, super::no_image_alternates_or_opi),
            2,
            "Alternates and OPI are two findings on one image"
        );
        assert_eq!(found(&document, super::image_interpolation_is_off), 1);
        assert_eq!(
            found(&document, super::no_postscript_xobjects),
            0,
            "nothing in this file is a PostScript XObject"
        );
    }

    /// The output intent rules that need more than one entry, and the profile that must be
    /// shared between them.
    #[test]
    fn two_output_intents_have_to_name_one_profile_object() {
        let document = document_of(&[
            "<< /Type /Catalog /Pages 2 0 R /OutputIntents [3 0 R 4 0 R] >>",
            "<< /Type /Pages /Kids [] /Count 0 >>",
            "<< /Type /OutputIntent /S /GTS_PDFA1 /DestOutputProfile 5 0 R >>",
            "<< /Type /OutputIntent /S /GTS_PDFX /DestOutputProfile 6 0 R \
              /DestOutputProfileRef << >> >>",
            "<< /Length 0 >> stream\n\nendstream",
            "<< /Length 0 >> stream\n\nendstream",
        ]);
        assert!(document.catalog().is_ok());
        assert_eq!(
            found(
                &document,
                super::one_destination_profile_per_output_intents_array
            ),
            1,
            "the second entry names a different object from the first"
        );
        assert_eq!(
            found(
                &document,
                super::no_destination_profile_reference_in_a_pdfx_output_intent
            ),
            1
        );
        assert_eq!(found(&document, super::no_destination_profile_reference), 1);
        assert_eq!(
            found(&document, super::destination_profile_class_and_colour_space),
            2,
            "neither empty stream is an ICC profile"
        );
    }

    /// Two Separation arrays naming one colourant, one agreeing and one not.
    #[test]
    fn separations_are_compared_through_indirection() {
        let document = document_of(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << /ColorSpace << \
              /S0 [/Separation /Spot /DeviceGray 6 0 R] \
              /S1 [/Separation /Spot 5 0 R 6 0 R] \
              /S2 [/Separation /Spot /DeviceCMYK 6 0 R] >> >> >>",
            "<< >>",
            "/DeviceGray",
            "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>",
        ]);
        assert!(document.catalog().is_ok());
        assert_eq!(
            found(&document, super::separations_of_one_name_agree),
            1,
            "S1 states the same alternate space through a reference and S2 a different one"
        );
    }

    /// A page carrying the resources and content a colour rule is about, and an sRGB-shaped ICC
    /// profile stream of the given class and colour space.
    fn coloured_page(content: &str, page_extra: &str, output_intent: &str, more: &str) -> Document {
        let stream = format!(
            "<< /Length {} >> stream\n{content}\nendstream",
            content.len()
        );
        let bodies = [
            format!("<< /Type /Catalog /Pages 2 0 R {output_intent} >>"),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
                 {page_extra} >>"
            ),
            stream,
            more.to_owned(),
        ];
        let borrowed: Vec<&str> = bodies.iter().map(String::as_str).collect();
        document_of(&borrowed)
    }

    /// ISO 19005-2 section 6.2.4.3's first sentence, and ISO 19005-4's, disagree by one licence —
    /// so one file is a failure under part 2 and a pass under part 4.
    #[test]
    fn the_blending_space_licenses_a_device_colour_only_under_part_four() {
        let document = coloured_page(
            "1 0 0 rg 0 0 1 1 re f",
            "/Group << /S /Transparency /CS [/CalRGB << /WhitePoint [1 1 1] >>] >>",
            "",
            "<< >>",
        );
        assert!(document.catalog().is_ok());
        assert_eq!(
            found(&document, super::device_rgb_under_part_two),
            1,
            "part 2 admits only a default space or the file's output intent"
        );
        assert_eq!(
            found(&document, super::device_rgb_under_part_four),
            0,
            "part 4 admits the current transparency blending space as well"
        );
        assert_eq!(
            found(&document, super::device_gray_under_part_four),
            0,
            "nothing in this page paints in DeviceGray"
        );
    }

    /// §8.6.5.6's default licenses the colour under both parts, and only its own family's.
    #[test]
    fn a_device_independent_default_licenses_the_family_it_names_and_no_other() {
        let document = coloured_page(
            "1 0 0 rg 0 0 1 1 re B",
            "/Resources << /ColorSpace << /DefaultRGB [/CalRGB << /WhitePoint [1 1 1] >>] >> >>",
            "",
            "<< >>",
        );
        assert!(document.catalog().is_ok());
        assert_eq!(found(&document, super::device_rgb_under_part_two), 0);
        assert_eq!(
            found(&document, super::device_gray_under_part_two),
            1,
            "the stroke is painted in the initial DeviceGray, which DefaultRGB does not reach"
        );
    }

    /// `TechNote 0010` A028: the page's default does not reach a form that states no `Resources`.
    ///
    /// The whole of what the clarification changed is *which* dictionary the default is read
    /// from, and the two parts read it differently — so one file is a failure under ISO 19005-2
    /// and a pass under ISO 19005-4, and a row that quietly went back to the resources in force
    /// would look exactly like one that never moved. The corpus cannot rank this: no document in
    /// `doc/veraPDF-corpus` both states a default colour space and runs a stream against a
    /// dictionary it fell back on, so the sweep is byte-for-byte unchanged either way.
    #[test]
    fn a_default_reaches_a_form_that_states_no_resources_only_under_part_four() {
        let inner = "0 g 0 0 1 1 re f";
        let document = coloured_page(
            "/Fm0 Do",
            "/Resources << /ColorSpace << /DefaultGray [/CalGray << /WhitePoint [1 1 1] >>] >> \
             /XObject << /Fm0 5 0 R >> >>",
            "",
            &format!(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Length {} >> \
                 stream\n{inner}\nendstream",
                inner.len()
            ),
        );
        assert!(document.catalog().is_ok());
        assert_eq!(
            found(&document, super::device_gray_under_part_two),
            1,
            "A028: the form has no explicitly associated resources dictionary, so no DefaultGray \
             is in force inside it"
        );
        assert_eq!(
            found(&document, super::device_gray_under_part_four),
            0,
            "A028 names ISO 19005-2 and ISO 19005-3, and part 4's own section 6.2.2 says nothing \
             about default colour spaces"
        );
        let row = "graphics/device-gray-needs-a-default-or-an-output-intent";
        assert_eq!(
            crate::clarification::clarifying(row, crate::target::Part::Two).map(|it| it.item),
            Some("A026 and A028"),
            "a part 2 verdict says which reading it applied"
        );
        assert_eq!(
            crate::clarification::clarifying(row, crate::target::Part::Four),
            None,
            "and the record is not printed under a part it does not name"
        );
    }

    /// ISO 19005-2 section 6.2.10 and ISO 19005-4 section 6.2.9: a transparent page with no output
    /// intent anywhere needs a Group with a CS, and a page-level output intent answers only part 4.
    #[test]
    fn a_transparent_page_needs_a_blending_space_or_the_output_intent_its_part_admits() {
        let bare = coloured_page(
            "/GS0 gs 0 0 1 1 re f",
            "/Resources << /ExtGState << /GS0 << /ca 0.5 >> >> >>",
            "",
            "<< >>",
        );
        assert!(bare.catalog().is_ok());
        assert_eq!(
            found(&bare, super::a_transparent_page_has_a_blending_space),
            1
        );
        assert_eq!(
            found(
                &bare,
                super::a_transparent_page_has_a_blending_space_or_an_output_intent
            ),
            1
        );

        let grouped = coloured_page(
            "/GS0 gs 0 0 1 1 re f",
            "/Resources << /ExtGState << /GS0 << /ca 0.5 >> >> >> \
             /Group << /S /Transparency /CS [/CalRGB << /WhitePoint [1 1 1] >>] >>",
            "",
            "<< >>",
        );
        assert!(grouped.catalog().is_ok());
        assert_eq!(
            found(&grouped, super::a_transparent_page_has_a_blending_space),
            0
        );
        assert_eq!(
            found(
                &grouped,
                super::group_colour_spaces_obey_the_colour_rules_under_part_two
            ),
            0,
            "a CalRGB blending space is device-independent and breaks no colour rule"
        );
    }

    /// ISO 19005-4 section 6.2.2's third sentence, over the streams the walk actually ran.
    #[test]
    fn a_named_resource_the_associated_dictionary_does_not_define_is_reported() {
        let document = coloured_page("/Fm0 Do", "/Resources << >>", "", "<< >>");
        assert!(document.catalog().is_ok());
        assert_eq!(found(&document, super::named_resources_are_defined), 1);
    }

    /// The same rule binds a PDF/A-2 file, on `TechNote 0010` A002 rather than on part 2's text.
    ///
    /// Pinned as a test because the reach is the whole of what the clarification changed, and a
    /// row that quietly went back to part 4 alone would look exactly like one that never moved.
    #[test]
    fn named_resources_bind_both_parts_and_part_two_cites_the_clarification() {
        let bound = |target| {
            crate::table::binding(target)
                .any(|row| row.id == "graphics/named-resources-are-defined")
        };
        assert!(bound(Target::Two(crate::Level::B)));
        assert!(bound(Target::Four(Flavour::Plain)));
        assert_eq!(
            crate::clarification::clarifying(
                "graphics/named-resources-are-defined",
                crate::target::Part::Two
            )
            .map(|record| record.item),
            Some("A002")
        );
    }

    /// ISO 19005-2 section 6.2.6's third place, and ISO 19005-2 section 6.2.8.1's inline image.
    #[test]
    fn the_operators_and_the_inline_images_a_page_states_are_read() {
        let content = "/Bogus ri BI /W 1 /H 1 /BPC 8 /CS /G /I true /Intent /Bogus ID \x00 EI";
        let document = coloured_page(content, "", "", "<< >>");
        assert!(document.catalog().is_ok());
        assert_eq!(
            found(
                &document,
                super::rendering_intent_operator_names_one_of_four
            ),
            1
        );
        assert_eq!(
            found(&document, super::inline_image_interpolation_is_off),
            1
        );
        assert_eq!(
            found(&document, super::rendering_intent_entries_name_one_of_four),
            1,
            "the inline image's Intent is an image dictionary's Intent"
        );
    }

    /// ISO 19005 section 6.2.4.4's `Colorants` sentence, over the four shapes a component name can
    /// have.
    ///
    /// One file rather than four, because the rule is stated of the colour space rather than of
    /// the page, so every space the objects hold is judged wherever it sits.
    #[test]
    fn a_devicen_space_is_judged_on_which_of_its_components_are_spot_colourants() {
        let document = document_of(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << /ColorSpace \
              << /CS0 4 0 R /CS1 5 0 R /CS2 6 0 R >> >> >>",
            "[/DeviceN [/Spot1 /Spot2] /DeviceRGB 9 0 R << /Colorants << >> >>]",
            "[/DeviceN [/Cyan /Magenta /Yellow /Black /None] /DeviceCMYK 9 0 R]",
            "[/DeviceN [/Spot1 /PrRed] /DeviceRGB 9 0 R << /Subtype /NChannel \
              /Process << /ColorSpace /DeviceRGB /Components [/PrRed] >> \
              /Colorants << /Spot1 [/Separation /Spot1 /DeviceRGB 9 0 R] >> >>]",
            "<< /Type /Nothing >>",
            "<< /Type /Nothing >>",
            "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>",
        ]);
        assert!(document.catalog().is_ok());
        assert_eq!(
            found(
                &document,
                super::spot_colourants_appear_in_the_colorants_dictionary
            ),
            2,
            "the two spot colourants of the first space, and nothing from the other two: the \
             reserved CMYK names and None are never spot, and the third space defines its one \
             spot colourant and declares its other component a process one"
        );
    }

    /// ISO 19005 section 6.2.5's transfer function sentence, in the three positions it reaches.
    #[test]
    fn a_transfer_function_is_judged_by_where_the_halftone_sits() {
        let document = document_of(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
              /Resources << /ExtGState << /GS0 4 0 R /GS1 7 0 R >> >> >>",
            "<< /Type /ExtGState /HT 5 0 R >>",
            "<< /Type /Halftone /HalftoneType 5 /Cyan 6 0 R /PANTONE 8 0 R /Default 6 0 R >>",
            "<< /Type /Halftone /HalftoneType 1 /Frequency 60 /Angle 45 /SpotFunction /Round \
              /TransferFunction /Identity >>",
            "<< /Type /ExtGState /HT 6 0 R >>",
            "<< /Type /Halftone /HalftoneType 1 /Frequency 60 /Angle 45 /SpotFunction /Round >>",
        ]);
        assert!(document.catalog().is_ok());
        assert_eq!(
            found(
                &document,
                super::halftone_transfer_function_only_where_required
            ),
            3,
            "the type 5 halftone states one for Cyan, which is a standard primary; it omits the \
             one PANTONE requires; and the second graphics state makes a halftone carrying one \
             the current halftone parameter. Its Default entry is not judged either way"
        );
    }

    /// A profile whose header a test can write, since only three fields of it are read.
    ///
    /// The 128 bytes ISO 32000-2 §8.6.5.5's Table 67 and `acsp` signature are read out of, all
    /// of them printable so that the fixture stays a string.
    fn profile(class: &str, space: &str) -> String {
        let mut bytes = vec![b' '; 128];
        bytes[12..16].copy_from_slice(class.as_bytes());
        bytes[16..20].copy_from_slice(space.as_bytes());
        bytes[36..40].copy_from_slice(b"acsp");
        String::from_utf8(bytes).unwrap_or_default()
    }

    /// ISO 19005-4 section 6.2.4.2's first sentence, through ISO 32000-2 §8.6.5.5's two tables.
    #[test]
    fn an_icc_based_space_is_judged_on_its_component_count_and_its_profile_type() {
        let good = profile("mntr", "RGB ");
        let linked = profile("link", "RGB ");
        let exotic = profile("mntr", "YCbr");
        let document = document_of(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << /ColorSpace \
              << /CS0 4 0 R /CS1 6 0 R /CS2 8 0 R /CS3 10 0 R >> >> >>",
            "[/ICCBased 5 0 R]",
            &format!("<< /N 3 /Length 128 >> stream\n{good}\nendstream"),
            "[/ICCBased 7 0 R]",
            &format!("<< /N 2 /Length 128 >> stream\n{good}\nendstream"),
            "[/ICCBased 9 0 R]",
            &format!("<< /N 3 /Length 128 >> stream\n{linked}\nendstream"),
            "[/ICCBased 11 0 R]",
            &format!("<< /N 3 /Length 128 >> stream\n{exotic}\nendstream"),
        ]);
        assert!(document.catalog().is_ok());
        assert_eq!(
            found(&document, super::icc_profiles_conform_to_the_base_standard),
            4,
            "the first space is well formed; the second states an N of 2 and then disagrees \
             with its own profile; the third is a device link; the fourth's data colour space \
             is not one of the four"
        );
    }

    /// An empty document meets every implemented row: none of these rules asks a file to state
    /// anything, so a file that states nothing breaks none of them. ISO 32000-2 §7.8.3 lets a page
    /// reach its resources by inheritance and requires every other kind of content stream to carry
    /// the entry, so ISO 19005 section 6.2.2's word *explicitly* is what each of these turns on.
    #[test]
    fn a_stream_that_names_a_resource_without_owning_a_dictionary_is_reported() {
        let inherited = document_of(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 \
             /Resources << /ColorSpace << /CS0 /DeviceRGB >> >> >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>",
            "<< /Length 12 >> stream\n/CS0 cs 0 sc\nendstream",
        ]);
        assert_eq!(
            found(&inherited, super::content_streams_carry_their_own_resources),
            1,
            "the page names /CS0 through a dictionary its ancestor states"
        );
        let stated = document_of(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
             /Resources << /ColorSpace << /CS0 /DeviceRGB >> >> >>",
            "<< /Length 12 >> stream\n/CS0 cs 0 sc\nendstream",
        ]);
        assert_eq!(
            found(&stated, super::content_streams_carry_their_own_resources),
            0
        );
    }

    /// One page with an `ICCBased` CMYK space in force for both sides, painted on one of them.
    fn overprinting_page(content: &str, state: &str) -> Document {
        let stream = format!(
            "<< /Length {} >> stream\n{content}\nendstream",
            content.len()
        );
        let bodies = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
                 /Resources << /ColorSpace << /CS0 [/ICCBased 5 0 R] >> \
                 /ExtGState << /GS0 {state} >> >> >>"
            ),
            stream,
            "<< /N 4 /Length 4 >> stream\nabcd\nendstream".to_owned(),
        ];
        let borrowed: Vec<&str> = bodies.iter().map(String::as_str).collect();
        document_of(&borrowed)
    }

    /// ISO 32000-2 §8.6.7 makes the prohibition act on the side of the operator that paints, so
    /// an overprinting stroke that never strokes is not one.
    #[test]
    fn overprint_mode_one_is_reported_only_for_the_side_that_painted() {
        let fills = overprinting_page(
            "/GS0 gs /CS0 cs 0 0 0 0 scn 0 0 1 1 re f",
            "<< /OP false /op true /OPM 1 >>",
        );
        assert_eq!(
            found(&fills, super::no_overprint_mode_one_under_icc_cmyk),
            1
        );
        let strokes_only_in_the_state = overprinting_page(
            "/GS0 gs /CS0 CS /CS0 cs 0 0 0 0 scn 0 0 1 1 re f",
            "<< /OP true /op false /OPM 1 >>",
        );
        assert_eq!(
            found(
                &strokes_only_in_the_state,
                super::no_overprint_mode_one_under_icc_cmyk
            ),
            0,
            "the stroking space is never stroked with"
        );
        let mode_zero = overprinting_page(
            "/GS0 gs /CS0 cs 0 0 0 0 scn 0 0 1 1 re f",
            "<< /OP true /op true /OPM 0 >>",
        );
        assert_eq!(
            found(&mode_zero, super::no_overprint_mode_one_under_icc_cmyk),
            0
        );
    }

    /// One page whose `/CS0` is an `ICCBased` space formed from `profile`, beside an output
    /// intent whose destination profile is object 6.
    fn duplicating_page(content: &str, space: &str, profiles: &[&str]) -> Document {
        let stream = format!(
            "<< /Length {} >> stream\n{content}\nendstream",
            content.len()
        );
        let mut bodies = vec![
            "<< /Type /Catalog /Pages 2 0 R /OutputIntents [5 0 R] >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
                 /Resources << /ColorSpace << /CS0 {space} >> >> >>"
            ),
            stream,
            "<< /Type /OutputIntent /S /GTS_PDFA1 /DestOutputProfile 6 0 R >>".to_owned(),
        ];
        bodies.extend(profiles.iter().map(|body| (*body).to_owned()));
        let borrowed: Vec<&str> = bodies.iter().map(String::as_str).collect();
        document_of(&borrowed)
    }

    /// ISO 19005-4 section 6.2.4.2's two stated tests, and the two things that take a document out
    /// of the rule's reach: a profile that is not CMYK, and a space nothing selects.
    #[test]
    fn an_icc_space_duplicating_the_current_profile_is_reported_when_the_content_selects_it() {
        let one_profile = ["<< /N 4 /Length 4 >> stream\nabcd\nendstream"];
        let same_reference =
            duplicating_page("/CS0 cs 0 0 0 0 scn", "[/ICCBased 6 0 R]", &one_profile);
        assert_eq!(
            found(
                &same_reference,
                super::no_icc_space_duplicating_a_current_profile
            ),
            1
        );
        // Section 6.2.4.4 sends the alternate space to section 6.2.4.2, and it is the section
        // 6.2.4.4 row that reports it — cited where the sentence that put the restriction there
        // stands.
        let alternate = duplicating_page(
            "/CS0 cs 1 scn",
            "[/Separation /Spot [/ICCBased 6 0 R] 7 0 R]",
            &[
                "<< /N 4 /Length 4 >> stream\nabcd\nendstream",
                "<< /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [1 1 1 1] /N 1 >>",
            ],
        );
        assert_eq!(
            found(
                &alternate,
                super::separation_alternates_duplicating_a_current_profile
            ),
            1
        );
        assert_eq!(
            found(
                &alternate,
                super::no_icc_space_duplicating_a_current_profile
            ),
            0,
            "the section 6.2.4.2 row keeps the spaces the content names directly"
        );
        let unused = duplicating_page("0 0 0 0 k", "[/ICCBased 6 0 R]", &one_profile);
        assert_eq!(
            found(&unused, super::no_icc_space_duplicating_a_current_profile),
            0,
            "the rule binds a space the content uses"
        );
        let two_profiles = [
            "<< /N 4 /Length 4 >> stream\nabcd\nendstream",
            "<< /N 4 /Length 4 >> stream\nabcd\nendstream",
        ];
        let equal_bytes =
            duplicating_page("/CS0 cs 0 0 0 0 scn", "[/ICCBased 7 0 R]", &two_profiles);
        assert_eq!(
            found(
                &equal_bytes,
                super::no_icc_space_duplicating_a_current_profile
            ),
            1,
            "two profiles of equal bytes hash the same however the hash is defined"
        );
        let different_bytes = [
            "<< /N 4 /Length 4 >> stream\nabcd\nendstream",
            "<< /N 4 /Length 4 >> stream\nefgh\nendstream",
        ];
        let differs =
            duplicating_page("/CS0 cs 0 0 0 0 scn", "[/ICCBased 7 0 R]", &different_bytes);
        assert_eq!(
            found(&differs, super::no_icc_space_duplicating_a_current_profile),
            0
        );
        let rgb = ["<< /N 3 /Length 4 >> stream\nabcd\nendstream"];
        let not_cmyk = duplicating_page("/CS0 cs 0 0 0 scn", "[/ICCBased 6 0 R]", &rgb);
        assert_eq!(
            found(&not_cmyk, super::no_icc_space_duplicating_a_current_profile),
            0,
            "the clause binds a CMYK destination profile"
        );
    }

    #[test]
    fn a_document_with_nothing_in_it_breaks_no_graphics_rule() {
        let document = Document::empty();
        let exam = Examination::new(&document, Target::Four(Flavour::Plain));
        for requirement in requirements().filter(|row| row.id.starts_with("graphics/")) {
            if let crate::requirement::Check::Implemented(predicate) = requirement.check {
                let mut findings = Findings::default();
                predicate(&exam, &mut findings);
                assert!(
                    findings.met(),
                    "{} reported {:?} against an empty document",
                    requirement.id,
                    findings.kept()
                );
            }
        }
    }
}
