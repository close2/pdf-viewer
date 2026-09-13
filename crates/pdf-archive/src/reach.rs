//! What reaches an object, and what nothing reaches: one walk of the file, shared by every row.
//!
//! # The question this answers, and why no predicate could
//!
//! A [`crate::Requirement`]'s predicate is handed an [`crate::Examination`] and sees *one object
//! at a time*. Several requirements of ISO 19005 are not about an object at all but about a
//! **relationship between objects** — whether anything reaches this one, and by what route. The
//! standing case is the last sentence of ISO 19005-2 section 6.2.2 and ISO 19005-4 section 6.2.2,
//! the exemption [`Exempt`] below computes: a named resource the associated content stream never
//! references is not used for rendering. Answering it needs the whole file, once, which is what
//! this module is.
//!
//! It was a hand walk in `examples/unreferenced.rs` before it was this, and the example is now a
//! printer over it. `doc/todo/62` is the item, `doc/adr/1021` the decision.
//!
//! # The roots, and why they are more than `/Root`
//!
//! ISO 32000-2 §7.5.5 makes the trailer the entry point, so the walk starts at every reference
//! the trailer states — `/Root` and `/Info` in nearly every file, `/Encrypt` where §7.6 applies.
//! Two further populations are named by the file's *structure* rather than by any reference, and
//! a walk that did not start at them would report the file's own plumbing as unreferenced:
//!
//! - §7.5.7's object streams. A compressed object's location is "which stream, and the index
//!   within it", so the container is named by the cross-reference table and by nothing else.
//! - §7.5.8's cross-reference streams, including the one a hybrid-reference file's `/XRefStm`
//!   names (§7.5.8.4). Each is found by a byte offset — `startxref`, then the `/Prev` chain —
//!   and the trailer this reader hands out is the merge of every section's, so the sections
//!   themselves are reachable by no edge of the object graph.
//!
//! **Annex F's linearisation parameter dictionary and hint stream are deliberately not roots.**
//! The first object of a linearised file states `/Linearized`, and its `/H` gives the hint
//! stream's offset and length as integers rather than as a reference, so no edge of the object
//! graph reaches either. A walk that named them would be implementing a normative annex
//! `CLAUDE.md`'s scope list leaves out until linearisation is separately ratified, and leaving
//! them in [`Reach::unreferenced`] is the true answer to the question this module asks. They are
//! most of what that set holds over `doc/veraPDF-corpus`; a stray `/Info` no trailer names, an
//! orphaned integer and a `/Metadata` nothing references are the rest, and each of those three is
//! a fact about the document worth having.
//!
//! # What a bound does here, and which direction it fails in
//!
//! Every walk in this tree is bounded, and this one's bound decides whether a requirement is
//! *withdrawn*. So when a bound stops the walk the answer is not truncated, it is **refused**:
//! [`Reach::unreferenced`] and [`Exempt::objects`] are empty, [`Reach::stopped_at`] names the
//! limit, and every row judges every object exactly as it did before this module existed. An
//! exemption inferred from a walk that did not finish is a failure withdrawn in silence, which
//! `doc/todo/62` section 3 names as the one direction a validator may not move by accident.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Lexer, Location, Name, Object, ObjectId, Token};

use crate::examination::Examination;
use crate::requirement::Clauses;
use crate::target::{Part, Target};

/// How one edge of the object graph was taken.
///
/// A [`Reach::path`] is a sequence of these, root first, so that a report can say *how* a
/// requirement's subject is reached and not merely that it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// A dictionary key — `/Root`, `/Kids`, `/Font`.
    Key(Name),
    /// A zero-based position in an array.
    Index(usize),
    /// The file's own structure named it rather than any reference: see the module comment.
    Structure(&'static str),
}

/// How one object was first reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrival {
    /// The object the edge left, or `None` where the trailer or the file's structure named it.
    pub from: Option<ObjectId>,
    /// The entries *within* that object that led here, outermost first.
    ///
    /// More than one where the reference sits inside a direct value: a page reached through the
    /// `/Kids` array of a `/Pages` node arrives through `[Key("Kids"), Index(0)]`, and recording
    /// only the innermost of those would say the page came from the node's first entry of any
    /// kind. `doc/reviews/984` finding 5 asks for "the entries it was reached through", plural,
    /// and the plural is the load-bearing part.
    pub through: Vec<Entry>,
}

/// A bound this walk stopped at, by name.
///
/// `CLAUDE.md` principle 3: a bound is reported rather than applied in silence, and here it is
/// load-bearing for correctness as well as for honesty — see the module comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    /// More references were examined than [`EDGES`].
    Edges(usize),
    /// A direct value nested deeper than [`DEPTH`] inside one object.
    Depth(usize),
}

/// How many references one walk examines before it refuses to answer.
///
/// Sized against the largest file this project measures anything on — ISO 32000-2's own
/// specification, 110 000 objects — with room for a document an order of magnitude larger. A
/// file that crosses it gets the unbounded answer, which is the answer this crate gave before
/// the walk existed.
pub const EDGES: usize = 8_000_000;

/// How deeply a *direct* value may nest inside one object before the walk refuses.
///
/// Not the depth of the object graph, which needs no bound because each object is visited once:
/// this is the array-inside-dictionary-inside-array nesting within a single object's own bytes.
///
/// **It is `pdf_syntax::Limits::max_depth`'s number, and that crate refuses first**, so this
/// bound cannot fire for a value this reader parsed — the parser will not build the object. It is
/// here because the traversal below must be total over any [`Object`] it is handed rather than
/// over the ones one parser happens to produce, and because a bound that depends on another
/// crate's constant is better stated than assumed. A round that raises the parser's limit and
/// not this one gets a refusal named [`Limit::Depth`] rather than an exemption read off a walk
/// that stopped.
pub const DEPTH: usize = 256;

/// Which objects the file reaches from its trailer, by what route, and which it reaches not at all.
///
/// Computed once per [`crate::Examination`] and lent to every row; see [`Examination::reaches`].
#[derive(Debug, Clone)]
pub struct Reach {
    /// Every object reached, with how it was first reached.
    arrivals: BTreeMap<ObjectId, Arrival>,
    /// Every object a cross-reference section names that the walk did not reach.
    unreferenced: BTreeSet<ObjectId>,
    /// The bound that stopped the walk, where one did.
    stopped: Option<Limit>,
}

impl Reach {
    /// Walks the document once from the roots the module comment names.
    #[must_use]
    pub fn of(exam: &Examination<'_>) -> Self {
        let walk = Walk::run(exam, &Cuts::none());
        let unreferenced = if walk.stopped.is_some() {
            BTreeSet::new()
        } else {
            exam.objects()
                .iter()
                .map(|(id, _)| *id)
                .filter(|id| !walk.arrivals.contains_key(id))
                .collect()
        };
        Self {
            arrivals: walk.arrivals,
            unreferenced,
            stopped: walk.stopped,
        }
    }

    /// Whether the walk reached this object.
    #[must_use]
    pub fn reaches(&self, id: ObjectId) -> bool {
        self.arrivals.contains_key(&id)
    }

    /// How this object was first reached, or `None` where nothing reached it.
    #[must_use]
    pub fn arrival(&self, id: ObjectId) -> Option<&Arrival> {
        self.arrivals.get(&id)
    }

    /// The entries this object was reached through, root first.
    ///
    /// **One route rather than every route**, and which one is decided by the breadth-first
    /// order: the fewest objects, and among equally short routes the one the walk met first. A
    /// question about *all* the routes to an object is a different walk, and [`Exempt`] is the
    /// one this crate needs — it asks whether every route passes through a named resource entry
    /// nothing references, which is answered by cutting those edges and walking again rather
    /// than by enumerating paths.
    #[must_use]
    pub fn path(&self, id: ObjectId) -> Option<Vec<Entry>> {
        let mut path: Vec<Vec<Entry>> = Vec::new();
        let mut at = id;
        // Each step moves to the object the arrival came from, and an arrival is recorded only
        // when the object is first reached, so the links form a tree and cannot cycle. The
        // count is bounded by the arrivals for the same reason; the guard states it rather than
        // trusting it.
        for _ in 0..=self.arrivals.len() {
            let arrival = self.arrivals.get(&at)?;
            path.push(arrival.through.clone());
            match arrival.from {
                None => {
                    path.reverse();
                    return Some(path.concat());
                }
                Some(previous) => at = previous,
            }
        }
        None
    }

    /// Every object a cross-reference section names that the walk did not reach.
    ///
    /// Empty where a bound stopped the walk, which the module comment argues for: an
    /// unreferenced set read off an unfinished walk names objects the file may well reference.
    #[must_use]
    pub fn unreferenced(&self) -> &BTreeSet<ObjectId> {
        &self.unreferenced
    }

    /// How many objects the walk reached.
    #[must_use]
    pub fn len(&self) -> usize {
        self.arrivals.len()
    }

    /// Whether the walk reached nothing at all — a file with no trailer worth the name.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.arrivals.is_empty()
    }

    /// The bound that stopped this walk, where one did.
    #[must_use]
    pub const fn stopped_at(&self) -> Option<Limit> {
        self.stopped
    }
}

/// The resources dictionary's own subdictionaries, ISO 32000-2 §7.8.3's Table 34.
///
/// `/ProcSet` is absent: its value is an array of names rather than a dictionary of named
/// resources, so it holds no entry this exemption could be about.
const CATEGORIES: [&str; 7] = [
    "ExtGState",
    "ColorSpace",
    "Pattern",
    "Shading",
    "XObject",
    "Font",
    "Properties",
];

/// The names one owner's content streams state as operands.
type Names = BTreeSet<Vec<u8>>;

/// ISO 19005 section 6.2.2's last sentence: the objects no content stream's names reach.
///
/// # The sentence
///
/// ISO 19005-2 section 6.2.2 and ISO 19005-4 section 6.2.2 both close with the same exemption: a
/// named resource present in a resources dictionary whose name the associated content stream
/// never references is not used for rendering, and is therefore exempt from the part's
/// requirements — wholly in part 2 as published, and in part 4 with its four object-syntax
/// subclauses carved back out. `TechNote 0010` A010 narrows part 2's to a comparable shape.
/// [`crate::clarification`] carries both readings and [`exemption_narrows`] is where the carve-out
/// is applied; this type computes only the population, which is the half no predicate can.
///
/// # What "associated" means, and what it costs to get wrong
///
/// ISO 32000-2 §7.8.3 and `TechNote 0010` A003 draw the association the same way: a resources
/// dictionary belongs to the stream that states it — a form `XObject`, a tiling pattern, an
/// annotation appearance — to the page whose `/Contents` it governs, or to the glyph procedures
/// of the Type 3 font that states it, and **never** to a page that inherits it, which is A003's
/// own sentence. A resources dictionary with no such owner exempts nothing here: an `/AcroForm`
/// `/DR` has no associated content stream, so the clause's premise fails — and neither does one
/// whose owner states no content at all, for the same reason and not a weaker one. [`owners`]
/// carries both cases and what each cost when it was got wrong.
///
/// # Two approximations, and both run the same way
///
/// **Every name operand of a content stream counts as a reference**, with no operator table, so
/// `/F1` reaching a `Tf` and `/F1` reaching nothing are the same token. And a resources
/// dictionary two owners share carries the union of their names. Both can only make the exempt
/// set *smaller*, so this type under-exempts rather than over-exempts — the direction
/// `doc/todo/62` section 3 argues every unit of risk in this work lies against.
#[derive(Debug, Clone)]
pub struct Exempt {
    /// Every object reachable only through a named resource entry nothing references.
    objects: Arc<BTreeSet<ObjectId>>,
    /// How many such entries the file states.
    entries: usize,
    /// The bound that stopped one of the two walks, where one did.
    stopped: Option<Limit>,
}

impl Exempt {
    /// Computes the exempt population: the objects the full walk reaches and the cut walk does not.
    #[must_use]
    pub fn of(exam: &Examination<'_>, reach: &Reach) -> Self {
        let cuts = Cuts::of(exam.document, exam.objects());
        let kept = Walk::run(exam, &cuts);
        let stopped = reach.stopped_at().or(kept.stopped);
        let objects = if stopped.is_some() {
            BTreeSet::new()
        } else {
            reach
                .arrivals
                .keys()
                .filter(|id| !kept.arrivals.contains_key(id))
                .copied()
                .collect()
        };
        Self {
            objects: Arc::new(objects),
            entries: kept.entries,
            stopped,
        }
    }

    /// Every object this part's exemption reaches.
    #[must_use]
    pub fn objects(&self) -> &BTreeSet<ObjectId> {
        &self.objects
    }

    /// The same set, shareable with a [`crate::Findings`] that is to drop what it exempts.
    #[must_use]
    pub fn shared(&self) -> Arc<BTreeSet<ObjectId>> {
        Arc::clone(&self.objects)
    }

    /// Whether the exemption reaches this object.
    #[must_use]
    pub fn holds(&self, id: ObjectId) -> bool {
        self.objects.contains(&id)
    }

    /// Whether the exemption reaches nothing in this document.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// How many named resource entries the associated content stream does not reference.
    ///
    /// Not the same number as [`Self::objects`]' length, and both are worth printing: an entry
    /// whose value is a direct object, or an indirect one something else also reaches, states
    /// the exemption's premise without exempting anything.
    ///
    /// Counted by the walk that cuts, so it counts the entries of every resources dictionary the
    /// file **reaches** — and not those inside a subtree the cut already removed, which no
    /// conforming reader reaches either.
    #[must_use]
    pub const fn entries(&self) -> usize {
        self.entries
    }

    /// The bound that stopped one of the two walks, where one did.
    #[must_use]
    pub const fn stopped_at(&self) -> Option<Limit> {
        self.stopped
    }
}

/// Where in a document's object graph one value sits, for the purpose of the exemption.
#[derive(Debug, Clone, Copy)]
enum Spot<'a> {
    /// Anywhere else — every edge from here is an ordinary reference.
    Elsewhere,
    /// A `/Resources` dictionary, with what its associated content streams reference.
    Resources(&'a Names),
    /// One category of such a dictionary, so its keys are the resource names.
    Category(&'a Names),
}

/// Which edges of the object graph the exemption cuts.
///
/// Empty for [`Reach`], which follows every edge; filled for [`Exempt`]'s second walk, whose
/// reached set is what survives *without* the unreferenced named resource entries.
#[derive(Debug, Default)]
struct Cuts {
    /// The objects each resources dictionary's owners reference by name, by object number.
    resources: BTreeMap<ObjectId, Names>,
    /// The same for a category dictionary that is an indirect object of its own.
    categories: BTreeMap<ObjectId, Names>,
    /// Every object A003 makes an owner of a resources dictionary, with what its streams name.
    owners: BTreeMap<ObjectId, Names>,
    /// Whether this walk cuts anything at all.
    cutting: bool,
}

impl Cuts {
    /// The walk that follows every edge.
    fn none() -> Self {
        Self::default()
    }

    /// The walk that cuts a named resource entry the associated content stream never references.
    fn of(document: &Document, objects: &[(ObjectId, Object)]) -> Self {
        let owners = owners(document, objects);
        let (resources, categories) = indirect_dictionaries(objects, &owners);
        Self {
            resources,
            categories,
            owners,
            cutting: true,
        }
    }
}

/// Where an object sits, given what the two passes found.
///
/// A free function rather than a method so that the [`Spot`] it returns borrows the cuts and not
/// the walker holding them, which is what lets the traversal take `&mut self` in the same breath.
fn spot_of(cuts: &Cuts, id: ObjectId) -> Spot<'_> {
    if let Some(names) = cuts.categories.get(&id) {
        Spot::Category(names)
    } else if let Some(names) = cuts.resources.get(&id) {
        Spot::Resources(names)
    } else {
        Spot::Elsewhere
    }
}

/// One breadth-first walk of the object graph, with or without the exemption's cuts.
struct Walk {
    arrivals: BTreeMap<ObjectId, Arrival>,
    entries: usize,
    stopped: Option<Limit>,
}

impl Walk {
    /// Runs it, from the roots the module comment names.
    fn run<'x>(exam: &'x Examination<'_>, cuts: &'x Cuts) -> Self {
        let mut state = Walker {
            document: exam.document,
            cuts,
            entries: 0,
            arrivals: BTreeMap::new(),
            queue: VecDeque::new(),
            edges: 0,
            stopped: None,
        };
        state.seed(exam);
        state.drain(exam);
        Self {
            arrivals: state.arrivals,
            entries: state.entries,
            stopped: state.stopped,
        }
    }
}

/// The walk's own state, apart so that the traversal can borrow one thing.
struct Walker<'a> {
    document: &'a Document,
    cuts: &'a Cuts,
    /// Named resource entries nothing references, counted as the walk meets them.
    entries: usize,
    arrivals: BTreeMap<ObjectId, Arrival>,
    queue: VecDeque<ObjectId>,
    edges: usize,
    stopped: Option<Limit>,
}

impl<'a> Walker<'a> {
    /// Enters the roots: the trailer's references, then the two structural populations.
    ///
    /// The trailer is walked as a value rather than key by key, so an `/Encrypt` or an `/Info`
    /// reached through a direct array the trailer states is a root as surely as `/Root` is.
    fn seed(&mut self, exam: &Examination<'_>) {
        let trailer = Object::Dictionary(self.document.trailer().clone());
        self.follow(None, &trailer, Spot::Elsewhere, None);
        for (id, object) in exam.objects() {
            if let Some(Location::InStream { stream, .. }) =
                self.document.xref().location(id.number)
            {
                self.enter(
                    ObjectId::new(stream, 0),
                    None,
                    vec![Entry::Structure("§7.5.7's object stream")],
                );
            }
            let Object::Stream(stream) = object else {
                continue;
            };
            // §7.5.8.2's `/Type` is required and is the only mark a cross-reference stream
            // carries; it is a direct name in every file that has one, and resolving a
            // reference here would be asking the object graph about an object the object
            // graph does not reach.
            if stream
                .dict
                .get("Type")
                .and_then(Object::as_name)
                .is_some_and(|kind| kind.0.as_ref() == b"XRef")
            {
                self.enter(
                    *id,
                    None,
                    vec![Entry::Structure("§7.5.8's cross-reference stream")],
                );
            }
        }
    }

    /// Walks until the queue is empty or a bound stops it.
    fn drain<'v>(&mut self, exam: &'v Examination<'_>)
    where
        'a: 'v,
    {
        let objects = exam.objects();
        while let Some(id) = self.queue.pop_front() {
            if self.stopped.is_some() {
                return;
            }
            let Some(object) = object_at(objects, id) else {
                continue;
            };
            let spot = spot_of(self.cuts, id);
            let owner = self.cuts.owners.get(&id);
            self.follow(Some(id), object, spot, owner);
        }
    }

    /// Records one object's outgoing references, honouring the cuts.
    fn follow<'v>(
        &mut self,
        from: Option<ObjectId>,
        object: &'v Object,
        spot: Spot<'v>,
        owner: Option<&'v Names>,
    ) where
        'a: 'v,
    {
        // An explicit stack rather than a recursion: how deeply a direct value nests inside one
        // object is the document's to choose, and `DEPTH` is what this walk will follow.
        //
        // `trail` is how an arrival gets the *whole* run of entries inside this object without a
        // `Vec` per step: every entry is appended once and remembers the one it hangs off, so a
        // stack item carries an index into it and a reference reconstructs its run by walking
        // back. A page reached through `/Kids` and then position 0 costs two slots and no
        // allocation until the arrival is recorded.
        let mut trail: Vec<(Entry, Option<usize>)> = Vec::new();
        let mut stack: Vec<(&'v Object, Spot<'v>, Option<usize>, usize)> =
            vec![(object, spot, None, 0)];
        while let Some((value, here, through, depth)) = stack.pop() {
            if depth > DEPTH {
                self.stopped = Some(Limit::Depth(DEPTH));
                return;
            }
            match value {
                Object::Reference(id) => {
                    self.edges = self.edges.saturating_add(1);
                    if self.edges > EDGES {
                        self.stopped = Some(Limit::Edges(EDGES));
                        return;
                    }
                    let run = run_of(&trail, through);
                    self.enter(*id, from, run);
                }
                Object::Array(items) => {
                    for (at, item) in items.iter().enumerate() {
                        trail.push((Entry::Index(at), through));
                        stack.push((
                            item,
                            Spot::Elsewhere,
                            Some(trail.len().saturating_sub(1)),
                            depth.saturating_add(1),
                        ));
                    }
                }
                Object::Dictionary(dict) => {
                    self.push_keys(dict, here, owner, depth, through, &mut trail, &mut stack);
                }
                Object::Stream(stream) => {
                    self.push_keys(
                        &stream.dict,
                        here,
                        owner,
                        depth,
                        through,
                        &mut trail,
                        &mut stack,
                    );
                }
                _ => {}
            }
        }
    }

    /// One dictionary's entries, with the cut applied to a category's unreferenced names.
    #[expect(
        clippy::too_many_arguments,
        reason = "one traversal step's whole context: where it is, what it may cut, how deep it \
                  is, and the two buffers the loop above owns"
    )]
    fn push_keys<'v>(
        &mut self,
        dict: &'v Dictionary,
        spot: Spot<'v>,
        owner: Option<&'v Names>,
        depth: usize,
        through: Option<usize>,
        trail: &mut Vec<(Entry, Option<usize>)>,
        stack: &mut Vec<(&'v Object, Spot<'v>, Option<usize>, usize)>,
    ) {
        for (key, value) in dict.iter() {
            if let Spot::Category(names) = spot
                && !names.contains(key.0.as_ref())
            {
                self.entries = self.entries.saturating_add(1);
                if self.cuts.cutting {
                    continue;
                }
            }
            trail.push((Entry::Key(key.clone()), through));
            stack.push((
                value,
                inner_spot(spot, owner, key.0.as_ref()),
                Some(trail.len().saturating_sub(1)),
                depth.saturating_add(1),
            ));
        }
    }

    /// Records an arrival at an object the walk has not seen before.
    fn enter(&mut self, id: ObjectId, from: Option<ObjectId>, through: Vec<Entry>) {
        let id = numbered(id);
        if self.arrivals.contains_key(&id) {
            return;
        }
        self.arrivals.insert(id, Arrival { from, through });
        self.queue.push_back(id);
    }
}

/// The run of entries a stack item hangs off, outermost first.
fn run_of(trail: &[(Entry, Option<usize>)], at: Option<usize>) -> Vec<Entry> {
    let mut run = Vec::new();
    let mut here = at;
    // The links only ever point backwards into the arena, so the walk terminates; the bound is
    // the arena's length and is stated rather than assumed.
    for _ in 0..=trail.len() {
        let Some(index) = here else {
            run.reverse();
            return run;
        };
        let Some((entry, previous)) = trail.get(index) else {
            run.reverse();
            return run;
        };
        run.push(entry.clone());
        here = *previous;
    }
    run.reverse();
    run
}

/// One object identity as this reader keys them: by number, with the generation dropped.
///
/// **Not a reading of §7.3.10**, which pairs a reference's object number *and* generation with
/// the object it names. It is agreement with the program: `pdf_syntax::Document::get` caches and
/// resolves on the object number alone, `XrefTable` maps numbers to locations, and
/// `Examination::objects` names every object at generation zero for that reason. A walk that kept
/// the generation would report an object as unreferenced because the only reference to it spells
/// a generation the reader never compares — a finding about this crate's own bookkeeping rather
/// than about the document. It changes no count over `doc/veraPDF-corpus`, where no reference
/// spells a generation other than zero; it is here for the file that does.
fn numbered(id: ObjectId) -> ObjectId {
    ObjectId::new(id.number, 0)
}

/// Where one entry's value sits, given where its dictionary sits and what the key is.
fn inner_spot<'a>(spot: Spot<'a>, owner: Option<&'a Names>, key: &[u8]) -> Spot<'a> {
    match spot {
        Spot::Elsewhere if key == b"Resources" => owner.map_or(Spot::Elsewhere, Spot::Resources),
        Spot::Resources(names) if CATEGORIES.iter().any(|name| name.as_bytes() == key) => {
            Spot::Category(names)
        }
        // A resources dictionary's other keys, and everything below a category's entries, are
        // ordinary objects again.
        _ => Spot::Elsewhere,
    }
}

/// Every object A003 makes an owner of a resources dictionary, with what its streams reference.
///
/// The three owners A003 recognises, and no fourth: a **stream** — a form `XObject`, a tiling
/// pattern, an annotation appearance — is associated with its own data; a **page** with the
/// streams its `/Contents` names; a **Type 3 font** with its glyph procedures. Anything else that
/// carries a `/Resources` key is absent here and exempts nothing, which is the clause's own
/// premise: the exemption is about a resource *an associated content stream* does not reference,
/// and where there is no such stream the premise fails.
///
/// **The test is what the object is *and* whether it has content, not whether it happens to hold
/// a `/Resources` key.** Two cases make the difference, and both were over-exempting when this
/// walk was lifted out of `examples/unreferenced.rs`:
///
/// - A `/Type /Pages` node holding the `/Resources` its descendants inherit. A003 says
///   inheritance is not association, and an owner list that took the node would hand it an
///   *empty* set of referenced names — exempting every resource in a dictionary a hundred pages
///   draw with.
/// - A page, or a Type 3 font, with a `/Resources` entry and **no content at all**. There is no
///   associated content stream, so there is no fact about what it references, so the clause's
///   premise fails exactly as it does for an `/AcroForm` `/DR`. Reading the absent stream as one
///   that references nothing is an *inference* — the resource draws nothing, so nothing uses it —
///   and a different sentence from the one the standard states. Where the reading is open the
///   direction that withdraws no failure is the one this crate takes, and `doc/todo/62` section 3
///   says why.
fn owners(document: &Document, objects: &[(ObjectId, Object)]) -> BTreeMap<ObjectId, Names> {
    let mut found: BTreeMap<ObjectId, Names> = BTreeMap::new();
    for (id, object) in objects {
        let dict = match object {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            _ => continue,
        };
        if matches!(document.get_key(dict, "Resources"), Object::Null) {
            continue;
        }
        let mut names = Names::new();
        let mut has_content = false;
        // A stream is associated with its own data: a form XObject, a tiling pattern, an
        // annotation appearance. There is always content, even where it is empty.
        if let Object::Stream(stream) = object {
            has_content = true;
            read_names(document, &Object::Stream(stream.clone()), &mut names);
        }
        // A page is associated with the streams its `/Contents` names.
        if document
            .get_key(dict, "Type")
            .as_name()
            .is_some_and(|kind| kind.0.as_ref() == b"Page")
        {
            let contents = document.get_key(dict, "Contents");
            has_content |= holds_a_stream(document, &contents);
            read_names(document, &contents, &mut names);
        }
        // A Type 3 font is associated with its glyph procedures.
        if let Object::Dictionary(procedures) = document.get_key(dict, "CharProcs") {
            for (_, value) in procedures.iter() {
                let procedure = document.resolve(value);
                has_content |= holds_a_stream(document, &procedure);
                read_names(document, &procedure, &mut names);
            }
        }
        if has_content {
            found.insert(*id, names);
        }
    }
    found
}

/// Whether a `/Contents` or glyph procedure value is, or holds, a stream.
///
/// §7.7.3.3 lets a page's `/Contents` be one stream or an array of them, "which, when
/// concatenated, shall behave as a single stream"; an empty array and an absent entry are both a
/// page with no content stream at all.
fn holds_a_stream(document: &Document, value: &Object) -> bool {
    match value {
        Object::Stream(_) => true,
        Object::Array(items) => items
            .iter()
            .any(|item| holds_a_stream(document, &document.resolve(item))),
        _ => false,
    }
}

/// Adds every name a content stream, or an array of them, states as an operand.
fn read_names(document: &Document, object: &Object, into: &mut Names) {
    match object {
        Object::Array(items) => {
            for item in items {
                read_names(document, &document.resolve(item), into);
            }
        }
        Object::Stream(stream) => {
            let Some(data) = document.decoded_stream_data(stream) else {
                return;
            };
            let mut lexer = Lexer::new(&data);
            while let Some(token) = lexer.next_token() {
                if let Token::Name(name) = token {
                    into.insert(name);
                }
            }
        }
        _ => {}
    }
}

/// The resources dictionaries and categories that are indirect objects of their own.
type Indirect = (BTreeMap<ObjectId, Names>, BTreeMap<ObjectId, Names>);

/// Finds them, with the names of every owner that reaches each.
///
/// **Two passes, and two is the whole answer rather than an approximation of it.**
/// [`inner_spot`] admits exactly two steps: a `/Resources` key under an owner makes a resources
/// dictionary, and a category key under one of those makes a category — everything below a
/// category's entries is an ordinary object again. So an owner's own value yields every resources
/// dictionary it names indirectly (and every category of one it states directly), and a second
/// pass over those yields the categories that were invisible until their dictionary was known.
/// Nothing a third pass could find exists.
///
/// This replaced a fixpoint that re-read every object of the file on every round; on
/// ISO 32000-2's own specification that was 110 000 fetches a round and most of what a report
/// asking for the exemption cost. A dictionary two owners share carries the union of their names,
/// which can only shrink the exempt set — the direction [`Exempt`] states.
fn indirect_dictionaries(
    objects: &[(ObjectId, Object)],
    owners: &BTreeMap<ObjectId, Names>,
) -> Indirect {
    let mut resources: BTreeMap<ObjectId, Names> = BTreeMap::new();
    let mut categories: BTreeMap<ObjectId, Names> = BTreeMap::new();
    for (id, object) in objects {
        let Some(names) = owners.get(id) else {
            continue;
        };
        note(
            object,
            Spot::Elsewhere,
            Some(names),
            &mut resources,
            &mut categories,
        );
    }
    let found: Vec<(ObjectId, Names)> = resources
        .iter()
        .map(|(id, names)| (*id, names.clone()))
        .collect();
    for (id, names) in found {
        if let Some(object) = object_at(objects, id) {
            note(
                object,
                Spot::Resources(&names),
                None,
                &mut resources,
                &mut categories,
            );
        }
    }
    (resources, categories)
}

/// One object of the shared population, found by number.
///
/// `Examination::objects` is built from the cross-reference table's ascending object numbers, so
/// it is sorted and a search costs a logarithm rather than a map of its own over a hundred
/// thousand entries.
fn object_at(objects: &[(ObjectId, Object)], id: ObjectId) -> Option<&Object> {
    let at = objects
        .binary_search_by_key(&id.number, |(had, _)| had.number)
        .ok()?;
    objects.get(at).map(|(_, object)| object)
}

/// Records every indirect resources dictionary and category this value names.
fn note(
    object: &Object,
    spot: Spot<'_>,
    owner: Option<&Names>,
    resources: &mut BTreeMap<ObjectId, Names>,
    categories: &mut BTreeMap<ObjectId, Names>,
) {
    match object {
        Object::Array(items) => {
            for item in items {
                note(item, Spot::Elsewhere, owner, resources, categories);
            }
        }
        Object::Dictionary(dict) => note_keys(dict, spot, owner, resources, categories),
        Object::Stream(stream) => note_keys(&stream.dict, spot, owner, resources, categories),
        _ => {}
    }
}

/// The same, for one dictionary's entries.
fn note_keys(
    dict: &Dictionary,
    spot: Spot<'_>,
    owner: Option<&Names>,
    resources: &mut BTreeMap<ObjectId, Names>,
    categories: &mut BTreeMap<ObjectId, Names>,
) {
    for (key, value) in dict.iter() {
        let inner = inner_spot(spot, owner, key.0.as_ref());
        match (inner, value) {
            (Spot::Resources(names), Object::Reference(id)) => {
                resources
                    .entry(numbered(*id))
                    .or_default()
                    .extend(names.iter().cloned());
            }
            (Spot::Category(names), Object::Reference(id)) => {
                categories
                    .entry(numbered(*id))
                    .or_default()
                    .extend(names.iter().cloned());
            }
            _ => note(value, inner, owner, resources, categories),
        }
    }
}

/// Whether section 6.2.2's exemption narrows this requirement's population, for this target.
///
/// # The two carve-outs, and why they are read by clause rather than by row
///
/// Both parts exempt an unreferenced named resource from the part's requirements and then take
/// some of them back, and the two take back different sets:
///
/// - **ISO 19005-4 section 6.2.2** keeps its own four object-syntax subclauses, sections 6.1.6 to
///   6.1.9, in its published text.
/// - **ISO 19005-2 section 6.2.2** as published keeps nothing at all, and `TechNote 0010` A010
///   narrows it to the part's file-structure and implementation-limit subclauses, sections 6.1.2
///   to 6.1.13. `doc/todo/62` section 3 is why the two land together or not at all: part 2's
///   published sentence *without* A010 would withdraw failures this crate and the corpus agree
///   on, so A010 is what makes the exemption safe rather than a refinement to apply after it.
///
/// **Where the per-subclause reading of this function lives.** [`crate::withdrawal`] records, for
/// every subclause a row cites, whether the exemption can reach it at all — the carve-out keeps
/// it, a named resource can be its subject, nothing a resources dictionary names can be, or no
/// document can state a finding there — and its tests hold that reading against this function.
/// The single `else` arm below is right for the clause and says nothing about which of those four
/// a subclause is.
///
/// A carve-out stated as a range of *clauses* is honoured where a clause is known, which is here
/// and not inside a predicate: a row identifier says which rule a requirement is, and the
/// sentence being read says which part of the part it is in. `doc/todo/62` section 4's second
/// point asks for exactly this placement.
///
/// **Section 5.1 is kept for both parts**, and for two different reasons. For part 2 it is A010's
/// second sentence: an unreferenced resource is exempt from the part and shall still conform to
/// the base standard, which is what section 5.1 requires. For part 4 no clarification says so and
/// the published sentence reads wider; keeping it is the direction that withdraws nothing, and
/// nothing turns on it today because `conformance/adheres-to-the-base-standard` is
/// [`crate::Check::Unchecked`] in both parts — the residue `doc/todo/62` section 4's third point
/// names, and the one part of this work that could ever *add* a failure.
#[must_use]
pub fn exemption_narrows(clauses: Clauses, target: Target) -> bool {
    let (clause, kept) = match target.part() {
        Part::Two => (clauses.two, 2..=13),
        Part::Four => (clauses.four, 6..=9),
    };
    let Some(parts) = clause.map(components) else {
        // A requirement the target's part does not state is not judged at all; saying "not
        // narrowed" here keeps this function total rather than making its caller ask twice.
        return false;
    };
    // Section 5.1's base standard is kept for both parts, and so is a row citing a whole clause
    // — `6`, or `6.1` — because that covers the subclauses inside it, the carve-out's among
    // them: the exemption may not withdraw a finding it cannot place.
    let kept_whole = matches!(parts.as_slice(), [5, 1] | [6]) || parts.as_slice() == [6, 1];
    if kept_whole {
        return false;
    }
    if let [6, 1, subclause, ..] = parts.as_slice() {
        !kept.contains(subclause)
    } else {
        true
    }
}

/// A clause number's components — `"6.1.7.2"` as `[6, 1, 7, 2]` — and an empty reading for a
/// number that is not digits and dots.
///
/// **An annex is such a number, and rows of [`crate::table`] cite one** — three of them with a
/// predicate, at ISO 19005-2 Annex B.1, ISO 19005-4 Annex A.2 and ISO 19005-4 Annex B.2.2, and
/// the rest binding a processor. An empty reading falls outside
/// both carve-outs, so [`exemption_narrows`] answers *yes* for them — the exemption reaches an
/// annex — and that is what the two sentences say rather than an accident of the parser: each
/// exempts the resource from every requirement of the **document**, and a normative annex is part
/// of the document. [`crate::withdrawal`] records the answer for each of those three subclauses
/// beside the reason, which is where a reader should find it.
///
/// This comment used to claim the opposite — that a clause the table did not write as digits
/// would be *kept*, and that every clause in the table was digits and dots. Both halves were
/// wrong, and the second was checkable: session 1007 found the annex rows above.
fn components(clause: &str) -> Vec<u32> {
    let mut out = Vec::new();
    for piece in clause.split('.') {
        let Ok(number) = piece.parse::<u32>() else {
            return Vec::new();
        };
        out.push(number);
    }
    out
}
