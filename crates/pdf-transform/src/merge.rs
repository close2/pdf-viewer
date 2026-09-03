//! `merge` — several documents into one, RFC 0002 section 6.2.
//!
//! # The mechanism is `split`'s; the work is the reconciliations
//!
//! Cross-file renumbering is [`Assembly`]'s already — it takes a list of sources and answers the
//! output's own numbering for an object out of any of them — so concatenating pages is the easy
//! half and this module is mostly the other one. A document is not a bag of pages: it states
//! optional-content groups, form fields, name trees, an outline, page labels and an output
//! intent, and each of those is *one* construct per document that several documents cannot
//! simply both have. RFC 0002 section 6.2 calls them a long tail of individually small
//! decisions, each of which must be a documented choice rather than an accident.
//!
//! So every one of them is below, under the clause it is derived from, and each one either
//! **carries** with a stated construction or **refuses by name**. A refusal that names the
//! clause and the collision is a correct outcome here. What is not acceptable is two documents'
//! constructs silently coexisting in a file where the standard says they cannot.
//!
//! ## What a page carries, and what stops it (§7.7.3.4, §7.3.10)
//!
//! Exactly `split`'s rules, and `split.rs` has the reading: a page crosses as its producer's
//! dictionary with `/Parent` renamed to the merged tree, Table 31's four inheritable attributes
//! flattened onto it because the ancestors that carried them are not coming along, and its
//! object closure copied byte for byte. The walk stops at a page or a page-tree node it is not
//! carrying, and such a reference becomes §7.3.10's null and is counted.
//!
//! ## §8.11's optional content
//!
//! §8.11.4.2 makes `/OCGs` an array of "all the optional content groups in the document" and
//! `/D` a single required default configuration, so the groups **concatenate** and the two
//! defaults have to become one. §8.11.4.3 is what makes that derivable rather than a guess:
//!
//! > If BaseState is present in the document's default configuration dictionary, its value
//! > shall be ON
//!
//! A conforming source's default configuration therefore starts every group ON and names the
//! exceptions in `/OFF`, so the merged `/D` is `/BaseState` absent (its default is `ON`) with
//! an `/OFF` array that is the union of what each source turned off. A source that states
//! `/BaseState /OFF` has said something that clause forbids; it is still read rather than
//! refused — every group of that source not in its own `/ON` array goes into the merged `/OFF`,
//! which is that source's own initial state written the way the merged file can state it — and
//! it is warned about by name. `/Order`, `/RBGroups`, `/Locked` and `/AS` are lists, so they
//! concatenate; `/Configs` is a list of whole alternate configurations and concatenates too.
//! `/ListMode`, `/Intent`, `/Name` and `/Creator` are single-valued and the first source that
//! states one wins, with a warning where a later source states a different value.
//!
//! **Two groups may share a `/Name` and that is not a collision.** Table 96 makes `/Name` "[t]he
//! name of the optional content group, suitable for presentation in an interactive PDF
//! processor's user interface" — a label, not an identifier: §8.11.3.2 makes content name its
//! groups through the resource dictionary's `/Properties`, by object. So nothing is renamed, and
//! a duplicate label is a warning so that two identically-labelled layers do not surprise the
//! person who asked for the merge.
//!
//! ## §12.7's interactive form
//!
//! §12.7.4.2 defines the fully qualified field name and closes with the sentence this
//! reconciliation rests on:
//!
//! > In addition, actual field dictionaries with the same fully qualified field name shall have
//! > the same field type ( FT ), value ( V ), and default value ( DV ).
//!
//! So a fully qualified name that two sources share is **permitted** where the three agree —
//! the clause's own case of one field with several representations — and **cannot be written**
//! where any of them differs. The first is carried with a warning naming the field; the second
//! is [`Refusal::FieldCollision`], naming every colliding name and the clause.
//!
//! The construction that would resolve it is worth recording as the road not taken: §12.7.4.2's
//! hierarchy would let this program put each source's roots under a synthesised non-terminal
//! field, so that every fully qualified name gained a per-source prefix. That renames every
//! field in the output, and a field's name is what §12.7.6.2's submit-form action exports and
//! what a data file matches on — so it is a change to what the document *means* that is
//! invisible on the page. It is not taken silently, and no flag asks for it yet.
//!
//! The rest of Table 224 reconciles entry by entry. `/NeedAppearances` is the union, because the
//! table makes it a claim about the whole document — "[a] PDF writer shall include this key,
//! with a value of true, if it has not provided appearance streams for all visible widget
//! annotations present in the document" — and a merged document has not provided them if any
//! source had not. `/SigFlags` is a bitwise union for the same reason: Table 225's bit 1 says
//! "the document contains at least one signature field" and bit 2 says it "contains signatures
//! that may be invalidated", and either is true of the merge if it is true of a source. `/CO` is
//! a calculation *order* and concatenates in input order. `/DR` is a resource dictionary and is
//! unioned per category with the first source winning a colliding resource name, warned by name:
//! §12.7.4.3 makes `/DR` matter only where an appearance is *constructed* rather than read, and
//! every appearance stream the sources hold crosses byte for byte. `/DA` and `/Q` are
//! "document-wide default value"s and take the first source that states one. `/XFA` is dropped
//! and warned: `CLAUDE.md` excludes Annex K, and §K.1 makes the `AcroForm` the consistent copy —
//! "[t]he other entries in the interactive form dictionary shall be consistent with the
//! information in the XFA resource" — so a document without the packet is the document the
//! packet described.
//!
//! ## §12.8's signatures
//!
//! §12.8.1 states what a signature covers and what a difference means:
//!
//! > A byte range digest shall be computed over a range of bytes in the PDF file, that shall be
//! > indicated by the ByteRange entry in the signature dictionary.
//!
//! > The digest shall be recomputed and compared with the one stored in the document.
//! > Differences between the two indicates that modifications have been made since the document
//! > was signed and thus the signature shall be considered invalid.
//!
//! A merged file is not the file any signature was computed over, and its `/ByteRange` offsets
//! name unrelated bytes in it. So a signature field crosses **without its `/V`**: the field is
//! carried, the signature dictionary it named is not, and the output states no signature rather
//! than one it knows cannot verify. Warned by name, per field. `/SigFlags` bit 1 stays set,
//! correctly — the document does still contain a signature field.
//!
//! That also settles §12.8.2.2 by construction. "A document can contain only one signature field
//! that contains a `DocMDP` transform method", and merging two certified documents would produce
//! two; with no `/V` carried, no transform method survives at all.
//!
//! ## §7.9.6's name trees
//!
//! > The keys contained within the various nodes' Names entries shall not overlap; each Names
//! > entry shall contain a single contiguous range of all the keys in the tree.
//!
//! So a key two sources share cannot appear twice, and the later source's is **renamed** — the
//! first free `key (2)`, `key (3)` and so on, deterministically — and reported. Each category
//! (`/EmbeddedFiles`, `/JavaScript`, `/AP`, `/Pages`, `/Templates`, `/IDS`, `/URLS`,
//! `/Renditions`, `/AlternatePresentations`) is its own namespace. The merged tree is one root
//! node holding one `/Names` array, sorted by key bytes, which is what the clause's "[s]horter
//! keys shall appear before longer ones beginning with the same byte sequence" describes and
//! what `Ord` on a byte string already does.
//!
//! **`/Dests` is the one whose references are chased.** §12.3.2.4 gives a named destination two
//! homes — the catalog's `/Dests` dictionary keyed by name objects, and the name tree keyed by
//! strings — and `pdf_model::destination` looks in both by the same bytes, so this module treats
//! them as **one** namespace across both homes and all sources, and emits each source's entries
//! back into the home it used them from. A renamed key is then rewritten wherever the standard
//! says a name-string destination is stated: an annotation's or an outline item's `/Dest`
//! (§12.3.2.3, §12.3.3) and a `/GoTo` action's `/D` (§12.6.4.2), in the carried objects
//! themselves — which is why those objects cross *replaced* rather than copied, and why the
//! renames are computed before the closure walk begins rather than after it.
//!
//! For every other category a renamed key is reported and its references are **not** chased,
//! because this program does not know what states them; the warning names the category and the
//! key so that the fact is in the report rather than in the file.
//!
//! ## §12.3.3's outline
//!
//! Table 150 makes an outline dictionary's items "a linked list, chained together through their
//! Prev and Next entries and accessed through the First and Last entries", and Table 151 makes
//! a top-level item's `/Parent` "the outline dictionary itself". So the sources' top-level
//! chains are **spliced into one**: each top-level item crosses with its `/Parent` naming the
//! merged outline dictionary and its `/Prev`/`/Next` naming its neighbours in input order, and
//! the merged dictionary's `/First`, `/Last` and `/Count` follow. Every item below the top level
//! crosses untouched, so each source's outline keeps its own shape.
//!
//! RFC 0002 section 6.2 proposed instead "one top-level item per source, a documented choice".
//! It is not taken, and the reason is Table 151: `/Title` is "( Required )" and this program has
//! no title for a source — the seam holds no paths and a document need state no `/Title` — so
//! the proposal costs an invented string on every merge. Splicing invents nothing.
//!
//! An item whose destination names a page the merge did not carry keeps §7.3.10's null and is
//! counted with every other dropped reference; it is not deleted, because deleting it would
//! rebuild a chain the source stated.
//!
//! **An `/Outlines` with no item is not carried and nothing is lost.** Table 150 makes `/First`
//! and `/Last` "( Required if there are any open or closed outline entries )", so a dictionary
//! with neither states no items at all; 29 corpus documents have exactly that. The merged
//! document states an `/Outlines` where at least one source contributed an item, and none where
//! no source did.
//!
//! ## §12.4.2's page labels
//!
//! A number tree keyed by page index cannot be concatenated: the merged indices are new and a
//! source's selection may reorder or subset its pages, so no range of the source's survives.
//! Where any source states `/PageLabels`, the merged tree therefore holds **one entry per output
//! page**, each reproducing the label that page had. §12.4.2 is what makes that exact:
//!
//! > There is no default numbering style; if no S entry is present, page labels shall consist
//! > solely of a label prefix with no numeric portion.
//!
//! so `<< /P (the label) >>` is the label and nothing else, and
//!
//! > The tree shall include a value for page index 0.
//!
//! is met because every page has an entry.
//!
//! **One documented choice with an edge**: a page out of a source that states no `/PageLabels`
//! at all had no label, and the standard does not say what such a page is called. It gets
//! `<< /S /D /St n >>` with `n` its own one-based position in its own source — the decimal
//! number a reader shows for an unlabelled page — so that it keeps its identification instead of
//! falling into the preceding source's range, which is the one answer that is certainly wrong.
//!
//! ## §14.11.5's output intents
//!
//! > The optional OutputIntents entry in the document catalog dictionary (see 7.7.2, "Document
//! > catalog dictionary") or a Page dictionary (see 7.7.3.3, "Page objects") holds an array of
//! > output intent dictionaries
//!
//! > If a PDF processor chooses to respect output intents, then when processing a page that has
//! > an associated (page-level) output intent, that page-level output intent shall be used.
//!
//! The clause gives the array two homes and makes the page's the one in force, which is exactly
//! the reconciliation a merge needs: where one source states a catalog array it is carried to
//! the merged catalog, and where **several** sources state arrays that are not all one, each
//! source's array is written onto its own carried pages. A device colour on a page then means
//! what it meant in the document the page came from. Session 888 taught this tree's own colour
//! path to read the page-level home for the same clause's sake, so the construction is one this
//! program can read back.
//!
//! ## What the merged catalog does not carry
//!
//! [`NOT_CARRIED`], every one named in a warning where a source states it, on `split`'s
//! argument: a construct nobody thought about is a construct nobody is told about.
//!
//! **`/Info` is deliberately in that list.** §14.3.3's entries are claims about *the document* —
//! its title, its author, its producer, when it was created — and the merged document was made
//! by no source's producer at no source's creation time, so carrying one source's would write a
//! false claim and synthesising one would be authoring metadata this program does not author.
//! Every source that states one is named in the warning.
//!
//! # Determinism
//!
//! The output is a function of the sources and the plan: objects are numbered in the order they
//! are added, the name trees are sorted by key bytes, the renames are the first free suffix, and
//! nothing here reads a clock. RFC 0002 section 9's first layer, with no flag.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::Write as _;
use std::sync::Arc;

use pdf_model::Pages;
use pdf_model::page_label::PageLabels;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::{Assembly, Form, Options, serialize};
use pdf_syntax::{Document, Version};

use crate::pattern::{Fill, Pattern};
use crate::range::Selection;
use crate::structure::{CarriedPage, Carry, Host};
use crate::{Origin, Output, Refusal, Report, Sinks, Warning, structure};

/// Several documents into one file.
#[derive(Debug, Clone, PartialEq)]
pub struct MergePlan {
    /// The inputs, in the order their pages appear in the output.
    pub inputs: Vec<Input>,
    /// Whether the inputs' pages interleave rather than concatenate — pdftk's `shuffle`.
    pub collate: bool,
    /// How the one output is named.
    pub names: Pattern,
}

/// One input of a merge: a source, and which of its pages in which order.
#[derive(Debug, Clone, PartialEq)]
pub struct Input {
    /// Which source.
    pub source: usize,
    /// Which pages, in which order.
    pub pages: Selection,
}

/// One page of the output: which document it comes from, which page of it, and what §7.7.3.3
/// `/Rotate` the output states for it.
///
/// The engine below writes a document out of a list of these, and the two verbs on it differ
/// only in how they build the list: `merge` concatenates or interleaves several documents'
/// selections, `pages` edits one document's list in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Placement {
    /// The document's position among the opened ones.
    pub(crate) at: usize,
    /// Its page, zero-based.
    pub(crate) page: usize,
    /// The `/Rotate` to write, where an edit decided one; `None` leaves the page's own — or,
    /// where §7.7.3.4 gave it one, its ancestor's, flattened as every inheritable attribute is.
    pub(crate) rotate: Option<i64>,
}

/// Whether one source page may take more than one place in the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Duplicates {
    /// Table 31 gives a page one `/Parent`, so naming it twice is [`Refusal::PageTwice`].
    ///
    /// `merge`'s answer: a merge that named one page twice is a plan whose author meant
    /// something this verb cannot tell from a mistake.
    Refuse,
    /// The second and later occurrences cross as **their own page objects**.
    ///
    /// `pages --insert`'s answer, and Table 31 is what makes it the only one: `/Parent` is
    /// "( Required; shall be an indirect reference ) The page tree node that is the immediate
    /// parent of this page object", so two places in one tree need two page objects. The
    /// content stream, the resources and everything below them are shared references — nothing
    /// in a page's closure points back at the page except its annotations, which are duplicated
    /// with it.
    Copy,
}

/// How deep a value tree is rewritten before its tail is dropped.
///
/// The parser's own `max_depth` is 256, so nothing it admitted is deeper.
const MAX_DEPTH: usize = 257;

/// How far up §7.7.3.4's `/Parent` chain an inheritable attribute is looked for.
///
/// `split.rs`'s bound and its reasoning: a tree's height is small, and the bound exists because
/// `/Parent` is a reference a hostile file can make into a cycle.
const MAX_ANCESTORS: usize = 64;

/// The four entries Table 31 marks inheritable, which §7.7.3.3 makes a closed list.
const INHERITABLE: [&str; 4] = ["Resources", "MediaBox", "CropBox", "Rotate"];

/// The catalog entries carried straight from the first source that states one.
///
/// Single-valued presentation entries with no per-page meaning: a merged document opens the way
/// its first input did. §7.7.2's `/Version` is deliberately absent — the header states the
/// highest version any source claims, and a catalog entry naming a lower one would say the
/// output conforms to less than it does.
const FIRST_WINS: [&str; 4] = ["Lang", "ViewerPreferences", "PageLayout", "PageMode"];

/// The catalog entries a merge does not carry, each named in a warning where a source has one.
///
/// `/AcroForm`, `/OCProperties`, `/Names`, `/Dests`, `/Outlines`, `/PageLabels`,
/// `/OutputIntents`, `/StructTreeRoot` and `/MarkInfo` are **not** here: each has its own
/// reconciliation, the last two in [`crate::structure`] since session 897. What is left is the
/// document-level constructs whose merging is nobody's documented choice yet, plus `/Info`,
/// whose reason is in the module comment.
const NOT_CARRIED: [&str; 9] = [
    "Metadata",
    "Threads",
    "SpiderInfo",
    "Collection",
    "Perms",
    "Legal",
    "Requirements",
    "DPartRoot",
    "OpenAction",
];

/// The §7.9.6 name trees a document's `/Names` dictionary may hold, in Table 33's order.
///
/// `/Dests` is first because it is the one whose renames are chased into the objects that state
/// them; the rest are renamed and reported.
const NAME_TREES: [&str; 10] = [
    "Dests",
    "AP",
    "JavaScript",
    "Pages",
    "Templates",
    "IDS",
    "URLS",
    "EmbeddedFiles",
    "AlternatePresentations",
    "Renditions",
];

/// One input's pages resolved against its document, with the source it came from.
struct Resolved {
    /// Which source, as the plan names it.
    source: usize,
    /// Its position among the opened documents.
    at: usize,
    /// Its selected pages, as zero-based indices.
    pages: Vec<usize>,
}

/// The object table being built, and everything the walk needs to fill it.
struct Merge<'a> {
    /// The opened documents, in [`Assembly`] source order.
    documents: &'a [Document],
    /// The output's objects.
    assembly: Assembly<'a>,
    /// Every page object each document has, so that the walk knows where to stop.
    all_pages: Vec<BTreeSet<ObjectId>>,
    /// Per document, the destination keys this merge renamed: the old bytes to the new.
    renames: Vec<BTreeMap<Vec<u8>, Vec<u8>>>,
    /// Per document, the signature fields whose `/V` does not cross (§12.8.1).
    unsigned: Vec<BTreeSet<ObjectId>>,
    /// While a duplicated page is being built, its annotations' own numbering.
    ///
    /// A page duplicated by `pages --insert` gets its own annotation objects (Table 172 gives
    /// an annotation one `/P`), and an annotation that names another — §12.5.6.14's `/Popup`,
    /// §12.5.6.10's `/IRT` — has to name the duplicate's copy rather than the original's. So
    /// the whole of the page's `/Annots` is numbered before any of it is built, and this map
    /// redirects every reference into that set while the page is under construction.
    redirect: BTreeMap<(usize, ObjectId), ObjectId>,
    /// Objects whose contents have still to be reached.
    pending: VecDeque<(usize, ObjectId)>,
    /// Objects that cross changed, and the slot each was given.
    to_rebuild: VecDeque<(usize, ObjectId, ObjectId)>,
    /// How many references named a page or a node the output does not hold.
    dropped: u64,
    /// Source objects the walk refuses outright: §14.7's elements that reach no carried page.
    blocked: BTreeSet<(usize, ObjectId)>,
    /// Whether this merge writes a §14.7 structure tree, so that §14.7.5.4's keys are its own.
    ///
    /// **Every** source's objects, not only the tagged ones: an annotation out of an untagged
    /// document that kept its producer's `/StructParent` would be a key into the *other*
    /// source's renumbered tree, naming an element it has nothing to do with. Asked by
    /// [`Merge::changes`], which runs from the first object the walk reaches — before
    /// [`Carry::plan`] has answered — so an object stating the key crosses *replaced*.
    restating_keys: bool,
    /// §14.7's carry, once planned.
    structure: Option<Carry>,
}

impl Merge<'_> {
    /// The new number for a source object, or `None` where the walk stops at it.
    fn map(&mut self, from: usize, id: ObjectId) -> Option<ObjectId> {
        if let Some(instead) = self.redirect.get(&(from, id)) {
            return Some(*instead);
        }
        if let Some(already) = self.assembly.copied(from, id) {
            return Some(already);
        }
        if self.blocked.contains(&(from, id))
            || self
                .all_pages
                .get(from)
                .is_some_and(|pages| pages.contains(&id))
            || self.is_tree_node(from, id)
        {
            self.dropped = self.dropped.saturating_add(1);
            return None;
        }
        if self.changes(from, id) {
            let placed = self.assembly.replace(from, id).ok()?;
            self.to_rebuild.push_back((from, id, placed));
            return Some(placed);
        }
        let placed = self.assembly.copy(from, id).ok()?;
        self.pending.push_back((from, id));
        Some(placed)
    }

    /// Whether this object cannot cross byte for byte.
    ///
    /// The two reasons are the module comment's: it states a destination name this merge
    /// renamed (§7.9.6), or it is a signature field whose `/V` cannot survive (§12.8.1). A
    /// stream is never rebuilt — nothing that states either lives in one — so a stream that
    /// somehow answered yes crosses copied rather than being turned into a dictionary.
    fn changes(&self, from: usize, id: ObjectId) -> bool {
        let Some(document) = self.documents.get(from) else {
            return false;
        };
        let value = document.get(id);
        // §14.7.5.4's third home. Table 359 puts `/StructParent` on "the stream dictionary of a
        // form or image XObject, or in an annotation dictionary", and its value "shall be the
        // integer key under which the entry corresponding to the object shall be found in the
        // structural parent tree" — this file's tree, so the key is rewritten. A stream is the
        // one object that crosses rebuilt *and* keeps its bytes: the dictionary is rewritten and
        // the encoded data is the same `Arc` the source holds.
        if self.restating_keys && structure::struct_parent(&value).is_some() {
            return true;
        }
        if !matches!(value, Object::Dictionary(_)) {
            return false;
        }
        if self.unsigned.get(from).is_some_and(|set| set.contains(&id)) {
            return true;
        }
        self.renames
            .get(from)
            .is_some_and(|renames| !renames.is_empty() && states_a_destination(&value, renames, 0))
    }

    /// Whether an object is a page-tree node, which the walk stops at because §7.7.3.2's
    /// `/Kids` reaches every other page in the document.
    fn is_tree_node(&self, from: usize, id: ObjectId) -> bool {
        self.documents.get(from).is_some_and(|document| {
            document
                .get_key_of(id, "Type")
                .as_ref()
                .and_then(Object::as_name)
                .is_some_and(|name| name.as_bytes() == b"Pages")
        })
    }

    /// One value with every reference mapped into the output's numbering and every renamed
    /// destination rewritten.
    ///
    /// Used for the objects this verb *builds*. A copied object needs none of it: the
    /// serializer renumbers it and answers §7.3.10's null for what the output does not hold.
    fn carry(&mut self, from: usize, value: &Object, depth: usize) -> Object {
        if depth >= MAX_DEPTH {
            return Object::Null;
        }
        match value {
            Object::Reference(id) => match self.map(from, *id) {
                Some(placed) => Object::Reference(placed),
                None => Object::Null,
            },
            Object::Array(items) => Object::Array(
                items
                    .iter()
                    .map(|item| self.carry(from, item, depth.saturating_add(1)))
                    .collect(),
            ),
            Object::Dictionary(dict) => {
                let renamed = self
                    .renames
                    .get(from)
                    .map(|renames| rename_destinations(dict, renames));
                let dict = renamed.as_ref().unwrap_or(dict);
                let mut out = Dictionary::new();
                for (key, entry) in dict.iter() {
                    let carried = self.carry(from, entry, depth.saturating_add(1));
                    out.insert(key.clone(), carried);
                }
                Object::Dictionary(out)
            }
            other => other.clone(),
        }
    }

    /// Registers every object one value refers to.
    fn reach(&mut self, from: usize, value: &Object, depth: usize) {
        if depth >= MAX_DEPTH {
            return;
        }
        match value {
            Object::Reference(id) => {
                let _ = self.map(from, *id);
            }
            Object::Array(items) => {
                for item in items {
                    self.reach(from, item, depth.saturating_add(1));
                }
            }
            Object::Dictionary(dict) => {
                for (_, entry) in dict.iter() {
                    self.reach(from, entry, depth.saturating_add(1));
                }
            }
            Object::Stream(stream) => {
                for (_, entry) in stream.dict.iter() {
                    self.reach(from, entry, depth.saturating_add(1));
                }
            }
            _ => {}
        }
    }

    /// Copies everything the pending objects reach, rebuilding the ones that cross changed.
    fn drain(&mut self) {
        loop {
            if let Some((from, id, placed)) = self.to_rebuild.pop_front() {
                let value = self
                    .documents
                    .get(from)
                    .map_or(Object::Null, |document| document.get(id));
                let rebuilt = self.rebuild(from, id, &value);
                let _ = self.assembly.place(placed, rebuilt);
                continue;
            }
            let Some((from, id)) = self.pending.pop_front() else {
                break;
            };
            let value = self
                .documents
                .get(from)
                .map_or(Object::Null, |document| document.get(id));
            self.reach(from, &value, 0);
        }
    }

    /// One object that crosses changed: its `/V` gone where it is a signature field this merge
    /// unsigned, and every reference and renamed destination in it carried.
    fn rebuild(&mut self, from: usize, id: ObjectId, value: &Object) -> Object {
        match value {
            Object::Dictionary(dict) => {
                let mut out = dict.clone();
                // §12.8.1: the digest was computed over another file's bytes, so the merged file
                // states no signature rather than one it knows cannot verify. Which objects
                // those are was decided by the field walk, where §12.7.4.1's inheritance of
                // `/FT` could be read.
                if self.unsigned.get(from).is_some_and(|set| set.contains(&id)) {
                    out.remove("V");
                }
                self.restate_structure_key(from, &mut out);
                self.carry(from, &Object::Dictionary(out), 0)
            }
            Object::Stream(stream) => {
                let mut dict = stream.dict.clone();
                self.restate_structure_key(from, &mut dict);
                let Object::Dictionary(dict) = self.carry(from, &Object::Dictionary(dict), 0)
                else {
                    return Object::Null;
                };
                // The bytes are the source's `Arc`, never decoded and never re-encoded: only the
                // dictionary crosses changed.
                Object::Stream(Arc::new(Stream {
                    dict,
                    data: Arc::clone(&stream.data),
                    decryption_failed: stream.decryption_failed,
                }))
            }
            other => self.carry(from, other, 0),
        }
    }

    /// §14.7.5.4's key for one object, restated in the output's own parent tree.
    ///
    /// Removed rather than kept where the carry has nothing to point the key at, because a key
    /// into a tree the output *does* state, naming nothing, tells an assistive processor that
    /// the content has a parent element and then hands it none (ADR 0831 section 2's distinction).
    fn restate_structure_key(&mut self, from: usize, dict: &mut Dictionary) {
        let Some(old) = dict.get("StructParent").and_then(Object::as_integer) else {
            return;
        };
        let Some(document) = self.documents.get(from) else {
            return;
        };
        let key = self
            .structure
            .as_mut()
            .and_then(|carry| carry.object_key(document, from, old));
        match key {
            Some(key) => {
                dict.insert(Name::new(&b"StructParent"[..]), Object::Integer(key));
            }
            None => {
                dict.remove("StructParent");
            }
        }
    }
}

/// [`crate::structure`]'s view of this merge's object table.
///
/// §14.7 is read and written once, in that module, for both verbs on this engine and for
/// `split`; what each verb owns is its own walk state, which is what this trait keeps out of it.
impl Host for Merge<'_> {
    fn source(&self, at: usize) -> Option<&Document> {
        self.documents.get(at)
    }

    fn carry_value(&mut self, at: usize, value: &Object) -> Object {
        self.carry(at, value, 0)
    }

    fn reserve_slot(&mut self) -> Option<ObjectId> {
        self.assembly.reserve().ok()
    }

    fn replace_object(&mut self, at: usize, id: ObjectId) -> Option<ObjectId> {
        self.assembly.replace(at, id).ok()
    }

    fn place_object(&mut self, id: ObjectId, object: Object) {
        // The slot was reserved by this module a moment ago and nothing else can have filled it.
        drop(self.assembly.place(id, object));
    }

    fn block_object(&mut self, at: usize, id: ObjectId) {
        self.blocked.insert((at, id));
    }
}

/// Whether this value states a named destination among the ones renamed.
///
/// The two places §12.3.2.3 and §12.6.4.2 put one: a `/Dest` entry, and a `/GoTo` action's
/// `/D`. Direct values only — an indirect one is its own object and is asked about separately.
fn states_a_destination(
    value: &Object,
    renames: &BTreeMap<Vec<u8>, Vec<u8>>,
    depth: usize,
) -> bool {
    if depth >= MAX_DEPTH {
        return false;
    }
    match value {
        Object::Array(items) => items
            .iter()
            .any(|item| states_a_destination(item, renames, depth.saturating_add(1))),
        Object::Dictionary(dict) => {
            if destination_key(dict).is_some_and(|key| renames.contains_key(&key)) {
                return true;
            }
            dict.iter()
                .any(|(_, entry)| states_a_destination(entry, renames, depth.saturating_add(1)))
        }
        _ => false,
    }
}

/// The destination name this dictionary states, where it states one as a name or a string.
fn destination_key(dict: &Dictionary) -> Option<Vec<u8>> {
    let entry = if dict
        .get("S")
        .and_then(Object::as_name)
        .is_some_and(|name| name.as_bytes() == b"GoTo")
    {
        dict.get("D")
    } else {
        dict.get("Dest")
    }?;
    name_or_string(entry)
}

/// A name object's or a string object's bytes, which §12.3.2.4 looks a destination up by.
fn name_or_string(value: &Object) -> Option<Vec<u8>> {
    match value {
        Object::Name(name) => Some(name.as_bytes().to_vec()),
        Object::String(bytes) => Some(bytes.to_vec()),
        _ => None,
    }
}

/// This dictionary with its stated destination renamed, where it states a renamed one.
fn rename_destinations(dict: &Dictionary, renames: &BTreeMap<Vec<u8>, Vec<u8>>) -> Dictionary {
    let Some(key) = destination_key(dict) else {
        return dict.clone();
    };
    let Some(to) = renames.get(&key) else {
        return dict.clone();
    };
    let entry = if dict
        .get("S")
        .and_then(Object::as_name)
        .is_some_and(|name| name.as_bytes() == b"GoTo")
    {
        "D"
    } else {
        "Dest"
    };
    let replacement = match dict.get(entry) {
        Some(Object::Name(_)) => Object::Name(Name::new(to.clone())),
        _ => Object::String(to.as_slice().into()),
    };
    let mut out = dict.clone();
    out.insert(Name::new(entry.as_bytes().to_vec()), replacement);
    out
}

/// §7.7.3.4's value for one inheritable attribute, taken unresolved from the nearest ancestor.
///
/// > If such an attribute is omitted from a page object, its value shall be inherited from an
/// > ancestor node in the page tree.
pub(crate) fn inherited(document: &Document, page: ObjectId, key: &str) -> Option<Object> {
    let mut at = document
        .get(page)
        .as_dict()
        .and_then(|dict| dict.get("Parent"))
        .and_then(Object::as_reference)?;
    let mut seen = BTreeSet::new();
    for _ in 0..MAX_ANCESTORS {
        if !seen.insert(at) {
            return None;
        }
        let node = document.get(at);
        let node = node.as_dict()?;
        if let Some(value) = node.get(key) {
            return Some(value.clone());
        }
        at = node.get("Parent").and_then(Object::as_reference)?;
    }
    None
}

/// What the document-level reconciliations are computed over.
struct Scope<'s> {
    /// The documents contributing pages, by position among the opened ones, in the order their
    /// pages first appear: the order that decides which source wins a §7.9.6 collision and
    /// which states a single-valued catalog entry.
    contributing: &'s [usize],
    /// The plan's own source index for each opened document, for a sentence a person reads.
    sources: &'s [usize],
    /// The output's pages, in the order they are written.
    order: &'s [Placement],
}

impl Scope<'_> {
    /// The plan's source index for one opened document.
    fn source(&self, at: usize) -> usize {
        self.sources.get(at).copied().unwrap_or(at)
    }
}

/// Where §14.11.5's array goes in the output.
enum Intents {
    /// No contributing document states one.
    Nowhere,
    /// One document contributes and states one, so it is the merged document's.
    Catalog(Object),
    /// Several documents contribute, so each stating one has it written onto its own pages —
    /// "when processing a page that has an associated (page-level) output intent, that
    /// page-level output intent shall be used".
    PerPage,
}

/// Every document-level reconciliation's outcome, built before the catalog is written.
struct Reconciled {
    /// §8.11.4.2's `/OCProperties`.
    optional_content: Option<Object>,
    /// §12.7.3's `/AcroForm`, where any source states one.
    form: Option<Object>,
    /// §7.9.6's merged `/Names` dictionary.
    names: Option<Object>,
    /// §12.3.2.4's catalog `/Dests` dictionary, the PDF 1.1 home.
    dests: Option<Object>,
    /// §12.3.3's spliced outline, where any source has one.
    outlines: Option<ObjectId>,
    /// §12.4.2's per-page number tree.
    page_labels: Option<Object>,
    /// §14.11.5's answer.
    intents: Intents,
}

/// Merges the inputs into one file.
///
/// `sources` is the plan's source index for each opened document, in [`Assembly`] order.
pub(crate) fn run(
    plan: &MergePlan,
    sources: &[usize],
    documents: &[Document],
    sinks: &dyn Sinks,
    report: &mut Report,
) -> Result<(), Refusal> {
    let mut resolved = Vec::new();
    for input in &plan.inputs {
        let at = sources
            .iter()
            .position(|source| *source == input.source)
            .ok_or(Refusal::NoSuchSource {
                at: input.source,
                count: sources.len(),
            })?;
        let document = documents.get(at).ok_or(Refusal::NoSuchSource {
            at: input.source,
            count: documents.len(),
        })?;
        let pages = Pages::new(document);
        let labels = PageLabels::read(document);
        let selected = input
            .pages
            .resolve(pages.len(), |index| labels.label(index))
            .map_err(|error| Refusal::Selection {
                at: input.source,
                error,
            })?;
        resolved.push(Resolved {
            source: input.source,
            at,
            pages: selected,
        });
    }

    let order = order(&resolved, plan.collate);
    // Table 31 gives a page "( Required; shall be an indirect reference ) The page tree node
    // that is the immediate parent of this page object" — one parent, so one place in the tree.
    let mut seen = BTreeSet::new();
    for place in &order {
        if !seen.insert((place.at, place.page)) {
            let source = resolved
                .iter()
                .find(|input| input.at == place.at)
                .map_or(place.at, |input| input.source);
            return Err(Refusal::PageTwice {
                at: source,
                page: place.page.saturating_add(1),
            });
        }
    }

    let assembled = write(
        &order,
        documents,
        sources,
        Duplicates::Refuse,
        &plan.names,
        sinks,
        report,
    )?;
    report.outputs.push(Output {
        name: assembled.name,
        bytes: assembled.bytes,
        sanitised: assembled.sanitised,
        origin: Origin::Merged {
            sources: resolved.iter().map(|input| input.source).collect(),
            pages: order.len(),
            objects: assembled.objects,
        },
    });
    Ok(())
}

/// One file written out of a list of placements: what both verbs on the serializer share.
///
/// The output's name, expanded from the pattern; its size and object count, for the caller's
/// own [`Origin`]. The caller pushes the [`Output`], because what an output *is* — a merge of
/// several documents, or one document's pages edited — is the verb's word rather than this
/// function's.
///
/// # Errors
///
/// [`Refusal::Assembly`] where the document cannot be built, [`Refusal::FieldCollision`] where
/// §12.7.4.2 forbids it, and [`Refusal::Sink`] where the output cannot be written.
pub(crate) fn write(
    order: &[Placement],
    documents: &[Document],
    sources: &[usize],
    duplicates: Duplicates,
    names: &Pattern,
    sinks: &dyn Sinks,
    report: &mut Report,
) -> Result<Assembled, Refusal> {
    if order.is_empty() {
        return Err(Refusal::Assembly(
            "a document of no pages would have a page tree with no leaf, and §7.7.3.2 makes \
             /Kids \"an array of indirect references to the immediate children\" of a node that \
             has some"
                .to_owned(),
        ));
    }
    let expanded = names.expand(&Fill {
        ordinal: 1,
        count: 1,
        page: None,
        label: None,
        title: None,
    });
    let mut warnings = Vec::new();
    let assembly = assemble(order, documents, sources, duplicates, &mut warnings)?;

    // `Document::version` is already §7.5.2's header raised by Table 29's `/Version` where the
    // catalog states a later one, so the highest of these is the highest any source claims —
    // which is why the written catalog states no `/Version` of its own.
    let version = documents
        .iter()
        .filter_map(Document::version)
        .max()
        .unwrap_or(Version { major: 1, minor: 7 });
    let form = Form::of_all(documents.iter());
    let mut writer = sinks.open(&expanded.name).map_err(|error| Refusal::Sink {
        name: expanded.name.clone(),
        error,
    })?;
    let written = serialize(&assembly, version, Options::new(form), &mut writer)
        .map_err(|error| Refusal::Assembly(format!("{}: {error}", expanded.name)))?;
    writer.flush().map_err(|error| Refusal::Sink {
        name: expanded.name.clone(),
        error,
    })?;
    drop(writer);

    report.warnings.append(&mut warnings);
    Ok(Assembled {
        name: expanded.name,
        bytes: written.bytes,
        sanitised: expanded.sanitised,
        objects: written.objects,
    })
}

/// What [`write`] produced, for the verb that asked for it to account for.
pub(crate) struct Assembled {
    /// The name the sink was opened with.
    pub(crate) name: String,
    /// How many bytes were written.
    pub(crate) bytes: u64,
    /// Whether the name had a byte replaced by sanitisation.
    pub(crate) sanitised: bool,
    /// How many indirect objects it took.
    pub(crate) objects: u32,
}

/// The output's pages, in the order they are written.
///
/// Concatenated by default, and interleaved under `--collate` — pdftk's `shuffle`, taking one
/// page from each input in turn until every input is spent, which is what makes two scanned
/// sides of a stack of paper into one document.
fn order(resolved: &[Resolved], collate: bool) -> Vec<Placement> {
    let place = |at: usize, page: usize| Placement {
        at,
        page,
        rotate: None,
    };
    if !collate {
        return resolved
            .iter()
            .flat_map(|input| input.pages.iter().map(|index| place(input.at, *index)))
            .collect();
    }
    let longest = resolved
        .iter()
        .map(|input| input.pages.len())
        .max()
        .unwrap_or(0);
    let mut out = Vec::new();
    for round in 0..longest {
        for input in resolved {
            if let Some(index) = input.pages.get(round) {
                out.push(place(input.at, *index));
            }
        }
    }
    out
}

/// The merged document's object table.
///
/// `Err` names why the merge cannot be assembled at all — a numbering ceiling, a page that is
/// not an indirect object, or §12.7.4.2's field-name collision.
///
/// # Errors
///
/// [`Refusal::Assembly`] and [`Refusal::FieldCollision`].
fn assemble<'a>(
    order: &[Placement],
    documents: &'a [Document],
    sources: &[usize],
    duplicates: Duplicates,
    warnings: &mut Vec<Warning>,
) -> Result<Assembly<'a>, Refusal> {
    /// The one sentence every numbering failure gets, since they all mean the same thing.
    const TOO_MANY: &str = "the merge needs more objects than one file can number";

    // The documents that contribute pages, in the order their pages first appear: the order
    // that decides which source's name wins a §7.9.6 collision and which states a first-wins
    // catalog entry.
    let mut contributing: Vec<usize> = Vec::new();
    for place in order {
        if !contributing.contains(&place.at) {
            contributing.push(place.at);
        }
    }

    let all_pages: Vec<BTreeSet<ObjectId>> = documents
        .iter()
        .map(|document| Pages::new(document).indices().into_keys().collect())
        .collect();
    let (renames, reported) = plan_destination_renames(documents, &contributing);
    let unsigned = plan_unsigned(documents);

    let mut merge = Merge {
        documents,
        assembly: Assembly::new(documents.iter().collect()),
        all_pages,
        renames,
        unsigned,
        redirect: BTreeMap::new(),
        pending: VecDeque::new(),
        to_rebuild: VecDeque::new(),
        dropped: 0,
        blocked: BTreeSet::new(),
        restating_keys: contributing
            .iter()
            .any(|at| structure::states_a_tree(documents.get(*at))),
        structure: None,
    };

    // The catalog and the page tree take the first two numbers, so that the output's numbering
    // starts where a reader would expect and does not depend on what the pages reached.
    let catalog = merge
        .assembly
        .reserve()
        .map_err(|_| Refusal::Assembly(TOO_MANY.to_owned()))?;
    let tree = merge
        .assembly
        .reserve()
        .map_err(|_| Refusal::Assembly(TOO_MANY.to_owned()))?;

    // §12.3.3's splice: the outline root and every source's top-level items are numbered before
    // anything is walked, so that each item's `/Prev` and `/Next` can name its new neighbours.
    let tops = top_level_outline_items(documents, &contributing);
    let outlines = if tops.is_empty() {
        None
    } else {
        Some(
            merge
                .assembly
                .reserve()
                .map_err(|_| Refusal::Assembly(TOO_MANY.to_owned()))?,
        )
    };
    let mut placed_tops: Vec<(usize, ObjectId, ObjectId)> = Vec::new();
    for (at, id) in &tops {
        let placed = merge
            .assembly
            .replace(*at, *id)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
        placed_tops.push((*at, *id, placed));
    }

    let pages = reserve_pages(&mut merge, order, sources, duplicates)?;

    let scope = Scope {
        contributing: &contributing,
        sources,
        order,
    };
    merge.structure = plan_structure(&mut merge, &pages, &contributing, sources, warnings)?;

    let intents = plan_intents(documents, &contributing);
    for page in &pages {
        let built = build_page(&mut merge, page, tree, &intents);
        merge
            .assembly
            .place(page.placed, built)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
    }
    splice_outline(&mut merge, &placed_tops, outlines)?;

    let reconciled = reconcile(&mut merge, &scope, outlines, intents, &reported, warnings)?;
    let root = build_catalog(&mut merge, tree, &reconciled, &scope, warnings);
    merge.drain();
    // The elements, the parent tree and the structure tree root, built last because the object
    // keys above are assigned by the walk that has just finished — and drained again, because
    // an element's attributes reach objects nothing else did.
    if let Some(carry) = merge.structure.take() {
        carry.finish(&mut merge, warnings);
        merge.drain();
    }

    if let Some(outlines) = reconciled.outlines {
        let node = build_outline_root(&placed_tops, documents, &tops);
        merge
            .assembly
            .place(outlines, node)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
    }

    merge
        .assembly
        .place(tree, page_tree(&pages))
        .and_then(|()| merge.assembly.place(catalog, root))
        .map_err(|error| Refusal::Assembly(error.to_string()))?;
    merge.assembly.set_root(catalog);

    report_losses(&merge, &scope, warnings);
    Ok(merge.assembly)
}

/// The output's one page-tree node: §7.7.3.2's `/Kids` in output order, and its `/Count`.
///
/// One node rather than a copy of the sources' shapes, because §7.7.3.2 makes the tree's shape
/// the producer's business — "[t]he simplest structure can consist of a single page tree node
/// that references all of the document's page objects directly" — and a merge's page order is
/// its own rather than any source's.
fn page_tree(pages: &[PlacedPage]) -> Object {
    let kids: Vec<Object> = pages
        .iter()
        .map(|page| Object::Reference(page.placed))
        .collect();
    let mut node = Dictionary::new();
    node.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Pages"[..])),
    );
    node.insert(
        Name::new(&b"Count"[..]),
        Object::Integer(i64::try_from(kids.len()).unwrap_or(i64::MAX)),
    );
    node.insert(Name::new(&b"Kids"[..]), Object::Array(kids));
    Object::Dictionary(node)
}

/// §14.7's carry, planned after every page has its number and before any page is built.
///
/// The order is what makes it work: a page's `/StructParents` is the carry's to state, so it has
/// to be decided before `build_page` runs, and every kept element needs its slot before the
/// closure walk can reach one by reference and copy the source's subtree in behind it.
///
/// # Errors
///
/// [`Refusal::StructureConflict`] and [`Refusal::Assembly`].
fn plan_structure(
    merge: &mut Merge<'_>,
    pages: &[PlacedPage],
    contributing: &[usize],
    sources: &[usize],
    warnings: &mut Vec<Warning>,
) -> Result<Option<Carry>, Refusal> {
    let carried: Vec<CarriedPage> = pages
        .iter()
        .map(|page| CarriedPage {
            at: page.at,
            source: page.id,
            placed: page.placed,
            duplicate: page.duplicate,
        })
        .collect();
    let source_of = |at: usize| sources.get(at).copied().unwrap_or(at);
    Carry::plan(merge, contributing, &carried, warnings, &source_of)
}

/// Every selected page's slot, taken before any page is built.
///
/// The numbering has to come first so that a reference from one page's annotation to another
/// page of the merge maps to a page rather than to §7.3.10's null.
///
/// # Errors
///
/// [`Refusal::Assembly`] where a page is not an indirect object or the numbering is exhausted.
fn reserve_pages(
    merge: &mut Merge<'_>,
    order: &[Placement],
    sources: &[usize],
    duplicates: Duplicates,
) -> Result<Vec<PlacedPage>, Refusal> {
    let documents = merge.documents;
    let mut pages = Vec::with_capacity(order.len());
    let mut seen = BTreeSet::new();
    for place in order {
        let document = documents.get(place.at).ok_or_else(|| {
            Refusal::Assembly("the plan names a document it was not given".to_owned())
        })?;
        let id = Pages::new(document)
            .get(place.page)
            .and_then(|page| page.id)
            .ok_or_else(|| {
                Refusal::Assembly(format!(
                    "page {} of source {} is not an indirect object, and a page tree's /Kids is \
                     \"an array of indirect references\" (§7.7.3.2)",
                    place.page.saturating_add(1),
                    sources.get(place.at).copied().unwrap_or(place.at)
                ))
            })?;
        // The first occurrence stands in for the source page, so every reference to it from
        // anywhere in the closure — a destination, an annotation's `/P` — reaches a page rather
        // than §7.3.10's null. A second occurrence is a page object of its own, which nothing
        // in the source names, and it is only reached from the tree's `/Kids`.
        let duplicate = !seen.insert((place.at, place.page));
        let placed = if duplicate && matches!(duplicates, Duplicates::Copy) {
            merge
                .assembly
                .reserve()
                .map_err(|error| Refusal::Assembly(error.to_string()))?
        } else {
            merge
                .assembly
                .replace(place.at, id)
                .map_err(|error| Refusal::Assembly(error.to_string()))?
        };
        pages.push(PlacedPage {
            at: place.at,
            id,
            placed,
            rotate: place.rotate,
            duplicate,
        });
    }
    Ok(pages)
}

/// One page of the output with its slot taken: what [`build_page`] needs to fill it.
struct PlacedPage {
    /// The document's position among the opened ones.
    at: usize,
    /// The source page object.
    id: ObjectId,
    /// Its number in the output.
    placed: ObjectId,
    /// The `/Rotate` an edit decided, where one did.
    rotate: Option<i64>,
    /// Whether an earlier placement already stands in for this source page.
    duplicate: bool,
}

/// §12.3.3's spliced chain, placed: each top-level item with the merged outline as its parent
/// and its input-order neighbours as its `/Prev` and `/Next`.
///
/// # Errors
///
/// [`Refusal::Assembly`] where a slot cannot be filled.
fn splice_outline(
    merge: &mut Merge<'_>,
    placed_tops: &[(usize, ObjectId, ObjectId)],
    outlines: Option<ObjectId>,
) -> Result<(), Refusal> {
    for (position, (at, id, placed)) in placed_tops.iter().enumerate() {
        let previous = position
            .checked_sub(1)
            .and_then(|before| placed_tops.get(before))
            .map(|(_, _, id)| *id);
        let next = placed_tops
            .get(position.saturating_add(1))
            .map(|(_, _, id)| *id);
        let Some(root) = outlines else {
            return Ok(());
        };
        let item = build_outline_item(merge, *at, *id, root, previous, next);
        merge
            .assembly
            .place(*placed, item)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
    }
    Ok(())
}

/// What the merged document lost that no reconciliation could keep: §7.6's protection, and
/// §7.3.10's nulls where a reference named a page the merge does not hold.
fn report_losses(merge: &Merge<'_>, scope: &Scope<'_>, warnings: &mut Vec<Warning>) {
    for at in scope.contributing {
        if merge.documents.get(*at).is_some_and(Document::is_encrypted) {
            warnings.push(Warning {
                source: scope.source(*at),
                page: None,
                detail: "the source is encrypted (§7.6) and the merged document is not; the \
                         serializer writes no /Encrypt"
                    .to_owned(),
            });
        }
    }
    if merge.dropped > 0 {
        warnings.push(Warning {
            source: scope.contributing.first().map_or(0, |at| scope.source(*at)),
            page: None,
            detail: format!(
                "{} reference(s) named a page or page-tree node the merge does not hold and were \
                 written as §7.3.10's null",
                merge.dropped
            ),
        });
    }
}

/// One emitted page: the source's dictionary, `/Parent` renamed, §7.7.3.4's inheritance
/// flattened onto it, §14.11.5's array written on where the merge put it there, §7.7.3.3's
/// `/Rotate` where an edit decided one, and every reference and renamed destination in it
/// carried.
fn build_page(
    merge: &mut Merge<'_>,
    page: &PlacedPage,
    tree: ObjectId,
    intents: &Intents,
) -> Object {
    let (from, source) = (page.at, page.id);
    let structure_key = merge
        .structure
        .as_ref()
        .and_then(|carry| carry.page_key(page.placed));
    // §7.7.3.3 makes a page a dictionary, and `Pages` would not have counted anything else as
    // one — so an empty dictionary here is a page the reader already disowned, not a panic.
    let dict = match merge
        .documents
        .get(from)
        .map(|document| document.get(source))
    {
        Some(Object::Dictionary(dict)) => dict,
        _ => Dictionary::new(),
    };
    // A duplicated page's annotations are its own: Table 172 makes `/P` "[a]n indirect
    // reference to the page object with which this annotation is associated", one page, so a
    // second placement of the page needs a second set. They are numbered before anything is
    // built so that an annotation naming another — §12.5.6.14's `/Popup`, §12.5.6.10's `/IRT` —
    // names the copy on this page rather than the original on the other one.
    if page.duplicate {
        merge.redirect = duplicate_annotations(merge, from, &dict);
    }
    let mut out = Dictionary::new();
    for (key, value) in dict.iter() {
        if key.as_bytes() == b"Parent" {
            continue;
        }
        let carried = merge.carry(from, value, 0);
        out.insert(key.clone(), carried);
    }
    for key in INHERITABLE {
        if out.get(key).is_none()
            && let Some(document) = merge.documents.get(from)
            && let Some(value) = inherited(document, source, key)
        {
            let carried = merge.carry(from, &value, 0);
            out.insert(Name::new(key.as_bytes()), carried);
        }
    }
    // §7.7.3.3: "The number of degrees by which the page shall be rotated clockwise when
    // displayed or printed. The value shall be a multiple of 90. Default value: 0." So a
    // rotation of zero is stated by saying nothing, and any other is stated outright —
    // replacing whatever the page or, through §7.7.3.4, its ancestor said, because the
    // ancestor is not coming along.
    if let Some(degrees) = page.rotate {
        out.remove("Rotate");
        if degrees != 0 {
            out.insert(Name::new(&b"Rotate"[..]), Object::Integer(degrees));
        }
    }
    // §14.11.5's page-level home, written only where the page does not state one of its own —
    // the clause makes the page's the array that "shall be used", so a page that states one
    // has already answered.
    if matches!(intents, Intents::PerPage)
        && out.get("OutputIntents").is_none()
        && let Some(array) = catalog_entry(merge.documents.get(from), "OutputIntents")
    {
        let carried = merge.carry(from, &array, 0);
        out.insert(Name::new(&b"OutputIntents"[..]), carried);
    }
    // Table 359: "( Required for all content streams containing marked-content sequences that
    // are structural content items; PDF 1.3 ) The integer key of this object's entry in the
    // structural parent tree." The key is this file's, so the carry states it; a page whose
    // source stated one that this output has no tree for loses the entry rather than keeping a
    // number that names nothing.
    if merge.structure.is_some() {
        out.remove("StructParents");
        if let Some(key) = structure_key {
            out.insert(Name::new(&b"StructParents"[..]), Object::Integer(key));
        }
    }
    out.insert(Name::new(&b"Parent"[..]), Object::Reference(tree));
    if page.duplicate {
        let annotations = std::mem::take(&mut merge.redirect);
        for ((_, id), placed) in &annotations {
            let copy = build_annotation(merge, from, *id, page.placed, &annotations);
            // The slot was reserved a moment ago and nothing else can have filled it.
            drop(merge.assembly.place(*placed, copy));
        }
    }
    Object::Dictionary(out)
}

/// Every annotation of a page about to be duplicated, given a number of its own.
///
/// Only an annotation that is an indirect object: one written directly into the `/Annots` array
/// is a value rather than an object, so the copy of the array is already a copy of it.
fn duplicate_annotations(
    merge: &mut Merge<'_>,
    from: usize,
    page: &Dictionary,
) -> BTreeMap<(usize, ObjectId), ObjectId> {
    let mut out = BTreeMap::new();
    let Some(Object::Array(items)) = page.get("Annots").map(|value| {
        merge
            .documents
            .get(from)
            .map_or_else(|| value.clone(), |document| document.resolve(value))
    }) else {
        return out;
    };
    for item in &items {
        let Some(id) = item.as_reference() else {
            continue;
        };
        if !matches!(
            merge.documents.get(from).map(|document| document.get(id)),
            Some(Object::Dictionary(_))
        ) {
            continue;
        }
        if let Ok(placed) = merge.assembly.reserve() {
            out.insert((from, id), placed);
        }
    }
    out
}

/// One duplicated annotation: the source's entries with `/P` naming the page it is now on.
///
/// Every reference into the duplicated set is redirected while this is built, so an annotation
/// that names another names the copy beside it.
fn build_annotation(
    merge: &mut Merge<'_>,
    from: usize,
    source: ObjectId,
    page: ObjectId,
    within: &BTreeMap<(usize, ObjectId), ObjectId>,
) -> Object {
    let Some(Object::Dictionary(dict)) = merge
        .documents
        .get(from)
        .map(|document| document.get(source))
    else {
        return Object::Null;
    };
    merge.redirect.clone_from(within);
    let mut out = Dictionary::new();
    for (key, value) in dict.iter() {
        if key.as_bytes() == b"P" {
            continue;
        }
        let carried = merge.carry(from, value, 0);
        out.insert(key.clone(), carried);
    }
    merge.redirect.clear();
    // Table 172: "( Optional except as noted below; PDF 1.3; indirect reference ) An indirect
    // reference to the page object with which this annotation is associated."
    out.insert(Name::new(&b"P"[..]), Object::Reference(page));
    Object::Dictionary(out)
}

/// One spliced top-level outline item: Table 151's `/Parent`, `/Prev` and `/Next` renamed to
/// the merged chain's, and everything else the source's.
fn build_outline_item(
    merge: &mut Merge<'_>,
    from: usize,
    source: ObjectId,
    root: ObjectId,
    previous: Option<ObjectId>,
    next: Option<ObjectId>,
) -> Object {
    let dict = match merge
        .documents
        .get(from)
        .map(|document| document.get(source))
    {
        Some(Object::Dictionary(dict)) => dict,
        _ => Dictionary::new(),
    };
    let mut out = Dictionary::new();
    for (key, value) in dict.iter() {
        if matches!(key.as_bytes(), b"Parent" | b"Prev" | b"Next") {
            continue;
        }
        let carried = merge.carry(from, value, 0);
        out.insert(key.clone(), carried);
    }
    // "The parent of a top-level item shall be the outline dictionary itself."
    out.insert(Name::new(&b"Parent"[..]), Object::Reference(root));
    // "( Required for all but the first item at each level )" and "( Required for all but the
    // last item at each level )": the ends of the merged chain state neither.
    if let Some(previous) = previous {
        out.insert(Name::new(&b"Prev"[..]), Object::Reference(previous));
    }
    if let Some(next) = next {
        out.insert(Name::new(&b"Next"[..]), Object::Reference(next));
    }
    Object::Dictionary(out)
}

/// Table 150's outline dictionary over the spliced chain.
fn build_outline_root(
    placed: &[(usize, ObjectId, ObjectId)],
    documents: &[Document],
    tops: &[(usize, ObjectId)],
) -> Object {
    let mut out = Dictionary::new();
    out.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Outlines"[..])),
    );
    if let Some((_, _, first)) = placed.first() {
        out.insert(Name::new(&b"First"[..]), Object::Reference(*first));
    }
    if let Some((_, _, last)) = placed.last() {
        out.insert(Name::new(&b"Last"[..]), Object::Reference(*last));
    }
    // "( Required if the document has any open outline entries ) Total number of visible
    // outline items at all levels of the outline. The value cannot be negative. This entry
    // shall be omitted if there are no open outline items." Each source counted its own; the
    // merged document's visible items are the sum, and a source stating none contributes none.
    let mut count: i64 = 0;
    let mut sources: BTreeSet<usize> = BTreeSet::new();
    for (at, _) in tops {
        sources.insert(*at);
    }
    for at in sources {
        let stated = documents
            .get(at)
            .and_then(|document| document.catalog().ok())
            .map(|catalog| {
                documents.get(at).map_or(Object::Null, |document| {
                    document.get_key(&catalog, "Outlines")
                })
            })
            .and_then(|outlines| {
                let dict = outlines.as_dict()?;
                documents.get(at)?.get_key(dict, "Count").as_integer()
            })
            .unwrap_or(0);
        count = count.saturating_add(stated.max(0));
    }
    if count > 0 {
        out.insert(Name::new(&b"Count"[..]), Object::Integer(count));
    }
    Object::Dictionary(out)
}

/// One dictionary entry that shall be an array, resolved.
///
/// **Every one of these entries may be an indirect object**, and reading them unresolved was a
/// defect the corpus walk found on its first run: `issue18823.pdf` states `/OFF`, `/Order` and
/// `/RBGroups` as references, so a merge that asked the dictionary for an array got none, wrote
/// a default configuration with nothing turned off, and drew the page with layers the document
/// had hidden. §7.3.10 makes an indirect reference "equivalent" to the object it names, so a
/// reader that only accepts the direct form is reading a different file.
fn array_at(document: Option<&Document>, dict: &Dictionary, key: &str) -> Vec<Object> {
    document.map_or_else(Vec::new, |document| {
        document
            .get_key(dict, key)
            .as_array()
            .map_or_else(Vec::new, <[Object]>::to_vec)
    })
}

/// One entry of a document's catalog, resolved.
fn catalog_entry(document: Option<&Document>, key: &str) -> Option<Object> {
    let document = document?;
    let catalog = document.catalog().ok()?;
    let value = document.get_key(&catalog, key);
    (!value.is_null()).then_some(value)
}

/// Every top-level outline item, per contributing document, in input order.
///
/// Table 150's `/First` and each item's `/Next`, walked with a visited set because both are
/// references a file states and a file can state a cycle.
fn top_level_outline_items(
    documents: &[Document],
    contributing: &[usize],
) -> Vec<(usize, ObjectId)> {
    let mut out = Vec::new();
    for at in contributing {
        let Some(document) = documents.get(*at) else {
            continue;
        };
        let Some(outlines) = catalog_entry(Some(document), "Outlines") else {
            continue;
        };
        let Some(dict) = outlines.as_dict() else {
            continue;
        };
        let mut next = dict.get("First").and_then(Object::as_reference);
        let mut seen = BTreeSet::new();
        while let Some(id) = next {
            if !seen.insert(id) {
                break;
            }
            let item = document.get(id);
            let Some(item) = item.as_dict() else {
                break;
            };
            out.push((*at, id));
            next = item.get("Next").and_then(Object::as_reference);
        }
    }
    out
}

/// §12.3.2.4's two homes for a named destination, as this tree's own reader asks them.
fn destination_keys(document: &Document) -> Vec<Vec<u8>> {
    let mut keys: Vec<Vec<u8>> = catalog_dests(document)
        .into_iter()
        .map(|(key, _)| key)
        .collect();
    keys.extend(
        tree_entries(document, "Dests")
            .into_iter()
            .map(|(key, _)| key),
    );
    keys.sort();
    keys.dedup();
    keys
}

/// The catalog's PDF 1.1 `/Dests` dictionary, as key bytes and unresolved values.
fn catalog_dests(document: &Document) -> Vec<(Vec<u8>, Object)> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    let dests = document.get_key(&catalog, "Dests");
    let Some(dests) = dests.as_dict() else {
        return Vec::new();
    };
    dests
        .iter()
        .map(|(key, value)| (key.as_bytes().to_vec(), value.clone()))
        .collect()
}

/// One §7.9.6 name tree of a document's `/Names` dictionary, as the leaves state it.
fn tree_entries(document: &Document, category: &str) -> Vec<(Vec<u8>, Object)> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    let names = document.get_key(&catalog, "Names");
    let Some(names) = names.as_dict() else {
        return Vec::new();
    };
    let root = document.get_key(names, category);
    let Some(root) = root.as_dict() else {
        return Vec::new();
    };
    pdf_syntax::tree::name_entries(root, &|object| document.resolve(object))
}

/// The first free key of the form `key (2)`, `key (3)`, … .
///
/// Deterministic, so that the same merge renames the same way every time — RFC 0002 section 9's
/// first layer applies to a rename as much as to an offset.
pub(crate) fn free_key(key: &[u8], taken: &BTreeSet<Vec<u8>>) -> Vec<u8> {
    for ordinal in 2..=u32::MAX {
        let mut candidate = key.to_vec();
        candidate.extend_from_slice(format!(" ({ordinal})").as_bytes());
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
    // Unreachable in practice — `taken` is finite and this tried four billion suffixes — and
    // stated rather than panicked, because principle 1 has no room for an unexplained abort.
    let mut candidate = key.to_vec();
    candidate.extend_from_slice(b" (duplicate)");
    candidate
}

/// Every document's destination renames, and the list of them for the report.
///
/// The first half is indexed by document position and maps a source's old key to its new one;
/// the second is `(document, from, to)` for every rename, in the order they were made.
type Renames = (
    Vec<BTreeMap<Vec<u8>, Vec<u8>>>,
    Vec<(usize, Vec<u8>, Vec<u8>)>,
);

/// Which destination names collide across the contributing documents, and what they become.
///
/// One namespace across both of §12.3.2.4's homes and every source, because
/// `pdf_model::destination` looks a name up in both by the same bytes: a key that stayed in one
/// home while a colliding one lived in the other would resolve to whichever this reader asked
/// for first, which is exactly the silent coexistence §7.9.6 forbids.
fn plan_destination_renames(documents: &[Document], contributing: &[usize]) -> Renames {
    let mut taken: BTreeSet<Vec<u8>> = BTreeSet::new();
    let mut renames: Vec<BTreeMap<Vec<u8>, Vec<u8>>> =
        (0..documents.len()).map(|_| BTreeMap::new()).collect();
    let mut reported = Vec::new();
    for at in contributing {
        let Some(document) = documents.get(*at) else {
            continue;
        };
        for key in destination_keys(document) {
            if taken.insert(key.clone()) {
                continue;
            }
            let to = free_key(&key, &taken);
            taken.insert(to.clone());
            if let Some(renames) = renames.get_mut(*at) {
                renames.insert(key.clone(), to.clone());
            }
            reported.push((*at, key, to));
        }
    }
    (renames, reported)
}

/// The signature fields whose `/V` does not cross, per document.
fn plan_unsigned(documents: &[Document]) -> Vec<BTreeSet<ObjectId>> {
    documents
        .iter()
        .map(|document| {
            fields_of(document)
                .into_iter()
                .filter(|field| field.signed)
                .filter_map(|field| field.id)
                .collect()
        })
        .collect()
}

/// Where §14.11.5's array goes.
///
/// One contributing document that states one keeps it in the catalog, which is the split's
/// answer and the ordinary one. Two or more contributing documents — whether or not both state
/// an array — put each stating document's array on its own pages, because a catalog array is a
/// statement about *every* page of the document and a page out of a source that stated none
/// would then be drawn under a colour meaning it never had.
fn plan_intents(documents: &[Document], contributing: &[usize]) -> Intents {
    let stated: Vec<Object> = contributing
        .iter()
        .filter_map(|at| catalog_entry(documents.get(*at), "OutputIntents"))
        .collect();
    match (contributing.len(), stated.len()) {
        (_, 0) => Intents::Nowhere,
        (1, _) => stated
            .into_iter()
            .next()
            .map_or(Intents::Nowhere, Intents::Catalog),
        _ => Intents::PerPage,
    }
}

/// One field of §12.7.4.1's hierarchy, with the three entries §12.7.4.2 makes a merge's business.
struct FieldEntry {
    /// §12.7.4.2's fully qualified field name.
    name: Vec<u8>,
    /// The object it is, where it is one (§12.7.4.1: "shall be an indirect object").
    id: Option<ObjectId>,
    /// Table 226's `/FT`, inherited where the field omits it.
    kind: Option<Vec<u8>>,
    /// Table 226's `/V`, inherited where the field omits it.
    value: Object,
    /// Table 226's `/DV`, inherited where the field omits it.
    default: Object,
    /// Whether this dictionary itself states the `/V` of a signature field (§12.8.1).
    signed: bool,
}

/// How deep §12.7.4.1's field hierarchy is walked.
///
/// The clause says "[a]n interactive PDF processor shall not limit the range of inheritance for
/// field dictionaries", which is a statement about *inheritance* rather than a licence to walk a
/// cycle; the visited set is what stops one and this is the belt beside it.
const MAX_FIELD_DEPTH: usize = 64;

/// Every field of a document's interactive form, with its fully qualified name.
///
/// §12.7.4.2:
///
/// > A field dictionary that does not have a partial field name ( T entry) of its own shall not
/// > be considered a field but simply a Widget annotation.
///
/// so a `/Kids` entry with no `/T` contributes no name and is not descended into as a field.
fn fields_of(document: &Document) -> Vec<FieldEntry> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    let form = document.get_key(&catalog, "AcroForm");
    let Some(form) = form.as_dict() else {
        return Vec::new();
    };
    let roots = document.get_key(form, "Fields");
    let Some(roots) = roots.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for root in roots {
        walk_field(
            document,
            root,
            &[],
            &Inherited::default(),
            0,
            &mut seen,
            &mut out,
        );
    }
    out
}

/// The three inheritable entries §12.7.4.2's collision rule is about, as an ancestor stated them.
#[derive(Clone)]
struct Inherited {
    /// Table 226's `/FT`.
    kind: Option<Vec<u8>>,
    /// Table 226's `/V`.
    value: Object,
    /// Table 226's `/DV`.
    default: Object,
}

impl Default for Inherited {
    /// Nothing inherited. §7.3.9 makes an absent entry and a null one the same — "equivalent to
    /// omitting the entry" — which is what a root field's ancestors state.
    fn default() -> Self {
        Self {
            kind: None,
            value: Object::Null,
            default: Object::Null,
        }
    }
}

/// One field and its descendants.
fn walk_field(
    document: &Document,
    entry: &Object,
    prefix: &[u8],
    inherited: &Inherited,
    depth: usize,
    seen: &mut BTreeSet<ObjectId>,
    out: &mut Vec<FieldEntry>,
) {
    if depth >= MAX_FIELD_DEPTH {
        return;
    }
    let id = entry.as_reference();
    if let Some(id) = id
        && !seen.insert(id)
    {
        return;
    }
    let resolved = document.resolve(entry);
    let Some(dict) = resolved.as_dict() else {
        return;
    };
    let Some(partial) = dict.get("T").and_then(Object::as_string) else {
        // A widget annotation, not a field: it contributes no name and no children of its own.
        return;
    };
    let mut name = prefix.to_vec();
    if !name.is_empty() {
        name.push(b'.');
    }
    name.extend_from_slice(partial);

    let mine = Inherited {
        kind: dict
            .get("FT")
            .and_then(Object::as_name)
            .map(|name| name.as_bytes().to_vec())
            .or_else(|| inherited.kind.clone()),
        value: match dict.get("V") {
            Some(value) => document.resolve(value),
            None => inherited.value.clone(),
        },
        default: match dict.get("DV") {
            Some(value) => document.resolve(value),
            None => inherited.default.clone(),
        },
    };
    out.push(FieldEntry {
        name: name.clone(),
        id,
        kind: mine.kind.clone(),
        value: mine.value.clone(),
        default: mine.default.clone(),
        signed: mine.kind.as_deref() == Some(b"Sig".as_slice()) && dict.get("V").is_some(),
    });

    let kids = document.get_key(dict, "Kids");
    if let Some(kids) = kids.as_array() {
        for kid in kids {
            walk_field(
                document,
                kid,
                &name,
                &mine,
                depth.saturating_add(1),
                seen,
                out,
            );
        }
    }
}

/// How deep two field values are compared.
const MAX_VALUE_DEPTH: usize = 32;

/// Whether two field values out of two documents are the same value.
///
/// `None` where they cannot be shown to be — a cycle, or a nesting past [`MAX_VALUE_DEPTH`] —
/// which the caller treats as a difference, because §12.7.4.2's `shall` is a claim the merge has
/// to be able to make rather than one it may assume.
fn same_value(
    left_document: &Document,
    left: &Object,
    right_document: &Document,
    right: &Object,
    depth: usize,
) -> Option<bool> {
    if depth >= MAX_VALUE_DEPTH {
        return None;
    }
    let left = left_document.resolve(left);
    let right = right_document.resolve(right);
    match (&left, &right) {
        (Object::Null, Object::Null) => Some(true),
        (Object::Boolean(a), Object::Boolean(b)) => Some(a == b),
        (Object::Integer(a), Object::Integer(b)) => Some(a == b),
        (Object::Real(a), Object::Real(b)) => Some(a.to_bits() == b.to_bits()),
        (Object::String(a), Object::String(b)) => Some(a == b),
        (Object::Name(a), Object::Name(b)) => Some(a.as_bytes() == b.as_bytes()),
        (Object::Array(a), Object::Array(b)) => {
            if a.len() != b.len() {
                return Some(false);
            }
            for (a, b) in a.iter().zip(b.iter()) {
                if !same_value(left_document, a, right_document, b, depth.saturating_add(1))? {
                    return Some(false);
                }
            }
            Some(true)
        }
        (Object::Dictionary(a), Object::Dictionary(b)) => {
            if a.len() != b.len() {
                return Some(false);
            }
            for ((left_key, a), (right_key, b)) in a.iter().zip(b.iter()) {
                if left_key.as_bytes() != right_key.as_bytes() {
                    return Some(false);
                }
                if !same_value(left_document, a, right_document, b, depth.saturating_add(1))? {
                    return Some(false);
                }
            }
            Some(true)
        }
        // A stream's bytes are a document's, and a reference that survived `resolve` is a cycle.
        (Object::Stream(_) | Object::Reference(_), _)
        | (_, Object::Stream(_) | Object::Reference(_)) => None,
        _ => Some(false),
    }
}

/// §7.9.2.2's text string, in the encoding that carries the bytes this label holds.
///
/// ASCII goes in as a literal string, which every version of every reader reads the same way;
/// anything else goes in as UTF-16BE behind §7.9.2.2's byte order marker, which is the encoding
/// the clause defines for a text string that `PDFDocEncoding` cannot state.
fn text_string(text: &str) -> Object {
    if text.is_ascii() {
        return Object::String(text.as_bytes().into());
    }
    let mut bytes = vec![0xFE, 0xFF];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    Object::String(bytes.as_slice().into())
}

/// Every document-level reconciliation, in one place.
fn reconcile(
    merge: &mut Merge<'_>,
    scope: &Scope<'_>,
    outlines: Option<ObjectId>,
    intents: Intents,
    reported: &[(usize, Vec<u8>, Vec<u8>)],
    warnings: &mut Vec<Warning>,
) -> Result<Reconciled, Refusal> {
    for (at, from, to) in reported {
        warnings.push(Warning {
            source: scope.source(*at),
            page: None,
            detail: format!(
                "§7.9.6: the destination name {} is already this merge's, so this source's \
                 became {}; every /Dest and /GoTo in its pages and outline was rewritten to \
                 match",
                printable(from),
                printable(to)
            ),
        });
    }
    let (names, dests) = merge_name_trees(merge, scope, warnings);
    Ok(Reconciled {
        optional_content: merge_optional_content(merge, scope, warnings),
        form: merge_form(merge, scope, warnings)?,
        names,
        dests,
        outlines,
        page_labels: merge_page_labels(merge, scope.order),
        intents,
    })
}

/// A key's bytes, for a sentence a person reads.
fn printable(key: &[u8]) -> String {
    String::from_utf8_lossy(key).escape_debug().to_string()
}

/// What the optional-content reconciliation gathers as it reads each source.
#[derive(Default)]
struct Layers {
    /// Whether any source states `/OCProperties` at all.
    any: bool,
    /// Table 98's `/OCGs`: "[e]very optional content group shall be included in this array."
    groups: Vec<Object>,
    /// The merged `/D`'s `/OFF`, which under a `/BaseState` of `ON` states every exception.
    off: Vec<Object>,
    /// Table 99's `/Order`, concatenated.
    presentation: Vec<Object>,
    /// Table 99's `/RBGroups`, concatenated: each source's radio collections stay its own.
    radio: Vec<Object>,
    /// Table 99's `/Locked`, concatenated.
    locked: Vec<Object>,
    /// Table 99's `/AS`, concatenated.
    automatic: Vec<Object>,
    /// Table 98's `/Configs`, concatenated.
    configs: Vec<Object>,
    /// Table 99's single-valued entries, and the source each was taken from.
    singles: BTreeMap<&'static str, (usize, Object)>,
    /// Table 96's `/Name` labels already seen, and the source that used each.
    labels: BTreeMap<Vec<u8>, usize>,
}

/// Table 99's entries that are one value rather than a list.
///
/// `/Intent` and `/ListMode` decide what a processor consults and shows; `/Name` and `/Creator`
/// are strings for a user interface. None of them can be concatenated, so the first source that
/// states one wins and a later disagreement is a warning.
const ONE_VALUED_CONFIGURATION: [&str; 4] = ["ListMode", "Intent", "Name", "Creator"];

/// §8.11.4.2's optional content properties, merged.
fn merge_optional_content(
    merge: &mut Merge<'_>,
    scope: &Scope<'_>,
    warnings: &mut Vec<Warning>,
) -> Option<Object> {
    let mut layers = Layers::default();
    for at in scope.contributing {
        read_layers(merge, *at, scope.source(*at), &mut layers, warnings);
    }
    if !layers.any {
        return None;
    }
    let mut default = Dictionary::new();
    for (key, (_, value)) in &layers.singles {
        default.insert(Name::new(key.as_bytes()), value.clone());
    }
    for (key, items) in [
        ("OFF", layers.off),
        ("Order", layers.presentation),
        ("RBGroups", layers.radio),
        ("Locked", layers.locked),
        ("AS", layers.automatic),
    ] {
        if !items.is_empty() {
            default.insert(Name::new(key.as_bytes()), Object::Array(items));
        }
    }
    let mut out = Dictionary::new();
    out.insert(Name::new(&b"OCGs"[..]), Object::Array(layers.groups));
    out.insert(Name::new(&b"D"[..]), Object::Dictionary(default));
    if !layers.configs.is_empty() {
        out.insert(Name::new(&b"Configs"[..]), Object::Array(layers.configs));
    }
    Some(Object::Dictionary(out))
}

/// One source's optional content, read into the gathering.
fn read_layers(
    merge: &mut Merge<'_>,
    at: usize,
    source: usize,
    layers: &mut Layers,
    warnings: &mut Vec<Warning>,
) {
    let documents = merge.documents;
    let Some(document) = documents.get(at) else {
        return;
    };
    let Some(properties) = catalog_entry(Some(document), "OCProperties") else {
        return;
    };
    let Some(properties) = properties.as_dict() else {
        return;
    };
    layers.any = true;
    let stated = array_at(Some(document), properties, "OCGs");
    for group in &stated {
        let carried = merge.carry(at, group, 0);
        if !carried.is_null() {
            layers.groups.push(carried);
        }
        let label = document
            .resolve(group)
            .as_dict()
            .and_then(|dict| dict.get("Name"))
            .and_then(Object::as_string)
            .map(<[u8]>::to_vec);
        if let Some(label) = label
            && let Some(first) = layers.labels.insert(label.clone(), source)
            && first != source
        {
            warnings.push(Warning {
                source,
                page: None,
                detail: format!(
                    "§8.11.2.1 Table 96: source {first} also has an optional content group named \
                     {}, and the name is a user-interface label rather than an identifier, so \
                     both groups are carried under it",
                    printable(&label)
                ),
            });
        }
    }

    let default = document.get_key(properties, "D");
    let Some(default) = default.as_dict() else {
        return;
    };
    read_initial_states(merge, at, source, default, &stated, layers, warnings);
    let Some(document) = documents.get(at) else {
        return;
    };
    for (key, into) in [
        ("Order", &mut layers.presentation),
        ("RBGroups", &mut layers.radio),
        ("Locked", &mut layers.locked),
        ("AS", &mut layers.automatic),
    ] {
        for item in &array_at(documents.get(at), default, key) {
            into.push(merge.carry(at, item, 0));
        }
    }
    for item in &array_at(documents.get(at), properties, "Configs") {
        layers.configs.push(merge.carry(at, item, 0));
    }
    for key in ONE_VALUED_CONFIGURATION {
        let value = document.get_key(default, key);
        if value.is_null() {
            continue;
        }
        let carried = merge.carry(at, &value, 0);
        keep_first(
            &mut layers.singles,
            key,
            source,
            carried,
            warnings,
            |first| {
                format!(
                    "§8.11.4.3: /{key} is one value for a configuration and source {first} states a \
                 different one; the merged /D keeps source {first}'s"
                )
            },
        );
    }
}

/// §8.11.4.3's initial states for one source's groups, written the way the merged `/D` states
/// them.
///
/// §8.11.4.3, Table 99's `/BaseState` row:
///
/// > If BaseState is present in the document's default configuration dictionary, its value
/// > shall be ON
///
/// so a conforming source names its exceptions in `/OFF` and those are what the merged `/OFF`
/// takes. A source that says otherwise is read rather than refused: with `/BaseState /OFF`,
/// every group it did not turn on starts off, which is that source's own initial state stated
/// the way a merged default configuration can state it.
fn read_initial_states(
    merge: &mut Merge<'_>,
    at: usize,
    source: usize,
    default: &Dictionary,
    stated: &[Object],
    layers: &mut Layers,
    warnings: &mut Vec<Warning>,
) {
    let stated_off = array_at(merge.documents.get(at), default, "OFF");
    let stated_on = array_at(merge.documents.get(at), default, "ON");
    let base = merge
        .documents
        .get(at)
        .map(|document| document.get_key(default, "BaseState"))
        .as_ref()
        .and_then(Object::as_name)
        .map(|name| name.as_bytes().to_vec());
    let turn_off: Vec<Object> = match base.as_deref() {
        Some(b"OFF") => {
            let on: BTreeSet<Option<ObjectId>> =
                stated_on.iter().map(Object::as_reference).collect();
            stated
                .iter()
                .filter(|group| !on.contains(&group.as_reference()))
                .cloned()
                .collect()
        }
        _ => stated_off,
    };
    for group in &turn_off {
        let carried = merge.carry(at, group, 0);
        if !carried.is_null() {
            layers.off.push(carried);
        }
    }
    if let Some(other) = base.as_deref()
        && other != b"ON"
    {
        warnings.push(Warning {
            source,
            page: None,
            detail: format!(
                "§8.11.4.3: a default configuration's /BaseState \"shall be ON\", and this source \
                 states /{}; the merged /D turns off every group this source's own configuration \
                 started off",
                printable(other)
            ),
        });
    }
}

/// The first source that states a single-valued entry keeps it, and a later disagreement is a
/// warning worded by the caller.
fn keep_first(
    into: &mut BTreeMap<&'static str, (usize, Object)>,
    key: &'static str,
    source: usize,
    value: Object,
    warnings: &mut Vec<Warning>,
    detail: impl FnOnce(usize) -> String,
) {
    match into.get(key) {
        None => {
            into.insert(key, (source, value));
        }
        Some((first, before)) if *before != value => warnings.push(Warning {
            source,
            page: None,
            detail: detail(*first),
        }),
        Some(_) => {}
    }
}

/// What the form reconciliation gathers as it reads each source.
#[derive(Default)]
struct Gathered {
    /// Whether any source states an `/AcroForm` at all.
    any: bool,
    /// Table 224's `/Fields`, concatenated.
    roots: Vec<Object>,
    /// Table 224's `/CO`, concatenated: each source's calculation order stays its own.
    calculation: Vec<Object>,
    /// Table 224's `/NeedAppearances`, the union.
    need_appearances: bool,
    /// Table 225's flags, the bitwise union.
    signature_flags: i64,
    /// Table 224's `/DR`, unioned per §7.8.3 category.
    default_resources: Vec<(Name, Dictionary)>,
    /// Table 224's `/DA` and `/Q`, and the source each was taken from.
    singles: BTreeMap<&'static str, (usize, Object)>,
    /// Every fully qualified field name already claimed, and the document that claimed it.
    claimed: BTreeMap<Vec<u8>, (usize, FieldEntry)>,
    /// §12.7.4.2's collisions, worded.
    collisions: Vec<String>,
}

/// §12.7.3's interactive form dictionary, merged entry by entry.
///
/// # Errors
///
/// [`Refusal::FieldCollision`] where two sources state the same fully qualified field name with
/// a different `/FT`, `/V` or `/DV`, which §12.7.4.2 forbids a single document to hold.
fn merge_form(
    merge: &mut Merge<'_>,
    scope: &Scope<'_>,
    warnings: &mut Vec<Warning>,
) -> Result<Option<Object>, Refusal> {
    let mut form = Gathered::default();
    for at in scope.contributing {
        read_form(merge, *at, scope, &mut form, warnings);
    }
    if !form.collisions.is_empty() {
        form.collisions.sort();
        form.collisions.dedup();
        return Err(Refusal::FieldCollision {
            fields: form.collisions.join("; "),
        });
    }
    // §12.7.3 makes an interactive form dictionary what "[t]he contents and properties of a
    // document's interactive form shall be defined by", and Table 224 makes `/Fields` — "( Required )
    // An array of references to the document's root fields" — the whole of those contents. A merge
    // whose sources between them state no root field has no interactive form, so it states none,
    // and `/DA`, `/DR` and `/Q` are defaults for fields that do not exist.
    //
    // **Found by the corpus walk, on `bug1865341.pdf`.** The fixed second document states an
    // `/AcroForm` whose `/Fields` is empty; carrying it put an interactive form dictionary into a
    // merged document with no field in it, and this tree's own reader then drew a *different*
    // source's annotation differently. An entry that changes what another source's page marks is
    // exactly what the reconciliations exist to prevent.
    if !form.any || form.roots.is_empty() {
        return Ok(None);
    }
    let mut out = Dictionary::new();
    // "( Required ) An array of references to the document's root fields".
    out.insert(Name::new(&b"Fields"[..]), Object::Array(form.roots));
    for (key, (_, value)) in &form.singles {
        out.insert(Name::new(key.as_bytes()), value.clone());
    }
    if form.need_appearances {
        out.insert(Name::new(&b"NeedAppearances"[..]), Object::Boolean(true));
    }
    if form.signature_flags != 0 {
        out.insert(
            Name::new(&b"SigFlags"[..]),
            Object::Integer(form.signature_flags),
        );
    }
    if !form.calculation.is_empty() {
        out.insert(Name::new(&b"CO"[..]), Object::Array(form.calculation));
    }
    if !form.default_resources.is_empty() {
        let mut resources = Dictionary::new();
        for (category, entries) in form.default_resources {
            resources.insert(category, Object::Dictionary(entries));
        }
        out.insert(Name::new(&b"DR"[..]), Object::Dictionary(resources));
    }
    Ok(Some(Object::Dictionary(out)))
}

/// One source's interactive form, read into the gathering.
fn read_form(
    merge: &mut Merge<'_>,
    at: usize,
    scope: &Scope<'_>,
    form: &mut Gathered,
    warnings: &mut Vec<Warning>,
) {
    let source = scope.source(at);
    let documents = merge.documents;
    let Some(document) = documents.get(at) else {
        return;
    };
    let Some(stated) = catalog_entry(Some(document), "AcroForm") else {
        return;
    };
    let Some(stated) = stated.as_dict() else {
        return;
    };
    form.any = true;
    for (key, into) in [("Fields", &mut form.roots), ("CO", &mut form.calculation)] {
        for item in &array_at(documents.get(at), stated, key) {
            let carried = merge.carry(at, item, 0);
            if !carried.is_null() {
                into.push(carried);
            }
        }
    }
    // "A PDF writer shall include this key, with a value of true, if it has not provided
    // appearance streams for all visible widget annotations present in the document."
    form.need_appearances = form.need_appearances
        || matches!(
            document.get_key(stated, "NeedAppearances"),
            Object::Boolean(true)
        );
    // Table 225's two flags are both existence claims about the document, so the merged
    // document's word is the union of its sources'.
    form.signature_flags |= document
        .get_key(stated, "SigFlags")
        .as_integer()
        .unwrap_or(0);
    if stated.get("XFA").is_some() {
        warnings.push(Warning {
            source,
            page: None,
            detail: "§K.1: this source carries an /XFA packet, which the merged document does \
                     not; the annex makes the AcroForm the consistent copy — \"[t]he other \
                     entries in the interactive form dictionary shall be consistent with the \
                     information in the XFA resource\""
                .to_owned(),
        });
    }
    merge_default_resources(
        merge,
        at,
        stated,
        source,
        &mut form.default_resources,
        warnings,
    );
    for key in ["DA", "Q"] {
        let value = document.get_key(stated, key);
        if value.is_null() {
            continue;
        }
        let carried = merge.carry(at, &value, 0);
        keep_first(&mut form.singles, key, source, carried, warnings, |first| {
            format!(
                "Table 224: /{key} is a document-wide default and source {first} states a \
                 different one; the merged form keeps source {first}'s"
            )
        });
    }
    claim_field_names(documents, at, scope, form, warnings);
}

/// §12.7.4.2's collision test over one source's fields.
fn claim_field_names(
    documents: &[Document],
    at: usize,
    scope: &Scope<'_>,
    form: &mut Gathered,
    warnings: &mut Vec<Warning>,
) {
    let source = scope.source(at);
    let Some(document) = documents.get(at) else {
        return;
    };
    for field in fields_of(document) {
        if field.signed {
            warnings.push(Warning {
                source,
                page: None,
                detail: format!(
                    "§12.8.1: the signature on field {} was computed over this source's bytes, \
                     which the merged file is not, so the field crosses without its /V and the \
                     output states no signature",
                    printable(&field.name)
                ),
            });
        }
        let Some((first_at, first)) = form.claimed.get(&field.name) else {
            form.claimed.insert(field.name.clone(), (at, field));
            continue;
        };
        let Some(first_document) = documents.get(*first_at) else {
            continue;
        };
        let same = first.kind == field.kind
            && same_value(first_document, &first.value, document, &field.value, 0).unwrap_or(false)
            && same_value(first_document, &first.default, document, &field.default, 0)
                .unwrap_or(false);
        if same {
            warnings.push(Warning {
                source,
                page: None,
                detail: format!(
                    "§12.7.4.2: source {} states the same fully qualified field name {} with the \
                     same /FT, /V and /DV, which the clause permits — the two are \
                     representations of one field and the merged form holds both",
                    scope.source(*first_at),
                    printable(&field.name)
                ),
            });
        } else if *first_at == at {
            // **The collision is inside one source, and the merge did not make it.** §12.7.4.2
            // binds the document that holds both fields, and this document already held them;
            // carrying what the producer wrote is RFC 0002 section 11.1's whole premise, and
            // refusing here would decline to merge a file every reader opens. Named, not
            // refused. `issue15096.pdf` is the corpus witness.
            warnings.push(Warning {
                source,
                page: None,
                detail: format!(
                    "§12.7.4.2: this source itself states the fully qualified field name {} twice \
                     with a different /FT, /V or /DV, which the clause forbids one document to \
                     do; the merge carries what the producer wrote rather than inventing an \
                     answer",
                    printable(&field.name)
                ),
            });
        } else {
            form.collisions.push(format!(
                "{} (sources {} and {source})",
                printable(&field.name),
                scope.source(*first_at)
            ));
        }
    }
}

/// Table 224's `/DR`, unioned per §7.8.3 category with the first source winning a name.
///
/// §12.7.4.3 makes `/DR` matter only where an appearance stream is *constructed* — a field
/// whose value "is not known until viewing time" — and every appearance the sources hold
/// crosses byte for byte, so a losing resource name costs a regenerated appearance rather than a
/// mark. That is why this warns instead of refusing.
fn merge_default_resources(
    merge: &mut Merge<'_>,
    at: usize,
    form: &Dictionary,
    source: usize,
    into: &mut Vec<(Name, Dictionary)>,
    warnings: &mut Vec<Warning>,
) {
    let Some(document) = merge.documents.get(at) else {
        return;
    };
    let stated = document.get_key(form, "DR");
    let Some(stated) = stated.as_dict() else {
        return;
    };
    let categories: Vec<(Name, Dictionary)> = stated
        .iter()
        .filter_map(|(category, value)| {
            document
                .resolve(value)
                .as_dict()
                .map(|entries| (category.clone(), entries.clone()))
        })
        .collect();
    for (category, entries) in categories {
        let position = into
            .iter()
            .position(|(known, _)| known.as_bytes() == category.as_bytes());
        let mut merged = match position {
            Some(position) => into.get(position).map(|(_, entries)| entries.clone()),
            None => None,
        }
        .unwrap_or_default();
        for (name, value) in entries.iter() {
            if merged.get_by_name(name).is_some() {
                warnings.push(Warning {
                    source,
                    page: None,
                    detail: format!(
                        "Table 224: the form's default resources already name /{}/{} and an \
                         earlier source's wins; a field of this source whose /DA names it draws \
                         with the earlier one only where an appearance is constructed (§12.7.4.3)",
                        printable(category.as_bytes()),
                        printable(name.as_bytes())
                    ),
                });
                continue;
            }
            let carried = merge.carry(at, value, 0);
            merged.insert(name.clone(), carried);
        }
        match position {
            Some(position) => {
                if let Some(slot) = into.get_mut(position) {
                    slot.1 = merged;
                }
            }
            None => into.push((category, merged)),
        }
    }
}

/// §7.9.6's name trees, merged: the `/Names` dictionary and §12.3.2.4's catalog `/Dests`.
///
/// One root node holding one `/Names` array per category, sorted by key bytes — which is the
/// clause's own order, "[s]horter keys shall appear before longer ones beginning with the same
/// byte sequence" being what `Ord` on a byte string already does — and legal because "[i]f the
/// root node has a Names entry, it shall be the only node in the tree".
fn merge_name_trees(
    merge: &mut Merge<'_>,
    scope: &Scope<'_>,
    warnings: &mut Vec<Warning>,
) -> (Option<Object>, Option<Object>) {
    let documents = merge.documents;
    let mut trees: Vec<(&'static str, BTreeMap<Vec<u8>, Object>)> = Vec::new();
    let mut catalog_dictionary: BTreeMap<Vec<u8>, Object> = BTreeMap::new();

    for category in NAME_TREES {
        let mut merged: BTreeMap<Vec<u8>, Object> = BTreeMap::new();
        let mut taken: BTreeSet<Vec<u8>> = BTreeSet::new();
        for at in scope.contributing {
            let source = scope.source(*at);
            let Some(document) = documents.get(*at) else {
                continue;
            };
            // `/Dests` was renamed before the walk began, because its references are chased;
            // every other category is renamed here, where nothing depends on the answer.
            let dests = category == "Dests";
            if dests {
                for (key, value) in catalog_dests(document) {
                    let key = final_key(merge, *at, &key);
                    let carried = merge.carry(*at, &value, 0);
                    catalog_dictionary.entry(key).or_insert(carried);
                }
            }
            for (key, value) in tree_entries(document, category) {
                let key = if dests {
                    final_key(merge, *at, &key)
                } else if taken.contains(&key) {
                    let to = free_key(&key, &taken);
                    warnings.push(Warning {
                        source,
                        page: None,
                        detail: format!(
                            "§7.9.6: /Names /{category} already holds the key {} and \"[t]he keys \
                             contained within the various nodes' Names entries shall not \
                             overlap\", so this source's became {}; a reference stating that key \
                             by name is not rewritten, because this program does not know what \
                             states it",
                            printable(&key),
                            printable(&to)
                        ),
                    });
                    to
                } else {
                    key
                };
                taken.insert(key.clone());
                let carried = merge.carry(*at, &value, 0);
                merged.entry(key).or_insert(carried);
            }
        }
        if !merged.is_empty() {
            trees.push((category, merged));
        }
    }

    let names = if trees.is_empty() {
        None
    } else {
        let mut out = Dictionary::new();
        for (category, entries) in trees {
            let mut array = Vec::new();
            for (key, value) in entries {
                array.push(Object::String(key.as_slice().into()));
                array.push(value);
            }
            let mut root = Dictionary::new();
            root.insert(Name::new(&b"Names"[..]), Object::Array(array));
            out.insert(Name::new(category.as_bytes()), Object::Dictionary(root));
        }
        Some(Object::Dictionary(out))
    };
    let dests = (!catalog_dictionary.is_empty()).then(|| {
        let mut out = Dictionary::new();
        for (key, value) in catalog_dictionary {
            out.insert(Name::new(key), value);
        }
        Object::Dictionary(out)
    });
    (names, dests)
}

/// The name a destination key ended up with in the merged document.
fn final_key(merge: &Merge<'_>, at: usize, key: &[u8]) -> Vec<u8> {
    merge
        .renames
        .get(at)
        .and_then(|renames| renames.get(key))
        .cloned()
        .unwrap_or_else(|| key.to_vec())
}

/// §12.4.2's page labels, one entry per output page.
///
/// `None` where no contributing document states any, which is most of them: a merged document
/// that labels nothing is a merged document with no `/PageLabels`, exactly as its sources were.
fn merge_page_labels(merge: &mut Merge<'_>, order: &[Placement]) -> Option<Object> {
    let documents = merge.documents;
    let labels: Vec<PageLabels> = documents.iter().map(PageLabels::read).collect();
    if labels.iter().all(PageLabels::is_empty) {
        return None;
    }
    let mut nums = Vec::new();
    for (position, place) in order.iter().enumerate() {
        let (at, index) = (&place.at, &place.page);
        let mut entry = Dictionary::new();
        // "There is no default numbering style; if no S entry is present, page labels shall
        // consist solely of a label prefix with no numeric portion."
        if let Some(label) = labels.get(*at).and_then(|labels| labels.label(*index)) {
            entry.insert(Name::new(&b"P"[..]), text_string(&label));
        } else {
            // A page out of a document that labelled nothing. The standard says what such a
            // page's index is and nothing about what it is called, so this writes the decimal
            // number it had where it came from — a documented choice, and the one answer that
            // keeps it out of the preceding source's labelling range.
            entry.insert(Name::new(&b"S"[..]), Object::Name(Name::new(&b"D"[..])));
            entry.insert(
                Name::new(&b"St"[..]),
                Object::Integer(i64::try_from(index.saturating_add(1)).unwrap_or(1)),
            );
        }
        nums.push(Object::Integer(i64::try_from(position).unwrap_or(i64::MAX)));
        nums.push(Object::Dictionary(entry));
    }
    let mut root = Dictionary::new();
    root.insert(Name::new(&b"Nums"[..]), Object::Array(nums));
    Some(Object::Dictionary(root))
}

/// The merged catalog, and the warning naming every construct left behind.
fn build_catalog(
    merge: &mut Merge<'_>,
    tree: ObjectId,
    reconciled: &Reconciled,
    scope: &Scope<'_>,
    warnings: &mut Vec<Warning>,
) -> Object {
    let documents = merge.documents;
    let mut root = Dictionary::new();
    root.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Catalog"[..])),
    );
    root.insert(Name::new(&b"Pages"[..]), Object::Reference(tree));
    for key in FIRST_WINS {
        for at in scope.contributing {
            let Some(value) = catalog_entry(documents.get(*at), key) else {
                continue;
            };
            let carried = merge.carry(*at, &value, 0);
            root.insert(Name::new(key.as_bytes()), carried);
            break;
        }
    }
    for (key, value) in [
        ("OCProperties", reconciled.optional_content.clone()),
        ("AcroForm", reconciled.form.clone()),
        ("Names", reconciled.names.clone()),
        ("Dests", reconciled.dests.clone()),
        ("PageLabels", reconciled.page_labels.clone()),
    ] {
        if let Some(value) = value {
            root.insert(Name::new(key.as_bytes()), value);
        }
    }
    if let Some(outlines) = reconciled.outlines {
        root.insert(Name::new(&b"Outlines"[..]), Object::Reference(outlines));
    }
    // §14.7.2 locates the whole construct: the structure tree root is "located by means of the
    // StructTreeRoot entry in the document catalog dictionary".
    if let Some((structure, mark_info)) = merge
        .structure
        .as_ref()
        .map(|carry| (carry.root(), carry.mark_info()))
    {
        root.insert(
            Name::new(&b"StructTreeRoot"[..]),
            Object::Reference(structure),
        );
        if let Some(flags) = mark_info {
            root.insert(Name::new(&b"MarkInfo"[..]), flags);
        }
    }
    if let Intents::Catalog(array) = &reconciled.intents {
        let carried = scope
            .contributing
            .first()
            .map_or(Object::Null, |at| merge.carry(*at, array, 0));
        if !carried.is_null() {
            root.insert(Name::new(&b"OutputIntents"[..]), carried);
        }
    }

    for at in scope.contributing {
        let source = scope.source(*at);
        let mut left_behind: Vec<&str> = NOT_CARRIED
            .into_iter()
            .filter(|key| catalog_entry(documents.get(*at), key).is_some())
            .collect();
        if documents
            .get(*at)
            .is_some_and(|document| document.trailer().get("Info").is_some())
        {
            left_behind.push("Info");
        }
        if !left_behind.is_empty() {
            warnings.push(Warning {
                source,
                page: None,
                detail: format!(
                    "this source states /{} and the written document carries none of them; \
                     /Info is a deliberate omission whose reason is in this module",
                    left_behind.join(", /")
                ),
            });
        }
    }
    Object::Dictionary(root)
}
