//! `split` — one document into many, RFC 0002 section 6.1.
//!
//! The suite's first verb on a whole-file writer, and the first thing this project has written
//! that is a *new* document rather than an update to one somebody else made. What makes that
//! legal is `CLAUDE.md`'s redrawn exclusion (RFC 0002 section 11.1, ratified 2026-09-03, ADR 0816):
//! assembling documents from existing documents is in scope, and "every content stream in their
//! output is a producer's, carried byte for byte".
//!
//! # What a piece is
//!
//! A new file whose page tree is one flat `/Pages` node over the pages the piece names, in the
//! order it names them, each page object the source's own with two changes and no others:
//!
//! - **`/Parent` names the piece's page tree**, which Table 30 requires of every page and which
//!   cannot be the source's node, since that node is not coming along.
//! - **ISO 32000-2 §7.7.3.4's inheritable attributes are flattened onto the page.** The clause:
//!
//!   > If such an attribute is omitted from a page object, its value shall be inherited from an
//!   > ancestor node in the page tree.
//!
//!   Table 31 marks exactly four of a page's entries inheritable — `/Resources`, `/MediaBox`,
//!   `/CropBox` and `/Rotate` — and the ancestors that carried them are not coming along, so an
//!   attribute the source stated on one of them is written onto the page here. One the page
//!   states itself is left alone, because the clause makes inheritance what happens when the
//!   entry is *omitted*. The value is copied **unresolved**, so a `/Resources` several pages
//!   shared through one indirect object is still one object in the piece.
//!
//! Everything else in the page dictionary — its `/Contents`, its `/Annots`, its `/Group`, its
//! `/StructParents` — is the producer's, and reaches the output through the closure walk below
//! without being read for meaning.
//!
//! # The closure, and where it stops
//!
//! From each emitted page, every indirect reference is followed and the object it names is
//! copied, transitively. It stops at exactly one thing: **an object that is a page or a
//! page-tree node**. A reference to a page *of this piece* becomes that page's new number; a
//! reference to any other page becomes `null`, which is §7.3.10's own answer — "[a]n indirect
//! reference to an undefined object … shall be treated as a reference to the null object" — and
//! is counted and warned about, because RFC 0002 section 6.1 requires that destinations pointing out of
//! the piece are "dropped with a warning (exit 3), not silently".
//!
//! Without that stop a split would copy the whole document into every piece, since a page's
//! `/Parent` reaches the tree and the tree reaches every other page.
//!
//! # What a piece carries, and what it does not
//!
//! The catalog is synthesised. It carries `/Pages`, and the entries whose absence would change
//! what the pages *look like*: `/Version`, `/Lang`, `/ViewerPreferences`, `/PageLayout`,
//! `/PageMode`, §8.11's `/OCProperties`, §12.7.3's `/AcroForm`, §14.11.5's
//! `/OutputIntents` and — since session 897, through [`crate::structure`] — §14.7's
//! `/StructTreeRoot` and `/MarkInfo`.
//!
//! Since session 910 it also carries RFC 0002 section 6.1's three document-level constructs, and
//! [`carry_navigation`] holds the derivation of each: §12.3.3's outline pruned to the items that
//! reach the piece, §12.4.2's labels recomputed one entry per page, and §12.3.2.4's named
//! destinations subsetted to the ones that resolve inside it. Everything else the source catalog
//! states — `/Metadata`, `/Threads`, `/Collection` and the rest of [`NOT_CARRIED`] — is **not
//! carried, and every one of them is named in a warning**, because a construct nobody thought
//! about would otherwise be a construct nobody is told about.
//!
//! # Encryption
//!
//! A piece of an encrypted document is **not encrypted**: the serializer emits no `/Encrypt`
//! (its module comment says why), and every object it is handed is plaintext because a
//! `pdf_syntax::Document` decrypts on load. That is a warning on every piece, by name, and
//! `--restrictions=on` is what refuses the operation outright before it starts.
//!
//! # Determinism and parallelism
//!
//! Pieces are independent, so they are written across rayon (RFC 0002 section 12: a transform is
//! throughput-first). The report is assembled in piece order whatever order the threads
//! finished in, and each piece's bytes are a function of its sources and its plan alone — RFC
//! 0002 §9's first layer, with no flag and no clock.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::Write as _;
use std::sync::Arc;

use pdf_model::Pages;
use pdf_model::destination::Destination;
use pdf_model::outline::{Item, Outline};
use pdf_model::page_label::PageLabels;
use pdf_model::retrieval::sections;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::{Assembly, AssemblyError, Form, Options, serialize};
use pdf_syntax::{Document, Version};
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::pattern::{Fill, Pattern};
use crate::range::Selection;
use crate::structure::{CarriedPage, Carry, Host};
use crate::{Declined, Origin, Output, Refusal, Report, Sinks, Warning, structure};

/// One document into many files.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitPlan {
    /// Which source.
    pub source: usize,
    /// Which pages, in which order.
    pub pages: Selection,
    /// How the selected pages are cut into pieces.
    pub pieces: Pieces,
    /// How the outputs are named.
    pub names: Pattern,
}

/// Where the cuts are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pieces {
    /// One file per page — pdftk's `burst`, poppler's `pdfseparate`, and the default.
    EachPage,
    /// Pieces of this many pages, the last one shorter where the count does not divide —
    /// qpdf's `--split-pages=n`.
    Every(usize),
    /// One file per comma-separated group of the selection, which is RFC 0002 section 6.1's
    /// `--pages 1-3,7-end` writing two files.
    Groups,
    /// One file per §12.3.3 outline item at this depth or shallower — RFC 0002 section 6.1's
    /// `--at-bookmarks[=depth]`, with 1 naming the top-level items.
    ///
    /// [`bookmark_cuts`] states where a piece begins and [`cut`] states what runs to the next
    /// one.
    AtBookmarks(usize),
}

/// One piece of the split: the pages it holds, and the title of the item that begins it.
struct Cut {
    /// Its pages, as zero-based indices into the source.
    pages: Vec<usize>,
    /// Table 151's `/Title` of the outline item that cut it, for the pattern's `%t`. `None` for
    /// every other mode, and for the leading piece of an `--at-bookmarks` split.
    title: Option<String>,
}

/// How deep a page dictionary's own value tree is rewritten before the tail is dropped.
///
/// The parser's own `max_depth` is 256, so nothing it admitted is deeper; a page dictionary
/// deeper than this could only come from a synthesised object, and there are none here.
const MAX_DEPTH: usize = 257;

/// How far up §7.7.3.4's `/Parent` chain an inheritable attribute is looked for.
///
/// The clause makes the chain a tree's height, and a tree over a million pages is twenty levels
/// at any sane branching factor. The bound exists because `/Parent` is a reference a file states
/// and a hostile file can state a cycle; `Document::get` already answers null to one, but a walk
/// that trusted the chain to end would still not terminate on a long enough one.
const MAX_ANCESTORS: usize = 64;

/// The four entries Table 31 marks "( Required; inheritable )" or "( Optional; inheritable )".
///
/// Written out rather than derived, because the table is the only place the standard says which
/// entries are inheritable and §7.7.3.3 makes that list closed: "[a]ttributes that are not
/// explicitly identified in the table as inheritable shall not be inherited."
const INHERITABLE: [&str; 4] = ["Resources", "MediaBox", "CropBox", "Rotate"];

/// The catalog entries a piece carries, because a page drawn without them draws differently.
///
/// Nothing here names a page: `/OCProperties` names optional-content groups (§8.11.4.3) and
/// `/AcroForm` names fields, and both reach their own objects through the closure walk. The
/// list is short on purpose — an entry is here because it changes the marks, not because it
/// would be nice to keep.
///
/// **`/OutputIntents` is here on evidence rather than on foresight.** §14.11.5 makes an output
/// intent a statement about the *document*'s colour, and `pdf_model::content::colour` reads its
/// `/DestOutputProfile` to decide what a device colour means; two corpus documents drew
/// differently without it — `issue17671.pdf` and `issue20513.pdf`, the only two of the 974 that
/// state one on page one — which is the walk finding what the reading had missed. The clause
/// makes it the right entry to carry as well as the observed one: a piece's pages mark with the
/// colours the source's pages marked with, and the intent is what says so.
const CARRIED: [&str; 8] = [
    "Version",
    "Lang",
    "ViewerPreferences",
    "PageLayout",
    "PageMode",
    "OCProperties",
    "AcroForm",
    "OutputIntents",
];

/// The catalog entries a piece does not carry and names in a warning where the source has one.
///
/// Every one of them is a document-level construct whose *subsetting* is a decision RFC 0002
/// §6.1 describes and this verb has not taken. Listing them here rather than warning about
/// whatever is left over is deliberate: a construct nobody thought about is then a construct
/// nobody is told about, and this array is where the thinking is recorded.
///
/// `/Outlines`, `/Names`, `/Dests` and `/PageLabels` **left this list in session 910** and are
/// carried by [`carry_navigation`], which is where the three clauses that decide them are read.
const NOT_CARRIED: [&str; 8] = [
    "Metadata",
    "Threads",
    "SpiderInfo",
    "Collection",
    "Perms",
    "Legal",
    "Requirements",
    "DPartRoot",
];

/// One piece's assembly, and the bookkeeping the walk needs.
struct Piece<'a> {
    /// The document being split.
    document: &'a Document,
    /// The object table being built.
    assembly: Assembly<'a>,
    /// Every page object the document has, so that the walk knows where to stop.
    all_pages: &'a BTreeSet<ObjectId>,
    /// This piece's pages, and the output number each was given.
    mine: Vec<(ObjectId, ObjectId)>,
    /// Objects whose contents have still to be reached.
    pending: VecDeque<ObjectId>,
    /// Objects that cross changed, and the slot each was given.
    to_rebuild: VecDeque<(ObjectId, ObjectId)>,
    /// How many references named a page outside the piece and became null.
    dropped: u64,
    /// Source objects the walk refuses outright: §14.7's elements that reach no kept page.
    blocked: BTreeSet<ObjectId>,
    /// Whether the document states a §14.7 structure tree this piece is carrying.
    ///
    /// Asked by [`Piece::map`] from the first object the walk reaches, before [`Carry::plan`]
    /// has answered, so that an object stating Table 359's `/StructParent` crosses *replaced*
    /// and can have its key renumbered.
    tagged: bool,
    /// §14.7's carry, once planned.
    structure: Option<Carry>,
}

impl Piece<'_> {
    /// The new number for a source object, or `None` where the walk stops at it.
    fn map(&mut self, id: ObjectId) -> Option<ObjectId> {
        if let Some((_, placed)) = self.mine.iter().find(|(source, _)| *source == id) {
            return Some(*placed);
        }
        if self.blocked.contains(&id) || self.all_pages.contains(&id) || self.is_tree_node(id) {
            self.dropped = self.dropped.saturating_add(1);
            return None;
        }
        if let Some(already) = self.assembly.copied(0, id) {
            return Some(already);
        }
        // §14.7.5.4's third home. Table 359 puts `/StructParent` on "the stream dictionary of a
        // form or image XObject, or in an annotation dictionary", and its value is the key of
        // "this object's entry in the structural parent tree" — the *piece's* tree, so the
        // object crosses with its key restated rather than byte for byte.
        if self.tagged && structure::struct_parent(&self.document.get(id)).is_some() {
            let placed = self.assembly.replace(0, id).ok()?;
            self.to_rebuild.push_back((id, placed));
            return Some(placed);
        }
        let placed = self.assembly.copy(0, id).ok()?;
        self.pending.push_back(id);
        Some(placed)
    }

    /// One object that crosses changed: §14.7.5.4's key restated in the piece's own parent tree.
    ///
    /// A stream is the one object rebuilt that keeps its bytes — the dictionary is rewritten and
    /// the encoded data is the same `Arc` the source holds, never decoded and never re-encoded.
    fn rebuild(&mut self, id: ObjectId) -> Object {
        match self.document.get(id) {
            Object::Dictionary(dict) => {
                let mut out = dict.clone();
                self.restate_structure_key(&mut out);
                self.carry(&Object::Dictionary(out), 0)
            }
            Object::Stream(stream) => {
                let mut dict = stream.dict.clone();
                self.restate_structure_key(&mut dict);
                let Object::Dictionary(dict) = self.carry(&Object::Dictionary(dict), 0) else {
                    return Object::Null;
                };
                Object::Stream(Arc::new(Stream {
                    dict,
                    data: Arc::clone(&stream.data),
                    decryption_failed: stream.decryption_failed,
                }))
            }
            other => self.carry(&other, 0),
        }
    }

    /// §14.7.5.4's key for one object, restated in the piece's own parent tree.
    ///
    /// Removed rather than kept where the carry has nothing to point the key at: a key into a
    /// tree the piece *does* state, naming nothing, tells an assistive processor that the
    /// content has a parent element and then hands it none (ADR 0831 section 2's distinction).
    fn restate_structure_key(&mut self, dict: &mut Dictionary) {
        let Some(old) = dict.get("StructParent").and_then(Object::as_integer) else {
            return;
        };
        let document = self.document;
        let key = self
            .structure
            .as_mut()
            .and_then(|carry| carry.object_key(document, 0, old));
        match key {
            Some(key) => {
                dict.insert(Name::new(&b"StructParent"[..]), Object::Integer(key));
            }
            None => {
                dict.remove("StructParent");
            }
        }
    }

    /// Whether an object is a page-tree node, which the walk stops at for the same reason a page
    /// is: §7.7.3.2's `/Kids` reaches every other page in the document.
    fn is_tree_node(&self, id: ObjectId) -> bool {
        self.document
            .get_key_of(id, "Type")
            .as_ref()
            .and_then(Object::as_name)
            .is_some_and(|name| name.as_bytes() == b"Pages")
    }

    /// One value with every reference mapped into the piece's numbering.
    ///
    /// Used for the objects this verb *builds* — the emitted page dictionaries and the catalog —
    /// whose references must already be the output's. A copied object needs none of this: the
    /// serializer renumbers it, and answers §7.3.10's null for whatever the piece does not hold.
    fn carry(&mut self, value: &Object, depth: usize) -> Object {
        if depth >= MAX_DEPTH {
            return Object::Null;
        }
        match value {
            Object::Reference(id) => match self.map(*id) {
                Some(placed) => Object::Reference(placed),
                None => Object::Null,
            },
            Object::Array(items) => Object::Array(
                items
                    .iter()
                    .map(|item| self.carry(item, depth.saturating_add(1)))
                    .collect(),
            ),
            Object::Dictionary(dict) => {
                let mut out = Dictionary::new();
                for (key, entry) in dict.iter() {
                    out.insert(key.clone(), self.carry(entry, depth.saturating_add(1)));
                }
                Object::Dictionary(out)
            }
            other => other.clone(),
        }
    }

    /// Copies everything the pending objects reach, transitively.
    ///
    /// The objects themselves cross verbatim, so this only has to *register* what they refer to;
    /// the serializer's own renumbering is what rewrites the references inside them, and what
    /// turns a reference to something never registered into null.
    fn drain(&mut self) {
        loop {
            if let Some((id, placed)) = self.to_rebuild.pop_front() {
                let rebuilt = self.rebuild(id);
                let _ = self.assembly.place(placed, rebuilt);
                continue;
            }
            let Some(id) = self.pending.pop_front() else {
                break;
            };
            let value = self.document.get(id);
            self.reach(&value, 0);
        }
    }

    /// Registers every object one value refers to.
    fn reach(&mut self, value: &Object, depth: usize) {
        if depth >= MAX_DEPTH {
            return;
        }
        match value {
            Object::Reference(id) => {
                let _ = self.map(*id);
            }
            Object::Array(items) => {
                for item in items {
                    self.reach(item, depth.saturating_add(1));
                }
            }
            Object::Dictionary(dict) => {
                for (_, entry) in dict.iter() {
                    self.reach(entry, depth.saturating_add(1));
                }
            }
            Object::Stream(stream) => {
                for (_, entry) in stream.dict.iter() {
                    self.reach(entry, depth.saturating_add(1));
                }
            }
            _ => {}
        }
    }
}

/// [`crate::structure`]'s view of this piece's object table.
///
/// One document, so `at` is always zero; §14.7 is read and written in that module for this verb
/// and for the two on `merge`'s engine alike.
impl Host for Piece<'_> {
    fn source(&self, at: usize) -> Option<&Document> {
        (at == 0).then_some(self.document)
    }

    fn carry_value(&mut self, _at: usize, value: &Object) -> Object {
        self.carry(value, 0)
    }

    fn reserve_slot(&mut self) -> Result<ObjectId, AssemblyError> {
        self.assembly.reserve()
    }

    fn replace_object(&mut self, _at: usize, id: ObjectId) -> Result<ObjectId, AssemblyError> {
        self.assembly.replace(0, id)
    }

    fn place_object(&mut self, id: ObjectId, object: Object) {
        // The slot was reserved by that module a moment ago and nothing else can have filled it.
        drop(self.assembly.place(id, object));
    }

    fn block_object(&mut self, _at: usize, id: ObjectId) {
        self.blocked.insert(id);
    }
}

/// §7.7.3.4's value for one inheritable attribute, taken unresolved from the nearest ancestor.
///
/// > If such an attribute is omitted from a page object, its value shall be inherited from an
/// > ancestor node in the page tree.
///
/// Unresolved because sharing is worth keeping: a hundred pages under one `/Pages` node with one
/// indirect `/Resources` become a hundred pages naming one object, not a hundred copies.
fn inherited(document: &Document, page: ObjectId, key: &str) -> Option<Object> {
    // The reference itself, not what it resolves to: `Document::get_key_of` resolves, and an
    // ancestor is reached by walking references rather than by holding one node at a time.
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

/// What one piece produced, or why it did not.
struct Done {
    /// The output, or the refusal that named the piece.
    outcome: Result<Output, Problem>,
    /// What was met on the way.
    warnings: Vec<Warning>,
}

/// Why one piece produced nothing.
enum Problem {
    /// This program declined the piece by name; the others were still written.
    Declined(Declined),
    /// The sink failed, which stops everything.
    Sink(String, std::io::Error),
}

/// Cuts the selection into pieces and writes each one.
pub(crate) fn run(
    plan: &SplitPlan,
    document: &Document,
    sinks: &dyn Sinks,
    report: &mut Report,
) -> Result<(), Refusal> {
    let pages = Pages::new(document);
    let labels = PageLabels::read(document);
    let groups = plan
        .pages
        .groups(pages.len(), |index| labels.label(index))
        .map_err(|error| Refusal::Selection {
            at: plan.source,
            error,
        })?;
    let indices = pages.indices();
    let outline = Outline::read(document, &pages);
    let marks = match plan.pieces {
        Pieces::AtBookmarks(depth) => {
            let marks = bookmark_cuts(document, &pages, &outline, depth);
            if marks.is_empty() {
                return Err(Refusal::NoBookmarks {
                    at: plan.source,
                    depth,
                });
            }
            marks
        }
        Pieces::EachPage | Pieces::Every(_) | Pieces::Groups => BTreeMap::new(),
    };
    let cuts = cut(&groups, plan.pieces, &marks);
    if !plan.names.distinguishes(cuts.len()) {
        return Err(Refusal::Pattern(format!(
            "{} pieces would be written and the output name {:?} has no %d to tell them apart",
            cuts.len(),
            plan.names.to_string()
        )));
    }
    if plan.names.names_a_title() && !matches!(plan.pieces, Pieces::AtBookmarks(_)) {
        return Err(Refusal::Pattern(
            "%t names a title, and only --at-bookmarks gives a piece one".to_owned(),
        ));
    }

    let all_pages: BTreeSet<ObjectId> = indices.keys().copied().collect();
    let version = document.version().unwrap_or(Version { major: 1, minor: 7 });
    let form = Form::of(document);
    let count = cuts.len();

    let done: Vec<Done> = cuts
        .par_iter()
        .enumerate()
        .map(|(ordinal, piece)| {
            write_piece(&Job {
                plan,
                document,
                pages: &pages,
                labels: &labels,
                all_pages: &all_pages,
                indices: &indices,
                outline: &outline,
                version,
                form,
                sinks,
                count,
                ordinal,
                piece: &piece.pages,
                title: piece.title.as_deref(),
            })
        })
        .collect();

    for Done { outcome, warnings } in done {
        report.warnings.extend(warnings);
        match outcome {
            Ok(output) => report.outputs.push(output),
            Err(Problem::Declined(declined)) => report.refused.push(declined),
            Err(Problem::Sink(name, error)) => return Err(Refusal::Sink { name, error }),
        }
    }
    Ok(())
}

/// Everything one piece's job needs, so that the parallel closure takes one value.
struct Job<'a> {
    /// The plan.
    plan: &'a SplitPlan,
    /// The document.
    document: &'a Document,
    /// Its pages.
    pages: &'a Pages<'a>,
    /// Its §12.4.2 labels, for `%l`.
    labels: &'a PageLabels,
    /// Every page object it has.
    all_pages: &'a BTreeSet<ObjectId>,
    /// Its pages by object number, prepared once so that resolving many destinations is not a
    /// page-tree walk apiece.
    indices: &'a BTreeMap<ObjectId, usize>,
    /// §12.3.3's outline, read once for the whole split.
    outline: &'a Outline,
    /// The header's version.
    version: Version,
    /// Which cross-reference form.
    form: Form,
    /// Where the output goes.
    sinks: &'a dyn Sinks,
    /// How many pieces there are.
    count: usize,
    /// Which piece this is, from 0.
    ordinal: usize,
    /// Its pages, as zero-based indices into the source.
    piece: &'a [usize],
    /// The title of the outline item that began it, where `--at-bookmarks` gave it one.
    title: Option<&'a str>,
}

/// Where the pieces of an `--at-bookmarks` split begin: a page, and the title of the item.
///
/// §12.3.3's outline is "a tree-structured hierarchy of outline items … which serve as a visual
/// table of contents", and §12.3.2's destinations are what turn one into a page. The resolution
/// is `pdf_model::retrieval::sections`, the machinery ADR 0257 built for `pdf-retrieve`, so a
/// destination is read here exactly as the reader reads it — a named one through §12.3.2.4's two
/// tables included.
///
/// **`depth` counts §12.3.3's levels from 1**, so `--at-bookmarks=1` cuts at the top-level items
/// and `=2` at those and their immediate children; a [`Section`](sections)'s own `depth` counts
/// the same levels from 0. Where two items land on one page the *first* in the outline's own
/// order names the piece, which makes the answer a function of the file rather than of the
/// iteration.
fn bookmark_cuts(
    document: &Document,
    pages: &Pages<'_>,
    outline: &Outline,
    depth: usize,
) -> BTreeMap<usize, String> {
    let mut out: BTreeMap<usize, String> = BTreeMap::new();
    for section in sections(document, pages, outline) {
        if section.depth < depth.max(1) {
            out.entry(section.first_page).or_insert(section.title);
        }
    }
    out
}

/// The selected pages cut into pieces.
///
/// **Where a piece begins**, for `--at-bookmarks`: at every selected page that an outline item
/// at the stated depth or shallower resolves to, and each piece runs to the page before the next
/// such page. The pages before the first mark are a piece of their own with no title — a split
/// whose pieces do not cover the selection would have lost pages, and front matter ahead of the
/// first bookmark is exactly that case. A page that carries two marks starts one piece, not two.
fn cut(groups: &[Vec<usize>], pieces: Pieces, marks: &BTreeMap<usize, String>) -> Vec<Cut> {
    let untitled = |pages: Vec<usize>| Cut { pages, title: None };
    match pieces {
        Pieces::Groups => groups.iter().cloned().map(untitled).collect(),
        Pieces::EachPage => groups
            .concat()
            .into_iter()
            .map(|page| untitled(vec![page]))
            .collect(),
        Pieces::Every(every) => groups
            .concat()
            .chunks(every.max(1))
            .map(|pages| untitled(pages.to_vec()))
            .collect(),
        Pieces::AtBookmarks(_) => {
            let mut out = Vec::new();
            let mut current: Vec<usize> = Vec::new();
            let mut title: Option<String> = None;
            for page in groups.concat() {
                if let Some(mark) = marks.get(&page) {
                    if !current.is_empty() {
                        out.push(Cut {
                            pages: std::mem::take(&mut current),
                            title: title.take(),
                        });
                    }
                    title = Some(mark.clone());
                }
                current.push(page);
            }
            if !current.is_empty() {
                out.push(Cut {
                    pages: current,
                    title,
                });
            }
            out
        }
    }
}

/// Builds and writes one piece.
fn write_piece(job: &Job<'_>) -> Done {
    let mut warnings = Vec::new();
    let first = job.piece.first().copied().unwrap_or_default();
    let label = job.labels.label(first);
    let expanded = job.plan.names.expand(&Fill {
        ordinal: job.ordinal.saturating_add(1),
        count: job.count,
        page: Some(first.saturating_add(1)),
        label: label.as_deref(),
        title: job.title,
    });

    let assembly = match assemble(job, &expanded.name, &mut warnings) {
        Ok(assembly) => assembly,
        Err(detail) => {
            return Done {
                outcome: Err(Problem::Declined(Declined {
                    source: job.plan.source,
                    page: Some(first.saturating_add(1)),
                    subject: expanded.name,
                    detail,
                })),
                warnings,
            };
        }
    };

    let mut writer = match job.sinks.open(&expanded.name) {
        Ok(writer) => writer,
        Err(error) => {
            return Done {
                outcome: Err(Problem::Sink(expanded.name, error)),
                warnings,
            };
        }
    };
    let written = match serialize(&assembly, job.version, Options::new(job.form), &mut writer) {
        Ok(written) => written,
        Err(error) => {
            return Done {
                outcome: Err(Problem::Declined(Declined {
                    source: job.plan.source,
                    page: Some(first.saturating_add(1)),
                    subject: expanded.name.clone(),
                    detail: error.to_string(),
                })),
                warnings,
            };
        }
    };
    if let Err(error) = writer.flush() {
        return Done {
            outcome: Err(Problem::Sink(expanded.name, error)),
            warnings,
        };
    }

    Done {
        outcome: Ok(Output {
            name: expanded.name,
            bytes: written.bytes,
            sanitised: expanded.sanitised,
            origin: Origin::Piece {
                source: job.plan.source,
                first_page: first.saturating_add(1),
                pages: job.piece.len(),
                label,
                objects: written.objects,
            },
        }),
        warnings,
    }
}

/// The piece's object table: its pages, their closure, its page tree and its catalog.
///
/// `Err` is a sentence naming why this piece cannot be assembled, which the caller turns into a
/// refusal by name — the other pieces are still written, which is what a batch tool owes.
fn assemble<'a>(
    job: &Job<'a>,
    name: &str,
    warnings: &mut Vec<Warning>,
) -> Result<Assembly<'a>, String> {
    /// The one sentence every numbering failure gets, since they all mean the same thing.
    const TOO_MANY: &str = "the piece needs more objects than one file can number";

    let mut piece = Piece {
        document: job.document,
        assembly: Assembly::new(vec![job.document]),
        all_pages: job.all_pages,
        mine: Vec::new(),
        pending: VecDeque::new(),
        to_rebuild: VecDeque::new(),
        dropped: 0,
        blocked: BTreeSet::new(),
        tagged: structure::states_a_tree(Some(job.document)),
        structure: None,
    };

    // The catalog and the page tree take the first two numbers, so that a piece's numbering
    // starts where a reader looking at it would expect and does not depend on how many objects
    // the pages happened to reach.
    let catalog = piece.assembly.reserve().map_err(|_| TOO_MANY.to_owned())?;
    let tree = piece.assembly.reserve().map_err(|_| TOO_MANY.to_owned())?;

    // Every page is given its number before any of them is built, so that a reference from one
    // page's annotation to another page of the same piece maps to a page rather than to null.
    for index in job.piece {
        let id = job
            .pages
            .get(*index)
            .and_then(|page| page.id)
            .ok_or_else(|| {
                format!(
                    "page {} is not an indirect object, and a page tree's /Kids is \"an array of \
                 indirect references\" (§7.7.3.2)",
                    index.saturating_add(1)
                )
            })?;
        let placed = piece
            .assembly
            .replace(0, id)
            .map_err(|error| error.to_string())?;
        piece.mine.push((id, placed));
    }

    piece.structure = plan_structure(&mut piece, job, warnings).map_err(|e| e.to_string())?;

    for (source, placed) in piece.mine.clone() {
        let page = build_page(&mut piece, source, tree);
        piece
            .assembly
            .place(placed, page)
            .map_err(|error| error.to_string())?;
    }

    let mut root = build_catalog(&mut piece, job, tree, name, warnings);
    piece.drain();
    // The elements, the parent tree and the structure tree root, built last because the object
    // keys above are assigned by the walk that has just finished — and drained again, because an
    // element's attributes reach objects nothing else did.
    if let Some(carry) = piece.structure.take() {
        carry.finish(&mut piece, warnings);
        piece.drain();
    }
    // RFC 0002 section 6.1's document-level carrying, last because two of its three answers are
    // *what the closure walk copied* — and drained again, because carrying a destination or a
    // name-tree value reaches objects nothing else did.
    carry_navigation(&mut piece, job, &mut root, name, warnings);
    piece.drain();

    let kids: Vec<Object> = piece
        .mine
        .iter()
        .map(|(_, placed)| Object::Reference(*placed))
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
    piece
        .assembly
        .place(tree, Object::Dictionary(node))
        .and_then(|()| piece.assembly.place(catalog, Object::Dictionary(root)))
        .map_err(|error| error.to_string())?;
    piece.assembly.set_root(catalog);

    carry_info(&mut piece, job);
    report_losses(&piece, job, name, warnings);
    Ok(piece.assembly)
}

/// §14.3.3's information dictionary, which is one object with no page in it, so it crosses whole.
///
/// Nothing here writes a `/ModDate`: this crate has no clock, and RFC 0002 section 9 makes the
/// absence of one the determinism the gates rest on.
fn carry_info(piece: &mut Piece<'_>, job: &Job<'_>) {
    if let Some(source) = job
        .document
        .trailer()
        .get("Info")
        .and_then(Object::as_reference)
        && let Some(placed) = piece.map(source)
    {
        piece.assembly.set_info(Some(placed));
        piece.drain();
    }
}

/// What the piece lost that no carrying could keep: §7.6's protection, and §7.3.10's nulls.
fn report_losses(piece: &Piece<'_>, job: &Job<'_>, name: &str, warnings: &mut Vec<Warning>) {
    if piece.assembly.has_encrypted_source() {
        warnings.push(Warning {
            source: job.plan.source,
            page: None,
            detail: format!(
                "{name}: the source is encrypted (§7.6) and this piece is not; the serializer \
                 writes no /Encrypt"
            ),
        });
    }
    if piece.dropped > 0 {
        warnings.push(Warning {
            source: job.plan.source,
            page: None,
            detail: format!(
                "{name}: {} reference(s) named a page outside the piece and were written as \
                 §7.3.10's null",
                piece.dropped
            ),
        });
    }
}

/// §14.7's carry, planned after every page has its number and before any page is built.
///
/// The order is what makes it work: a page's `/StructParents` is the carry's to state, so it has
/// to be decided before [`build_page`] runs, and every kept element needs its slot before the
/// closure walk can reach one by reference. A piece is one document's, so no cross-source
/// collision is possible and the refusal path cannot fire.
///
/// # Errors
///
/// [`Refusal::Assembly`] where the numbering is spent.
fn plan_structure(
    piece: &mut Piece<'_>,
    job: &Job<'_>,
    warnings: &mut Vec<Warning>,
) -> Result<Option<Carry>, Refusal> {
    let carried: Vec<CarriedPage> = piece
        .mine
        .iter()
        .map(|(source, placed)| CarriedPage {
            at: 0,
            source: *source,
            placed: *placed,
            duplicate: false,
        })
        .collect();
    let source = job.plan.source;
    Carry::plan(piece, &[0], &carried, warnings, &|_| source)
}

/// One emitted page: the source's dictionary, its `/Parent` replaced, §7.7.3.4's inheritance
/// flattened onto it, and every reference in it mapped into the piece.
fn build_page(piece: &mut Piece<'_>, source: ObjectId, tree: ObjectId) -> Object {
    let structure_key = piece
        .mine
        .iter()
        .find(|(from, _)| *from == source)
        .and_then(|(_, placed)| {
            piece
                .structure
                .as_ref()
                .and_then(|carry| carry.page_key(*placed))
        });
    // §7.7.3.3 makes a page a dictionary, and `Pages` would not have counted anything else as
    // one — so an empty dictionary here is a page the reader already disowned, not a panic.
    let dict = match piece.document.get(source) {
        Object::Dictionary(dict) => dict,
        _ => Dictionary::new(),
    };
    let mut out = Dictionary::new();
    for (key, value) in dict.iter() {
        if key.as_bytes() == b"Parent" {
            continue;
        }
        let carried = piece.carry(value, 0);
        out.insert(key.clone(), carried);
    }
    for key in INHERITABLE {
        if out.get(key).is_none()
            && let Some(value) = inherited(piece.document, source, key)
        {
            let carried = piece.carry(&value, 0);
            out.insert(Name::new(key.as_bytes()), carried);
        }
    }
    // Table 359's `/StructParents` is "[t]he integer key of this object's entry in the
    // structural parent tree" — the piece's tree, so the carry states it, and a page whose
    // source stated one that this piece has no tree for loses the entry rather than keeping a
    // number that names nothing.
    if piece.structure.is_some() {
        out.remove("StructParents");
        if let Some(key) = structure_key {
            out.insert(Name::new(&b"StructParents"[..]), Object::Integer(key));
        }
    }
    out.insert(Name::new(&b"Parent"[..]), Object::Reference(tree));
    Object::Dictionary(out)
}

/// The piece's catalog, and the warning naming every document-level construct left behind.
fn build_catalog(
    piece: &mut Piece<'_>,
    job: &Job<'_>,
    tree: ObjectId,
    name: &str,
    warnings: &mut Vec<Warning>,
) -> Dictionary {
    let source_catalog = job.document.catalog().unwrap_or_default();
    let mut root = Dictionary::new();
    root.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Catalog"[..])),
    );
    root.insert(Name::new(&b"Pages"[..]), Object::Reference(tree));
    for key in CARRIED {
        if let Some(value) = source_catalog.get(key) {
            let carried = piece.carry(value, 0);
            root.insert(Name::new(key.as_bytes()), carried);
        }
    }
    // §14.7.2 locates the whole construct: the structure tree root is "located by means of the
    // StructTreeRoot entry in the document catalog dictionary".
    if let Some((structure, mark_info)) = piece
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
    let left_behind: Vec<&str> = NOT_CARRIED
        .into_iter()
        .filter(|key| source_catalog.get(key).is_some())
        .collect();
    if !left_behind.is_empty() {
        warnings.push(Warning {
            source: job.plan.source,
            page: None,
            detail: format!(
                "{name}: the document states /{} and a piece carries none of them; subsetting them \
                 is doc/todo/57's",
                left_behind.join(", /")
            ),
        });
    }
    root
}

/// RFC 0002 section 6.1's three document-level constructs, written onto the piece's catalog.
///
/// The three are one question — *what does a document say about its pages that a piece of it
/// still says* — and ISO 32000-2 answers it three different ways. Each verdict below is the
/// clause's, and the code under it is what the verdict allows.
///
/// **§12.3.3's outline: permitted.** "A PDF document may contain a document outline", so a piece
/// without one conforms and nothing requires a derivative to keep its source's. What binds is
/// the *shape*, once there is one: Table 150 makes `/First` and `/Last` "( Required if there are
/// any open or closed outline entries )", Table 151 makes `/Parent` required with "[t]he parent
/// of a top-level item … the outline dictionary itself", `/Prev` and `/Next` required at every
/// position but the ends of a level, and `/Count` required of an item "if the item has any
/// descendants". So the piece keeps the subset of the hierarchy that reaches its pages and
/// rebuilds all five, and [`visible_descendants`] runs §12.3.3's own counting algorithm over the
/// subset rather than carrying a number that counted something else.
///
/// **§12.4.2's labels: permitted, and the source's own are forbidden.** "[A] document may
/// optionally define page labels" — permitted. But a label is a *position*. The clause makes a
/// page index "the page's relative position within the document", makes the `/PageLabels` number
/// tree's keys "the page index of the first page in a labelling range", and states outright that
/// "[t]he tree shall include a value for page index 0". A piece is a document and its indices run
/// from 0, so a piece that begins at the source's page 13 and carried the source's tree would
/// state a tree whose lowest key is 12 and which therefore breaks that `shall`. The labels are
/// recomputed instead, one entry per page, each stating the label that page carried in the
/// source — which is `merge`'s construction seen from the other side, and is `merge`'s code.
///
/// **§12.3.2.4's named destinations: carried by whatever still names them.** A destination may be
/// "referred to indirectly by means of a name object … or a byte string", and the correspondence
/// "shall be defined by the Dests entry in the document catalog dictionary" or, in PDF 1.2 and
/// later, by the `/Dests` name tree. §7.3.10's null — the answer this verb gives every reference
/// to a page it does not hold — **is not available here**, because a name is not an indirect
/// reference: a piece that kept a link and dropped the tables would state a destination the
/// standard gives no meaning to at all. So the entries that resolve into the piece are carried,
/// the rest are dropped and counted, and an outline item whose own destination is among them
/// loses the entry rather than naming nothing.
fn carry_navigation(
    piece: &mut Piece<'_>,
    job: &Job<'_>,
    root: &mut Dictionary,
    name: &str,
    warnings: &mut Vec<Warning>,
) {
    let held: BTreeSet<usize> = job.piece.iter().copied().collect();

    let carried = carry_name_trees(piece, job, &held);
    if let Some(names) = carried.names {
        root.insert(Name::new(&b"Names"[..]), names);
    }
    if let Some(dests) = carried.dests {
        root.insert(Name::new(&b"Dests"[..]), dests);
    }
    if carried.dropped > 0 {
        warnings.push(Warning {
            source: job.plan.source,
            page: None,
            detail: format!(
                "{name}: {} §7.9.6 name-tree entr(ies) named something this piece does not hold \
                 and were dropped; anything left in the piece that states one of those keys now \
                 names nothing, which §7.3.10's null cannot answer because a name is not an \
                 indirect reference",
                carried.dropped
            ),
        });
    }

    if let Some(labels) = crate::merge::page_labels(
        std::slice::from_ref(job.labels),
        &job.piece
            .iter()
            .map(|index| (0_usize, *index))
            .collect::<Vec<_>>(),
    ) {
        root.insert(Name::new(&b"PageLabels"[..]), labels);
    }

    let kept = keep_items(job, &held, &job.outline.items);
    if kept.is_empty() {
        if !job.outline.items.is_empty() {
            warnings.push(Warning {
                source: job.plan.source,
                page: None,
                detail: format!(
                    "{name}: the document states §12.3.3's outline and no item in it resolves to \
                     a page of this piece, so the piece has none"
                ),
            });
        }
        return;
    }
    match place_outline(piece, &kept) {
        Some(outlines) => {
            root.insert(Name::new(&b"Outlines"[..]), Object::Reference(outlines));
        }
        None => warnings.push(Warning {
            source: job.plan.source,
            page: None,
            detail: format!(
                "{name}: §12.3.3's outline subset needs more objects than this file can number, \
                 so the piece carries none"
            ),
        }),
    }
}

/// What §7.9.6's name trees leave in one piece.
struct CarriedNames {
    /// §7.7.4's name dictionary, where any category kept an entry.
    names: Option<Object>,
    /// §12.3.2.4's PDF 1.1 home, the catalog's own `/Dests` dictionary.
    dests: Option<Object>,
    /// How many entries were dropped because what they named is not in the piece.
    dropped: usize,
}

/// The name-tree entries this piece still reaches, in both of §12.3.2.4's homes.
///
/// Two tests, because a name tree is used two ways and the standard says so in two places:
///
/// - **`/Dests` is entered by name**, so an entry survives when the destination it holds
///   resolves to a page the piece holds. That is §12.3.2.4's own operation, run by
///   `Destination::read` — the reader's, so the tree is asked here exactly as it is asked when a
///   link is followed.
/// - **Every other category maps a name to an *object***, and the piece reaches those objects
///   through its pages rather than by name: §12.5.6.15's file attachment annotation names its
///   own file specification, a widget names its own appearance. So an entry survives when the
///   closure walk has already copied what it names, and is dropped when it has not — a tree
///   listing objects the piece does not hold would be an index of nothing.
///
/// No key can collide, because a piece has one source; §7.9.6's "[t]he keys contained within the
/// various nodes' Names entries shall not overlap" is `merge`'s problem and not this one.
fn carry_name_trees(piece: &mut Piece<'_>, job: &Job<'_>, held: &BTreeSet<usize>) -> CarriedNames {
    let document = job.document;
    let mut out = CarriedNames {
        names: None,
        dests: None,
        dropped: 0,
    };
    let mut catalog: BTreeMap<Vec<u8>, Object> = BTreeMap::new();
    for (key, value) in crate::merge::catalog_dests(document) {
        if lands_in(job, held, &value) {
            let carried = piece.carry(&value, 0);
            catalog.insert(key, carried);
        } else {
            out.dropped = out.dropped.saturating_add(1);
        }
    }
    let mut trees: Vec<(&'static str, BTreeMap<Vec<u8>, Object>)> = Vec::new();
    for category in crate::merge::NAME_TREES {
        let mut kept: BTreeMap<Vec<u8>, Object> = BTreeMap::new();
        for (key, value) in crate::merge::tree_entries(document, category) {
            let keep = if category == "Dests" {
                lands_in(job, held, &value)
            } else {
                value
                    .as_reference()
                    .is_some_and(|id| piece.assembly.copied(0, id).is_some())
            };
            if keep {
                let carried = piece.carry(&value, 0);
                kept.insert(key, carried);
            } else {
                out.dropped = out.dropped.saturating_add(1);
            }
        }
        if !kept.is_empty() {
            trees.push((category, kept));
        }
    }

    // Table 33 closes §7.7.4's name dictionary, so a key outside [`crate::merge::NAME_TREES`] is
    // one this program has no rule for subsetting; it is counted rather than passed through, so
    // that the warning is about everything the piece lost rather than everything it recognised.
    if let Ok(catalog_dict) = document.catalog() {
        let names = document.get_key(&catalog_dict, "Names");
        if let Some(names) = names.as_dict() {
            for (key, _) in names.iter() {
                if !crate::merge::NAME_TREES
                    .iter()
                    .any(|known| known.as_bytes() == key.as_bytes())
                {
                    out.dropped = out.dropped.saturating_add(1);
                }
            }
        }
    }

    if !trees.is_empty() {
        let mut dict = Dictionary::new();
        for (category, entries) in trees {
            // §7.9.6: a root that is also a leaf states `/Names`, "an array of the form [ key1
            // value1 key2 value2 … keyn valuen ] … The keys shall be sorted in lexical order",
            // which is what iterating a `BTreeMap` over the key bytes gives.
            let mut array = Vec::new();
            for (key, value) in entries {
                array.push(Object::String(key.as_slice().into()));
                array.push(value);
            }
            let mut node = Dictionary::new();
            node.insert(Name::new(&b"Names"[..]), Object::Array(array));
            dict.insert(Name::new(category.as_bytes()), Object::Dictionary(node));
        }
        out.names = Some(Object::Dictionary(dict));
    }
    if !catalog.is_empty() {
        let mut dict = Dictionary::new();
        for (key, value) in catalog {
            dict.insert(Name::new(key), value);
        }
        out.dests = Some(Object::Dictionary(dict));
    }
    out
}

/// Whether this destination value names a page the piece holds.
fn lands_in(job: &Job<'_>, held: &BTreeSet<usize>, value: &Object) -> bool {
    Destination::read(job.document, value)
        .and_then(|destination| destination.page_index_with(job.document, job.pages, job.indices))
        .is_some_and(|index| held.contains(&index))
}

/// One §12.3.3 outline item a piece keeps, and the kept items under it.
struct Kept {
    /// The source's own object, which is what the rebuilt item is copied from.
    source: ObjectId,
    /// Whether the source states an open `/Count`, which decides this item's sign.
    open: bool,
    /// Whether the item's own destination resolves to a page this piece holds.
    lands_here: bool,
    /// Whether the item states a destination at all — Table 151's `/Dest`, or the `/D` of a
    /// go-to action in its `/A`.
    states_destination: bool,
    /// The kept children, in the source's own order.
    children: Vec<Kept>,
}

/// The outline subset that reaches this piece's pages.
///
/// An item is kept when its own destination lands in the piece **or** when one of its
/// descendants' does, because Table 151 makes `/Parent` "( Required )" and dropping an ancestor
/// would leave a kept item with nothing to name. An item that resolves nowhere at all — §12.3.3
/// permits one whose `/A` runs a script or opens another file — is kept only as such an ancestor,
/// which is `pdf_model::retrieval::sections`'s rule for the same reason.
///
/// The recursion is bounded by the read: `pdf_model::outline` follows at most 32 levels and
/// refuses to visit an object twice, so this walks a finite tree whatever the file says.
fn keep_items(job: &Job<'_>, held: &BTreeSet<usize>, items: &[Item]) -> Vec<Kept> {
    let mut out = Vec::new();
    for item in items {
        let children = keep_items(job, held, &item.children);
        let lands_here = item
            .destination
            .as_ref()
            .and_then(|destination| {
                destination.page_index_with(job.document, job.pages, job.indices)
            })
            .is_some_and(|index| held.contains(&index));
        if !lands_here && children.is_empty() {
            continue;
        }
        out.push(Kept {
            source: item.id,
            open: item.open,
            lands_here,
            states_destination: item.destination.is_some(),
            children,
        });
    }
    out
}

/// Every kept item, pre-order, which is the order the slots are handed out in.
fn flatten<'k>(kept: &'k [Kept], out: &mut Vec<&'k Kept>) {
    for item in kept {
        out.push(item);
        flatten(&item.children, out);
    }
}

/// §12.3.3's own counting algorithm, run over the kept hierarchy.
///
/// > Step 1. Initialize Count to zero. Step 2. Add to Count the number of immediate children.
/// > During repetitions of this step, update only the Count of the original outline item.
/// > Step 3. For each of those immediate children whose Count is positive and non-zero, repeat
/// > steps 2 and 3.
///
/// The answer is the number of descendants that would be visible with this item open, which
/// Table 151 makes `/Count` where the item is open and `-/Count` where it is closed: "[i]f the
/// outline item is closed, Count is negative and its absolute value is the number of descendants
/// that would be visible if the outline item were opened."
fn visible_descendants(item: &Kept) -> i64 {
    let mut total = i64::try_from(item.children.len()).unwrap_or(i64::MAX);
    for child in &item.children {
        if child.open && !child.children.is_empty() {
            total = total.saturating_add(visible_descendants(child));
        }
    }
    total
}

/// The piece's outline dictionary and every item under it, placed; the dictionary's number.
///
/// `None` where the piece's numbering is spent, and then nothing is left half-written: ADR 0817's
/// serializer refuses an assembly holding a slot nobody filled, so every slot already reserved is
/// filled with null before the answer is given.
fn place_outline(piece: &mut Piece<'_>, kept: &[Kept]) -> Option<ObjectId> {
    let mut flat: Vec<&Kept> = Vec::new();
    flatten(kept, &mut flat);
    let mut slots: Vec<ObjectId> = Vec::new();
    // One for the outline dictionary, then one per item in the order [`flatten`] produced.
    for _ in 0..=flat.len() {
        let Ok(id) = piece.assembly.reserve() else {
            for id in slots {
                drop(piece.assembly.place(id, Object::Null));
            }
            return None;
        };
        slots.push(id);
    }
    let root = *slots.first()?;
    let placed: BTreeMap<ObjectId, ObjectId> = flat
        .iter()
        .zip(slots.iter().skip(1))
        .map(|(item, slot)| (item.source, *slot))
        .collect();
    build_level(piece, kept, root, &placed);

    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Outlines"[..])),
    );
    // Table 150: `/First` and `/Last` are "( Required if there are any open or closed outline
    // entries; shall be an indirect reference )", and this piece has entries or it would not be
    // here.
    if let (Some(first), Some(last)) = (kept.first(), kept.last())
        && let (Some(first), Some(last)) = (placed.get(&first.source), placed.get(&last.source))
    {
        dict.insert(Name::new(&b"First"[..]), Object::Reference(*first));
        dict.insert(Name::new(&b"Last"[..]), Object::Reference(*last));
    }
    // Table 150's `/Count` is "( Required if the document has any open outline entries ) Total
    // number of visible outline items at all levels of the outline … This entry shall be omitted
    // if there are no open outline items." Both halves are obeyed against the *subset*: the
    // number is §12.3.3's algorithm over what the piece kept, and where nothing it kept is open
    // the entry is not written at all.
    if flat
        .iter()
        .any(|item| item.open && !item.children.is_empty())
    {
        let mut total = i64::try_from(kept.len()).unwrap_or(i64::MAX);
        for item in kept {
            if item.open {
                total = total.saturating_add(visible_descendants(item));
            }
        }
        dict.insert(Name::new(&b"Count"[..]), Object::Integer(total));
    }
    drop(piece.assembly.place(root, Object::Dictionary(dict)));
    Some(root)
}

/// One level of the kept hierarchy, and everything under it.
fn build_level(
    piece: &mut Piece<'_>,
    level: &[Kept],
    parent: ObjectId,
    placed: &BTreeMap<ObjectId, ObjectId>,
) {
    for (position, item) in level.iter().enumerate() {
        let Some(slot) = placed.get(&item.source).copied() else {
            continue;
        };
        let previous = position
            .checked_sub(1)
            .and_then(|before| level.get(before))
            .and_then(|sibling| placed.get(&sibling.source).copied());
        let next = level
            .get(position.saturating_add(1))
            .and_then(|sibling| placed.get(&sibling.source).copied());
        let first = item
            .children
            .first()
            .and_then(|child| placed.get(&child.source).copied());
        let last = item
            .children
            .last()
            .and_then(|child| placed.get(&child.source).copied());
        let built = build_outline_item(piece, item, parent, [previous, next, first, last]);
        drop(piece.assembly.place(slot, built));
        build_level(piece, &item.children, slot, placed);
    }
}

/// One kept outline item: the source's Table 151 entries, with the hierarchy this piece's own.
///
/// `chain` is `/Prev`, `/Next`, `/First` and `/Last`, in that order — four references that are
/// each "( Required for all but … )" or "( Required if the item has any descendants )", so an
/// absent one is a position at the end of a level rather than an omission.
fn build_outline_item(
    piece: &mut Piece<'_>,
    item: &Kept,
    parent: ObjectId,
    chain: [Option<ObjectId>; 4],
) -> Object {
    // §12.3.3 makes an outline item a dictionary, and `pdf_model::outline` would not have read
    // anything else as one — so an empty dictionary here is an item the reader already disowned.
    let source = match piece.document.get(item.source) {
        Object::Dictionary(dict) => dict,
        _ => Dictionary::new(),
    };
    let mut out = Dictionary::new();
    for (key, value) in source.iter() {
        if matches!(
            key.as_bytes(),
            b"Parent" | b"Prev" | b"Next" | b"First" | b"Last" | b"Count"
        ) {
            continue;
        }
        let carried = piece.carry(value, 0);
        out.insert(key.clone(), carried);
    }
    // An item kept because a *descendant* reaches this piece may state a destination that does
    // not. Table 151 makes `/Dest` and `/A` alike optional and §12.3.3 describes activation as a
    // jump "to a destination or trigger an action" — so an item with neither is a heading, which
    // the clause admits. Removing the entry is a stronger answer than §7.3.10's null: Table 149
    // states no destination array whose page is null, and a named destination whose key this
    // piece dropped is not an indirect reference for §7.3.10 to be about.
    if item.states_destination && !item.lands_here {
        if source.get("Dest").is_some() {
            out.remove("Dest");
        } else {
            out.remove("A");
        }
    }
    // Table 151's `/SE` "shall be an indirect reference" to a structure element. Where §14.7's
    // carry kept no such element the reference became null, and an entry that shall be a
    // reference and is not is worse than an absent optional one.
    if out.get("SE").is_some_and(Object::is_null) {
        out.remove("SE");
    }
    // "The parent of a top-level item shall be the outline dictionary itself", which is what
    // `parent` is for the first level.
    out.insert(Name::new(&b"Parent"[..]), Object::Reference(parent));
    for (key, value) in [&b"Prev"[..], b"Next", b"First", b"Last"]
        .into_iter()
        .zip(chain)
    {
        if let Some(id) = value {
            out.insert(Name::new(key), Object::Reference(id));
        }
    }
    if !item.children.is_empty() {
        let visible = visible_descendants(item);
        out.insert(
            Name::new(&b"Count"[..]),
            Object::Integer(if item.open {
                visible
            } else {
                visible.saturating_neg()
            }),
        );
    }
    Object::Dictionary(out)
}
