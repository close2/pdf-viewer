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
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use pdf_archive::Target;
use pdf_model::Pages;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::{Assembly, Form, ObjectStreams, Options, Streams, flate_encode};
use pdf_syntax::{Document, Version, serialize::serialize};

use crate::Refusal;

use super::COMPRESSION_LEVEL;
use super::fonts::{Metrics, Substitutes};
use super::jpeg2000::Specifications;
use super::prepare::{
    Appearances, Cleaned, DefaultCmyk, ExtraAppearanceStates, ForbiddenAnnotations, Headers,
    HiddenAnnotations, Intent, Metadata, Prepared, Respelled, intent_dictionary, metadata_stream,
    output_intent_entries,
};
use super::preserve::Composed;
use super::signatures::{ForeignHandlers, Signatures, Site};
use super::sites::{
    self, AppearanceStates, ColorantEntries, CompletedOrders, DescriptorSets, PageResources,
    RELATIVE_COLORIMETRIC, RENDERING_INTENTS, SharedProfile, StandardEncodings,
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
            Self::DefaultCmyk => {
                "every resource dictionary explicitly associated with a content stream states a \
                 DefaultCMYK: a DeviceN over Cyan, Magenta, Yellow and Black whose tint \
                 transform is ISO 32000-2 \u{a7}10.4.2.5's, over this file's ICC sRGB alternate"
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
            Self::ExtensionSchemaPrefixes => {
                "an extension schema container field stated with another prefix is respelled \
                 with the one its table requires, the declaration that bound the old prefix \
                 along with it, leaving every other byte of the packet as its producer wrote it"
            }
            Self::AnnotationFlags => {
                "an annotation stating no /F is given one whose only set bit is Print, which is \
                 the value the requirement asks for and the default in every other bit"
            }
            Self::ForbiddenAnnotationRemoved => {
                "an annotation of a subtype ISO 19005 does not admit is taken out of its page's \
                 Annots array, and everything only that annotation reached — its media stream, \
                 its 3D artwork, its popup — leaves the file with it"
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
            Self::DefaultCmyk => "default-cmyk",
            Self::MarkInfo => "mark-info",
            Self::AssociatedFileNames => "associated-file-names",
            Self::AssociatedFileRelationship => "associated-file-relationship",
            Self::ToUnicode => "to-unicode",
            Self::PropertyOutsideItsSchema => "property-outside-its-schema",
            Self::ExtensionSchemaPrefixes => "extension-schema-prefixes",
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
            Self::SymbolicTrueTypeEncodingRemoved => "symbolic-truetype-encoding-removed",
            Self::StandardTrueTypeEncoding => "standard-truetype-encoding",
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
    let rewriter = Rewriter {
        document,
        target,
        wanted,
        sites,
        remedies,
        added: prepared.added(),
        intent: prepared.intent.as_ref().ok(),
        metadata: prepared.metadata.as_ref().ok(),
        default_cmyk: prepared.default_cmyk.as_ref().ok(),
        to_unicode: prepared.to_unicode.as_ref().ok(),
        cleaned: prepared.properties.as_ref().ok(),
        respelled: prepared.respelled.as_ref().ok(),
        appearances: prepared.appearances.as_ref().ok(),
        forbidden_annotations: prepared.forbidden_annotations.as_ref().ok(),
        hidden_annotations: prepared.hidden_annotations.as_ref().ok(),
        extra_appearance_states: prepared.extra_appearance_states.as_ref().ok(),
        automatic_states: prepared.automatic_states.as_ref().ok(),
        substitutes: prepared.substitutes.as_ref().ok(),
        metrics: prepared.metrics.as_ref().ok(),
        rendering_intents: prepared.owed.rendering_intents.as_ref().ok(),
        blend_modes: prepared.owed.blend_modes.as_ref().ok(),
        appearance_states: prepared.owed.appearance_states.as_ref().ok(),
        orders: prepared.owed.orders.as_ref().ok(),
        page_resources: prepared.owed.page_resources.as_ref().ok(),
        descriptor_sets: prepared.owed.descriptor_sets.as_ref().ok(),
        cid_to_gid: prepared.owed.cid_to_gid.as_ref().ok(),
        packet_headers: prepared.owed.packet_headers.as_ref().ok(),
        symbolic_encodings: prepared.owed.symbolic_encodings.as_ref().ok(),
        standard_encodings: prepared.owed.standard_encodings.as_ref().ok(),
        shared_profile: prepared.owed.shared_profile.as_ref().ok(),
        colorants: prepared.owed.colorants.as_ref().ok(),
        specifications: prepared.owed.specifications.as_ref().ok(),
        signatures: Some(&prepared.signatures),
        foreign_handlers: prepared.owed.foreign_handlers.as_ref().ok(),
        preserved: prepared.preserved.as_ref().ok(),
        actions: prepared.actions.as_ref().ok(),
        hexadecimal: prepared.hexadecimal.as_ref().ok(),
        external_data: prepared.external_data.as_ref().ok(),
        boundaries: prepared.boundaries.as_ref().ok(),
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
    count_what_has_no_place(wanted, prepared, &mut applied);
    Ok(Converted { bytes, applied })
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
        if wanted.contains(&whole) {
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
    if wanted.contains(&Rewrite::ExtensionSchemaPrefixes)
        && let Ok(respelled) = &prepared.respelled
    {
        applied.insert(Rewrite::ExtensionSchemaPrefixes, respelled.fields);
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
    /// §7.7.3.2's root node of the page tree, where a rewrite has to add a page to it.
    root_of_the_pages: Option<ObjectId>,

    /// Every annotation object a page's `/Annots` names, except a `Popup`.
    annotations: BTreeSet<ObjectId>,
    /// Where a `/DefaultCMYK` has to be written, where one is being written.
    default_cmyk: CmykSites,
}

impl Sites {
    /// The positions, read off the document.
    fn of(document: &Document, catalog: ObjectId, wanted: &BTreeSet<Rewrite>) -> Self {
        let names = document
            .get(catalog)
            .as_dict()
            .and_then(|dict| dict.get("Names").and_then(Object::as_reference));
        let tree = Pages::new(document);
        let pages: BTreeSet<ObjectId> = (0..tree.len())
            .filter_map(|index| tree.get(index).and_then(|page| page.id))
            .collect();
        let default_cmyk = if wanted.contains(&Rewrite::DefaultCmyk) {
            CmykSites::of(document, &tree)
        } else {
            // Nothing is walked that no failed requirement asked for, which is `doc/adr/0947`'s
            // first rule applied to a *reading* rather than to a rewrite.
            CmykSites::default()
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
            default_cmyk,
        }
    }
}

/// Every annotation §12.5.2's page `/Annots` array names, except the subtype ISO 19005 exempts.
///
/// **A structural walk rather than the validator's list**, for [`CmykSites`]'s reason: a findings
/// list is capped, and a document with more annotations than the cap is one this rewrite would
/// otherwise half-finish. An annotation written directly into the array rather than as an object
/// of its own is not reached — this walk rewrites objects — and the requirement then stays failed,
/// which `doc/adr/0947`'s third stage refuses the file for.
fn annotation_objects(document: &Document, tree: &Pages<'_>) -> BTreeSet<ObjectId> {
    let mut out = BTreeSet::new();
    for index in 0..tree.len() {
        let Some(page) = tree.get(index) else {
            continue;
        };
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

/// Every dictionary a `/DefaultCMYK` has to reach, found once before the walk starts.
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
struct CmykSites {
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

/// How many objects [`CmykSites`] visits before it stops.
///
/// A bound rather than a reading (principle 3): the walk follows a resource graph whose shape is
/// the document's, and a file can nest form `XObject`s inside patterns inside forms as deep as it
/// likes. Past this, the sites that were found are still written and the output's own verdict is
/// what reports that the requirement is not met — which is the net `doc/adr/0947` built.
const MAX_RESOURCE_SITES: usize = 65_536;

impl CmykSites {
    /// The four dictionaries A003 names, reached from every page.
    fn of(document: &Document, tree: &Pages<'_>) -> Self {
        let mut found = Self::default();
        let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
        let mut queue: VecDeque<Object> = VecDeque::new();
        for index in 0..tree.len() {
            let Some(page) = tree.get(index) else {
                continue;
            };
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
    /// Not written at all.
    Dropped,
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
    /// The `/ToUnicode` `CMap` each font is to name, where any are being written.
    to_unicode: Option<&'a DerivedMaps>,
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
    /// The annotations whose `/AP` is reduced to `/N`, where any are.
    extra_appearance_states: Option<&'a ExtraAppearanceStates>,
    /// The optional content configurations losing their `/AS`, where any are.
    automatic_states: Option<&'a sites::AutomaticStates>,
    /// The `/FontFile` entry each font descriptor is to gain, where any are being embedded.
    substitutes: Option<&'a Substitutes>,
    /// The font program stream to write in place of each one being restated.
    metrics: Option<&'a Metrics>,
    /// The graphics states and images whose rendering intent is restated.
    rendering_intents: Option<&'a sites::Sites>,
    /// The dictionaries whose `/BM` array is restated.
    blend_modes: Option<&'a sites::Sites>,
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
    /// The entries the action clauses ask each object to restate, where any are asked.
    actions: Option<&'a super::actions::Removals>,
    /// The content streams whose hexadecimal strings state their final digit, where any do.
    hexadecimal: Option<&'a super::hexadecimal::Completed>,
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
        if changed {
            Rewritten::Changed(out)
        } else {
            Rewritten::Carried
        }
    }

    /// A dictionary object, rewritten where its position asks for it.
    /// Writes the `/CIDToGIDMap` a descendant `CIDFont` owes once a program is embedded in it.
    ///
    /// §9.7.4.2 makes a Type 2 `CIDFont`'s `/CIDToGIDMap` the mapping to "the glyph indices for
    /// the appropriate glyph descriptions in that font program", and Table 115 makes the entry
    /// required of such a `CIDFont` once it has one — so a descendant whose program this
    /// conversion has just written in owes an entry it did not owe before. `Identity` is the only
    /// value written, and it is what the glyphs the preparation checked were chosen under
    /// (`doc/adr/1222`).
    fn identity_cid_to_gid(&self, id: ObjectId, out: &mut Dictionary) -> bool {
        if !self.wants(Rewrite::SubstituteFontProgram)
            || !self
                .substitutes
                .is_some_and(|substitutes| substitutes.identity_cid_to_gid.contains(&id))
        {
            return false;
        }
        out.insert(
            Name::new(&b"CIDToGIDMap"[..]),
            Object::Name(Name::new(&b"Identity"[..])),
        );
        true
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
        changed |= self.preserve(id, &mut out, applied);
        changed |= self.remove_the_actions(id, &mut out);
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
        changed |= self.remove_the_annotations(id, &mut out, applied);
        changed |= self.relocate_onto_page(id, &mut out, applied);
        if self.wants(Rewrite::AnnotationFlags)
            && self.sites.annotations.contains(&id)
            // §12.5.3 makes the entry "an integer interpreted as one-bit flags", so a value that
            // is not one states no flags at all and is written over by the same rule as an absent
            // entry. Where the annotation does state flags, they are its producer's and stay:
            // the second sentence of section 6.3.2 is a row of its own and not this one.
            && self.document.get_key(&out, "F").as_integer().is_none()
        {
            out.insert(Name::new(&b"F"[..]), Object::Integer(PRINT));
            count(applied, Rewrite::AnnotationFlags);
            changed = true;
        }
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
        changed |= self.write_default_cmyk(id, &mut out, applied);
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
        changed |= self.restate_blend_mode(id, out, applied);
        changed |= self.collapse_appearance_states(id, out, applied);
        changed |= self.reduce_appearance_dictionary(id, out, applied);
        changed |= self.complete_order(id, out, applied);
        // After `complete_order`, which replaces an `/OCProperties` dictionary wholesale with the
        // one its own preparation built: this removal edits whatever `out` now holds, so the two
        // compose on a document that needs both rather than one undoing the other.
        changed |= self.remove_automatic_states(id, out, applied);
        changed |= self.attach_page_resources(id, out, applied);
        changed |= self.remove_descriptor_sets(id, out, applied);
        changed |= self.remove_page_boundaries(id, out, applied);
        changed |= self.write_cid_to_gid_map(id, out, applied);
        changed |= self.restate_truetype_encoding(id, out, applied);
        changed |= self.share_destination_profile(id, out, applied);
        changed |= self.write_colorants(id, out, applied);
        changed |= self.agree_separations(out, applied);
        changed
    }

    /// ISO 19005-2 section 6.2.4.4 and ISO 19005-4 section 6.2.4.4, inside whatever object holds
    /// the `Separation` array.
    ///
    /// Descended rather than edited at the top level, for [`Self::write_colorants`]' reason: a
    /// colour space written in a page's resource dictionary is reported at the page. Unlike that
    /// one this reaches **every** object rather than the ones a finding named, and it has to: the
    /// arrays that need rewriting are the ones that *disagree* with the chosen definition, and a
    /// finding names only the later array of each disagreeing pair.
    fn agree_separations(
        &self,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::SeparationAgreed) || self.remedies.separations.is_empty() {
            return false;
        }
        let reached = sites::agree_separations(self.document, out, &self.remedies.separations, 0);
        for _ in 0..reached.len() {
            count(applied, Rewrite::SeparationAgreed);
        }
        !reached.is_empty()
    }

    /// ISO 19005-2 section 6.2.4.4 and ISO 19005-4 section 6.2.4.4, inside the object the
    /// validator named.
    ///
    /// `doc/pdf-a-mitigations.md` section 13.3.1's lesson: a colour space is reported at the
    /// object it is *written in*, which for a space defined in a page's resource dictionary is
    /// the page. So the object is descended rather than edited at its top level, and
    /// [`super::sites::place_colorants`] is the same placement the preparation already proved
    /// reaches every colourant it promised.
    fn write_colorants(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::SpotColorantEntry) {
            return false;
        }
        let Some(entries) = self.colorants.and_then(|written| written.at.get(&id)) else {
            return false;
        };
        let placed = sites::place_colorants(self.document, out, entries, 0);
        for _ in 0..placed.len() {
            count(applied, Rewrite::SpotColorantEntry);
        }
        !placed.is_empty()
    }

    /// ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3, at one output intent object.
    ///
    /// The entries written directly inside the catalog's array are corrected by
    /// [`Self::rewrite_catalog`] instead, because there is no object of their own to reach.
    fn share_destination_profile(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::SharedDestinationProfile) {
            return false;
        }
        let Some(shared) = self.shared_profile else {
            return false;
        };
        if !shared.at.contains(&id) {
            return false;
        }
        out.insert(
            Name::new(&b"DestOutputProfile"[..]),
            Object::Reference(shared.profile),
        );
        count(applied, Rewrite::SharedDestinationProfile);
        true
    }

    /// ISO 19005-2 section 6.2.11.6 and ISO 19005-4 section 6.2.10.6's two font-dictionary rows,
    /// written at the fonts [`super::sites`] proved the change safe for.
    ///
    /// **Nothing is decided here.** Which fonts are symbolic is the validator's reading of the
    /// descriptor's flags, and whether the new value leaves every code on the glyph it already
    /// reached was settled by the preparation — a font that failed that comparison never reaches
    /// this walk, because the requirement is refused and no file is written.
    ///
    /// The value written for a non-symbolic font is the producer's own encoding dictionary where
    /// they wrote one, with its `/BaseEncoding` restated. An encoding dictionary the file held
    /// indirectly is written directly into this font, which changes the object graph and not one
    /// name a code resolves through: another font sharing that object goes on sharing it, and
    /// this one now states the same entries with one name corrected.
    fn restate_truetype_encoding(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if self.wants(Rewrite::SymbolicTrueTypeEncodingRemoved)
            && self
                .symbolic_encodings
                .is_some_and(|sites| sites.at.contains(&id))
            && out.remove("Encoding").is_some()
        {
            count(applied, Rewrite::SymbolicTrueTypeEncodingRemoved);
            changed = true;
        }
        if self.wants(Rewrite::StandardTrueTypeEncoding)
            && let Some(value) = self
                .standard_encodings
                .and_then(|encodings| encodings.at.get(&id))
        {
            out.insert(Name::new(&b"Encoding"[..]), value.clone());
            count(applied, Rewrite::StandardTrueTypeEncoding);
            changed = true;
        }
        changed
    }

    /// §8.6.5.8's answer, written into every entry inside this object that stated a name it does
    /// not define.
    ///
    /// **Inside the object rather than on its top-level dictionary**, and that is not a detail:
    /// `pdf_archive` reports a dictionary at the object it is written in, so a resource
    /// dictionary a page states directly carries its graphics states at the page's own object
    /// number. An edit that looked only at the dictionary the number names would fix nothing
    /// there and leave the requirement failed.
    ///
    /// The two keys are guarded differently, because only one of them is unambiguous. `/RI` is a
    /// graphics state parameter and §8.4.5's Table 57 gives it no other home, so any value
    /// outside §8.6.5.8's four names is the one this requirement is about. `/Intent` is an image
    /// `XObject`'s in §8.9.5.1's Table 87 **and** an optional content group's in §8.11.2.3,
    /// where its values are `View` and `Design` — so it is restated only where the dictionary
    /// states `/Subtype /Image`, which is the validator's own test.
    fn restate_rendering_intent(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::RenderingIntent)
            || !self
                .rendering_intents
                .is_some_and(|sites| sites.at.contains(&id))
        {
            return false;
        }
        let mut places = 0usize;
        self.restate_inside(out, Entry::RenderingIntent, 0, &mut places);
        for _ in 0..places {
            count(applied, Rewrite::RenderingIntent);
        }
        places > 0
    }

    /// §8.4.1's Table 57 answer, written wherever inside this object the entry was an array of
    /// names it does not define.
    ///
    /// [`Self::restate_rendering_intent`]'s reason for descending, and `/BM` needs no guard of
    /// its own: §8.4.1's Table 57 is the only place the base standard gives the key, and an array
    /// with no recognised name in it means `Normal` to a reader wherever it is written.
    fn restate_blend_mode(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::BlendModeNormal)
            || !self.blend_modes.is_some_and(|sites| sites.at.contains(&id))
        {
            return false;
        }
        let mut places = 0usize;
        self.restate_inside(out, Entry::BlendMode, 0, &mut places);
        for _ in 0..places {
            count(applied, Rewrite::BlendModeNormal);
        }
        places > 0
    }

    /// One dictionary and everything written inside it, with `entry` restated where it is wrong.
    ///
    /// References are **not** followed, for the reason `pdf_archive`'s own walk does not follow
    /// them: a dictionary written as its own object is reported at that object and is reached by
    /// this rewrite there.
    fn restate_inside(
        &self,
        dict: &mut Dictionary,
        entry: Entry,
        depth: usize,
        places: &mut usize,
    ) {
        if depth >= sites::MAX_ENTRY_DEPTH {
            return;
        }
        if let Some((key, value)) = self.restated_entry(dict, entry) {
            dict.insert(Name::new(key.as_bytes()), value);
            *places = places.saturating_add(1);
        }
        let deeper = depth.saturating_add(1);
        let inside: Vec<(Name, Object)> = dict
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        for (key, value) in inside {
            let mut held = value;
            if self.restate_value(&mut held, entry, deeper, places) {
                dict.insert(key, held);
            }
        }
    }

    /// One value inside a dictionary, and whether the restating changed it.
    fn restate_value(
        &self,
        value: &mut Object,
        entry: Entry,
        depth: usize,
        places: &mut usize,
    ) -> bool {
        let before = *places;
        match value {
            Object::Dictionary(dict) => self.restate_inside(dict, entry, depth, places),
            Object::Array(items) => {
                for item in items {
                    self.restate_value(item, entry, depth.saturating_add(1), places);
                }
            }
            _ => {}
        }
        *places > before
    }

    /// The entry this dictionary states wrongly, and what it is to state instead.
    fn restated_entry(&self, dict: &Dictionary, entry: Entry) -> Option<(&'static str, Object)> {
        match entry {
            Entry::RenderingIntent => {
                let key = if dict.get("RI").is_some() {
                    "RI"
                } else if dict.get("Intent").is_some() && self.subtype_is(dict, b"Image") {
                    "Intent"
                } else {
                    return None;
                };
                let stated = self.document.get_key(dict, key);
                let defined = stated
                    .as_name()
                    .is_some_and(|name| RENDERING_INTENTS.contains(&name.as_bytes()));
                (!defined).then(|| (key, Object::Name(Name::new(RELATIVE_COLORIMETRIC))))
            }
            Entry::BlendMode => {
                dict.get("BM")?;
                let Object::Array(names) = self.document.get_key(dict, "BM") else {
                    return None;
                };
                names
                    .iter()
                    .all(|name| !sites::is_blend_mode(&self.document.resolve(name)))
                    .then(|| ("BM", Object::Name(Name::new(&b"Normal"[..]))))
            }
        }
    }

    /// The one appearance stream an annotation's `/AS` selects, written as its `/N`.
    ///
    /// The `/AP` dictionary is written **direct** whatever the source stated it as, for the
    /// reason [`Rewrite::OutputIntent`]'s array is: the value is the annotation's own and this
    /// walk rewrites objects, so an `/AP` some other object held is copied here and left behind
    /// where nothing else refers to it. Every other key of it crosses unchanged — an `/R` or a
    /// `/D` beside the normal appearance is a different requirement's business.
    fn collapse_appearance_states(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(stream) = self
            .appearance_states
            .filter(|_| self.wants(Rewrite::NormalAppearanceFromState))
            .and_then(|states| states.at.get(&id))
        else {
            return false;
        };
        let Some(mut appearance) = self.document.get_key(out, "AP").as_dict().cloned() else {
            return false;
        };
        appearance.insert(Name::new(&b"N"[..]), Object::Reference(*stream));
        out.insert(Name::new(&b"AP"[..]), Object::Dictionary(appearance));
        count(applied, Rewrite::NormalAppearanceFromState);
        true
    }

    /// One annotation's appearance dictionary, reduced to the normal appearance alone.
    ///
    /// ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 admit `/N` and no other key, and
    /// §12.5.5's Table 170 makes the normal appearance the default of both `/R` and `/D` — so
    /// what a reader draws in those states after this is what it already draws for an annotation
    /// whose producer stated neither. The artwork is what goes, which is why the decision carries
    /// a loss (`doc/adr/1234`).
    ///
    /// The `/AP` is written **direct** whatever the source stated it as, on
    /// [`Self::collapse_appearance_states`]' reading and for its reason.
    fn reduce_appearance_dictionary(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let reduced = self
            .extra_appearance_states
            .filter(|_| self.wants(Rewrite::ExtraAppearanceStatesRemoved));
        if reduced.is_none_or(|states| !states.at.contains(&id)) {
            return false;
        }
        let Some(appearance) = self.document.get_key(out, "AP").as_dict().cloned() else {
            return false;
        };
        let Some(normal) = appearance.get("N").cloned() else {
            return false;
        };
        let mut only_normal = Dictionary::new();
        only_normal.insert(Name::new(&b"N"[..]), normal);
        out.insert(Name::new(&b"AP"[..]), Object::Dictionary(only_normal));
        count(applied, Rewrite::ExtraAppearanceStatesRemoved);
        true
    }

    /// The `/AS` entry ISO 19005-2 section 6.9 forbids, gone from wherever this object holds one.
    ///
    /// Three shapes, because §8.11.4.3's configuration dictionaries are written at three places
    /// and `super::sites` found which: the configuration that is this object, and the
    /// `/OCProperties` object holding one directly. The third — an `/OCProperties` the catalog
    /// states directly — is [`Self::rewrite_catalog`]'s, since the catalog is what holds it.
    fn remove_automatic_states(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(states) = self
            .automatic_states
            .filter(|_| self.wants(Rewrite::AutomaticStatesRemoved))
        else {
            return false;
        };
        if states.at.contains(&id) && out.remove("AS").is_some() {
            count(applied, Rewrite::AutomaticStatesRemoved);
            return true;
        }
        if states.properties == Some(id) {
            return Self::strip_automatic_states(out, applied);
        }
        false
    }

    /// The `/AS` entries of an `/OCProperties` dictionary the catalog states directly.
    ///
    /// The third of [`Self::remove_automatic_states`]' three places, here rather than there
    /// because the catalog is the object that holds it. Run after the `/Order` completion, which
    /// may have just written this dictionary: the removal edits what the catalog now states
    /// rather than what the source did.
    fn remove_the_catalogs_automatic_states(
        &self,
        catalog: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::AutomaticStatesRemoved)
            || !self
                .automatic_states
                .is_some_and(|states| states.in_catalog)
        {
            return false;
        }
        let Some(Object::Dictionary(properties)) = catalog.get("OCProperties") else {
            return false;
        };
        let mut properties = properties.clone();
        if !Self::strip_automatic_states(&mut properties, applied) {
            return false;
        }
        catalog.insert(
            Name::new(&b"OCProperties"[..]),
            Object::Dictionary(properties),
        );
        true
    }

    /// Every `/AS` written directly inside one `/OCProperties` dictionary, gone.
    ///
    /// §8.11.4.1's Table 98 gives the dictionary a `/D` holding one configuration and a
    /// `/Configs` holding an array of them, and a configuration reached by reference is edited in
    /// its own object instead — so what this touches is the ones written in place.
    fn strip_automatic_states(
        properties: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if let Some(Object::Dictionary(default)) = properties.get("D") {
            let mut default = default.clone();
            if default.remove("AS").is_some() {
                properties.insert(Name::new(&b"D"[..]), Object::Dictionary(default));
                count(applied, Rewrite::AutomaticStatesRemoved);
                changed = true;
            }
        }
        if let Some(Object::Array(items)) = properties.get("Configs") {
            let mut rebuilt = Vec::with_capacity(items.len());
            let mut any = false;
            for item in items {
                match item {
                    Object::Dictionary(configuration) => {
                        let mut configuration = configuration.clone();
                        if configuration.remove("AS").is_some() {
                            count(applied, Rewrite::AutomaticStatesRemoved);
                            any = true;
                        }
                        rebuilt.push(Object::Dictionary(configuration));
                    }
                    other => rebuilt.push(other.clone()),
                }
            }
            if any {
                properties.insert(Name::new(&b"Configs"[..]), Object::Array(rebuilt));
                changed = true;
            }
        }
        changed
    }

    /// An optional content configuration's completed `/Order`, or the `/OCProperties` object
    /// holding configurations written directly inside it.
    fn complete_order(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(orders) = self
            .orders
            .filter(|_| self.wants(Rewrite::OptionalContentOrder))
        else {
            return false;
        };
        if let Some(order) = orders.at.get(&id) {
            out.insert(Name::new(&b"Order"[..]), Object::Array(order.clone()));
            count(applied, Rewrite::OptionalContentOrder);
            return true;
        }
        if let Some((at, properties)) = &orders.properties
            && *at == id
        {
            *out = properties.clone();
            count(applied, Rewrite::OptionalContentOrder);
            return true;
        }
        false
    }

    /// §7.7.3.4's inherited resources, copied down onto the page that already resolves through
    /// them.
    ///
    /// **Never over an entry that is there.** The population is a page stating none, and a page
    /// the `/DefaultCMYK` rewrite has already given one is a page this rewrite has nothing left
    /// to do for — the value the two write is the same value, read from the same tree.
    fn attach_page_resources(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::PageResources) || out.get("Resources").is_some() {
            return false;
        }
        let Some(resources) = self
            .page_resources
            .and_then(|pages| pages.at.get(&id))
            .cloned()
        else {
            return false;
        };
        out.insert(Name::new(&b"Resources"[..]), resources);
        count(applied, Rewrite::PageResources);
        true
    }

    /// The out-of-range optional page boundary entries one page loses.
    ///
    /// ISO 19005-2 section 6.1.13's limit, answered by ISO 32000-2 §7.7.3.3's Table 31 and
    /// §14.11.2.1's defaults: an entry Table 31 marks optional and whose default is another box
    /// in the same file is removable, and the preparation has already decided — per entry, on
    /// §14.11.2.1's intersection sentence — whether the region a reader computes changes with it
    /// (`doc/adr/1210`). Nothing is decided here.
    fn remove_page_boundaries(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(keys) = self
            .boundaries
            .filter(|_| self.wants(Rewrite::PageBoundaryRemoved))
            .and_then(|removals| removals.at.get(&id))
        else {
            return false;
        };
        let mut changed = false;
        for key in keys {
            if out.remove(key).is_some() {
                count(applied, Rewrite::PageBoundaryRemoved);
                changed = true;
            }
        }
        changed
    }

    /// A font descriptor's incomplete `/CharSet` or `/CIDSet`, removed.
    fn remove_descriptor_sets(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(keys) = self
            .descriptor_sets
            .filter(|_| self.wants(Rewrite::DescriptorSetRemoved))
            .and_then(|sets| sets.at.get(&id))
        else {
            return false;
        };
        let mut changed = false;
        for key in keys {
            if out.remove(key).is_some() {
                count(applied, Rewrite::DescriptorSetRemoved);
                changed = true;
            }
        }
        changed
    }

    /// ISO 32000-1's default for `/CIDToGIDMap`, written where a part 2 target asks for the
    /// entry.
    fn write_cid_to_gid_map(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::CidToGidIdentity)
            || !self.cid_to_gid.is_some_and(|sites| sites.at.contains(&id))
            || !self.document.get_key(out, "CIDToGIDMap").is_null()
        {
            return false;
        }
        out.insert(
            Name::new(&b"CIDToGIDMap"[..]),
            Object::Name(Name::new(&b"Identity"[..])),
        );
        count(applied, Rewrite::CidToGidIdentity);
        true
    }

    /// An embedded file's specification: §7.11.3's `/F` and `/UF`, and its `/AFRelationship`.
    ///
    /// **The population is the `/EF` key**, which is what makes a file specification one that
    /// *carries* an embedded file — the population both ISO 19005-2 section 6.8 and ISO 19005-4
    /// section 6.9 state their rules over. A specification the document holds as a direct
    /// dictionary inside its `/EmbeddedFiles` name tree is not reached, because this walk
    /// rewrites objects rather than values inside them; the requirement then stays failed and
    /// `doc/adr/0947`'s third stage refuses the file rather than writing a half-corrected one.
    fn complete_file_specification(
        &self,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if out.get("EF").is_none() {
            return false;
        }
        let mut changed = false;
        if self.wants(Rewrite::AssociatedFileNames)
            && let Some(pair) = self.file_names(out)
        {
            out.insert(Name::new(&b"F"[..]), Object::String(pair.clone().into()));
            out.insert(Name::new(&b"UF"[..]), Object::String(pair.into()));
            count(applied, Rewrite::AssociatedFileNames);
            changed = true;
        }
        if self.wants(Rewrite::AssociatedFileRelationship)
            && self.document.get_key(out, "AFRelationship").is_null()
        {
            out.insert(
                Name::new(&b"AFRelationship"[..]),
                Object::Name(Name::new(&b"Unspecified"[..])),
            );
            count(applied, Rewrite::AssociatedFileRelationship);
            changed = true;
        }
        changed
    }

    /// The one ASCII file name a specification missing `/F` or `/UF` states in the other key.
    ///
    /// `None` where both keys are already there, where neither is, or where the one that is
    /// holds a byte outside ASCII — the three cases [`Rewrite::AssociatedFileNames`] does not
    /// write, and the last of them is the one that matters: outside ASCII the byte string's own
    /// character set is not something this file states.
    fn file_names(&self, out: &Dictionary) -> Option<Vec<u8>> {
        let stated = |key: &str| {
            self.document
                .get_key(out, key)
                .as_string()
                .map(<[u8]>::to_vec)
        };
        let (present, absent) = (stated("F"), stated("UF"));
        if present.is_some() == absent.is_some() {
            return None;
        }
        let name = present.or(absent)?;
        name.is_ascii().then_some(name)
    }

    /// §8.6.5.6's `/DefaultCMYK`, written wherever [`CmykSites`] found a place for it.
    ///
    /// Four places, and they are one rule read through four shapes a document can take: the
    /// `/ColorSpace` subdictionary itself where it is an object, the resource dictionary where
    /// that subdictionary is direct or absent, the holder where the resource dictionary itself is
    /// direct, and a page that has no resource dictionary of its own at all.
    ///
    /// **An entry the producer already wrote is never replaced.** A `/DefaultCMYK` in the file is
    /// the producer saying how their `DeviceCMYK` is to be read, and overwriting it would be
    /// changing what the file says rather than adding what ISO 19005 requires it to say. Where
    /// such an entry is one the clause does not license, the requirement stays failed and
    /// `doc/adr/0947`'s third stage refuses to write the file.
    fn write_default_cmyk(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(default) = self
            .default_cmyk
            .filter(|_| self.wants(Rewrite::DefaultCmyk))
        else {
            return false;
        };
        let sites = &self.sites.default_cmyk;
        let written = if sites.spaces.contains(&id) {
            with_default_cmyk(out, default.space).map(|spaces| {
                *out = spaces;
            })
        } else if sites.resources.contains(&id) {
            resources_with_default_cmyk(out, default.space).map(|resources| {
                *out = resources;
            })
        } else {
            // A page that states none of its own is given one first, and the entry then goes
            // into that dictionary by the same rule as any other direct one.
            if let Some(resources) = sites.pages.get(&id) {
                out.insert(Name::new(&b"Resources"[..]), resources.clone());
            }
            sites
                .direct
                .contains(&id)
                .then(|| out.get("Resources").and_then(Object::as_dict).cloned())
                .flatten()
                .and_then(|resources| resources_with_default_cmyk(&resources, default.space))
                .map(|resources| {
                    out.insert(Name::new(&b"Resources"[..]), Object::Dictionary(resources));
                })
        };
        if written.is_some() {
            count(applied, Rewrite::DefaultCmyk);
            return true;
        }
        // A page given an inherited dictionary whose own `/ColorSpace` is another object has
        // still been changed: the entry itself goes into that object.
        sites.pages.contains_key(&id)
    }

    /// The catalog: §7.7.2's `/Requirements`, `/Version` and a direct `/Names`.
    /// §12.4.2's labels, where the catalog states the number tree inline rather than by reference.
    ///
    /// The extended tree is the preparation's whichever of the two shapes the document used, and
    /// [`Self::preserve`] writes the other.
    fn state_the_labels_inline(&self, catalog: &mut Dictionary) -> bool {
        if !self.wants(Rewrite::PreservedAsPage) {
            return false;
        }
        let Some(composed) = self.preserved else {
            return false;
        };
        let Some((None, labels)) = composed.labels.as_ref() else {
            return false;
        };
        catalog.insert(
            Name::new(&b"PageLabels"[..]),
            Object::Dictionary(labels.clone()),
        );
        true
    }

    /// The dictionaries an appended page changes: the page tree's root, the label tree, the
    /// structure tree.
    ///
    /// `doc/adr/0954` is why the second is here at all — a page count that changes leaves
    /// §12.4.2's labels describing a document that no longer exists, and neither the label nor the
    /// page is a conformance requirement, so both are owed for the reader rather than for the
    /// validator. The third is a requirement: §14.8.2.2.1 makes content the structure tree does not
    /// reach an artifact, and the preserved content is not one (`doc/adr/1163`).
    ///
    /// **None of the three is counted.** `Rewrite::PreservedAsPage`'s count is the pages appended,
    /// which is what the report's "N place(s)" means here; the entries those pages owe the document
    /// they land in are part of appending them rather than places of their own.
    fn preserve(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::PreservedAsPage) {
            return false;
        }
        let Some(composed) = self.preserved else {
            return false;
        };
        let mut changed = false;
        if Some(id) == self.sites.root_of_the_pages && !composed.pages.is_empty() {
            changed |= self.append_the_pages(out, composed, applied);
        }
        if let Some((Some(at), labels)) = composed.labels.as_ref()
            && *at == id
        {
            *out = labels.clone();
            changed = true;
        }
        if let Some(structure) = composed.structure.as_ref()
            && !composed.pages.is_empty()
        {
            changed |= structure.apply(self.document, id, out);
        }
        changed
    }

    /// Puts a forbidden annotation's marks back onto the producer's own page, where §12.5.5 had them.
    ///
    /// `doc/adr/1123`, the construction `doc/adr/1120`'s amendment put in scope. The two edits are
    /// composed in [`super::preserve::relocate_a_page`]: the page's `/Contents` becomes the array
    /// with the `q`-prepend and the closing stream around the producer's own streams, and its
    /// `/Resources` gains the appearance under `/XObject`. Neither is a mark — what draws is
    /// §12.5.5's own placement of the producer's own stream. The producer's `/Contents` streams are
    /// referenced, never rewritten, so not a byte of the marks changes.
    fn relocate_onto_page(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(composed) = self.preserved else {
            return false;
        };
        let Some(relocated) = composed.relocations.get(&id) else {
            return false;
        };
        out.insert(Name::new(&b"Contents"[..]), relocated.contents.clone());
        out.insert(Name::new(&b"Resources"[..]), relocated.resources.clone());
        count(applied, Rewrite::RelocatedOnPage);
        true
    }

    /// Restates the entries ISO 19005's action clauses ask of this object.
    ///
    /// [`super::actions`] worked out what each holder, each `/AA` and each surviving action is to
    /// state; this puts it in the object. **Each edit names the rewrite that asked for it**, so a
    /// conversion whose caller departed from one of the three rows carries out the other two and
    /// leaves that one's entries as the producer wrote them.
    fn remove_the_actions(&self, id: ObjectId, out: &mut Dictionary) -> bool {
        let Some(edits) = self.actions.and_then(|removals| removals.edits.get(&id)) else {
            return false;
        };
        let mut changed = false;
        for edit in edits {
            if !self.wants(edit.by()) {
                continue;
            }
            match edit {
                super::actions::Edit::Entry { key, value, .. } => match value {
                    None => changed |= out.remove(key).is_some(),
                    Some(value) => {
                        if out.get(key) != Some(value) {
                            out.insert(Name::new(key.as_bytes()), value.clone());
                            changed = true;
                        }
                    }
                },
                super::actions::Edit::Whole { dict, .. } => {
                    if out != dict {
                        out.clone_from(dict);
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    /// The two removals ISO 19005's annotation clauses ask of one page's `/Annots` array.
    ///
    /// Section 6.3.1's subtype rule and section 6.3.2's flag rule are two authorisations and two
    /// counts, and they are one pass because each reads the array the other left: a page that
    /// loses its last annotation to either loses the array itself, and a page whose annotations
    /// both rules name loses them together.
    fn remove_the_annotations(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.sites.pages.contains(&id) {
            return false;
        }
        let mut changed = self.remove_the_forbidden_annotations(out, applied);
        changed |= self.remove_the_hidden_annotations(out, applied);
        changed
    }

    /// Takes every annotation the target's part does not admit out of one page's `/Annots`.
    ///
    /// ISO 19005-2 section 6.3.1 and ISO 19005-4 section 6.3.1 forbid the subtype and offer
    /// nothing to put in its place, so the reference is what goes. Nothing hunts for the
    /// annotation object: the walk copies what the converted document reaches, so an annotation
    /// no array names is an object the output does not hold — and its media stream, its artwork
    /// and its popup go with it without this rewrite naming any of them.
    ///
    /// **An array left empty is removed rather than written empty.** §7.7.3.3's Table 31 makes
    /// `/Annots` optional, so a page whose every annotation the target forbids ends up saying
    /// what it now means — this page has no annotations — rather than stating an empty array.
    fn remove_the_forbidden_annotations(
        &self,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(forbidden) = self
            .forbidden_annotations
            .filter(|_| self.wants(Rewrite::ForbiddenAnnotationRemoved))
        else {
            return false;
        };
        self.take_out_of_the_page(
            out,
            applied,
            &forbidden.at,
            Rewrite::ForbiddenAnnotationRemoved,
        )
    }

    /// Takes every annotation whose stated `/F` ISO 19005 forbids out of one page's `/Annots`.
    ///
    /// ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2's second sentence, by the same act
    /// the subtype rows are answered with and for the reason `doc/adr/1234` states: the clause
    /// offers nothing to write in place of the flags, and the other future — writing the ones it
    /// asks for — shows a mark on a page its producer kept it off.
    fn remove_the_hidden_annotations(
        &self,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(hidden) = self
            .hidden_annotations
            .filter(|_| self.wants(Rewrite::HiddenAnnotationRemoved))
        else {
            return false;
        };
        self.take_out_of_the_page(out, applied, &hidden.at, Rewrite::HiddenAnnotationRemoved)
    }

    /// One page's `/Annots` without the annotations named, counted against the rewrite that asked.
    ///
    /// **An array left empty is removed rather than written empty.** §7.7.3.3's Table 31 makes
    /// `/Annots` optional, so a page whose every annotation goes ends up saying what it now means
    /// — this page has no annotations — rather than stating an empty array.
    fn take_out_of_the_page(
        &self,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
        at: &BTreeSet<ObjectId>,
        rewrite: Rewrite,
    ) -> bool {
        let Some(listed) = self
            .document
            .get_key(out, "Annots")
            .as_array()
            .map(<[Object]>::to_vec)
        else {
            return false;
        };
        let kept: Vec<Object> = listed
            .iter()
            .filter(|entry| entry.as_reference().is_none_or(|id| !at.contains(&id)))
            .cloned()
            .collect();
        let removed = listed.len().saturating_sub(kept.len());
        if removed == 0 {
            return false;
        }
        for _ in 0..removed {
            count(applied, rewrite);
        }
        if kept.is_empty() {
            out.remove("Annots");
        } else {
            out.insert(Name::new(&b"Annots"[..]), Object::Array(kept));
        }
        true
    }

    /// Adds the composed pages to the page tree's root node.
    ///
    /// §7.7.3.2's Table 30 is the whole of what this has to get right, and it makes the two edits
    /// one each. `/Kids`:
    ///
    /// > An array of indirect references to the immediate children of this node. The children
    /// > shall only be page objects or other page tree nodes.
    ///
    /// A composed page is one of the two things a kid may be, so it is added to the root node
    /// without any other node in the tree knowing. And `/Count`:
    ///
    /// > The number of leaf nodes (page objects) that are descendants of this node within the page
    /// > tree.
    ///
    /// The root is an ancestor of every page, so it is the only node whose count changes. The
    /// producer's own number is added to rather than recomputed: a file whose count was wrong
    /// before this conversion is wrong in the same way afterwards, and correcting it is a change
    /// no failed requirement asked for.
    fn append_the_pages(
        &self,
        node: &mut Dictionary,
        composed: &Composed,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(mut kids) = self
            .document
            .get_key(node, "Kids")
            .as_array()
            .map(<[Object]>::to_vec)
        else {
            return false;
        };
        for page in &composed.pages {
            kids.push(Object::Reference(*page));
            count(applied, Rewrite::PreservedAsPage);
        }
        let was = self
            .document
            .get_key(node, "Count")
            .as_integer()
            .unwrap_or(0);
        let gained = i64::try_from(composed.pages.len()).unwrap_or(0);
        node.insert(Name::new(&b"Kids"[..]), Object::Array(kids));
        node.insert(
            Name::new(&b"Count"[..]),
            Object::Integer(was.saturating_add(gained)),
        );
        true
    }

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
        if self.wants(Rewrite::NeedsRendering) && catalog.remove("NeedsRendering").is_some() {
            count(applied, Rewrite::NeedsRendering);
            changed = true;
        }
        changed |= self.state_the_labels_inline(catalog);
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
        // ISO 19005 section 6.2.3's array, which two rewrites write: one corrects the entries
        // the source holds and one appends a PDF/A entry, and a document wanting both gets one
        // array with both done to it rather than two writers overwriting each other.
        let corrected = self
            .wants(Rewrite::SharedDestinationProfile)
            .then(|| {
                self.shared_profile
                    .and_then(|shared| shared.entries.clone())
            })
            .flatten();
        if let Some(entries) = &corrected
            && !self.wants(Rewrite::OutputIntent)
        {
            catalog.insert(
                Name::new(&b"OutputIntents"[..]),
                Object::Array(entries.clone()),
            );
            count(applied, Rewrite::SharedDestinationProfile);
            changed = true;
        }
        if self.wants(Rewrite::OutputIntent)
            && let Some(intent) = self.intent
        {
            // Written as a direct array whatever the source stated it as: an array object the
            // source held indirectly is not carried, because nothing in the rewritten catalog
            // refers to it any more. Its entries are — a reference among them is renumbered like
            // any other reference this verb rewrites.
            let mut entries = corrected
                .clone()
                .unwrap_or_else(|| output_intent_entries(self.document, catalog));
            if corrected.is_some() {
                count(applied, Rewrite::SharedDestinationProfile);
            }
            entries.push(Object::Dictionary(intent_dictionary(intent)));
            catalog.insert(Name::new(&b"OutputIntents"[..]), Object::Array(entries));
            changed = true;
        }
        if self.wants_metadata()
            && let Some(metadata) = self.metadata
            && metadata.written.is_some()
        {
            catalog.insert(Name::new(&b"Metadata"[..]), Object::Reference(metadata.at));
            changed = true;
        }
        if self.wants(Rewrite::MarkInfo) {
            // ISO 19005-2 section 6.7.2.2 asks for one entry in one dictionary, and the entry
            // is written into whatever `/MarkInfo` the producer stated rather than over it: a
            // `/UserProperties` or `/Suspects` beside it is theirs and says nothing this
            // requirement is about.
            let mut mark_info = match self.document.get_key(catalog, "MarkInfo") {
                Object::Dictionary(stated) => stated,
                _ => Dictionary::new(),
            };
            mark_info.insert(Name::new(&b"Marked"[..]), Object::Boolean(true));
            catalog.insert(Name::new(&b"MarkInfo"[..]), Object::Dictionary(mark_info));
            count(applied, Rewrite::MarkInfo);
            changed = true;
        }
        if self.wants(Rewrite::OptionalContentOrder)
            && let Some(orders) = self.orders
            && let Some(properties) = &orders.in_catalog
        {
            // The catalog states `/OCProperties` directly, so the completed configurations go
            // back into the catalog rather than into an object of their own.
            catalog.insert(
                Name::new(&b"OCProperties"[..]),
                Object::Dictionary(properties.clone()),
            );
            count(applied, Rewrite::OptionalContentOrder);
            changed = true;
        }
        changed |= self.remove_the_catalogs_automatic_states(catalog, applied);
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
        changed |= self.edit_catalog_dictionaries(catalog, applied);
        changed
    }

    /// §12.8.6's permissions dictionary and §12.7.3's interactive form dictionary, where the
    /// catalog writes either directly: the same edits their own objects would take.
    fn edit_catalog_dictionaries(
        &self,
        catalog: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if self.permissions_site() == Some(Site::InCatalog)
            && let Some(Object::Dictionary(permissions)) = catalog.get("Perms")
        {
            let mut permissions = permissions.clone();
            if self.edit_permissions(&mut permissions, applied) {
                catalog.insert(Name::new(&b"Perms"[..]), Object::Dictionary(permissions));
                changed = true;
            }
        }
        if self
            .signatures
            .filter(|_| self.wants(Rewrite::SignatureValueRemoved))
            .and_then(|signatures| signatures.form)
            == Some(Site::InCatalog)
            && let Some(Object::Dictionary(form)) = catalog.get("AcroForm")
        {
            let mut form = form.clone();
            if clear_append_only(&mut form, applied) {
                catalog.insert(Name::new(&b"AcroForm"[..]), Object::Dictionary(form));
                changed = true;
            }
        }
        changed
    }

    /// `doc/pdf-a-conversion-limits.md` section 3.6, at one signature field or widget: the
    /// value goes, and nothing else in the dictionary is touched.
    fn unsign(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(signatures) = self
            .signatures
            .filter(|_| self.wants(Rewrite::SignatureValueRemoved))
        else {
            return false;
        };
        if !signatures.values_at.contains(&id) || out.remove("V").is_none() {
            return false;
        }
        count(applied, Rewrite::SignatureValueRemoved);
        true
    }

    /// Where the permissions dictionary is, if either rewrite that edits it is wanted.
    fn permissions_site(&self) -> Option<Site> {
        let signed = self
            .signatures
            .filter(|_| self.wants(Rewrite::SignatureValueRemoved))
            .and_then(|signatures| signatures.permissions);
        let foreign = self
            .foreign_handlers
            .filter(|_| self.wants(Rewrite::ForeignPermissionHandlers))
            .map(|handlers| handlers.at);
        signed.or(foreign)
    }

    /// Both edits to §12.8.6's permissions dictionary, at whichever site holds it.
    ///
    /// The two are one function because they are one dictionary: a document wanting both gets
    /// one edited dictionary rather than two writers overwriting each other. The `DocMDP` and
    /// `UR3` entries go under [`Rewrite::SignatureValueRemoved`] — Table 263 makes each a
    /// signature — and every other key under [`Rewrite::ForeignPermissionHandlers`]. What is
    /// left may be empty, and stays: Table 263 makes every entry optional, so an empty
    /// permissions dictionary states nothing and is not a shape the conversion has to answer for.
    fn edit_permissions(
        &self,
        permissions: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if self
            .signatures
            .filter(|_| self.wants(Rewrite::SignatureValueRemoved))
            .is_some_and(|signatures| signatures.permissions.is_some())
        {
            for key in ["DocMDP", "UR3"] {
                if permissions.remove(key).is_some() {
                    count(applied, Rewrite::SignatureValueRemoved);
                    changed = true;
                }
            }
        }
        if let Some(handlers) = self
            .foreign_handlers
            .filter(|_| self.wants(Rewrite::ForeignPermissionHandlers))
        {
            for key in &handlers.keys {
                if permissions.remove(key).is_some() {
                    count(applied, Rewrite::ForeignPermissionHandlers);
                    changed = true;
                }
            }
        }
        changed
    }

    /// A stream object: the `XObject` rules, and the filter chain.
    #[expect(
        clippy::too_many_lines,
        reason = "one branch per rewrite that replaces a whole stream, each with the two or                   three sentences saying why its bytes and its dictionary change together, and                   the branches are tried in an order a reader has to be able to see. Splitting                   them across functions would hide that order behind calls"
    )]
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
        // The attachment a tool derived: a whole new stream rather than a dictionary edit, because
        // its bytes, its `/Length`, its `/Subtype` and its `/Params` all change together. The
        // construction and the four edits are `super::remedies::derived_stream`.
        if self.wants(Rewrite::DerivedEmbeddedFile)
            && let Some(derived) = self.remedies.derived.get(&id)
        {
            count(applied, Rewrite::DerivedEmbeddedFile);
            return Rewritten::Changed(derived.clone());
        }
        // The stream whose data was outside the file: its bytes, its `/Filter`, its
        // `/DecodeParms` and its `/Length` all change together, so a whole stream crosses rather
        // than a dictionary edit. `super::external`'s construction is Table 5 read straight.
        if self.wants(Rewrite::ExternalDataEmbedded)
            && let Some(embedded) = self.external_data.and_then(|external| external.at.get(&id))
        {
            count(applied, Rewrite::ExternalDataEmbedded);
            return Rewritten::Changed(embedded.clone());
        }
        // The same shape, for the same reason: the repaired bytes, their `/Filter` and their
        // `/Length` change together, so what crosses is a stream rather than a dictionary edit.
        if let Some(repaired) = self.hexadecimal_repair(id) {
            count(applied, Rewrite::HexadecimalDigitCompleted);
            return Rewritten::Changed(repaired);
        }
        // **Four writers over one packet, and the last of them is what counts the others'
        // work.** `super::prepare` folds the header cut into the bytes the property removal
        // starts from, those into the bytes the container respelling starts from, and those into
        // the bytes the identification schema is restated into — so whichever of the branches
        // below writes this stream is writing every edit, and a rewrite that happened has to be
        // counted wherever it is carried, not only where it is the sole reason the stream
        // changed.
        let header_cut = self.wants(Rewrite::PacketHeaderAttributes)
            && self
                .packet_headers
                .is_some_and(|headers| headers.packet(id).is_some());
        // The producer's own packet, with the identification schema's properties cut out of it
        // and this target's put in — every other byte of it the producer's. The removal of
        // section 6.6.2.3.1's properties has already been folded into these bytes, because the
        // catalog's packet is one of the packets it edits.
        if self.wants_metadata()
            && let Some(metadata) = self.metadata
            && metadata.written.is_none()
            && metadata.at == id
        {
            if header_cut {
                count(applied, Rewrite::PacketHeaderAttributes);
            }
            return Rewritten::Changed(metadata_stream(&stream.dict, &metadata.packet));
        }
        // Every *other* metadata stream one of the two RDF writers edited: an object's own packet
        // is not the document's, and neither part restricts either requirement to the catalog's.
        // The respelling is asked first because it is the last of the two to run, so its bytes
        // carry the removal's as well.
        if self.wants(Rewrite::ExtensionSchemaPrefixes)
            && let Some(packet) = self
                .respelled
                .and_then(|respelled| respelled.packets.get(&id))
        {
            if header_cut {
                count(applied, Rewrite::PacketHeaderAttributes);
            }
            return Rewritten::Changed(metadata_stream(&stream.dict, packet));
        }
        if self.wants(Rewrite::PropertyOutsideItsSchema)
            && let Some(packet) = self.cleaned.and_then(|cleaned| cleaned.packets.get(&id))
        {
            if header_cut {
                count(applied, Rewrite::PacketHeaderAttributes);
            }
            return Rewritten::Changed(metadata_stream(&stream.dict, packet));
        }
        // And a packet whose *only* edit is the header's, which is the common case: neither
        // attribute has anything to do with the RDF, so a file whose metadata is otherwise
        // conforming fails this requirement on its own.
        if header_cut
            && let Some(packet) = self.packet_headers.and_then(|headers| headers.packet(id))
        {
            count(applied, Rewrite::PacketHeaderAttributes);
            return Rewritten::Changed(metadata_stream(&stream.dict, packet));
        }
        // The font program itself, restated: a whole new stream rather than a dictionary edit,
        // because its bytes, its `/Length` and its `/Length1` all change together.
        // One stream whichever of the two requirements asked for it, and counted under each
        // that did: a font whose dictionary disagrees with its program in both directions is
        // restated twice into one set of bytes, and writing it twice would drop the first.
        if let Some(metrics) = self.metrics
            && let Some(restated) = metrics.at.get(&id)
        {
            let horizontal =
                self.wants(Rewrite::RestateFontMetrics) && metrics.horizontal.contains(&id);
            let vertical =
                self.wants(Rewrite::RestateVerticalFontMetrics) && metrics.vertical.contains(&id);
            if horizontal {
                count(applied, Rewrite::RestateFontMetrics);
            }
            if vertical {
                count(applied, Rewrite::RestateVerticalFontMetrics);
            }
            if horizontal || vertical {
                return Rewritten::Changed(restated.clone());
            }
        }
        // The JPEG 2000 data itself, with the colour specification boxes the part directs a
        // processor to ignore taken out: the bytes and the `/Length` change together, so it is a
        // whole stream for the same reason the font program above is.
        if self.wants(Rewrite::Jpeg2000ColourSpecifications)
            && let Some(reduced) = self
                .specifications
                .and_then(|specifications| specifications.at.get(&id))
        {
            count(applied, Rewrite::Jpeg2000ColourSpecifications);
            return Rewritten::Changed(reduced.clone());
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

    /// The content stream `super::hexadecimal` repaired for this object, where it repaired one.
    fn hexadecimal_repair(&self, id: ObjectId) -> Option<Object> {
        if !self.wants(Rewrite::HexadecimalDigitCompleted) {
            return None;
        }
        self.hexadecimal
            .and_then(|completed| completed.at.get(&id))
            .cloned()
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
    /// Three rewrites reach it and they reach it for different reasons: the identification schema
    /// is the file's claim about itself, the `/DefaultCMYK` is an action `doc/questions/A48`
    /// requires recorded in `xmpMM:History`, and a removed property is both an edit to the packet
    /// and `doc/pdf-a-conversion-limits.md` section 4.2's entry beside it. Any one alone is enough
    /// to make the packet the prepared one.
    fn wants_metadata(&self) -> bool {
        self.wants(Rewrite::IdentificationSchema)
            || self.wants(Rewrite::DefaultCmyk)
            || self.wants(Rewrite::PropertyOutsideItsSchema)
    }
}

/// One `/ColorSpace` subdictionary with §8.6.5.6's `/DefaultCMYK` added.
///
/// `None` where it already states one, which is the producer's and stays theirs.
fn with_default_cmyk(spaces: &Dictionary, space: ObjectId) -> Option<Dictionary> {
    if spaces.get("DefaultCMYK").is_some() {
        return None;
    }
    let mut out = spaces.clone();
    out.insert(Name::new(&b"DefaultCMYK"[..]), Object::Reference(space));
    Some(out)
}

/// One resource dictionary whose `/ColorSpace` states the default.
///
/// `None` where the entry is already there, and where `/ColorSpace` is another object — that
/// object is [`CmykSites::spaces`]'s business and this dictionary is left alone.
fn resources_with_default_cmyk(resources: &Dictionary, space: ObjectId) -> Option<Dictionary> {
    let spaces = match resources.get("ColorSpace") {
        None => Dictionary::new(),
        Some(Object::Dictionary(spaces)) => spaces.clone(),
        Some(_) => return None,
    };
    let spaces = with_default_cmyk(&spaces, space)?;
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
