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
use super::prepare::{
    Appearances, Cleaned, DefaultCmyk, Intent, Metadata, Prepared, intent_dictionary,
    metadata_stream, output_intent_entries,
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
    /// Every annotation but a `Popup` that states no `/F` is given one whose only set bit is
    /// `Print`.
    ///
    /// ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2 require the entry and require that
    /// bit. §12.5.3's Table 167 numbers the flags from the low-order bit, and `Print` is bit
    /// position 3, so the value is 4 — every other flag left clear, which is exactly what
    /// §12.5.2's Table 166 already put in force by giving `/F` a default of 0. **The one bit that
    /// changes is the one the requirement is about.**
    AnnotationFlags,
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
            Self::AnnotationFlags => {
                "an annotation stating no /F is given one whose only set bit is Print, which is \
                 the value the requirement asks for and the default in every other bit"
            }
            Self::AppearanceDictionary => {
                "an annotation stating no appearance dictionary is given an /AP whose /N names a \
                 form XObject constructed from the entries its own subtype clause states"
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
            Self::AnnotationFlags => "annotation-flags",
            Self::AppearanceDictionary => "appearance-dictionary",
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
) -> Result<Converted, Refusal> {
    let root = crate::optimize::catalog_of(document)?;
    let sites = Sites::of(document, root, wanted);
    let rewriter = Rewriter {
        document,
        target,
        wanted,
        sites,
        added: prepared.added(),
        intent: prepared.intent.as_ref().ok(),
        metadata: prepared.metadata.as_ref().ok(),
        default_cmyk: prepared.default_cmyk.as_ref().ok(),
        to_unicode: prepared.to_unicode.as_ref().ok(),
        cleaned: prepared.properties.as_ref().ok(),
        appearances: prepared.appearances.as_ref().ok(),
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
    // The one rewrite whose places are properties rather than objects: a packet may lose six of
    // them and a report saying "1 done" would be counting the stream instead of the loss.
    if wanted.contains(&Rewrite::PropertyOutsideItsSchema)
        && let Ok(cleaned) = &prepared.properties
    {
        applied.insert(Rewrite::PropertyOutsideItsSchema, cleaned.removed.len());
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
        Self {
            catalog,
            names,
            pages,
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
    /// therefore asking for the entry the page does not have. What is written is what §7.7.3.3's
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

    /// A page with no `Resources` entry of its own, given the one §7.7.3.3 already puts in force.
    fn inherited(&mut self, document: &Document, id: ObjectId, page: &Dictionary) {
        let mut node = page.clone();
        for _ in 0..MAX_INHERITANCE_DEPTH {
            let Some(parent) = node.get("Parent").and_then(Object::as_reference) else {
                break;
            };
            let Some(above) = document.get(parent).as_dict().cloned() else {
                break;
            };
            match above.get("Resources") {
                // Inherited by reference: the page is given the same reference, so the dictionary
                // is not copied and the entry is written into it once for every page that shares
                // it. Nothing about what the page resolves changes.
                Some(reference @ Object::Reference(resources)) => {
                    self.resources.insert(*resources);
                    self.pages.insert(id, reference.clone());
                    return;
                }
                Some(Object::Dictionary(resources)) => {
                    self.pages.insert(id, Object::Dictionary(resources.clone()));
                    // The copy is the page's own object now, so the entry goes into it inline.
                    self.direct.insert(id);
                    return;
                }
                _ => node = above,
            }
        }
        // No resources anywhere above it: the page gets one holding the default and nothing else,
        // which is the smallest dictionary that makes the entry explicitly associated.
        self.pages.insert(id, Object::Dictionary(Dictionary::new()));
        self.direct.insert(id);
    }
}

/// How far up §7.7.3.3's page tree a `Resources` entry is looked for.
///
/// `pdf_syntax::Limits::DEFAULT`'s `max_depth`, which is the depth the parser admitted the tree
/// at: a `/Parent` chain longer than that is one no page in the document was read through.
const MAX_INHERITANCE_DEPTH: usize = 256;

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
    /// The `/DefaultCMYK` to write, where one is being written.
    default_cmyk: Option<&'a DefaultCmyk>,
    /// The `/ToUnicode` `CMap` each font is to name, where any are being written.
    to_unicode: Option<&'a DerivedMaps>,
    /// The packet each metadata stream is to carry, where properties are being removed.
    cleaned: Option<&'a Cleaned>,
    /// The appearance each annotation's `/AP` `/N` is to name, where any are being constructed.
    appearances: Option<&'a Appearances>,
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
        changed |= self.write_default_cmyk(id, &mut out, applied);
        changed |= self.complete_file_specification(&mut out, applied);
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
        // and this target's put in — every other byte of it the producer's. The removal of
        // section 6.6.2.3.1's properties has already been folded into these bytes, because the
        // catalog's packet is one of the packets it edits.
        if self.wants_metadata()
            && let Some(metadata) = self.metadata
            && metadata.written.is_none()
            && metadata.at == id
        {
            return Rewritten::Changed(metadata_stream(&stream.dict, &metadata.packet));
        }
        // Every *other* metadata stream a property was taken out of: an object's own packet is
        // not the document's, and neither part restricts the requirement to the catalog's.
        if self.wants(Rewrite::PropertyOutsideItsSchema)
            && let Some(packet) = self.cleaned.and_then(|cleaned| cleaned.packets.get(&id))
        {
            return Rewritten::Changed(metadata_stream(&stream.dict, packet));
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
