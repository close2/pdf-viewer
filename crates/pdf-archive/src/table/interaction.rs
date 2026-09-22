//! ISO 19005-2 clauses 6.3 to 6.5 and ISO 19005-4 clauses 6.3 to 6.6: what a document lets a
//! reader *do* — its annotations, its form fields, its signatures and its actions.
//!
//! # The tranche where the two parts disagree, and where they do not
//!
//! Every other area of clause 6 is mostly one rule cited twice. This one is not, and the
//! difference is the whole of PDF/A-4's change of posture: ISO 19005-2 forbids everything that
//! could execute or that reaches outside the file, while ISO 19005-4 permits ECMAScript and
//! additional-actions dictionaries and puts the restriction on *when a processor may run them*
//! instead. The rows below are split wherever that happens, because a row's predicate cannot
//! see which target it is being judged for — a rule the two parts state differently is two
//! rows, each citing the part that states it.
//!
//! The four sharpest divergences, each a row of its own:
//!
//! - **Annotation subtypes.** Part 2 admits the ISO 32000-1 set less `3D`, `Sound`, `Screen`
//!   and `Movie`; part 4 admits the ISO 32000-2 set less `Sound`, `Screen` and `Movie`, and
//!   then confines `3D` and `RichMedia` to PDF/A-4e and `FileAttachment` to PDF/A-4f. Those
//!   last two are [`Applies::Flavours`] rows, written the way the annexes leave them: the
//!   prohibition is the rule, and it binds every flavour the annex does not relax.
//! - **ECMAScript.** Part 2 forbids the action outright; part 4 permits it and constrains its
//!   execution, which is a processor obligation rather than a fact about the file. The same
//!   split, one step milder, holds for `SetOCGState` and `GoTo3DView`: forbidden outright by
//!   part 2, and by part 4 everywhere but a PDF/A-4e file.
//! - **Additional actions.** Part 2 forbids `/AA` on the catalog, on a page and on a widget or
//!   field; part 4 permits it on a widget and limits which trigger keys an `/AA` elsewhere may
//!   hold.
//! - **Annotation appearances.** Part 2 states the have-an-appearance rule itself; part 4's
//!   section 6.3.3 states only the `/N`-only rule and attributes the other to ISO 32000-2 §12.5.2,
//!   whose exempt list is one subtype longer.
//!
//! # The rows no document can fail
//!
//! This is the tranche where a processor's obligations are thickest, because part 4's whole change
//! of posture is to permit a construct and constrain what a reader *does* with it. Seven rows here
//! are [`Check::Processor`] — displaying an annotation's `/Contents`, rendering a form field from
//! its appearance rather than its value, running ECMAScript only on an explicit user action,
//! showing the target of a `GoToR`, `GoToE`, `URI` or `SubmitForm` action, performing the four
//! permitted named actions, and Annex B's two sentences about displaying 3D artwork. They are
//! carried and named rather than left out, and they sit outside the coverage denominator: counting
//! them as debts this crate owes overstated the gap, and counting them as met would claim
//! something about a *program* in a verdict about a file.
//!
//! One row that looks like they do is not one, and the line is worth stating. ISO 19005-4 section
//! 6.4.1's rule about a stripped form's XFDF binds a processor **writing** a file, and a clause
//! that tells a writer what to do constrains the file it produces. It stays `Unchecked`, for a
//! different reason: a document with no XFDF attachment may simply be one nobody stripped, so this
//! crate cannot tell a conforming file from a violating one.
//!
//! # What the walks are bounded by
//!
//! Annotations are those the page tree reaches through `/Annots`, and fields those the
//! catalog's `/AcroForm` reaches through `/Fields`; an annotation dictionary that is in the
//! file but on no page and in no field tree is not visited, because nothing in the document
//! says it is an annotation of anything. Actions are those reachable from a place a reader
//! could perform one — `/OpenAction`, an `/AA` entry, an annotation's or field's `/A`, an
//! outline item's `/A`, the document-level ECMAScript name tree — followed through `/Next`.

use std::collections::BTreeSet;

use pdf_model::Pages;
use pdf_model::outline::{Item, Outline};
use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use crate::Examination;
use crate::finding::{Findings, Where};
use crate::requirement::{Applies, Check, Clauses, Requirement};
use crate::target::{Flavour, Part, Target};

/// The rows this module contributes, which `super::TRANCHES` concatenates.
pub(super) static REQUIREMENTS: &[Requirement] = &[
    Requirement {
        id: "annotations/subtype-defined-in-iso-32000-1",
        asks: "Every annotation shall have a subtype ISO 32000-1 defines, and shall not be a \
               3D, Sound, Screen or Movie annotation.",
        clauses: Clauses::only_two("6.3.1"),
        applies: Applies::Always,
        check: Check::Implemented(subtype_permitted_by_part_two),
    },
    Requirement {
        id: "annotations/subtype-defined-in-iso-32000-2",
        asks: "Every annotation shall have a subtype ISO 32000-2 Table 171 defines, and shall \
               not be a Sound, Screen or Movie annotation.",
        clauses: Clauses::only_four("6.3.1"),
        applies: Applies::Always,
        check: Check::Implemented(subtype_permitted_by_part_four),
    },
    Requirement {
        id: "annotations/three-dimensional-only-in-engineering-files",
        asks: "A 3D or RichMedia annotation shall appear only in a PDF/A-4e file.",
        clauses: Clauses::only_four("6.3.1"),
        applies: Applies::Flavours(&[Flavour::Plain, Flavour::F]),
        check: Check::Implemented(no_three_dimensional_annotation),
    },
    Requirement {
        id: "annotations/file-attachment-only-in-embedded-file-files",
        asks: "A FileAttachment annotation shall appear only in a PDF/A-4f file.",
        clauses: Clauses::only_four("6.3.1"),
        applies: Applies::Flavours(&[Flavour::Plain, Flavour::E]),
        check: Check::Implemented(no_file_attachment_annotation),
    },
    Requirement {
        id: "annotations/three-dimensional-stream-format",
        asks: "A 3D stream dictionary's Subtype shall be U3D or PRC.",
        clauses: Clauses::only_four("B.2.2"),
        applies: Applies::Flavours(&[Flavour::E]),
        check: Check::Implemented(three_dimensional_stream_format),
    },
    Requirement {
        id: "annotations/three-dimensional-artwork-displayed",
        asks: "A processor that can render 3D artwork shall display it as the base standard \
               defines, and one that cannot shall display the annotation's normal appearance.",
        clauses: Clauses::only_four("B.2.1"),
        applies: Applies::Flavours(&[Flavour::E]),
        check: Check::Processor(
            "Annex B's one sentence about a file here is the format rule above; this pair is what \
             the two kinds of processor do with the artwork, and the appearance a file has to \
             carry for the second of them is the section 6.3.3 rows'",
        ),
    },
    Requirement {
        id: "annotations/flags-entry-present",
        asks: "Every annotation whose subtype is not Popup shall state an F entry holding its \
               annotation flags.",
        clauses: Clauses::both("6.3.2", "6.3.2"),
        applies: Applies::Always,
        check: Check::Implemented(flags_entry_present),
    },
    Requirement {
        id: "annotations/printable-and-visible",
        asks: "An annotation's F entry shall set the Print flag and clear the Hidden, \
               Invisible, ToggleNoView and NoView flags.",
        clauses: Clauses::both("6.3.2", "6.3.2"),
        applies: Applies::Always,
        check: Check::Implemented(printable_and_visible),
    },
    Requirement {
        id: "annotations/appearance-dictionary-present",
        asks: "Every annotation, except one whose subtype is Popup or Link and one whose Rect \
               is degenerate, shall have at least one appearance dictionary.",
        clauses: Clauses::only_two("6.3.3"),
        applies: Applies::Always,
        check: Check::Implemented(appearance_dictionary_present_two),
    },
    Requirement {
        id: "annotations/appearance-dictionary-present-from-base-standard",
        asks: "Every annotation, except one whose subtype is Popup, Projection or Link and one \
               whose Rect is degenerate, shall have at least one appearance dictionary.",
        clauses: Clauses::only_four("6.3.3"),
        applies: Applies::Always,
        check: Check::Implemented(appearance_dictionary_present_four),
    },
    Requirement {
        id: "annotations/appearance-dictionary-holds-only-normal",
        asks: "An annotation's appearance dictionary shall contain the N key and no other.",
        clauses: Clauses::both("6.3.3", "6.3.3"),
        applies: Applies::Always,
        check: Check::Implemented(appearance_dictionary_holds_only_normal),
    },
    Requirement {
        id: "annotations/normal-appearance-shape",
        asks: "The N entry shall be an appearance subdictionary for a button field's widget and \
               an appearance stream for every other annotation.",
        clauses: Clauses::both("6.3.3", "6.3.3"),
        applies: Applies::Always,
        check: Check::Implemented(normal_appearance_shape),
    },
    Requirement {
        id: "annotations/appearance-rendered-without-the-other-entries",
        asks: "A processor shall render an annotation from its appearance dictionary alone, \
               ignoring the colour, border, caption and style entries of the annotation \
               dictionary.",
        clauses: Clauses::only_two("6.3.3"),
        applies: Applies::Always,
        check: Check::Processor(
            "the file's half is the three rows above — an appearance is present, its dictionary \
             holds only N, and N has the right shape — and this sentence says what a processor \
             does with the entries it then has to leave alone. Part 4 states no equivalent \
             sentence, which is why this row is part 2's only",
        ),
    },
    Requirement {
        id: "annotations/appearance-graphics-conform",
        asks: "The graphics content of every appearance dictionary shall meet the same colour, \
               image, transparency and font rules as page content.",
        clauses: Clauses::only_four("6.3.3"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "delegated, and the delegation is real rather than promised: the sentence sends an \
             appearance dictionary's graphics to clause 6.2, and clause 6.2's rows already \
             reach it. The content survey walks every /AP entry of every page annotation as the \
             form XObject ISO 32000-2 section 12.5.5 makes it, so the colour, transparency, \
             operator and content-stream rows judge what an appearance stream draws exactly as \
             they judge a page's content, and the object-population rows reach the images, \
             profiles and fonts it uses because those are objects a cross-reference section \
             names. A predicate here would report those same failures under a clause number \
             that adds nothing to them. Two limits, and they are the survey's rather than this \
             row's: an appearance on no page's /Annots is not walked, nor is a soft mask's \
             group or a shading's function anywhere; and a fault inside an appearance is \
             reported against the page the annotation is on",
        ),
    },
    Requirement {
        id: "annotations/contents-displayable",
        asks: "An interactive processor shall offer a way to display the Contents entry of \
               every annotation except a signature widget.",
        clauses: Clauses::both("6.3.4", "6.3.4"),
        applies: Applies::Always,
        check: Check::Processor(
            "a requirement on the interactive processor rather than on the file: no property of \
             a document can satisfy or break it",
        ),
    },
    Requirement {
        id: "forms/field-value-not-used-for-rendering",
        asks: "A conforming processor shall render a form field from its appearance dictionary \
               rather than from the field's value.",
        clauses: Clauses::both("6.4.1", "6.4.1"),
        applies: Applies::Always,
        check: Check::Processor(
            "a requirement on the processor's rendering rather than on the file; what the file \
             must carry for it to be satisfiable is the appearance-dictionary rows above",
        ),
    },
    Requirement {
        id: "forms/no-action-on-widget-or-field",
        asks: "A widget annotation dictionary or field dictionary shall not contain the A key.",
        clauses: Clauses::both("6.4.1", "6.4.1"),
        applies: Applies::Always,
        check: Check::Implemented(no_action_on_widget_or_field),
    },
    Requirement {
        id: "forms/need-appearances-absent-or-false",
        asks: "The interactive form dictionary's NeedAppearances flag shall be absent or false.",
        clauses: Clauses::both("6.4.1", "6.4.1"),
        applies: Applies::Always,
        check: Check::Implemented(need_appearances_absent_or_false),
    },
    Requirement {
        id: "forms/removed-scripts-kept-as-xfdf",
        asks: "A processor that strips ECMAScript actions but keeps the form's logic shall keep \
               it as an XFDF embedded file whose relationship is FormData.",
        clauses: Clauses::only_four("6.4.1"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "it binds a processor that removes scripts while writing a file, so it says nothing \
             about a file being read: a document with no XFDF attachment may simply be one \
             nobody stripped",
        ),
    },
    Requirement {
        id: "forms/no-xfa-key",
        asks: "The interactive form dictionary shall not contain the XFA key.",
        clauses: Clauses::both("6.4.2", "6.4.2"),
        applies: Applies::Always,
        check: Check::Implemented(no_xfa_key),
    },
    Requirement {
        id: "forms/no-needs-rendering",
        asks: "The document catalog dictionary shall not contain the NeedsRendering key.",
        clauses: Clauses::both("6.4.2", "6.4.2"),
        applies: Applies::Always,
        check: Check::Implemented(no_needs_rendering),
    },
    Requirement {
        id: "signatures/signature-widgets-meet-the-annotation-rules",
        asks: "Every annotation belonging to a signature field shall meet the annotation flag \
               and appearance rules.",
        clauses: Clauses::both("6.4.3", "6.5.1"),
        applies: Applies::Always,
        check: Check::Implemented(signature_widgets_meet_the_annotation_rules),
    },
    Requirement {
        id: "signatures/signatures-use-signature-fields",
        asks: "A signature shall be specified through a signature field, as the base standard \
               defines one.",
        clauses: Clauses::both("6.4.3", "6.5.1"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "unimplemented, and what would settle it is a population rather than a reading: \
             every signature dictionary the file holds, compared with the ones the AcroForm's \
             field tree reaches as the V of a field whose FT is Sig. \
             `pdf_signature::signature::signatures` gives the second set today; the first needs a \
             walk of every object that is a signature dictionary, which is the same walk \
             `super::file_structure`'s rows already make over the cross-reference sections and \
             which nothing here reuses yet",
        ),
    },
    Requirement {
        id: "signatures/signing-does-not-break-conformance",
        asks: "A processor generating a signature appearance, or any other object, as part of \
               signing shall not thereby break the file's conformance.",
        clauses: Clauses::both("6.4.3", "6.5.1"),
        applies: Applies::Always,
        check: Check::Processor(
            "a rule about what a signing processor may produce, and the file it produces is \
             judged by every other row of this table. A document in front of a validator has \
             already been signed or has not; nothing in it says what the signer would have done",
        ),
    },
    Requirement {
        id: "signatures/pades-profile",
        asks: "A digital signature shall conform to one of the PAdES profiles of ISO 32000-2 or \
               ISO 14533-3.",
        clauses: Clauses::only_four("6.5.2"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "half of the disjunction is ISO 14533-3, which this project does not hold, so a \
             signature departing from ISO 32000-2 §12.8.3.4 could still meet the clause by the \
             other route; `pdf_signature::signature::pades_departures` already answers the half we \
             can read",
        ),
    },
    Requirement {
        id: "signatures/timestamped-file-follows-the-base-standard",
        asks: "A timestamped conforming file shall follow the base standard's document \
               timestamp clause.",
        clauses: Clauses::only_four("6.5.3"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "unimplemented. The subclause's second sentence is conditional on an aspiration — \
             what a file needs *in order to* be deterministically valid over the long term — so \
             only the first binds a file outright, and it delegates wholly to ISO 32000-2 \
             §12.8.5. `pdf_signature::signature` reads a document timestamp's CMS object already; \
             what is missing is a predicate over §12.8.5's own requirements, which is the same \
             owed reading as the PAdES row above and is better done once for both",
        ),
    },
    Requirement {
        id: "signatures/digest-covers-the-whole-file",
        asks: "A signature's digest shall be computed over the entire file, excluding only the \
               signature value itself.",
        clauses: Clauses::only_two("B.1"),
        applies: Applies::Always,
        check: Check::Implemented(digest_covers_the_whole_file),
    },
    Requirement {
        id: "signatures/signature-is-a-single-signer-cms-object",
        asks: "The signature value shall be a DER-encoded PKCS#7 object carrying at least the \
               signer's X.509 certificate and exactly one signer.",
        clauses: Clauses::only_two("B.1"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "unimplemented, and closable in part from what this tree already reads: \
             `pdf_signature::cms::SignedData` states `signers` and the entries of `certificates`, \
             `pdf_signature::x509::Certificate::is_named_by` matches the signer's `sid` to one of \
             them, and failing to parse at all is its `CmsError`. **What keeps it here is the \
             first word of the sentence, not the count**: *DER-encoded* is a claim about the \
             encoding, and `pdf_signature::der` accepts X.690's indefinite lengths on purpose — it \
             records `Value::had_indefinite_length` and refuses nothing — so a predicate written \
             from that reader could say *parses as CMS* and could not say *is DER*. Judging DER \
             needs the reader to refuse, or to report, every departure X.690 clause 10 names, \
             which is a change in `pdf-model` rather than here. The annex's reference to RFC \
             2315 is the row below, split out so that this half is not hostage to it; and the \
             converter's census row (`crates/pdf-transform/tests/archive_unconsidered.txt`) is \
             owed in the same commit as the predicate, as it was for the digest row above",
        ),
    },
    Requirement {
        id: "signatures/signature-object-conforms-to-pkcs7",
        asks: "The signature value shall be a PKCS#7 object conforming to the specification the \
               annex names.",
        clauses: Clauses::only_two("B.1"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "unimplemented, and a separate sentence of the annex from the single-signer one \
             above — split out of that row's reason so that the half this tree can count is not \
             held hostage to the half it cannot. The annex names **RFC 2315**, which is a \
             narrower object than the RFC 5652 `SignedData` `pdf_signature::cms` reads, and nothing \
             in this tree holds that document; judging a signature against the later RFC and \
             citing the annex would be this crate deciding the two are the same object, which \
             is the claim that would have to be argued first",
        ),
    },
    Requirement {
        id: "signatures/revocation-information-is-a-signed-attribute",
        asks: "Revocation information, and as much of the certificate chain as is available, \
               shall be captured before the signature is completed, and the revocation \
               information shall be a signed attribute of it.",
        clauses: Clauses::only_two("B.1"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "unimplemented. Half the sentence is about the signing process and leaves no trace a \
             file can be judged by; the half that does is the signed attribute, and \
             `pdf_signature::cms::SignedData::has_signed_attribute` would answer it given the \
             object identifier the annex intends. The annex names no identifier, and choosing \
             one from a de-facto convention would be this crate supplying the rule rather than \
             applying it",
        ),
    },
    Requirement {
        id: "signatures/signature-handlers-available",
        asks: "A processor shall be able to call the appropriate signature handler, and shall \
               support the two subfilters the base standard documents.",
        clauses: Clauses::only_two("B.1"),
        applies: Applies::Always,
        check: Check::Processor(
            "an obligation on the program rather than on the file: a conforming file may state \
             either subfilter, and what a processor can do about it is `doc/PLAN.md` section \
             5a's ledger to record",
        ),
    },
    Requirement {
        id: "signatures/signatures-validated-as-the-annex-describes",
        asks: "A processor validating a signature shall compare the document digest, validate \
               the certificate path at the indicated time, and check revocation status.",
        clauses: Clauses::only_two("B.2"),
        applies: Applies::Always,
        check: Check::Processor(
            "the whole subclause is a validation procedure a processor carries out, and no \
             property of a document satisfies or breaks it",
        ),
    },
    Requirement {
        id: "actions/no-launch-multimedia-or-form-actions",
        asks: "No action shall be of type Launch, Sound, Movie, ResetForm, ImportData, Hide, \
               Rendition or Trans.",
        clauses: Clauses::both("6.5.1", "6.6.1"),
        applies: Applies::Always,
        check: Check::Implemented(no_launch_multimedia_or_form_actions),
    },
    Requirement {
        id: "actions/no-deprecated-set-state-or-no-op-actions",
        asks: "No action shall be one of the two deprecated actions earlier PDF specifications \
               called set-state and no-op.",
        clauses: Clauses::both("6.5.1", "6.6.1"),
        applies: Applies::Always,
        check: Check::Implemented(no_deprecated_set_state_or_no_op_actions),
    },
    Requirement {
        id: "actions/no-javascript-action",
        asks: "No action shall be of type JavaScript.",
        clauses: Clauses::only_two("6.5.1"),
        applies: Applies::Always,
        check: Check::Implemented(no_javascript_action),
    },
    Requirement {
        id: "actions/no-optional-content-or-view-action",
        asks: "No action shall be of type SetOCGState or GoTo3DView.",
        clauses: Clauses::only_two("6.5.1"),
        applies: Applies::Always,
        check: Check::Implemented(no_optional_content_or_view_action),
    },
    Requirement {
        id: "actions/optional-content-or-view-action-only-in-engineering-files",
        asks: "A SetOCGState or GoTo3DView action shall appear only in a PDF/A-4e file.",
        clauses: Clauses::only_four("6.6.1"),
        applies: Applies::Flavours(&[Flavour::Plain, Flavour::F]),
        check: Check::Implemented(no_optional_content_or_view_action),
    },
    Requirement {
        id: "actions/named-action-is-page-navigation",
        asks: "A named action shall name one of NextPage, PrevPage, FirstPage and LastPage.",
        clauses: Clauses::both("6.5.1", "6.6.1"),
        applies: Applies::Always,
        check: Check::Implemented(named_action_is_page_navigation),
    },
    Requirement {
        id: "actions/named-actions-performed",
        asks: "A conforming interactive processor shall perform the base standard's action for \
               each of the four named actions the parts leave permitted.",
        clauses: Clauses::both("6.5.1", "6.6.1"),
        applies: Applies::Always,
        check: Check::Processor(
            "the sentence beside the prohibition above, and it faces the other way: what the file \
             may name is the row above, and this is what a reader does when the user invokes one",
        ),
    },
    Requirement {
        id: "actions/no-additional-actions-dictionary",
        asks: "The document catalog, a page, a widget annotation and a field shall not state an \
               AA entry.",
        clauses: Clauses::only_two("6.5.2"),
        applies: Applies::Always,
        check: Check::Implemented(no_additional_actions_dictionary),
    },
    Requirement {
        id: "actions/additional-actions-outside-widgets-hold-only-annotation-triggers",
        asks: "An AA entry on the catalog, a page or a non-widget annotation shall hold no keys \
               but E, X, D, U, Fo and Bl.",
        clauses: Clauses::only_four("6.6.3"),
        applies: Applies::Always,
        check: Check::Implemented(additional_actions_hold_only_annotation_triggers),
    },
    Requirement {
        id: "actions/javascript-only-on-explicit-user-action",
        asks: "An interactive processor shall run an ECMAScript action only when a user invokes \
               it explicitly, and a non-interactive one shall never run it.",
        clauses: Clauses::only_four("6.6.2"),
        applies: Applies::Always,
        check: Check::Processor(
            "a requirement on when a processor executes a script, which no property of a \
             document can satisfy or break",
        ),
    },
    Requirement {
        id: "actions/external-targets-displayable",
        asks: "An interactive processor shall offer a way to display the file, destination and \
               URI a GoToR, GoToE, URI or SubmitForm action names.",
        clauses: Clauses::both("6.5.3", "6.6.4"),
        applies: Applies::Always,
        check: Check::Processor(
            "a requirement on what the interactive processor shows the reader rather than on \
             the file",
        ),
    },
    Requirement {
        id: "actions/a-processor-that-declines-scripts-says-so",
        asks: "An interactive processor that renders 3D content but does not process \
               ECMAScript actions shall tell the user so.",
        clauses: Clauses::only_four("B.3.1"),
        applies: Applies::Flavours(&[Flavour::E]),
        check: Check::Processor(
            "an obligation on the program, and an unusual one: the annex lets a processor \
             decline the scripts a 3D annotation would run and requires it to say that it has. \
             Nothing in a file bears on it",
        ),
    },
    Requirement {
        id: "actions/on-instantiate-script-only-on-explicit-user-action",
        asks: "A processor that runs a 3D stream's OnInstantiate script shall run it only when \
               the user explicitly initiates an action.",
        clauses: Clauses::only_four("B.3.2"),
        applies: Applies::Flavours(&[Flavour::E]),
        check: Check::Processor(
            "the annex permits a processor to ignore the entry outright, so a file may state it \
             and conform; what is constrained is when a processor that honours it may run the \
             script",
        ),
    },
];

/// ISO 32000-2 Table 171's annotation subtypes, less the three both parts forbid outright.
///
/// The `Sound`, `Screen` and `Movie` types are absent because ISO 19005-2 section 6.3.1 and ISO
/// 19005-4 section 6.3.1 each strike them by name, so a file naming one has broken the same rule
/// that a file naming a subtype outside the table has.
static PERMITTED_IN_PART_FOUR: &[&str] = &[
    "Text",
    "Link",
    "FreeText",
    "Line",
    "Square",
    "Circle",
    "Polygon",
    "PolyLine",
    "Highlight",
    "Underline",
    "Squiggly",
    "StrikeOut",
    "Caret",
    "Stamp",
    "Ink",
    "Popup",
    "FileAttachment",
    "Widget",
    "PrinterMark",
    "TrapNet",
    "Watermark",
    "3D",
    "Redact",
    "Projection",
    "RichMedia",
];

/// The two subtypes ISO 32000-2 Table 171 marks `(PDF 2.0)`, and which ISO 32000-1 therefore
/// does not define.
///
/// Subtracting them from [`PERMITTED_IN_PART_FOUR`] is how ISO 19005-2 section 6.3.1's set is
/// obtained: the later table says of each row which edition introduced it, so what is left is the
/// earlier edition's set.
///
/// **This comment used to call that a derivation made "without reading ISO 32000-1, which this
/// project does not hold", and cited `Part::Two`'s note for it — a note that has said the opposite
/// since the owner obtained ISO 32000-1:2008 on 2026-09-07** (`doc/questions/A49`; the text is
/// `doc/PDF32000_2008.pdf` and `doc/md/ISO_32000-1_2008.md`). So the derivation is no longer the
/// only route to the set, and [`the_part_two_subtypes_are_the_ones_iso_32000_1_defines`] now holds
/// it to the table itself, spelled out from that edition's 12.5.6.1. It agrees, which is worth
/// having recorded: a subtraction that has been checked against the document it stands in for is a
/// different claim from one that has not. ADR 0964.
static ADDED_BY_ISO_32000_2: &[&str] = &["Projection", "RichMedia"];

/// The subtype ISO 19005-2 section 6.3.1 strikes and ISO 19005-4 section 6.3.1 only confines to
/// PDF/A-4e.
static THREE_DIMENSIONAL: &str = "3D";

/// One annotation a target's section 6.3.1 does not admit, with what it would take off the page.
///
/// **The reading as a population rather than as a verdict**, on the same footing as
/// [`MissingAppearance`] and `crate::properties_outside_their_schema`: a converter that removes
/// these annotations has to be given exactly the ones the requirement reported, and a findings
/// list is capped where a document's annotations are not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForbiddenSubtype {
    /// The object the annotation is, where it is an indirect one.
    ///
    /// `None` for an annotation written directly into a page's `/Annots` array, which nothing
    /// that rewrites objects can reach.
    pub at: Option<ObjectId>,
    /// The zero-based page it is on.
    pub page: usize,
    /// Its `/Subtype`, where it states one.
    pub subtype: Option<String>,
    /// The object its `/AP` `/N` names, where that is a stream of its own.
    ///
    /// The one thing a removal takes off the page that the file could still hold somewhere else:
    /// §12.5.5 makes the normal appearance the marks a reader draws for the annotation, and
    /// `doc/adr/1099` is what keeping them costs. `None` where the annotation states no appearance
    /// stream — a subdictionary of states is `None` too, because no single stream is *the* normal
    /// appearance until the annotation's own `/AS` picks one, and §12.5.5's own words make that a
    /// question about which state the document is in.
    pub normal_appearance: Option<ObjectId>,
}

/// Whether a part's section 6.3.1 admits an annotation of this subtype at all.
///
/// The two rows above, read as one predicate: ISO 19005-2 section 6.3.1 admits the ISO 32000-1
/// set less `3D`, `Sound`, `Screen` and `Movie`, and ISO 19005-4 section 6.3.1 admits the ISO
/// 32000-2 Table 171 set less `Sound`, `Screen` and `Movie`.
///
/// **A part rather than a target, and the flavour conditions are deliberately outside it.** ISO
/// 19005-4 section 6.3.1's later paragraphs confine `3D` and `RichMedia` to PDF/A-4e and
/// `FileAttachment` to PDF/A-4f, and those are two rows of their own because the answer to them is
/// a different one: the flavour that admits the subtype is the shorter route, and for a
/// `FileAttachment` the file it names may stay in the document when the annotation does not. A
/// caller answering *this* predicate's refusals must not be given their population as well.
///
/// `None` — an annotation stating no `/Subtype` — is admitted by neither part: it is of no type
/// either edition's table defines, which is what the first sentence of each clause forbids.
#[must_use]
pub fn annotation_subtype_permitted(subtype: Option<&str>, part: Part) -> bool {
    let Some(subtype) = subtype else {
        return false;
    };
    if !PERMITTED_IN_PART_FOUR.contains(&subtype) {
        return false;
    }
    match part {
        Part::Two => !ADDED_BY_ISO_32000_2.contains(&subtype) && subtype != THREE_DIMENSIONAL,
        Part::Four => true,
    }
}

/// Every annotation of a subtype the target's part does not admit at all.
///
/// The population [`annotation_subtype_permitted`] rejects, in page order, with the normal
/// appearance each one would take off the page.
#[must_use]
pub fn annotations_of_a_forbidden_subtype(
    document: &Document,
    target: Target,
) -> Vec<ForbiddenSubtype> {
    let exam = Examination::new(document, target);
    exam.annotations()
        .iter()
        .filter_map(|annotation| {
            let subtype = name_at(document, &annotation.dict, "Subtype");
            if annotation_subtype_permitted(subtype.as_deref(), target.part()) {
                return None;
            }
            Some(ForbiddenSubtype {
                at: annotation.id,
                page: annotation.page,
                subtype,
                normal_appearance: normal_appearance_stream(document, &annotation.dict),
            })
        })
        .collect()
}

/// The object an annotation's `/AP` `/N` names, where that object is a stream.
fn normal_appearance_stream(document: &Document, annotation: &Dictionary) -> Option<ObjectId> {
    let appearances = document.get_key(annotation, "AP");
    let normal = appearances.as_dict()?.get("N")?;
    let id = normal.as_reference()?;
    document.get(id).as_stream().map(|_| id)
}

/// ISO 32000-2 Table 167's flag values, which are `1 << (bit position - 1)`.
///
/// §12.5.3 states the numbering: bits run "from low-order to high-order, with the lowest-order
/// bit numbered 1", so Table 167's position 3 — `Print` — is the value 4.
const INVISIBLE: i64 = 1;
/// Table 167 bit 2.
const HIDDEN: i64 = 1 << 1;
/// Table 167 bit 3.
const PRINT: i64 = 1 << 2;
/// Table 167 bit 6.
const NO_VIEW: i64 = 1 << 5;
/// Table 167 bit 9.
const TOGGLE_NO_VIEW: i64 = 1 << 8;

/// The four named actions ISO 19005-2 section 6.5.1 and ISO 19005-4 section 6.6.1 leave permitted.
static PERMITTED_NAMED_ACTIONS: &[&str] = &["NextPage", "PrevPage", "FirstPage", "LastPage"];

/// The action types both parts forbid in every file.
static PROHIBITED_ACTIONS: &[&str] = &[
    "Launch",
    "Sound",
    "Movie",
    "ResetForm",
    "ImportData",
    "Hide",
    "Rendition",
    "Trans",
];

/// The two action types ISO 19005-2 forbids outright and ISO 19005-4 confines to PDF/A-4e.
static ENGINEERING_ACTIONS: &[&str] = &["SetOCGState", "GoTo3DView"];

/// ISO 19005-4 section 6.6.3's permitted keys in an additional-actions dictionary outside a widget.
///
/// They are ISO 32000-2 Table 197's annotation triggers, which is why an `/AA` on a catalog or
/// a page can hold none of them: Table 199's `/WC` and Table 198's `/O` are not in this list.
static ANNOTATION_TRIGGERS: &[&str] = &["E", "X", "D", "U", "Fo", "Bl"];

/// How far a `/Parent` chain or a field tree is followed before it is treated as unbounded.
///
/// A field nested thirty-two deep describes no form anybody filled in, and a chain that long
/// in a hostile file is the shape of an exhaustion attack rather than of a document.
const MAX_DEPTH: usize = 32;

// ---------------------------------------------------------------------------------------
// The walks every predicate below is built from.
// ---------------------------------------------------------------------------------------

/// One place inside a page, which is what an annotation's witness needs.
fn on_page(object: Option<ObjectId>, page: usize) -> Where {
    Where {
        object,
        page: Some(page),
        name: None,
    }
}

/// A name entry read as a string a report can print.
fn name_at(document: &Document, dictionary: &Dictionary, key: &str) -> Option<String> {
    document
        .get_key(dictionary, key)
        .as_name()
        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
}

/// One dictionary's keys, as strings a report can print.
fn keys(dictionary: &Dictionary) -> Vec<String> {
    dictionary
        .iter()
        .map(|(key, _)| String::from_utf8_lossy(key.as_bytes()).into_owned())
        .collect()
}

/// Every annotation the page tree reaches, with the page it is on.
///
/// `/Annots` is a page's own array, and a file may name the same annotation twice in it, so the
/// identities seen are kept and a repeat is visited once — the same bound
/// `pdf_model::retrieval::annotations` applies, for the same reason.
fn for_each_annotation(exam: &Examination<'_>, mut visit: impl FnMut(&Where, &Dictionary)) {
    for annotation in exam.annotations() {
        visit(&on_page(annotation.id, annotation.page), &annotation.dict);
    }
}

/// The catalog's interactive form dictionary, where the document states one.
fn acro_form(document: &Document) -> Option<Dictionary> {
    let catalog = document.catalog().ok()?;
    document.get_key(&catalog, "AcroForm").as_dict().cloned()
}

/// Every node of the interactive form's field tree, fields and merged widgets alike.
///
/// §12.7.4.1 lets a terminal field's `/Kids` hold widget annotations rather than fields, and
/// both parts' prohibitions are addressed to "a widget annotation dictionary or field
/// dictionary" — so the walk visits every node and does not try to sort them.
fn for_each_field(document: &Document, mut visit: impl FnMut(&Where, &Dictionary)) {
    let Some(form) = acro_form(document) else {
        return;
    };
    let Object::Array(roots) = document.get_key(&form, "Fields") else {
        return;
    };
    let mut seen = BTreeSet::new();
    for root in &roots {
        descend_field(document, root, 0, &mut seen, &mut visit);
    }
}

/// One node of the field tree, and its `/Kids`.
fn descend_field(
    document: &Document,
    entry: &Object,
    depth: usize,
    seen: &mut BTreeSet<ObjectId>,
    visit: &mut impl FnMut(&Where, &Dictionary),
) {
    if depth >= MAX_DEPTH {
        return;
    }
    if let Some(id) = entry.as_reference()
        && !seen.insert(id)
    {
        return;
    }
    let resolved = document.resolve(entry);
    let Some(dictionary) = resolved.as_dict() else {
        return;
    };
    let place = entry.as_reference().map_or_else(Where::file, Where::object);
    visit(&place, dictionary);
    if let Object::Array(kids) = document.get_key(dictionary, "Kids") {
        for kid in &kids {
            descend_field(document, kid, depth.saturating_add(1), seen, visit);
        }
    }
}

/// Every widget annotation and every field, each visited once.
///
/// The two populations overlap wherever §12.7.4.1's merged dictionary is both, and they are not
/// the same set either way round: a widget can sit in `/Annots` and in no field tree, and a
/// non-terminal field is in the tree and on no page.
fn for_each_widget_or_field(exam: &Examination<'_>, mut visit: impl FnMut(&Where, &Dictionary)) {
    let document = exam.document;
    let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
    for_each_annotation(exam, |place, dictionary| {
        if name_at(document, dictionary, "Subtype").as_deref() != Some("Widget") {
            return;
        }
        if let Some(id) = place.object
            && !seen.insert(id)
        {
            return;
        }
        visit(place, dictionary);
    });
    for_each_field(document, |place, dictionary| {
        if let Some(id) = place.object
            && !seen.insert(id)
        {
            return;
        }
        visit(place, dictionary);
    });
}

/// A field entry, looked up through §12.7.4.1's `/Parent` chain.
///
/// `/FT` is inheritable, so a widget merged into a kid of a terminal field states no `/FT` of
/// its own and the rules that turn on the field type have to climb to find it.
fn inherited(document: &Document, dictionary: &Dictionary, key: &str) -> Option<String> {
    let mut node = dictionary.clone();
    for _ in 0..MAX_DEPTH {
        if let Some(value) = name_at(document, &node, key) {
            return Some(value);
        }
        let parent = document.get_key(&node, "Parent");
        node = parent.as_dict()?.clone();
    }
    None
}

/// Every additional-actions dictionary on the catalog and on a page.
fn for_each_document_additional_actions(
    document: &Document,
    mut visit: impl FnMut(&Where, &Object),
) {
    if let Ok(catalog) = document.catalog()
        && catalog.get("AA").is_some()
    {
        visit(
            &Where::file().named("AA"),
            &document.get_key(&catalog, "AA"),
        );
    }
    let pages = Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        // `/AA` is not one of §7.7.3.4's inheritable entries, so the page's own dictionary is
        // the whole of what it states.
        if page.dict.get("AA").is_some() {
            visit(
                &on_page(page.id, index).named("AA"),
                &document.get_key(&page.dict, "AA"),
            );
        }
    }
}

/// Every action dictionary the document reaches from a place a reader could perform one.
///
/// Both parts' prohibitions are about which actions a *file contains*, so the roots are every
/// entry the base standard defines as holding one, and each root's `/Next` chain is followed:
/// §12.6.2 makes a `/Next` action performed as surely as the action naming it.
fn for_each_action(exam: &Examination<'_>, mut visit: impl FnMut(&Where, &Dictionary)) {
    let document = exam.document;
    let mut roots: Vec<(Where, Object)> = Vec::new();
    let pages = Pages::new(document);
    if let Ok(catalog) = document.catalog() {
        if let Some(open) = catalog.get("OpenAction") {
            roots.push((Where::file().named("OpenAction"), open.clone()));
        }
        push_additional_actions(document, &catalog, &Where::file(), &mut roots);
        push_document_scripts(document, &catalog, &mut roots);
        let outline = Outline::read(document, &pages);
        push_outline_actions(document, &outline.items, &mut roots);
    }
    for_each_document_additional_actions(document, |place, actions| {
        if let Some(actions) = actions.as_dict() {
            for (key, entry) in actions.iter() {
                let trigger = String::from_utf8_lossy(key.as_bytes()).into_owned();
                roots.push((place.clone().named(trigger), entry.clone()));
            }
        }
    });
    for_each_annotation(exam, |place, annotation| {
        push_actions_of(document, annotation, place, &mut roots);
    });
    for_each_field(document, |place, field| {
        push_actions_of(document, field, place, &mut roots);
    });

    let mut seen = BTreeSet::new();
    for (place, root) in &roots {
        descend_action(document, root, place, 0, &mut seen, &mut visit);
    }
}

/// One dictionary's `/A` and every entry of its `/AA`, as action roots.
fn push_actions_of(
    document: &Document,
    dictionary: &Dictionary,
    place: &Where,
    roots: &mut Vec<(Where, Object)>,
) {
    if let Some(action) = dictionary.get("A") {
        roots.push((place.clone().named("A"), action.clone()));
    }
    push_additional_actions(document, dictionary, place, roots);
}

/// Every entry of a dictionary's `/AA`, as action roots.
fn push_additional_actions(
    document: &Document,
    dictionary: &Dictionary,
    place: &Where,
    roots: &mut Vec<(Where, Object)>,
) {
    let additional = document.get_key(dictionary, "AA");
    let Some(additional) = additional.as_dict() else {
        return;
    };
    for (key, entry) in additional.iter() {
        let named = String::from_utf8_lossy(key.as_bytes()).into_owned();
        roots.push((place.clone().named(named), entry.clone()));
    }
}

/// §12.6.4.17's document-level scripts, which are action dictionaries in a name tree.
///
/// The row this serves binds both parts, and the subclause is **12.6.4.16** in
/// ISO 32000-1:2008 — the edition PDF/A-2 adheres to, which calls the action JavaScript rather
/// than ECMAScript. The two editions describe the same name tree under `/JavaScript`.
fn push_document_scripts(
    document: &Document,
    catalog: &Dictionary,
    roots: &mut Vec<(Where, Object)>,
) {
    let tree = document.get_key(catalog, "Names");
    let Some(tree) = tree.as_dict() else {
        return;
    };
    let scripts = document.get_key(tree, "JavaScript");
    let Some(scripts) = scripts.as_dict() else {
        return;
    };
    let resolve = |object: &Object| document.resolve(object);
    for (key, entry) in pdf_syntax::tree::name_entries(scripts, &resolve) {
        let script = String::from_utf8_lossy(&key).into_owned();
        roots.push((Where::file().named(script), entry));
    }
}

/// §12.3.3's outline items, each of which may carry an `/A`.
fn push_outline_actions(document: &Document, items: &[Item], roots: &mut Vec<(Where, Object)>) {
    for item in items {
        if let Some(action) = document
            .get(item.id)
            .as_dict()
            .and_then(|dict| dict.get("A"))
        {
            roots.push((Where::object(item.id).named("A"), action.clone()));
        }
        push_outline_actions(document, &item.children, roots);
    }
}

/// One action, and the `/Next` chain performed after it.
fn descend_action(
    document: &Document,
    entry: &Object,
    place: &Where,
    depth: usize,
    seen: &mut BTreeSet<ObjectId>,
    visit: &mut impl FnMut(&Where, &Dictionary),
) {
    if depth >= MAX_DEPTH {
        return;
    }
    if let Some(id) = entry.as_reference()
        && !seen.insert(id)
    {
        return;
    }
    let here = match entry.as_reference() {
        Some(id) => Where {
            object: Some(id),
            page: place.page,
            name: place.name.clone(),
        },
        None => place.clone(),
    };
    let resolved = document.resolve(entry);
    let Some(dictionary) = resolved.as_dict() else {
        return;
    };
    visit(&here, dictionary);
    let Some(next) = dictionary.get("Next") else {
        return;
    };
    match document.resolve(next) {
        Object::Array(items) => {
            for item in &items {
                descend_action(document, item, &here, depth.saturating_add(1), seen, visit);
            }
        }
        _ => descend_action(document, next, &here, depth.saturating_add(1), seen, visit),
    }
}

// ---------------------------------------------------------------------------------------
// 6.3 Annotations
// ---------------------------------------------------------------------------------------

/// ISO 19005-2 section 6.3.1.
///
/// The permitted set is ISO 32000-2 Table 171 less the two rows that table marks `(PDF 2.0)` —
/// which is ISO 32000-1's set — less the four subtypes the clause strikes by name.
fn subtype_permitted_by_part_two(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_annotation(exam, |place, annotation| {
        let Some(subtype) = name_at(document, annotation, "Subtype") else {
            findings.record(
                place.clone(),
                "an annotation states no Subtype, so it is of no type ISO 32000-1 defines",
            );
            return;
        };
        let known = PERMITTED_IN_PART_FOUR.contains(&subtype.as_str())
            && !ADDED_BY_ISO_32000_2.contains(&subtype.as_str());
        if !known || subtype == THREE_DIMENSIONAL {
            findings.record(
                place.clone().named(subtype),
                "an annotation is of a subtype PDF/A-2 does not permit",
            );
        }
    });
}

/// ISO 19005-4 section 6.3.1's first paragraph.
///
/// `3D`, `RichMedia` and `FileAttachment` are permitted here: this row is the table membership
/// and the three struck names, and the two rows below carry the flavour conditions the clause's
/// later paragraphs put on those three.
fn subtype_permitted_by_part_four(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_annotation(exam, |place, annotation| {
        let Some(subtype) = name_at(document, annotation, "Subtype") else {
            findings.record(
                place.clone(),
                "an annotation states no Subtype, so it is of no type Table 171 defines",
            );
            return;
        };
        if !PERMITTED_IN_PART_FOUR.contains(&subtype.as_str()) {
            findings.record(
                place.clone().named(subtype),
                "an annotation is of a subtype PDF/A-4 does not permit",
            );
        }
    });
}

/// ISO 19005-4 section 6.3.1's third paragraph, which Annex B relaxes for PDF/A-4e alone.
fn no_three_dimensional_annotation(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_annotation(exam, |place, annotation| {
        let Some(subtype) = name_at(document, annotation, "Subtype") else {
            return;
        };
        if subtype == THREE_DIMENSIONAL || subtype == "RichMedia" {
            findings.record(
                place.clone().named(subtype),
                "a 3D or RichMedia annotation is permitted only in a PDF/A-4e file",
            );
        }
    });
}

/// The two 3D formats ISO 19005-4 section B.2.2 admits, which are also the only two ISO 32000-2's
/// Table 311 recognises.
static THREE_DIMENSIONAL_FORMATS: &[&str] = &["U3D", "PRC"];

/// ISO 19005-4 section B.2.2, the one sentence of Annex B's 3D subclauses that is about a file.
///
/// The rest of §B.2 is addressed to a processor — which artwork it displays, how it colour-manages
/// it — and `CLAUDE.md`'s clause-13 exclusion is about building that. Reading a name out of a
/// dictionary is neither, so this row is implemented while its neighbours are
/// [`Check::Processor`]: the exclusion is on the media engine, not on the validator.
///
/// **The population is where the sentence puts it and no wider.** section B.2.2 names the 3D stream
/// dictionary of ISO 32000-2 §13.6.3, and its own NOTE says that a stream reached from a
/// `RichMedia` assets tree *may* use another format — so a 3D stream is one a 3D annotation names
/// through `/3DD`, or one that declares itself with `/Type /3D`. An embedded file stream carrying
/// 3D data for a `RichMedia` annotation is neither, and is left alone.
///
/// Table 311 makes `/Type` optional and `/Subtype` required, which is why both routes are walked:
/// the `/3DD` route reaches a stream that states no `/Type`, and the `/Type` route reaches one no
/// annotation points at.
fn three_dimensional_stream_format(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    // Gathered before anything is reported, because the two routes overlap: one stream a 3D
    // annotation names and that also declares `/Type /3D` is one stream, and reporting it twice
    // would count two faults against a document that has one.
    let mut judged: BTreeSet<ObjectId> = BTreeSet::new();
    let mut streams: Vec<(Where, Dictionary)> = Vec::new();

    for_each_annotation(exam, |place, annotation| {
        if name_at(document, annotation, "Subtype").as_deref() != Some(THREE_DIMENSIONAL) {
            return;
        }
        if let Some(id) = annotation.get("3DD").and_then(Object::as_reference)
            && !judged.insert(id)
        {
            return;
        }
        if let Some(stream) = document.get_key(annotation, "3DD").as_stream() {
            streams.push((place.clone().named("3DD"), stream.dict.clone()));
        }
    });

    for (id, object) in exam.objects() {
        if let Object::Stream(stream) = object
            && name_at(document, &stream.dict, "Type").as_deref() == Some(THREE_DIMENSIONAL)
            && judged.insert(*id)
        {
            streams.push((Where::object(*id), stream.dict.clone()));
        }
    }

    for (place, stream) in streams {
        match name_at(document, &stream, "Subtype") {
            Some(format) if THREE_DIMENSIONAL_FORMATS.contains(&format.as_str()) => {}
            Some(format) => findings.record(
                place.named(format),
                "a 3D stream states a format other than U3D or PRC",
            ),
            // Table 311 makes the entry required and section B.2.2 requires it to be one of two
            // values, so a stream stating none has satisfied neither.
            None => findings.record(
                place.named("Subtype"),
                "a 3D stream states no Subtype naming its format",
            ),
        }
    }
}

/// ISO 19005-4 section 6.3.1's fifth paragraph, which Annex A relaxes for PDF/A-4f alone.
fn no_file_attachment_annotation(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_annotation(exam, |place, annotation| {
        if name_at(document, annotation, "Subtype").as_deref() == Some("FileAttachment") {
            findings.record(
                place.clone().named("FileAttachment"),
                "a FileAttachment annotation is permitted only in a PDF/A-4f file",
            );
        }
    });
}

/// ISO 19005-2 section 6.3.2, ISO 19005-4 section 6.3.2, first sentence.
///
/// ISO 32000-2 Table 166 makes `/F` optional with a default of 0; both parts make it required
/// on everything but a popup, which is why an absent entry is a finding here rather than a
/// value of zero read from the default.
fn flags_entry_present(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_annotation(exam, |place, annotation| {
        if name_at(document, annotation, "Subtype").as_deref() == Some("Popup") {
            return;
        }
        match document.get_key(annotation, "F") {
            Object::Integer(_) => {}
            Object::Null => findings.record(
                place.clone().named("F"),
                "an annotation states no F entry of annotation flags",
            ),
            other => findings.record(
                place.clone().named("F"),
                format!(
                    "an annotation's F entry is a {} rather than an integer",
                    other.type_name()
                ),
            ),
        }
    });
}

/// ISO 19005-2 section 6.3.2, ISO 19005-4 section 6.3.2, second sentence.
fn printable_and_visible(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_annotation(exam, |place, annotation| {
        let Some(flags) = document.get_key(annotation, "F").as_integer() else {
            return;
        };
        if flags & PRINT == 0 {
            findings.record(
                place.clone().named("Print"),
                "an annotation's F entry leaves the Print flag clear",
            );
        }
        for (bit, name) in [
            (HIDDEN, "Hidden"),
            (INVISIBLE, "Invisible"),
            (TOGGLE_NO_VIEW, "ToggleNoView"),
            (NO_VIEW, "NoView"),
        ] {
            if flags & bit != 0 {
                findings.record(
                    place.clone().named(name),
                    format!("an annotation's F entry sets the {name} flag"),
                );
            }
        }
    });
}

/// Whether `/Rect` states the degenerate rectangle both parts exempt from needing an appearance.
///
/// ISO 32000-2 §12.5.2's own Table 166 states the same two exceptions and settles the reading
/// of this one, in a NOTE dated 2020: the condition joins the two equalities with *and* rather
/// than *or*, to match what the PDF/A parts require. So a rectangle with zero width and a real
/// height is **not** exempt.
fn rect_is_degenerate(document: &Document, annotation: &Dictionary) -> bool {
    let rect = document.get_key(annotation, "Rect");
    let Some(items) = rect.as_array() else {
        return false;
    };
    let at = |index: usize| {
        items
            .get(index)
            .and_then(|item| document.resolve(item).as_number())
    };
    let (Some(first), Some(second), Some(third), Some(fourth)) = (at(0), at(1), at(2), at(3))
    else {
        return false;
    };
    // The clause asks whether the numbers the file *states* are equal, which is exact equality
    // of two values read from the same file rather than a comparison of two computed results.
    #[expect(
        clippy::float_cmp,
        reason = "the rule is about two numbers a producer wrote, not about a tolerance"
    )]
    let degenerate = first == third && second == fourth;
    degenerate
}

/// Whether the annotation states an appearance dictionary at all.
fn has_appearance(document: &Document, annotation: &Dictionary) -> bool {
    matches!(
        document.get_key(annotation, "AP"),
        Object::Dictionary(ref dictionary) if !dictionary.is_empty()
    )
}

/// The body ISO 19005-2 section 6.3.3 and ISO 32000-2 §12.5.2 share, with the exempt subtypes
/// named.
fn appearance_dictionary_present(
    exam: &Examination<'_>,
    findings: &mut Findings,
    exempt: &[&str],
    what: &'static str,
) {
    let document = exam.document;
    for_each_annotation(exam, |place, annotation| {
        if wants_an_appearance(document, annotation, exempt) {
            findings.record(place.clone().named("AP"), what);
        }
    });
}

/// Whether ISO 19005 requires an appearance dictionary of this annotation and it states none.
fn wants_an_appearance(document: &Document, annotation: &Dictionary, exempt: &[&str]) -> bool {
    if name_at(document, annotation, "Subtype")
        .as_deref()
        .is_some_and(|name| exempt.contains(&name))
    {
        return false;
    }
    !rect_is_degenerate(document, annotation) && !has_appearance(document, annotation)
}

/// The subtypes each part's section 6.3.3 exempts from needing an appearance dictionary.
///
/// Part 4's list is longer by `Projection`, which PDF 2.0 added and ISO 19005-2 could not have
/// named — the difference [`appearance_dictionary_present_four`] exists for.
const fn exempt_from_an_appearance(part: Part) -> &'static [&'static str] {
    match part {
        Part::Two => &["Popup", "Link"],
        Part::Four => &["Popup", "Projection", "Link"],
    }
}

/// One annotation ISO 19005 requires an appearance dictionary of, which states none.
///
/// **The reading as a population rather than as a verdict**, on the same footing as
/// `crate::properties_outside_their_schema`: a converter that constructs the missing appearance
/// has to be given exactly the annotations the requirement reported, and a findings list is
/// capped where a document's annotations are not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingAppearance {
    /// The object the annotation is, where it is an indirect one.
    ///
    /// `None` for an annotation written directly into a page's `/Annots` array, which nothing
    /// that rewrites objects can reach.
    pub at: Option<ObjectId>,
    /// The zero-based page it is on.
    pub page: usize,
    /// Its `/Subtype`, where it states one.
    pub subtype: Option<String>,
}

/// Every annotation the target's own section 6.3.3 requires an appearance dictionary of and which
/// states none.
#[must_use]
pub fn annotations_without_an_appearance(
    document: &Document,
    target: Target,
) -> Vec<MissingAppearance> {
    let exam = Examination::new(document, target);
    let exempt = exempt_from_an_appearance(target.part());
    exam.annotations()
        .iter()
        .filter(|annotation| wants_an_appearance(document, &annotation.dict, exempt))
        .map(|annotation| MissingAppearance {
            at: annotation.id,
            page: annotation.page,
            subtype: name_at(document, &annotation.dict, "Subtype"),
        })
        .collect()
}

/// ISO 19005-2 section 6.3.3's first paragraph, whose exempt subtypes are `Popup` and `Link`.
fn appearance_dictionary_present_two(exam: &Examination<'_>, findings: &mut Findings) {
    appearance_dictionary_present(
        exam,
        findings,
        exempt_from_an_appearance(Part::Two),
        "an annotation has no appearance dictionary",
    );
}

/// ISO 19005-4 section 6.3.3, whose NOTE 1 attributes this rule to ISO 32000-2 §12.5.2 rather than
/// restating it.
///
/// **That is a real difference between the parts and not a formality.** Table 166's own wording
/// exempts a third subtype, `Projection`, which PDF 2.0 added and which ISO 19005-2 could not
/// have named; so the part-4 row is a separate row with a longer exempt list, and the clause
/// cited is where PDF/A-4 addresses annotation appearances.
fn appearance_dictionary_present_four(exam: &Examination<'_>, findings: &mut Findings) {
    appearance_dictionary_present(
        exam,
        findings,
        exempt_from_an_appearance(Part::Four),
        "an annotation has no appearance dictionary",
    );
}

/// ISO 19005-2 section 6.3.3, ISO 19005-4 section 6.3.3: an appearance dictionary holds `/N` and
/// nothing else.
fn appearance_dictionary_holds_only_normal(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_annotation(exam, |place, annotation| {
        let Object::Dictionary(appearance) = document.get_key(annotation, "AP") else {
            return;
        };
        for key in keys(&appearance) {
            if key != "N" {
                findings.record(
                    place.clone().named(key),
                    "an appearance dictionary states a key other than N",
                );
            }
        }
    });
}

/// ISO 19005-2 section 6.3.3, ISO 19005-4 section 6.3.3: what the `/N` entry's value has to be.
///
/// A button field's widget needs the subdictionary of appearance states ISO 32000-2 §12.7.5.2.3
/// describes — **12.7.4.2.3 in ISO 32000-1:2008**, which is the edition a PDF/A-2 file adheres
/// to and in which the field-type subclauses are one level up — one per value the button takes;
/// every other annotation has one appearance and therefore a stream. `/FT` is inheritable, so the field type is looked up through `/Parent`.
///
/// # Two clarifications, both of which this row already satisfies
///
/// The published sentence is short and two of its readings have been resolved by the ISO working
/// group, which is why the record is cited beside a verdict here:
///
/// - **`TechNote 0010` A012**: a push button has no permanent value and so only one appearance,
///   which reads as though it should carry a stream — and the resolution is that it shall not.
///   Every field of type `Btn` takes an appearance subdictionary as the value of `/N`, a push
///   button included, even where the subdictionary holds a single entry. So the test above is on
///   the field type alone and admits no push-button exception.
/// - **`TechNote 0010` A023**: where a widget is not merged with its field — a radio group's kids,
///   for instance — the annotation dictionary has no `/FT` of its own, and reading the key off the
///   annotation would make the sentence say nothing about it. The resolution reads the field type
///   from the parent form field dictionary in that case, which is what `inherited` does: merged,
///   the key is on the annotation; unmerged, it is found through `/Parent`.
///
/// Both resolutions name parts 1 to 3, and the citation carries their reach. This row binds ISO
/// 19005-4 as well, whose section 6.3.3 states the sentence in the same words about the same
/// inheritable key — so the predicate is one predicate, and under part 4 the reading rests on
/// part 4's own text rather than on a resolution that does not name it.
fn normal_appearance_shape(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_annotation(exam, |place, annotation| {
        let Object::Dictionary(appearance) = document.get_key(annotation, "AP") else {
            return;
        };
        let button = name_at(document, annotation, "Subtype").as_deref() == Some("Widget")
            && inherited(document, annotation, "FT").as_deref() == Some("Btn");
        let normal = appearance
            .get("N")
            .map_or(Object::Null, |entry| document.resolve(entry));
        let wrong = match (&normal, button) {
            (Object::Dictionary(_), true) | (Object::Stream(_), false) => None,
            (Object::Null, _) => Some("an appearance dictionary states no N entry"),
            (_, true) => Some("a button widget's N entry is not an appearance subdictionary"),
            (_, false) => Some("an annotation's N entry is not an appearance stream"),
        };
        if let Some(wrong) = wrong {
            findings.record(place.clone().named("N"), wrong);
        }
    });
}

// ---------------------------------------------------------------------------------------
// 6.4 Interactive forms, and the signature clauses that lean on 6.3
// ---------------------------------------------------------------------------------------

/// ISO 19005-2 section 6.4.1, ISO 19005-4 section 6.4.1.
///
/// The `/A` half only: part 2 forbids `/AA` in the same sentence, but it forbids it in three
/// more places in section 6.5.2 and that is the clause this table cites for it.
fn no_action_on_widget_or_field(exam: &Examination<'_>, findings: &mut Findings) {
    for_each_widget_or_field(exam, |place, dictionary| {
        if dictionary.get("A").is_some() {
            findings.record(
                place.clone().named("A"),
                "a widget annotation or field dictionary states an A entry",
            );
        }
    });
}

/// ISO 19005-2 section 6.4.1, ISO 19005-4 section 6.4.1, last sentence.
fn need_appearances_absent_or_false(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let Some(form) = acro_form(document) else {
        return;
    };
    match document.get_key(&form, "NeedAppearances") {
        Object::Null | Object::Boolean(false) => {}
        _ => findings.record(
            Where::file().named("NeedAppearances"),
            "the interactive form dictionary asks the processor to build appearances",
        ),
    }
}

/// ISO 19005-2 section 6.4.2, ISO 19005-4 section 6.4.2, first sentence.
fn no_xfa_key(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let Some(form) = acro_form(document) else {
        return;
    };
    if form.get("XFA").is_some() {
        findings.record(
            Where::file().named("XFA"),
            "the interactive form dictionary states an XFA key",
        );
    }
}

/// ISO 19005-2 section 6.4.2, ISO 19005-4 section 6.4.2, second sentence.
fn no_needs_rendering(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let Ok(catalog) = document.catalog() else {
        return;
    };
    if catalog.get("NeedsRendering").is_some() {
        findings.record(
            Where::file().named("NeedsRendering"),
            "the document catalog states a NeedsRendering key",
        );
    }
}

/// ISO 19005-2 section 6.4.3, ISO 19005-4 section 6.5.1: a signature field's annotations obey 6.3.2
/// and 6.3.3.
///
/// It is not the annotation rows over again, and the population is why: those walk `/Annots`,
/// and a signature widget the field tree names but no page lists is invisible to them. What is
/// checked here is the same two clauses, over the widgets the *form* reaches.
fn signature_widgets_meet_the_annotation_rules(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_field(document, |place, field| {
        if inherited(document, field, "FT").as_deref() != Some("Sig")
            || name_at(document, field, "Subtype").as_deref() != Some("Widget")
        {
            return;
        }
        match document.get_key(field, "F").as_integer() {
            None => findings.record(
                place.clone().named("F"),
                "a signature field's annotation states no F entry of annotation flags",
            ),
            Some(flags) => {
                if flags & PRINT == 0 {
                    findings.record(
                        place.clone().named("Print"),
                        "a signature field's annotation leaves the Print flag clear",
                    );
                }
                if flags & (HIDDEN | INVISIBLE | TOGGLE_NO_VIEW | NO_VIEW) != 0 {
                    findings.record(
                        place.clone().named("F"),
                        "a signature field's annotation sets a flag that hides it",
                    );
                }
            }
        }
        let Object::Dictionary(appearance) = document.get_key(field, "AP") else {
            return;
        };
        for key in keys(&appearance) {
            if key != "N" {
                findings.record(
                    place.clone().named(key),
                    "a signature field's appearance dictionary states a key other than N",
                );
            }
        }
    });
}

/// ISO 19005-2 Annex B.1, first sentence: a signature's digest takes in every byte of the file,
/// the signature dictionary included, and leaves out nothing but the signature itself — and
/// `/ByteRange` is where the file says so.
///
/// # The reading, which two rounds priced as a predicate and a third found was a clause
///
/// ADRs 0972 and 0981 recorded this row as one predicate away; ADR 0986 found that the
/// predicate depended on a sentence nobody had settled — whether the annex's "entire file"
/// forbids a conforming file from carrying an incremental update after a signature, which
/// would make every signature but the newest in such a file a fault. ADR 1003 settles it, and
/// this is the short form.
///
/// **The annex's sentence is about the moment of signing.** Its own subject is *computing* the
/// digest, and its NOTE 1 says what the sentence is: ISO 32000-1:2008, 12.8.1's recommendation
/// that the range be the entire file, made a requirement. Turning a *should* into a *shall*
/// changes the modal verb and not the subject, and that base clause reads "entire file" as the
/// file that exists when the digest is computed. It says so three times, and a PDF/A-2 file
/// inherits all three unchanged through ISO 19005-2 section 5.1: 12.8.1's NOTE 1, that a signed
/// document modified and saved by incremental update keeps the bytes the original signature's
/// range covers, so the state at signing can be recreated; Table 252's `Changes` entry, that
/// each signature results in an incremental save and later signatures have a greater length;
/// and 12.8.2.2.1, whose `P` values 2 and 3 permit changes after a certification signature.
///
/// **And ISO 19005-2's own section 6.4.3 requires this reading.** It permits a conforming file
/// to contain the signatures 12.8.1 permits — in the plural, and 12.8.1 has approval signatures
/// *follow* a certification signature — so a conforming PDF/A-2 file may hold two signatures,
/// and in such a file the first one's range necessarily stops where the file stopped when it
/// was signed. Reading the annex as forbidding any byte after a signature's range would make
/// section 6.4.3's permission unreachable, which `doc/habits/reading-the-specification.md`
/// names as the reading to reject when two clauses seem to disagree: the one that makes a
/// file's own words mean nothing.
///
/// What the annex adds to the base standard is therefore the *shall* alone: a signer may not
/// choose a smaller range — 12.8.1 says other ranges are not recommended because they do not
/// check for all changes — and NOTE 2 says what the restriction buys, that no byte of the file
/// as signed lies outside the digest but the signature value. On the file in front of a
/// validator that is four checks per signature, each one the annex's own:
///
/// - the range starts at byte zero and has exactly two pairs — one gap, not several, because
///   what is excluded is the signature value and nothing else;
/// - the gap *is* the signature value: the bytes the range leaves out are the `/Contents`
///   string, with or without its angle brackets. The standard says the value is excluded and
///   does not say whether a hexadecimal string's delimiters are the value's bytes, so both
///   shapes are accepted — a deliberate choice, and one every real producer read the same way;
/// - the signature dictionary is inside the range, which follows from the check above: 12.8.1
///   makes the value a direct object of the dictionary, so a dictionary whose `/Contents` is the
///   gap surrounds it;
/// - the range ends where a file ends. At the end of this file, which is the plain case; or at
///   the end of an `%%EOF` marker plus at most one end-of-line, which is where ISO 32000-1:2008,
///   7.5.5 and 7.5.6 end every revision — the file as it was when this signature was computed —
///   with an update appended afterwards, which is what 12.8.1's NOTE 1 describes and is not a
///   fault of this signature. **A marker followed by nothing but white space is not a revision
///   boundary**: then the bytes past the range were the signed file's own last line, and they
///   are outside the digest. A marker followed by an end-of-line and then more file is
///   accepted whichever revision that end-of-line belonged to, because the file cannot say.
///
/// A range that names bytes past the end of the file is one no digest over this file can have
/// been computed with, and is reported as such rather than as a range that stops short.
///
/// # The population
///
/// The form's signature fields, by the walk [`signature_widgets_meet_the_annotation_rules`]
/// makes, because ISO 32000-1:2008, 12.8.1 puts a signature dictionary in a signature field's
/// `/V` and section 6.4.3 requires it. A signature reachable only another way — the corpus has
/// one, referenced from `/Perms` alone — is `signatures/signatures-use-signature-fields`'s
/// finding and not this row's. A field whose `/V` states neither `/ByteRange` nor `/Contents`
/// is prepared and unsigned, and [`pdf_signature::signature::read`] declines it.
///
/// veraPDF's rule for this clause was run on the same shapes, as evidence and not as the
/// target (`CLAUDE.md` principle 5): it passes a signature whose range ends at its own revision's
/// marker with an update appended after it, and fails one whose range stops short of the
/// marker or runs past the file — which is this reading exactly. It fails, and this row
/// accepts, a range that stops at `%%EOF` with the file's own end-of-line after it followed by
/// an update; the paragraph above says why that case is undecidable from the bytes.
fn digest_covers_the_whole_file(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let file = document.bytes();
    let length = u64::try_from(file.len()).unwrap_or(u64::MAX);
    for_each_field(document, |place, field| {
        if inherited(document, field, "FT").as_deref() != Some("Sig") {
            return;
        }
        let value = document.get_key(field, "V");
        let Some(dict) = value.as_dict() else {
            return;
        };
        let Some(signature) = pdf_signature::signature::read(document, dict) else {
            return;
        };
        let place = place.clone().named("ByteRange");
        let [(start, head), (resume, tail)] = signature.byte_range.as_slice() else {
            findings.record(
                place,
                "the signature's ByteRange is not two pairs leaving one gap for the value",
            );
            return;
        };
        if *start != 0 {
            findings.record(
                place,
                "the signature's ByteRange does not start at the first byte of the file",
            );
            return;
        }
        let gap_start = start.saturating_add(*head);
        let end = resume.saturating_add(*tail);
        if *resume < gap_start {
            findings.record(place, "the signature's ByteRange pairs overlap");
            return;
        }
        if end > length {
            findings.record(
                place,
                format!(
                    "the signature's ByteRange runs {} bytes past the end of the file, so no \
                     digest over this file was computed with it",
                    end.saturating_sub(length)
                ),
            );
            return;
        }
        if !gap_is_the_value(file, gap_start, *resume, &signature.contents) {
            findings.record(
                place.clone(),
                "the bytes the signature's ByteRange leaves out are not its Contents value",
            );
        }
        if end != length && !ends_a_signed_revision(file, end) {
            findings.record(
                place,
                format!(
                    "the signature's ByteRange stops {} bytes before the end of the file at a \
                     point that is not the end-of-file marker of a revision",
                    length.saturating_sub(end)
                ),
            );
        }
    });
}

/// Whether the bytes of `file` in `start..end` are the hexadecimal string holding `value`.
///
/// With or without the angle brackets: the `/Contents` string is a direct object of the
/// dictionary, written `<…>`, and a producer's `/ByteRange` gap either includes the delimiters
/// or does not. ISO 32000-1:2008, 7.3.4.3's own rules decode what is between them — white space
/// is ignored, and a missing final digit is zero — so that the comparison is with the value the
/// file states rather than with one spelling of it.
fn gap_is_the_value(file: &pdf_syntax::FileBytes, start: u64, end: u64, value: &[u8]) -> bool {
    let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
        return false;
    };
    if end <= start {
        return false;
    }
    let gap = file.read(start..end);
    let mut digits = gap.as_ref();
    if let [b'<', rest @ ..] = digits {
        digits = rest;
    }
    if let [rest @ .., b'>'] = digits {
        digits = rest;
    }
    let mut decoded = Vec::with_capacity(digits.len() / 2);
    let mut pending = None;
    for &byte in digits {
        let nibble = match byte {
            b'0'..=b'9' => byte.wrapping_sub(b'0'),
            b'a'..=b'f' => byte.wrapping_sub(b'a').saturating_add(10),
            b'A'..=b'F' => byte.wrapping_sub(b'A').saturating_add(10),
            b' ' | b'\t' | b'\r' | b'\n' | 0x0c | 0x00 => continue,
            _ => return false,
        };
        match pending.take() {
            None => pending = Some(nibble),
            Some(high) => decoded.push((high << 4) | nibble),
        }
    }
    if let Some(high) = pending {
        decoded.push(high << 4);
    }
    decoded == value
}

/// Whether byte `end` of `file` is where a revision ended: just after an `%%EOF` marker, or
/// after that marker and one end-of-line — and with something other than white space after it.
///
/// ISO 32000-1:2008, 7.5.5 makes the marker the file's last line and 7.5.6 gives every appended
/// trailer its own, so a range ending here covered the whole of some earlier state of the file.
/// The white-space condition is the other half of the same sentence: a marker with nothing but
/// white space after it is the *current* file's last line, and a range that stops before its
/// end-of-line has left bytes of the signed file outside the digest.
fn ends_a_signed_revision(file: &pdf_syntax::FileBytes, end: u64) -> bool {
    /// `%%EOF`, `\r`, `\n`: the most a marker and one end-of-line occupy.
    const WINDOW: u64 = 7;
    /// How much of what follows is read at a time while looking for a byte that is not white
    /// space.
    const PROBE: usize = 512;
    let Ok(at) = usize::try_from(end) else {
        return false;
    };
    let Ok(from) = usize::try_from(end.saturating_sub(WINDOW)) else {
        return false;
    };
    let before = file.read(from..at);
    let marker_then_line_end = [
        &b"%%EOF"[..],
        &b"%%EOF\r"[..],
        &b"%%EOF\n"[..],
        &b"%%EOF\r\n"[..],
    ]
    .iter()
    .any(|shape| before.ends_with(shape));
    if !marker_then_line_end {
        return false;
    }
    // What follows has to be more file, not the tail of this line: read as much as could be
    // white space before the first byte that is not.
    let mut probe = at;
    while probe < file.len() {
        let next = probe.saturating_add(PROBE).min(file.len());
        let bytes = file.read(probe..next);
        if bytes
            .iter()
            .any(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | 0x0c | 0x00))
        {
            return true;
        }
        probe = next;
    }
    false
}

// ---------------------------------------------------------------------------------------
// 6.5 / 6.6 Actions
// ---------------------------------------------------------------------------------------

/// Reports every action whose `/S` is in `prohibited`.
fn actions_of_type(
    exam: &Examination<'_>,
    findings: &mut Findings,
    prohibited: &[&str],
    what: &'static str,
) {
    let document = exam.document;
    for_each_action(exam, |place, action| {
        let Some(kind) = name_at(document, action, "S") else {
            return;
        };
        if prohibited.contains(&kind.as_str()) {
            findings.record(place.clone().named(kind), what);
        }
    });
}

/// ISO 19005-2 section 6.5.1, ISO 19005-4 section 6.6.1, first sentence of each.
fn no_launch_multimedia_or_form_actions(exam: &Examination<'_>, findings: &mut Findings) {
    actions_of_type(
        exam,
        findings,
        PROHIBITED_ACTIONS,
        "the document contains an action of a type neither part permits",
    );
}

/// ISO 19005-2 section 6.5.1, which ISO 19005-4 section 6.6.2 reverses into a permission.
fn no_javascript_action(exam: &Examination<'_>, findings: &mut Findings) {
    actions_of_type(
        exam,
        findings,
        &["JavaScript"],
        "the document contains a JavaScript action, which PDF/A-2 forbids",
    );
}

/// ISO 19005-2 section 6.5.1, ISO 19005-4 section 6.6.1, the sentence after the list of eight.
///
/// **The names come from a specification, not from another validator, and the route is worth
/// recording** because both parts describe these two actions without naming their `/S` values: part
/// 2 calls them "the deprecated set-state and no- op actions" and part 4 "the obsoleted set-state
/// and no-op actions, that were defined in earlier PDF specifications". ISO 32000-2 dropped both
/// from Table 201, and ISO 32000-1:2008 — which this tree now holds — dropped them from Table 198
/// too, keeping only a NOTE that the set-state action is obsolete. So the earlier specification the
/// sentence points at is Adobe's PDF 1.2, and what this project holds of it is the Arlington PDF
/// Model (`doc/arlington-pdf-model`, the same pinned data `pdf-spec` is generated from):
/// `tsv/1.2/ActionSetState.tsv` and `tsv/1.2/ActionNOP.tsv` each give the `/S` value as a closed
/// choice, and each records that it is documented only in Adobe PDF 1.2 and deprecated there.
///
/// The names are `SetState` and `NOP`. Nothing else in either standard admits an action of either
/// type, so a document naming one has broken this sentence whichever way it is read.
static DEPRECATED_ACTIONS: &[&str] = &["SetState", "NOP"];

/// ISO 19005-2 section 6.5.1, ISO 19005-4 section 6.6.1: the two actions earlier specifications
/// withdrew.
fn no_deprecated_set_state_or_no_op_actions(exam: &Examination<'_>, findings: &mut Findings) {
    actions_of_type(
        exam,
        findings,
        DEPRECATED_ACTIONS,
        "the document contains a set-state or no-op action, which earlier PDF specifications \
         withdrew and both parts forbid by name",
    );
}

/// ISO 19005-2 section 6.5.1 and ISO 19005-4 section 6.6.1's second paragraph, which differ only in
/// reach.
///
/// One predicate, two rows: part 2 forbids these two types in every file, and part 4 forbids
/// them in every file but a PDF/A-4e one, which is an [`Applies`] difference rather than a
/// different rule.
fn no_optional_content_or_view_action(exam: &Examination<'_>, findings: &mut Findings) {
    actions_of_type(
        exam,
        findings,
        ENGINEERING_ACTIONS,
        "the document contains a SetOCGState or GoTo3DView action",
    );
}

/// ISO 19005-2 section 6.5.1, ISO 19005-4 section 6.6.1: the four named actions and no others.
fn named_action_is_page_navigation(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_action(exam, |place, action| {
        if name_at(document, action, "S").as_deref() != Some("Named") {
            return;
        }
        let named = name_at(document, action, "N");
        match named {
            Some(named) if PERMITTED_NAMED_ACTIONS.contains(&named.as_str()) => {}
            Some(named) => findings.record(
                place.clone().named(named),
                "a named action names something other than the four page commands",
            ),
            None => findings.record(
                place.clone().named("N"),
                "a named action states no N entry naming what to perform",
            ),
        }
    });
}

/// ISO 19005-2 section 6.5.2, which forbids `/AA` in four places.
///
/// Section 6.4.1 restates the widget-and-field half of this; the table cites section 6.5.2 because
/// that is the clause stating the whole of it. Note what it does *not* reach: an `/AA` on an
/// annotation that is not a widget is outside the four places this clause enumerates.
fn no_additional_actions_dictionary(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_document_additional_actions(document, |place, _| {
        findings.record(
            place.clone(),
            "the document catalog or a page states an AA entry",
        );
    });
    for_each_widget_or_field(exam, |place, dictionary| {
        if dictionary.get("AA").is_some() {
            findings.record(
                place.clone().named("AA"),
                "a widget annotation or field dictionary states an AA entry",
            );
        }
    });
}

/// ISO 19005-4 section 6.6.3's fourth paragraph.
///
/// The permitted keys are Table 197's annotation triggers, so in practice this forbids every
/// key a catalog's or a page's additional-actions dictionary could legitimately hold — Table
/// 199's `/WC`, `/WS`, `/DS`, `/WP` and `/DP` and Table 198's `/O` and `/C` are none of them.
/// A widget's `/AA` is exempt, and the preceding paragraph says so.
fn additional_actions_hold_only_annotation_triggers(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    let mut report = |place: &Where, actions: &Dictionary| {
        for key in keys(actions) {
            if !ANNOTATION_TRIGGERS.contains(&key.as_str()) {
                findings.record(
                    place.clone().named(key),
                    "an additional-actions dictionary outside a widget states a key that is not \
                     one of E, X, D, U, Fo and Bl",
                );
            }
        }
    };
    for_each_document_additional_actions(document, |place, actions| {
        if let Some(actions) = actions.as_dict() {
            report(place, actions);
        }
    });
    for_each_annotation(exam, |place, annotation| {
        if name_at(document, annotation, "Subtype").as_deref() == Some("Widget") {
            return;
        }
        if let Some(actions) = document.get_key(annotation, "AA").as_dict() {
            report(&place.clone().named("AA"), actions);
        }
    });
}

// ---------------------------------------------------------------------------------------
// What a target admits of an action, for the converter that takes the rest out
// ---------------------------------------------------------------------------------------

/// Where an action or an additional-actions dictionary sits.
///
/// The distinction the two parts' action clauses turn on: ISO 19005-2 section 6.5.2 forbids
/// `/AA` on four kinds of holder and ISO 19005-4 section 6.6.3 exempts one of them, so a rule
/// about an `/AA` cannot be answered without knowing which dictionary it is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionHolder {
    /// ISO 32000-2 §7.7.2's document catalog.
    Catalog,
    /// §7.7.3.3's page object.
    Page,
    /// §12.5.6.19's widget annotation, whether or not §12.7.4.1 merged a field into it.
    Widget,
    /// §12.7.4.1's field dictionary that is not also a widget annotation.
    Field,
    /// Any other annotation of a page.
    Annotation,
    /// §12.3.3's outline item.
    OutlineItem,
}

/// What a target's part admits in an additional-actions dictionary at one holder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdditionalActions {
    /// Every key the base standard defines there.
    Permitted,
    /// Only ISO 32000-2 Table 197's `E`, `X`, `D`, `U`, `Fo` and `Bl`.
    OnlyAnnotationTriggers,
    /// No `/AA` entry at all.
    Forbidden,
}

/// One place in a document from which an action can be performed.
///
/// The population [`action_sites`] walks, which is the population every predicate above is
/// judged over: the roots of ISO 32000-2 §12.6's action trees, each with the page it is on
/// where it is on one.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionSite {
    /// The holder's own object, or `None` where the file wrote it directly into its parent.
    pub at: Option<ObjectId>,
    /// Which kind of dictionary it is, which is what the `/AA` rules turn on.
    pub holder: ActionHolder,
    /// The zero-based page the holder is on, where it is on one.
    pub page: Option<usize>,
    /// The holder itself, so that a caller reads the entries off what was walked.
    pub dict: Dictionary,
}

/// Whether the target's part admits an action of this type at all.
///
/// One predicate over the same four lists the rows above report from, so a converter removing
/// what a part forbids cannot disagree with the validator that reported it. `kind` is the
/// action's `/S` and `named` a `Named` action's `/N`.
///
/// **An action stating no `/S` is admitted**, and that is the rows' reading rather than a
/// tolerance invented here: every prohibition names a type, and a dictionary naming none has
/// broken none of them. ISO 32000-2 Table 196 makes `/S` required, which is a base-standard
/// failure and not one of these rows.
#[must_use]
pub fn action_admitted(kind: Option<&str>, named: Option<&str>, target: Target) -> bool {
    let Some(kind) = kind else {
        return true;
    };
    if PROHIBITED_ACTIONS.contains(&kind) || DEPRECATED_ACTIONS.contains(&kind) {
        return false;
    }
    if kind == "JavaScript" {
        // ISO 19005-2 section 6.5.1 forbids it; ISO 19005-4 section 6.6.2 permits it and moves
        // the restriction onto when a processor may run one.
        return target.part() == Part::Four;
    }
    if ENGINEERING_ACTIONS.contains(&kind) {
        // ISO 19005-4 section 6.6.1's second paragraph confines both to a PDF/A-4e file.
        return target.flavour() == Some(Flavour::E);
    }
    if kind == "Named" {
        return named.is_some_and(|name| PERMITTED_NAMED_ACTIONS.contains(&name));
    }
    true
}

/// What the target's part admits in an `/AA` written in this kind of dictionary.
///
/// ISO 19005-2 section 6.5.2 forbids the entry on the catalog, on a page and on a widget
/// annotation or field dictionary, and reaches no other annotation; ISO 19005-4 section 6.6.3
/// permits it on a widget and admits only the annotation triggers elsewhere. An outline item
/// states no `/AA` in either edition's tables, so neither clause reaches one.
#[must_use]
pub const fn additional_actions_admitted(
    holder: ActionHolder,
    target: Target,
) -> AdditionalActions {
    match holder {
        // §12.3.3's Table 194 gives an outline item no `/AA` at all, so neither clause reaches one.
        ActionHolder::OutlineItem => AdditionalActions::Permitted,
        ActionHolder::Catalog | ActionHolder::Page => match target.part() {
            Part::Two => AdditionalActions::Forbidden,
            Part::Four => AdditionalActions::OnlyAnnotationTriggers,
        },
        // ISO 19005-4 section 6.6.3's paragraph before the key list exempts a widget outright.
        ActionHolder::Widget | ActionHolder::Field => match target.part() {
            Part::Two => AdditionalActions::Forbidden,
            Part::Four => AdditionalActions::Permitted,
        },
        // ISO 19005-2 section 6.5.2 enumerates four places, and an annotation that is not a
        // widget is none of them.
        ActionHolder::Annotation => match target.part() {
            Part::Two => AdditionalActions::Permitted,
            Part::Four => AdditionalActions::OnlyAnnotationTriggers,
        },
    }
}

/// Whether this `/AA` key is one of ISO 32000-2 Table 197's annotation triggers.
#[must_use]
pub fn annotation_trigger(key: &str) -> bool {
    ANNOTATION_TRIGGERS.contains(&key)
}

/// Whether either part admits an `/A` entry written in this kind of dictionary.
///
/// ISO 19005-2 section 6.4.1 and ISO 19005-4 section 6.4.1 both forbid it on a widget annotation
/// or field dictionary; nothing in either part forbids an `/A` anywhere else, so what a link
/// annotation or an outline item states is judged by its action's type alone.
#[must_use]
pub const fn action_entry_admitted(holder: ActionHolder) -> bool {
    !matches!(holder, ActionHolder::Widget | ActionHolder::Field)
}

/// Every place in the document an action can be performed from.
///
/// The walks the rows above are judged over, handed out as a population so that a converter
/// removing what a part forbids visits exactly what the validator read. A holder the file wrote
/// directly into its parent has no object of its own and is reported with `at: None` — a caller
/// that has to *edit* one has nothing to reach.
#[must_use]
pub fn action_sites(document: &Document, target: Target) -> Vec<ActionSite> {
    let exam = Examination::new(document, target);
    let mut out = Vec::new();
    let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
    if document.catalog().is_ok() {
        // §7.5.5 makes the trailer's `/Root` the catalog's own reference, which is what an
        // edit to the catalog has to name.
        out.push(ActionSite {
            at: document
                .trailer()
                .get("Root")
                .and_then(Object::as_reference),
            holder: ActionHolder::Catalog,
            page: None,
            dict: document.catalog().unwrap_or_default(),
        });
    }
    let pages = Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        out.push(ActionSite {
            at: page.id,
            holder: ActionHolder::Page,
            page: Some(index),
            dict: page.dict.clone(),
        });
    }
    for annotation in exam.annotations() {
        let widget = name_at(document, &annotation.dict, "Subtype").as_deref() == Some("Widget");
        if let Some(id) = annotation.id
            && !seen.insert(id)
        {
            continue;
        }
        out.push(ActionSite {
            at: annotation.id,
            holder: if widget {
                ActionHolder::Widget
            } else {
                ActionHolder::Annotation
            },
            page: Some(annotation.page),
            dict: annotation.dict.clone(),
        });
    }
    for_each_field(document, |place, dictionary| {
        if let Some(id) = place.object
            && !seen.insert(id)
        {
            return;
        }
        out.push(ActionSite {
            at: place.object,
            holder: ActionHolder::Field,
            page: None,
            dict: dictionary.clone(),
        });
    });
    let outline = Outline::read(document, &pages);
    push_outline_sites(document, &outline.items, &mut out);
    out
}

/// Every outline item of the tree, as a site of its own.
fn push_outline_sites(document: &Document, items: &[Item], out: &mut Vec<ActionSite>) {
    for item in items {
        if let Some(dict) = document.get(item.id).as_dict() {
            out.push(ActionSite {
                at: Some(item.id),
                holder: ActionHolder::OutlineItem,
                page: None,
                dict: dict.clone(),
            });
        }
        push_outline_sites(document, &item.children, out);
    }
}

#[cfg(test)]
mod tests {
    use crate::Examination;
    use std::collections::BTreeSet;
    use std::fmt::Write as _;

    use pdf_syntax::{Document, ObjectId};

    use crate::finding::Findings;
    use crate::target::{Flavour, Level, Target};

    use crate::target::Part;

    use super::{
        PERMITTED_IN_PART_FOUR, REQUIREMENTS, additional_actions_hold_only_annotation_triggers,
        annotation_subtype_permitted, annotations_of_a_forbidden_subtype,
        appearance_dictionary_holds_only_normal, appearance_dictionary_present_four,
        appearance_dictionary_present_two, flags_entry_present, named_action_is_page_navigation,
        no_action_on_widget_or_field, no_additional_actions_dictionary,
        no_file_attachment_annotation, no_javascript_action, no_launch_multimedia_or_form_actions,
        no_three_dimensional_annotation, normal_appearance_shape, printable_and_visible,
        subtype_permitted_by_part_four, subtype_permitted_by_part_two,
    };
    use super::{
        digest_covers_the_whole_file, no_deprecated_set_state_or_no_op_actions,
        three_dimensional_stream_format,
    };

    /// A file built from its objects, numbered from 1, with `/Root 1 0 R`.
    fn document(objects: &[&str]) -> Document {
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("the fixture is a valid file")
    }

    /// A one-page document whose page carries the annotations given, numbered from object 4.
    fn with_annotations(annotations: &[&str]) -> Document {
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            String::new(),
        ];
        let mut references = String::new();
        for (index, annotation) in annotations.iter().enumerate() {
            let number = index.saturating_add(4);
            let _ = write!(references, "{number} 0 R ");
            objects.push((*annotation).to_owned());
        }
        objects[2] =
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Annots [{references}] >>");
        let borrowed: Vec<&str> = objects.iter().map(String::as_str).collect();
        document(&borrowed)
    }

    /// How many places a predicate reported.
    fn faults(document: &Document, predicate: fn(&Examination<'_>, &mut Findings)) -> usize {
        let mut findings = Findings::default();
        let exam = Examination::new(document, Target::Four(Flavour::Plain));
        predicate(&exam, &mut findings);
        findings.seen()
    }

    /// ISO 32000-1:2008, 12.5.6.1, Table 169, in that table's own order, less the three both
    /// parts strike by name.
    ///
    /// Transcribed from the edition ISO 19005-2 section 5.1 names rather than derived from the
    /// later one, which is the whole point of the test below: [`ADDED_BY_ISO_32000_2`] claims that
    /// subtracting ISO 32000-2 Table 171's two PDF 2.0 rows leaves exactly this, and a claim about
    /// a document is checked against the document.
    static ISO_32000_1_TABLE_169_LESS_THE_THREE_STRUCK: &[&str] = &[
        "Text",
        "Link",
        "FreeText",
        "Line",
        "Square",
        "Circle",
        "Polygon",
        "PolyLine",
        "Highlight",
        "Underline",
        "Squiggly",
        "StrikeOut",
        "Stamp",
        "Caret",
        "Ink",
        "Popup",
        "FileAttachment",
        "Widget",
        "PrinterMark",
        "TrapNet",
        "Watermark",
        "3D",
        "Redact",
    ];

    /// ISO 19005-2 section 6.3.1 admits what ISO 32000-1 defines, less `3D`, `Sound`, `Screen` and
    /// `Movie` — and the set this crate reaches by subtraction is that set.
    #[test]
    fn the_part_two_subtypes_are_the_ones_iso_32000_1_defines() {
        let derived: BTreeSet<&str> = PERMITTED_IN_PART_FOUR
            .iter()
            .copied()
            .filter(|subtype| !super::ADDED_BY_ISO_32000_2.contains(subtype))
            .collect();
        let transcribed: BTreeSet<&str> = ISO_32000_1_TABLE_169_LESS_THE_THREE_STRUCK
            .iter()
            .copied()
            .collect();
        assert_eq!(
            derived, transcribed,
            "subtracting Table 171's PDF 2.0 rows gives ISO 32000-1's Table 169 less the three \
             section 6.3.1 strikes"
        );
    }

    /// [`annotation_subtype_permitted`] answers exactly what the two rows report, subtype for
    /// subtype and part for part.
    ///
    /// **The predicate is what a converter removes annotations by**, so a predicate that drifted
    /// from the rows would have it removing an annotation the requirement passed or leaving one it
    /// failed. The population is derived rather than listed — every subtype either part's table
    /// names, plus the three struck ones and a name no table defines — so a row added to
    /// `PERMITTED_IN_PART_FOUR` is covered without this test being edited (trap 25).
    #[test]
    fn the_predicate_answers_what_the_two_rows_report() {
        let mut subtypes: Vec<&str> = PERMITTED_IN_PART_FOUR.to_vec();
        subtypes.extend(["Sound", "Screen", "Movie", "Salamander"]);
        for subtype in subtypes {
            let file = with_annotations(&[&format!(
                "<< /Type /Annot /Subtype /{subtype} /Rect [0 0 1 1] /F 4 /AP << /N 9 0 R >> >>"
            )]);
            for (part, row) in [
                (
                    Part::Two,
                    subtype_permitted_by_part_two as fn(&Examination<'_>, &mut Findings),
                ),
                (Part::Four, subtype_permitted_by_part_four),
            ] {
                assert_eq!(
                    annotation_subtype_permitted(Some(subtype), part),
                    faults(&file, row) == 0,
                    "{subtype} under {part:?}"
                );
            }
        }
    }

    /// An annotation stating no `/Subtype` is of no type either edition's table defines.
    #[test]
    fn an_annotation_stating_no_subtype_is_permitted_by_neither_part() {
        let file =
            with_annotations(&["<< /Type /Annot /Rect [0 0 1 1] /F 4 /AP << /N 9 0 R >> >>"]);
        for (part, row) in [
            (
                Part::Two,
                subtype_permitted_by_part_two as fn(&Examination<'_>, &mut Findings),
            ),
            (Part::Four, subtype_permitted_by_part_four),
        ] {
            assert!(!annotation_subtype_permitted(None, part));
            assert_eq!(faults(&file, row), 1, "{part:?}");
        }
    }

    /// The population a converter is handed is the annotations the rows fail, with what each drew.
    #[test]
    fn the_population_names_the_annotation_the_page_holds_and_its_normal_appearance() {
        let file = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Annots [4 0 R 5 0 R] >>",
            "<< /Type /Annot /Subtype /Sound /Rect [0 0 1 1] /F 4 /AP << /N 6 0 R >> >>",
            "<< /Type /Annot /Subtype /Square /Rect [0 0 1 1] /F 4 /AP << /N 7 0 R >> >>",
            "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Length 0 >>\nstream\n\nendstream",
            "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Length 0 >>\nstream\n\nendstream",
        ]);
        let forbidden = annotations_of_a_forbidden_subtype(&file, Target::Four(Flavour::Plain));
        assert_eq!(forbidden.len(), 1, "the Square is one part 4 admits");
        let one = &forbidden[0];
        assert_eq!(one.subtype.as_deref(), Some("Sound"));
        assert_eq!(one.page, 0);
        assert_eq!(
            one.normal_appearance,
            Some(ObjectId::new(6, 0)),
            "and the marks a removal would take off the page are named"
        );
    }

    /// An `/AP` whose `/N` is a subdictionary of states names no single normal appearance.
    ///
    /// §12.5.5 makes which of a subdictionary's streams is drawn a question the annotation's own
    /// `/AS` answers, so no one stream is *the* normal appearance until it does.
    #[test]
    fn a_normal_appearance_that_is_a_subdictionary_of_states_names_no_stream() {
        let file = with_annotations(&[
            "<< /Type /Annot /Subtype /Sound /Rect [0 0 1 1] /F 4 /AP << /N << /On 5 0 R >> >> >>",
        ]);
        let forbidden = annotations_of_a_forbidden_subtype(&file, Target::Four(Flavour::Plain));
        assert_eq!(forbidden.len(), 1);
        assert_eq!(forbidden[0].normal_appearance, None);
    }

    /// ISO 19005-4 section 6.3.1 admits the two subtypes PDF 2.0 added; ISO 19005-2 section 6.3.1
    /// cannot.
    #[test]
    fn the_two_parts_disagree_about_the_subtypes_pdf_2_added() {
        let file = with_annotations(&[
            "<< /Type /Annot /Subtype /Projection /Rect [0 0 1 1] /F 4 /AP << /N 9 0 R >> >>",
        ]);
        assert_eq!(
            faults(&file, subtype_permitted_by_part_four),
            0,
            "Table 171 lists Projection"
        );
        assert_eq!(
            faults(&file, subtype_permitted_by_part_two),
            1,
            "ISO 32000-1 does not, and PDF/A-2 admits only what it defines"
        );
    }

    /// ISO 19005-2 section 6.5.1, ISO 19005-4 section 6.6.1: the two actions earlier specifications
    /// withdrew.
    ///
    /// The `/S` values are the ones the Arlington PDF Model's Adobe PDF 1.2 tables give — see
    /// [`DEPRECATED_ACTIONS`] — and they are what the corpus's own witnesses for both clauses use.
    #[test]
    fn a_set_state_or_no_op_action_is_found_wherever_a_reader_could_reach_it() {
        for kind in ["SetState", "NOP"] {
            let file = with_annotations(&[&format!(
                "<< /Type /Annot /Subtype /Link /Rect [0 0 1 1] /F 4 /A << /S /{kind} >> >>"
            )]);
            assert_eq!(
                faults(&file, no_deprecated_set_state_or_no_op_actions),
                1,
                "/S /{kind} is one of the two"
            );
        }
        let permitted = with_annotations(&[
            "<< /Type /Annot /Subtype /Link /Rect [0 0 1 1] /F 4 /A << /S /GoTo >> >>",
        ]);
        assert_eq!(
            faults(&permitted, no_deprecated_set_state_or_no_op_actions),
            0
        );
    }

    /// ISO 19005-4 section B.2.2, and the boundary its own NOTE draws.
    ///
    /// A 3D stream is one a 3D annotation names through `/3DD` or one that declares `/Type /3D`;
    /// an embedded file stream carrying 3D data for a `RichMedia` annotation is neither, which is
    /// what the clause's NOTE says in as many words.
    #[test]
    fn a_three_dimensional_stream_states_one_of_the_two_formats_annex_b_admits() {
        let with_format = |subtype: &str| {
            document(&[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Annots [4 0 R] >>",
                "<< /Type /Annot /Subtype /3D /Rect [0 0 1 1] /F 4 /3DD 5 0 R \
                 /AP << /N 6 0 R >> >>",
                &format!("<< /Type /3D /Subtype /{subtype} /Length 0 >>\nstream\n\nendstream"),
                "<< /Length 0 >>\nstream\n\nendstream",
            ])
        };
        for admitted in ["U3D", "PRC"] {
            assert_eq!(
                faults(&with_format(admitted), three_dimensional_stream_format),
                0
            );
        }
        assert_eq!(
            faults(&with_format("u3d"), three_dimensional_stream_format),
            1,
            "a PDF name is case-sensitive, so u3d is not U3D"
        );

        let rich_media = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Annots [4 0 R] >>",
            "<< /Type /Annot /Subtype /RichMedia /Rect [0 0 1 1] /F 4 /AP << /N 5 0 R >> \
             /RichMediaContent << /Subtype /3D /Assets << /Names [(a.prc) 6 0 R] >> >> >>",
            "<< /Length 0 >>\nstream\n\nendstream",
            "<< /Type /Filespec /F (a.prc) /UF (a.prc) /EF << /F 5 0 R >> >>",
        ]);
        assert_eq!(
            faults(&rich_media, three_dimensional_stream_format),
            0,
            "the clause's NOTE puts a RichMedia asset outside the rule"
        );
    }

    /// Both parts strike the same three multimedia subtypes.
    #[test]
    fn a_screen_annotation_is_refused_by_both_parts() {
        let file = with_annotations(&["<< /Type /Annot /Subtype /Screen /Rect [0 0 1 1] /F 4 >>"]);
        assert_eq!(faults(&file, subtype_permitted_by_part_two), 1);
        assert_eq!(faults(&file, subtype_permitted_by_part_four), 1);
    }

    /// ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2 exempt a popup from stating
    /// flags and nothing else. (A `§` here would name ISO 32000-2, whose own 6.3.2 is about
    /// what makes a PDF processor conforming.)
    #[test]
    fn only_a_popup_may_omit_its_flags() {
        let file = with_annotations(&[
            "<< /Type /Annot /Subtype /Popup /Rect [0 0 1 1] >>",
            "<< /Type /Annot /Subtype /Text /Rect [0 0 1 1] >>",
        ]);
        assert_eq!(faults(&file, flags_entry_present), 1);
    }

    /// The Print flag set, and the four hiding flags clear.
    #[test]
    fn the_flags_that_hide_an_annotation_are_each_reported() {
        // 1 | 2 | 32 | 256 sets Invisible, Hidden, NoView and ToggleNoView and leaves Print clear.
        let file = with_annotations(&["<< /Type /Annot /Subtype /Text /Rect [0 0 1 1] /F 291 >>"]);
        assert_eq!(
            faults(&file, printable_and_visible),
            5,
            "four flags set, and Print left clear"
        );
        let clean = with_annotations(&["<< /Type /Annot /Subtype /Text /Rect [0 0 1 1] /F 4 >>"]);
        assert_eq!(faults(&clean, printable_and_visible), 0);
    }

    /// The degenerate rectangle joins its two equalities with *and*, as ISO 32000-2 §12.5.2's
    /// 2020 NOTE settles.
    #[test]
    fn a_rectangle_flat_in_one_direction_still_needs_an_appearance() {
        let flat = with_annotations(&["<< /Type /Annot /Subtype /Square /Rect [1 2 1 9] /F 4 >>"]);
        assert_eq!(faults(&flat, appearance_dictionary_present_two), 1);
        let point = with_annotations(&["<< /Type /Annot /Subtype /Square /Rect [1 2 1 2] /F 4 >>"]);
        assert_eq!(faults(&point, appearance_dictionary_present_two), 0);
    }

    /// Part 4's exempt list is one subtype longer than part 2's.
    #[test]
    fn a_projection_annotation_needs_no_appearance_under_part_four() {
        let file =
            with_annotations(&["<< /Type /Annot /Subtype /Projection /Rect [0 0 4 4] /F 4 >>"]);
        assert_eq!(faults(&file, appearance_dictionary_present_four), 0);
        assert_eq!(
            faults(&file, appearance_dictionary_present_two),
            1,
            "part 2 exempts only Popup and Link"
        );
    }

    /// A button widget wants a subdictionary of states; everything else wants a stream.
    #[test]
    fn the_normal_appearance_takes_the_shape_the_field_type_asks_for() {
        let button = with_annotations(&[
            "<< /Type /Annot /Subtype /Widget /FT /Btn /Rect [0 0 4 4] /F 4 /AP << /N << /Off 9 0 R >> >> >>",
        ]);
        assert_eq!(faults(&button, normal_appearance_shape), 0);
        let wrong = with_annotations(&[
            "<< /Type /Annot /Subtype /Widget /FT /Tx /Rect [0 0 4 4] /F 4 /AP << /N << /Off 9 0 R >> >> >>",
        ]);
        assert_eq!(
            faults(&wrong, normal_appearance_shape),
            1,
            "a text field's normal appearance is a stream"
        );
    }

    /// `/FT` is inheritable, so a widget kid of a button field is still a button.
    #[test]
    fn the_field_type_is_looked_up_through_the_parent_chain() {
        let file = with_annotations(&[
            "<< /Type /Annot /Subtype /Widget /Parent 5 0 R /Rect [0 0 4 4] /F 4 /AP << /N << /Off 9 0 R >> >> >>",
            "<< /FT /Btn /Kids [4 0 R] >>",
        ]);
        assert_eq!(faults(&file, normal_appearance_shape), 0);
    }

    /// A `/Next` chain is walked, because §12.6.2 performs what it names.
    #[test]
    fn an_action_reached_only_through_next_is_still_in_the_file() {
        let file = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OpenAction 4 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /S /GoTo /D [3 0 R /Fit] /Next 5 0 R >>",
            "<< /S /Launch /F (x) >>",
        ]);
        assert_eq!(faults(&file, no_launch_multimedia_or_form_actions), 1);
    }

    /// The document-level script tree is a place a JavaScript action hides from a page walk.
    #[test]
    fn a_document_level_script_is_found() {
        let file = document(&[
            "<< /Type /Catalog /Pages 2 0 R /Names << /JavaScript << /Names [(one) 4 0 R] >> >> >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /S /JavaScript /JS (app.alert\\(1\\);) >>",
        ]);
        assert_eq!(faults(&file, no_javascript_action), 1);
    }

    /// Only the four page commands are permitted names.
    #[test]
    fn a_named_action_outside_the_four_is_reported() {
        let good = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /Named /N /LastPage >> >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        ]);
        assert_eq!(faults(&good, named_action_is_page_navigation), 0);
        let bad = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /Named /N /Print >> >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        ]);
        assert_eq!(faults(&bad, named_action_is_page_navigation), 1);
    }

    /// Part 2 forbids `/AA` on a page outright; part 4 allows the entry and limits its keys.
    #[test]
    fn the_parts_treat_a_page_additional_actions_dictionary_differently() {
        let file = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /AA << /O 4 0 R >> >>",
            "<< /S /GoTo /D [3 0 R /Fit] >>",
        ]);
        assert_eq!(
            faults(&file, no_additional_actions_dictionary),
            1,
            "PDF/A-2 forbids the entry"
        );
        assert_eq!(
            faults(&file, additional_actions_hold_only_annotation_triggers),
            1,
            "PDF/A-4 permits the entry and not the /O key in it"
        );
    }

    /// A widget's own `/AA` is exempt from part 4's key restriction.
    #[test]
    fn a_widgets_additional_actions_are_left_alone_by_part_four() {
        let file = with_annotations(&[
            "<< /Type /Annot /Subtype /Widget /Rect [0 0 4 4] /F 4 /AA << /K 5 0 R >> >>",
            "<< /S /GoTo /D [3 0 R /Fit] >>",
        ]);
        assert_eq!(
            faults(&file, additional_actions_hold_only_annotation_triggers),
            0
        );
        assert_eq!(
            faults(&file, no_additional_actions_dictionary),
            1,
            "PDF/A-2 forbids it on a widget"
        );
    }

    /// The flavour rows bind exactly the flavours the annexes do not relax.
    #[test]
    fn the_annex_relaxations_land_on_the_flavours_they_name() {
        let row = |id: &str| {
            REQUIREMENTS
                .iter()
                .find(|requirement| requirement.id == id)
                .expect("the row is in this module's table")
        };
        let attachment = row("annotations/file-attachment-only-in-embedded-file-files");
        assert!(attachment.binds(Target::Four(Flavour::Plain)));
        assert!(attachment.binds(Target::Four(Flavour::E)));
        assert!(
            !attachment.binds(Target::Four(Flavour::F)),
            "Annex A is what permits the annotation, so the prohibition stops at PDF/A-4f"
        );
        assert!(
            !attachment.binds(Target::Two(Level::B)),
            "ISO 19005-2 states no such rule: its FileAttachment is permitted outright"
        );
        let three_d = row("annotations/three-dimensional-only-in-engineering-files");
        assert!(three_d.binds(Target::Four(Flavour::Plain)));
        assert!(three_d.binds(Target::Four(Flavour::F)));
        assert!(
            !three_d.binds(Target::Four(Flavour::E)),
            "Annex B permits it"
        );
    }

    /// A file attachment is a subtype part 4 lists and part 2 permits without condition.
    #[test]
    fn a_file_attachment_annotation_is_the_flavour_rows_business_alone() {
        let file = with_annotations(&[
            "<< /Type /Annot /Subtype /FileAttachment /Rect [0 0 4 4] /F 4 /AP << /N 9 0 R >> >>",
        ]);
        assert_eq!(faults(&file, subtype_permitted_by_part_two), 0);
        assert_eq!(faults(&file, subtype_permitted_by_part_four), 0);
        assert_eq!(faults(&file, no_file_attachment_annotation), 1);
        assert_eq!(faults(&file, no_three_dimensional_annotation), 0);
    }

    /// An appearance dictionary holds `/N` and nothing else, in both parts.
    #[test]
    fn a_rollover_appearance_is_a_key_too_many() {
        let file = with_annotations(&[
            "<< /Type /Annot /Subtype /Square /Rect [0 0 4 4] /F 4 /AP << /N 9 0 R /R 9 0 R /D 9 0 R >> >>",
        ]);
        assert_eq!(faults(&file, appearance_dictionary_holds_only_normal), 2);
    }

    /// Both parts forbid `/A` on a widget, and the field tree is walked as well as the page.
    #[test]
    fn an_action_on_a_field_the_page_never_lists_is_still_found() {
        let file = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /FT /Btn /T (b) /A << /S /GoTo /D [3 0 R /Fit] >> >>",
        ]);
        assert_eq!(faults(&file, no_action_on_widget_or_field), 1);
    }

    /// The bytes of a file built from its objects, numbered from 1, with `/Root 1 0 R` — the
    /// same construction as [`document`], kept as bytes so that a test can patch offsets into
    /// them before the file is opened.
    fn file(objects: &[&str]) -> Vec<u8> {
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        out.into_bytes()
    }

    /// Hexadecimal characters reserved for a fixture signature's value.
    const ROOM: usize = 64;

    /// Characters the fixture reserves for its `/ByteRange` array: room for three pairs, so that
    /// a test can state one gap too many.
    const HOLE: usize = "[0000000000 0000000000 0000000000 0000000000 0000000000 0000000000]".len();

    /// A file signed the way ISO 32000-1:2008, 12.8.1 says one is, minus the cryptography: a
    /// form with one signature field whose `/V` states a `/ByteRange` naming everything but the
    /// `/Contents` string, delimiters included, which is where every real producer puts the gap.
    ///
    /// Returns the bytes and the offset of the `/ByteRange` array's first character, so that a
    /// test can overwrite the array — every number is written ten digits wide and the array is
    /// padded to [`HOLE`] for exactly that reason — with a range that breaks the annex in one
    /// particular way.
    fn signed_file() -> (Vec<u8>, usize) {
        let signature = format!(
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /ByteRange [0000000000 0000000000 0000000000 0000000000 0000000000 0000000000] \
             /Contents <{}> >>",
            "ab".repeat(ROOM / 2)
        );
        let mut bytes = file(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [3 0 R] /SigFlags 3 >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /FT /Sig /T (Signature1) /V 4 0 R /Subtype /Widget >>",
            &signature,
        ]);
        let open = bytes
            .windows(11)
            .position(|window| window == b"/Contents <")
            .expect("the /Contents string")
            .saturating_add(10);
        let after = open.saturating_add(ROOM).saturating_add(2);
        let range_at = bytes
            .windows(11)
            .position(|window| window == b"/ByteRange ")
            .expect("the /ByteRange entry")
            .saturating_add(11);
        let tail = bytes.len().saturating_sub(after);
        write_range(&mut bytes, range_at, &[(0, open), (after, tail)]);
        (bytes, range_at)
    }

    /// Overwrites the fixture's `/ByteRange` array in place, ten digits a number.
    fn write_range(bytes: &mut [u8], at: usize, pairs: &[(usize, usize)]) {
        let mut text = String::from("[");
        for (index, (start, length)) in pairs.iter().enumerate() {
            if index > 0 {
                text.push(' ');
            }
            let _ = write!(text, "{start:010} {length:010}");
        }
        text.push(']');
        let hole = HOLE;
        assert!(
            text.len() <= hole,
            "the array fits where the fixture left room"
        );
        while text.len() < hole {
            text.insert(text.len().saturating_sub(1), ' ');
        }
        bytes[at..at.saturating_add(hole)].copy_from_slice(text.as_bytes());
    }

    /// The pairs the fixture's `/ByteRange` states.
    fn pairs_of(bytes: &[u8], at: usize) -> Vec<(usize, usize)> {
        let text = std::str::from_utf8(&bytes[at.saturating_add(1)..at.saturating_add(HOLE - 1)])
            .expect("ten-digit numbers");
        let numbers: Vec<usize> = text
            .split_whitespace()
            .map(|n| n.parse().expect("a number"))
            .collect();
        numbers.chunks(2).map(|pair| (pair[0], pair[1])).collect()
    }

    /// ISO 19005-2 Annex B.1: a range from byte zero to the end of the file, with one gap that
    /// is the `/Contents` string, is the range the annex requires.
    #[test]
    fn a_range_over_the_whole_file_but_the_value_meets_the_annex() {
        let (bytes, _) = signed_file();
        let document = Document::open(bytes).expect("a valid file");
        assert_eq!(faults(&document, digest_covers_the_whole_file), 0);
    }

    /// ISO 32000-1:2008, 12.8.1 NOTE 1: a signed document saved by incremental update keeps the
    /// bytes the signature's range covers, and ISO 19005-2 section 6.4.3 permits the signatures
    /// 12.8.1 permits — so a range ending at its own revision's `%%EOF` with an update after it
    /// covered the entire file *as it was signed*, which is what the annex asks.
    #[test]
    fn an_update_appended_after_the_signature_is_not_the_signatures_fault() {
        let (mut bytes, _) = signed_file();
        let previous = bytes
            .windows(9)
            .rposition(|window| window == b"startxref")
            .expect("startxref");
        let previous: usize = std::str::from_utf8(&bytes[previous + 10..])
            .expect("ascii")
            .lines()
            .next()
            .expect("the offset line")
            .trim()
            .parse()
            .expect("an offset");
        // §7.5.6: the update's own cross-reference section, a trailer restating the previous
        // one's entries plus /Prev, and its own marker.
        let object_at = bytes.len();
        let update = format!(
            "5 0 obj\n<< /Later true >>\nendobj\nxref\n0 1\n0000000000 65535 f \n5 1\n{object_at:010} \
             00000 n \ntrailer\n<< /Size 6 /Root 1 0 R /Prev {previous} >>\nstartxref\n{}\n%%EOF\n",
            object_at + "5 0 obj\n<< /Later true >>\nendobj\n".len()
        );
        bytes.extend_from_slice(update.as_bytes());
        let document = Document::open(bytes).expect("a valid updated file");
        assert!(
            document
                .get_key(document.trailer(), "Prev")
                .as_integer()
                .is_some()
        );
        assert_eq!(faults(&document, digest_covers_the_whole_file), 0);
    }

    /// A range that stops short of the marker has left bytes of the signed file outside the
    /// digest, which is the shape the annex's NOTE 2 says the rule exists to prevent.
    #[test]
    fn a_range_that_stops_before_the_marker_is_reported() {
        let (mut bytes, at) = signed_file();
        let pairs = pairs_of(&bytes, at);
        write_range(&mut bytes, at, &[pairs[0], (pairs[1].0, pairs[1].1 - 3)]);
        let document = Document::open(bytes).expect("a valid file");
        assert_eq!(faults(&document, digest_covers_the_whole_file), 1);
    }

    /// A range that stops at `%%EOF` and leaves the file's own last end-of-line outside the
    /// digest has not covered the entire file: the marker is the current file's last line, not
    /// a boundary an update was appended after.
    #[test]
    fn the_files_own_last_end_of_line_is_part_of_the_entire_file() {
        let (mut bytes, at) = signed_file();
        let pairs = pairs_of(&bytes, at);
        write_range(&mut bytes, at, &[pairs[0], (pairs[1].0, pairs[1].1 - 1)]);
        assert!(bytes.ends_with(b"%%EOF\n"));
        let document = Document::open(bytes).expect("a valid file");
        assert_eq!(faults(&document, digest_covers_the_whole_file), 1);
    }

    /// A range naming bytes the file does not have is one no digest over this file was computed
    /// with — the corpus's `6-1-12-t01-pass-a` has exactly this shape, on a signature no field
    /// reaches.
    #[test]
    fn a_range_past_the_end_of_the_file_is_reported() {
        let (mut bytes, at) = signed_file();
        let pairs = pairs_of(&bytes, at);
        write_range(&mut bytes, at, &[pairs[0], (pairs[1].0, pairs[1].1 + 10)]);
        let document = Document::open(bytes).expect("a valid file");
        assert_eq!(faults(&document, digest_covers_the_whole_file), 1);
    }

    /// The gap has to be the signature value: one that starts a byte early excludes a byte of
    /// the dictionary from the digest and includes a delimiter-less spelling of nothing.
    #[test]
    fn a_gap_that_is_not_the_value_is_reported() {
        let (mut bytes, at) = signed_file();
        let pairs = pairs_of(&bytes, at);
        write_range(&mut bytes, at, &[(0, pairs[0].1 - 1), pairs[1]]);
        let document = Document::open(bytes).expect("a valid file");
        assert_eq!(faults(&document, digest_covers_the_whole_file), 1);
    }

    /// The value's delimiters may be on either side of the gap: a gap over the hexadecimal
    /// digits alone still excludes exactly the signature value.
    #[test]
    fn a_gap_that_leaves_the_delimiters_in_the_digest_still_excludes_the_value() {
        let (mut bytes, at) = signed_file();
        let pairs = pairs_of(&bytes, at);
        write_range(
            &mut bytes,
            at,
            &[(0, pairs[0].1 + 1), (pairs[1].0 - 1, pairs[1].1 + 1)],
        );
        let document = Document::open(bytes).expect("a valid file");
        assert_eq!(faults(&document, digest_covers_the_whole_file), 0);
    }

    /// Two gaps exclude more than the signature value, whatever the second one is.
    #[test]
    fn a_second_gap_is_reported() {
        let (mut bytes, at) = signed_file();
        let pairs = pairs_of(&bytes, at);
        write_range(&mut bytes, at, &[(0, 10), (12, pairs[0].1 - 12), pairs[1]]);
        let document = Document::open(bytes).expect("a valid file");
        assert_eq!(faults(&document, digest_covers_the_whole_file), 1);
    }

    /// A field prepared for a signature and never signed states neither entry, and is not a
    /// signature the annex has anything to say about.
    #[test]
    fn an_unsigned_signature_field_is_not_judged() {
        let document = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [3 0 R] /SigFlags 3 >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /FT /Sig /T (Signature1) /Subtype /Widget /V 4 0 R >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite >>",
        ]);
        assert_eq!(faults(&document, digest_covers_the_whole_file), 0);
    }
}
