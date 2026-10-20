//! The third stage's mechanism: one document rewritten object by object, and serialized.
//!
//! [`Rewrite`] names each thing this converter can do to a file, [`Rewriter`] does it one object at
//! a time, and [`convert`] walks the document through `pdf_syntax`'s serializer with the rewritten
//! objects replacing the source's. The walk is [`crate::optimize`]'s closure walk with one
//! difference — a changed object is `replace`d rather than `copied`, so that every reference to it
//! lands on the new one and the closure follows what the **rewritten** object holds.
//!
//! Nothing here decides anything: [`super::decision`] chose the rewrites and [`super::prepare`]
//! built the objects they insert. What this file owes is that a rewrite reaches every place the
//! requirement it answers is about, and that it changes nothing else.
//!
//! This file keeps [`Rewrite`], the state [`Rewriter`] holds, the walk that reaches every object
//! and the three routing functions (`rewrite`, `rewrite_array` and `rewrite_dictionary`) that decide
//! which concern an object belongs to. Each concern is one module holding an `impl Rewriter`:
//! `values` (an entry restated in place), `appearance_states`, `configurations`, `page_structure`,
//! `attachments`, `annotations`, `catalogue` (the catalogue and the interactive form) and
//! `streams` (a stream's dictionary, filters and data).
mod annotations;
mod appearance_states;
mod attachments;
mod catalogue;
mod configurations;
mod page_structure;
mod streams;
mod values;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use pdf_archive::Target;
use pdf_model::Pages;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::{Assembly, Form, ObjectStreams, Options, Streams};
use pdf_syntax::{Document, Version, serialize::serialize};

use crate::Refusal;

use super::fonts::{Metrics, Substitutes};
use super::jpeg2000::Specifications;
use super::prepare::{
    Amended, Appearances, Cleaned, DefaultCmyk, ExtraAppearanceStates, FieldAppearances,
    FieldsDescribed, ForbiddenAnnotations, Fresh, Headers, HiddenAnnotations, Intent, Metadata,
    Prepared, Respelled, SchemasDescribed,
};
use super::preserve::Composed;
use super::signatures::{ForeignHandlers, Signatures, Site};
use super::sites::{
    self, AppearanceStates, ColorantEntries, CompletedOrders, DescriptorSets, PageResources,
    SharedProfile, StandardEncodings, StandardRouteNames, Type3Renames,
};
use super::to_unicode::DerivedMaps;

/// The deepest a rewritten object's value tree is walked for references.
///
/// [`crate::optimize`]'s number, for its reason: one more than `pdf_syntax::Limits::DEFAULT`'s
/// `max_depth`, so that every object the parser admitted has its references reached.
const MAX_WALK_DEPTH: usize = 257;

/// One rewrite this converter performs, named so that a report can say what was done.
///
/// Each is a *mechanism*; which clause required it comes from the requirement it answers, so
/// nothing here restates a clause number the validator already carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rewrite {
    /// The file header states the version the target's part admits.
    ///
    /// Not an object rewrite at all: every output of this verb is a whole new file, so the
    /// header is written fresh. What is decided is the version it states — `super::version_for`.
    FileHeader,
    /// The catalog's `/Version` is restated in the shape the target requires.
    CatalogVersion,
    /// The whole file is written afresh, in the shape `pdf_syntax::write` emits.
    ///
    /// The remedy for every row of `super::decision::WRITER_EMITS`: nothing is decided about the
    /// document, and there is no place to count, because the rewrite is the file. What it fixes
    /// is the *syntax* a producer wrote — an odd hexadecimal digit count, a `/Length` that
    /// disagrees with the bytes, a keyword's line endings — and it fixes it everywhere at once.
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
    /// Every PostScript `XObject` is written as a form `XObject` with no content, which draws
    /// what it drew and keeps the resource entries naming it defined (`doc/adr/2115`).
    PostScriptXObject,
    /// The catalog's `/Requirements` is removed.
    RequirementsDictionary,
    /// The name dictionary's `/AlternatePresentations` is removed.
    AlternatePresentations,
    /// Every page's `/PresSteps` is removed.
    PresentationSteps,
    /// The destination profile every output intent names, where this reader does not draw
    /// through it, is written as the profile a file stating no intent is given.
    ///
    /// ISO 19005-2 and ISO 19005-4 section 6.2.3's class and colour-space row. §14.11.5 lets a
    /// processor disregard an output intent, and this reader disregards one it cannot parse, so
    /// the page was drawn as though no intent were stated; the profile the `OutputIntent` rewrite
    /// would write is written at the object every entry already names (`doc/adr/2007`).
    DestinationProfileReplaced,
    /// The destination profile every output intent names has its header restated as what its
    /// own bytes are: a version 5 header on a profile written in ICC.1:1998-09's tags and types
    /// written 2.2.0, or a stale `Profile ID` written as the digest of the profile.
    ///
    /// ISO 19005-2 and ISO 19005-4 section 6.2.3's tag and identifier rows, where the header alone
    /// fails them; no transform reads either field this reader draws by (`doc/adr/2021`).
    DestinationProfileRestated,
    /// A PDF/A output intent identified as `Custom` and stating no destination profile names the
    /// profile this conversion's own entry would name.
    ///
    /// ISO 19005-2 and ISO 19005-4 section 6.2.3's first sentence; the entry is this verb's own
    /// construction lacking its profile, and the page was drawn as though no intent were stated
    /// (`doc/adr/2035`).
    DestinationProfileSupplied,
    /// The catalog's `/OutputIntents` gains a PDF/A entry naming a destination profile.
    ///
    /// **The one rewrite of this verb that changes what the file *means*** rather than only what
    /// it holds, which is why it is a `Decision::Stated` and not a `Decision::Mechanical`.
    OutputIntent,
    /// The document's XMP packet states the identification schema the target's part requires,
    /// and is created where the document had none.
    IdentificationSchema,
    /// Every resource dictionary explicitly associated with a content stream gains §8.6.5.6's
    /// `/DefaultCMYK`, written as a `DeviceN` over the four process colourants whose tint
    /// transform is §10.4.2.5's.
    ///
    /// The second `Decision::Stated` of this verb, and the one `doc/questions/A48` answers.
    /// **Nothing on any page moves**: the content stream still says what its producer wrote, and
    /// a `/DefaultCMYK` only tells a reader how to read the four numbers already there.
    DefaultCmyk,
    /// `/DefaultRGB`, written as an `ICCBased` space over the shipped sRGB profile, where the
    /// destination profile in hand is of another family (`doc/adr/2114`).
    ///
    /// A `Decision::Stated` for `DefaultCmyk`'s reason: the content stream is the producer's, and
    /// the entry says how its three numbers are read — as sRGB, which this reader drew them as.
    DefaultRgb,
    /// The catalog states `/MarkInfo` with `/Marked true`, where the file already carries a
    /// structure tree.
    ///
    /// ISO 19005-2 section 6.7.2.2 asks for the entry and nothing else. §14.7.1's Table 353 says
    /// what the entry means, and it is why writing it is not inventing anything:
    ///
    /// > ( Optional ) A flag indicating whether the document conforms to tagged PDF conventions
    /// > (see 14.8, "Tagged PDF"). Default value: false . If Suspects is true , the document may
    /// > not completely conform to tagged PDF conventions.
    ///
    /// A file whose catalog states a `/StructTreeRoot` has *demonstrated* the conventions the
    /// flag records. **Where there is no tree the entry is not written** — the flag would then be
    /// a claim about content nobody had produced, which is the one thing
    /// `doc/pdf-a-conversion-limits.md` section 5.1 refuses.
    MarkInfo,
    /// Every embedded file's file specification states both `/F` and `/UF`, each derived from
    /// the other.
    ///
    /// ISO 19005-2 section 6.8 and ISO 19005-4 section 6.9 require both keys, and §7.11.3's
    /// Table 43 makes them the same file name written twice in two types. `F` is
    ///
    /// > A file specification string of the form described in 7.11.2, "File specification
    /// > strings"
    ///
    /// and `UF` is
    ///
    /// > A Unicode text string that provides file specification of the form described in 7.11.2,
    /// > "File specification strings".
    ///
    /// **The derivation is only made where the value is ASCII**, and that limit is the whole of
    /// what keeps this from inventing something. `F` is a byte string whose encoding §7.11.2
    /// leaves to the file system and `UF` a §7.9.2.2 text string; over the ASCII range the two
    /// agree byte for byte, so writing one from the other states no encoding the producer did
    /// not. Outside it, the byte string's own character set is a fact this converter does not
    /// have, and the entry is left as it was.
    ///
    /// A specification stating **neither** key is left alone as well: there is nothing to derive
    /// from, and `doc/adr/0947`'s third stage is what then refuses the file.
    AssociatedFileNames,
    /// Every embedded file's file specification states an `/AFRelationship`.
    ///
    /// ISO 19005-4 section 6.9 requires the key; §7.11.3's Table 43 supplies the value, because it
    /// gives the entry a default and that default is the name written here:
    ///
    /// > Unspecified shall be used when the relationship is not known or cannot be described
    /// > using one of the other values.
    ///
    /// > Default: Unspecified
    ///
    /// So a specification with no `/AFRelationship` **already** relates to the document in
    /// exactly the way `/Unspecified` names, and writing the name asserts nothing a conforming
    /// reader did not already read there. What it does not do is guess: none of Table 43's other
    /// seven values can be established from a file specification, and this rewrite never writes
    /// one.
    AssociatedFileRelationship,
    /// Every font the Unicode requirements failed at states a `/ToUnicode` `CMap` derived from its
    /// own encoding.
    ///
    /// ISO 19005-2 section 6.2.11.7.2's two rules — a `CMap` on every non-exempt font, and usable
    /// values in the ones a file already has — answered by §9.10.2's second method over the codes
    /// the content streams actually showed. [`super::to_unicode`] is the derivation and the six
    /// reasons it declines; **a code whose meaning is not derivable refuses the document** rather
    /// than being invented, which is `doc/pdf-a-conversion-limits.md` section 4.3's line.
    ToUnicode,
    /// Every metadata stream whose packet breaks ISO 16684-1's grammar or data model carries a
    /// fresh one this conversion composes instead.
    ///
    /// ISO 19005-2 section 6.6.2.1 and ISO 19005-4 section 6.7.2.1. The three writers beside this
    /// one edit a producer's packet *by span* — a property cut out, a prefix moved, the
    /// identification schema restated — and a packet that will not parse has no spans to write
    /// into, which is why this is a replacement rather than an edit and why it is a loss rather
    /// than a mechanical rewrite.
    ///
    /// What goes in its place is [`pdf_model::xmp::empty_packet`], which states no property at
    /// all. The catalog's stream then gains the identification schema and this conversion's
    /// recorded events from [`Self::IdentificationSchema`]'s own writer, so the one place in this
    /// verb that states a conformance claim stays one place; every other metadata stream keeps
    /// the empty packet, because nothing in the file says what the unreadable one meant and
    /// composing properties for an object's own packet would be this converter writing the
    /// producer's metadata. What the producer wrote is kept where the configuration asked for it
    /// — [`super::preserve`]'s appended page — and lost where it did not.
    FreshMetadataPacket,
    /// Every XMP packet loses the properties whose own predefined schema does not define them.
    ///
    /// ISO 19005-2 section 6.6.2.3.1, and the one route of the three
    /// `doc/pdf-a-conversion-limits.md` section 3.9 leaves open. `pdf_model::xmp::remove` cuts the
    /// property out of the producer's own bytes by span, so **every other byte of the packet
    /// crosses unchanged** — what goes is the property and nothing beside it.
    PropertyOutsideItsSchema,
    /// Every extension schema container field spelled with another prefix gains the required one.
    ///
    /// ISO 19005-2 section 6.6.2.3.3's four tables each name the prefix their fields are to be
    /// spelled with, and section 6.6.2.2 is what makes that load-bearing: a prefix means nothing
    /// *except* where one is identified as required, and these four identify one. So a field
    /// stated in the field namespace its table gives it, with its local name and its value the
    /// producer's, and spelled with another prefix, is a file that says the right thing in the
    /// wrong letters — and this is the edit that changes the letters and nothing else.
    ///
    /// `pdf_model::xmp::respell` moves the prefix tokens in the producer's own bytes, the
    /// declaration that bound the old prefix along with the names that used it, so the packet
    /// gains no attribute and loses none and every other byte crosses unchanged. **A field the
    /// packet does not state at all is not this rewrite's**: nothing in the file says what its
    /// value would be, and `super::prepare` refuses the document rather than half-correcting it.
    ExtensionSchemaPrefixes,
    /// An extension schema container missing a field nothing in the file states is discarded,
    /// and every property in a namespace it described goes with it — section 3.9's loss, named
    /// per property (`super::extension_container`).
    ExtensionContainerDiscarded,
    /// The identification schema loses an amendment or corrigendum identifier of the wrong form.
    ///
    /// ISO 19005-2 section 6.6.4 makes `pdfaid:amd` and `pdfaid:corr` optional and fixes their
    /// value as the number and the year separated by a colon. Neither half can be recovered from
    /// a value that is not of that form — correcting it would be deciding what the producer
    /// meant, which is `doc/questions/A48`'s forbidden half — and the entry being optional is
    /// what leaves removal available at all.
    ///
    /// `pdf_model::xmp::remove` cuts the property out of the producer's own bytes by span, so
    /// every other byte of the packet crosses unchanged. **A separate rewrite from the
    /// identification schema beside it** although both write the same packet: a document whose
    /// only fault is the form of this one entry asks for no restatement of part, conformance or
    /// revision, and `doc/adr/0947`'s rule is that nothing is changed that no failed requirement
    /// asked for.
    AmendmentIdentifierRemoved,
    /// Every extension schema a packet uses and describes nowhere gains a container describing it.
    ///
    /// ISO 19005-2 section 6.6.2.3.2 requires every extension schema a metadata stream uses to be
    /// described in that stream or in the catalog's, with section 6.6.2.3.3's container schema.
    /// `doc/pdf-a-conversion-limits.md` section 4.2 makes describing them the default and calls
    /// it authoring in a small way; the alternative is deleting the producer's properties, which
    /// is worse.
    ///
    /// **What comes out of the file and what does not is the whole of the decision here**
    /// (`doc/adr/1245`). The namespace URI, the prefix and each property's local name are the
    /// packet's own, and the value type is the form its own serialisation shows. The schema's
    /// human-readable name, each property's category and its description are nowhere in any
    /// file, so each is one fixed sentence saying so — and the report names every schema
    /// described, because a container written by the converter is not a producer's statement.
    ExtensionSchemaDescribed,
    /// Every structure field a packet states and its own value type leaves out is described in
    /// that value type.
    ///
    /// ISO 19005-2 section 6.6.2.3.1 requires every property to use the extension schema that
    /// describes it, and a value type's fields are described by its `pdfaType:field` and by
    /// nothing else (section 6.6.2.3.3's Tables 5 and 6). **The same decision as the container's,
    /// one level down** (`doc/adr/2050`): the field's namespace, its name and the value type its
    /// serialisation shows are the packet's own, and the description Table 6 requires is one fixed
    /// sentence saying the converter wrote it. `pdf_model::xmp::describe_fields` inserts the
    /// Table 6 structure into the producer's own description of the type, so every other byte of
    /// the packet crosses unchanged; the report names every field described.
    ExtensionFieldDescribed,
    /// Every annotation but a `Popup` that states no `/F` is given one whose only set bit is
    /// `Print`.
    ///
    /// ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2 require the entry and require that
    /// bit. §12.5.3's Table 167 numbers the flags from the low-order bit, and `Print` is bit
    /// position 3, so the value is 4 — every other flag left clear, which is exactly what
    /// §12.5.2's Table 166 already put in force by giving `/F` a default of 0. **The one bit that
    /// changes is the one the requirement is about.**
    AnnotationFlags,
    /// Every annotation of a subtype the target's part does not admit is taken out of the
    /// `/Annots` array of the page it is on.
    ///
    /// ISO 19005-2 section 6.3.1 and ISO 19005-4 section 6.3.1 state a prohibition and no
    /// alternative, so the only rewrite that meets either is removal.
    /// `doc/pdf-a-conversion-limits.md` section 3.2 classes it *Ask*, which is why the decision
    /// is a `Decision::Authorised` carrying [`super::Loss::ForbiddenAnnotation`]; `doc/adr/1099`
    /// is the argument, and `doc/adr/0816` is why the alternative of re-badging the annotation as
    /// a subtype the part admits is not available.
    ///
    /// **The reference is what is removed, and nothing hunts for the object.** The walk copies
    /// what the converted document reaches, so an annotation no `/Annots` array names is an
    /// object the output does not hold — and its `/Sound`, `/Movie`, `/RichMediaContent` or
    /// `/3DD` go with it for the same reason, without this rewrite having to know their names.
    /// A `Popup` whose `/Parent` was removed goes too: §12.5.6.14 makes it the window belonging
    /// to some other annotation, and one whose parent is gone is a window onto nothing.
    ///
    /// **The normal appearance is the one part that can be kept**, and `remedy = "preserve"` is
    /// how: [`Self::PreservedAsPage`] then holds the same stream object on an appended page, so
    /// it stays reachable and the producer's marks stay in the archive.
    ForbiddenAnnotationRemoved,
    /// Every annotation whose stated `/F` fails ISO 19005 is taken out of the `/Annots` array of
    /// the page it is on.
    ///
    /// ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2 require the `Print` flag set and
    /// `Hidden`, `Invisible`, `NoView` and `ToggleNoView` clear, and offer nothing to write in
    /// the place of an `/F` that says otherwise. The other future is writing the flags the
    /// requirement asks for, and `doc/pdf-a-conversion-limits.md` section 3.7 makes removal the
    /// default of the two: an annotation somebody hid was hidden on purpose, and showing it puts
    /// a mark on a page its producer kept off — which is a larger act than taking the annotation
    /// out. The decision is a `Decision::Authorised` carrying [`super::Loss::HiddenAnnotation`];
    /// `doc/adr/1234` is the argument.
    ///
    /// **An annotation stating no `/F` at all is not this rewrite's**: §12.5.2's Table 166 gives
    /// the entry a default of 0, so its producer decided nothing, and [`Self::AnnotationFlags`]
    /// writes the one bit the requirement is about.
    ///
    /// The reference is what goes, on [`Self::ForbiddenAnnotationRemoved`]'s reading and for its
    /// reason: the walk copies what the converted document reaches, so the annotation's own
    /// objects leave the file without this rewrite naming any of them.
    HiddenAnnotationRemoved,
    /// Every appearance dictionary states its normal appearance and nothing else.
    ///
    /// ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 admit `/N` and no other key.
    /// §12.5.5's Table 170 makes `/R` and `/D` optional and gives each the same default — the
    /// value of the `/N` entry — so what a reader draws with the pointer over the annotation or
    /// the mouse button down becomes the normal appearance, which is what the standard already
    /// has it draw for a dictionary stating neither. The artwork itself is gone from the file,
    /// which is why the decision carries [`super::Loss::AppearanceStates`] rather than being
    /// mechanical.
    ///
    /// **A dictionary stating no `/N` is refused rather than emptied.** Table 170's default for
    /// the keys being removed *is* the `/N` entry, so where there is none the removal falls back
    /// to nothing and would leave an annotation with an appearance dictionary that describes no
    /// appearance; `super::sites` says so by name.
    ExtraAppearanceStatesRemoved,
    /// Every optional content configuration dictionary loses its `/AS` entry.
    ///
    /// ISO 19005-2 section 6.9 forbids it, and ISO 19005-4 section 6.10 does not — part 4 keeps
    /// the key and requires a conforming processor to ignore it, so this rewrite is a part 2
    /// target's alone. §8.11.4.3 makes `/AS` the array by which a processor sets group states
    /// from external factors, so the document is left in the state the configuration's own
    /// `/BaseState`, `/ON` and `/OFF` put it in and nothing switches afterwards: the decision
    /// carries [`super::Loss::AutomaticStates`].
    ///
    /// **The entry is removed where the walk can reach it**, which is a configuration that is an
    /// object of its own, the `/OCProperties` dictionary that holds one directly, and the catalog
    /// that holds that dictionary directly. A configuration written inside a `/Configs` array
    /// that is an object of its own is what `super::sites` refuses by name.
    AutomaticStatesRemoved,
    /// Every annotation requiring an appearance dictionary and stating none gains an `/AP` whose
    /// `/N` names a form `XObject` constructed from the annotation's own entries.
    ///
    /// ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 require the dictionary, and
    /// §12.5.2's Table 166 requires it of a writer in the base standard's own words:
    ///
    /// > A PDF writer shall include an appearance dictionary when writing or updating the PDF file
    /// > except for the two cases listed below.
    ///
    /// So the construction is the standard's rather than this program's invention — §12.5.5 and
    /// §12.7.4.3 name the operation, and each subtype's clause says what the marks are. The
    /// *detail* is this renderer's, which is why the decision is a `Decision::Stated` carrying a
    /// sentence saying so, and why `doc/questions/A21` makes reporting every one the condition.
    ///
    /// **Only `/N` is written**, which is what both parts' section 6.3.3 requires of an appearance
    /// dictionary, so nothing here can create the `/R` or `/D` entry the same subclause forbids.
    AppearanceDictionary,
    /// Every rendered font whose descriptor carries no program gains one of the faces this
    /// program ships, embedded under the `/FontFile` key ISO 32000-2 §9.9's Table 124 admits.
    ///
    /// ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 require the program of
    /// every font a content stream renders to be in the file, and
    /// `doc/pdf-a-conversion-limits.md` section 4.9 makes embedding a face the **default** rather
    /// than a refusal: a PDF whose font is not embedded has no appearance of its own, so writing
    /// one down removes an indeterminacy instead of creating one. `doc/questions/A47` settled
    /// that, and made "where no shipped face covers a document's characters, refuse rather than
    /// guess" its condition. [`super::fonts`] is where both halves are.
    ///
    /// **The descriptor's `/CharSet` and `/CIDSet` are removed with the same edit.** §9.8.1's
    /// Table 122 makes each a description of the *embedded* program, and this descriptor
    /// described one the file did not carry; ISO 19005-2 section 6.2.11.4.2 requires such a
    /// description to be complete, so leaving a producer's beside a face this converter chose
    /// would leave the file stating something false about its own bytes.
    SubstituteFontProgram,
    /// Every embedded font program whose stated advances disagree with its own font dictionary
    /// has the *program's* advances restated.
    ///
    /// ISO 19005-2 section 6.2.11.5 and ISO 19005-4 section 6.2.10.5 require the two statements
    /// to agree, and §9.2.4 says which of them a reader positions glyphs by:
    ///
    /// > Storing this information in the font dictionary, although redundant, enables a PDF
    /// > processor to determine glyph positioning without having to look inside the font program.
    ///
    /// So the program's number is the one that may move and the dictionary's is not — nothing in
    /// this verb writes a `/Widths`, a `/W` or a `/DW`. **No mark on any page moves**, and no
    /// outline changes: what is rewritten is `hmtx` for an sfnt and the leading width operand of
    /// a charstring for a CFF.
    RestateFontMetrics,
    /// Every embedded font program whose stated advance *heights* disagree with the `/DW2` and
    /// `/W2` of the `CIDFont` dictionary that shows it vertically has the program's restated.
    ///
    /// ISO 19005-4 section 6.2.10.5's third paragraph, and part 2 states no such rule — which is
    /// why this is a rewrite of its own rather than the one above: at a part 2 target no
    /// requirement asks for it, and `doc/adr/0947`'s first rule is that nothing else may happen.
    ///
    /// The direction is the same and the reason is stronger. §9.2.4's inference is what makes
    /// restating `/Widths` the forbidden half of the horizontal case; here §9.9.1 says it
    /// outright:
    ///
    /// > The "vhea" and "vmtx" tables that specify vertical metrics shall never be used by a PDF
    /// > processor. The only way to specify vertical metrics in PDF shall be by means of the DW2
    /// > and W2 entries in a CIDFont dictionary.
    ///
    /// So rewriting `vmtx` changes nothing any conforming processor does, by the clause's own
    /// words, and rewriting `/DW2` or `/W2` would move every glyph on a vertical line on the
    /// authority of a table no reader may consult. **No mark moves and no outline changes**:
    /// what is rewritten is the advance field of `vmtx`, in place.
    RestateVerticalFontMetrics,
    /// Every graphics state's `/RI` and image dictionary's `/Intent` that names an intent the
    /// base standard does not define is restated as `RelativeColorimetric`.
    ///
    /// ISO 19005-2 section 6.2.6 admits only the four §8.6.5.8 defines, and that subclause says
    /// what a reader does with any other name:
    ///
    /// > If a PDF processor does not recognise the specified name, it shall use the
    /// > RelativeColorimetric intent by default.
    ///
    /// So the entry is restated to the name every conforming reader was already using, which is
    /// `doc/adr/0948`'s `Stated` class rather than a choice this converter made. **The same name
    /// as the `ri` operator's operand is not touched**: that is inside a content stream.
    RenderingIntent,
    /// Every `/BM` that is an array naming no blend mode the base standard defines is restated
    /// as the name `Normal`.
    ///
    /// ISO 19005-2 section 6.2.10 and ISO 19005-4 section 6.2.9 require a defined mode, and
    /// §8.4.1's Table 57 says what a reader does with such an array:
    ///
    /// > In the latter case, the PDF reader shall use the first blend mode in the array that it
    /// > recognises (or Normal if it recognises none of them).
    ///
    /// An array with no recognised name in it is therefore already `Normal` to every conforming
    /// reader, and writing the name down changes no composite. **A bare name the standard does
    /// not define is left alone**, because no sentence of the standard says what it means.
    BlendModeNormal,
    /// Every annotation whose `/AP` `/N` is a subdictionary of states, and which states an `/AS`
    /// naming one of them, has `/N` restated as that one stream.
    ///
    /// ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 require the normal appearance of
    /// an annotation that is not a button widget to be a stream, and §12.5.2's Table 166 says
    /// which of a subdictionary's streams a reader draws — the `/AS` entry is
    ///
    /// > The annotation's appearance state , which selects the applicable appearance stream from
    /// > an appearance subdictionary
    ///
    /// So the collapse writes down the reader's own choice. **Where the annotation states no
    /// `/AS` nothing is written**: nothing in the file then says which state the document is in.
    NormalAppearanceFromState,
    /// Every optional content configuration whose `/Order` omits a group the file states has the
    /// missing groups appended to it, in the order `/OCGs` lists them.
    ///
    /// ISO 19005-2 section 6.9 and ISO 19005-4 section 6.10 require the array to reference every
    /// group. The groups are the file's and the order is the file's, so nothing is invented; and
    /// §8.11.4.3's Table 99 says what the entry decides, which is a panel rather than a page:
    ///
    /// > An array specifying the order for presentation of optional content groups in an
    /// > interactive PDF processor's user interface.
    ///
    /// > Any groups not listed in this array shall not be presented in any user interface that
    /// > uses the configuration.
    ///
    /// So a group a producer left out of the array was a group no reader offered, and appending
    /// it adds a line to that list. **No content changes**: §8.11.2.3 decides what is drawn from
    /// each group's own state, which this rewrite does not touch.
    OptionalContentOrder,
    /// Every page that names a resource and states no `/Resources` of its own is given the
    /// dictionary §7.7.3.4's inheritance already puts in force.
    ///
    /// ISO 19005-2 section 6.2.2 and ISO 19005-4 section 6.2.2 require a content stream's
    /// resources to be *explicitly associated* with it, which `TechNote 0010`'s A003 reads as
    /// the `/Resources` entry of the page itself. Copying the inherited value down resolves
    /// every name to the object it already resolved to. **Only a page**: a form `XObject` or a
    /// Type 3 glyph procedure with no resources draws with whatever invoked it, and one
    /// dictionary cannot answer for every invocation.
    PageResources,
    /// Every font descriptor whose `/CharSet` or `/CIDSet` does not describe the whole of its own
    /// embedded program loses that entry.
    ///
    /// **Both requirements this answers bind a part 2 target and nothing else**, so the base
    /// standard the argument has to hold under is ISO 32000-1:2008, and it does: that edition's
    /// Table 122 makes `/CharSet` optional and its Table 124 makes `/CIDSet` optional, and each
    /// says what the entry's *absence* indicates: for each of them, a subset is then indicated
    /// by the subset tag in `/FontName` and by nothing else. So a descriptor stating neither is
    /// a conforming descriptor of the edition PDF/A-2 adheres to, and what the removed entry said
    /// is still said by the subset tag and by the program itself.
    ///
    /// ISO 19005-2 section 6.2.11.4.2 requires each to be complete. §9.8.1's Table 122 makes each
    /// optional, **deprecates** both — which ISO 32000-1 does not, and which is why the
    /// deprecation is the reason removal stays right at a later target rather than the reason it
    /// is right at this one — and says what is indicated by their absence rather than by
    /// their contents. `/CharSet` is
    ///
    /// > ( Optional; meaningful only in Type 1 fonts; PDF 1.1; deprecated in PDF 2.0 ) A string
    /// > listing the character names defined in a font subset.
    ///
    /// > If this entry is absent, the only indication of a font subset shall be the subset tag
    /// > in the FontName entry
    ///
    /// and `/CIDSet` is "( Optional; deprecated in PDF 2.0 ) A stream identifying which CIDs are
    /// present in the `CIDFont` file." Both describe a program **this file carries**, so what an
    /// incomplete one states is recoverable from the program itself and removing it loses
    /// nothing a reader could use.
    ///
    /// **Recomputing the entry is the other lossless route, and it is not the one taken.** Both
    /// editions make the key optional and give its absence a meaning, ISO 32000-2 deprecates it,
    /// and ISO 19005-4 states no such requirement at all — so the entry a conforming file wants
    /// is no entry, and writing a fuller one would leave a key an archive's later readers are
    /// told to ignore, to say what the subset tag already says.
    DescriptorSetRemoved,
    /// Every embedded Type 2 `CIDFont` stating no `/CIDToGIDMap` is given the name `Identity`.
    ///
    /// ISO 19005-2 section 6.2.11.3.2 requires the entry. **Written only at a part 2 target**,
    /// and the reason is which base document the target names: ISO 19005-2 section 5.1 makes a
    /// PDF/A-2 file one that adheres to ISO 32000-1, whose Table 117 gives `Identity` as this
    /// entry's own default — so writing it restates what a reader of that edition already
    /// applies. ISO 32000-2's Table 121 states no default, so the same write at a part 4 target
    /// would assert a mapping its base document does not supply.
    CidToGidIdentity,
    /// Every XMP packet whose header states the deprecated `bytes` or `encoding` attribute loses
    /// it.
    ///
    /// ISO 19005-2 section 6.6.2.1 and ISO 19005-4 section 6.7.2.1 forbid both. Each describes
    /// the packet's own framing rather than anything inside it, so the attribute is cut out of
    /// the processing instruction and every other byte of the producer's packet crosses
    /// unchanged — `super::prepare::without_deprecated_header_attributes` is the cut.
    ///
    /// **The edit is the last of the three a packet can take.** A property removal and the
    /// identification schema both rewrite the RDF the header wraps, so the attributes come off
    /// the bytes those left rather than off the producer's original — which is why the
    /// preparation is threaded through them rather than applied beside them.
    PacketHeaderAttributes,
    /// The catalog's `/NeedsRendering` is removed.
    ///
    /// ISO 19005-2 section 6.4.2 and ISO 19005-4 section 6.4.2 forbid the key. §7.7.2's Table 29
    /// says what it is, and every clause of the sentence is why removing it loses nothing:
    ///
    /// > ( Optional; deprecated in PDF 2.0 ) A flag used to expedite the display of PDF documents
    /// > containing XFA forms. It specifies whether the document shall be regenerated when the
    /// > document is first opened. See Annex K, ' XFA forms ' . Default value: false .
    ///
    /// So the entry is deprecated, its subject is the XFA form, and the value an absent entry
    /// states is `false` — the file goes on saying what it said to every reader that had no XFA
    /// engine.
    ///
    /// **What makes it lossless rather than nearly so is the requirement beside it.** A document
    /// whose form dictionary still states an `/XFA` fails `forms/no-xfa-key`, which this
    /// converter refuses, so no file this rewrite reaches is one where a reader had a form to
    /// regenerate. The two are one clause and this half is the half whose answer the standard
    /// prints.
    NeedsRendering,
    /// The interactive form dictionary's `/NeedAppearances` is removed.
    ///
    /// ISO 19005-2 section 6.4.1 and ISO 19005-4 section 6.4.1 require the flag to be absent or
    /// false, and §12.7.3's Table 224 states the condition a writer may say so under:
    ///
    /// > A PDF writer shall include this key, with a value of true , if it has not provided
    /// > appearance streams for all visible widget annotations present in the document.
    ///
    /// So the removal is a *claim about the file*, and [`Self::WidgetAppearance`] beside it is
    /// what makes the claim true: every visible widget the file provides no normal appearance for
    /// is given one first, and a widget whose appearance could not be built leaves this rewrite
    /// refused rather than clearing a flag the file would then be lying with (`doc/adr/1257`).
    ///
    /// **A widget whose `/AP` already states an `/N` is left exactly as its producer wrote it.**
    /// §12.7.2 makes the consistency of a stated appearance with its field's value the producer's
    /// obligation, and the file kept it; rebuilding such a stream would replace a producer's marks
    /// with this program's for no requirement at all (`doc/adr/0816`'s fence, and
    /// `doc/adr/0947`'s rule that nothing is changed no failed requirement asked for).
    ///
    /// **The other answer is the operator's**: `construct = false` clears the flag and constructs
    /// nothing, so a field whose appearance the producer never wrote is blank in the archive. That
    /// is what [`super::Loss::FieldAppearances`] is authorised for; with `construct = true` there
    /// is nothing to authorise and the decision is a statement instead.
    NeedAppearancesCleared,
    /// Every visible widget the file provides no normal appearance for gains one.
    ///
    /// The construction is `pdf_model::appearance`'s, which is §12.7.4.3's — the clause that
    /// states it is the one Table 224's flag points at, and the marks come from the widget's own
    /// `/MK` characteristics and its field's value. A button field's widget is never given one:
    /// §12.7.5.2.3 makes its `/N` a subdictionary of one appearance per state, and one stream
    /// written there would replace a set of states with a picture.
    ///
    /// **Not [`Self::AppearanceDictionary`]**, although both write an `/AP` `/N`. That one answers
    /// ISO 19005's own section 6.3.3 over the annotations with no appearance dictionary at all;
    /// this answers §12.7.3's Table 224 over the widgets a *flag* asks a processor to draw, and the
    /// preparation passes the first population to the second so a widget in both is written once.
    WidgetAppearance,
    /// The interactive form dictionary's `/XFA` is removed.
    ///
    /// ISO 19005-2 section 6.4.2 and ISO 19005-4 section 6.4.2 forbid the key and offer nothing to
    /// put in its place. What makes the removal something other than deleting the form is §K.2's
    /// own consistency requirement, of a file that states the key at all:
    ///
    /// > The other entries in the interactive form dictionary shall be consistent with the
    /// > information in the XFA resource.
    ///
    /// > PDF interactive form field objects shall be present for each field specified in the XFA
    /// > resource. The XFA field values shall be consistent with the corresponding V entries of
    /// > the PDF field objects.
    ///
    /// So in a conforming file the `AcroForm` *is* the form, field for field and value for value,
    /// and what goes with the key is the calculations, validations and formatting the template
    /// carried — [`super::Loss::XfaForm`]. A document whose own `/NeedsRendering` says its pages
    /// are regenerated is the case where that reading does not hold, and `doc/adr/1257` refuses it
    /// unless the operator answers `dynamic` otherwise.
    XfaRemoved,
    /// Every file specification whose embedded file the target does not admit is taken out of the
    /// `/EmbeddedFiles` name tree and out of every `/AF` array that named it.
    ///
    /// ISO 19005-2 section 6.8 requires an embedded file to conform to a part of ISO 19005 and
    /// ISO 19005-4 section 6.9 requires the same of a plain PDF/A-4 file; neither offers anywhere
    /// to put one that does not, so the only rewrite that meets either removes it —
    /// [`super::Loss::EmbeddedFile`].
    ///
    /// **The references are what go, and nothing hunts for the object** (`doc/adr/1099`'s rule).
    /// The walk copies what the converted document reaches, so a specification nothing names is
    /// one the output does not hold, and the embedded file stream under it goes with it. Both
    /// entries are removed rather than one: §7.7.4's Table 31 requires every file specification a
    /// `/EmbeddedFiles` name tree references to state an `/EF` holding an embedded file stream, so
    /// a tree entry left behind would be a defect this conversion wrote.
    EmbeddedFileRemoved,
    /// Every symbolic TrueType font that states an `/Encoding` loses it, where no code it could
    /// show reaches a different glyph without it.
    ///
    /// ISO 19005-2 section 6.2.11.6 and ISO 19005-4 section 6.2.10.6 forbid the entry on a
    /// symbolic font. The entry is in the font dictionary rather than in the program, so ADR
    /// 0816's fence does not stand in the way — but §9.6.5.4 makes the encoding decide which
    /// `cmap` subtable a code is looked up through, so taking it away can change which glyph a
    /// code draws. **The proof is what makes this mechanical**: the font is loaded as the file
    /// states it and again with the entry gone, and every one of the 256 codes a simple font can
    /// show has to reach the same glyph in both. A single disagreement refuses the document
    /// rather than moving a mark.
    SymbolicTrueTypeEncodingRemoved,
    /// Every non-symbolic TrueType font whose encoding is neither `MacRomanEncoding` nor
    /// `WinAnsiEncoding` is given whichever of the two leaves every code on the glyph it already
    /// reached.
    ///
    /// ISO 19005-2 section 6.2.11.6 and ISO 19005-4 section 6.2.10.6 require one of the two
    /// names. [`Self::SymbolicTrueTypeEncodingRemoved`]'s proof, in the other direction: the
    /// candidate is written into a copy of the font dictionary, the font is loaded both ways, and
    /// the name is used only where all 256 codes reach the glyph they reached before.
    /// `WinAnsiEncoding` is tried first and `MacRomanEncoding` second, which is an order rather
    /// than a preference — a font that passes under either is unchanged by the choice, and one
    /// that passes under neither is refused.
    ///
    /// **A `/Differences` array is left exactly as its producer wrote it.** §9.6.5.1 makes the
    /// base encoding and the differences two entries of one dictionary, and this requirement is
    /// about the first; the rows about the second are their own.
    StandardTrueTypeEncoding,
    /// Every code a non-symbolic TrueType font shows that reached its glyph by a mapping of the
    /// reader's choosing is named in the encoding's `/Differences` array, by the Adobe Glyph List
    /// name of the character its `/ToUnicode` states.
    ///
    /// ISO 19005-2 section 6.2.11.6 and ISO 19005-4 section 6.2.10.6 require §9.6.5.4's own route.
    /// The entry is the font dictionary's, so ADR 0816's fence over the program does not stand in
    /// the way, and [`super::sites::standard_route_names`] proves the names first: every code
    /// shown has to reach the glyph it reached before, by the route (ADR 1942).
    StandardRouteNames,
    /// A Type 3 font's glyph named `.notdef` is given another name, in its encoding's
    /// `/Differences` and its `/CharProcs` alike.
    ///
    /// ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing
    /// operator to reference `.notdef`. A Type 3 font's glyphs are the producer's own procedures
    /// keyed by name (ISO 32000-2 §9.6.4), so the same procedure under another name draws the same
    /// marks; [`super::sites::type3_notdef_renames`] is the argument (ADR 1942).
    Type3NotdefRenamed,
    /// Every entry of the catalog's `OutputIntents` array names one destination profile object.
    ///
    /// ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3 require every entry that states a
    /// `DestOutputProfile` to state *the same indirect object*, which the clause's own note
    /// explains: a file conforming both to ISO 19005 and to PDF/X or PDF/E carries one intent per
    /// standard, and the colours can only be referred to one destination.
    ///
    /// **The proof is what makes this mechanical**: the profiles are decoded and the rewrite is
    /// performed only where every entry's profile is the same bytes under the same stream
    /// dictionary, so the object that goes carried a copy of the one that stays and no entry
    /// changes what it refers its colours to. Entries naming genuinely different profiles are
    /// refused, because one destination would then be discarded.
    SharedDestinationProfile,
    /// A `DeviceN` colour space's `/Colorants` gains the entry a spot colourant it uses is
    /// missing, written from the `Separation` array this file already states for that colourant.
    ///
    /// ISO 19005-2 section 6.2.4.4 and ISO 19005-4 section 6.2.4.4 require the entry, and the
    /// same subclause is what decides its value: every `Separation` array in one file naming a
    /// given colourant, the arrays written inside a `Colorants` dictionary expressly included, must
    /// state the same alternate space and the same tint transform, compared as PDF objects. So
    /// where the file states one for this colourant there is nothing to choose, and the producer's
    /// own definition of the ink is what goes in.
    ///
    /// **Only that shape.** Deriving the entry from the `DeviceN` space's own N-input tint
    /// transform is the general route and it is refused, because §7.10 gives a PDF function no
    /// way to call another: the one-input transform could only be a *sample* of the producer's,
    /// and an archive would carry an approximation written as though it were their definition.
    SpotColorantEntry,
    /// A `JPXDecode` image keeps only the colour space specification box the standards' own
    /// sentences make the used one.
    ///
    /// ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 require exactly one
    /// specification to be marked as the best available where the data states several, and
    /// require the method a colour box states to be one of three. Every *value* this converter
    /// could write into such a box is a choice — which is `doc/pdf-a-mitigations.md` section
    /// 4.5's finding and stands — and the route that chooses nothing is removal: the sentence
    /// after the method's, in both parts, says a conforming processor shall use only the selected
    /// colour space and shall ignore all the other specifications.
    ///
    /// [`super::jpeg2000`] is the whole reading: which box the two standards select, why a file
    /// neither of them settles stays refused, why the loss is the §7.4.9 fallback chain rather
    /// than any pixel, and what shapes of JP2 structure this declines to shift bytes inside.
    Jpeg2000ColourSpecifications,
    /// Every signature the source carries loses its value; its field, widget and appearance stay.
    ///
    /// `doc/pdf-a-conversion-limits.md` section 3.6, built as [`super::signatures`] reads it:
    /// each signature field's `/V` goes (§12.7.5.5), the permissions dictionary's `DocMDP` and
    /// `UR3` entries go with the signatures they rested on (Table 263, §12.8.2.3), and the form's
    /// `AppendOnly` flag is cleared because the output holds no signature it could describe
    /// (Table 225). The widget and its `/AP` stay, because ISO 19005-2 section 6.3.3 requires
    /// an appearance and §12.7.5.5 forbids the appearance to carry a validation status — so
    /// what stays asserts nothing the file no longer has.
    ///
    /// **Never mechanical.** The conversion invalidates every signature whatever requirement
    /// asked for the rewrite, so this is wanted whenever a non-conforming source carries one,
    /// and it is wanted under [`super::Loss::SignatureAssertion`] — a person authorises it, and
    /// the report names what went.
    SignatureValueRemoved,
    /// A permissions dictionary loses every key but `UR3` and `DocMDP`.
    ///
    /// ISO 19005-2 section 6.1.12 and ISO 19005-4 section 6.1.11. §12.8.6 makes each key the
    /// name of a permission handler and Table 263 names the two the standard defines; a key
    /// naming any other is one no conforming processor can consult, so nothing any reader did
    /// ever turned on it. [`super::signatures::foreign_handlers`] is the reading.
    ForeignPermissionHandlers,
    /// Every embedded file the target refuses is replaced by what a declared tool derived from it.
    ///
    /// `doc/rfc/0007` section 2's `derive` kind, allowed by `doc/questions/A55` on terms, and the
    /// catalogue's flagship entry (`doc/pdf-a-mitigations.md` section 11): ISO 19005-2 section 6.8
    /// and ISO 19005-4 section 6.9 require an embedded file to conform to a part of ISO 19005, and a
    /// spreadsheet does not. The bytes a program derived from it do.
    ///
    /// **This is the one rewrite of this verb that puts content in the file that was in no
    /// document.** It is therefore not [`Decision::Mechanical`] and not [`Decision::Stated`]: it is
    /// [`Decision::Configured`], carried out only where the operator's configuration named the site
    /// *and* the tool, reported per document in the words *this is derived, not original*, and
    /// recorded in the file's own `xmpMM:History`. [`super::remedies`] is the construction and the
    /// four dictionary edits it makes.
    ///
    /// [`Decision::Mechanical`]: super::Decision::Mechanical
    /// [`Decision::Stated`]: super::Decision::Stated
    /// [`Decision::Configured`]: super::Decision::Configured
    DerivedEmbeddedFile,
    /// Every associated file's stream states the media type the operator's configuration supplied.
    ///
    /// ISO 19005-4 section 6.9, by way of §14.13.2, requires an associated file's embedded stream to
    /// state a `/Subtype` that is a MIME media type, and §7.11.4.1's Table 44 makes the entry that
    /// media type. **Nothing in a file specification states one**: an extension is a convention
    /// rather than a declaration, so reading it as one would be this converter asserting what the
    /// bytes are, which is why the requirement is otherwise refused. An operator whose pipeline
    /// produces the attachments does know, and `doc/pdf-a-mitigations.md` section 0.2's `supply` is
    /// them saying so.
    ///
    /// **The value is written on the operator's authority and the file says so**: the report names
    /// it beside the requirement it answered and `xmpMM:History` records that a human rather than
    /// the document is its source.
    SuppliedMediaType,
    /// The page tree's root node gains the pages a `preserve` remedy composed, and the page-label
    /// tree a range for them.
    ///
    /// `doc/adr/1014`'s amendment to `CLAUDE.md`'s authoring exclusion: a page composed solely of
    /// content the document already holds is on the near side of the line, and the pages here
    /// carry nothing else. Two edits, and the second is `doc/adr/0954`'s cost rather than a
    /// conformance requirement — §12.4.2's labels stop describing a document whose page count
    /// changed, so the appended pages get a range of their own ([`super::preserve`] says what it
    /// states and why).
    PreservedAsPage,
    /// A forbidden annotation's normal appearance is put back where §12.5.5 had it, on the
    /// producer's own page rather than on a page this conversion appends.
    ///
    /// `doc/adr/1123`, the construction `doc/adr/1120`'s amendment put in scope. The page's
    /// `/Contents` gains a `q` before the producer's operators and a closing stream after them —
    /// §8.4.2's balance kept across the array — and its `/Resources` gains the appearance as a
    /// form `XObject`. Neither is a mark: what draws is §12.5.5's own placement of the producer's
    /// own stream. Where the construction is refused, [`Self::PreservedAsPage`] is the fallback.
    RelocatedOnPage,
    /// An action of a type the target's part does not admit is taken out of the tree it sits in.
    ///
    /// ISO 19005-2 section 6.5.1 and ISO 19005-4 section 6.6.1, whose prohibitions name action
    /// *types*, so what goes is the action and not the entry that reached it. §12.6.2's Table 196
    /// gives the removed action a `/Next`, and [`super::actions`] promotes it into the place the
    /// action held: the actions behind a forbidden one are failing nothing, and ADR 0947's second
    /// rule is that nothing changes which no failed requirement asked to change.
    ///
    /// **No mark moves.** Every content stream crosses this rewrite byte for byte; what changes is
    /// what a reader does when a user clicks, which is not something the page shows (ADR 1175).
    ForbiddenActionRemoved,
    /// An `/AA` entry the target's part does not admit, or the keys of one it does not admit.
    ///
    /// ISO 19005-2 section 6.5.2 forbids the entry on the catalog, on a page and on a widget
    /// annotation or field dictionary; ISO 19005-4 section 6.6.3 permits it on a widget and admits
    /// only §12.6.3's Table 197 annotation triggers elsewhere. So the two parts ask for two sizes
    /// of the same act — the whole dictionary, or the keys outside the permitted set — and an
    /// `/AA` left with nothing in it goes with its last entry.
    AdditionalActionsRemoved,
    /// The `/A` of a widget annotation or field dictionary.
    ///
    /// ISO 19005-2 section 6.4.1 and ISO 19005-4 section 6.4.1 both forbid the entry itself rather
    /// than an action type, so the whole chain behind it goes: there is no part of it the clause
    /// leaves a place for. A widget's `/AA` is a separate question, and part 4 answers it
    /// differently — [`Self::AdditionalActionsRemoved`] is where that lives.
    WidgetActionEntryRemoved,
    /// A content stream states the final hexadecimal digit §7.3.4.3 already assumed.
    ///
    /// ISO 19005-2 section 6.1.6 and ISO 19005-4 section 6.1.5 require an even number of digits
    /// and each attaches a NOTE saying what the rule is for: it removes the base standard's
    /// provision for a missing final one. That provision states the string's value outright —
    ///
    /// > If the final digit of a hexadecimal string is missing -that is, if there is an odd
    /// > number of digits -the final digit shall be assumed to be 0.
    ///
    /// — so the byte written here is the one every conforming reader already supplies, and the
    /// operand's value is the same before and after. `doc/adr/1176` is the argument and
    /// [`super::hexadecimal`] the reading; the sibling row about a byte that is not a digit stays
    /// refused, because §7.3.4.3 gives that byte no value to transcribe.
    HexadecimalDigitCompleted,
    /// Every `Separation` array naming one colourant states the one definition of it the
    /// operator's configuration chose.
    ///
    /// ISO 19005-2 section 6.2.4.4 and ISO 19005-4 section 6.2.4.4 require the agreement; nothing
    /// in the file says which of two definitions its producer meant, so the choice is a `supply`
    /// and never a default (`doc/rfc/0007` section 2, `doc/adr/1188`).
    ///
    /// **What is written is the document's own bytes.** The alternate space and tint transform
    /// this rewrite puts into the losing arrays are the ones a winning array already stated, so
    /// nothing is invented — what changes is that marks drawn through a losing definition are
    /// painted through the winning one. §8.6.6.4 makes that a real change on a screen, where a
    /// `Separation` "never applies a process colourant directly; it always reverts to the
    /// alternate colour space".
    SeparationAgreed,
    /// The document's encryption is not carried into the output.
    ///
    /// ISO 19005-2 section 6.1.3 and ISO 19005-4 section 6.1.3 forbid an `/Encrypt` key in the
    /// trailer, and sections 6.1.7.2 and 6.1.6.2 forbid a `Crypt` filter naming anything but
    /// `Identity`. Both are answered by the same act, because §7.6.2 makes encryption a property
    /// of the *file* rather than of anything in the object graph: the strings and stream data
    /// every object here holds were decrypted when the source was opened, and the output is
    /// serialized from those objects with no encryption dictionary and no document identifier
    /// derived from one.
    ///
    /// **Nothing in the document changes, and that is the point**: not a mark, not a string, not
    /// a byte of any stream's decoded data. What changes is the *file*, which stops needing a key
    /// to read — and with it Table 22's permission flags stop being asserted, which is the loss
    /// [`super::Loss::Encryption`] names and `doc/adr/1187` argues.
    EncryptionRemoved,
    /// A stream whose data the source kept outside the file carries it inside, and the keys that
    /// put it outside are gone.
    ///
    /// ISO 19005-2 section 6.1.7.1 and ISO 19005-4 section 6.1.6.1, and the construction is
    /// §7.3.8.2's Table 5 read straight: the external file's bytes go where the stream's own
    /// were — the table having said a reader ignores those while `/F` stands — `/Filter` and
    /// `/DecodeParms` become the `F`-prefixed pair that described the data now written,
    /// `/Length` is restated, and the forbidden keys are removed. Where a stream states one of
    /// the filter keys and no `/F`, nothing is fetched and nothing changes but the removal:
    /// Table 5 gives those keys meaning only through `/F`, so a conforming reader never consulted
    /// them (`doc/adr/1199`).
    ExternalDataEmbedded,
    /// Every page loses the out-of-range optional boundary entry ISO 19005-2 section 6.1.13's
    /// limit is failed at, so that §14.11.2.1's own default states the boundary instead.
    ///
    /// §7.7.3.3's Table 31 makes `/MediaBox` required and the crop, bleed, trim and art
    /// boxes optional, and §14.11.2.1 gives each optional one a default that is another box
    /// in the same file: the crop box's "default value is the page's media box", and the bleed,
    /// trim and art boxes' "default value is the page's crop box". So the removal is the page
    /// saying itself the way Table 31 admits rather than an edit to what it says, and no mark
    /// moves either way.
    ///
    /// Whether anything a reader *computes* moves is `doc/adr/1210`'s predicate, asked per
    /// entry, and §14.11.2.1 answers it for the commonest case outright:
    ///
    /// > If the bounds of the crop, trim, bleed or art box extends outside of the bounds of the
    /// > media box, a processor shall treat the box as its intersection with the media box.
    ///
    /// An over-sized box is therefore already its intersection with the media box to every
    /// conforming processor, and where that intersection is what the default would give the
    /// entry carried no information a reader used. Where the two differ the removal costs
    /// [`super::Loss::PageBoundary`] and the caller authorises it. A failing **media box** is
    /// neither: Table 31 requires it, so there is no default to fall back to and the
    /// requirement stays refused by name.
    PageBoundaryRemoved,
    /// The document information dictionary's entries are written into the XMP packet, and the
    /// dictionary goes.
    ///
    /// ISO 19005-4 section 6.1.3, whose two sentences leave `/Info` no entry but `/ModDate` and
    /// admit even that only beside a `/PieceInfo`. §14.3.3 is why moving the values rather than
    /// dropping them is the standard's own answer:
    ///
    /// > Earlier versions of the PDF file format used the document information dictionary to
    /// > represent document level metadata. In PDF 2.0 such use is deprecated except for two
    /// > entries, CreationDate and ModDate . For any other document level metadata, a metadata
    /// > stream (see 14.3.2 "Metadata streams") should be used instead.
    ///
    /// and Table 349's NOTEs name the counterpart of each key one by one — `dc:title` for
    /// `/Title`, `dc:creator` for `/Author`, `dc:description` for `/Subject`, `pdf:Keywords`,
    /// `xmp:CreatorTool`, `pdf:Producer`, `xmp:CreateDate`, `xmp:ModifyDate` and `pdf:Trapped`.
    /// So every property written is the one the standard pairs with the key, and every value is
    /// the producer's own; [`super::prepare::Information`] is the whole construction.
    ///
    /// **Nothing overwrites what the packet already says.** §14.3.4 permits the addition only
    /// into a silence — "a PDF processor may add the information to the other, as long as both
    /// are fully equivalent" — and says of the other case that a processor "should leave the
    /// inconsistent values unchanged". `pdf_model::xmp::supplement` is additive for exactly that
    /// reason, and a dictionary value the packet contradicts is dropped rather than written.
    ///
    /// **The dictionary is kept where the clause keeps it**: a catalog stating a `/PieceInfo`
    /// leaves `/Info` holding `/ModDate` and nothing else, because §14.5 makes that date what a
    /// page-piece dictionary's `LastModified` is compared against.
    InformationMovedIntoThePacket,
    /// The producer's own bytes stay in the archive as an embedded file, filed in §7.7.4's
    /// `/EmbeddedFiles` name tree and associated with the document by §14.13.3's catalog `/AF`.
    ///
    /// `doc/rfc/0007` section 4.6.1's other `preserve` mechanism, and the one the targets differ
    /// about: ISO 19005-2 section 6.8 and ISO 19005-4 section 6.9 require an embedded file to
    /// conform to a part of ISO 19005, and Annex A lifts that for PDF/A-4f and Annex B for
    /// PDF/A-4e — so bytes that are not a PDF stay inside the archive at those two targets and
    /// nowhere else. [`super::attach`] is the construction and `doc/adr/1270` the argument.
    ///
    /// **Not a mark and not a page.** What the document gains is three objects and two entries in
    /// its catalog; nothing on any page changes, and the bytes attached are the producer's,
    /// carried across without being decoded, re-encoded or re-parsed.
    PreservedAsAttachment,
    /// Every annotation whose stated `/F` ISO 19005 forbids is kept on its page and given the
    /// flags the requirement asks for.
    ///
    /// ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2 require the `Print` bit set and
    /// `Hidden`, `Invisible`, `NoView` and `ToggleNoView` clear. `doc/pdf-a-conversion-limits.md`
    /// section 3.7 gives the document two futures and [`Self::HiddenAnnotationRemoved`] is the
    /// default one; this is the other, reached only by a configuration answering the site with
    /// `preserve`. The value written is `pdf_archive::flags_permitting`'s, so the five bits of
    /// §12.5.3's Table 167 the requirement is not about stay as their producer wrote them.
    /// **What it costs is on the page**: a mark its producer kept off the screen or off paper is
    /// now on both, which is why the report names every annotation shown with the flags it had
    /// (`doc/adr/1285`).
    HiddenAnnotationShown,
    /// Every reference `XObject` keeps its form and loses the `Ref` entry that made it one.
    ///
    /// ISO 19005-2 section 6.2.9.2 and ISO 19005-4 section 6.2.8.2 forbid reference `XObject`s,
    /// and §8.10.4.1 makes the form carrying the entry a proxy that "should be processed by a PDF
    /// processor when the referenced content is not available" — so without the entry every
    /// reader draws what a reader of an archive, which holds no other file, would have drawn. What
    /// goes is the pointer, and the report names the file and page it named (`doc/adr/1285`).
    ReferenceXObjectProxied,
    /// Every JPEG 2000 image the baseline requirement named, where its dictionary states its own
    /// colour space, is decoded to its samples and written again under `FlateDecode`.
    ///
    /// ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 bind JPEG 2000 data, and
    /// §7.4.9 has a stated `/ColorSpace` decide what the samples mean — so the copy keeps that
    /// space, each component's depth and every integer, and what changes is the encoding. The
    /// costs are the file's size and the decoder's output kept as the picture, which the report
    /// names image by image (`doc/adr/1400`).
    Jpeg2000TranscodedToFlate,
    /// Every composite font naming a `CMap` the base standard does not predefine names a stream
    /// holding Adobe's published program for it instead.
    ///
    /// ISO 19005-2 section 6.2.11.3.3 and ISO 19005-4 section 6.2.10.3.3 require such a `CMap` to
    /// be embedded, and §9.7.5.3 is the stream: the program byte for byte, with Table 118's
    /// entries stating what it states. The mapping is the one the name already meant, so no code
    /// selects a different CID (`doc/adr/1286`).
    ShippedCMapEmbedded,
    /// Every embedded `CMap` whose dictionary and program disagreed about its writing mode states
    /// one mode in both, the one the operator said its producer meant.
    ///
    /// ISO 19005-2 section 6.2.11.3.3 and ISO 19005-4 section 6.2.10.3.3; Table 118 makes the
    /// dictionary's `/WMode` the program's own value. A `supply`: the
    /// operator's `write-mode` says which statement stands, and the other is made to agree with it
    /// — the dictionary's entry written, or the program's own entry's digit replaced
    /// (`doc/adr/1286`).
    WriteModeAgreed,
    /// Every stream whose `/Filter` names a filter by the abbreviation §8.9.7's Table 92 gives it
    /// states the filter's full name instead; the stream's bytes are not touched.
    ///
    /// ISO 19005-2 section 6.1.7.2 and ISO 19005-4 section 6.1.6.2 admit only the base standard's
    /// standard filters, and §8.9.7 confines the abbreviations to inline images:
    ///
    /// > These abbreviations are valid only in inline images; they shall not be used in image
    /// > XObjects.
    ///
    /// The standard says nothing of what a reader does with one in a stream, and this reader's
    /// decoder takes each as the filter Table 92 names — so the page a person saw was drawn
    /// through that filter, and writing the name out in full states the reading it was drawn by
    /// (`doc/adr/1952`). A name that is no abbreviation is refused by `super::sites`.
    FilterNamesSpelledOut,
    /// Every optional content configuration that states no `/Name` is given the one the
    /// operator's `name-template` makes, numbered past every name the file already uses.
    ///
    /// ISO 19005-2 section 6.9 and ISO 19005-4 section 6.10 require each configuration to carry
    /// a `/Name` unique within the file; §8.11.4.3's Table 99 makes it a label for a user
    /// interface. A `supply`, because no name exists in the file to be found: the text is the
    /// operator's, the report and `xmpMM:History` say so, and a configuration that states a name
    /// another one also states keeps its refusal (`doc/adr/1952`).
    ConfigurationNamed,
    /// Every graphics state stating `/TR /Identity` loses the entry, where no graphics state in
    /// the file states any other transfer function.
    ///
    /// ISO 19005-2 section 6.2.5 and ISO 19005-4 section 6.2.5 forbid the key. §10.5 is drawn
    /// here, so a real function changes what is shown and is not this rewrite's; but where every
    /// `/TR` the file states is the name `Identity`, the function in force is the identity before
    /// and after each `gs`, and removing the entry leaves every colour where it was on this
    /// reader's device, whose initial transfer function §8.4.1's Table 52 leaves device dependent
    /// and which is the identity here (`doc/adr/1952` section 3). A `/TR2 /Identity` is written as
    /// Table 57's `Default`, that same initial function, by the same reasoning (`doc/adr/1978`),
    /// and a halftone's `TransferFunction /Identity` where §10.6.5.6 requires none is removed,
    /// since the identity it overrode the graphics state's function with is what every graphics
    /// state then states (`doc/adr/2007`).
    IdentityTransferFunctionRemoved,
    /// Every graphics state stating `/TR` loses the entry, where one of them states a function
    /// other than the identity — each such function first written into `xmpMM:History`.
    ///
    /// The same removal as [`Self::IdentityTransferFunctionRemoved`] over the same graphics
    /// states and halftones, and a different rewrite because it costs [`super::Loss::TransferFunction`]: §10.5's
    /// function is drawn, so the colours it adjusted move (`doc/adr/1964`).
    TransferFunctionRemoved,
    /// A halftone stating `HalftoneName` loses the entry, keeping the parameters it defines.
    ///
    /// ISO 19005-2 section 6.2.5 and ISO 19005-4 section 6.2.5 forbid the key, and ISO 32000-2
    /// §10.6.5.1 makes it a pointer into a marking device's own halftones: nothing a screen shows
    /// moves, and a press holding a halftone of that name screens with the dictionary's own
    /// parameters instead, which is [`super::Loss::HalftoneName`] (`doc/adr/1965`).
    HalftoneNameRemoved,
    /// A graphics state whose `HT` names a halftone of a type other than 1 or 5 states Table 57's
    /// `Default`, and the halftone leaves the file.
    ///
    /// ISO 19005-2 section 6.2.5 and ISO 19005-4 section 6.2.5 admit types 1 and 5 alone; nothing a
    /// screen shows moves, and a press screens with its own halftone, which is
    /// [`super::Loss::Halftone`] (`doc/adr/2034`).
    HalftoneWrittenDefault,
    /// A graphics state's `HTP` is removed: an entry neither ISO 32000-1 nor ISO 32000-2 defines,
    /// which ISO 19005-2 section 6.2.5 forbids (`doc/adr/2035`).
    HalftonePhaseRemoved,
    /// A graphics state's `HTO` is removed, which ISO 19005-4 section 6.2.5 forbids; a press places
    /// its screen at its own origin, which is [`super::Loss::Halftone`] (`doc/adr/2034`).
    HalftoneOriginRemoved,
    /// An output intent's `DestOutputProfileRef` is removed where the same intent embeds its
    /// `DestOutputProfile`, which ISO 19005-4 section 6.2.3 requires of every intent and ISO
    /// 19005-2 section 6.2.3 of a PDF/X one (`doc/adr/2083`).
    DestinationProfileReferenceRemoved,
    /// Every graphics state that set overprint mode 1 for an `ICCBased` CMYK mark painted with
    /// overprinting on states mode 0, where none of them governs a `DeviceCMYK` mark.
    ///
    /// ISO 19005-2 section 6.2.4.2 and ISO 19005-4 section 6.2.4.2 forbid the mode there, and
    /// ISO 32000-2 §8.6.7 applies a non-zero mode only to `DeviceCMYK` marks, and §8.6.5.7 to an
    /// `ICCBased` one only where a processor treats the space as its output device's own — which
    /// this reader does only on a page compositing in a press, refused (`doc/adr/1978`).
    OverprintModeRestated,
    /// The same states state mode 0 where one of them also governs a `DeviceCMYK` mark painted
    /// with overprinting on, which costs [`super::Loss::OverprintMode`]: §8.6.7 has such a mark's
    /// zero components leave the backdrop under mode 1 and paint it out under mode 0
    /// (`doc/adr/1978`).
    OverprintModeZeroed,
    /// Every ICC profile this reader draws through that lacked only a description or a copyright
    /// tag carries the one the operator's text makes, every byte of its colour data kept.
    ///
    /// ISO 19005-2 section 6.2.4.2 holds the profile to the tags an ICC edition requires of its
    /// class, and ICC.1:1998-09 requires both of every class; no colour transform reads either.
    /// A `supply`, because what the text says is not in the file (`doc/adr/1990`).
    ProfileTextSupplied,
    /// Every `ICCBased` array naming a profile this reader does not draw through is written as
    /// the profile's `Alternate` space, or the device space of its `N`.
    ///
    /// ISO 19005-2 section 6.2.4.2 holds the profile to an ICC edition's tags, and ISO 32000-2
    /// §8.6.5.5's Table 65 makes the alternate the space used where the profile is not supported
    /// — which is what this reader drew, so writing it states the page's colours as they were
    /// shown (`doc/adr/1979`, `doc/adr/1990`).
    ProfileAlternateWritten,
    /// Every `ICCBased` array naming a CMYK profile identical to the PDF/A output intent's or
    /// the blending space's wherever it is used is written `DeviceCMYK`.
    ///
    /// ISO 19005-4 section 6.2.4.2 forbids the duplicate, and the device space takes its colour
    /// from that same profile — through the intent by that standard's section 6.2.4.3, through
    /// the group by ISO 32000-2 §11.7.2 — in a file that turns overprinting on nowhere
    /// (`doc/adr/2098`).
    DuplicateProfileWrittenDeviceCmyk,
    /// An `ICCBased` profile stream whose `N` is missing or disagrees with its profile states
    /// the profile's own count, which ISO 32000-2 §8.6.5.5's Table 66 requires it to match and
    /// which this reader draws with (`doc/adr/2099`).
    IccComponentCountRestated,
    /// Every non-standard structure type whose role map chain ends unmapped is mapped to the
    /// standard type the operator's `role-map` names, every entry the producer wrote kept.
    ///
    /// ISO 19005-2 section 6.7.3.4; ISO 32000-2 §14.7.3 makes `/RoleMap` the mapping. A `supply`:
    /// which standard type a producer's own type means is not in the file (`doc/adr/1990`).
    RoleMapSupplied,
    /// The catalog's `/Lang`, where it is not a language identifier, states the empty identifier
    /// for an unknown language, or the operator's tag.
    ///
    /// ISO 19005-2 section 6.7.4; ISO 32000-2 §14.9.2.2 admits those two values. A loss where the
    /// value is the empty one and a `supply` where it is the operator's (`doc/adr/2066`).
    CatalogLanguageRestated,
    /// Every structure element's and marked-content property list's `/Lang` that is not a
    /// language identifier states the same, a property list written into a content stream
    /// respelled in place with every other byte of the stream as it was (`doc/adr/2067`).
    MarkedLanguagesRestated,
}

impl Rewrite {
    /// What the rewrite does, in one sentence for a person.
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per rewrite, and the arms are prose for a person rather than code. \
                  Splitting the match would need either a catch-all, which stops a new rewrite \
                  from failing to compile until somebody has written its sentence, or an \
                  unreachable arm"
    )]
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
                "a PostScript XObject is written as a form XObject with no content, which draws \
                 what it drew - nothing - and keeps every name for it defined"
            }
            Self::RequirementsDictionary => "the catalog's /Requirements is removed",
            Self::AlternatePresentations => {
                "the name dictionary's /AlternatePresentations is removed"
            }
            Self::PresentationSteps => "a page's /PresSteps is removed",
            Self::DestinationProfileReplaced => {
                "the destination profile every output intent names, which this reader could not \
                 draw through, is written as the profile a file stating no intent is given"
            }
            Self::DestinationProfileRestated => {
                "the destination profile every output intent names has its header restated as \
                 what its own bytes are - the ICC edition its tags are written in, or the digest \
                 of the profile as its Profile ID - every tag and every colour kept"
            }
            Self::DestinationProfileSupplied => {
                "a PDF/A output intent identified as Custom and stating no destination profile \
                 names the profile this conversion's own output intent would name, its Info kept"
            }
            Self::OutputIntent => {
                "the catalog states a PDF/A output intent whose destination profile says what \
                 this file's device colours mean"
            }
            Self::IdentificationSchema => {
                "the document's XMP packet states this part's identification schema, and is \
                 created where the document had none"
            }
            Self::DefaultCmyk => {
                "every resource dictionary explicitly associated with a content stream states a \
                 DefaultCMYK: a DeviceN over Cyan, Magenta, Yellow and Black whose tint \
                 transform is ISO 32000-2 \u{a7}10.4.2.5's, over this file's ICC sRGB alternate"
            }
            Self::DefaultRgb => {
                "every resource dictionary explicitly associated with a content stream states a \
                 DefaultRGB: an ICCBased space over the ICC sRGB profile, beside a destination \
                 profile of another colour family"
            }
            Self::MarkInfo => {
                "the catalog states MarkInfo with Marked true, which the structure tree this \
                 file already carries had demonstrated"
            }
            Self::AssociatedFileNames => {
                "an embedded file's specification states both F and UF, each written from the \
                 other's value"
            }
            Self::AssociatedFileRelationship => {
                "an embedded file's specification states AFRelationship as Unspecified, which \
                 ISO 32000-2 Table 43 gives as the entry's own default"
            }
            Self::ToUnicode => {
                "a font states a ToUnicode CMap derived from its own encoding, by ISO 32000-2 \
                 \u{a7}9.10.2's glyph-name route, over the codes its content streams showed"
            }
            Self::PropertyOutsideItsSchema => {
                "a metadata property whose predefined schema does not define the value it holds \
                 is cut out of the packet, leaving every other byte of it as its producer wrote it"
            }
            Self::FreshMetadataPacket => {
                "a metadata stream whose packet breaks the XMP grammar or data model carries a \
                 packet this conversion composed instead, stating no property of its own — the \
                 catalog's gaining the identification schema and what was recorded beside it, \
                 because nothing in the file says what the unreadable packet meant"
            }
            Self::ExtensionSchemaPrefixes => {
                "an extension schema container field stated with another prefix is respelled \
                 with the one its table requires, the declaration that bound the old prefix \
                 along with it, leaving every other byte of the packet as its producer wrote it"
            }
            Self::ExtensionContainerDiscarded => {
                "an extension schema container leaving out a field the file states nowhere is \
                 discarded, and every property in a namespace it described goes with it, each \
                 named in the report and in the file's xmpMM:History"
            }
            Self::AmendmentIdentifierRemoved => {
                "an amendment or corrigendum identifier whose value is not the number and the \
                 year separated by a colon is cut out of the identification schema, leaving \
                 every other byte of the packet as its producer wrote it"
            }
            Self::ExtensionSchemaDescribed => {
                "an extension schema the packet uses and describes nowhere gains a container \
                 describing it: the namespace, the prefix, each property's name and the value \
                 type its own serialisation shows, with one fixed sentence in each field the \
                 file does not state"
            }
            Self::ExtensionFieldDescribed => {
                "a structure field the packet states and its own value type leaves out is \
                 described in that value type: the field's name and the value type its own \
                 serialisation shows, with one fixed sentence for the description the file does \
                 not state"
            }
            Self::AnnotationFlags => {
                "an annotation stating no /F is given one whose only set bit is Print, which is \
                 the value the requirement asks for and the default in every other bit"
            }
            Self::ForbiddenAnnotationRemoved => {
                "an annotation of a subtype ISO 19005 does not admit is taken out of its page's \
                 Annots array, and everything only that annotation reached — its media stream, \
                 its 3D artwork, its popup — leaves the file with it; a FileAttachment \
                 annotation's file is filed in the EmbeddedFiles tree instead"
            }
            Self::HiddenAnnotationRemoved => {
                "an annotation whose F entry ISO 19005 forbids — Print clear, or Hidden, \
                 Invisible, NoView or ToggleNoView set — is taken out of its page's Annots \
                 array, rather than being shown on a page its producer kept it off"
            }
            Self::ExtraAppearanceStatesRemoved => {
                "an appearance dictionary loses every key but N, so the rollover and down \
                 appearances go and ISO 32000-2 \u{a7}12.5.5's Table 170 leaves a reader drawing \
                 the normal one in their place"
            }
            Self::AutomaticStatesRemoved => {
                "an optional content configuration loses the AS array ISO 19005-2 section 6.9 \
                 forbids, so the document stays in the state that configuration's own entries \
                 set and no external factor switches a layer"
            }
            Self::AppearanceDictionary => {
                "an annotation stating no appearance dictionary is given an /AP whose /N names a \
                 form XObject constructed from the entries its own subtype clause states"
            }
            Self::SubstituteFontProgram => {
                "a font the file renders and does not embed gains one of the faces this program \
                 ships, embedded under the /FontFile key ISO 32000-2 \u{a7}9.9's Table 124 \
                 admits for its dictionary; the descriptor's CharSet and CIDSet go with it, \
                 because each describes a font program this file never carried"
            }
            Self::RestateFontMetrics => {
                "an embedded font program's stated advances are restated to the widths the font \
                 dictionary already states, which is the one of the two that does not move a mark"
            }
            Self::RestateVerticalFontMetrics => {
                "an embedded font program's vmtx is restated to the advance heights the CIDFont \
                 dictionary's DW2 and W2 already state — which ISO 32000-2 \u{a7}9.9.1 makes the \
                 only vertical metrics a PDF processor may use, the vmtx being a table it says \
                 shall never be used, so no conforming reader can observe the change"
            }
            Self::RenderingIntent => {
                "a graphics state's RI or an image's Intent that names no intent ISO 32000-2 \
                 defines is restated as RelativeColorimetric, which \u{a7}8.6.5.8 is what a \
                 processor uses for such a name anyway"
            }
            Self::BlendModeNormal => {
                "a BM that is an array naming no blend mode ISO 32000-2 defines is restated as \
                 Normal, which \u{a7}8.4.1's Table 57 is what a reader takes from such an array"
            }
            Self::NormalAppearanceFromState => {
                "an annotation whose normal appearance is a subdictionary of states is given the \
                 one stream its own AS entry selects"
            }
            Self::OptionalContentOrder => {
                "an optional content configuration's Order array gains the groups it left out, \
                 in the order the file's OCGs array lists them"
            }
            Self::PageResources => {
                "a page that names a resource and states no Resources of its own is given the \
                 dictionary ISO 32000-2 \u{a7}7.7.3.4's inheritance already puts in force for it"
            }
            Self::DescriptorSetRemoved => {
                "a font descriptor's CharSet or CIDSet, which describes the embedded program \
                 incompletely, is removed — both editions of the base standard make the entry \
                 optional and give its absence the same meaning, the subset tag in FontName, and \
                 ISO 32000-2 deprecates it besides"
            }
            Self::CidToGidIdentity => {
                "an embedded Type 2 CIDFont stating no CIDToGIDMap is given the name Identity, \
                 which ISO 32000-1 gives as that entry's default and which a PDF/A-2 file's own \
                 base document therefore already applies"
            }
            Self::PacketHeaderAttributes => {
                "an XMP packet header loses the deprecated bytes or encoding attribute, leaving \
                 every other byte of the packet as its producer wrote it"
            }
            Self::NeedAppearancesCleared => {
                "the interactive form dictionary's NeedAppearances goes, so no reader is asked to \
                 rebuild the form's appearances; every visible widget the file provided none for \
                 has one written first, which is what makes the file's new claim true"
            }
            Self::WidgetAppearance => {
                "a visible widget the file provides no normal appearance for is given an /AP \
                 whose /N names a form XObject built from its own characteristics and its \
                 field's value, by the construction ISO 32000-2 \u{a7}12.7.4.3 states"
            }
            Self::EmbeddedFileRemoved => {
                "a file specification whose embedded file this target does not admit is taken \
                 out of the EmbeddedFiles name tree and out of every AF array naming it, so \
                 nothing in the output reaches the file and the bytes are not carried"
            }
            Self::XfaRemoved => {
                "the interactive form dictionary's XFA goes, and with it the calculations, \
                 validations and formatting the template carried; the fields and their values \
                 stay, because ISO 32000-2 Annex K requires the AcroForm to agree with the \
                 resource that is being removed"
            }
            Self::NeedsRendering => {
                "the catalog's NeedsRendering goes, which ISO 32000-2 deprecates and whose \
                 absence states the false its own table gives as the default"
            }
            Self::SymbolicTrueTypeEncodingRemoved => {
                "a symbolic TrueType font's Encoding entry goes, where every one of the 256 codes \
                 it could show reaches the same glyph without it"
            }
            Self::StandardTrueTypeEncoding => {
                "a non-symbolic TrueType font is given WinAnsiEncoding or MacRomanEncoding, \
                 whichever leaves every one of the 256 codes it could show on the glyph it \
                 already reached"
            }
            Self::StandardRouteNames => {
                "a TrueType font's encoding names, in its Differences array, the glyph each code \
                 it shows was drawn with where only a mapping of the reader's choosing reached \
                 it, by the Adobe Glyph List name of the character its ToUnicode states, so the \
                 route ISO 32000-2 \u{a7}9.6.5.4 sets out reaches the same glyph"
            }
            Self::Type3NotdefRenamed => {
                "a Type 3 font's glyph procedure named .notdef is given another name in its \
                 encoding and its CharProcs alike, so the codes that drew it draw the same \
                 procedure and no text-showing operator references .notdef"
            }
            Self::SharedDestinationProfile => {
                "an output intent naming a second copy of the destination profile another entry \
                 of the same OutputIntents array names is pointed at that one object, which is \
                 the same profile byte for byte"
            }
            Self::SpotColorantEntry => {
                "a DeviceN colour space's Colorants dictionary gains the entry a spot colourant \
                 it uses was missing, which is the Separation array this file already states for \
                 that colourant and which ISO 19005 section 6.2.4.4 requires it to agree with"
            }
            Self::Jpeg2000ColourSpecifications => {
                "a JPXDecode image keeps only the colour space specification ISO 19005 makes the \
                 used one, the boxes it directs a processor to ignore going with the rest; not a \
                 sample of the image is touched"
            }
            Self::SignatureValueRemoved => {
                "each signature field keeps its widget and appearance and loses its value — the \
                 signature dictionary, and with it the digest and the certificate — the \
                 permissions dictionary loses the DocMDP and UR3 entries that rested on those \
                 signatures, and the form's AppendOnly flag is cleared"
            }
            Self::ForeignPermissionHandlers => {
                "the permissions dictionary loses every key but UR3 and DocMDP, each naming a \
                 permission handler ISO 32000 does not define and no conforming processor can \
                 consult"
            }
            Self::DerivedEmbeddedFile => {
                "an embedded file the target does not admit is replaced by what a program this \
                 configuration declares derived from it, so the attachment in the archive is a \
                 representation of the original rather than the original"
            }
            Self::SuppliedMediaType => {
                "an associated file's stream states the MIME media type the operator's \
                 configuration supplied, on the operator's authority rather than the document's"
            }
            Self::PreservedAsPage => {
                "content this target will not hold where it was is kept on a page appended to the \
                 document, carrying nothing that did not come from the file, and the page-label \
                 tree gains a range for it"
            }
            Self::RelocatedOnPage => {
                "a forbidden annotation's normal appearance is put back where ISO 32000-2 §12.5.5 \
                 had it, in the content of the producer's own page — a q before the producer's \
                 operators and a closing stream after them, and the appearance named in the \
                 page's resources — so no page is appended and no mark is composed"
            }
            Self::ForbiddenActionRemoved => {
                "an action of a type this part does not admit is taken out of the tree it sits \
                 in, and the actions ISO 32000-2 \u{a7}12.6.2's Next entry performed after it \
                 take its place"
            }
            Self::AdditionalActionsRemoved => {
                "an additional-actions dictionary this part does not admit is removed, or the \
                 trigger keys of one it does not admit are"
            }
            Self::WidgetActionEntryRemoved => {
                "a widget annotation's or field dictionary's /A entry is removed, with the \
                 action chain behind it"
            }
            Self::SeparationAgreed => {
                "every Separation array naming one colourant states the definition of it the \
                 configuration chose, which is one the file already stated"
            }
            Self::ExternalDataEmbedded => {
                "a stream whose data this file kept outside itself carries that data inside it, \
                 filtered as the F-prefixed entries said it was, and the keys that pointed off \
                 the file's edge are gone - or, where those keys described an external file the \
                 stream never named, the keys alone are gone and every byte stands"
            }
            Self::EncryptionRemoved => {
                "the document's encryption is not carried into the output, which no longer needs \
                 a password to read"
            }
            Self::HexadecimalDigitCompleted => {
                "a content stream's hexadecimal string states the final digit ISO 32000-2 \
                 \u{a7}7.3.4.3 already assumed, so the string reads the same and the file says so"
            }
            Self::PageBoundaryRemoved => {
                "a page loses the optional crop, bleed, trim or art box entry whose size ISO \
                 19005-2 section 6.1.13 does not admit, so that ISO 32000-2 \u{a7}14.11.2.1's \
                 own default states that boundary instead"
            }
            Self::InformationMovedIntoThePacket => {
                "the document information dictionary's entries are written into the XMP packet, \
                 each under the property Table 349's NOTE names for it, and the dictionary goes"
            }
            Self::PreservedAsAttachment => {
                "the producer's own bytes stay in the archive as an embedded file, filed in the \
                 EmbeddedFiles name tree and associated with the document by the catalog's AF"
            }
            Self::HiddenAnnotationShown => {
                "an annotation whose F entry ISO 19005 forbids stays on its page with the Print \
                 bit set and Hidden, Invisible, NoView and ToggleNoView clear, every other bit \
                 as its producer wrote it, so a mark kept off the page or off paper is on both"
            }
            Self::WriteModeAgreed => {
                "an embedded CMap whose dictionary and program disagreed about its writing mode \
                 states in both the one the operator said its producer meant"
            }
            Self::FilterNamesSpelledOut => {
                "a stream naming a filter by the abbreviation ISO 32000-2 \u{a7}8.9.7's Table 92 \
                 gives it for inline images states the filter's full name, its bytes untouched"
            }
            Self::ConfigurationNamed => {
                "an optional content configuration stating no Name is given the one the \
                 operator's name-template makes, numbered past every name the file already uses"
            }
            Self::IdentityTransferFunctionRemoved => {
                "a graphics state's TR entry naming the identity is removed and a TR2 naming it \
                 is written Default, and a halftone's TransferFunction naming it is removed where \
                 none is required, in a file that states no other transfer function"
            }
            Self::TransferFunctionRemoved => {
                "every graphics state's TR entry is removed, every TR2 is written Default and \
                 every halftone's TransferFunction is removed where none is required, and each \
                 function other than the identity is first written into the file's own \
                 xmpMM:History"
            }
            Self::HalftoneNameRemoved => {
                "a halftone's HalftoneName is removed and every parameter it defines is kept"
            }
            Self::HalftoneWrittenDefault => {
                "a graphics state naming a halftone of a type other than 1 or 5 sets Default, the \
                 halftone in effect at the start of the page, and the halftone leaves the file"
            }
            Self::HalftonePhaseRemoved => {
                "a graphics state's HTP, which the base standard does not define, is removed"
            }
            Self::HalftoneOriginRemoved => "a graphics state's halftone origin HTO is removed",
            Self::DestinationProfileReferenceRemoved => {
                "an output intent that embeds its destination profile no longer states a \
                 DestOutputProfileRef naming profiles outside the file"
            }
            Self::OverprintModeRestated => {
                "a graphics state that set overprint mode 1 for ICCBased CMYK marks states mode \
                 0, in a file where no DeviceCMYK mark is painted under it with overprinting on"
            }
            Self::OverprintModeZeroed => {
                "a graphics state that set overprint mode 1 for ICCBased CMYK marks states mode \
                 0, and the DeviceCMYK marks it governed paint their zero components"
            }
            Self::ProfileTextSupplied => {
                "an ICC profile that lacked only its description or copyright tag carries the \
                 text the operator supplied for it, its colour data byte for byte as it was"
            }
            Self::RoleMapSupplied => {
                "a non-standard structure type that mapped to no standard one is mapped in the \
                 role map to the standard type the operator named for it"
            }
            Self::CatalogLanguageRestated => {
                "the catalog's Lang, which was not a language identifier, states the empty \
                 identifier ISO 32000-2 \u{a7}14.9.2.2 gives an unknown language, or the tag the \
                 operator supplied"
            }
            Self::MarkedLanguagesRestated => {
                "a structure element's or a marked-content property list's Lang that was not a \
                 language identifier states the empty identifier for an unknown language, or the \
                 tag the operator supplied; a property list inside a content stream is respelled \
                 in place and no other byte of the stream changes"
            }
            Self::ProfileAlternateWritten => {
                "an ICCBased colour space whose profile this reader cannot draw through is \
                 written as the Alternate space ISO 32000-2 \u{a7}8.6.5.5 draws it in, or the \
                 device space of its N"
            }
            Self::IccComponentCountRestated => {
                "an ICCBased profile stream whose N is missing or disagrees with its profile \
                 states the number of components the profile has"
            }
            Self::DuplicateProfileWrittenDeviceCmyk => {
                "an ICCBased colour space whose CMYK profile is the output intent's or the \
                 blending space's wherever it is used is written DeviceCMYK, which takes its \
                 colour from that same profile"
            }
            Self::ShippedCMapEmbedded => {
                "a composite font naming a CMap the base standard does not predefine names a \
                 stream holding Adobe's published program for it, byte for byte, so the same \
                 codes select the same CIDs from a mapping the file now carries"
            }
            Self::ReferenceXObjectProxied => {
                "a reference XObject loses its Ref entry and keeps its form, so every reader \
                 draws the proxy ISO 32000-2 \u{a7}8.10.4.1 has a processor draw when the \
                 imported page is not available"
            }
            Self::Jpeg2000TranscodedToFlate => {
                "a JPEG 2000 image outside the JPX baseline that states its own ColorSpace is \
                 decoded to its samples and written again under FlateDecode in that space, each \
                 integer kept at its own depth"
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
            Self::DestinationProfileReplaced => "destination-profile-replaced",
            Self::DestinationProfileRestated => "destination-profile-restated",
            Self::DestinationProfileSupplied => "destination-profile-supplied",
            Self::OutputIntent => "output-intent",
            Self::IdentificationSchema => "identification-schema",
            Self::DefaultCmyk => "default-cmyk",
            Self::DefaultRgb => "default-rgb",
            Self::MarkInfo => "mark-info",
            Self::AssociatedFileNames => "associated-file-names",
            Self::AssociatedFileRelationship => "associated-file-relationship",
            Self::ToUnicode => "to-unicode",
            Self::PropertyOutsideItsSchema => "property-outside-its-schema",
            Self::FreshMetadataPacket => "fresh-metadata-packet",
            Self::ExtensionSchemaPrefixes => "extension-schema-prefixes",
            Self::ExtensionContainerDiscarded => "extension-container-discarded",
            Self::AmendmentIdentifierRemoved => "amendment-identifier-removed",
            Self::ExtensionSchemaDescribed => "extension-schema-described",
            Self::ExtensionFieldDescribed => "extension-field-described",
            Self::AnnotationFlags => "annotation-flags",
            Self::ForbiddenAnnotationRemoved => "forbidden-annotation-removed",
            Self::HiddenAnnotationRemoved => "hidden-annotation-removed",
            Self::ExtraAppearanceStatesRemoved => "extra-appearance-states-removed",
            Self::AutomaticStatesRemoved => "automatic-states-removed",
            Self::AppearanceDictionary => "appearance-dictionary",
            Self::SubstituteFontProgram => "substitute-font-program",
            Self::RestateFontMetrics => "restate-font-metrics",
            Self::RestateVerticalFontMetrics => "restate-vertical-font-metrics",
            Self::RenderingIntent => "rendering-intent",
            Self::BlendModeNormal => "blend-mode-normal",
            Self::NormalAppearanceFromState => "normal-appearance-from-state",
            Self::OptionalContentOrder => "optional-content-order",
            Self::PageResources => "page-resources",
            Self::DescriptorSetRemoved => "descriptor-set-removed",
            Self::CidToGidIdentity => "cid-to-gid-identity",
            Self::PacketHeaderAttributes => "packet-header-attributes",
            Self::NeedsRendering => "needs-rendering",
            Self::NeedAppearancesCleared => "need-appearances-cleared",
            Self::WidgetAppearance => "widget-appearance",
            Self::XfaRemoved => "xfa-removed",
            Self::EmbeddedFileRemoved => "embedded-file-removed",
            Self::SymbolicTrueTypeEncodingRemoved => "symbolic-truetype-encoding-removed",
            Self::StandardTrueTypeEncoding => "standard-truetype-encoding",
            Self::StandardRouteNames => "standard-route-names",
            Self::Type3NotdefRenamed => "type3-notdef-renamed",
            Self::SharedDestinationProfile => "shared-destination-profile",
            Self::SpotColorantEntry => "spot-colorant-entry",
            Self::Jpeg2000ColourSpecifications => "jpeg2000-colour-specifications",
            Self::SignatureValueRemoved => "signature-value-removed",
            Self::ForeignPermissionHandlers => "foreign-permission-handlers",
            Self::DerivedEmbeddedFile => "derived-embedded-file",
            Self::SuppliedMediaType => "supplied-media-type",
            Self::PreservedAsPage => "preserved-as-page",
            Self::RelocatedOnPage => "relocated-on-page",
            Self::ForbiddenActionRemoved => "forbidden-action-removed",
            Self::AdditionalActionsRemoved => "additional-actions-removed",
            Self::WidgetActionEntryRemoved => "widget-action-entry-removed",
            Self::HexadecimalDigitCompleted => "hexadecimal-digit-completed",
            Self::SeparationAgreed => "separation-agreed",
            Self::EncryptionRemoved => "encryption-removed",
            Self::ExternalDataEmbedded => "external-data-embedded",
            Self::PageBoundaryRemoved => "page-boundary-removed",
            Self::InformationMovedIntoThePacket => "information-moved-into-the-packet",
            Self::PreservedAsAttachment => "preserved-as-attachment",
            Self::HiddenAnnotationShown => "hidden-annotation-shown",
            Self::ReferenceXObjectProxied => "reference-xobject-proxied",
            Self::Jpeg2000TranscodedToFlate => "jpeg2000-transcoded-to-flate",
            Self::ShippedCMapEmbedded => "shipped-cmap-embedded",
            Self::WriteModeAgreed => "write-mode-agreed",
            Self::FilterNamesSpelledOut => "filter-names-spelled-out",
            Self::ConfigurationNamed => "configuration-named",
            Self::IdentityTransferFunctionRemoved => "identity-transfer-function-removed",
            Self::TransferFunctionRemoved => "transfer-function-removed",
            Self::HalftoneNameRemoved => "halftone-name-removed",
            Self::HalftoneWrittenDefault => "halftone-written-default",
            Self::HalftonePhaseRemoved => "halftone-phase-removed",
            Self::HalftoneOriginRemoved => "halftone-origin-removed",
            Self::DestinationProfileReferenceRemoved => "destination-profile-reference-removed",
            Self::OverprintModeRestated => "overprint-mode-restated",
            Self::OverprintModeZeroed => "overprint-mode-zeroed",
            Self::ProfileTextSupplied => "profile-text-supplied",
            Self::ProfileAlternateWritten => "profile-alternate-written",
            Self::DuplicateProfileWrittenDeviceCmyk => "duplicate-profile-written-device-cmyk",
            Self::IccComponentCountRestated => "icc-component-count-restated",
            Self::RoleMapSupplied => "role-map-supplied",
            Self::CatalogLanguageRestated => "catalog-language-restated",
            Self::MarkedLanguagesRestated => "marked-languages-restated",
        }
    }
}

/// The bytes of a converted document, and what the rewrites touched.
pub(super) struct Converted {
    /// The whole output file.
    pub(super) bytes: Vec<u8>,
    /// How many places each rewrite touched.
    pub(super) applied: BTreeMap<Rewrite, usize>,
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
pub(super) fn convert(
    document: &Document,
    target: Target,
    wanted: &BTreeSet<Rewrite>,
    version: Version,
    prepared: &Prepared,
    remedies: &super::remedies::Remedies,
) -> Result<Converted, Refusal> {
    let root = crate::optimize::catalog_of(document)?;
    let sites = Sites::of(document, root, wanted);
    let rewriter = Rewriter::of(document, target, wanted, sites, remedies, prepared);
    let mut applied = BTreeMap::new();

    let mut assembly = Assembly::new(vec![document]);
    let mut replaced: Vec<(ObjectId, Object)> = Vec::new();
    let mapped = walk(&rewriter, &mut assembly, root, &mut replaced, &mut applied)
        .map_err(|error| Refusal::Assembly(error.to_string()))?;
    assembly.set_root(mapped);
    the_information_dictionary(
        &rewriter,
        &mut assembly,
        prepared,
        &mut replaced,
        &mut applied,
    )?;

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
    count_what_has_no_place(wanted, prepared, &mut applied);
    Ok(Converted { bytes, applied })
}

impl<'a> Rewriter<'a> {
    /// The rewriter for one conversion: what was wanted, and each preparation's result where it
    /// succeeded.
    fn of(
        document: &'a Document,
        target: Target,
        wanted: &'a BTreeSet<Rewrite>,
        sites: Sites,
        remedies: &'a super::remedies::Remedies,
        prepared: &'a Prepared,
    ) -> Self {
        Self {
            document,
            target,
            wanted,
            sites,
            remedies,
            added: prepared.added(),
            intent: prepared.intent.as_ref().ok(),
            metadata: prepared.metadata.as_ref().ok(),
            default_cmyk: prepared.default_cmyk.as_ref().ok(),
            default_rgb: prepared.default_rgb.as_ref().ok(),
            to_unicode: prepared.to_unicode.as_ref().ok(),
            fresh: prepared.fresh.as_ref().ok(),
            amended: prepared.amended.as_ref().ok(),
            described: prepared.described.as_ref().ok(),
            fields_described: prepared.fields_described.as_ref().ok(),
            cleaned: prepared.properties.as_ref().ok(),
            respelled: prepared.respelled.as_ref().ok(),
            appearances: prepared.appearances.as_ref().ok(),
            forbidden_annotations: prepared.forbidden_annotations.as_ref().ok(),
            hidden_annotations: prepared.hidden_annotations.as_ref().ok(),
            reference_xobjects: prepared.reference_xobjects.as_ref().ok(),
            transcodes: prepared.transcodes.as_ref().ok(),
            shipped_cmaps: prepared.shipped_cmaps.as_ref().ok(),
            extra_appearance_states: prepared.extra_appearance_states.as_ref().ok(),
            automatic_states: prepared.automatic_states.as_ref().ok(),
            form: prepared.form.as_ref().ok().copied(),
            xfa: prepared.xfa.as_ref().ok().copied(),
            field_appearances: prepared.field_appearances.as_ref().ok(),
            embedded_files: prepared.embedded_files.as_ref().ok(),
            substitutes: prepared.substitutes.as_ref().ok(),
            metrics: prepared.metrics.as_ref().ok(),
            rendering_intents: prepared.owed.rendering_intents.as_ref().ok(),
            transfers: prepared.owed.transfers.as_ref().ok(),
            blend_modes: prepared.owed.blend_modes.as_ref().ok(),
            halftone_names: prepared.owed.halftone_names.as_ref().ok(),
            halftone_defaults: prepared.owed.halftone_defaults.as_ref().ok(),
            halftone_phases: prepared.owed.halftone_phases.as_ref().ok(),
            halftone_origins: prepared.owed.halftone_origins.as_ref().ok(),
            profile_references: prepared.owed.profile_references.as_ref().ok(),
            supplied_destination: prepared.supplied_destination.as_ref().ok(),
            replaced_destination: prepared.replaced_destination.as_ref().ok(),
            restated_destination: prepared.restated_destination.as_ref().ok(),
            overprint_modes: prepared.owed.overprint_modes.as_ref().ok(),
            profile_alternates: prepared
                .owed
                .profile_alternates
                .as_ref()
                .ok()
                .map(|alternates| &alternates.at),
            duplicate_profiles: prepared
                .owed
                .duplicate_profiles
                .as_ref()
                .ok()
                .map(|duplicates| &duplicates.profiles),
            component_counts: prepared
                .owed
                .component_counts
                .as_ref()
                .ok()
                .map(|counts| &counts.at),
            appearance_states: prepared.owed.appearance_states.as_ref().ok(),
            orders: prepared.owed.orders.as_ref().ok(),
            page_resources: prepared.owed.page_resources.as_ref().ok(),
            descriptor_sets: prepared.owed.descriptor_sets.as_ref().ok(),
            cid_to_gid: prepared.owed.cid_to_gid.as_ref().ok(),
            packet_headers: prepared.owed.packet_headers.as_ref().ok(),
            symbolic_encodings: prepared.owed.symbolic_encodings.as_ref().ok(),
            standard_encodings: prepared.owed.standard_encodings.as_ref().ok(),
            standard_route: prepared.owed.standard_route.as_ref().ok(),
            type3_notdef: prepared.owed.type3_notdef.as_ref().ok(),
            shared_profile: prepared.owed.shared_profile.as_ref().ok(),
            colorants: prepared.owed.colorants.as_ref().ok(),
            specifications: prepared.owed.specifications.as_ref().ok(),
            signatures: Some(&prepared.signatures),
            foreign_handlers: prepared.owed.foreign_handlers.as_ref().ok(),
            preserved: prepared.preserved.as_ref().ok(),
            attached: prepared.attached.as_ref().ok(),
            actions: prepared.actions.as_ref().ok(),
            hexadecimal: prepared.hexadecimal.as_ref().ok(),
            languages: prepared.languages.as_ref().ok(),
            external_data: prepared.external_data.as_ref().ok(),
            boundaries: prepared.boundaries.as_ref().ok(),
        }
    }
}

/// §14.3.3's document information dictionary, carried into the output or left out of it.
///
/// The trailer's second root: nothing in the catalog reaches it, so a walk from `/Root` alone
/// would drop a document's title and author. ISO 19005-4 section 6.1.3 is the other case — the
/// values are in the packet by then, so what the trailer states is either nothing at all or the
/// one entry the clause keeps beside a `/PieceInfo`. **Not walking the dictionary is what removes
/// it**, and the objects only it reached go with it, which is the rule every removal in this verb
/// follows (`doc/adr/1099`).
fn the_information_dictionary(
    rewriter: &Rewriter<'_>,
    assembly: &mut Assembly<'_>,
    prepared: &Prepared,
    replaced: &mut Vec<(ObjectId, Object)>,
    applied: &mut BTreeMap<Rewrite, usize>,
) -> Result<(), Refusal> {
    let Some(info) = rewriter
        .document
        .trailer()
        .get("Info")
        .and_then(Object::as_reference)
    else {
        return Ok(());
    };
    let Some(information) = prepared
        .information
        .as_ref()
        .ok()
        .filter(|_| rewriter.wants(Rewrite::InformationMovedIntoThePacket))
    else {
        let carried = walk(rewriter, assembly, info, replaced, applied)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
        assembly.set_info(Some(carried));
        return Ok(());
    };
    if let Some(date) = &information.kept_modification_date {
        let mut kept = Dictionary::new();
        kept.insert(Name::new(&b"ModDate"[..]), date.clone());
        let placed = assembly
            .add(Object::Dictionary(kept))
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
        assembly.set_info(Some(placed));
    }
    Ok(())
}

/// The rewrites there is no place to count, counted from what was prepared instead.
///
/// Four of them are the file's own shape or a construction that is one per document by
/// definition; the rest are the ones whose *places* are not objects — a packet may lose six
/// properties, a stream may state six odd hexadecimal strings, and a report saying "1 done" would
/// be counting the object rather than the work.
fn count_what_has_no_place(
    wanted: &BTreeSet<Rewrite>,
    prepared: &Prepared,
    applied: &mut BTreeMap<Rewrite, usize>,
) {
    for whole in [
        Rewrite::FileHeader,
        Rewrite::WholeFileRewritten,
        Rewrite::OutputIntent,
        Rewrite::IdentificationSchema,
        Rewrite::EncryptionRemoved,
    ] {
        // No entry is appended where the producer's own was completed instead (`doc/adr/2035`).
        let completed_instead = whole == Rewrite::OutputIntent
            && wanted.contains(&Rewrite::DestinationProfileSupplied)
            && prepared.supplied_destination.is_ok();
        if wanted.contains(&whole) && !completed_instead {
            applied.insert(whole, 1);
        }
    }
    // The one rewrite whose places are properties rather than objects: a packet may lose six of
    // them and a report saying "1 done" would be counting the stream instead of the loss.
    if wanted.contains(&Rewrite::PropertyOutsideItsSchema)
        && let Ok(cleaned) = &prepared.properties
    {
        applied.insert(Rewrite::PropertyOutsideItsSchema, cleaned.removed.len());
    }
    // The same, for the same reason: a packet may respell five fields of one description, and a
    // report saying "1 done" would be counting the stream rather than the spellings.
    if (wanted.contains(&Rewrite::ExtensionSchemaPrefixes)
        || wanted.contains(&Rewrite::ExtensionContainerDiscarded))
        && let Ok(respelled) = &prepared.respelled
    {
        applied.insert(Rewrite::ExtensionSchemaPrefixes, respelled.fields);
        // A container and each property it took with it are a place apiece, and every one of
        // them is in the removed list.
        if respelled.discarded > 0 {
            applied.insert(
                Rewrite::ExtensionContainerDiscarded,
                respelled.removed.len(),
            );
        }
    }
    // The three writers `doc/adr/1245` and `doc/adr/1246` added, counted from the preparation
    // rather than from the walk. **A packet the identification schema is also restated into is
    // written by the branch above their own**, which returns before they are reached, so a walk
    // counting them would report nothing done for the one stream that matters most.
    if wanted.contains(&Rewrite::FreshMetadataPacket)
        && let Ok(fresh) = &prepared.fresh
    {
        applied.insert(Rewrite::FreshMetadataPacket, fresh.replaced.len());
    }
    if wanted.contains(&Rewrite::AmendmentIdentifierRemoved)
        && let Ok(amended) = &prepared.amended
    {
        applied.insert(Rewrite::AmendmentIdentifierRemoved, amended.removed.len());
    }
    if wanted.contains(&Rewrite::ExtensionSchemaDescribed)
        && let Ok(described) = &prepared.described
    {
        applied.insert(Rewrite::ExtensionSchemaDescribed, described.described.len());
    }
    if wanted.contains(&Rewrite::ExtensionFieldDescribed)
        && let Ok(fields) = &prepared.fields_described
    {
        applied.insert(Rewrite::ExtensionFieldDescribed, fields.described.len());
    }
    // The entries rather than the dictionary: one `/Info` may hold eight of them, and a report
    // saying "1 done" would be counting the object instead of what moved.
    if wanted.contains(&Rewrite::InformationMovedIntoThePacket)
        && let Ok(information) = &prepared.information
    {
        applied.insert(
            Rewrite::InformationMovedIntoThePacket,
            information.entries.len(),
        );
    }
    // The three action rewrites, for the same reason once more: one edited object may be one
    // action removed or six, and a count of objects would tell an operator who authorised
    // *buttons stop doing things* how many dictionaries had changed rather than how many buttons.
    // The digits rather than the streams: one stream may state six odd strings, and a report
    // saying "1 done" would be counting the object instead of the repair.
    if wanted.contains(&Rewrite::HexadecimalDigitCompleted)
        && let Ok(completed) = &prepared.hexadecimal
    {
        applied.insert(Rewrite::HexadecimalDigitCompleted, completed.digits);
    }
    // The entries rather than the objects: a content stream may respell three property lists'
    // `/Lang`, and a report saying "1 done" would be counting the stream (`doc/adr/2066`).
    if let Ok(restated) = &prepared.languages {
        for (rewrite, class) in [
            (
                Rewrite::CatalogLanguageRestated,
                super::languages::Class::Catalog,
            ),
            (
                Rewrite::MarkedLanguagesRestated,
                super::languages::Class::Marked,
            ),
        ] {
            if wanted.contains(&rewrite) {
                applied.insert(rewrite, restated.places(class));
            }
        }
    }
    if let Ok(removals) = &prepared.actions {
        for rewrite in [
            Rewrite::ForbiddenActionRemoved,
            Rewrite::AdditionalActionsRemoved,
            Rewrite::WidgetActionEntryRemoved,
        ] {
            if wanted.contains(&rewrite) {
                applied.insert(rewrite, removals.places(rewrite));
            }
        }
    }
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
/// `Ok(None)` for an object the file does not hold, which is not an error: §7.3.10 makes a
/// reference to one "a reference to the null object", and the serializer writes that null.
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
    /// §7.7.3.2's root node of the page tree, where a rewrite has to add a page to it.
    root_of_the_pages: Option<ObjectId>,

    /// Every annotation object a page's `/Annots` names, except a `Popup`.
    annotations: BTreeSet<ObjectId>,
    /// Where a `/DefaultCMYK` or a `/DefaultRGB` has to be written, where one is being written.
    defaults: DefaultSites,
}

impl Sites {
    /// The positions, read off the document.
    fn of(document: &Document, catalog: ObjectId, wanted: &BTreeSet<Rewrite>) -> Self {
        let names = document
            .get(catalog)
            .as_dict()
            .and_then(|dict| dict.get("Names").and_then(Object::as_reference));
        let tree = Pages::new(document);
        let pages: BTreeSet<ObjectId> = tree
            .every()
            .into_iter()
            .filter_map(|page| page.and_then(|page| page.id))
            .collect();
        let defaults =
            if wanted.contains(&Rewrite::DefaultCmyk) || wanted.contains(&Rewrite::DefaultRgb) {
                DefaultSites::of(document, &tree)
            } else {
                // Nothing is walked that no failed requirement asked for, which is `doc/adr/0947`'s
                // first rule applied to a *reading* rather than to a rewrite.
                DefaultSites::default()
            };
        let annotations = if wanted.contains(&Rewrite::AnnotationFlags) {
            annotation_objects(document, &tree)
        } else {
            BTreeSet::new()
        };
        // A page appended to the document is added to the node §7.7.3.2 makes the catalog's
        // `/Pages`, whose `/Kids` "may be a combination of page tree nodes and page objects" — so
        // a page may be a kid of the root whatever shape the rest of the tree has.
        let root_of_the_pages = wanted
            .contains(&Rewrite::PreservedAsPage)
            .then(|| {
                document
                    .get(catalog)
                    .as_dict()
                    .and_then(|dict| dict.get("Pages").and_then(Object::as_reference))
            })
            .flatten();
        Self {
            catalog,
            names,
            pages,
            root_of_the_pages,
            annotations,
            defaults,
        }
    }
}

/// Every annotation §12.5.2's page `/Annots` array names, except the subtype ISO 19005 exempts.
///
/// **A structural walk rather than the validator's list**, for [`DefaultSites`]'s reason: a findings
/// list is capped, and a document with more annotations than the cap is one this rewrite would
/// otherwise half-finish. An annotation written directly into the array rather than as an object
/// of its own is not reached — this walk rewrites objects — and the requirement then stays failed,
/// which `doc/adr/0947`'s third stage refuses the file for.
fn annotation_objects(document: &Document, tree: &Pages<'_>) -> BTreeSet<ObjectId> {
    let mut out = BTreeSet::new();
    for page in tree.every().into_iter().flatten() {
        let Some(annotations) = document
            .get_key(&page.dict, "Annots")
            .as_array()
            .map(<[Object]>::to_vec)
        else {
            continue;
        };
        for entry in &annotations {
            let Some(id) = entry.as_reference() else {
                continue;
            };
            let resolved = document.get(id);
            let Some(dict) = resolved.as_dict() else {
                continue;
            };
            // §12.5.6.14's popup is the window belonging to some other annotation, and both
            // parts' section 6.3.2 name it as the one subtype the entry is not required of.
            if document
                .get_key(dict, "Subtype")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Popup")
            {
                continue;
            }
            out.insert(id);
        }
    }
    out
}

/// §12.5.3's Table 167 `Print` bit, at bit position 3 counted from the low-order bit.
const PRINT: i64 = 4;

/// Every dictionary a `/DefaultCMYK` or a `/DefaultRGB` has to reach, found once before the walk
/// starts.
///
/// # Why this is a structural walk rather than a list of failures
///
/// `TechNote 0010`'s A028 resolves that ISO 19005-2 is read as if a default colour space had
/// itself to be defined in the resources dictionary *explicitly associated* with the content
/// stream, and A003 says which four dictionaries that names: a page's, a tiling pattern's, a form
/// `XObject`'s and a Type 3 font's. So the sites are decided by the document's structure, not by
/// where the validator happened to record a failure — and the validator's own list is capped,
/// which would make a document failing in more places than the cap a document this rewrite
/// silently half-finished.
///
/// **Every such dictionary, not only the ones whose stream paints in `DeviceCMYK`.** A
/// `/DefaultCMYK` says how the four numbers of a `DeviceCMYK` value are to be read, and a file in
/// which the same `0 0 0 1 k` meant one thing on one page and another on the next would be a
/// worse file than the one that came in. Adding the entry where nothing selects `DeviceCMYK`
/// changes no mark at all: §8.6.5.6's remapping happens when a device space is *used*.
#[derive(Debug, Default)]
struct DefaultSites {
    /// `/ColorSpace` subdictionaries reached by reference, which receive the entry themselves.
    spaces: BTreeSet<ObjectId>,
    /// Resource dictionaries reached by reference whose `/ColorSpace` is direct or absent.
    resources: BTreeSet<ObjectId>,
    /// Objects whose own `/Resources` is a direct dictionary with a direct or absent
    /// `/ColorSpace`, so that the entry is written inside the object being rewritten anyway.
    direct: BTreeSet<ObjectId>,
    /// The `/Resources` value a page stating none of its own is to be given.
    ///
    /// A page with no `Resources` entry has no explicitly associated dictionary at all, so A028
    /// leaves it no default whatever the page tree above it defines — and the requirement is
    /// therefore asking for the entry the page does not have. What is written is what §7.7.3.4's
    /// inheritance already puts in force: the reference the page inherits, where it inherits one
    /// by reference, and a copy of the dictionary otherwise. Neither changes what any name on
    /// that page resolves to.
    pages: BTreeMap<ObjectId, Object>,
}

/// How many objects [`DefaultSites`] visits before it stops.
///
/// A bound rather than a reading (principle 3): the walk follows a resource graph whose shape is
/// the document's, and a file can nest form `XObject`s inside patterns inside forms as deep as it
/// likes. Past this, the sites that were found are still written and the output's own verdict is
/// what reports that the requirement is not met — which is the net `doc/adr/0947` built.
const MAX_RESOURCE_SITES: usize = 65_536;

impl DefaultSites {
    /// The four dictionaries A003 names, reached from every page.
    fn of(document: &Document, tree: &Pages<'_>) -> Self {
        let mut found = Self::default();
        let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
        let mut queue: VecDeque<Object> = VecDeque::new();
        for page in tree.every().into_iter().flatten() {
            if let Some(id) = page.id {
                found.holder(
                    document,
                    id,
                    &page.dict,
                    Some(&page.dict),
                    &mut seen,
                    &mut queue,
                );
            }
            for appearance in appearance_streams(document, &page.dict) {
                queue.push_back(appearance);
            }
        }
        while let Some(object) = queue.pop_front() {
            if seen.len() >= MAX_RESOURCE_SITES {
                break;
            }
            let Some(id) = object.as_reference() else {
                continue;
            };
            if !seen.insert(id) {
                continue;
            }
            let resolved = document.get(id);
            let dict = match &resolved {
                Object::Stream(stream) => &stream.dict,
                Object::Dictionary(dict) => dict,
                _ => continue,
            };
            found.holder(document, id, dict, None, &mut seen, &mut queue);
        }
        found
    }

    /// One dictionary that carries an explicitly associated resources dictionary, or could.
    ///
    /// `page` is the page's own dictionary where this holder *is* a page, because only a page
    /// inherits resources and only a page therefore gets one written for it.
    fn holder(
        &mut self,
        document: &Document,
        id: ObjectId,
        dict: &Dictionary,
        page: Option<&Dictionary>,
        seen: &mut BTreeSet<ObjectId>,
        queue: &mut VecDeque<Object>,
    ) {
        match dict.get("Resources") {
            Some(Object::Reference(resources)) => {
                let held = document.get(*resources);
                if let Some(held) = held.as_dict() {
                    self.resource_dictionary(document, Some(*resources), held, seen, queue);
                }
            }
            Some(Object::Dictionary(resources)) => {
                self.resource_dictionary(document, None, resources, seen, queue);
                if !Self::spaces_are_indirect(resources) {
                    self.direct.insert(id);
                }
            }
            _ => {
                if let Some(page) = page {
                    self.inherited(document, id, page);
                }
            }
        }
    }

    /// Whether a resource dictionary states its `/ColorSpace` as another object.
    ///
    /// Where it does, that object is what receives the entry and the dictionary holding it is
    /// left exactly as its producer wrote it.
    fn spaces_are_indirect(resources: &Dictionary) -> bool {
        matches!(resources.get("ColorSpace"), Some(Object::Reference(_)))
    }

    /// One resource dictionary: where its `/DefaultCMYK` goes, and what it reaches.
    ///
    /// A dictionary shared by five hundred pages is read once, not five hundred times: the
    /// entries it names are the same entries whichever page reached it.
    fn resource_dictionary(
        &mut self,
        document: &Document,
        id: Option<ObjectId>,
        resources: &Dictionary,
        seen: &mut BTreeSet<ObjectId>,
        queue: &mut VecDeque<Object>,
    ) {
        if let Some(id) = id
            && !seen.insert(id)
        {
            return;
        }
        match resources.get("ColorSpace") {
            Some(Object::Reference(spaces)) => {
                self.spaces.insert(*spaces);
            }
            _ => {
                if let Some(id) = id {
                    self.resources.insert(id);
                }
            }
        }
        // §7.8.3's categories, narrowed to the three that can hold a content stream of their own.
        for (category, wanted) in [
            ("XObject", Wanted::Form),
            ("Pattern", Wanted::Tiling),
            ("Font", Wanted::Type3),
        ] {
            let Some(entries) = document.get_key(resources, category).as_dict().cloned() else {
                continue;
            };
            for (_, entry) in entries.iter() {
                if wanted.matches(document, entry) {
                    queue.push_back(entry.clone());
                }
            }
        }
    }

    /// A page with no `Resources` entry of its own, given the one §7.7.3.4 already puts in force.
    ///
    /// The value is `super::sites::inherited_resources`'s, which is the same question asked for
    /// `Rewrite::PageResources` — a page inherits one dictionary and both rewrites give it that
    /// one. What differs is only where the `/DefaultCMYK` then goes: a reference is shared, so
    /// the entry is written into the dictionary once for every page that shares it, and a copy
    /// is the page's own object and takes the entry inline.
    fn inherited(&mut self, document: &Document, id: ObjectId, page: &Dictionary) {
        let resources = sites::inherited_resources(document, page);
        match &resources {
            Object::Reference(at) => {
                self.resources.insert(*at);
            }
            _ => {
                self.direct.insert(id);
            }
        }
        self.pages.insert(id, resources);
    }
}

/// Which kind of content-stream holder a resource entry has to be to matter.
#[derive(Debug, Clone, Copy)]
enum Wanted {
    /// §8.10's form `XObject`, which is also what an annotation's appearance is.
    Form,
    /// §8.7.3.1's tiling pattern, whose `PatternType` is 1 and which is a stream.
    Tiling,
    /// §9.6.4's Type 3 font, whose glyph procedures are content streams.
    Type3,
}

impl Wanted {
    /// Whether this resource entry is one.
    fn matches(self, document: &Document, entry: &Object) -> bool {
        let resolved = document.resolve(entry);
        let dict = match &resolved {
            Object::Stream(stream) => &stream.dict,
            Object::Dictionary(dict) => dict,
            _ => return false,
        };
        let named = |key: &str, value: &[u8]| {
            document
                .get_key(dict, key)
                .as_name()
                .is_some_and(|name| name.as_bytes() == value)
        };
        match self {
            Self::Form => named("Subtype", b"Form"),
            Self::Tiling => document.get_key(dict, "PatternType").as_integer() == Some(1),
            Self::Type3 => named("Subtype", b"Type3"),
        }
    }
}

/// Every appearance stream §12.5.5 reaches from one page's annotations.
///
/// An annotation's appearance is a form `XObject` with resources of its own, so it is one of
/// A003's four dictionaries — and nothing in a page's own `/Resources` reaches it.
fn appearance_streams(document: &Document, page: &Dictionary) -> Vec<Object> {
    let mut out = Vec::new();
    let Some(annotations) = document
        .get_key(page, "Annots")
        .as_array()
        .map(<[Object]>::to_vec)
    else {
        return out;
    };
    for annotation in &annotations {
        let Some(appearances) = document
            .resolve(annotation)
            .as_dict()
            .map(|dict| document.get_key(dict, "AP"))
            .and_then(|ap| ap.as_dict().cloned())
        else {
            continue;
        };
        // §12.5.5's Table 168: each of `/N`, `/R` and `/D` is either a stream or a
        // subdictionary of streams, one per appearance state.
        for (_, state) in appearances.iter() {
            match state {
                Object::Reference(_) => out.push(state.clone()),
                _ => {
                    if let Some(states) = document.resolve(state).as_dict() {
                        out.extend(states.iter().map(|(_, one)| one.clone()));
                    }
                }
            }
        }
    }
    out
}

/// Which entry [`Rewriter::restate_inside`] is restating.
///
/// Two rewrites share one walk because they share one problem: the validator names the object a
/// dictionary is *written in*, and the dictionary the requirement is about may be several levels
/// down inside it. What differs is one key and one test, which is what this enum carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    /// A graphics state's `/RI`, and an image `XObject`'s `/Intent`.
    RenderingIntent,
    /// A `/BM` written as an array of names.
    BlendMode,
}

/// What became of one object on the way into the output.
enum Rewritten {
    /// Unchanged: the source's bytes cross to the sink.
    Carried,
    /// Changed, in the source's numbering.
    Changed(Object),
}

/// Table 225's `AppendOnly` bit of `/SigFlags`: "[i]f set, the document contains signatures
/// that may be invalidated if the PDF file is saved (written) in a way that alters its previous
/// contents". The output contains none, so the bit comes off; bit 1, `SignaturesExist`, is a
/// statement about the *fields*, which stay, and is left as the producer wrote it.
///
/// §12.7.3 numbers the positions "from 1 (low-order) to 32 (high-order)", so bit position 2 is
/// the value 2. An entry that is not an integer states no flags and is left alone.
fn clear_append_only(form: &mut Dictionary, applied: &mut BTreeMap<Rewrite, usize>) -> bool {
    const APPEND_ONLY: i64 = 2;
    let Some(flags) = form.get("SigFlags").and_then(Object::as_integer) else {
        return false;
    };
    if flags & APPEND_ONLY == 0 {
        return false;
    }
    form.insert(
        Name::new(&b"SigFlags"[..]),
        Object::Integer(flags & !APPEND_ONLY),
    );
    count(applied, Rewrite::SignatureValueRemoved);
    true
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
    /// The `/DefaultCMYK` to write, where one is being written.
    default_cmyk: Option<&'a DefaultCmyk>,
    /// The `/DefaultRGB` to write, where one is being written (`doc/adr/2114`).
    default_rgb: Option<&'a super::default_rgb::DefaultRgb>,
    /// The `/ToUnicode` `CMap` each font is to name, where any are being written.
    to_unicode: Option<&'a DerivedMaps>,
    /// The packet each metadata stream is to carry in place of one this tree cannot read.
    fresh: Option<&'a Fresh>,
    /// The packet each metadata stream is to carry, where a malformed amendment identifier goes.
    amended: Option<&'a Amended>,
    /// The packet each metadata stream is to carry, where an extension schema gains a container.
    described: Option<&'a SchemasDescribed>,
    /// The packet each metadata stream is to carry, where a value type gains its fields.
    fields_described: Option<&'a FieldsDescribed>,
    /// The packet each metadata stream is to carry, where properties are being removed.
    cleaned: Option<&'a Cleaned>,
    /// The packet each metadata stream is to carry, where container prefixes are being respelled.
    respelled: Option<&'a Respelled>,
    /// The appearance each annotation's `/AP` `/N` is to name, where any are being constructed.
    appearances: Option<&'a Appearances>,
    /// The annotations to take out of their pages' `/Annots`, where any are being removed.
    forbidden_annotations: Option<&'a ForbiddenAnnotations>,
    /// The annotations whose stated `/F` ISO 19005 forbids, where any are being removed.
    hidden_annotations: Option<&'a HiddenAnnotations>,
    /// The reference `XObject`s whose `Ref` entry a `preserve` removes (`doc/adr/1285`).
    reference_xobjects: Option<&'a super::in_place::ReferenceXObjects>,
    /// The JPEG 2000 images a `preserve` transcodes, keyed by the image (`doc/adr/1400`).
    transcodes: Option<&'a super::transcode::Transcodes>,
    /// The `CMap` streams a `preserve` embeds, keyed by the font naming each (`doc/adr/1286`).
    shipped_cmaps: Option<&'a super::cmaps::ShippedCMaps>,
    /// The annotations whose `/AP` is reduced to `/N`, where any are.
    extra_appearance_states: Option<&'a ExtraAppearanceStates>,
    /// The optional content configurations losing their `/AS`, where any are.
    automatic_states: Option<&'a sites::AutomaticStates>,
    /// Where the interactive form dictionary is, where its `/NeedAppearances` is being cleared.
    form: Option<Site>,
    /// The same site, where its `/XFA` is being removed.
    xfa: Option<Site>,
    /// The appearance each widget's `/AP` `/N` is to name, where any are being constructed.
    field_appearances: Option<&'a FieldAppearances>,
    /// The file specifications whose embedded file goes, and the entries that named them.
    embedded_files: Option<&'a sites::EmbeddedFiles>,
    /// The `/FontFile` entry each font descriptor is to gain, where any are being embedded.
    substitutes: Option<&'a Substitutes>,
    /// The font program stream to write in place of each one being restated.
    metrics: Option<&'a Metrics>,
    /// The graphics states and images whose rendering intent is restated.
    rendering_intents: Option<&'a sites::Sites>,
    /// The graphics states whose `/TR` goes.
    transfers: Option<&'a super::transfer::TransferFunctions>,
    /// The dictionaries whose `/BM` array is restated.
    blend_modes: Option<&'a sites::Sites>,
    /// The halftones whose `HalftoneName` goes.
    halftone_names: Option<&'a sites::Sites>,
    /// The graphics states whose `HT` is written `Default` (`doc/adr/2034`).
    halftone_defaults: Option<&'a sites::Sites>,
    /// The graphics states whose `HTP` goes (`doc/adr/2035`).
    halftone_phases: Option<&'a sites::Sites>,
    /// The graphics states whose `HTO` goes (`doc/adr/2034`).
    halftone_origins: Option<&'a sites::Sites>,
    /// The output intents whose `DestOutputProfileRef` goes (`doc/adr/2083`).
    profile_references: Option<&'a sites::Sites>,
    /// The PDF/A output intents given the profile this conversion's own entry names
    /// (`doc/adr/2035`).
    supplied_destination: Option<&'a super::prepare::SuppliedDestination>,
    /// The destination profile written over one this reader does not draw through.
    replaced_destination: Option<&'a super::prepare::ReplacedDestination>,
    /// The destination profile whose header is restated, where it is (`doc/adr/2021`).
    restated_destination: Option<&'a super::destination::RestatedDestination>,
    /// The graphics states whose overprint mode becomes 0.
    overprint_modes: Option<&'a super::overprint::OverprintModes>,
    /// The space each unreadable ICC profile's `ICCBased` arrays are written as, by profile.
    profile_alternates: Option<&'a BTreeMap<ObjectId, Object>>,
    /// The profiles whose `ICCBased` arrays are written `DeviceCMYK` (`doc/adr/2098`).
    duplicate_profiles: Option<&'a BTreeSet<ObjectId>>,
    /// The `N` each `ICCBased` profile stream restates, by stream (`doc/adr/2099`).
    component_counts: Option<&'a BTreeMap<ObjectId, i64>>,
    /// The stream each annotation's normal appearance collapses to.
    appearance_states: Option<&'a AppearanceStates>,
    /// The `/Order` arrays completed, and where each goes.
    orders: Option<&'a CompletedOrders>,
    /// The `/Resources` each page with none of its own is given.
    page_resources: Option<&'a PageResources>,
    /// The subset descriptions removed from each font descriptor.
    descriptor_sets: Option<&'a DescriptorSets>,
    /// The `CIDFont`s given `/CIDToGIDMap` `/Identity`.
    cid_to_gid: Option<&'a sites::Sites>,
    /// The packets whose header loses a deprecated attribute, where any do.
    packet_headers: Option<&'a Headers>,
    /// The symbolic TrueType fonts whose `/Encoding` goes.
    symbolic_encodings: Option<&'a sites::Sites>,
    /// The `/Encoding` each non-symbolic TrueType font is to state.
    standard_encodings: Option<&'a StandardEncodings>,
    /// The `/Encoding` each TrueType font whose codes are named for §9.6.5.4's route is to state.
    standard_route: Option<&'a StandardRouteNames>,
    /// The `/Encoding` and `/CharProcs` each Type 3 font with a renamed `.notdef` is to state.
    type3_notdef: Option<&'a Type3Renames>,
    /// The one destination profile every output intent is to name, where they are being shared.
    shared_profile: Option<&'a SharedProfile>,
    /// The `/Colorants` entries each `DeviceN` colour space is to gain, where any are written.
    colorants: Option<&'a ColorantEntries>,
    /// The `JPXDecode` images whose colour specification boxes are reduced, where any are.
    specifications: Option<&'a Specifications>,
    /// Every signature the source carries, and where the rewrite reaches each.
    signatures: Option<&'a Signatures>,
    /// The permissions dictionary's keys outside Table 263, where any are removed.
    foreign_handlers: Option<&'a ForeignHandlers>,
    /// What the operator's configuration answered, for the two remedies that reach a rewrite.
    remedies: &'a super::remedies::Remedies,
    /// The pages a `preserve` remedy composed, where any were composed.
    preserved: Option<&'a Composed>,
    /// The files a `preserve` remedy attached, where any were attached.
    attached: Option<&'a super::attach::Attached>,
    /// The entries the action clauses ask each object to restate, where any are asked.
    actions: Option<&'a super::actions::Removals>,
    /// The content streams whose hexadecimal strings state their final digit, where any do.
    hexadecimal: Option<&'a super::hexadecimal::Completed>,
    /// The `/Lang` entries restated, and the objects they are in (`doc/adr/2066`).
    languages: Option<&'a super::languages::Restated>,
    /// The streams whose data comes inside the file, where any do.
    external_data: Option<&'a super::external::Embedded>,
    /// The out-of-range optional page boundary entries each page loses, where any do.
    boundaries: Option<&'a super::boundaries::Boundaries>,
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
            // A colour space array written as its own object, which is the shape two rewrites
            // reach — `Rewrite::SpotColorantEntry` and `Rewrite::SeparationAgreed` — and the only
            // shape either of them does: every other rewrite in this table writes a dictionary
            // entry.
            Object::Array(_) => self.rewrite_array(id, value, applied),
            _ => Rewritten::Carried,
        }
    }

    /// An object that is an array: both of ISO 19005 section 6.2.4.4's array rewrites, and
    /// nothing else.
    fn rewrite_array(
        &self,
        id: ObjectId,
        value: &Object,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> Rewritten {
        let mut out = value.clone();
        let mut changed = false;
        if self.wants(Rewrite::SpotColorantEntry)
            && let Some(entries) = self.colorants.and_then(|written| written.at.get(&id))
        {
            let placed = sites::place_colorants_in(self.document, &mut out, entries);
            for _ in 0..placed.len() {
                count(applied, Rewrite::SpotColorantEntry);
            }
            changed |= !placed.is_empty();
        }
        if self.wants(Rewrite::SeparationAgreed) && !self.remedies.separations.is_empty() {
            let reached =
                sites::agree_separations_in(self.document, &mut out, &self.remedies.separations);
            for _ in 0..reached.len() {
                count(applied, Rewrite::SeparationAgreed);
            }
            changed |= !reached.is_empty();
        }
        // An `ICCBased` array written as its own object becomes the alternate itself, which may
        // be a name rather than an array — the one rewrite here that changes the object's kind.
        if self.wants(Rewrite::ProfileAlternateWritten)
            && let Some(alternates) = self.profile_alternates
        {
            let written = super::icc::write_alternates(self.document, &mut out, alternates, 0);
            for _ in 0..written {
                count(applied, Rewrite::ProfileAlternateWritten);
            }
            changed |= written > 0;
        }
        if self.wants(Rewrite::DuplicateProfileWrittenDeviceCmyk)
            && let Some(profiles) = self.duplicate_profiles
        {
            let written =
                super::duplicate_profiles::write_device_cmyk(self.document, &mut out, profiles, 0);
            for _ in 0..written {
                count(applied, Rewrite::DuplicateProfileWrittenDeviceCmyk);
            }
            changed |= written > 0;
        }
        if changed {
            Rewritten::Changed(out)
        } else {
            Rewritten::Carried
        }
    }

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
        changed |= self.extend_the_role_map(id, &mut out, applied);
        changed |= self.restate_languages(id, &mut out);
        changed |= self.preserve(id, &mut out, applied);
        changed |= self.remove_the_actions(id, &mut out);
        if Some(id) == self.sites.names
            && self.wants(Rewrite::AlternatePresentations)
            && out.remove("AlternatePresentations").is_some()
        {
            count(applied, Rewrite::AlternatePresentations);
            changed = true;
        }
        changed |= self.file_in_the_name_dictionary(id, &mut out);
        if self.sites.pages.contains(&id)
            && self.wants(Rewrite::PresentationSteps)
            && out.remove("PresSteps").is_some()
        {
            count(applied, Rewrite::PresentationSteps);
            changed = true;
        }
        changed |= self.remove_the_annotations(id, &mut out, applied);
        changed |= self.relocate_onto_page(id, &mut out, applied);
        changed |= self.write_entries_in_place(id, &mut out, applied);
        if self.wants(Rewrite::AppearanceDictionary)
            && let Some(stream) = self
                .appearances
                .and_then(|appearances| appearances.at.get(&id))
        {
            // Both parts' section 6.3.3 make `/N` the only key an appearance dictionary holds,
            // and this annotation states no `/AP` at all — that is the population the preparation
            // was built over — so the dictionary is written whole rather than added to.
            let mut appearance = Dictionary::new();
            appearance.insert(Name::new(&b"N"[..]), Object::Reference(*stream));
            out.insert(Name::new(&b"AP"[..]), Object::Dictionary(appearance));
            count(applied, Rewrite::AppearanceDictionary);
            changed = true;
        }
        changed |= self.interactive_edits(id, &mut out, applied);
        if self.wants(Rewrite::SubstituteFontProgram)
            && let Some(embedding) = self
                .substitutes
                .and_then(|substitutes| substitutes.at.get(&id))
        {
            // §9.9's Table 124 admits at most one of the three keys:
            //
            // > At most, only one of the FontFile , FontFile2 , and FontFile3 entries shall be
            // > present.
            //
            // This descriptor *states* none — the preparation's population is a descriptor whose
            // every such key resolves to null — but it may well hold one of the keys with
            // nothing behind it, which §7.3.7 makes the same thing and Table 124's sentence
            // counts all the same. So the other two go before this one is written.
            for key in ["FontFile", "FontFile2", "FontFile3"] {
                if key != embedding.key {
                    out.remove(key);
                }
            }
            // **`/CharSet` and `/CIDSet` describe a program, and the program they describe was
            // never in this file.** §9.8.1's Table 122 makes both optional and makes each a list
            // of what the *embedded* program contains; ISO 19005-2 section 6.2.11.4.2 then
            // requires the list to be complete. A producer who wrote one for a font they did not
            // embed wrote a description of a font nobody had, so carrying it forward beside a
            // face this converter chose would leave the file asserting something false about its
            // own bytes — and would fail the very clause that checks the assertion, which is how
            // the corpus found this. Removing a description of an absent program takes nothing
            // from the document that a reader could have used.
            out.remove("CharSet");
            out.remove("CIDSet");
            out.insert(
                Name::new(embedding.key.as_bytes()),
                Object::Reference(embedding.at),
            );
            count(applied, Rewrite::SubstituteFontProgram);
            changed = true;
        }
        if self.identity_cid_to_gid(id, &mut out) {
            count(applied, Rewrite::SubstituteFontProgram);
            changed = true;
        }
        if self.wants(Rewrite::SuppliedMediaType)
            && let Some(media_type) = self.remedies.supplied.get(&id)
        {
            // §7.11.4.1's Table 44 makes `/Subtype` the embedded file's media type, and the value
            // written is the operator's own. Their authority is what the report and the file's
            // `xmpMM:History` both record; nothing here is derived from the document.
            out.insert(
                Name::new(&b"Subtype"[..]),
                Object::Name(Name::new(media_type.as_bytes())),
            );
            count(applied, Rewrite::SuppliedMediaType);
            changed = true;
        }
        changed |= self.write_defaults(id, &mut out, applied);
        changed |= self.complete_file_specification(&mut out, applied);
        changed |= self.owed_rewrites(id, &mut out, applied);
        changed |= self.unsign(id, &mut out, applied);
        if self.permissions_site() == Some(Site::Object(id)) {
            changed |= self.edit_permissions(&mut out, applied);
        }
        if self
            .signatures
            .filter(|_| self.wants(Rewrite::SignatureValueRemoved))
            .and_then(|signatures| signatures.form)
            == Some(Site::Object(id))
        {
            changed |= clear_append_only(&mut out, applied);
        }
        if self.wants(Rewrite::ToUnicode)
            && let Some(at) = self.to_unicode.and_then(|maps| maps.at.get(&id))
        {
            // §9.10.3 puts the CMap on the font dictionary, so the entry replaces whatever the
            // producer stated there — including a CMap of their own, whose usable values the
            // derivation has already carried across into the one being written.
            out.insert(Name::new(&b"ToUnicode"[..]), Object::Reference(*at));
            count(applied, Rewrite::ToUnicode);
            changed = true;
        }
        changed.then_some(out)
    }

    /// `doc/pdf-a-mitigations.md` section 13.3's rewrites, each at the objects its own
    /// preparation found.
    ///
    /// One function rather than seven branches in [`Self::rewrite_dictionary`] because they share
    /// exactly one property and it is the one worth naming: every one of them writes a value the
    /// standard states, at objects `super::sites` chose from the validator's findings, and
    /// nowhere else.
    fn owed_rewrites(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        changed |= self.restate_rendering_intent(id, out, applied);
        // One removal under either of its two rewrites: which one the decision took is what
        // the removal costs, and the objects it reaches are the same (`doc/adr/1964`).
        for rewrite in [
            Rewrite::IdentityTransferFunctionRemoved,
            Rewrite::TransferFunctionRemoved,
        ] {
            if self.wants(rewrite)
                && self
                    .transfers
                    .is_some_and(|functions| functions.at.contains(&id))
                && out.remove("TR").is_some()
            {
                count(applied, rewrite);
                changed = true;
            }
            // §10.6.5.6 requires a halftone's own function only of a type 5 component for a
            // nonstandard colourant, and these are the halftones stating one elsewhere.
            if self.wants(rewrite)
                && self
                    .transfers
                    .is_some_and(|functions| functions.halftones.contains(&id))
                && out.remove("TransferFunction").is_some()
            {
                count(applied, rewrite);
                changed = true;
            }
            // Table 57's `Default`, the one value ISO 19005-2 section 6.2.5 admits for `/TR2`.
            if self.wants(rewrite)
                && self
                    .transfers
                    .is_some_and(|functions| functions.second.contains(&id))
            {
                out.insert(
                    Name::new(&b"TR2"[..]),
                    Object::Name(Name::new(&b"Default"[..])),
                );
                count(applied, rewrite);
                changed = true;
            }
        }
        changed |= self.restate_blend_mode(id, out, applied);
        if self.wants(Rewrite::HalftoneNameRemoved)
            && self
                .halftone_names
                .is_some_and(|sites| sites.at.contains(&id))
            && out.remove("HalftoneName").is_some()
        {
            count(applied, Rewrite::HalftoneNameRemoved);
            changed = true;
        }
        changed |= self.halftone_and_intent_entries(id, out, applied);
        // One change under either of its two rewrites, as for the transfer functions above: the
        // decision says what it costs, and the states it reaches are the same (`doc/adr/1978`).
        for rewrite in [Rewrite::OverprintModeRestated, Rewrite::OverprintModeZeroed] {
            if self.wants(rewrite)
                && self
                    .overprint_modes
                    .is_some_and(|modes| modes.at.contains(&id))
            {
                out.insert(Name::new(&b"OPM"[..]), Object::Integer(0));
                count(applied, rewrite);
                changed = true;
            }
        }
        changed |= self.collapse_appearance_states(id, out, applied);
        changed |= self.reduce_appearance_dictionary(id, out, applied);
        changed |= self.complete_order(id, out, applied);
        // After `complete_order`, which replaces an `/OCProperties` dictionary wholesale with the
        // one its own preparation built: this removal edits whatever `out` now holds, so the two
        // compose on a document that needs both rather than one undoing the other.
        changed |= self.remove_automatic_states(id, out, applied);
        // After both, for the same reason: it edits what `out` holds by then.
        changed |= self.name_configurations(id, out, applied);
        changed |= self.attach_page_resources(id, out, applied);
        changed |= self.remove_descriptor_sets(id, out, applied);
        changed |= self.remove_page_boundaries(id, out, applied);
        changed |= self.write_cid_to_gid_map(id, out, applied);
        changed |= self.restate_truetype_encoding(id, out, applied);
        changed |= self.share_destination_profile(id, out, applied);
        changed |= self.write_colorants(id, out, applied);
        changed |= self.agree_separations(out, applied);
        changed |= self.write_profile_alternates(out, applied);
        if self.wants(Rewrite::IccComponentCountRestated)
            && let Some(stated) = self.component_counts.and_then(|counts| counts.get(&id))
        {
            out.insert(Name::new(&b"N"[..]), Object::Integer(*stated));
            count(applied, Rewrite::IccComponentCountRestated);
            changed = true;
        }
        if self.wants(Rewrite::DuplicateProfileWrittenDeviceCmyk)
            && let Some(profiles) = self.duplicate_profiles
        {
            let written =
                super::duplicate_profiles::write_device_cmyk_in(self.document, out, profiles, 0);
            for _ in 0..written {
                count(applied, Rewrite::DuplicateProfileWrittenDeviceCmyk);
            }
            changed |= written > 0;
        }
        changed
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

    /// Whether the document's XMP packet is being written.
    ///
    /// Eight rewrites reach it and each for its own reason: the identification schema is the
    /// file's claim about itself, the `/DefaultCMYK` is an action `doc/questions/A48` requires
    /// recorded in `xmpMM:History`, a removed property — misused, or described by a discarded
    /// container — is both an edit to the packet and
    /// `doc/pdf-a-conversion-limits.md` section 4.2's entry beside it, the replacement, the
    /// amendment cut and the container are the three `doc/adr/1245` and `doc/adr/1246` added, and
    /// the information dictionary's entries are supplemented into it (`doc/adr/1269`). Any one
    /// alone is enough to make the packet the prepared one — which is what carries every edit the
    /// others made, since [`super::prepare::Edited`] folds them in that order.
    fn wants_metadata(&self) -> bool {
        self.wants(Rewrite::IdentificationSchema)
            || self.wants(Rewrite::DefaultCmyk)
            || self.wants(Rewrite::PropertyOutsideItsSchema)
            || self.wants(Rewrite::ExtensionContainerDiscarded)
            || self.wants(Rewrite::FreshMetadataPacket)
            || self.wants(Rewrite::AmendmentIdentifierRemoved)
            || self.wants(Rewrite::ExtensionSchemaDescribed)
            || self.wants(Rewrite::ExtensionFieldDescribed)
            || self.wants(Rewrite::InformationMovedIntoThePacket)
    }
}

/// One `/ColorSpace` subdictionary with §8.6.5.6's `/DefaultCMYK` or `/DefaultRGB` added, as
/// `key` names.
///
/// `None` where it already states one, which is the producer's and stays theirs.
fn with_default(spaces: &Dictionary, key: &str, space: ObjectId) -> Option<Dictionary> {
    if spaces.get(key).is_some() {
        return None;
    }
    let mut out = spaces.clone();
    out.insert(Name::new(key.as_bytes()), Object::Reference(space));
    Some(out)
}

/// One resource dictionary whose `/ColorSpace` states the default.
///
/// `None` where the entry is already there, and where `/ColorSpace` is another object — that
/// object is [`DefaultSites::spaces`]'s business and this dictionary is left alone.
fn resources_with_default(
    resources: &Dictionary,
    key: &str,
    space: ObjectId,
) -> Option<Dictionary> {
    let spaces = match resources.get("ColorSpace") {
        None => Dictionary::new(),
        Some(Object::Dictionary(spaces)) => spaces.clone(),
        Some(_) => return None,
    };
    let spaces = with_default(&spaces, key, space)?;
    let mut out = resources.clone();
    out.insert(Name::new(&b"ColorSpace"[..]), Object::Dictionary(spaces));
    Some(out)
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

/// A configuration's `/Name`, as §7.9.2.2's text string that Table 99 makes it.
fn name_string(name: &str) -> Object {
    Object::String(pdf_syntax::text_string::encode_text_string(name).into())
}

/// The filter each of §8.9.7's Table 92 abbreviations names, as this tree's decoder reads it.
pub(super) const FILTER_ABBREVIATIONS: [(&[u8], &[u8]); 7] = [
    (b"AHx", b"ASCIIHexDecode"),
    (b"A85", b"ASCII85Decode"),
    (b"LZW", b"LZWDecode"),
    (b"Fl", b"FlateDecode"),
    (b"RL", b"RunLengthDecode"),
    (b"CCF", b"CCITTFaxDecode"),
    (b"DCT", b"DCTDecode"),
];

/// A stream dictionary whose `/Filter` names each abbreviated filter in full, where it named one.
///
/// `None` where nothing is abbreviated, or where `/Filter` is not a name or an array of names —
/// an indirect entry is left as its producer wrote it, and the output's own verdict says so.
fn filter_names_spelled_out(dict: &Dictionary) -> Option<Dictionary> {
    let full = |name: &Name| {
        FILTER_ABBREVIATIONS
            .iter()
            .find(|(short, _)| *short == name.as_bytes())
            .map(|(_, long)| Name::new(*long))
    };
    let spelled = match dict.get("Filter")? {
        Object::Name(name) => Object::Name(full(name)?),
        Object::Array(items) => {
            let mut any = false;
            let spelled: Vec<Object> = items
                .iter()
                .map(|item| match item {
                    Object::Name(name) => full(name).map_or_else(
                        || item.clone(),
                        |long| {
                            any = true;
                            Object::Name(long)
                        },
                    ),
                    other => other.clone(),
                })
                .collect();
            if !any {
                return None;
            }
            Object::Array(spelled)
        }
        _ => return None,
    };
    let mut out = dict.clone();
    out.insert(Name::new(&b"Filter"[..]), spelled);
    Some(out)
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
