//! ISO 32000-2 §14.7's logical structure and §14.9's accessibility entries, as a tree a host can
//! hand to a platform accessibility API.
//!
//! # Why this is a `Query` and not an `Event`
//!
//! A screen reader asks for the tree when it attaches, and again when the page changes. It is a
//! question with an answer the viewer already holds, which is exactly what the second channel is
//! for — the same argument [`crate::Query::Selection`] makes.
//!
//! # What this is, and who reads it
//!
//! `pdf-model` has read §14.7's structure tree since the seventy-eighth session and §14.9's
//! `/Alt`, `/E`, `/Lang` and `/ActualText` since the sixtieth, and this file was the first
//! consumer of either. What crosses is toolkit-free by construction — a role name, a string to
//! speak, a language tag and the quadrilaterals the element covers, in the same device pixels
//! [`crate::Query::Selection`] answers in — so a host builds `AccessKit`'s nodes, AT-SPI's, or
//! `NSAccessibility`'s from it without this crate naming any of them.
//!
//! **`viewer-accessibility` is that host since the three-hundred-and-seventy-sixth session**, and
//! the sentence this comment used to carry — "until now nothing in this program handed a
//! structure tree to anybody" — is what it retired. Two things this answer owes it are stated
//! below and were both wrong until that round: the role is mapped through §14.7.3's `/RoleMap`,
//! and an element's name is its own text rather than its subtree's. ADR 0214.
//!
//! # The order the nodes are in
//!
//! §14.8.2.5's *logical* order, which is the structure tree's own order and not the content
//! stream's. That is the whole reason a tagged document is worth reading: a page whose producer
//! wrote its columns out of order gives its text in that order to a selection and in the right
//! order here.
//!
//! # What is on this page, and what is merely in the file
//!
//! A structure tree spans the whole document; this answers for one page. An element is kept when
//! it, or something below it, names a content item on the page being asked about — Table 355's
//! `/Pg` and Table 358's, through §14.7.5.2's marked-content sequences and §14.7.5.3's object
//! references. Everything else belongs to another page and is not answered with, which is what
//! ADR 0134 said this did and what the three-hundred-and-seventy-sixth session found it did not:
//! a thousand-page document handed a screen reader every element in the file, with text and
//! quadrilaterals on none but one page's.
//!
//! # How the page's elements are *found*, which is not the same question
//!
//! Keeping the right elements is one thing; reaching them is another, and this walked the whole
//! document's tree to do it. [`MAX_NODES`] then stopped the walk after the first few pages' worth
//! of elements, so every page past those answered with **nothing** — ISO 32000-2's page 400 and
//! two thirds of its 1023 pages, and nothing said so. A screen reader heard silence and could not
//! tell it from an untagged page.
//!
//! §14.7.5.4 states the route that does not have this shape: the structural parent tree, keyed by
//! the page's own `/StructParents`, names the elements this page's content items belong to
//! directly. [`pdf_model::structure::Tree::elements_on_page`] asks it and
//! [`pdf_model::structure::Tree::ancestry`] follows Table 355's `/P` up from each answer, so the
//! walk below descends from the root **only into the subtree the page occupies** — the ancestors
//! it must pass through and nothing beside them. What was a walk of the document is a walk of the
//! page. ADR 0325.
//!
//! # What an object reference is worth, which is two answers rather than none
//!
//! §14.7.5.3's `/OBJR` makes an element's content "an entire PDF object", and for a long time this
//! walk took one fact from it: that the element is on this page. Everything else about such an
//! element was empty — no shapes, because it marked no text, and a generic group on the far side
//! whatever the object was.
//!
//! Two clauses answer for the annotation half of that sentence, and both do it in the space Table
//! 379's rectangle is already mapped from. §12.5.2 states where the annotation is; §12.7 states
//! what control a widget annotation belongs to. So an element whose content *is* an annotation
//! gets [`AccessibilityNode::bounds`], and one whose content is a widget gets
//! [`AccessibilityNode::control`] — which is what turns §14.8.4.7.2's `Form` from a group into a
//! check box. ADR 0338.

use std::collections::{BTreeMap, BTreeSet};

use pdf_model::accessibility::Described;
use pdf_model::content::MarkedSpan;
use pdf_model::structure::{Child, HeaderScope, StandardType, TableStack, Tree};
use pdf_syntax::{Dictionary, Document, ObjectId};

/// How deep a structure tree is walked before the walk is abandoned.
///
/// A tree deeper than this is hostile rather than merely complex — §14.7.2 puts no bound on
/// nesting and a file can state a cycle through `/K` that no `/Type` distinguishes from a deep
/// tree. The same reasoning and the same order of magnitude as `pdf-model`'s own bounds.
const MAX_DEPTH: usize = 64;

/// How many nodes one page's tree may hold.
///
/// A bound on the *answer* rather than on the document: a host asks this question when a screen
/// reader attaches, and a page that produced a million nodes would stall it.
const MAX_NODES: usize = 8192;

/// One element of §14.7's structure tree, as an accessibility API would take it.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityNode {
    /// Which node encloses this one, as an index into the answer, or `None` for a root.
    ///
    /// An index rather than a nesting of children, because that is the shape both `AccessKit` and
    /// AT-SPI want and because a flat list has no recursion for a host to bound.
    pub parent: Option<usize>,
    /// §14.7.4's `/S`, **after §14.7.3's and §14.8.6.2's role mapping**.
    ///
    /// ISO 32000-2 §14.7.3:
    ///
    /// > A structure type shall always be mapped to its corresponding name in the role map, if
    /// > there is one, even if the original name is one of the standard types.
    ///
    /// so a document's own `Chap` crosses as the `Sect` its own `/RoleMap` says it is.
    /// [`pdf_model::structure::Tree::role`] is where that is done, transitively and bounded.
    ///
    /// **This used to be the raw `/S`**, on the argument that "a host that knows the platform's
    /// own vocabulary is better placed to map `H1` or `TD` onto it than this crate is" — which is
    /// true and is about a *different* mapping. §14.7.3's role map is the file's own statement
    /// about its own names and is a `shall`; §14.8.4's standard set onto a platform's roles is the
    /// host's. Two mappings wearing one coat, and only the second was ever the host's (ADR 0214).
    ///
    /// Still a name rather than [`pdf_model::structure::StandardType`]: §14.8.4.1 requires a
    /// tagged document's types to be standard or mapped to standard ones, so a name that is
    /// neither is a fact about the document a host may want to say, and
    /// [`pdf_model::structure::StandardType::read`] is one call away for the ones that are.
    pub role: String,
    /// What a text-to-speech engine would say for **this element and no other**.
    ///
    /// §14.9.3's `/Alt` where the element states one — "human-readable text that could, for
    /// example, be vocalised by a text-to-speech engine" — else §14.9.5's `/E`, else the text the
    /// element's *own* marked-content sequences produced. The precedence is
    /// [`pdf_model::accessibility::Described`]'s, stated once there and followed here.
    ///
    /// **The element's own text, not its subtree's**, which is the shape every platform
    /// accessibility tree takes: text belongs to the node that carries it, and a container whose
    /// name repeated its children's would be read twice. Where the element states a substitution
    /// there is nothing to repeat, and [`Self::substituted`] says which of the two this is.
    pub name: String,
    /// Whether [`Self::name`] **replaces** what is below this element, or merely names it.
    ///
    /// ISO 32000-2 §14.9.3 makes `/Alt` "a complete (or whole) word or phrase substitution for the
    /// current element", and §14.9.5 says the same of `/E` for what it expands — so an element
    /// stating one has said what to speak *instead of* its content, and a host handing this to a
    /// platform API stops there rather than descending. `false` is the ordinary case: the name is
    /// this element's own text and its children carry theirs.
    pub substituted: bool,
    /// §14.9.2's language, where the element or an enclosing one states one.
    pub language: Option<String>,
    /// Where the element is, in device pixels of the viewport, as quadrilaterals.
    ///
    /// The same shapes and the same space [`crate::Query::Selection`] answers in, so a host that
    /// draws a selection can draw a focus ring with no second mapping. Empty for an element whose
    /// content drew no text — a figure, a table cell holding an image — which is a statement
    /// about this program's text layer rather than about the element.
    pub quads: Vec<[f32; 8]>,
    /// Which of a table's axes this element describes, for a `TH` and nothing else.
    ///
    /// §14.8.4.8.3 makes a table header cell one "describing one or more rows, columns or rows and
    /// columns of the table", and Table 384's `/Scope` is which. It crosses because a host cannot
    /// work it out: where the document states no `/Scope`, §14.8.5.7's answer is an assumption
    /// about the cell's place in its table's *grid*, and the grid is a fact about the structure
    /// tree — spans and all — that this side has and the other side does not.
    ///
    /// `None` for every element that is not a `TH`, and for a `TH` this reader could place in no
    /// grid, which is one a document put outside a `TR`. The second of those is not the same as a
    /// column header and is deliberately not reported as one: a host says it does not know.
    pub header_scope: Option<HeaderScope>,
    /// Where the **document says** the element is, in the same device pixels [`Self::quads`] are
    /// in.
    ///
    /// A different kind of statement from [`Self::quads`]: those are the shapes *this program*
    /// found by drawing the element's text, and this is what the producer wrote down. The two are
    /// carried side by side rather than merged, because an element whose content is a picture or a
    /// form control has the second and not the first, and a host that wants somewhere to point a
    /// magnifier can take whichever it has.
    ///
    /// # Two clauses answer, in this order
    ///
    /// **Table 379's `/BBox` first**, which §14.8.5.4.3 makes "the rectangle that completely
    /// encloses its visible content" — the element's own statement about itself.
    ///
    /// **Then §12.5.2's annotation rectangle**, for an element whose own content item is
    /// §14.7.5.3's object reference to an annotation this page lists. Table 166 makes `/Rect`
    /// required and "defining the location of the annotation on the page in default user space
    /// units", so where the element's content *is* an annotation the document has said where the
    /// element is without using a layout attribute at all — the union of the rectangles where the
    /// element names more than one, which Table 368's `Annot` permits. The `/BBox` wins because it
    /// is a statement about the *element* and this one is a statement about its content.
    ///
    /// `pdf-model --example element_bounds_census`, over 1245 documents: 2079 elements produced no
    /// text, 404 of them state a `/BBox`, and **333 of the remainder are placed by an annotation
    /// rectangle** — among them every one of the 272 `Form` elements, which mark no text by nature.
    ///
    /// `[x0, y0, x1, y1]` with y increasing downwards, because the mapping from those clauses'
    /// default user space runs through the same flip [`Self::quads`] take: §7.7.3.3's `/Rotate`
    /// and the crop box's origin first, then the viewport's magnification and centring.
    ///
    /// `None` for an element neither clause answers for, which is most of them, and that is not a
    /// failure.
    pub bounds: Option<[f32; 4]>,
    /// Which of §12.7.5's controls the widget annotation behind this element is, where it names
    /// one.
    ///
    /// §14.8.4.7.2's `Form` is the structure type this exists for. Table 368 makes it "[e]ither an
    /// association between content enclosed by the Form structure element and a corresponding
    /// widget annotation or a mechanism to include a widget annotation in the structure tree", and
    /// requires one per widget: "[i]n a tagged PDF, Form shall be used for each PDF widget
    /// annotation that belongs to the real content of the document". So a `Form` is a *control*,
    /// and a host that announced it as a group would tell a person there is a box on the page
    /// without saying it is a check box, what it is called, or whether it is ticked.
    ///
    /// The route is §14.7.5.3's object reference, which is the only thing that names the widget,
    /// and the answer is `pdf_model::form`'s — the same [`pdf_model::form::Control`] a host
    /// already builds a native control from in [`crate::FormField`], with the same view state
    /// behind it, so a check box a person has just ticked answers `on`.
    ///
    /// Carried for **any** element whose own object reference names a widget of a field on this
    /// page, rather than only for a `Form`: which type *should* name one is §14.8.4.7.2's
    /// question and a host's to apply, and a file that puts the reference on some other element
    /// has stated a fact this crate has no reason to withhold.
    ///
    /// `None` for every element that names no widget, which is all but 272 of the corpus's 166 115
    /// (`pdf-model --example element_bounds_census`) — and for a widget the field tree does not
    /// reach, which §12.7.4.2 makes "simply a Widget annotation" belonging to no field.
    pub control: Option<pdf_model::form::Control>,
    /// The header cells that describe this one, as indices into the answer.
    ///
    /// §14.8.4.8.3 gives a table cell its headers twice over — Table 384's `/Headers`, an array of
    /// the `/ID`s the producer wrote, and an algorithm for every cell that states none — and both
    /// are read. The order is the clause's: the row's headers, then the column's, each from most
    /// specific to most general.
    ///
    /// **Indices rather than names**, because a header is a *node* and the host already has its
    /// text, its role and its bounds; a copied string would be a second statement of the same
    /// thing that could disagree with the first.
    ///
    /// **An index is always lower than this node's own.** §14.8.4.8.3's search walks *out* from a
    /// cell towards its table's first, so what it finds is always earlier in the tree; a
    /// `/Headers` array naming a cell later than the one that states it has no index in a
    /// parent-first list and is dropped, which no corpus document does — all 475 stated entries
    /// name a `TH` earlier in the walk, measured.
    ///
    /// Empty for every element that is not a table cell, and for a cell no header describes —
    /// 4452 of the corpus's 21 883 cells, against 17 431 that end with at least one and 17 152 of
    /// those answered by the search rather than by an array, measured by
    /// `pdf-model --example cell_header_census`.
    ///
    /// **A header on another page is not here**, and that is a real loss rather than a decision:
    /// this answer is one page's, so a table whose header row is on the page before has its data
    /// cells' headers pruned away with it. Nothing in §14.8 makes a table stay on one page.
    pub headers: Vec<usize>,
    /// The element's own text again, one line at a time, with each character's place.
    ///
    /// [`Self::name`] is what the element is *called* and this is what it *says*, and the two are
    /// different questions with different answers. A name is one string for a whole paragraph, so
    /// an assistive technology reads the paragraph or does not; moving through it by character, by
    /// word or by line, and reporting where the caret is, needs to know where each character
    /// begins and which characters share a line. That is what a platform text interface asks for —
    /// AT-SPI's `org.a11y.atspi.Text`, `NSAccessibility`'s marked ranges, UIA's `TextPattern` —
    /// and none of them can be built from a string.
    ///
    /// # Why this is the readback and not the speech
    ///
    /// [`Self::name`] applies §14.9's substitutions: an `/ActualText` inside the element replaces
    /// what was drawn, and an `/E` expands an abbreviation. This does **not**, deliberately. A
    /// caret moves over what is on the page, and `GetCharacterExtents` asks where the *glyph* is;
    /// a substitution has no glyphs, so a run built out of one would report positions for
    /// characters nobody drew. The two answers are carried side by side for the same reason
    /// [`Self::quads`] and [`Self::bounds`] are.
    ///
    /// Empty for an element that states §14.9.3's `/Alt` or §14.9.5's `/E` of its own — the phrase
    /// substitutes for the whole element, which is why [`Self::substituted`] also stops a host
    /// descending — and for one whose own content items drew no text, which is most of them.
    ///
    /// The lines are the element's **own** content items, as [`Self::name`] is, and in the order
    /// the page drew them within §14.8.2.5's order over the elements.
    pub lines: Vec<TextLine>,
}

/// One line of an element's text, and where each of its characters is.
///
/// A *line* here is what [`crate::Query::Selection`]'s merge already means by one: a run of
/// character codes sharing both baseline corners' y, each beginning no further along than the last
/// one ended. It is the page's own geometry rather than a paragraph's logical line, which is the
/// only definition available to a reader — a PDF states no line breaks, and §9.4.2's `TJ` and `T*`
/// leave the line to be recovered from where the glyphs landed.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLine {
    /// Exactly the readback of [`Self::characters`], concatenated.
    ///
    /// The invariant a text interface needs and the reason the two are one type: the sum of the
    /// characters' [`Character::bytes`] is this string's length, so an offset into the string and
    /// an index into the characters convert into each other without either side guessing.
    pub text: String,
    /// One entry per character code the page drew, in the order it drew them.
    pub characters: Vec<Character>,
}

/// One character code's share of a line: how much of the text it produced, and where it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Character {
    /// How many bytes of [`TextLine::text`] this code produced.
    ///
    /// Usually one character's worth and not always: a code mapped through `/ToUnicode` to a
    /// several-character string — a ligature read back as `ffi` — produced one glyph in one place,
    /// and splitting its box into thirds would invent positions the file does not state. So the
    /// unit a caret moves by is the *code*, which is the unit the page actually drew.
    pub bytes: usize,
    /// Where the glyph is, in device pixels of the viewport: `[x0, y0, x1, y1]`.
    ///
    /// The same space [`AccessibilityNode::quads`] and [`AccessibilityNode::bounds`] are in. A
    /// rectangle rather than the quadrilateral, because a platform asks a character's extent as
    /// one; the quadrilateral is still what `quads` carries.
    pub bounds: [f32; 4],
}

/// What one element contributes before its quads are mapped into the viewport.
pub(crate) struct Gathered {
    /// The element's `/S`, role mapped.
    pub(crate) role: String,
    /// The `/MCID`s below it, including those of its descendants.
    ///
    /// Descendants' too, because the *place* an element occupies is everything it encloses: a
    /// focus ring round a table cell is drawn round what the cell contains. What it is *spoken*
    /// as is [`Self::own`], for the reason [`AccessibilityNode::name`] gives.
    pub(crate) mcids: Vec<i64>,
    /// The `/MCID`s of the element's own content items, without its descendants'.
    pub(crate) own: Vec<i64>,
    /// Whether this element, or one below it, names a content item on the page being asked about.
    ///
    /// The one thing an `/OBJR` contributes: §14.7.5.3's object reference is an annotation or an
    /// `XObject` rather than text, so it produces no `/MCID` and no quadrilateral — but Table 358
    /// gives it a `/Pg`, and an element whose only content item is an annotation on this page is
    /// on this page. Without it, such an element would be pruned as belonging elsewhere.
    pub(crate) on_page: bool,
    /// The objects the element's **own** §14.7.5.3 object references name, on this page.
    ///
    /// Its own rather than its descendants', which is the same division [`Self::own`] makes and
    /// for a sharper reason: what these answer is where the element is and what control it is, and
    /// both are statements about the object that *is* this element's content item. An ancestor's
    /// extent is a different question, and the standard states no union for it.
    pub(crate) objects: Vec<ObjectId>,
    /// §14.9.3's `/Alt` or §14.9.5's `/E`, where the element itself states one.
    pub(crate) phrase: Option<String>,
    /// §14.9.2's `/Lang`, where the element itself states one.
    pub(crate) language: Option<String>,
    /// Table 384's `/Scope` for a `TH`, stated or assumed.
    pub(crate) header_scope: Option<HeaderScope>,
    /// Table 379's `/BBox`, in **default user space** — the space the clause states it in.
    ///
    /// Mapped to the viewport by [`finish`], for the reason [`AccessibilityNode::bounds`] gives:
    /// this side of the walk has no magnification and no origin, and the flip between the page's
    /// y axis and the raster's belongs to whoever holds them.
    pub(crate) bounds: Option<[f32; 4]>,
    /// §14.8.4.8.3's header cells, as indices into this list — **before** [`prune`] moves them.
    ///
    /// Filled after the walk rather than during it, because Table 384's `/Headers` names cells by
    /// an identifier and nothing in the standard makes the cell it names one the walk has already
    /// reached. `pdf_model::structure::TableStack::headers` is what answers, in the tokens this
    /// walk gave it, which are exactly these indices.
    pub(crate) headers: Vec<usize>,
}

/// Reads the page's part of §14.7's structure tree.
///
/// `page` is the page object, which is what Table 355's `/Pg` names; an element belonging to
/// another page is skipped rather than answered with, because a screen reader is being told what
/// is on the screen.
///
/// Returns an empty list for an untagged document, which is 885 of the corpus's 974 — and that is
/// an answer rather than a failure: a host asking this of an untagged page learns that the page
/// says nothing about its own structure, which is exactly what §14.7 leaves it to say.
pub(crate) fn nodes(
    document: &Document,
    page: ObjectId,
    default_language: Option<&str>,
) -> Vec<(Option<usize>, Gathered)> {
    let Some(tree) = Tree::of(document) else {
        return Vec::new();
    };
    // §14.7.5.4's parent tree: which elements this page's content items name, and what is above
    // them. `None` where the page has not said — Table 359 requires `/StructParents` of "all
    // content streams containing marked-content sequences that are structural content items", so
    // a page lacking one has left the question unanswered and the whole tree is what is left.
    let owners = document
        .get(page)
        .as_dict()
        .and_then(|dict| tree.elements_on_page(document, dict));
    let within = owners
        .as_ref()
        .map(|owners| tree.ancestry(document, owners));
    let gathering = gather(document, &tree, page, default_language, within.as_ref());
    // The two chains a document states are `/K` downwards and `/P` upwards, and nothing checks
    // that they agree. Where they do not, an element the parent tree named is not where its own
    // ancestry says it is, and the walk above stepped over it — so the walk that needs no
    // agreement is run instead. It is what this function did for every page until ADR 0325, and
    // it is the right answer to a file whose two statements about itself differ.
    //
    // Asked of the walk that was actually made: a walk the bound stopped reached less than the
    // page's subtree for a reason of its own, and answering it with the walk that has the same
    // bound would be the same shortfall twice.
    let agreed = owners
        .as_ref()
        .is_none_or(|owners| owners.iter().all(|id| gathering.reached.contains(id)));
    if !agreed && !gathering.bounded {
        return gather(document, &tree, page, default_language, None).nodes;
    }
    gathering.nodes
}

/// One walk's result: what it gathered, which elements it reached, and whether it ran out of room.
struct Gathering {
    /// The page's elements, pruned and parent-first.
    nodes: Vec<(Option<usize>, Gathered)>,
    /// Every element the walk entered, by identity, **before** the pruning.
    ///
    /// What [`nodes`] checks §14.7.5.4's answer against: an element the parent tree named and this
    /// set does not hold is one the walk did not reach.
    reached: BTreeSet<ObjectId>,
    /// Whether [`MAX_NODES`] stopped the walk before the tree ran out.
    ///
    /// Not derivable from [`Self::nodes`], which is what makes it a field: pruning happens after
    /// the bound, so a walk that gathered 8192 elements of another page's and kept none of them
    /// answers with an empty list that looks exactly like a page with no structure.
    bounded: bool,
}

/// One walk of the tree, pruned to `within` where the page's own subtree is known.
///
/// Answers a [`Gathering`], whose other two fields are what [`nodes`] checks the pruning against.
fn gather(
    document: &Document,
    tree: &Tree,
    page: ObjectId,
    default_language: Option<&str>,
    within: Option<&BTreeSet<ObjectId>>,
) -> Gathering {
    let mut out: Vec<(Option<usize>, Gathered)> = Vec::new();
    let mut reached: BTreeSet<ObjectId> = BTreeSet::new();
    // §14.8.4.8.3's tables, kept as the walk descends: a cell's place in its grid is what
    // §14.8.5.7 assumes a header's axis from, and it is not knowable from the cell alone.
    let mut tables = TableStack::new();
    walk(
        document,
        tree,
        None,
        None,
        page,
        default_language,
        0,
        within,
        &mut tables,
        &mut reached,
        &mut out,
    );
    // §14.8.4.8.3's headers, once the whole tree has been seen — see `Gathered::headers`.
    for (token, headers) in tables.headers() {
        if let Some((_, entry)) = out.get_mut(token) {
            entry.headers = headers;
        }
    }
    // The bound, read before the pruning throws away the only evidence that it was reached.
    let bounded = out.len() >= MAX_NODES;
    Gathering {
        nodes: prune(out),
        reached,
        bounded,
    }
}

/// Drops the elements that have nothing on this page, and repairs the parent links.
///
/// An element is kept when it names a content item on this page or something below it does —
/// which is what `mcids` and `on_page` already record, because both are pushed up the chain to
/// every ancestor as the walk meets them. Order is preserved, so the answer stays parent-first
/// and a parent index stays lower than its child's.
fn prune(gathered: Vec<(Option<usize>, Gathered)>) -> Vec<(Option<usize>, Gathered)> {
    let mut moved: Vec<Option<usize>> = vec![None; gathered.len()];
    let mut out: Vec<(Option<usize>, Gathered)> = Vec::new();
    for (index, (parent, mut entry)) in gathered.into_iter().enumerate() {
        if entry.mcids.is_empty() && !entry.on_page {
            continue;
        }
        // §14.8.4.8.3's search only ever walks *out* to a header, so its new index is already
        // known here. Two kinds of header are dropped instead of being pointed at, and both are
        // stated in `AccessibilityNode::headers`: one on another page, which this answer is not
        // about, and one a `/Headers` array names *later* in the tree, which a parent-first list
        // has no index for yet. No corpus document states the second.
        entry.headers = entry
            .headers
            .iter()
            .filter_map(|header| moved.get(*header).copied().flatten())
            .collect();
        // A kept element's nearest kept ancestor. The walk pushed every content item to every
        // ancestor, so an ancestor of a kept element is itself kept and this is always the
        // parent — but reading it out of the map rather than assuming it is what makes the
        // answer well formed whatever the bounds did.
        let above = parent.and_then(|above| moved.get(above).copied().flatten());
        if let Some(slot) = moved.get_mut(index) {
            *slot = Some(out.len());
        }
        out.push((above, entry));
    }
    out
}

/// What §14.9 says one element's span of the readback should be spoken as.
///
/// The element's own `/Alt` or `/E` wins where it states one, because that is a substitution for
/// the whole element. Where it does not, the text is taken through
/// [`pdf_model::accessibility::speech`] — the *sequence*-level `/Alt`, `/E` and `/ActualText` the
/// interpreter recorded — rather than raw, because a `/Alt` on a `BDC` inside the element is as
/// much a substitution as one on the element, and reading the raw text would speak the letters an
/// abbreviation is written with.
///
/// The `Described` spans are rebased onto the slice, so a span that straddles the element's edge
/// is clipped to it rather than dropped: half a substitution is still what the clause says about
/// the half that is here.
fn spoken(text: &str, described: &[Described], spans: &[(usize, usize)]) -> String {
    let mut out = String::new();
    for (start, end) in spans {
        let Some(slice) = text.get(*start..*end) else {
            continue;
        };
        let within: Vec<Described> = described
            .iter()
            .filter(|item| item.range.start < *end && item.range.end > *start)
            .map(|item| Described {
                range: item.range.start.max(*start).saturating_sub(*start)
                    ..item.range.end.min(*end).saturating_sub(*start),
                alt: item.alt.clone(),
                expansion: item.expansion.clone(),
                language: item.language.clone(),
            })
            .collect();
        for run in pdf_model::accessibility::speech(slice, &within, None) {
            out.push_str(&run.text);
        }
    }
    out
}

/// One element and everything below it, appended to `out` parent-first.
///
/// `within` is §14.7.5.4's answer to "which elements does this page occupy", and a child outside
/// it is stepped over rather than descended into — the page's subtree instead of the document's.
/// `None` descends into everything, which is what a file that named no elements for this page
/// gets and what the inside of a **table** gets whatever the page said: §14.8.5.7 assumes a header
/// cell's axis from its place in the table's grid, and a table continued from the page before has
/// that place only if the rows before it were counted.
#[expect(
    clippy::too_many_arguments,
    reason = "a walk of a tree carries what it is walking, where it is, and what it inherits; \
              grouping them would build a struct that exists once per call"
)]
fn walk(
    document: &Document,
    tree: &Tree,
    element: Option<&Dictionary>,
    parent: Option<usize>,
    page: ObjectId,
    language: Option<&str>,
    depth: usize,
    within: Option<&BTreeSet<ObjectId>>,
    tables: &mut TableStack,
    reached: &mut BTreeSet<ObjectId>,
    out: &mut Vec<(Option<usize>, Gathered)>,
) {
    if depth >= MAX_DEPTH || out.len() >= MAX_NODES {
        return;
    }
    for (child, id) in tree.identified_children(document, element) {
        match child {
            Child::Element(dict) => {
                // Table 355 makes `/P` an indirect reference, so every element the parent tree
                // can name is one this walk reaches through a reference too: an element with no
                // identity is inside its parent's `/K` and is neither named by §14.7.5.4 nor an
                // ancestor of anything that is.
                if let Some(within) = within
                    && id.is_none_or(|id| !within.contains(&id))
                {
                    continue;
                }
                if let Some(id) = id {
                    reached.insert(id);
                }
                // §14.9.2.3's hierarchy: the innermost stated `/Lang` wins, and an element that
                // states none inherits what encloses it.
                let language =
                    text_entry(document, &dict, "Lang").or_else(|| language.map(str::to_owned));
                // §14.7.3's role map, which is a `shall` on whoever reads a structure type and
                // not a courtesy: "[a] structure type shall always be mapped to its corresponding
                // name in the role map, if there is one, even if the original name is one of the
                // standard types." `Tree::role` follows it transitively and through §14.8.6.2's
                // namespace maps; an element with no `/S` at all keeps the empty name, because
                // Table 355 requires the entry and inventing one would be a guess.
                let role = tree.role(document, &dict).unwrap_or_default();
                let phrase =
                    text_entry(document, &dict, "Alt").or_else(|| text_entry(document, &dict, "E"));
                let index = out.len();
                let header_scope = header_scope(document, tree, &dict, &role, depth, index, tables);
                let bounds = tree.bounds(document, &dict);
                // Inside a table the pruning stops, for the reason this function's own comment
                // gives: a grid missing the rows on the page before places every cell wrong.
                let below = match StandardType::read(&role) {
                    Some(StandardType::Table) => None,
                    _ => within,
                };
                out.push((
                    parent,
                    Gathered {
                        role,
                        mcids: Vec::new(),
                        own: Vec::new(),
                        on_page: false,
                        objects: Vec::new(),
                        phrase,
                        language: language.clone(),
                        header_scope,
                        bounds,
                        headers: Vec::new(),
                    },
                ));
                walk(
                    document,
                    tree,
                    Some(&dict),
                    Some(index),
                    page,
                    language.as_deref(),
                    depth.saturating_add(1),
                    below,
                    tables,
                    reached,
                    out,
                );
            }
            Child::MarkedContent { mcid, page: on } => {
                // Table 355 makes `/Pg` the page a bare integer belongs to; an element with no
                // `/Pg` anywhere up the chain is taken to be on the page being asked about,
                // which is what a single-page structure tree means by stating nothing.
                if on.is_some_and(|object| object != page) {
                    continue;
                }
                // The identifier belongs to the element that contains it *and* to every element
                // above it, because an ancestor's *extent* is everything it encloses. Its own
                // spoken text is only the first of those, which is what `own` records.
                if let Some(index) = parent
                    && let Some((_, entry)) = out.get_mut(index)
                {
                    entry.own.push(mcid);
                }
                let mut at = parent;
                while let Some(index) = at {
                    let Some((above, entry)) =
                        out.get_mut(index).map(|(above, entry)| (*above, entry))
                    else {
                        break;
                    };
                    entry.mcids.push(mcid);
                    at = above;
                }
            }
            // §14.7.5.3's object reference is an annotation or an XObject rather than text on
            // this page's readback. It contributes no `/MCID`, so its element gets no quads — but
            // it is not the dead end this comment used to describe: where the object is one of the
            // page's annotations, §12.5.2 states where it is and §12.7 states what control it is,
            // and both reach [`AccessibilityNode`] through `objects`. Table 358's `/Pg` is what
            // keeps such an element from being pruned as belonging to another page.
            Child::Object { object, page: on } => {
                if on.is_some_and(|object| object != page) {
                    continue;
                }
                // The element that *contains* the reference, for the reason `Gathered::objects`
                // gives: this is a statement about its own content item and not its ancestors'.
                if let Some(index) = parent
                    && let Some((_, entry)) = out.get_mut(index)
                {
                    entry.objects.push(object);
                }
                let mut at = parent;
                while let Some(index) = at {
                    let Some((above, entry)) =
                        out.get_mut(index).map(|(above, entry)| (*above, entry))
                    else {
                        break;
                    };
                    entry.on_page = true;
                    at = above;
                }
            }
        }
    }
}

/// Which of a table's axes one element describes, where the element is a `TH`.
///
/// The stack is told about **every** element rather than only the table ones, because it is what
/// closes a table the walk has left: a walk that named only tables would keep one open under
/// everything that came after it, and a paragraph three sections later would be placed in a grid.
///
/// `index` is the element's place in the answer being built, which the stack keeps so that
/// [`nodes`] can ask it for §14.8.4.8.3's headers once the whole tree has been walked.
fn header_scope(
    document: &Document,
    tree: &Tree,
    dict: &Dictionary,
    role: &str,
    depth: usize,
    index: usize,
    tables: &mut TableStack,
) -> Option<HeaderScope> {
    let kind = StandardType::read(role);
    let placement = tables.enter(depth, kind.as_ref(), index, || {
        tree.cell_facts(document, dict)
    });
    if kind != Some(StandardType::TableHeader) {
        return None;
    }
    // Table 384's own value where the document states one, and §14.8.5.7's assumption where it
    // does not — which needs the cell's place in the grid and answers nothing without it.
    tree.header_scope(document, dict)
        .or_else(|| placement.map(|cell| HeaderScope::assumed(cell.row, cell.column)))
}

/// A text-string entry, decoded through §7.9.2.2's rules.
fn text_entry(document: &Document, dict: &Dictionary, key: &str) -> Option<String> {
    let value = document.get_key(dict, key);
    let bytes = value.as_string()?;
    let text = pdf_syntax::text_string::text_string(bytes);
    (!text.is_empty()).then_some(text)
}

/// The byte ranges of the readback that a set of `/MCID`s covers.
pub(crate) fn ranges(marked: &[MarkedSpan], mcids: &[i64]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = marked
        .iter()
        .filter(|span| mcids.contains(&span.mcid) && span.range.start < span.range.end)
        .map(|span| (span.range.start, span.range.end))
        .collect();
    out.sort_unstable();
    out
}

/// What one page says, beside each element: the readback, and what its object references name.
///
/// One value rather than five arguments because it is the *page's* half of the answer and every
/// element of the page is finished against the same one.
pub(crate) struct Readback<'a> {
    /// The page's text, as [`crate::Interpretation::text`] read it back.
    pub(crate) text: &'a str,
    /// §14.7.5.2's marked-content sequences, with the range of the readback each covers.
    pub(crate) marked: &'a [MarkedSpan],
    /// §14.9's substitutions the interpreter recorded inside those sequences.
    pub(crate) described: &'a [Described],
    /// §12.5.2's `/Rect` for each annotation this page lists, in **default user space**.
    ///
    /// [`pdf_model::structure::annotation_rectangles`] is what reads it, and the mapping into the
    /// viewport is [`finish`]'s `place` — the same one Table 379's rectangle takes, because both
    /// clauses state their rectangle in the same space.
    pub(crate) places: &'a BTreeMap<ObjectId, [f32; 4]>,
    /// §12.7's control for each widget annotation of a field with a widget on this page.
    pub(crate) controls: &'a BTreeMap<ObjectId, pdf_model::form::Control>,
}

/// Turns a gathered element into what crosses the boundary.
pub(crate) fn finish(
    gathered: Gathered,
    parent: Option<usize>,
    page: &Readback<'_>,
    quads: impl Fn(usize, usize) -> Vec<[f32; 8]>,
    lines: impl Fn(&[(usize, usize)]) -> Vec<TextLine>,
    place: impl Fn([f32; 4]) -> Option<[f32; 4]>,
) -> AccessibilityNode {
    let substituted = gathered.phrase.is_some();
    let own = ranges(page.marked, &gathered.own);
    let name = gathered.phrase.unwrap_or_else(|| {
        // The element's own content items, not its descendants': see `AccessibilityNode::name`.
        spoken(page.text, page.described, &own)
    });
    // Nothing to move a caret through where the element has said what to say instead of its
    // content: see `AccessibilityNode::lines`.
    let drawn = if substituted { Vec::new() } else { lines(&own) };
    let mut all = Vec::new();
    for (start, end) in ranges(page.marked, &gathered.mcids) {
        all.extend(quads(start, end));
    }
    // Table 379's rectangle first, then §12.5.2's: see `AccessibilityNode::bounds` for why that
    // order and not the other.
    let stated = gathered
        .bounds
        .or_else(|| referenced_rectangle(&gathered.objects, page.places));
    AccessibilityNode {
        parent,
        role: gathered.role,
        name,
        substituted,
        language: gathered.language,
        quads: all,
        header_scope: gathered.header_scope,
        bounds: stated.and_then(place),
        control: gathered
            .objects
            .iter()
            .find_map(|object| page.controls.get(object).cloned()),
        headers: gathered.headers,
        lines: drawn,
    }
}

/// The rectangle §12.5.2 gives the annotations an element's own object references name.
///
/// The union where there is more than one, because Table 368 permits it — an `Annot` element
/// referencing several requires only that "they shall be of the same annotation type" — and a
/// magnifier pointed at one of several would be pointed at the wrong one as often as not.
///
/// `None` where the element names no object, and where none of the objects it names is an
/// annotation of this page: an `XObject` reference is the clause's other case and has no rectangle
/// of its own, which [`pdf_model::structure::annotation_rectangles`] states.
fn referenced_rectangle(
    objects: &[ObjectId],
    places: &BTreeMap<ObjectId, [f32; 4]>,
) -> Option<[f32; 4]> {
    let mut union: Option<[f32; 4]> = None;
    for rect in objects.iter().filter_map(|object| places.get(object)) {
        union = Some(match union {
            None => *rect,
            Some(so_far) => [
                so_far[0].min(rect[0]),
                so_far[1].min(rect[1]),
                so_far[2].max(rect[2]),
                so_far[3].max(rect[3]),
            ],
        });
    }
    union
}
