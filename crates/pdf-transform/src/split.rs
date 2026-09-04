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
//! # What a piece does not carry, and why it says so
//!
//! The catalog is synthesised. It carries `/Pages`, and the entries whose absence would change
//! what the pages *look like*: `/Version`, `/Lang`, `/ViewerPreferences`, `/PageLayout`,
//! `/PageMode`, §8.11's `/OCProperties`, §12.7.3's `/AcroForm`, §14.11.5's
//! `/OutputIntents` and — since session 897, through [`crate::structure`] — §14.7's
//! `/StructTreeRoot` and `/MarkInfo`. Everything else the source catalog states — the outline,
//! the name trees, `/PageLabels`, `/Metadata` and the rest — is **not carried, and every one
//! of them is named in a warning**. That is RFC 0002 section 6.1's document-level carrying, and this verb does not do it yet:
//! the outline subset whose destinations survive, page labels recomputed per piece, and name-tree
//! entries still referenced are each a documented choice with edges of its own, and `doc/todo/57`
//! carries them. What is not acceptable is doing it silently, which is why the report names
//! each construct that was left behind rather than the verb claiming a fidelity it has not got.
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

use std::collections::{BTreeSet, VecDeque};
use std::io::Write as _;
use std::sync::Arc;

use pdf_model::Pages;
use pdf_model::page_label::PageLabels;
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
const NOT_CARRIED: [&str; 12] = [
    "Outlines",
    "Names",
    "Dests",
    "PageLabels",
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
    let cuts = cut(&groups, plan.pieces);
    if !plan.names.distinguishes(cuts.len()) {
        return Err(Refusal::Pattern(format!(
            "{} pieces would be written and the output name {:?} has no %d to tell them apart",
            cuts.len(),
            plan.names.to_string()
        )));
    }
    if plan.names.names_a_title() {
        return Err(Refusal::Pattern(
            "%t names a title, and a piece of a document has none until --at-bookmarks lands"
                .to_owned(),
        ));
    }

    let all_pages: BTreeSet<ObjectId> = pages.indices().into_keys().collect();
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
                version,
                form,
                sinks,
                count,
                ordinal,
                piece,
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
}

/// The selected pages cut into pieces.
fn cut(groups: &[Vec<usize>], pieces: Pieces) -> Vec<Vec<usize>> {
    match pieces {
        Pieces::Groups => groups.to_vec(),
        Pieces::EachPage => groups.concat().into_iter().map(|page| vec![page]).collect(),
        Pieces::Every(every) => groups
            .concat()
            .chunks(every.max(1))
            .map(<[usize]>::to_vec)
            .collect(),
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
        title: None,
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

    let root = build_catalog(&mut piece, job, tree, name, warnings);
    piece.drain();
    // The elements, the parent tree and the structure tree root, built last because the object
    // keys above are assigned by the walk that has just finished — and drained again, because an
    // element's attributes reach objects nothing else did.
    if let Some(carry) = piece.structure.take() {
        carry.finish(&mut piece, warnings);
        piece.drain();
    }

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
        .and_then(|()| piece.assembly.place(catalog, root))
        .map_err(|error| error.to_string())?;
    piece.assembly.set_root(catalog);

    // §14.3.3's information dictionary is one object with no page in it, so it crosses whole.
    // Nothing here writes a `/ModDate`: this crate has no clock, and RFC 0002 section 9 makes the
    // absence of one the determinism the gates rest on.
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
    Ok(piece.assembly)
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
) -> Object {
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
    Object::Dictionary(root)
}
