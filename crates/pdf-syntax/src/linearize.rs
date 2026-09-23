//! A whole file written as ISO 32000-2 Annex F's linearised PDF, and a linearised file recognised.
//!
//! # What this is
//!
//! [`crate::serialize`] writes a finished [`Assembly`] in the assembly's own order. This module
//! writes the same assembly in the order Annex F prescribes, with the two additions §F.1 names —
//! "[r]ules for the ordering of objects in the PDF file" and "hint tables, that enable efficient
//! navigation within the document". It is structure and nothing else: every object's value is the
//! one the assembly holds, a stream's data is the assembly's bytes, and the only dictionaries this
//! module changes are page objects, which §F.3.7 and §F.3.10 require to state their inheritable
//! attributes themselves (§7.7.3.4's four entries, copied down from the nearest ancestor that
//! states each).
//!
//! # The file, part by part
//!
//! §F.3.1 divides a linearised file into eleven parts "[f]or pedagogical reasons", and
//! [`serialize_linearized`] writes them in this order:
//!
//! | part | what | F.x |
//! |---|---|---|
//! | 1 | §7.5.2's header | §F.3.2 |
//! | 2 | the linearization parameter dictionary | §F.3.3 |
//! | 3 | the first-page cross-reference section and trailer | §F.3.4 |
//! | 5 | the primary hint stream | §F.3.6 |
//! | 4 | the catalog and the objects §F.3.5 names | §F.3.5 |
//! | 6 | the first page's objects | §F.3.7 |
//! | 7 | every other page's objects, in page order | §F.3.8 |
//! | 8 | the objects two or more of those pages share | §F.3.9 |
//! | 9 | everything else, grouped by §F.3.10's categories | §F.3.10 |
//! | 11 | the main cross-reference section and trailer | §F.3.11 |
//!
//! Part 5 before part 4 is §F.3.6's own permission — "the order of this part and the document
//! catalog dictionary and document-level objects, shown as part 4, may be reversed" — and it is
//! taken because of §F.4.1's rule for positions: "a position greater than the hint stream offset
//! shall have the hint stream length added to it". Every position a hint table states is in part 6
//! or later, so with part 4 between the hint stream and the first of them, no stated position is
//! ever *equal* to the hint stream's offset, which is the one case that sentence does not decide.
//! Part 10, the overflow hint stream, is not written: §F.3.6 makes it optional ("[t]he overflow hint
//! stream, part 10, is optional") and every table fits in the primary one.
//!
//! **No object stream and no cross-reference stream is written.** §F.3.1 and §F.3.4 permit both —
//! "Cross-reference streams … may be used in place of traditional cross-reference tables" — and
//! grant them as permissions; §F.3.1's conditions on object streams bind only "[i]n linearized
//! files containing object streams". This writer's files contain none, so the conditions are met
//! by not arising, and §7.5.4's classic table is what both sections are.
//!
//! # Every offset is correct by construction
//!
//! The parameter dictionary states the file's length and the hint stream's offset, and it is the
//! second thing in the file; the first-page trailer states the main cross-reference section's
//! offset, and it is the third. So the lengths of parts 2, 3 and 5 decide every offset in the
//! file, and parts 2 and 3 contain offsets. [`serialize_linearized`] therefore renders every
//! object once, with its final number, and then **computes the layout to a fixed point before a
//! byte is written**: it lays the file out with the three variable parts at their current lengths,
//! renders the three from that layout, and repeats until none of them changed length. Each value
//! grows with the lengths and each length grows with the values, so the lengths only increase, and
//! they stop within a few iterations. Nothing is written until they have stopped, so nothing is
//! patched after the write and no field is padded to a guessed width. ADR 1293.
//!
//! The cost is memory: the rendered objects are held until the layout is known. A carried
//! stream's data is the assembly's `Arc<[u8]>` and is not copied; a recompressed one was already
//! in memory under [`Streams::Recompress`]'s own stated cost.
//!
//! # Reading one
//!
//! [`state`] is the reader's half, and it is §F.1's and Table F.1's rule rather than an
//! accelerator: "Incremental update shall still be permitted, but the resulting PDF is no longer
//! linearized and subsequently shall be treated as ordinary PDF", and a `/L` that does not match
//! "indicates that the file is not linearized and shall be treated as ordinary PDF file, ignoring
//! linearization information". This tree reads every file through §7.5's cross-reference chain,
//! which is what "ordinary PDF" means, so the hint tables are never trusted for anything; what
//! [`state`] answers is whether the file still *is* linearised, so that a caller can say so.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::Write;
use std::sync::Arc;

use crate::Document;
use crate::object::{Dictionary, Name, Object, ObjectId, Stream};
use crate::serialize::{
    Assembly, FREE_FOREVER, MAX_TABLE_OFFSET, SerializeError, Streams, Written, catalog_of, header,
};
use crate::version::Version;
use crate::write;
use crate::xref::Location;

/// The deepest a value tree is walked for references, the serializer's own bound.
const MAX_VALUE_DEPTH: usize = 257;

/// The most `/Parent` hops followed when a page's inheritable attributes are pushed down.
///
/// §7.7.3.4's inheritance runs up the page tree, and a tree deeper than this is one no reader in
/// this tree walks either.
const MAX_ANCESTORS: usize = 256;

/// The most times the layout is recomputed before the writer gives up.
///
/// The lengths only grow, and each can grow by a digit at most a handful of times, so this is a
/// ceiling on a loop that stops within three or four iterations on every input; reaching it would
/// be a defect in this module, which [`LinearizeError::Unsettled`] names rather than a hang.
const MAX_LAYOUT_PASSES: usize = 32;

/// §F.3.3: "The linearization parameter dictionary shall be entirely contained within the first
/// 1024 bytes of the PDF file."
const PARAMETERS_WITHIN: u64 = 1024;

/// §7.7.3.4's inheritable page attributes, which §F.3.10 requires "pushed down and replicated in
/// each of the leaf page objects".
const INHERITABLE: [&str; 4] = ["Resources", "MediaBox", "CropBox", "Rotate"];

/// Table F.3 item 13's denominator.
///
/// Item 5's numerator states where in a page's content stream a shared object is first
/// referenced, and this writer does not read content streams — that is the whole of what "emits
/// structure and never content" means. So the fraction is stated at the one precision a writer
/// that has not read the stream can state truthfully: a denominator of 1, a numerator of 0 for an
/// object the page reaches through its `/Resources` or `/Contents` (it is referenced from the
/// content stream, somewhere in its only portion), and a numerator of `d + 1` for one it reaches
/// only through annotations and other entries — "[a] value of d + 1 or greater shall indicate that
/// the shared object is needed after those objects". ADR 1293.
const DENOMINATOR: u64 = 1;

/// How one page is to be linearised: which objects are its pages, and which of them opens first.
///
/// The assembly's own numbering, because a page is an object the caller copied or placed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Every page object, in §7.7.3.2's page order.
    pub pages: Vec<ObjectId>,
    /// Which of them is §F.3.7's first page, counted from 0.
    ///
    /// Page 0 unless the catalog's `/OpenAction` names another, which §F.3.7 makes the first page
    /// and places in part 6 in page 0's stead. Deciding which page an `/OpenAction` names is a
    /// reading of §12.3.2 and §12.6.4.2, which is the caller's; this module places whatever page
    /// it is given.
    pub first_page: usize,
}

/// Why an assembly could not be written as a linearised file.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LinearizeError {
    /// The plan names no page.
    ///
    /// Table F.1's `/O` is "(Required) The object number of the first page's page object", so a
    /// file with no page has no parameter dictionary to write.
    #[error("Annex F: a linearised file has a first page, and this plan names no page")]
    NoPages,
    /// The first page is not one of the plan's pages.
    #[error("Annex F: the first page is page {first}, and the plan names {count}")]
    FirstPageOutOfRange {
        /// The index asked for.
        first: usize,
        /// How many pages there are.
        count: usize,
    },
    /// A page the plan names is not a dictionary the assembly holds.
    #[error("Annex F: object {} is named as a page and is not a dictionary", .id.number)]
    NotAPage {
        /// The number named.
        id: ObjectId,
    },
    /// A page the plan names twice.
    ///
    /// §F.3.8's page sections are "ordered by page number" and each page object "shall be the
    /// first object in each section"; one object cannot begin two sections.
    #[error("Annex F: object {} is named as two pages, and a page object begins one section", .id.number)]
    PageTwice {
        /// The number named twice.
        id: ObjectId,
    },
    /// More objects, with the parameter dictionary and the hint stream, than `u32` numbers.
    #[error("Annex F: this assembly holds more objects than an object number can count")]
    TooManyObjects,
    /// The layout did not reach a fixed point, which would be a defect in this module.
    #[error("Annex F: the linearised layout did not settle in {MAX_LAYOUT_PASSES} passes")]
    Unsettled,
    /// The parameter dictionary would not fit where §F.3.3 requires it.
    #[error("F.3.3: the linearization parameter dictionary would end at byte {end}, past 1024")]
    ParametersTooLate {
        /// Where it would end.
        end: u64,
    },
    /// The serializer's own refusals: no catalog, an unplaced slot, an offset too large.
    #[error(transparent)]
    Serialize(#[from] SerializeError),
}

/// What [`serialize_linearized`] wrote, beyond [`Written`]'s tallies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Linearized {
    /// The serializer's tallies, over the linearised file.
    pub written: Written,
    /// The keys of Table F.2 the primary hint stream's dictionary states, in the order written.
    pub hint_tables: Vec<&'static str>,
    /// How many objects are in §F.3.9's shared objects section (part 8).
    pub shared_objects: usize,
}

/// Where a linearised file stands, as a reader finds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// The first object in the file is not a linearization parameter dictionary.
    Ordinary,
    /// It is one, and its `/L` is the file's length.
    Linearized(Parameters),
    /// It is one, and its `/L` is not the file's length.
    ///
    /// Table F.1: "A mismatch indicates that the file is not linearized and shall be treated as
    /// ordinary PDF file, ignoring linearization information." §F.1 names the ordinary way this
    /// happens — "Incremental update shall still be permitted, but the resulting PDF is no longer
    /// linearized and subsequently shall be treated as ordinary PDF".
    NoLongerLinearized {
        /// What the dictionary states.
        parameters: Parameters,
        /// How long the file actually is.
        length: u64,
    },
}

/// Table F.1's entries, as a file states them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parameters {
    /// `/L`: the length the file had when it was linearised.
    pub length: u64,
    /// `/H`'s first pair: the primary hint stream's offset and length.
    pub primary_hints: (u64, u64),
    /// `/H`'s second pair, where there is an overflow hint stream.
    pub overflow_hints: Option<(u64, u64)>,
    /// `/O`: the first page's page object number.
    pub first_page_object: u32,
    /// `/E`: the offset of the end of the first page.
    pub end_of_first_page: u64,
    /// `/N`: the number of pages.
    pub pages: u64,
    /// `/T`: the main cross-reference table's first entry, less one byte.
    pub main_cross_reference: u64,
    /// `/P`: the first page's number, 0 by default.
    pub first_page: u64,
}

/// Whether a document is linearised, per §F.3.3 and Table F.1.
///
/// The parameter dictionary is "the first object in the body of the PDF file", "entirely
/// contained within the first 1024 bytes", so the object the cross-reference chain places lowest
/// in the file is asked, and only where it begins inside those bytes. A dictionary without every
/// required entry of Table F.1 is not one.
#[must_use]
pub fn state(document: &Document) -> State {
    let xref = document.xref();
    let first = xref
        .object_numbers()
        .filter_map(|number| match xref.location(number) {
            Some(Location::Offset(at)) => Some((at, number)),
            _ => None,
        })
        .min();
    let Some((at, number)) = first else {
        return State::Ordinary;
    };
    if u64::try_from(at).unwrap_or(u64::MAX) >= PARAMETERS_WITHIN {
        return State::Ordinary;
    }
    let object = document.get(ObjectId::new(number, 0));
    let Some(parameters) = object.as_dict().and_then(parameters_of) else {
        return State::Ordinary;
    };
    let length = u64::try_from(document.bytes().len()).unwrap_or(u64::MAX);
    if parameters.length == length {
        State::Linearized(parameters)
    } else {
        State::NoLongerLinearized { parameters, length }
    }
}

/// Table F.1's entries out of a dictionary, or `None` where it is not a parameter dictionary.
fn parameters_of(dict: &Dictionary) -> Option<Parameters> {
    dict.get("Linearized")?.as_number()?;
    let integer = |key: &str| {
        dict.get(key)
            .and_then(Object::as_integer)
            .and_then(|value| u64::try_from(value).ok())
    };
    let hints: Vec<u64> = dict
        .get("H")?
        .as_array()?
        .iter()
        .map(|item| {
            item.as_integer()
                .and_then(|value| u64::try_from(value).ok())
        })
        .collect::<Option<Vec<u64>>>()?;
    let (primary_hints, overflow_hints) = match hints.as_slice() {
        [offset, length] => ((*offset, *length), None),
        [offset, length, second, second_length] => {
            ((*offset, *length), Some((*second, *second_length)))
        }
        _ => return None,
    };
    Some(Parameters {
        length: integer("L")?,
        primary_hints,
        overflow_hints,
        first_page_object: u32::try_from(integer("O")?).ok()?,
        end_of_first_page: integer("E")?,
        pages: integer("N")?,
        main_cross_reference: integer("T")?,
        first_page: integer("P").unwrap_or(0),
    })
}

/// How a reference reached an object, where that decides §F.3.7's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Via {
    /// An ordinary entry.
    Plain,
    /// A font descriptor's `/FontFile` or `/FontFile2`, where the descriptor's Nonsymbolic flag
    /// is set: §F.3.7 (g)'s substitutable font program, which a reader may draw without.
    FontFile,
    /// An annotation's `/AP`: §F.3.7 (a)'s "[i]nformation required to draw the annotation may be
    /// deferred until later since annotations are always drawn on top of (hence after) the
    /// contents."
    Appearance,
}

/// One reference from one object to another, both as indices into the assembly.
#[derive(Debug, Clone, Copy)]
struct Edge {
    /// The object referred to.
    to: usize,
    /// How.
    via: Via,
}

/// Where a page's walk had got to when it reached an object: §F.3.7's lettered order, which is
/// also what Table F.4 item 5's numerator is derived from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Stage {
    /// The page object itself.
    Page,
    /// (a) "The Annots array and all annotation dictionaries".
    Annots,
    /// (b) "The B (beads) array and all bead dictionaries".
    Beads,
    /// (c) and (d): "The Resources dictionary" and then the resource objects.
    Resources,
    /// (e) "The page contents ( Contents )".
    Contents,
    /// Every other entry of the page dictionary.
    Other,
    /// What (a) defers: the annotations' appearances.
    Appearance,
    /// (f): the image `XObject`s, "in the order that they are first referenced".
    Images,
    /// (g): the `/FontFile` streams.
    FontFiles,
}

impl Stage {
    /// Table F.4 item 5's numerator for a shared object first reached at this stage, under
    /// [`DENOMINATOR`].
    fn numerator(self) -> u64 {
        match self {
            Self::Resources | Self::Contents | Self::Images | Self::FontFiles | Self::Page => 0,
            Self::Annots | Self::Beads | Self::Other | Self::Appearance => {
                DENOMINATOR.saturating_add(1)
            }
        }
    }
}

/// The assembly's objects, resolved once, and the references between them.
#[derive(Debug)]
struct Graph {
    /// Every object, with references in the assembly's numbering; index `i` is object `i + 1`.
    objects: Vec<Object>,
    /// Each object's references, in `Dictionary`'s key order, which is sorted.
    edges: Vec<Vec<Edge>>,
    /// Whether an object is an image `XObject`.
    image: Vec<bool>,
    /// Whether an object is an embedded file stream (§7.11.4), §F.3.10's own category.
    embedded: Vec<bool>,
    /// Whether an object is a §14.12 `DPart` node, which §F.3.7 excludes from a page's objects.
    dpart: Vec<bool>,
}

impl Graph {
    /// The object at an index, as a dictionary where it is one or a stream.
    fn dict(&self, index: usize) -> Option<&Dictionary> {
        self.objects.get(index).and_then(Object::as_dict)
    }

    /// The index a reference names, where the assembly holds it.
    fn index_of(&self, value: &Object) -> Option<usize> {
        let id = value.as_reference()?;
        let index = usize::try_from(id.number).ok()?.checked_sub(1)?;
        (index < self.objects.len()).then_some(index)
    }

    /// The references inside one value, as edges.
    fn refs(&self, value: Option<&Object>) -> Vec<Edge> {
        let mut out = Vec::new();
        let mut ignored = Vec::new();
        if let Some(value) = value {
            collect(
                value,
                Via::Plain,
                self.objects.len(),
                0,
                &mut out,
                &mut ignored,
            );
        }
        out
    }

    /// The indices one value refers to, directly.
    fn targets(&self, value: Option<&Object>) -> Vec<usize> {
        self.refs(value).into_iter().map(|edge| edge.to).collect()
    }

    /// One entry of the object at an index.
    fn entry(&self, index: usize, key: &str) -> Option<&Object> {
        self.dict(index).and_then(|dict| dict.get(key))
    }

    /// An entry's value, through one reference where it is indirect.
    fn entry_resolved(&self, index: usize, key: &str) -> Option<&Object> {
        let value = self.entry(index, key)?;
        match self.index_of(value) {
            Some(target) => self.objects.get(target),
            None => Some(value),
        }
    }
}

/// Every reference inside `value`, with how each was reached, and the embedded file streams it
/// names.
fn collect(
    value: &Object,
    via: Via,
    count: usize,
    depth: usize,
    out: &mut Vec<Edge>,
    embedded: &mut Vec<usize>,
) {
    if depth >= MAX_VALUE_DEPTH {
        return;
    }
    let deeper = depth.saturating_add(1);
    match value {
        Object::Reference(id) => {
            if let Some(index) = usize::try_from(id.number)
                .ok()
                .and_then(|number| number.checked_sub(1))
                .filter(|index| *index < count)
            {
                out.push(Edge { to: index, via });
            }
        }
        Object::Array(items) => {
            for item in items {
                collect(item, via, count, deeper, out, embedded);
            }
        }
        Object::Dictionary(dict) => collect_dict(dict, via, count, deeper, false, out, embedded),
        Object::Stream(stream) => {
            collect_dict(&stream.dict, via, count, deeper, true, out, embedded);
        }
        _ => {}
    }
}

/// [`collect`] over a dictionary's values.
///
/// A stream's `/Length` is not followed, for the reason [`crate::serialize`] writes it: the
/// writer re-derives it as a direct integer from the bytes, so the output refers to nothing
/// through it.
fn collect_dict(
    dict: &Dictionary,
    via: Via,
    count: usize,
    depth: usize,
    stream: bool,
    out: &mut Vec<Edge>,
    embedded: &mut Vec<usize>,
) {
    // Table 123's Nonsymbolic flag is bit 6, which §F.3.7 (g) names as what makes a font
    // substitutable.
    let nonsymbolic = dict
        .get("Flags")
        .and_then(Object::as_integer)
        .is_some_and(|flags| flags & 32 != 0);
    for (key, item) in dict.iter() {
        let key = key.as_bytes();
        if stream && key == b"Length" {
            continue;
        }
        if key == b"EF"
            && let Some(files) = item.as_dict()
        {
            for (_, file) in files.iter() {
                let mut found = Vec::new();
                let mut nested = Vec::new();
                collect(file, via, count, depth, &mut found, &mut nested);
                embedded.extend(found.iter().map(|edge| edge.to));
            }
        }
        let item_via = match (via, key) {
            (Via::Plain, b"FontFile" | b"FontFile2") if nonsymbolic => Via::FontFile,
            (Via::Plain, b"AP") => Via::Appearance,
            _ => via,
        };
        collect(item, item_via, count, depth, out, embedded);
    }
}

/// A walk's visited set, reusable across walks without clearing.
///
/// A stamp per object rather than a set per walk, because a page walk runs once per page and a
/// cleared `Vec<bool>` per page would make linearising an `n`-page document of `m` objects cost
/// `n * m` before a single object was visited.
#[derive(Debug)]
struct Seen {
    /// The walk that last marked each object.
    stamp: Vec<u32>,
    /// The current walk.
    current: u32,
}

impl Seen {
    /// A set over `count` objects.
    fn new(count: usize) -> Self {
        Self {
            stamp: vec![0; count],
            current: 0,
        }
    }

    /// Starts a new walk, forgetting every mark.
    fn next(&mut self) {
        self.current = self.current.wrapping_add(1);
        if self.current == 0 {
            self.stamp.iter_mut().for_each(|stamp| *stamp = 0);
            self.current = 1;
        }
    }

    /// Marks an object, answering whether it was already marked.
    fn mark(&mut self, index: usize) -> bool {
        match self.stamp.get_mut(index) {
            Some(stamp) if *stamp == self.current => true,
            Some(stamp) => {
                *stamp = self.current;
                false
            }
            None => true,
        }
    }

    /// Whether an object is marked.
    fn has(&self, index: usize) -> bool {
        self.stamp
            .get(index)
            .is_none_or(|stamp| *stamp == self.current)
    }
}

/// §F.3.10's categories that have a hint table, in the order they are placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    /// "The outline hierarchy" — Table F.2's `/O`.
    Outline,
    /// "Thread information dictionaries" — `/A`.
    Threads,
    /// "Named destinations" — `/E`.
    Destinations,
    /// "The document information dictionary" — `/I`.
    Information,
    /// "The interactive form field hierarchy" — `/V`.
    Form,
    /// "(PDF 1.3) The logical structure hierarchy" — `/C`.
    Structure,
    /// "(PDF 1.5) The renditions name tree hierarchy" — `/R`.
    Renditions,
    /// §12.4.2's page labels — `/L`.
    Labels,
}

impl Category {
    /// Table F.2's key.
    fn key(self) -> &'static str {
        match self {
            Self::Outline => "O",
            Self::Threads => "A",
            Self::Destinations => "E",
            Self::Information => "I",
            Self::Form => "V",
            Self::Structure => "C",
            Self::Renditions => "R",
            Self::Labels => "L",
        }
    }

    /// Whether §F.4.6 makes this an extended generic table.
    fn extended(self) -> bool {
        matches!(self, Self::Form | Self::Structure | Self::Renditions)
    }
}

/// A run of consecutive objects in the file's order: where it starts and how many.
#[derive(Debug, Clone, Copy, Default)]
struct Span {
    /// Its first object's position in [`Layout::sequence`].
    start: usize,
    /// How many objects.
    count: usize,
}

impl Span {
    /// The position after its last object.
    fn end(self) -> usize {
        self.start.saturating_add(self.count)
    }
}

/// A category's group, for its generic or extended generic table.
#[derive(Debug, Clone)]
struct Group {
    /// Which table.
    category: Category,
    /// Its objects.
    span: Span,
    /// Its shared object identifiers, for an extended table.
    shared: Vec<u64>,
}

/// One page's entry in the page offset hint table.
#[derive(Debug, Clone, Default)]
struct PageEntry {
    /// Its section.
    span: Span,
    /// Its content streams' positions in the file's order, where they are in its section.
    contents: Option<(usize, usize)>,
    /// Its shared object references: identifier and numerator.
    shared: Vec<(u64, u64)>,
}

/// Everything about the file that does not depend on where the variable parts end.
#[derive(Debug)]
struct Layout {
    /// Every object in file order, as assembly indices: parts 4, 6, 7, 8 and 9.
    sequence: Vec<usize>,
    /// Each position's rendered object.
    rendered: Vec<Rendered>,
    /// Each position's final object number.
    numbers: Vec<u32>,
    /// How many objects are in parts 4 and 6, the first group.
    first_group: usize,
    /// How many are in parts 7, 8 and 9, the second group — `k` in §F.3.1's numbering.
    second_group: u32,
    /// Part 6.
    first_page: Span,
    /// Part 6's leading run of objects no other page uses, the shared object table's entry 0.
    first_private: Span,
    /// The page offset table's entries, in the file's order of pages.
    pages: Vec<PageEntry>,
    /// Part 6's objects that another page also uses, each its own shared object group.
    first_shared: Span,
    /// Part 8.
    shared: Span,
    /// The categories' groups.
    groups: Vec<Group>,
    /// The thumbnail table, where there are thumbnails.
    thumbnails: Option<Thumbnails>,
    /// The embedded file stream groups.
    embedded: Vec<(usize, Span)>,
    /// The final number of the catalog.
    root: u32,
    /// The final number of the information dictionary.
    info: Option<u32>,
    /// Table F.1's `/O`.
    first_page_object: u32,
    /// Table F.1's `/N`.
    page_count: usize,
    /// Table F.1's `/P`.
    first_page_number: usize,
    /// §14.4's identifier: a digest of every object as written.
    identifier: [u8; 16],
    /// Table F.3 item 11 and Table F.10 item 6: the width of every shared object identifier.
    identifier_bits: u32,
}

/// §F.4.4's table: which pages have a thumbnail, and where each one's objects are.
#[derive(Debug, Clone)]
struct Thumbnails {
    /// For each page with a thumbnail, in page number order: the page and its objects.
    images: Vec<(usize, Span)>,
    /// The thumbnail shared objects section.
    shared: Span,
}

/// One object as it will be written, less nothing: head, carried data, tail.
#[derive(Debug, Clone)]
struct Rendered {
    /// `n 0 obj`, the value, and for a stream everything up to its data.
    head: Vec<u8>,
    /// A stream's data, carried as the assembly holds it.
    data: Option<Arc<[u8]>>,
    /// What follows the data.
    tail: &'static [u8],
}

impl Rendered {
    /// Its length in the file, which is Table F.4 item 2's unit: an object "including object
    /// overhead".
    fn len(&self) -> u64 {
        let data = self.data.as_ref().map_or(0, |data| data.len());
        u64::try_from(
            self.head
                .len()
                .saturating_add(data)
                .saturating_add(self.tail.len()),
        )
        .unwrap_or(u64::MAX)
    }
}

/// Writes a finished assembly as an Annex F linearised file.
///
/// The parts are in the order this module's documentation tables, the first page is
/// `plan.first_page`, and every object is written outside any object stream with §7.5.4's
/// classic cross-reference tables.
///
/// # Errors
///
/// [`LinearizeError`]: a plan with no page, a first page out of range, a page that is not a
/// dictionary or is named twice, and every refusal [`crate::serialize::serialize`] makes.
pub fn serialize_linearized<W: Write>(
    assembly: &Assembly<'_>,
    version: Version,
    streams: Streams,
    plan: &Plan,
    out: &mut W,
) -> Result<Linearized, LinearizeError> {
    let root = catalog_of(assembly)?;
    let mut tally = Written::default();
    let mut objects = Vec::with_capacity(assembly.slots.len());
    for index in 0..assembly.slots.len() {
        let object = assembly
            .resolved(index, streams, &mut tally)
            .ok_or_else(|| {
                let number = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
                SerializeError::Unplaced {
                    id: ObjectId::new(number, 0),
                }
            })?;
        objects.push(object);
    }
    let pages = pages_of(plan, &objects)?;
    let root = usize::try_from(root.number)
        .ok()
        .and_then(|number| number.checked_sub(1))
        .filter(|index| *index < objects.len())
        .ok_or(SerializeError::RootNotADictionary { id: root })?;
    let info = assembly.info.and_then(|id| {
        usize::try_from(id.number)
            .ok()
            .and_then(|number| number.checked_sub(1))
            .filter(|index| *index < objects.len())
    });
    let trailer_roots: Vec<usize> = std::iter::once(root).chain(info).collect();
    let reachable_before = reachable(&objects, &trailer_roots);
    push_down(&mut objects, &pages);
    let reachable_after = reachable(&objects, &trailer_roots);
    // What only a node's attribute reached is no longer reachable once §F.3.10 has pushed it down,
    // and it is not written — the same reachability `optimize` prunes by, so that linearising the
    // output again finds nothing more to remove. An object nothing reached before either is kept:
    // that is a caller who asked for no pruning.
    let dropped: Vec<bool> = reachable_before
        .iter()
        .zip(&reachable_after)
        .map(|(before, after)| *before && !*after)
        .collect();

    let graph = graph_of(objects);
    let placement = place(&graph, root, info, &pages, plan.first_page, &dropped);
    let layout = lay_out(&graph, &placement, root, info, &mut tally)?;
    write_out(&layout, version, &mut tally, out)
}

/// The plan's pages as indices, checked.
fn pages_of(plan: &Plan, objects: &[Object]) -> Result<Vec<usize>, LinearizeError> {
    if plan.pages.is_empty() {
        return Err(LinearizeError::NoPages);
    }
    if plan.first_page >= plan.pages.len() {
        return Err(LinearizeError::FirstPageOutOfRange {
            first: plan.first_page,
            count: plan.pages.len(),
        });
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::with_capacity(plan.pages.len());
    for id in &plan.pages {
        let index = usize::try_from(id.number)
            .ok()
            .and_then(|number| number.checked_sub(1))
            .filter(|index| matches!(objects.get(*index), Some(Object::Dictionary(_))))
            .ok_or(LinearizeError::NotAPage { id: *id })?;
        if !seen.insert(index) {
            return Err(LinearizeError::PageTwice { id: *id });
        }
        out.push(index);
    }
    Ok(out)
}

/// The push-down: every page states §7.7.3.4's inheritable attributes itself, which §F.3.7 asks
/// of the first page and §F.3.10 of every page. §F.3.7:
///
/// > This page object shall explicitly specify all required attributes, such as Resources and
/// > MediaBox ; the attributes may not be inherited from ancestor page tree nodes.
///
/// Each of the four is copied from the nearest ancestor that states it, which is §7.7.3.4's own
/// rule for what the page already meant; the value is copied as the ancestor states it, so a
/// reference stays a reference to the same object. A page whose chain states one of them nowhere
/// is left without it rather than given a value the producer never wrote.
///
/// **Then every ancestor loses the four**, which is the reading of "pushed down" rather than
/// "copied down": §F.3.10 says the attributes "shall be pushed down and replicated in each of the
/// leaf page objects", and once every leaf states each attribute it inherited, a node's copy
/// decides nothing for any page. Removing it is what makes the tree say so. qpdf's checker reads
/// the sentence the same way, and refuses to check a file whose nodes still state them (ADR 1293).
fn push_down(objects: &mut [Object], pages: &[usize]) {
    let mut ancestors = BTreeSet::new();
    for page in pages {
        let mut parent = objects
            .get(*page)
            .and_then(Object::as_dict)
            .and_then(|dict| dict.get("Parent"))
            .and_then(Object::as_reference);
        for _ in 0..MAX_ANCESTORS {
            let Some(index) = parent
                .and_then(|id| usize::try_from(id.number).ok())
                .and_then(|number| number.checked_sub(1))
                .filter(|index| !pages.contains(index))
            else {
                break;
            };
            if !ancestors.insert(index) {
                break;
            }
            parent = objects
                .get(index)
                .and_then(Object::as_dict)
                .and_then(|dict| dict.get("Parent"))
                .and_then(Object::as_reference);
        }
    }
    for page in pages {
        let Some(Object::Dictionary(dict)) = objects.get(*page) else {
            continue;
        };
        let mut dict = dict.clone();
        let mut changed = false;
        for key in INHERITABLE {
            if dict.get(key).is_some() {
                continue;
            }
            let mut parent = dict.get("Parent").and_then(Object::as_reference);
            for _ in 0..MAX_ANCESTORS {
                let Some(node) = parent
                    .and_then(|id| usize::try_from(id.number).ok())
                    .and_then(|number| number.checked_sub(1))
                    .and_then(|index| objects.get(index))
                    .and_then(Object::as_dict)
                else {
                    break;
                };
                if let Some(value) = node.get(key) {
                    dict.insert(Name::new(key.as_bytes()), value.clone());
                    changed = true;
                    break;
                }
                parent = node.get("Parent").and_then(Object::as_reference);
            }
        }
        if changed && let Some(slot) = objects.get_mut(*page) {
            *slot = Object::Dictionary(dict);
        }
    }
    for node in ancestors {
        if let Some(Object::Dictionary(dict)) = objects.get_mut(node) {
            for key in INHERITABLE {
                dict.remove(key);
            }
        }
    }
}

/// Which objects a walk from `roots` reaches, as a flag per object.
fn reachable(objects: &[Object], roots: &[usize]) -> Vec<bool> {
    let count = objects.len();
    let mut reached = vec![false; count];
    let mut stack: Vec<usize> = roots.to_vec();
    while let Some(index) = stack.pop() {
        match reached.get_mut(index) {
            Some(flag) if !*flag => *flag = true,
            _ => continue,
        }
        let Some(object) = objects.get(index) else {
            continue;
        };
        let mut edges = Vec::new();
        let mut ignored = Vec::new();
        collect(object, Via::Plain, count, 0, &mut edges, &mut ignored);
        stack.extend(edges.iter().map(|edge| edge.to));
    }
    reached
}

/// The objects and every reference between them.
fn graph_of(objects: Vec<Object>) -> Graph {
    let count = objects.len();
    let mut edges = Vec::with_capacity(count);
    let mut embedded = vec![false; count];
    let mut image = vec![false; count];
    let mut dpart = vec![false; count];
    for (index, object) in objects.iter().enumerate() {
        let mut out = Vec::new();
        let mut files = Vec::new();
        collect(object, Via::Plain, count, 0, &mut out, &mut files);
        for file in files {
            if let Some(flag) = embedded.get_mut(file) {
                *flag = true;
            }
        }
        edges.push(out);
        if let Some(dict) = object.as_dict() {
            let named = |key: &str, value: &[u8]| {
                dict.get(key)
                    .and_then(Object::as_name)
                    .is_some_and(|name| name.as_bytes() == value)
            };
            if matches!(object, Object::Stream(_)) {
                if let Some(flag) = image.get_mut(index) {
                    *flag = named("Subtype", b"Image");
                }
                if named("Type", b"EmbeddedFile")
                    && let Some(flag) = embedded.get_mut(index)
                {
                    *flag = true;
                }
            }
            if named("Type", b"DPart")
                && let Some(flag) = dpart.get_mut(index)
            {
                *flag = true;
            }
        }
    }
    // An embedded file stream is a stream: an `/EF` naming anything else names something §7.11.4
    // does not call one, and it keeps its ordinary place.
    for (index, flag) in embedded.iter_mut().enumerate() {
        if !matches!(objects.get(index), Some(Object::Stream(_))) {
            *flag = false;
        }
    }
    Graph {
        objects,
        edges,
        image,
        embedded,
        dpart,
    }
}

/// Part 9's thumbnails: each page with one and its own objects, then the thumbnail shared objects.
type ThumbnailRuns = (Vec<(usize, Vec<usize>)>, Vec<usize>);

/// Where every object goes, before anything is numbered.
#[derive(Debug, Default)]
struct Placement {
    /// Part 4, catalog first.
    part4: Vec<usize>,
    /// Part 6's leading run: the first page's own objects, then the outline where it belongs here.
    first_private: Vec<usize>,
    /// Part 6's trailing run: objects the first page shares with another page.
    first_shared: Vec<usize>,
    /// Part 7: each other page's own objects, in page order, as (plan index, objects).
    sections: Vec<(usize, Vec<usize>)>,
    /// Part 8.
    shared: Vec<usize>,
    /// Part 9, in order, each with the category whose hint table describes it where one does.
    nine: Vec<(Option<Category>, Vec<usize>)>,
    /// Part 9's thumbnails: each page's thumbnail objects, then the thumbnail shared objects.
    thumbnails: Option<ThumbnailRuns>,
    /// Part 9's embedded file stream groups.
    embedded: Vec<Vec<usize>>,
    /// Where part 9's thumbnail and embedded file runs sit among its categories.
    thumbnails_at: usize,
    /// The same, for the embedded file streams.
    embedded_at: usize,
    /// The outline, where §F.3.7 puts it in part 6: its objects, within `first_private`.
    outline_in_first_page: Option<Vec<usize>>,
    /// Each page's walk: every object it reaches and the stage that first reached it.
    closures: Vec<Vec<(usize, Stage)>>,
    /// Each page's content stream objects.
    contents: Vec<Vec<usize>>,
    /// The plan's pages, as indices.
    pages: Vec<usize>,
    /// Which plan index is the first page.
    first: usize,
    /// Extended groups' roots, whose references count as the group's.
    roots: Vec<(Category, Vec<usize>)>,
}

/// Assigns every object to its part, in §F.3's order.
#[expect(
    clippy::too_many_lines,
    reason = "F.3.5 to F.3.10 are one ordered sequence of claims, each depending on what the \
              ones before it took; splitting it would scatter the order across functions that \
              only make sense in this one"
)]
fn place(
    graph: &Graph,
    root: usize,
    info: Option<usize>,
    pages: &[usize],
    first: usize,
    dropped: &[bool],
) -> Placement {
    let count = graph.objects.len();
    // An object the push-down orphaned is claimed by nothing and written nowhere.
    let mut claimed: Vec<bool> = (0..count)
        .map(|index| dropped.get(index).copied().unwrap_or(false))
        .collect();
    let mut placement = Placement {
        pages: pages.to_vec(),
        first,
        ..Placement::default()
    };
    let mut is_page = vec![false; count];
    for page in pages {
        if let Some(flag) = is_page.get_mut(*page) {
            *flag = true;
        }
    }

    // §7.7.3.2's tree nodes, walked down `/Kids` from the catalog's `/Pages`: §F.3.7 excludes
    // "page tree nodes" from every page's objects, and §F.3.10 places "[t]he page tree" in part 9.
    let mut nodes: Vec<usize> = Vec::new();
    let mut is_node = vec![false; count];
    {
        let mut stack: Vec<usize> = graph.targets(graph.entry(root, "Pages"));
        stack.reverse();
        while let Some(node) = stack.pop() {
            if is_page.get(node).copied().unwrap_or(true)
                || is_node.get(node).copied().unwrap_or(true)
            {
                continue;
            }
            if let Some(flag) = is_node.get_mut(node) {
                *flag = true;
            }
            nodes.push(node);
            let mut kids = graph.targets(graph.entry_resolved(node, "Kids"));
            kids.reverse();
            stack.extend(kids);
        }
    }

    // §F.3.5's part 4: the catalog, and "the values of the following entries if they are present
    // and are indirect objects".
    let claim = |index: usize, claimed: &mut Vec<bool>, list: &mut Vec<usize>| {
        if let Some(flag) = claimed.get_mut(index)
            && !*flag
        {
            *flag = true;
            list.push(index);
        }
    };
    let mut part4 = Vec::new();
    claim(root, &mut claimed, &mut part4);
    // A value that is a page or a page tree node is placed by F.3.7 and F.3.10 instead: Table 29
    // makes `/OpenAction` "either an array defining a destination … or an action dictionary", so
    // one naming a page object directly is a malformed destination, and its page is still a page.
    let placed_by_the_tree = |index: usize| {
        is_page.get(index).copied().unwrap_or(true) || is_node.get(index).copied().unwrap_or(true)
    };
    for key in [
        "ViewerPreferences",
        "PageMode",
        "Threads",
        "OpenAction",
        "AcroForm",
    ] {
        if let Some(index) = graph
            .entry(root, key)
            .and_then(|value| graph.index_of(value))
            && !placed_by_the_tree(index)
        {
            claim(index, &mut claimed, &mut part4);
        }
    }
    // "The Threads entry …, along with all thread dictionaries it refers to. This does not include
    // the threads' information dictionaries or the individual bead dictionaries".
    let threads = graph.targets(graph.entry_resolved(root, "Threads"));
    for thread in &threads {
        if !placed_by_the_tree(*thread) {
            claim(*thread, &mut claimed, &mut part4);
        }
    }
    let mut in_part4 = vec![false; count];
    for index in &part4 {
        if let Some(flag) = in_part4.get_mut(*index) {
            *flag = true;
        }
    }

    // §F.3.7's page walks.
    let mut seen = Seen::new(count);
    let mut queued = Seen::new(count);
    let mut first_reach = vec![false; count];
    // `None`: reached by no other page; `Some(Some(p))`: by page `p` alone; `Some(None)`: by two
    // or more.
    let mut others: Vec<Option<Option<usize>>> = vec![None; count];
    for (at, page) in pages.iter().enumerate() {
        let stop = |target: usize| {
            target != *page
                && (target == root
                    || is_page.get(target).copied().unwrap_or(true)
                    || is_node.get(target).copied().unwrap_or(true)
                    || graph.dpart.get(target).copied().unwrap_or(true)
                    || graph.embedded.get(target).copied().unwrap_or(true)
                    || in_part4.get(target).copied().unwrap_or(true))
        };
        let closure = page_walk(graph, *page, &stop, &mut seen, &mut queued);
        for (index, _) in &closure {
            if at == first {
                if let Some(flag) = first_reach.get_mut(*index) {
                    *flag = true;
                }
            } else if let Some(slot) = others.get_mut(*index) {
                *slot = match *slot {
                    None => Some(Some(at)),
                    Some(Some(only)) if only == at => Some(Some(at)),
                    Some(_) => Some(None),
                };
            }
        }
        placement.contents.push(contents_of(graph, *page));
        placement.closures.push(closure);
    }

    // Part 6: the first page's own objects, in its walk's order, then the ones another page uses.
    let first_closure = placement.closures.get(first).cloned().unwrap_or_default();
    let mut first_private = Vec::new();
    let mut first_shared = Vec::new();
    for (index, _) in &first_closure {
        if others.get(*index).copied().flatten().is_none() {
            claim(*index, &mut claimed, &mut first_private);
        } else {
            claim(*index, &mut claimed, &mut first_shared);
        }
    }

    // §F.3.7: "The entire outline hierarchy, if the value of the PageMode entry in the catalog
    // dictionary is UseOutlines."
    let use_outlines = graph
        .entry_resolved(root, "PageMode")
        .and_then(Object::as_name)
        .is_some_and(|name| name.as_bytes() == b"UseOutlines");
    let outline_present = graph
        .entry_resolved(root, "Outlines")
        .is_some_and(|value| value.as_dict().is_some());
    let reserved_default = |target: usize| {
        graph.embedded.get(target).copied().unwrap_or(true)
            || graph.dpart.get(target).copied().unwrap_or(true)
    };

    // Part 7: every other page's own objects, and part 8: what two or more of them share.
    for (at, closure) in placement.closures.iter().enumerate() {
        if at == first {
            continue;
        }
        let mut section = Vec::new();
        for (index, _) in closure {
            if !first_reach.get(*index).copied().unwrap_or(true)
                && others.get(*index).copied().flatten() == Some(Some(at))
            {
                claim(*index, &mut claimed, &mut section);
            }
        }
        placement.sections.push((at, section));
    }
    // §F.3.9: "wherever a resource consists of a multiple-level structure, all components of the
    // structure shall be grouped together" — so part 8 is ordered by a plain depth-first walk of
    // each page in page order, which keeps a structure's components adjacent where §F.3.7's order
    // deferred some of them.
    for (at, page) in pages.iter().enumerate() {
        if at == first {
            continue;
        }
        let stop = |target: usize| {
            target != *page
                && (target == root
                    || is_page.get(target).copied().unwrap_or(true)
                    || is_node.get(target).copied().unwrap_or(true)
                    || graph.dpart.get(target).copied().unwrap_or(true)
                    || graph.embedded.get(target).copied().unwrap_or(true)
                    || in_part4.get(target).copied().unwrap_or(true))
        };
        let skip = ["Parent", "Thumb"];
        let starts: Vec<usize> = graph
            .dict(*page)
            .map(|dict| {
                dict.iter()
                    .filter(|(key, _)| !skip.iter().any(|skip| key.as_bytes() == skip.as_bytes()))
                    .flat_map(|(_, value)| graph.targets(Some(value)))
                    .collect()
            })
            .unwrap_or_default();
        for index in preorder(graph, &starts, &stop, &mut seen) {
            if !first_reach.get(index).copied().unwrap_or(true)
                && others.get(index).copied().flatten() == Some(None)
            {
                claim(index, &mut claimed, &mut placement.shared);
            }
        }
    }
    // Claimed only now, so that an object some page reaches stays that page's; it is still
    // written in part 6, where §F.3.7 puts it.
    if use_outlines && outline_present {
        let outline = outline_claim(graph, root, &mut claimed, &reserved_default, &mut seen);
        first_private.extend(outline.iter().copied());
        placement.outline_in_first_page = Some(outline);
    }
    placement.part4 = part4;
    placement.first_private = first_private;
    placement.first_shared = first_shared;

    // Part 9, §F.3.10's categories in its own order.
    let mut nine: Vec<(Option<Category>, Vec<usize>)> = Vec::new();

    // "The page tree."
    let mut tree = Vec::new();
    for node in &nodes {
        claim(*node, &mut claimed, &mut tree);
    }
    tree.extend(claim_walk(
        graph,
        &[],
        &nodes,
        &mut claimed,
        &reserved_default,
        &mut seen,
    ));
    nine.push((None, tree));

    // "Thumbnail images. These objects shall simply be ordered by page number." Then "[t]humbnail
    // shared objects".
    placement.thumbnails_at = nine.len();
    let thumbed: Vec<(usize, Vec<usize>)> = pages
        .iter()
        .enumerate()
        .filter(|(_, page)| graph.entry(**page, "Thumb").is_some())
        .map(|(at, page)| (at, graph.targets(graph.entry(*page, "Thumb"))))
        .collect();
    if !thumbed.is_empty() {
        // "Thumbnail shared objects. These are objects that shall be shared among some or all
        // thumbnail images and shall not be referenced from any other objects." So an object
        // something other than a thumbnail reaches is not a thumbnail's at all, and waits for the
        // category that reaches it.
        let elsewhere = reached_without_thumbnails(graph, root, info, &is_page);
        let mut reach: Vec<u32> = vec![0; count];
        let mut walks = Vec::new();
        for (at, starts) in &thumbed {
            let blocked = |target: usize| {
                claimed.get(target).copied().unwrap_or(true)
                    || reserved_default(target)
                    || elsewhere.get(target).copied().unwrap_or(true)
            };
            let walk = preorder(graph, starts, &blocked, &mut seen);
            for index in &walk {
                if let Some(slot) = reach.get_mut(*index) {
                    *slot = slot.saturating_add(1);
                }
            }
            walks.push((*at, walk));
        }
        let mut images = Vec::new();
        for (at, walk) in &walks {
            let mut own = Vec::new();
            for index in walk {
                if reach.get(*index).copied() == Some(1) {
                    claim(*index, &mut claimed, &mut own);
                }
            }
            images.push((*at, own));
        }
        let mut shared = Vec::new();
        for (_, walk) in &walks {
            for index in walk {
                claim(*index, &mut claimed, &mut shared);
            }
        }
        placement.thumbnails = Some((images, shared));
    }

    // "The outline hierarchy, if not located in part 6."
    if outline_present && !use_outlines {
        let outline = outline_claim(graph, root, &mut claimed, &reserved_default, &mut seen);
        nine.push((Some(Category::Outline), outline));
    }

    // "Thread information dictionaries, referenced from the I entries of thread dictionaries."
    // Table F.2's `/A` is "( Required only if article threads exist )": a `/Threads` array with a
    // thread in it, whether its elements are references or, against Table 29's own wording, the
    // dictionaries themselves.
    let thread_array = graph
        .entry_resolved(root, "Threads")
        .and_then(Object::as_array)
        .unwrap_or_default();
    if !thread_array.is_empty() {
        let starts: Vec<usize> = thread_array
            .iter()
            .flat_map(|thread| match graph.index_of(thread) {
                Some(index) => graph.targets(graph.entry(index, "I")),
                None => graph.targets(thread.as_dict().and_then(|dict| dict.get("I"))),
            })
            .collect();
        let group = claim_walk(
            graph,
            &starts,
            &[],
            &mut claimed,
            &reserved_default,
            &mut seen,
        );
        nine.push((Some(Category::Threads), group));
    }

    // "Named destinations. These objects include the value of the Dests or Names entry in the
    // document catalog dictionary and all the destination objects that it refers to".
    let names = graph
        .entry(root, "Names")
        .and_then(|value| graph.index_of(value));
    let names_dict = graph
        .entry_resolved(root, "Names")
        .and_then(Object::as_dict);
    let dests_in_names = names_dict.and_then(|dict| dict.get("Dests"));
    let dests_in_catalog = graph.entry(root, "Dests");
    if dests_in_catalog.is_some() || dests_in_names.is_some() {
        let mut group = Vec::new();
        if let Some(names) = names
            && dests_in_names.is_some()
        {
            claim(names, &mut claimed, &mut group);
        }
        let mut starts = graph.targets(dests_in_catalog);
        starts.extend(graph.targets(dests_in_names));
        group.extend(claim_walk(
            graph,
            &starts,
            &[],
            &mut claimed,
            &reserved_default,
            &mut seen,
        ));
        nine.push((Some(Category::Destinations), group));
    }

    // "The document information dictionary and the objects contained within it."
    if let Some(info) = info {
        let group = claim_walk(
            graph,
            &[info],
            &[],
            &mut claimed,
            &reserved_default,
            &mut seen,
        );
        nine.push((Some(Category::Information), group));
    }

    // "The interactive form field hierarchy. This group of objects shall not include the top-level
    // interactive form dictionary, which is located with the document catalog dictionary."
    if let Some(form) = graph.entry(root, "AcroForm") {
        let (starts, through) = match graph.index_of(form) {
            Some(index) => (Vec::new(), vec![index]),
            None => (graph.targets(Some(form)), Vec::new()),
        };
        let group = claim_walk(
            graph,
            &starts,
            &through,
            &mut claimed,
            &reserved_default,
            &mut seen,
        );
        let mut roots = starts;
        roots.extend(through);
        placement.roots.push((Category::Form, roots));
        nine.push((Some(Category::Form), group));
    }

    // "(PDF 1.3) The logical structure hierarchy."
    if let Some(structure) = graph.entry(root, "StructTreeRoot") {
        let starts = graph.targets(Some(structure));
        let group = claim_walk(
            graph,
            &starts,
            &[],
            &mut claimed,
            &reserved_default,
            &mut seen,
        );
        placement.roots.push((Category::Structure, starts));
        nine.push((Some(Category::Structure), group));
    }

    // "(PDF 1.5) The renditions name tree hierarchy."
    if let Some(renditions) = names_dict.and_then(|dict| dict.get("Renditions")) {
        let starts = graph.targets(Some(renditions));
        let group = claim_walk(
            graph,
            &starts,
            &[],
            &mut claimed,
            &reserved_default,
            &mut seen,
        );
        placement.roots.push((Category::Renditions, starts));
        nine.push((Some(Category::Renditions), group));
    }

    // "(PDF 1.5) Embedded file streams." Each one with what it alone refers to, so that §F.4.7's
    // groups are runs of adjacent objects.
    placement.embedded_at = nine.len();
    let others_of_its_kind = |target: usize| graph.embedded.get(target).copied().unwrap_or(true);
    for index in 0..count {
        if graph.embedded.get(index).copied().unwrap_or(false)
            && !claimed.get(index).copied().unwrap_or(true)
        {
            let mut group = Vec::new();
            claim(index, &mut claimed, &mut group);
            group.extend(claim_walk(
                graph,
                &[],
                &[index],
                &mut claimed,
                &|target| {
                    others_of_its_kind(target) || graph.dpart.get(target).copied().unwrap_or(true)
                },
                &mut seen,
            ));
            placement.embedded.push(group);
        }
    }

    // "(PDF 2.0) DPart tree and Document Part Metadata".
    if let Some(parts) = graph.entry(root, "DPartRoot") {
        let starts = graph.targets(Some(parts));
        let group = claim_walk(
            graph,
            &starts,
            &[],
            &mut claimed,
            &|target| graph.embedded.get(target).copied().unwrap_or(true),
            &mut seen,
        );
        nine.push((None, group));
    }

    // §12.4.2's page labels, Table F.2's `/L`: "Other entries in the document catalog dictionary",
    // grouped on their own so that a generic hint table can describe them.
    if let Some(labels) = graph.entry(root, "PageLabels") {
        let starts = graph.targets(Some(labels));
        let group = claim_walk(
            graph,
            &starts,
            &[],
            &mut claimed,
            &reserved_default,
            &mut seen,
        );
        nine.push((Some(Category::Labels), group));
    }

    // "Other entries in the document catalog dictionary that are not referenced from any page."
    let group = claim_walk(
        graph,
        &[],
        &[root],
        &mut claimed,
        &reserved_default,
        &mut seen,
    );
    nine.push((None, group));

    // Whatever nothing reaches — an assembly written without pruning keeps such objects, and
    // §F.3.10 opens with "any other objects that are part of the document but are not required
    // for displaying pages".
    let mut rest = Vec::new();
    for index in 0..count {
        claim(index, &mut claimed, &mut rest);
    }
    nine.push((None, rest));

    placement.nine = nine;
    placement
}

/// Which objects the trailer's roots reach without passing through a page's `/Thumb`.
fn reached_without_thumbnails(
    graph: &Graph,
    root: usize,
    info: Option<usize>,
    is_page: &[bool],
) -> Vec<bool> {
    let count = graph.objects.len();
    let mut reached = vec![false; count];
    let mut stack: Vec<usize> = std::iter::once(root).chain(info).collect();
    while let Some(index) = stack.pop() {
        match reached.get_mut(index) {
            Some(flag) if !*flag => *flag = true,
            _ => continue,
        }
        if is_page.get(index).copied().unwrap_or(false) {
            if let Some(dict) = graph.dict(index) {
                for (key, value) in dict.iter() {
                    if key.as_bytes() != b"Thumb" {
                        stack.extend(graph.targets(Some(value)));
                    }
                }
            }
        } else if let Some(edges) = graph.edges.get(index) {
            stack.extend(edges.iter().map(|edge| edge.to));
        }
    }
    reached
}

/// §F.3.7's walk of one page: the page object, then its objects in the clause's lettered order.
///
/// > The page object for the first page. This object shall be the first one in this part of the
/// > PDF file.
///
/// Then (a) the annotations, (b) the beads, (c) the `/Resources` dictionary and (d) the
/// resources it names, (e) the contents, every other entry, the annotations' appearances that (a)
/// defers, (f) the images and (g) the substitutable font programs. (d)'s one `shall` — "each
/// resource object shall precede the stream in which it is first referenced" — holds because
/// every resource object other than an image or a font program is placed before the first content
/// stream, and (f) and (g) are the types (d) itself excepts.
fn page_walk(
    graph: &Graph,
    page: usize,
    stop: &dyn Fn(usize) -> bool,
    seen: &mut Seen,
    queued: &mut Seen,
) -> Vec<(usize, Stage)> {
    seen.next();
    queued.next();
    let mut out = vec![(page, Stage::Page)];
    seen.mark(page);
    let mut deferred: [Vec<usize>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    let Some(dict) = graph.dict(page) else {
        return out;
    };
    let named = ["Annots", "B", "Resources", "Contents"];
    let mut keyed: Vec<(Stage, Vec<Edge>)> = vec![
        (Stage::Annots, graph.refs(dict.get("Annots"))),
        (Stage::Beads, graph.refs(dict.get("B"))),
        (Stage::Resources, graph.refs(dict.get("Resources"))),
        (Stage::Contents, graph.refs(dict.get("Contents"))),
    ];
    let mut rest = Vec::new();
    for (key, value) in dict.iter() {
        let key = key.as_bytes();
        // `/Parent` is the page tree, and `/Thumb` is §F.3.10's thumbnail category.
        if key == b"Parent" || key == b"Thumb" || named.iter().any(|name| name.as_bytes() == key) {
            continue;
        }
        rest.extend(graph.refs(Some(value)));
    }
    keyed.push((Stage::Other, rest));
    for (stage, edges) in keyed {
        for edge in edges {
            descend(
                graph,
                edge,
                stage,
                stop,
                seen,
                queued,
                &mut deferred,
                &mut out,
            );
        }
    }
    for (slot, stage) in [
        (0, Stage::Appearance),
        (1, Stage::Images),
        (2, Stage::FontFiles),
    ] {
        let mut at = 0;
        while let Some(index) = deferred.get(slot).and_then(|list| list.get(at)).copied() {
            at = at.saturating_add(1);
            let edge = Edge {
                to: index,
                via: Via::Plain,
            };
            descend(
                graph,
                edge,
                stage,
                stop,
                seen,
                queued,
                &mut deferred,
                &mut out,
            );
        }
    }
    out
}

/// One depth-first descent of [`page_walk`], deferring what §F.3.7 defers.
#[expect(
    clippy::too_many_arguments,
    reason = "one descent needs the graph, the edge, the stage, the stop rule, both visited sets, \
              the three deferred lists and the output; a struct for them would be a struct whose \
              only method is this function"
)]
fn descend(
    graph: &Graph,
    start: Edge,
    stage: Stage,
    stop: &dyn Fn(usize) -> bool,
    seen: &mut Seen,
    queued: &mut Seen,
    deferred: &mut [Vec<usize>; 3],
    out: &mut Vec<(usize, Stage)>,
) {
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let visit = |edge: Edge,
                 seen: &mut Seen,
                 queued: &mut Seen,
                 deferred: &mut [Vec<usize>; 3],
                 out: &mut Vec<(usize, Stage)>|
     -> bool {
        let target = edge.to;
        if seen.has(target) || stop(target) {
            return false;
        }
        let defer_to = if graph.image.get(target).copied().unwrap_or(false) && stage < Stage::Images
        {
            Some(1)
        } else if edge.via == Via::FontFile && stage < Stage::FontFiles {
            Some(2)
        } else if edge.via == Via::Appearance && stage < Stage::Appearance {
            Some(0)
        } else {
            None
        };
        if let Some(slot) = defer_to {
            if !queued.mark(target)
                && let Some(list) = deferred.get_mut(slot)
            {
                list.push(target);
            }
            return false;
        }
        seen.mark(target);
        out.push((target, stage));
        true
    };
    if visit(start, seen, queued, deferred, out) {
        stack.push((start.to, 0));
    }
    while let Some((object, position)) = stack.last_mut() {
        let Some(edge) = graph
            .edges
            .get(*object)
            .and_then(|edges| edges.get(*position))
            .copied()
        else {
            stack.pop();
            continue;
        };
        *position = position.saturating_add(1);
        // Inside an appearance, a font program or an image, the reference's own kind is what
        // it was reached through, so a font descriptor under an appearance stream still defers
        // its program.
        if visit(edge, seen, queued, deferred, out) {
            stack.push((edge.to, 0));
        }
    }
}

/// A plain depth-first preorder from `starts`, not entering what `stop` names.
fn preorder(
    graph: &Graph,
    starts: &[usize],
    stop: &dyn Fn(usize) -> bool,
    seen: &mut Seen,
) -> Vec<usize> {
    seen.next();
    let mut out = Vec::new();
    for start in starts {
        if seen.has(*start) || stop(*start) {
            continue;
        }
        seen.mark(*start);
        out.push(*start);
        let mut stack: Vec<(usize, usize)> = vec![(*start, 0)];
        while let Some((object, position)) = stack.last_mut() {
            let Some(edge) = graph
                .edges
                .get(*object)
                .and_then(|edges| edges.get(*position))
                .copied()
            else {
                stack.pop();
                continue;
            };
            *position = position.saturating_add(1);
            if seen.has(edge.to) || stop(edge.to) {
                continue;
            }
            seen.mark(edge.to);
            out.push(edge.to);
            stack.push((edge.to, 0));
        }
    }
    out
}

/// Claims, in preorder, every unclaimed object reachable from `starts`, and from the references of
/// `through` whether those are claimed or not; `reserved` names what belongs to another category.
fn claim_walk(
    graph: &Graph,
    starts: &[usize],
    through: &[usize],
    claimed: &mut [bool],
    reserved: &dyn Fn(usize) -> bool,
    seen: &mut Seen,
) -> Vec<usize> {
    let mut roots: Vec<usize> = starts.to_vec();
    for object in through {
        if let Some(edges) = graph.edges.get(*object) {
            roots.extend(edges.iter().map(|edge| edge.to));
        }
    }
    let walked = {
        let blocked =
            |target: usize| claimed.get(target).copied().unwrap_or(true) || reserved(target);
        preorder(graph, &roots, &blocked, seen)
    };
    for index in &walked {
        if let Some(flag) = claimed.get_mut(*index) {
            *flag = true;
        }
    }
    walked
}

/// §F.3.10's outline order, claimed: the outline dictionary, the items "in the order in which
/// they shall be displayed", then whatever else they refer to.
///
/// > This is a preorder traversal of the outline tree, skipping over any subtree that is closed
/// > (that is, whose parent's Count value is negative). Following that shall be the subtrees that
/// > were skipped over, in the order in which they would have appeared if they were all open.
fn outline_claim(
    graph: &Graph,
    root: usize,
    claimed: &mut [bool],
    reserved: &dyn Fn(usize) -> bool,
    seen: &mut Seen,
) -> Vec<usize> {
    let link = |item: usize, key: &str| graph.entry(item, key).and_then(|v| graph.index_of(v));
    let outlines = graph
        .entry(root, "Outlines")
        .and_then(|v| graph.index_of(v));
    let first_item = match outlines {
        Some(outlines) => link(outlines, "First"),
        None => graph
            .entry_resolved(root, "Outlines")
            .and_then(Object::as_dict)
            .and_then(|dict| dict.get("First"))
            .and_then(|value| graph.index_of(value)),
    };
    let mut order: Vec<usize> = outlines.into_iter().collect();
    let mut visited = BTreeSet::new();
    let mut skipped = Vec::new();
    // One pass that honours `/Count`, then one per skipped subtree that treats every item as open.
    let mut passes: Vec<(Option<usize>, bool)> = vec![(first_item, true)];
    let mut pass = 0;
    while let Some((start, honour)) = passes.get(pass).copied() {
        pass = pass.saturating_add(1);
        let mut stack: Vec<Option<usize>> = vec![start];
        while let Some(top) = stack.last_mut() {
            let Some(item) = *top else {
                stack.pop();
                continue;
            };
            *top = link(item, "Next");
            if !visited.insert(item) {
                stack.pop();
                continue;
            }
            order.push(item);
            if let Some(child) = link(item, "First") {
                let closed = graph
                    .entry(item, "Count")
                    .and_then(Object::as_integer)
                    .is_some_and(|count| count < 0);
                if honour && closed {
                    skipped.push(child);
                } else {
                    stack.push(Some(child));
                }
            }
        }
        if pass == 1 {
            passes.extend(skipped.iter().map(|child| (Some(*child), false)));
        }
    }
    let mut group = Vec::new();
    for index in &order {
        if let Some(flag) = claimed.get_mut(*index)
            && !*flag
            && !reserved(*index)
        {
            *flag = true;
            group.push(*index);
        }
    }
    let through = group.clone();
    group.extend(claim_walk(graph, &[], &through, claimed, reserved, seen));
    group
}

/// A page's content streams, as indices: `/Contents` itself where it is a stream, or every
/// stream its array names, and the array where the array is itself an indirect object.
fn contents_of(graph: &Graph, page: usize) -> Vec<usize> {
    let Some(value) = graph.entry(page, "Contents") else {
        return Vec::new();
    };
    match graph.index_of(value) {
        Some(index) => match graph.objects.get(index) {
            Some(Object::Array(_)) => {
                let mut out = vec![index];
                out.extend(graph.targets(graph.objects.get(index)));
                out
            }
            _ => vec![index],
        },
        None => graph.targets(Some(value)),
    }
}

/// Numbers every object, renders it, and lays out everything whose position does not move.
#[expect(
    clippy::too_many_lines,
    reason = "the numbering, the rendering and every hint table's static half come out of one \
              pass over the placement, and each needs the positions the others fix"
)]
fn lay_out(
    graph: &Graph,
    placement: &Placement,
    root: usize,
    info: Option<usize>,
    tally: &mut Written,
) -> Result<Layout, LinearizeError> {
    let count = graph.objects.len();
    let mut sequence: Vec<usize> = Vec::with_capacity(count);
    sequence.extend(&placement.part4);
    let first_page_start = sequence.len();
    sequence.extend(&placement.first_private);
    let first_private = Span {
        start: first_page_start,
        count: placement.first_private.len(),
    };
    let first_shared = Span {
        start: sequence.len(),
        count: placement.first_shared.len(),
    };
    sequence.extend(&placement.first_shared);
    let first_page = Span {
        start: first_page_start,
        count: sequence.len().saturating_sub(first_page_start),
    };
    let first_group = sequence.len();

    let mut sections = Vec::new();
    for (at, objects) in &placement.sections {
        sections.push((
            *at,
            Span {
                start: sequence.len(),
                count: objects.len(),
            },
        ));
        sequence.extend(objects);
    }
    let shared = Span {
        start: sequence.len(),
        count: placement.shared.len(),
    };
    sequence.extend(&placement.shared);

    let mut groups = Vec::new();
    let mut thumbnails = None;
    let mut embedded = Vec::new();
    for (at, (category, objects)) in placement.nine.iter().enumerate() {
        if at == placement.thumbnails_at
            && let Some((images, common)) = &placement.thumbnails
        {
            let mut spans = Vec::new();
            for (page, objects) in images {
                spans.push((
                    *page,
                    Span {
                        start: sequence.len(),
                        count: objects.len(),
                    },
                ));
                sequence.extend(objects);
            }
            let common_span = Span {
                start: sequence.len(),
                count: common.len(),
            };
            sequence.extend(common);
            thumbnails = Some(Thumbnails {
                images: spans,
                shared: common_span,
            });
        }
        if at == placement.embedded_at {
            for objects in &placement.embedded {
                if let Some(file) = objects.first() {
                    embedded.push((
                        *file,
                        Span {
                            start: sequence.len(),
                            count: objects.len(),
                        },
                    ));
                }
                sequence.extend(objects);
            }
        }
        let span = Span {
            start: sequence.len(),
            count: objects.len(),
        };
        sequence.extend(objects);
        if let Some(category) = category {
            groups.push(Group {
                category: *category,
                span,
                shared: Vec::new(),
            });
        }
    }
    if let Some(outline) = &placement.outline_in_first_page {
        // The outline is part 6's, inside the first page's leading run: its group is where that
        // run ends, counted back.
        let end = first_private.end();
        groups.insert(
            0,
            Group {
                category: Category::Outline,
                span: Span {
                    start: end.saturating_sub(outline.len()),
                    count: outline.len(),
                },
                shared: Vec::new(),
            },
        );
    }

    // §F.3.1's numbering: the second group from 1, the first group after it, the parameter
    // dictionary first among them and the hint stream last.
    // Every number has to fit in `u32` with the two objects this module adds.
    let second_group = u32::try_from(sequence.len().saturating_sub(first_group))
        .ok()
        .filter(|_| u32::try_from(sequence.len().saturating_add(2)).is_ok_and(|n| n < u32::MAX))
        .ok_or(LinearizeError::TooManyObjects)?;
    let mut numbers = vec![0u32; sequence.len()];
    let mut number_of = vec![0u32; count];
    for (position, index) in sequence.iter().enumerate() {
        let number = if position < first_group {
            // After the parameter dictionary, which is `k + 1`.
            second_group
                .saturating_add(2)
                .saturating_add(u32::try_from(position).unwrap_or(u32::MAX))
        } else {
            u32::try_from(position.saturating_sub(first_group).saturating_add(1))
                .unwrap_or(u32::MAX)
        };
        if let Some(slot) = numbers.get_mut(position) {
            *slot = number;
        }
        if let Some(slot) = number_of.get_mut(*index) {
            *slot = number;
        }
    }

    let mut rendered = Vec::with_capacity(sequence.len());
    let mut digest = <md5::Md5 as md5::Digest>::new();
    for (position, index) in sequence.iter().enumerate() {
        let object = graph.objects.get(*index).cloned().unwrap_or(Object::Null);
        let number = numbers.get(position).copied().unwrap_or(0);
        let item = render(&object, number, &number_of, tally);
        <md5::Md5 as md5::Digest>::update(&mut digest, &item.head);
        if let Some(data) = &item.data {
            <md5::Md5 as md5::Digest>::update(&mut digest, data);
        }
        <md5::Md5 as md5::Digest>::update(&mut digest, item.tail);
        rendered.push(item);
    }
    let identifier: [u8; 16] = <md5::Md5 as md5::Digest>::finalize(digest).into();

    let mut position_of = vec![usize::MAX; count];
    for (position, index) in sequence.iter().enumerate() {
        if let Some(slot) = position_of.get_mut(*index) {
            *slot = position;
        }
    }

    // The shared object hint table's identifiers: entry 0 is the first page's leading run, then
    // one entry per object of part 6's trailing run, then one per object of part 8.
    let mut entry_of: Vec<Option<u64>> = vec![None; count];
    for index in &placement.first_private {
        if let Some(slot) = entry_of.get_mut(*index) {
            *slot = Some(0);
        }
    }
    for (at, index) in placement
        .first_shared
        .iter()
        .chain(placement.shared.iter())
        .enumerate()
    {
        if let Some(slot) = entry_of.get_mut(*index) {
            *slot = Some(u64::try_from(at).unwrap_or(u64::MAX).saturating_add(1));
        }
    }

    // Table F.4, per page, in the file's order: the first page, then the rest by page number.
    let mut pages = Vec::new();
    let page_entry = |at: usize, span: Span, own_page: bool| -> PageEntry {
        let contents = placement.contents.get(at).and_then(|objects| {
            let positions: Vec<usize> = objects
                .iter()
                .filter_map(|index| position_of.get(*index).copied())
                .collect();
            let inside = !positions.is_empty()
                && positions
                    .iter()
                    .all(|position| *position >= span.start && *position < span.end());
            inside.then(|| {
                let low = positions.iter().copied().min().unwrap_or(span.start);
                let high = positions.iter().copied().max().unwrap_or(span.start);
                (low, high)
            })
        });
        let mut shared = Vec::new();
        if !own_page && let Some(closure) = placement.closures.get(at) {
            for (index, stage) in closure {
                if let Some(id) = entry_of.get(*index).copied().flatten() {
                    shared.push((id, stage.numerator()));
                }
            }
        }
        PageEntry {
            span,
            contents,
            shared,
        }
    };
    pages.push(page_entry(placement.first, first_page, true));
    for (at, span) in &sections {
        pages.push(page_entry(*at, *span, false));
    }

    // Extended generic tables' identifiers: every shared object a group's objects or its roots
    // refer to directly.
    for group in &mut groups {
        if !group.category.extended() {
            continue;
        }
        let mut ids = BTreeSet::new();
        let roots = placement
            .roots
            .iter()
            .filter(|(category, _)| *category == group.category)
            .flat_map(|(_, roots)| roots.iter().copied());
        let members = sequence
            .get(group.span.start..group.span.end())
            .unwrap_or_default()
            .iter()
            .copied();
        for object in roots.chain(members) {
            if let Some(id) = entry_of.get(object).copied().flatten() {
                ids.insert(id);
            }
            for edge in graph.edges.get(object).map_or(&[][..], Vec::as_slice) {
                if let Some(id) = entry_of.get(edge.to).copied().flatten() {
                    ids.insert(id);
                }
            }
        }
        group.shared = ids.into_iter().collect();
    }
    // Table F.10 item 7 states its width as Table F.3 item 11's, so that one number has to hold
    // every identifier either table states. ADR 1293.
    let greatest = pages
        .iter()
        .flat_map(|page| page.shared.iter().map(|(id, _)| *id))
        .chain(groups.iter().flat_map(|group| group.shared.iter().copied()))
        .max()
        .unwrap_or(0);

    let page_count = placement.pages.len();
    let first_page_object = numbers.get(first_page.start).copied().unwrap_or(0);
    Ok(Layout {
        sequence,
        rendered,
        numbers,
        first_group,
        second_group,
        first_page,
        first_private,
        pages,
        first_shared,
        shared,
        groups,
        thumbnails,
        embedded,
        root: number_of.get(root).copied().unwrap_or(0),
        info: info.and_then(|info| number_of.get(info).copied()),
        first_page_object,
        page_count,
        first_page_number: placement.first,
        identifier,
        identifier_bits: bits(greatest),
    })
}

/// One object as it will be written: its value with every reference in the final numbering.
fn render(object: &Object, number: u32, number_of: &[u32], tally: &mut Written) -> Rendered {
    let mut head = Vec::new();
    let _ = writeln!(Text(&mut head), "{number} 0 obj");
    match remap(object, number_of, 0, tally) {
        Object::Stream(stream) => {
            let mut dict = stream.dict.clone();
            let actual = i64::try_from(stream.data.len()).unwrap_or(i64::MAX);
            if dict.get("Length").and_then(Object::as_integer) != Some(actual) {
                tally.relengthed = tally.relengthed.saturating_add(1);
            }
            dict.insert(Name::new(&b"Length"[..]), Object::Integer(actual));
            write::object(&Object::Dictionary(dict), &mut head);
            head.extend_from_slice(b"\nstream\n");
            Rendered {
                head,
                data: Some(Arc::clone(&stream.data)),
                tail: b"\nendstream\nendobj\n",
            }
        }
        other => {
            write::object(&other, &mut head);
            Rendered {
                head,
                data: None,
                tail: b"\nendobj\n",
            }
        }
    }
}

/// A value with every reference mapped from the assembly's numbering to the file's.
///
/// A reference to a number the assembly does not hold becomes §7.3.10's null, and a dictionary
/// entry that became null is dropped under §7.3.7 — the serializer's own two rules, applied once
/// more because the assembly's numbering is not the file's.
fn remap(value: &Object, number_of: &[u32], depth: usize, tally: &mut Written) -> Object {
    if depth >= MAX_VALUE_DEPTH {
        return Object::Null;
    }
    let deeper = depth.saturating_add(1);
    match value {
        Object::Reference(id) => {
            if let Some(number) = usize::try_from(id.number)
                .ok()
                .and_then(|number| number.checked_sub(1))
                .and_then(|index| number_of.get(index))
            {
                Object::Reference(ObjectId::new(*number, 0))
            } else {
                tally.dangling = tally.dangling.saturating_add(1);
                Object::Null
            }
        }
        Object::Array(items) => Object::Array(
            items
                .iter()
                .map(|item| remap(item, number_of, deeper, tally))
                .collect(),
        ),
        Object::Dictionary(dict) => Object::Dictionary(remap_dict(dict, number_of, deeper, tally)),
        Object::Stream(stream) => Object::Stream(Arc::new(Stream {
            dict: remap_dict(&stream.dict, number_of, deeper, tally),
            data: Arc::clone(&stream.data),
            decryption_failed: stream.decryption_failed,
        })),
        other => other.clone(),
    }
}

/// [`remap`] over a dictionary's values.
fn remap_dict(
    dict: &Dictionary,
    number_of: &[u32],
    depth: usize,
    tally: &mut Written,
) -> Dictionary {
    let mut out = Dictionary::new();
    for (key, value) in dict.iter() {
        let value = remap(value, number_of, depth, tally);
        if !matches!(value, Object::Null) {
            out.insert(key.clone(), value);
        }
    }
    out
}

/// The parts whose lengths move every offset, rendered for one layout.
#[derive(Debug)]
struct Variable {
    /// Part 2.
    parameters: Vec<u8>,
    /// Part 3.
    first_xref: Vec<u8>,
    /// Part 5.
    hints: Vec<u8>,
    /// Part 11.
    main_xref: Vec<u8>,
    /// Which Table F.2 keys part 5 states.
    tables: Vec<&'static str>,
}

/// Lays the file out to a fixed point, then writes it.
fn write_out<W: Write>(
    layout: &Layout,
    version: Version,
    tally: &mut Written,
    out: &mut W,
) -> Result<Linearized, LinearizeError> {
    let head = header(version);
    let head_len = u64::try_from(head.len()).unwrap_or(u64::MAX);
    let mut lengths = (0u64, 0u64, 0u64);
    let mut settled = None;
    for _ in 0..MAX_LAYOUT_PASSES {
        let parts = variable_parts(layout, head_len, lengths)?;
        let now = (
            u64::try_from(parts.parameters.len()).unwrap_or(u64::MAX),
            u64::try_from(parts.first_xref.len()).unwrap_or(u64::MAX),
            u64::try_from(parts.hints.len()).unwrap_or(u64::MAX),
        );
        if now == lengths {
            settled = Some(parts);
            break;
        }
        lengths = now;
    }
    let parts = settled.ok_or(LinearizeError::Unsettled)?;
    let end = head_len.saturating_add(lengths.0);
    if end > PARAMETERS_WITHIN {
        return Err(LinearizeError::ParametersTooLate { end });
    }

    let mut written: u64 = 0;
    let mut put = |bytes: &[u8], out: &mut W| -> Result<(), SerializeError> {
        out.write_all(bytes)?;
        written = written.saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        Ok(())
    };
    put(&head, out)?;
    put(&parts.parameters, out)?;
    put(&parts.first_xref, out)?;
    put(&parts.hints, out)?;
    for item in &layout.rendered {
        put(&item.head, out)?;
        if let Some(data) = &item.data {
            put(data, out)?;
        }
        put(item.tail, out)?;
    }
    put(&parts.main_xref, out)?;

    tally.bytes = written;
    tally.objects = u32::try_from(layout.sequence.len().saturating_add(2)).unwrap_or(u32::MAX);
    Ok(Linearized {
        written: *tally,
        hint_tables: parts.tables,
        shared_objects: layout.shared.count,
    })
}

/// Every offset of the file, given the three variable lengths.
#[derive(Debug)]
struct Offsets {
    /// Part 2.
    parameters: u64,
    /// Part 5.
    hints: u64,
    /// Part 5's length.
    hints_len: u64,
    /// Each object's offset, in the file's order, and one more: where part 11 begins.
    objects: Vec<u64>,
}

impl Offsets {
    /// An object's offset, or where part 11 begins past the last.
    fn at(&self, position: usize) -> u64 {
        self.objects
            .get(position)
            .or_else(|| self.objects.last())
            .copied()
            .unwrap_or(0)
    }

    /// A position as a hint table states it: §F.4.1's "as if the primary hint stream itself were
    /// not present".
    fn hinted(&self, position: usize) -> u64 {
        let at = self.at(position);
        if at > self.hints {
            at.saturating_sub(self.hints_len)
        } else {
            at
        }
    }

    /// The bytes from one position to another.
    fn between(&self, from: usize, to: usize) -> u64 {
        self.at(to).saturating_sub(self.at(from))
    }
}

/// Parts 2, 3, 5 and 11, for one guess at the lengths of 2, 3 and 5.
#[expect(
    clippy::too_many_lines,
    reason = "the four parts state each other's offsets, so they are rendered from one layout in \
              one place"
)]
fn variable_parts(
    layout: &Layout,
    head_len: u64,
    (parameters_len, first_xref_len, hints_len): (u64, u64, u64),
) -> Result<Variable, LinearizeError> {
    let parameters = head_len;
    let first_xref = parameters.saturating_add(parameters_len);
    let hints = first_xref.saturating_add(first_xref_len);
    let mut objects = Vec::with_capacity(layout.rendered.len().saturating_add(1));
    let mut at = hints.saturating_add(hints_len);
    for item in &layout.rendered {
        objects.push(at);
        at = at.saturating_add(item.len());
    }
    objects.push(at);
    let offsets = Offsets {
        parameters,
        hints,
        hints_len,
        objects,
    };
    let main_at = offsets.at(layout.rendered.len());

    // Part 11: §F.3.11's main table. "It consists of a single cross-reference subsection,
    // beginning at object number 0. The first entry (for object number 0) shall be a free entry.
    // The remaining entries are for in-use objects, which shall be numbered consecutively,
    // starting at 1."
    let size = u64::from(layout.second_group).saturating_add(1);
    let mut main = String::new();
    let _ = write!(main, "xref\n0 {size}\n");
    // Table F.1's `/T`: "the offset of the white-space character preceding the first entry of the
    // main cross-reference table (the entry for object number 0)" — the end of line above.
    let main_first_entry = main_at
        .saturating_add(u64::try_from(main.len()).unwrap_or(u64::MAX))
        .saturating_sub(1);
    let _ = writeln!(main, "{:010} {FREE_FOREVER:05} f ", 0);
    for position in layout.first_group..layout.rendered.len() {
        entry(&mut main, offsets.at(position))?;
    }
    // "The main trailer has no Prev entry and should not contain any entries other than Size."
    // And §F.3.11: "The startxref line shall give the offset of the first-page cross-reference
    // table in the PDF file."
    let _ = write!(
        main,
        "trailer\n<< /Size {size} >>\nstartxref\n{first_xref}\n%%EOF\n"
    );
    let main_xref = main.into_bytes();
    let length = main_at.saturating_add(u64::try_from(main_xref.len()).unwrap_or(u64::MAX));

    // Part 5.
    let (data, tables) = hint_data(layout, &offsets);
    let hint_number = u64::from(layout.second_group)
        .saturating_add(2)
        .saturating_add(u64::try_from(layout.first_group).unwrap_or(u64::MAX));
    let mut stream_dict = Dictionary::new();
    stream_dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.bytes.len()).unwrap_or(i64::MAX)),
    );
    for (key, offset) in &data.positions {
        stream_dict.insert(
            Name::new(key.as_bytes()),
            Object::Integer(i64::try_from(*offset).unwrap_or(i64::MAX)),
        );
    }
    let mut hint_bytes = Vec::new();
    let _ = writeln!(Text(&mut hint_bytes), "{hint_number} 0 obj");
    write::object(&Object::Dictionary(stream_dict), &mut hint_bytes);
    hint_bytes.extend_from_slice(b"\nstream\n");
    hint_bytes.extend_from_slice(&data.bytes);
    hint_bytes.extend_from_slice(b"\nendstream\nendobj\n");

    // Part 2: Table F.1, every value direct.
    let end_of_first_page = offsets.at(layout.first_page.end());
    let mut text = format!(
        "{} 0 obj\n<< /Linearized 1 /L {length} /H [ {hints} {hints_len} ] /O {} /E \
         {end_of_first_page} /N {} /T {main_first_entry}",
        u64::from(layout.second_group).saturating_add(1),
        layout.first_page_object,
        layout.page_count,
    );
    if layout.first_page_number != 0 {
        let _ = write!(text, " /P {}", layout.first_page_number);
    }
    text.push_str(" >>\nendobj\n");
    let parameters_bytes = text.into_bytes();

    // Part 3: §F.3.4's first-page table — "a single cross-reference subsection that has no free
    // entries", the parameter dictionary at its beginning and the hint stream at its end.
    let first_count = u64::try_from(layout.first_group)
        .unwrap_or(u64::MAX)
        .saturating_add(2);
    let mut table = String::new();
    let _ = write!(
        table,
        "xref\n{} {first_count}\n",
        u64::from(layout.second_group).saturating_add(1)
    );
    entry(&mut table, offsets.parameters)?;
    for position in 0..layout.first_group {
        entry(&mut table, offsets.at(position))?;
    }
    entry(&mut table, offsets.hints)?;
    // "The first-page trailer shall contain valid Size and Root entries, as well as any other
    // entries needed to display the document. The Size value shall be the combined number of
    // entries in both the first-page cross-reference table and the main cross-reference table."
    // And "[t]he trailer's Prev entry shall give the offset of the main cross-reference table".
    let mut trailer = Dictionary::new();
    trailer.insert(
        Name::new(&b"Size"[..]),
        Object::Integer(i64::try_from(size.saturating_add(first_count)).unwrap_or(i64::MAX)),
    );
    trailer.insert(
        Name::new(&b"Prev"[..]),
        Object::Integer(i64::try_from(main_at).unwrap_or(i64::MAX)),
    );
    trailer.insert(
        Name::new(&b"Root"[..]),
        Object::Reference(ObjectId::new(layout.root, 0)),
    );
    if let Some(info) = layout.info {
        trailer.insert(
            Name::new(&b"Info"[..]),
            Object::Reference(ObjectId::new(info, 0)),
        );
    }
    // §14.4: "When a PDF file is first written, both identifiers shall be set to the same value."
    let identifier = Object::String(layout.identifier.to_vec().into());
    trailer.insert(
        Name::new(&b"ID"[..]),
        Object::Array(vec![identifier.clone(), identifier]),
    );
    let mut first_bytes = table.into_bytes();
    first_bytes.extend_from_slice(b"trailer\n");
    write::object(&Object::Dictionary(trailer), &mut first_bytes);
    first_bytes.push(b'\n');

    Ok(Variable {
        parameters: parameters_bytes,
        first_xref: first_bytes,
        hints: hint_bytes,
        main_xref,
        tables,
    })
}

/// One §7.5.4 entry: "exactly 20 bytes long".
fn entry(text: &mut String, offset: u64) -> Result<(), SerializeError> {
    if offset > MAX_TABLE_OFFSET {
        return Err(SerializeError::OffsetTooLarge { offset });
    }
    let _ = writeln!(text, "{offset:010} 00000 n ");
    Ok(())
}

/// The hint stream's data and the dictionary positions of each table.
#[derive(Debug, Default)]
struct HintData {
    /// The decoded stream.
    bytes: Vec<u8>,
    /// Table F.2's keys and where each table begins.
    positions: Vec<(&'static str, u64)>,
}

/// The number of bits needed to represent `value`: zero for zero.
fn bits(value: u64) -> u32 {
    u64::BITS.saturating_sub(value.leading_zeros())
}

/// A bit stream, "high-order bit first" (§F.4.1).
#[derive(Debug, Default)]
struct BitWriter {
    /// Finished bytes.
    bytes: Vec<u8>,
    /// The byte being filled.
    current: u8,
    /// How many of its bits are filled.
    filled: u32,
}

impl BitWriter {
    /// Appends the low `width` bits of `value`, high-order first.
    fn put(&mut self, value: u64, width: u32) {
        for bit in (0..width.min(64)).rev() {
            let one = u8::from((value >> bit) & 1 == 1);
            self.current = (self.current << 1) | one;
            self.filled = self.filled.saturating_add(1);
            if self.filled == 8 {
                self.bytes.push(self.current);
                self.current = 0;
                self.filled = 0;
            }
        }
    }

    /// Pads to a byte boundary: §F.4.1's "each hint table shall begin at a byte boundary".
    fn align(&mut self) {
        if self.filled > 0 {
            let shift = 8u32.saturating_sub(self.filled);
            self.bytes.push(self.current << shift);
            self.current = 0;
            self.filled = 0;
        }
    }

    /// Where the next byte-aligned table begins.
    fn position(&self) -> u64 {
        u64::try_from(self.bytes.len()).unwrap_or(u64::MAX)
    }
}

/// The least of some values and the bits their spread needs.
fn spread(values: impl Iterator<Item = u64> + Clone) -> (u64, u32) {
    let least = values.clone().min().unwrap_or(0);
    let greatest = values.max().unwrap_or(0);
    (least, bits(greatest.saturating_sub(least)))
}

/// Every hint table, in the order Table F.2 lists them, page offset first.
#[expect(
    clippy::too_many_lines,
    reason = "F.4.2 to F.4.7 are nine tables written one after another into one bit stream, each \
              aligned where the last ended; one function per table would pass the same writer \
              and layout through nine signatures"
)]
fn hint_data(layout: &Layout, offsets: &Offsets) -> (HintData, Vec<&'static str>) {
    let mut w = BitWriter::default();
    let mut data = HintData::default();
    let length = |span: Span| offsets.between(span.start, span.end());
    let width = layout.identifier_bits;

    // §F.4.2, Table F.3 and Table F.4. "Additionally, there is a required page offset hint table,
    // which shall be the first table in the stream and shall start at offset 0".
    let entries = &layout.pages;
    let (least_objects, objects_bits) = spread(
        entries
            .iter()
            .map(|page| u64::try_from(page.span.count).unwrap_or(u64::MAX)),
    );
    let (least_length, length_bits) = spread(entries.iter().map(|page| length(page.span)));
    let content = |page: &PageEntry| -> (u64, u64) {
        page.contents.map_or((0, 0), |(low, high)| {
            (
                offsets.between(page.span.start, low),
                offsets.between(low, high.saturating_add(1)),
            )
        })
    };
    let (least_offset, offset_bits) = spread(entries.iter().map(|page| content(page).0));
    let (least_content, content_bits) = spread(entries.iter().map(|page| content(page).1));
    let most_shared = entries
        .iter()
        .map(|page| u64::try_from(page.shared.len()).unwrap_or(u64::MAX))
        .max()
        .unwrap_or(0);
    let most_numerator = entries
        .iter()
        .flat_map(|page| page.shared.iter().map(|(_, numerator)| *numerator))
        .max()
        .unwrap_or(0);
    let shared_bits = bits(most_shared);
    let numerator_bits = bits(most_numerator);
    w.put(least_objects, 32); // 1
    w.put(offsets.hinted(layout.first_page.start), 32); // 2
    w.put(u64::from(objects_bits), 16); // 3
    w.put(least_length, 32); // 4
    w.put(u64::from(length_bits), 16); // 5
    w.put(least_offset, 32); // 6
    w.put(u64::from(offset_bits), 16); // 7
    w.put(least_content, 32); // 8
    w.put(u64::from(content_bits), 16); // 9
    w.put(u64::from(shared_bits), 16); // 10
    w.put(u64::from(width), 16); // 11
    w.put(u64::from(numerator_bits), 16); // 12
    w.put(DENOMINATOR, 16); // 13
    // "The order of items making up the per-page entries shall be as follows" — item by item
    // across every page, the bit stream running on "without regard to byte boundaries".
    for page in entries {
        let objects = u64::try_from(page.span.count).unwrap_or(u64::MAX);
        w.put(objects.saturating_sub(least_objects), objects_bits);
    }
    for page in entries {
        w.put(length(page.span).saturating_sub(least_length), length_bits);
    }
    for page in entries {
        w.put(
            u64::try_from(page.shared.len()).unwrap_or(u64::MAX),
            shared_bits,
        );
    }
    for page in entries {
        for (id, _) in &page.shared {
            w.put(*id, width);
        }
    }
    for page in entries {
        for (_, numerator) in &page.shared {
            w.put(*numerator, numerator_bits);
        }
    }
    for page in entries {
        w.put(content(page).0.saturating_sub(least_offset), offset_bits);
    }
    for page in entries {
        w.put(content(page).1.saturating_sub(least_content), content_bits);
    }
    w.align();

    // §F.4.3, Table F.5 and Table F.6.
    data.positions.push(("S", w.position()));
    let mut groups: Vec<Span> = vec![layout.first_private];
    for position in layout.first_shared.start..layout.first_shared.end() {
        groups.push(Span {
            start: position,
            count: 1,
        });
    }
    let first_page_entries = groups.len();
    for position in layout.shared.start..layout.shared.end() {
        groups.push(Span {
            start: position,
            count: 1,
        });
    }
    let (least_group, group_bits) = spread(groups.iter().map(|span| length(*span)));
    let most_objects = groups
        .iter()
        .map(|span| u64::try_from(span.count).unwrap_or(u64::MAX))
        .max()
        .unwrap_or(0);
    let count_bits = bits(most_objects);
    w.put(u64::from(layout.numbers_at(layout.shared.start)), 32); // 1
    w.put(offsets.hinted(layout.shared.start), 32); // 2
    w.put(u64::try_from(first_page_entries).unwrap_or(u64::MAX), 32); // 3
    w.put(u64::try_from(groups.len()).unwrap_or(u64::MAX), 32); // 4
    w.put(u64::from(count_bits), 16); // 5
    w.put(least_group, 32); // 6
    w.put(u64::from(group_bits), 16); // 7
    for span in &groups {
        w.put(length(*span).saturating_sub(least_group), group_bits);
    }
    // Item 2, the signature flag: no signature is written, because item 3 is "a 16-byte MD5 hash
    // that uniquely identifies the resource", and deciding which resources are the same resource
    // is a claim about content.
    for _ in &groups {
        w.put(0, 1);
    }
    for span in &groups {
        let objects = u64::try_from(span.count).unwrap_or(u64::MAX);
        w.put(objects.saturating_sub(1), count_bits);
    }
    w.align();

    // §F.4.4, Table F.7 and Table F.8: "( Required only if thumbnail images exist )".
    if let Some(thumbnails) = &layout.thumbnails {
        data.positions.push(("T", w.position()));
        let images = &thumbnails.images;
        let mut gaps = Vec::new();
        let mut previous: Option<usize> = None;
        for (page, _) in images {
            let gap = match previous {
                None => *page,
                Some(before) => page.saturating_sub(before).saturating_sub(1),
            };
            gaps.push(u64::try_from(gap).unwrap_or(u64::MAX));
            previous = Some(*page);
        }
        let gap_bits = bits(gaps.iter().copied().max().unwrap_or(0));
        let (least_len, len_bits) = spread(images.iter().map(|(_, span)| length(*span)));
        let (least_count, count_bits) = spread(
            images
                .iter()
                .map(|(_, span)| u64::try_from(span.count).unwrap_or(u64::MAX)),
        );
        let first = images
            .first()
            .map_or(thumbnails.shared.start, |(_, span)| span.start);
        w.put(u64::from(layout.numbers_at(first)), 32); // 1
        w.put(offsets.hinted(first), 32); // 2
        w.put(u64::try_from(images.len()).unwrap_or(u64::MAX), 32); // 3
        w.put(u64::from(gap_bits), 16); // 4
        w.put(least_len, 32); // 5
        w.put(u64::from(len_bits), 16); // 6
        w.put(least_count, 32); // 7
        w.put(u64::from(count_bits), 16); // 8
        w.put(u64::from(layout.numbers_at(thumbnails.shared.start)), 32); // 9
        w.put(offsets.hinted(thumbnails.shared.start), 32); // 10
        w.put(
            u64::try_from(thumbnails.shared.count).unwrap_or(u64::MAX),
            32,
        ); // 11
        w.put(length(thumbnails.shared), 32); // 12
        for gap in &gaps {
            w.put(*gap, gap_bits);
        }
        for (_, span) in images {
            let objects = u64::try_from(span.count).unwrap_or(u64::MAX);
            w.put(objects.saturating_sub(least_count), count_bits);
        }
        for (_, span) in images {
            w.put(length(*span).saturating_sub(least_len), len_bits);
        }
        w.align();
    }

    // §F.4.5 and §F.4.6, in Table F.2's order of keys.
    for key in ["O", "A", "E", "V", "I", "C", "L", "R"] {
        let Some(group) = layout
            .groups
            .iter()
            .find(|group| group.category.key() == key)
        else {
            continue;
        };
        // Table F.2 states no condition on `/L` — "( PDF 1.3 ) Page label hint table" — so it is
        // the one table here that is not required, and it is written only where there is a group
        // of objects for it to find: a `/PageLabels` stated directly in the catalog has none.
        if group.category == Category::Labels && group.span.count == 0 {
            continue;
        }
        data.positions.push((group.category.key(), w.position()));
        w.put(u64::from(layout.numbers_at(group.span.start)), 32); // 1
        w.put(offsets.hinted(group.span.start), 32); // 2
        w.put(u64::try_from(group.span.count).unwrap_or(u64::MAX), 32); // 3
        w.put(length(group.span), 32); // 4
        if group.category.extended() {
            w.put(u64::try_from(group.shared.len()).unwrap_or(u64::MAX), 32); // 5
            w.put(u64::from(width), 16); // 6
            for id in &group.shared {
                w.put(*id, width); // 7 …
            }
        }
        w.align();
    }

    // §F.4.7, Table F.11 and Table F.12: "( Required only if embedded file streams exist )".
    if !layout.embedded.is_empty() {
        data.positions.push(("B", w.position()));
        let groups = &layout.embedded;
        let number_bits = bits(
            groups
                .iter()
                .map(|(_, span)| u64::from(layout.numbers_at(span.start)))
                .max()
                .unwrap_or(0),
        );
        let objects_bits = bits(
            groups
                .iter()
                .map(|(_, span)| u64::try_from(span.count).unwrap_or(u64::MAX))
                .max()
                .unwrap_or(0),
        );
        let length_bits = bits(
            groups
                .iter()
                .map(|(_, span)| length(*span))
                .max()
                .unwrap_or(0),
        );
        let first = groups.first().map_or(0, |(_, span)| span.start);
        w.put(u64::from(layout.numbers_at(first)), 32); // 1
        w.put(offsets.hinted(first), 32); // 2
        w.put(u64::try_from(groups.len()).unwrap_or(u64::MAX), 32); // 3
        w.put(u64::from(number_bits), 16); // 4
        w.put(u64::from(objects_bits), 16); // 5
        w.put(u64::from(length_bits), 16); // 6
        w.put(0, 16); // 7: no group states a shared object reference
        for (_, span) in groups {
            w.put(u64::from(layout.numbers_at(span.start)), number_bits);
        }
        for (_, span) in groups {
            w.put(u64::try_from(span.count).unwrap_or(u64::MAX), objects_bits);
        }
        for (_, span) in groups {
            w.put(length(*span), length_bits);
        }
        // Items 4 and 5 are zero bits wide, because header item 7 is 0.
        w.align();
    }

    let tables = data.positions.iter().map(|(key, _)| *key).collect();
    data.bytes = w.bytes;
    (data, tables)
}

impl Layout {
    /// The final number of the object at a position, or of the object after the last where the
    /// position is past the end — an empty group is stated where it would begin.
    fn numbers_at(&self, position: usize) -> u32 {
        self.numbers.get(position).copied().unwrap_or_else(|| {
            // Past the last object: the next number the second group would have used.
            self.second_group.saturating_add(1)
        })
    }
}

/// A `fmt::Write` over a byte vector.
struct Text<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for Text<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{BitWriter, bits};

    /// §F.4.1: "this byte stream shall be treated as a bit stream, high-order bit first".
    #[test]
    fn a_bit_stream_is_written_high_order_bit_first_and_padded_only_at_a_table_s_end() {
        let mut w = BitWriter::default();
        w.put(0b101, 3);
        w.put(0b1, 1);
        w.put(0b0110, 4);
        w.put(0b11, 2);
        w.align();
        assert_eq!(w.bytes, vec![0b1011_0110, 0b1100_0000]);
    }

    /// Table F.3's "number of bits needed to represent" a value.
    #[test]
    fn the_bits_a_value_needs_are_zero_for_zero() {
        assert_eq!(bits(0), 0);
        assert_eq!(bits(1), 1);
        assert_eq!(bits(2), 2);
        assert_eq!(bits(255), 8);
        assert_eq!(bits(256), 9);
    }
}
