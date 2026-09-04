//! The document **edited in place**, by ISO 32000-2 §7.5.6's incremental update.
//!
//! # Why this is a different verb from `pages`, and not a flag on it
//!
//! `pages` and `merge` write a *new* file: every object renumbered, one page-tree node, the
//! document-level constructs reconciled. That is the right shape for a command line, whose
//! output is a path the caller named. It is the wrong shape for the one thing `CLAUDE.md`
//! permits to be done to the file a person already has open: its amended authoring exclusion
//! says that what a *user* does to a document already open — an annotation added, a field
//! filled — is not authoring, and that it is written back by §7.5.6's incremental update, the
//! new objects and a new cross-reference section appended, never a rewrite of what was there.
//!
//! RFC 0003's file-system faces write into the mounted document, so every one of their verbs is
//! that kind of writing and none of them may be a rewrite. §7.5.6 states the mechanism:
//!
//! > The contents of a PDF file can be updated incrementally without rewriting the entire file.
//! > When updating a PDF file incrementally, changes shall be appended to the end of the file,
//! > leaving its original contents intact.
//!
//! So this module edits **§7.7.3.2's page tree** and **§14.3.3's information dictionary** by
//! replacing the few objects that now read differently, and everything else in the file stays
//! where the producer put it, byte for byte. The output of every edit here is the whole file:
//! the source's own bytes and then the update.
//!
//! # The three edits, and what each rests on
//!
//! ## `DeletePage` — one leaf out of the tree
//!
//! §7.7.3.2's Table 30 is the whole definition of what has to change:
//!
//! > ( Required ) An array of indirect references to the immediate children of this node. The
//! > children shall only be page objects or other page tree nodes.
//!
//! > ( Required ) The number of leaf nodes (page objects) that are descendants of this node
//! > within the page tree.
//!
//! One `/Kids` entry goes, and every ancestor's `/Count` falls by one — that is all the tree
//! itself needs. The page object is then reached by nothing, and §7.5.6 says what an update
//! does with such an object:
//!
//! > Deleted objects shall be left unchanged in the PDF file, but shall be marked as deleted by
//! > means of their cross-reference entries.
//!
//! It is marked free, and everything *below* it — its content stream, its resources, its fonts —
//! is left in use, because nothing here can prove another page does not share it. **A deletion
//! therefore does not destroy bytes**, which RFC 0003 section 5.3 insists be said where a person
//! deletes: the page's content is still in the file under the append, unreferenced.
//!
//! Freeing the page object is also what makes every reference to it correct rather than
//! dangling. §7.3.10:
//!
//! > An indirect reference to an undefined object shall not be considered an error by a PDF
//! > processor; it shall be treated as a reference to the null object.
//!
//! and §12.3.2.2 makes a destination "an indirect reference to a page object", so an outline
//! item, an `/OpenAction` or a link pointing at the deleted page resolves to null — which is
//! exactly what `pages --delete` produces in its rewritten file, by the same clause.
//!
//! ## `InsertPages` — another document's pages carried in
//!
//! The incoming document's pages are copied into *this* document's numbering, appended as new
//! objects, and spliced into the `/Kids` array at the position the caller names. §7.7.3.3's
//! Table 31 is why each carried page's `/Parent` is rewritten and nothing else about it moves:
//!
//! > ( Required; shall be an indirect reference ) The page tree node that is the immediate
//! > parent of this page object
//!
//! and §7.7.3.4 is why the four inheritable attributes are flattened onto the carried page
//! rather than left to be inherited:
//!
//! > If such an attribute is omitted from a page object, its value shall be inherited from an
//! > ancestor node in the page tree.
//!
//! The ancestors it would inherit from are the *incoming* document's and are not coming with it,
//! so the value is written onto the page — the same flattening `merge::build_page` does, for the
//! same clause.
//!
//! **What an in-place insertion cannot carry, it refuses or names.** A rewrite reconciles the
//! document-level constructs because it is building a catalog; an update is not, so:
//!
//! - a page carrying a §12.7 widget annotation is **refused** by name, because §12.7.4.2 makes a
//!   field's fully qualified name its identity and a widget whose field did not come with it is
//!   a form this program would have invented;
//! - `/StructParents` is stripped from each carried page and the loss is warned, because
//!   §14.7.5.4's key is an index into *this* document's parent tree and a carried page's key
//!   would name somebody else's elements — a wrong answer given silently, which is worse than a
//!   page that is untagged;
//! - a `/StructTreeRoot`, an `/OCProperties`, an `/AcroForm`, an `/Outlines` or a `/Names` in
//!   the incoming document is warned about by name, because what those state about the carried
//!   pages is not coming with them (trap 5).
//!
//! §12.4.2's labels are positional:
//!
//! > the indices shall be fixed, running consecutively through the document starting from 0 for
//! > the first page, but the labels may be specified in any way that is appropriate for the
//! > particular document
//!
//! so any insertion or deletion moves every later index and no range of the source's tree
//! survives it. Where the document states one, it is rebuilt with one entry
//! per page of the new list, so a surviving page keeps the label it had; `merge` writes the same
//! shape for the same clause.
//!
//! ## `SetInformation` — §14.3.3's entries
//!
//! One object replaced, or one created and named by the update's own trailer (§7.5.5's Table 15
//! makes `/Info` a trailer entry). Table 349's nine keys and their types are what is accepted;
//! everything else the dictionary states is left exactly as the document states it, because this
//! edit is about the entries the table defines and a key it does not define is not this edit's
//! business.
//!
//! # Determinism
//!
//! The output is a function of the sources and the plan, RFC 0002 section 9's first layer:
//! nothing here reads a clock, and the object numbers a carry allocates are handed out in the
//! walk's own order.

use std::collections::{BTreeMap, BTreeSet};

use pdf_model::Pages;
use pdf_model::page_label::PageLabels;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::{Document, text_string};

use crate::merge::inherited;
use crate::pattern::{Fill, Pattern};
use crate::{Origin, Output, Refusal, Report, Sinks, Warning};

/// One document edited in place.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdatePlan {
    /// Which source is being edited.
    pub source: usize,
    /// What the edit is.
    pub edit: Edit,
    /// How the one output is named.
    pub names: Pattern,
}

/// One in-place edit.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// Every page of another source inserted before `at`, counted from 1; one past the end
    /// appends.
    InsertPages {
        /// Which source the pages come from.
        from: usize,
        /// Where they go, counted from 1.
        at: usize,
    },
    /// The page at this position, counted from 1, taken out of §7.7.3.2's tree.
    DeletePage {
        /// Which page.
        page: usize,
    },
    /// §14.3.3's entries set to what these state.
    SetInformation {
        /// One per Table 349 key the caller states; `None` removes the entry.
        entries: Vec<InfoEntry>,
    },
}

/// One entry of §14.3.3's document information dictionary, as a caller states it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoEntry {
    /// Table 349's key: `Title`, `Author`, `Subject`, `Keywords`, `Creator`, `Producer`,
    /// `CreationDate`, `ModDate` or `Trapped`.
    pub key: String,
    /// The value, or `None` for an entry the document shall no longer state.
    pub value: Option<String>,
}

/// Table 349's keys, in the table's own order.
///
/// The closed list this edit accepts: a key the table does not define is refused rather than
/// written, because §14.3.3 permits such a key and says nothing about what it would mean here,
/// and a file-system face that let a typo become an entry would be inventing metadata.
pub const INFORMATION_KEYS: [&str; 9] = [
    "Title",
    "Author",
    "Subject",
    "Keywords",
    "Creator",
    "Producer",
    "CreationDate",
    "ModDate",
    "Trapped",
];

/// Table 349's three names for `/Trapped`, which §14.3.3's row states are names and not
/// booleans: "This shall be the name True , not the boolean value true ."
const TRAPPED: [&str; 3] = ["True", "False", "Unknown"];

/// How far up §7.7.3.4's `/Parent` chain this walks.
///
/// `split.rs`'s and `merge.rs`'s bound and its reasoning: a page tree's height is small, and the
/// bound exists because `/Parent` is a reference a hostile file can make into a cycle.
const MAX_ANCESTRY: usize = 64;

/// How many page-tree nodes [`leaves_under`] visits before it stops counting.
///
/// `pdf_model`'s own `MAX_NODES_VISITED`, restated rather than shared because it is private
/// there — and restated *because* the two have to agree: this is what a `/Count` this writer
/// computes is compared against by the reader that will disbelieve the entry. A tree past it
/// yields an undercount, which is the safe direction: a `/Count` a reader disbelieves is
/// re-counted, and a `/Count` it believes over more pages than exist is not.
const MAX_NODES_VISITED: usize = 1 << 20;

/// How deep a carried value is rewritten before its tail is dropped.
///
/// The parser's own `max_depth` is 256, so nothing it admitted is deeper — `merge.rs`'s
/// constant and its reason.
const MAX_DEPTH: usize = 257;

/// The four entries Table 31 marks inheritable, which §7.7.3.3 makes a closed list.
const INHERITABLE: [&str; 4] = ["Resources", "MediaBox", "CropBox", "Rotate"];

/// Runs the verb.
///
/// `sources` is the plan's source index for each opened document, in the order [`crate::apply`]
/// opened them.
pub(crate) fn run(
    plan: &UpdatePlan,
    sources: &[usize],
    documents: &[Document],
    sinks: &dyn Sinks,
    report: &mut Report,
) -> Result<(), Refusal> {
    let at = plan.source;
    let document = opened(plan.source, sources, documents)?;
    let (bytes, pages, detail) = match &plan.edit {
        Edit::DeletePage { page } => delete_page(document, at, *page, report)?,
        Edit::InsertPages { from, at: position } => {
            let incoming = opened(*from, sources, documents)?;
            insert_pages(document, incoming, at, *from, *position, report)?
        }
        Edit::SetInformation { entries } => set_information(document, at, entries, report)?,
    };

    let expanded = plan.names.expand(&Fill {
        ordinal: 1,
        count: 1,
        page: None,
        label: None,
        title: None,
    });
    sinks
        .open(&expanded.name)
        .and_then(|mut sink| sink.write_all(&bytes).and_then(|()| sink.flush()))
        .map_err(|error| Refusal::Sink {
            name: expanded.name.clone(),
            error,
        })?;
    report.outputs.push(Output {
        name: expanded.name,
        bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        sanitised: expanded.sanitised,
        origin: Origin::Amended {
            source: at,
            pages,
            edit: detail,
        },
    });
    Ok(())
}

/// The opened document a plan's source index names.
fn opened<'a>(
    source: usize,
    sources: &[usize],
    documents: &'a [Document],
) -> Result<&'a Document, Refusal> {
    sources
        .iter()
        .position(|at| *at == source)
        .and_then(|position| documents.get(position))
        .ok_or(Refusal::NoSuchSource {
            at: source,
            count: documents.len(),
        })
}

/// §7.5.5's `/Root`, which every update needs an object number for.
fn root_of(document: &Document, at: usize) -> Result<(ObjectId, Dictionary), Refusal> {
    let catalog = document
        .catalog()
        .map_err(|error| Refusal::Unopenable { at, error })?;
    let Some(Object::Reference(root_id)) = document.trailer().get("Root").cloned() else {
        return Err(Refusal::Update {
            at,
            error: pdf_syntax::write::UpdateError::NoRoot,
        });
    };
    Ok((root_id, catalog))
}

/// Where the next object this update writes is numbered from.
fn next_object_number(document: &Document) -> u32 {
    let highest = document.xref().object_numbers().max().unwrap_or_default();
    let stated = document
        .trailer()
        .get("Size")
        .and_then(Object::as_integer)
        .and_then(|size| u32::try_from(size).ok())
        .unwrap_or_default();
    highest.saturating_add(1).max(stated)
}

/// One page's object number, refused where the document has no such page or the page is not an
/// indirect object.
fn page_id(document: &Document, at: usize, page: usize) -> Result<ObjectId, Refusal> {
    let pages = Pages::new(document);
    let count = pages.len();
    let found = page
        .checked_sub(1)
        .filter(|index| *index < count)
        .and_then(|index| pages.get(index))
        .ok_or(Refusal::NoSuchPage { at, page, count })?;
    // A page with no object number is one `Pages` recovered by scanning, and §7.5.6 has nothing
    // to chain an update to in such a file; `Recovered` is the true reason.
    found.id.ok_or(Refusal::Update {
        at,
        error: pdf_syntax::write::UpdateError::Recovered,
    })
}

/// A node's `/Kids`, resolved, as the references the array states.
fn kids_of(document: &Document, node: ObjectId) -> Vec<Object> {
    document
        .get_key_of(node, "Kids")
        .map(|kids| document.resolve(&kids))
        .and_then(|kids| kids.as_array().map(<[Object]>::to_vec))
        .unwrap_or_default()
}

/// The chain of §7.7.3.2 nodes from a page's own `/Parent` up to the root, nearest first.
///
/// Bounded and cycle-guarded, because `/Parent` is a reference a file states and a file can
/// state a cycle.
fn ancestry(document: &Document, page: ObjectId) -> Vec<ObjectId> {
    let mut chain = Vec::new();
    let mut seen: BTreeSet<ObjectId> = BTreeSet::from([page]);
    let mut here = page;
    for _ in 0..MAX_ANCESTRY {
        // The *unresolved* entry: `Document::get_key_of` resolves, and what is wanted here is
        // the object number Table 31 requires it to be — "( Required; shall be an indirect
        // reference ) The page tree node that is the immediate parent of this page object".
        let Some(parent) = document
            .get(here)
            .as_dict()
            .and_then(|dict| dict.get("Parent").cloned())
            .as_ref()
            .and_then(Object::as_reference)
        else {
            break;
        };
        if !seen.insert(parent) {
            break;
        }
        chain.push(parent);
        here = parent;
    }
    chain
}

/// A node's dictionary with `/Kids` and `/Count` as this edit leaves them.
fn node_with(document: &Document, node: ObjectId, kids: Vec<Object>, count: i64) -> Object {
    let mut dict = document
        .get(node)
        .as_dict()
        .cloned()
        .unwrap_or_else(Dictionary::new);
    dict.insert(Name::new(&b"Kids"[..]), Object::Array(kids));
    dict.insert(Name::new(&b"Count"[..]), Object::Integer(count));
    Object::Dictionary(dict)
}

/// §7.7.3.2's `/Count` as the document states it for a node, or the leaves under it counted.
///
/// **The second half of that sentence used to be a claim the body did not make**, and the
/// nine-hundred-and-ninth session's write-side corpus walk is what found it: the entry was read
/// and `unwrap_or_default()`ed, so a node that states no `/Count` counted as **zero** and an
/// insertion under it wrote `/Count 1` over a node that now held two pages. Table 30 makes the
/// entry required —
///
/// > ( Required ) The number of leaf nodes (page objects) that are descendants of this node
/// > within the page tree.
///
/// — so a node without one is malformed, and what a malformed node's descendants are is a
/// question the *tree* answers rather than the missing entry. `poppler-91414-0-53.pdf` and
/// `-54.pdf` are the witnesses: one node, no `/Count`, one kid, and an insertion that left a
/// two-page document reading as one page (trap 28 — the comment above a fallback is a claim
/// about the code, and this one had outlived it).
fn count_of(document: &Document, node: ObjectId) -> i64 {
    if let Some(stated) = document
        .get_key_of(node, "Count")
        .map(|count| document.resolve(&count))
        .and_then(|count| count.as_integer())
    {
        return stated;
    }
    let mut visited = 0_usize;
    leaves_under(document, node, &mut visited, 0)
}

/// The page objects below a node, counted by walking §7.7.3.2's `/Kids`.
///
/// **Read exactly as `pdf_model::count_leaves` reads it**, bounds included, and that agreement is
/// the point: the
/// number written into `/Count` here is the number the reader on the other side of the file will
/// count if it disbelieves the entry, so a second reading of the same tree would be a second
/// answer. A `/Kids` that is not an array is a leaf unless the node's own `/Type` says `Pages`,
/// because "trusting `/Type` in that direction would drop pages from files that leave it out"
/// and Table 30 makes a declared node's missing `/Kids` an absence of children rather than a
/// page.
///
/// Bounded the reader's own two ways — a depth and a node budget — rather than by a visited
/// *set*, and the difference matters twice. A set would make this disagree with the reader on a
/// malformed tree that reaches one node by two paths, which is the one thing this function may
/// not do; and it is an allocation a hostile file chooses the size of, which principle 3 does
/// not permit. `/Kids` holds references a file states and a file can state a cycle, so the depth
/// is what stops one.
fn leaves_under(document: &Document, node: ObjectId, visited: &mut usize, depth: usize) -> i64 {
    if depth > MAX_ANCESTRY || *visited > MAX_NODES_VISITED {
        return 0;
    }
    let Some(dict) = document.get(node).as_dict().cloned() else {
        return 0;
    };
    *visited = visited.saturating_add(1);
    let Some(kids) = document
        .get_key(&dict, "Kids")
        .as_array()
        .map(<[Object]>::to_vec)
    else {
        let declares_a_node = document
            .get_key(&dict, "Type")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Pages");
        return i64::from(!declares_a_node);
    };
    kids.iter()
        .filter_map(Object::as_reference)
        .map(|kid| leaves_under(document, kid, visited, depth.saturating_add(1)))
        .fold(0_i64, i64::saturating_add)
}

/// Refuses where the page an edit is about is not one the catalog's own tree reaches.
///
/// ISO 32000-2 §7.7.2's Table 29 makes the catalog's entry the tree a reader enters by:
///
/// > ( Required; shall be an indirect reference ) The page tree node that shall be the root of
/// > the document's page tree (see 7.7.3, "Page tree").
///
/// So a page whose `/Parent` chain does not pass through that object is a page the catalog does
/// not reach, and an update that edits its chain writes a perfectly correct `/Kids` into a tree
/// nobody enters. `pdf_model::Pages` reads such a file by scanning §7.7.3.2's own declarations
/// instead — a recovery that has no *positions* in it, only object numbers — so an insertion
/// "before page 1" lands after page 1, which is what `issue9418.pdf` did, and a splice into an
/// orphan node changes nothing at all, which is what `issue21436.pdf` did. Both were found by
/// the nine-hundred-and-ninth session's write-side corpus walk, and both are trap 5: an input
/// this verb cannot honestly serve is refused by name rather than served wrongly.
fn the_catalog_reaches(
    catalog: &Dictionary,
    chain: &[ObjectId],
    page: usize,
) -> Result<(), Refusal> {
    let Some(root) = catalog.get("Pages").and_then(Object::as_reference) else {
        return Err(Refusal::Assembly(format!(
            "this document's catalog states no /Pages, and §7.7.2 makes it \"( Required; shall \
             be an indirect reference ) The page tree node that shall be the root of the \
             document's page tree\"; page {page} was found by scanning the file's own /Type \
             /Page declarations, and a scan has no positions for an update to insert at"
        )));
    };
    if chain.contains(&root) {
        return Ok(());
    }
    Err(Refusal::Assembly(format!(
        "page {page} is not under object {}, which is the page tree root this document's catalog \
         states, so editing its /Parent chain would change a tree no reader enters",
        root.number
    )))
}

/// One page taken out of §7.7.3.2's tree, as §7.5.6's update.
fn delete_page(
    document: &Document,
    at: usize,
    page: usize,
    report: &mut Report,
) -> Result<(Vec<u8>, usize, String), Refusal> {
    let (root_id, catalog) = root_of(document, at)?;
    let count = Pages::new(document).len();
    if count <= 1 {
        return Err(Refusal::Assembly(
            "this document has one page, and §7.7.3.2 makes /Kids \"an array of indirect \
             references to the immediate children\" of a node that has some"
                .to_owned(),
        ));
    }
    let victim = page_id(document, at, page)?;
    let chain = ancestry(document, victim);
    if chain.is_empty() {
        return Err(Refusal::Assembly(format!(
            "page {page} states no /Parent, and Table 31 makes it \"( Required; shall be an \
             indirect reference )\", so this update cannot say which node to take it out of"
        )));
    }
    the_catalog_reaches(&catalog, &chain, page)?;

    let tree_root = catalog.get("Pages").and_then(Object::as_reference);

    let mut next = next_object_number(document);
    let mut fresh = move || {
        let id = ObjectId {
            number: next,
            generation: 0,
        };
        next = next.saturating_add(1);
        id
    };
    let mut replacements: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut freed: Vec<ObjectId> = vec![victim];

    // The nearest node loses the entry; every node above it loses one leaf. A node emptied by
    // the removal leaves *its* own parent too, because §7.7.3.2's `/Kids` is "an array of
    // indirect references to the immediate children of this node" and a node with no children
    // is not one of them; the root is the one node that may be left holding nothing, and this
    // verb refuses a document down to its last page before it can be.
    let mut removing = Some(Object::Reference(victim));
    for (height, node) in chain.iter().enumerate() {
        let mut kids = kids_of(document, *node);
        if let Some(reference) = removing.as_ref() {
            if let Some(position) = kids.iter().position(|kid| kid == reference) {
                kids.remove(position);
            } else if height == 0 {
                return Err(Refusal::Assembly(format!(
                    "page {page} names object {} as its /Parent, and that node's /Kids does \
                     not hold it",
                    node.number
                )));
            }
        }
        let leaves = count_of(document, *node).saturating_sub(1);
        if kids.is_empty() && Some(*node) != tree_root {
            freed.push(*node);
            removing = Some(Object::Reference(*node));
            continue;
        }
        replacements.insert(*node, node_with(document, *node, kids, leaves));
        removing = None;
    }
    if replacements.is_empty() {
        return Err(Refusal::Assembly(format!(
            "taking page {page} out would leave §7.7.3.2's tree with no node holding a page"
        )));
    }

    let after = count.saturating_sub(1);
    let kept: Vec<Placed> = (0..count)
        .filter(|index| *index != page.saturating_sub(1))
        .map(Placed::Own)
        .collect();
    rebuild_labels(
        document,
        None,
        &catalog,
        root_id,
        &kept,
        &mut fresh,
        &mut replacements,
    );
    report.warnings.push(Warning {
        source: at,
        page: Some(page),
        detail: String::from(
            "§7.5.6 leaves a deleted object's bytes in the file — \"[d]eleted objects shall be \
             left unchanged in the PDF file, but shall be marked as deleted by means of their \
             cross-reference entries\" — so this page's content is still in the document, \
             unreferenced, and removing it needs a rewrite rather than an update",
        ),
    });
    let bytes = pdf_syntax::write::incremental_update_freeing(document, &replacements, &freed)
        .map_err(|error| Refusal::Update { at, error })?;
    Ok((bytes, after, format!("page {page} deleted")))
}

/// Where one page of the edited list gets its §12.4.2 label from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Placed {
    /// A page this document already held, by its old zero-based index.
    Own(usize),
    /// A page carried in, by its zero-based index in the incoming document.
    Carried(usize),
}

/// §12.4.2's number tree rebuilt with one entry per page of the edited list.
///
/// > A document's labelling ranges shall be defined by the PageLabels entry in the document
/// > catalog dictionary
///
/// and the indices that tree is keyed by "shall be fixed, running consecutively through the
/// document starting from 0 for the first page", so a page taken out or put in moves every later
/// index and no range of the document's own tree survives the edit. One entry per page, each stating
/// the label the page already had as Table 161's `/P` prefix with no numeric portion —
///
/// > There is no default numbering style; if no S entry is present, page labels shall consist
/// > solely of a label prefix with no numeric portion.
///
/// — which is `merge`'s construction for the same clause, and the only one that keeps every
/// surviving page called what it was called.
///
/// A document that states no `/PageLabels` gets none: it labelled nothing before the edit and
/// labels nothing after it.
fn rebuild_labels(
    document: &Document,
    incoming: Option<&Document>,
    catalog: &Dictionary,
    root_id: ObjectId,
    order: &[Placed],
    fresh: &mut impl FnMut() -> ObjectId,
    replacements: &mut BTreeMap<ObjectId, Object>,
) {
    let Some(stated) = catalog.get("PageLabels") else {
        return;
    };
    let own = PageLabels::read(document);
    let theirs = incoming.map(PageLabels::read);
    let mut nums = Vec::with_capacity(order.len().saturating_mul(2));
    for (position, place) in order.iter().enumerate() {
        let label = match place {
            Placed::Own(index) => own.label(*index),
            Placed::Carried(index) => theirs.as_ref().and_then(|labels| labels.label(*index)),
        };
        let mut entry = Dictionary::new();
        if let Some(label) = label {
            entry.insert(Name::new(&b"P"[..]), text(&label));
        } else {
            // A page out of a document that labelled it nothing. The clause says what such a
            // page's *index* is and nothing about what it is called, so this writes the decimal
            // number of the position it now holds — a documented choice, and the one that keeps
            // it out of a neighbouring range's numbering.
            entry.insert(Name::new(&b"S"[..]), Object::Name(Name::new(&b"D"[..])));
            entry.insert(
                Name::new(&b"St"[..]),
                Object::Integer(i64::try_from(position.saturating_add(1)).unwrap_or(1)),
            );
        }
        nums.push(Object::Integer(i64::try_from(position).unwrap_or(i64::MAX)));
        nums.push(Object::Dictionary(entry));
    }
    let mut tree = Dictionary::new();
    tree.insert(Name::new(&b"Nums"[..]), Object::Array(nums));
    // Where the tree is its own object, that object now reads differently and nothing else
    // moves. Where the catalog holds it inline, the *catalog* is what now reads differently, and
    // the tree is given a number of its own so that the next edit finds a reference here rather
    // than repeating this.
    if let Some(id) = stated.as_reference() {
        replacements.insert(id, Object::Dictionary(tree));
    } else {
        let id = fresh();
        replacements.insert(id, Object::Dictionary(tree));
        let mut root = catalog.clone();
        root.insert(Name::new(&b"PageLabels"[..]), Object::Reference(id));
        replacements.insert(root_id, Object::Dictionary(root));
    }
}

/// §7.9.2.2's text string, in the encoding that carries the bytes this label holds.
fn text(value: &str) -> Object {
    Object::String(text_string::encode_text_string(value).into())
}

/// Another document's pages carried into this one, as §7.5.6's update.
fn insert_pages(
    document: &Document,
    incoming: &Document,
    at: usize,
    from: usize,
    position: usize,
    report: &mut Report,
) -> Result<(Vec<u8>, usize, String), Refusal> {
    let (root_id, catalog) = root_of(document, at)?;
    let here = Pages::new(document);
    let there = Pages::new(incoming);
    let count = here.len();
    let carried_count = there.len();
    if carried_count == 0 {
        return Err(Refusal::Assembly(format!(
            "source {from} has no pages to insert"
        )));
    }
    if position == 0 || position > count.saturating_add(1) {
        return Err(Refusal::Position {
            at,
            position,
            count,
        });
    }

    // Which existing page the carried block sits beside, and on which side. Its `/Parent` is the
    // node the carried pages join, which is correct for any tree shape: Table 31 requires one
    // `/Parent` per page, and a page's siblings are exactly the node's `/Kids`.
    let (neighbour, before) = if position <= count {
        (position, true)
    } else {
        (count, false)
    };
    let anchor = page_id(document, at, neighbour)?;
    let chain = ancestry(document, anchor);
    let Some(parent) = chain.first().copied() else {
        return Err(Refusal::Assembly(format!(
            "page {neighbour} states no /Parent, and Table 31 makes it \"( Required; shall be an \
             indirect reference )\", so this update cannot say which node to put the pages in"
        )));
    };
    the_catalog_reaches(&catalog, &chain, neighbour)?;

    let ids = carried_ids(incoming, &there, from, carried_count)?;

    let mut next = next_object_number(document);
    let mut carry = Carry::new(incoming, next);
    // Every carried page is numbered before any of them is walked, so that a reference from one
    // page's annotation to another page of the incoming block maps to the copy rather than to
    // §7.3.10's null. `merge::reserve_pages` takes the numbering first for the same reason.
    let placed: Vec<ObjectId> = ids.iter().map(|id| carry.reserve(*id)).collect();
    let mut replacements: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut stripped = Vec::new();
    for (index, (source, target)) in ids.iter().zip(&placed).enumerate() {
        let page = build_page(incoming, &mut carry, *source, parent)?;
        if page.1 {
            stripped.push(index.saturating_add(1));
        }
        replacements.insert(*target, page.0);
    }
    next = carry.finish(&mut replacements);
    let mut fresh = move || {
        let id = ObjectId {
            number: next,
            generation: 0,
        };
        next = next.saturating_add(1);
        id
    };

    splice_into_tree(
        document,
        &chain,
        Splice {
            anchor,
            before,
            neighbour,
            placed: &placed,
        },
        &mut replacements,
    )?;

    let mut order: Vec<Placed> = (0..count).map(Placed::Own).collect();
    let block: Vec<Placed> = (0..carried_count).map(Placed::Carried).collect();
    let cut = position.saturating_sub(1).min(order.len());
    order.splice(cut..cut, block);
    rebuild_labels(
        document,
        Some(incoming),
        &catalog,
        root_id,
        &order,
        &mut fresh,
        &mut replacements,
    );

    report_losses(incoming, from, &stripped, report);
    // A document whose cross-reference table this reader rebuilt by scanning is a document
    // somebody's copy may have been cut short in the middle of — and it is *also* what a
    // genuinely damaged file the caller meant to insert looks like, which is why this is a
    // sentence rather than a refusal (trap 5: unsupported input stays loud, and a reader that
    // declined a file it can read would be deciding for the person holding it).
    if incoming.was_recovered() {
        report.warnings.push(Warning {
            source: from,
            page: None,
            detail: format!(
                "source {from}: §7.5.5's cross-reference table could not be read and was rebuilt \
                 by scanning, so the {carried_count} page(s) carried in are the pages that \
                 scan found; a copy that was cut short looks exactly like this"
            ),
        });
    }

    let bytes = pdf_syntax::write::incremental_update(document, &replacements)
        .map_err(|error| Refusal::Update { at, error })?;
    Ok((
        bytes,
        count.saturating_add(carried_count),
        format!("{carried_count} page(s) of source {from} inserted at position {position}"),
    ))
}

/// The catalog entries an in-place insertion leaves behind, each named where a source has one.
///
/// A rewrite reconciles these because it is building a catalog (`merge`'s construction, session
/// 888's clause-by-clause derivation). An update is not building one, so what the incoming
/// document says about its pages at the document level does not come with them — and trap 5's
/// rule is that this is said out loud rather than discovered.
const NOT_CARRIED: [(&str, &str); 5] = [
    ("AcroForm", "§12.7's interactive form"),
    ("OCProperties", "§8.11's optional content configuration"),
    ("Outlines", "§12.3.3's outline"),
    (
        "Names",
        "§7.9.6's name trees, including its named destinations",
    ),
    ("StructTreeRoot", "§14.7's structure tree"),
];

/// Refuses a page whose `/Annots` holds a §12.7 widget.
///
/// §12.7.4.2 makes a field's fully qualified name its identity, and a widget is a field's
/// representation on a page. Carrying the page without its field would leave an annotation
/// naming a field this document does not have; carrying the field would be writing into
/// §12.7.2's form dictionary, which an in-place insertion does not touch. Both are a form edited
/// rather than a page inserted, so the operation is declined by name — `pages --insert` declines
/// the same construct for the same clause.
fn refuse_widget(document: &Document, at: usize, page: usize, id: ObjectId) -> Result<(), Refusal> {
    let annots = document
        .get_key_of(id, "Annots")
        .map(|annots| document.resolve(&annots));
    let Some(array) = annots.as_ref().and_then(Object::as_array) else {
        return Ok(());
    };
    for entry in array {
        let annotation = document.resolve(entry);
        let Some(dict) = annotation.as_dict() else {
            continue;
        };
        if document
            .get_key(dict, "Subtype")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Widget")
        {
            return Err(Refusal::DuplicateWidget { at, page });
        }
    }
    Ok(())
}

/// One carried page: the source's dictionary, `/Parent` renamed, §7.7.3.4's inheritance
/// flattened onto it, §14.7.5.4's key stripped, and every reference in it carried.
///
/// The second half of the answer says whether a `/StructParents` was stripped, so that the loss
/// is counted rather than assumed.
fn build_page(
    incoming: &Document,
    carry: &mut Carry<'_>,
    source: ObjectId,
    parent: ObjectId,
) -> Result<(Object, bool), Refusal> {
    let object = incoming.get(source);
    let Some(dict) = object.as_dict() else {
        return Err(Refusal::Assembly(format!(
            "the page at object {} is not a dictionary",
            source.number
        )));
    };
    let mut out = Dictionary::new();
    let mut stripped = false;
    for (key, value) in dict.iter() {
        match key.as_bytes() {
            // Table 31's `/Parent` is this document's to state.
            b"Parent" => {}
            // §14.7.5.4's key indexes the *holder's* parent tree, and the incoming tree is not
            // carried, so a key left here would name elements of this document that belong to
            // another page.
            b"StructParents" => stripped = true,
            _ => {
                out.insert(key.clone(), carry.value(value, 0)?);
            }
        }
    }
    // §7.7.3.4's inheritance, flattened: the ancestors these would be inherited from are the
    // incoming document's page tree, which is not coming with the page.
    for key in INHERITABLE {
        if out.get(key).is_some() {
            continue;
        }
        if let Some(value) = inherited(incoming, source, key) {
            out.insert(Name::new(key.as_bytes()), carry.value(&value, 0)?);
        }
    }
    out.insert(Name::new(&b"Parent"[..]), Object::Reference(parent));
    Ok((Object::Dictionary(out), stripped))
}

/// The walk that copies one document's objects into another's numbering.
#[derive(Debug)]
struct Carry<'a> {
    /// Where the objects come from.
    incoming: &'a Document,
    /// The source object number each carried object took here.
    map: BTreeMap<ObjectId, ObjectId>,
    /// What has been carried, by its new number.
    out: BTreeMap<ObjectId, Object>,
    /// The next free number in the *target*.
    next: u32,
}

impl<'a> Carry<'a> {
    /// A walk numbering from `next`.
    fn new(incoming: &'a Document, next: u32) -> Self {
        Self {
            incoming,
            map: BTreeMap::new(),
            out: BTreeMap::new(),
            next,
        }
    }

    /// A number for an object the caller will build itself.
    fn reserve(&mut self, source: ObjectId) -> ObjectId {
        let id = self.allocate();
        self.map.insert(source, id);
        id
    }

    /// The next number, generation 0 — a number nothing in the target has used.
    fn allocate(&mut self) -> ObjectId {
        let id = ObjectId {
            number: self.next,
            generation: 0,
        };
        self.next = self.next.saturating_add(1);
        id
    }

    /// One value, with every reference in it carried.
    fn value(&mut self, value: &Object, depth: usize) -> Result<Object, Refusal> {
        if depth >= MAX_DEPTH {
            // The parser admitted nothing deeper, so this is a value no document can state; a
            // null rather than a refusal, because §7.3.10 makes it a legible answer.
            return Ok(Object::Null);
        }
        Ok(match value {
            Object::Reference(source) => {
                if let Some(known) = self.map.get(source) {
                    return Ok(Object::Reference(*known));
                }
                let id = self.reserve(*source);
                let resolved = self.incoming.get(*source);
                let carried = self.value(&resolved, depth.saturating_add(1))?;
                self.out.insert(id, carried);
                Object::Reference(id)
            }
            Object::Array(items) => Object::Array(
                items
                    .iter()
                    .map(|item| self.value(item, depth.saturating_add(1)))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Object::Dictionary(dict) => Object::Dictionary(self.dictionary(dict, depth)?),
            Object::Stream(stream) => {
                // The stream's own bytes, as the file holds them — still under §7.4's filters,
                // and already out of §7.6's encryption, because that is the form this reader
                // holds and `incremental_update` is what puts the target's own encryption back
                // on the way out.
                Object::Stream(std::sync::Arc::new(Stream {
                    dict: self.dictionary(&stream.dict, depth)?,
                    data: stream.data.clone(),
                    decryption_failed: stream.decryption_failed,
                }))
            }
            other => other.clone(),
        })
    }

    /// One dictionary's entries, carried.
    fn dictionary(&mut self, dict: &Dictionary, depth: usize) -> Result<Dictionary, Refusal> {
        let mut out = Dictionary::new();
        for (key, value) in dict.iter() {
            out.insert(key.clone(), self.value(value, depth.saturating_add(1))?);
        }
        Ok(out)
    }

    /// Hands everything carried to the update, and answers the next free number.
    fn finish(self, replacements: &mut BTreeMap<ObjectId, Object>) -> u32 {
        for (id, value) in self.out {
            replacements.insert(id, value);
        }
        self.next
    }
}

/// §14.3.3's entries set to what the caller states, as §7.5.6's update.
fn set_information(
    document: &Document,
    at: usize,
    entries: &[InfoEntry],
    report: &mut Report,
) -> Result<(Vec<u8>, usize, String), Refusal> {
    for entry in entries {
        validate(entry)?;
    }
    let stated = document.trailer().get("Info").cloned();
    let mut dict = stated
        .as_ref()
        .map(|value| document.resolve(value))
        .and_then(|value| value.as_dict().cloned())
        .unwrap_or_else(Dictionary::new);
    let mut changed_a_date = false;
    for entry in entries {
        let key = Name::new(entry.key.as_bytes());
        match &entry.value {
            // Table 349's `/Trapped` is "a name object", and the table says so twice over: "This
            // shall be the name True , not the boolean value true ." Everything else in the
            // table is a text string, which §14.3.3 states as a `shall` for every key but the
            // two dates and which §7.9.4 states for those two as well ("A date shall be a text
            // string value").
            Some(value) if entry.key == "Trapped" => {
                dict.insert(key, Object::Name(Name::new(value.as_bytes())));
            }
            Some(value) => {
                dict.insert(key, text(value));
            }
            None => {
                dict.remove(entry.key.as_str());
            }
        }
        changed_a_date |= entry.key == "CreationDate" || entry.key == "ModDate";
    }

    let mut replacements: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut additions = Dictionary::new();
    // §7.5.5's Table 15 makes `/Info` "( Optional; shall be an indirect reference )", so a
    // document that states one inline has broken that `shall` and one that states none has
    // nowhere to put one: both get an object of their own, named by the update's own trailer.
    if let Some(id) = stated.as_ref().and_then(Object::as_reference) {
        replacements.insert(id, Object::Dictionary(dict));
    } else {
        let id = ObjectId {
            number: next_object_number(document),
            generation: 0,
        };
        replacements.insert(id, Object::Dictionary(dict));
        additions.insert(Name::new(&b"Info"[..]), Object::Reference(id));
        if stated.is_some() {
            report.warnings.push(Warning {
                source: at,
                page: None,
                detail: String::from(
                    "this document states /Info inline, and §7.5.5's Table 15 makes it \
                     \"( Optional; shall be an indirect reference )\"; the update writes it as \
                     an indirect object",
                ),
            });
        }
    }

    // §14.3.4 is about exactly this edit, and its rule is conditioned on writing both sources:
    // "[w]hen writing the time and date of the most recent modification … a PDF processor shall
    // ensure that the data in the document information dictionary and the document level
    // metadata stream — if both are written — are fully equivalent." This update writes one of
    // the two, because §14.3.2's stream is a derived file this face refuses to write; so the
    // inconsistency the clause is about is possible and is named rather than left to be found.
    if changed_a_date && has_metadata_stream(document) {
        report.warnings.push(Warning {
            source: at,
            page: None,
            detail: String::from(
                "this document also states §14.3.2's metadata stream, and this update writes a \
                 date into §14.3.3's dictionary alone; §14.3.4 says the two should be fully \
                 equivalent where both are written, and this writes one",
            ),
        });
    }

    let bytes =
        pdf_syntax::write::incremental_update_extending(document, &replacements, &[], &additions)
            .map_err(|error| Refusal::Update { at, error })?;
    let pages = Pages::new(document).len();
    Ok((
        bytes,
        pages,
        format!("{} §14.3.3 entr(y|ies) set", entries.len()),
    ))
}

/// Whether the catalog states §14.3.2's metadata stream.
fn has_metadata_stream(document: &Document) -> bool {
    document
        .catalog()
        .ok()
        .map(|catalog| document.get_key(&catalog, "Metadata"))
        .is_some_and(|object| object.as_stream().is_some())
}

/// One entry held to Table 349, refused by name where it is not.
fn validate(entry: &InfoEntry) -> Result<(), Refusal> {
    if !INFORMATION_KEYS.contains(&entry.key.as_str()) {
        return Err(Refusal::Pattern(format!(
            "{:?} is not one of §14.3.3's Table 349 keys ({})",
            entry.key,
            INFORMATION_KEYS.join(", ")
        )));
    }
    let Some(value) = entry.value.as_deref() else {
        return Ok(());
    };
    if entry.key == "Trapped" && !TRAPPED.contains(&value) {
        return Err(Refusal::Pattern(format!(
            "Table 349 makes /Trapped \"a name object\" that is True, False or Unknown, and \
             {value:?} is none of them"
        )));
    }
    if (entry.key == "CreationDate" || entry.key == "ModDate") && !is_a_date(value) {
        return Err(Refusal::Pattern(format!(
            "§7.9.4 makes a date \"a text string value containing no white-space, of the form\" \
             D:YYYYMMDDHHmmSSOHH'mm, whose \"prefix ' D: ' shall be present\" and whose year \
             \"shall be present\"; {value:?} is not one"
        )));
    }
    Ok(())
}

/// Whether a string is §7.9.4's date.
///
/// > The prefix ' D: ' shall be present, the year field (YYYY) shall be present and all other
/// > fields may be present but only if all of their preceding fields are also present.
///
/// So the check is the prefix, four digits, and then only the alphabet the clause's form is
/// written in — the field-by-field ranges are the producer's business and a reader that
/// re-derived them would be rejecting dates the clause admits. A date this program refuses is a
/// date it would otherwise write into somebody's file (trap 5).
fn is_a_date(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("D:") else {
        return false;
    };
    if rest.len() < 4 || !rest.as_bytes()[..4].iter().all(u8::is_ascii_digit) {
        return false;
    }
    rest.bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'Z' | b'\''))
}

/// Where a carried block joins §7.7.3.2's tree.
#[derive(Debug, Clone, Copy)]
struct Splice<'a> {
    /// The existing page it sits beside.
    anchor: ObjectId,
    /// Whether it goes before that page or after it.
    before: bool,
    /// Which page that is, counted from 1, for a message.
    neighbour: usize,
    /// The carried pages, in order, by the numbers they took here.
    placed: &'a [ObjectId],
}

/// The carried block put into the node the anchor belongs to, and Table 30's `/Count` raised on
/// that node and every node above it.
fn splice_into_tree(
    document: &Document,
    chain: &[ObjectId],
    splice: Splice<'_>,
    replacements: &mut BTreeMap<ObjectId, Object>,
) -> Result<(), Refusal> {
    let Some(parent) = chain.first().copied() else {
        return Ok(());
    };
    let mut kids = kids_of(document, parent);
    let slot = kids
        .iter()
        .position(|kid| *kid == Object::Reference(splice.anchor))
        .ok_or_else(|| {
            Refusal::Assembly(format!(
                "page {} names object {} as its /Parent, and that node's /Kids does not hold it",
                splice.neighbour, parent.number
            ))
        })?;
    let at = if splice.before {
        slot
    } else {
        slot.saturating_add(1)
    };
    let added: Vec<Object> = splice
        .placed
        .iter()
        .map(|id| Object::Reference(*id))
        .collect();
    kids.splice(at..at, added);
    let leaves = i64::try_from(splice.placed.len()).unwrap_or(i64::MAX);
    replacements.insert(
        parent,
        node_with(
            document,
            parent,
            kids,
            count_of(document, parent).saturating_add(leaves),
        ),
    );
    for above in chain.iter().skip(1) {
        let kids = kids_of(document, *above);
        replacements.insert(
            *above,
            node_with(
                document,
                *above,
                kids,
                count_of(document, *above).saturating_add(leaves),
            ),
        );
    }
    Ok(())
}

/// What the carried pages arrive without, each named (trap 5).
fn report_losses(incoming: &Document, from: usize, stripped: &[usize], report: &mut Report) {
    if !stripped.is_empty() {
        report.warnings.push(Warning {
            source: from,
            page: stripped.first().copied(),
            detail: format!(
                "§14.7.5.4's /StructParents was removed from {} carried page(s), because the key \
                 is an index into this document's own parent tree and the incoming document's \
                 structure tree is not carried by an in-place insertion; the pages arrive \
                 untagged",
                stripped.len()
            ),
        });
    }
    for (key, what) in NOT_CARRIED {
        if incoming.catalog().is_ok_and(|root| root.get(key).is_some()) {
            report.warnings.push(Warning {
                source: from,
                page: None,
                detail: format!(
                    "source {from} states /{key}, and {what} is not carried by an in-place \
                     insertion: the pages arrive without it"
                ),
            });
        }
    }
}

/// Every incoming page's object number, with §12.7's widgets refused on the way.
fn carried_ids(
    incoming: &Document,
    there: &Pages<'_>,
    from: usize,
    count: usize,
) -> Result<Vec<ObjectId>, Refusal> {
    let mut ids = Vec::with_capacity(count);
    for index in 0..count {
        let page = there.get(index).ok_or(Refusal::NoSuchPage {
            at: from,
            page: index.saturating_add(1),
            count,
        })?;
        // A page with no object number is one `Pages` recovered by scanning; §7.5.6 has nothing
        // to chain to in such a file, and `Recovered` is the true reason.
        let id = page.id.ok_or(Refusal::Update {
            at: from,
            error: pdf_syntax::write::UpdateError::Recovered,
        })?;
        refuse_widget(incoming, from, index.saturating_add(1), id)?;
        ids.push(id);
    }
    Ok(ids)
}
