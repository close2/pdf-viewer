//! A structure element's type, and the namespace that type is read in.
//!
//! ISO 32000-2 §14.7.3 maps a document's own type names to the standard's through the role map,
//! and §14.8.6.2 says which role map applies to which namespace; both questions end in the
//! same walk, so the element's role, its standard role, its namespace, and the counts of
//! foreign and `MathML` namespaces (§14.8.6.3) sit together.

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use super::{
    Child, DEFAULT_STANDARD_NAMESPACE, ForeignNamespace, MATHML_MATH_TYPE, MATHML_NAMESPACE,
    MAX_DEPTH, MAX_ELEMENTS, MappedRole, Namespace, Nonstandard, RoleMapEnd,
    STANDARD_NAMESPACE_2_0, StandardType, Tree, UnenclosedMathml, permitted_namespace, standard_in,
};

impl Tree {
    /// The element's structure type, mapped through §14.7.3's `/RoleMap` where one applies.
    ///
    /// > Where names other than the standard ones are used, a role map should be provided in
    /// > the structure tree root using the RoleMap entry
    ///
    /// so a document's own `/Sect2` becomes the standard `/Sect` it maps to. The map is
    /// followed transitively — a role map may name a type that is itself mapped — and stops at
    /// the first type it returns to, which is §14.7.3's NOTE 2; [`Self::mapped_role`] says which
    /// name a circular chain answers with and that it was one.
    ///
    /// **Which map applies is §14.8.6.2's question, not §14.7.3's.** An element that names its
    /// namespace is mapped by *that namespace's* `/RoleMapNS`: "if the structure element does
    /// not explicitly identify its namespace using an `NS` entry, it should use the `RoleMap`
    /// entry in the Structure Tree Root dictionary … If the structure element is in an
    /// explicit namespace, then … the `RoleMapNS` entry within that namespace dictionary shall
    /// provide the role mapping". A `/RoleMapNS` value may also name a *target namespace*
    /// alongside the type, which is how one document's vocabulary maps into another's, and
    /// this follows that too — the loop's state is a name **and** a namespace.
    ///
    /// `None` for an element with no `/S`, which Table 355 makes required: a structure
    /// element that states no type says nothing about what it is, and inventing one would be
    /// the fallback-that-fills-the-page in another clause's clothing.
    #[must_use]
    pub fn role(&self, document: &Document, element: &Dictionary) -> Option<String> {
        let (name, _) = self.resolved(document, element)?;
        Some(name)
    }

    /// The type an element ends at, **and the namespace it ends in**.
    ///
    /// [`Self::role`] answers the first half and [`Self::standard_role`] needs both: §14.8.6.2
    /// states its rules against the *pair*, so the name alone cannot say whether `Table` means
    /// §14.8.4's table or a foreign vocabulary's homonym.
    fn resolved(
        &self,
        document: &Document,
        element: &Dictionary,
    ) -> Option<(String, Option<Dictionary>)> {
        let end = self.walk_role_map(document, element)?;
        Some((end.name, end.namespace))
    }

    /// §14.7.3's and §14.8.6.2's walk through the role maps, and whether it came back on itself.
    ///
    /// §14.7.3 permits the loop it then has to stop:
    ///
    /// > NOTE 2 The same structure type can occur as both a key and a value in the role map, and
    /// > circular chains of association are explicitly permitted. Therefore, a single role map can
    /// > define a bidirectional mapping. A PDF processor using the role map needs to follow the
    /// > chain of associations until it either finds a structure type it recognises or returns to
    /// > one it has already encountered.
    ///
    /// **The normative sentence decides an acyclic chain and the NOTE a circular one.** "A
    /// structure type shall always be mapped to its corresponding name in the role map, if there
    /// is one, even if the original name is one of the standard types", so a chain that passes
    /// through a standard name the map maps onward is followed past it to the name the map does not
    /// map. A circular chain has no such name, so the `shall` cannot be met by any of its members
    /// and the NOTE's two stops are what is left: the walk ends at the first pair it returns to,
    /// and answers with the first type along the chain — the element's own name excepted, which
    /// the `shall` maps at least once — that a standard structure namespace defines. The NOTE's
    /// own example is why the order is that one: a bidirectional map `MyPara` ↔ `P` reads as `P`
    /// from either end. Where no member of the circle is a standard type the answer is the last
    /// new name the walk reached, which is the element's own name mapped at least once.
    ///
    /// [`MAX_DEPTH`] still bounds a chain of distinct names, which no NOTE permits and which only
    /// a file built to exhaust a reader states.
    fn walk_role_map(&self, document: &Document, element: &Dictionary) -> Option<RoleMapEnd> {
        let first = document.get_key(element, "S").as_name()?.clone();
        // The element's own namespace, if it states one. `None` means §14.8.6.1's default
        // standard structure namespace, which is where the root's `/RoleMap` applies.
        let mut chain: Vec<(Name, Option<Dictionary>)> =
            vec![(first, document.get_key(element, "NS").as_dict().cloned())];
        let mut returned_to = None;
        for _ in 0..MAX_DEPTH {
            let Some((name, namespace)) = chain.last() else {
                break;
            };
            // §14.8.6.2 decides which of the two maps is consulted, and nothing else does:
            // an element in an explicit namespace is mapped by that namespace's own.
            let map = match namespace {
                Some(space) => document.get_key(space, "RoleMapNS"),
                None => document.get_key(&self.root, "RoleMap"),
            };
            // The key is the element's own `/S`, which is a name the document states, so it
            // is probed as bytes — §7.3.5's exact binary match (ADR 0439).
            let mapped = map
                .as_dict()
                .map_or(Object::Null, |map| document.get_key_by_name(map, name));
            let next = match mapped {
                // "a single name identifying a structure element type in the default standard
                // structure namespace" — so the mapping leaves whatever namespace it was in.
                Object::Name(next) => (next, None),
                // "an array where the first value shall be a structure element type name in a
                // target namespace with the second value being an indirect reference to the
                // target namespace dictionary".
                Object::Array(items) => {
                    let Some(next) = items
                        .first()
                        .and_then(|item| document.resolve(item).as_name().cloned())
                    else {
                        break;
                    };
                    let target = items
                        .get(1)
                        .map(|item| document.resolve(item))
                        .and_then(|object| object.as_dict().cloned());
                    (next, target)
                }
                _ => break,
            };
            if chain.contains(&next) {
                returned_to = Some(next);
                break;
            }
            chain.push(next);
        }
        let Some(returned_to) = returned_to else {
            let (name, namespace) = chain.pop()?;
            return Some(RoleMapEnd {
                name: String::from_utf8_lossy(name.as_bytes()).into_owned(),
                namespace,
                circular: false,
            });
        };
        // The NOTE's first stop, read over the circle the walk has closed: the first type along
        // it that a standard structure namespace defines, the pair it returned to last.
        let recognised = chain
            .iter()
            .skip(1)
            .chain(std::iter::once(&returned_to))
            .find(|(name, namespace)| {
                standard_in(
                    document,
                    &String::from_utf8_lossy(name.as_bytes()),
                    namespace.as_ref(),
                )
                .is_some()
            })
            .cloned();
        let circular = recognised.is_none();
        let (name, namespace) = match recognised {
            Some(found) => found,
            None => chain.pop()?,
        };
        Some(RoleMapEnd {
            name: String::from_utf8_lossy(name.as_bytes()).into_owned(),
            namespace,
            circular,
        })
    }

    /// The element's type after its role map, with what §14.8.4's vocabulary makes of it — the
    /// standard type, or which of four reasons there is none.
    ///
    /// [`Self::role`] and [`Self::standard_role`] in one walk, and the one answer a consumer
    /// that hands the element on should carry: the name is what the document wrote or what its
    /// role map took it to, and the reading is what that name means in the namespace it ends in.
    /// A consumer reading the name again with [`StandardType::read`] would take a foreign
    /// namespace's `Table` for §14.8.4's (ADR 2028).
    ///
    /// A role map that comes back on itself is followed until it returns to a type it has met,
    /// which is where §14.7.3's NOTE 2 stops, and answers with the first standard type along the
    /// circle — the element's own name excepted, which the clause's `shall` maps at least once — or
    /// with [`Nonstandard::Circular`] where the circle holds none.
    #[must_use]
    pub fn mapped_role(&self, document: &Document, element: &Dictionary) -> MappedRole {
        let Some(end) = self.walk_role_map(document, element) else {
            return MappedRole {
                name: String::new(),
                standard: Err(Nonstandard::Unstated),
            };
        };
        let standard = match standard_in(document, &end.name, end.namespace.as_ref()) {
            Some(kind) => Ok(kind),
            None if end.circular => Err(Nonstandard::Circular),
            None => match &end.namespace {
                Some(space)
                    if Namespace::read(document, space)
                        .and_then(|space| space.standard())
                        .is_none() =>
                {
                    Err(Nonstandard::Foreign)
                }
                _ => Err(Nonstandard::Undefined),
            },
        };
        MappedRole {
            name: end.name,
            standard,
        }
    }

    /// The element's §14.8.4 standard type, after §14.7.3's and §14.8.6.2's role mapping.
    ///
    /// [`Self::role`] answers the *name*, which is what a document wrote or what its role map
    /// took it to; this answers what that name means in the standard's own vocabulary. `None`
    /// where the mapped name is not a standard type, which §14.8.4.1 makes a defect in the
    /// document rather than a gap here — "\[a]ll structure elements occurring within a tagged PDF
    /// document shall have a type matching one of those defined as a Standard Structure Type, or
    /// a role map providing a mapping from the non-standard type to a Standard Structure Type".
    ///
    /// **And `None` where the name ends in a namespace that is not a standard structure one**,
    /// which is the half of the question nothing here used to ask. §14.8.4's vocabulary is
    /// *defined by* the two namespaces §14.8.6.1 names, so a foreign namespace's `Table` is a
    /// homonym rather than a table — and §14.8.6.2 states which elements count as being in one:
    ///
    /// > An element shall be considered to be in one of these namespaces if:
    ///
    /// followed by three bullets, which are exactly this walk. An element that names a standard
    /// namespace through `/NS` is the first, and [`Namespace::is_standard`] answers it; an
    /// element that names none is the second, and is `Self::resolved`'s `None`; a role map
    /// "directly or transitively" into one is the third, and is the loop. A namespace dictionary
    /// stating no `/NS` of its own answers `None` too: Table 356 makes the entry required, so the
    /// alternative is to treat a document's broken namespace as the standard one.
    ///
    /// **The name is read in the vocabulary of the namespace it ends in**, because §14.8.6.1 makes
    /// each standard structure namespace a schema of its own and the default one PDF 1.7's: "\[w]hen
    /// a namespace is not explicitly specified for a given structure element or attribute, it shall
    /// be assumed to be within this default standard structure namespace". So a `Note`, a `TOCI` or
    /// a `Code` stating no `/NS` is a standard type, and so is one stating the PDF 1.7 namespace;
    /// an element stating PDF 2.0's is read in PDF 2.0's vocabulary alone, where a `Note` is a name
    /// nobody defined.
    ///
    /// **A PDF 2.0 type stating no namespace is read as PDF 2.0's, and that is a choice** (ADR
    /// 2014). Errata Collection 3 makes the type an element with no stated namespace ends at one the
    /// default namespace shall define (ADR 0253), so an `Em` or an `Artifact` with no `/NS` is a
    /// file breaking a `shall`, and what a reader does with it the standard does not say. The name
    /// is defined by exactly one standard structure namespace and the file stated no other, so it is
    /// read as that one's — which keeps an `Artifact` element's content out of a copy and an `H7`
    /// a heading. An *explicit* namespace is a statement and is held to: an `Em` stating the PDF
    /// 1.7 namespace is not a type.
    #[must_use]
    pub fn standard_role(&self, document: &Document, element: &Dictionary) -> Option<StandardType> {
        self.mapped_role(document, element).standard.ok()
    }

    /// The namespace name an element is in, §14.8.6.1's default where it states none.
    ///
    /// The clause's 2020 sentence — which `doc/md/` still carries, and which the conformance gate
    /// therefore still verifies — read
    ///
    /// > When a namespace is not explicitly specified for a given structure element or
    /// > attribute, it shall be assumed to be within this default standard structure namespace.
    ///
    /// **Errata Collection 3 replaced it** (Issue #151, `/State` `Review` `Completed`; ADR 0253)
    /// with one that says where in the process the assumption is made: an element with no stated
    /// namespace is placed in the default standard structure namespace *after* any role map has
    /// been applied transitively, and the type it ends at shall be one the default namespace
    /// defines. [`Self::role`] is that walk and has been since it was written — it follows
    /// `/RoleMap` name by name to a fixed point — so the erratum names the order this code
    /// already has rather than changing it.
    ///
    /// The namespace is [`DEFAULT_STANDARD_NAMESPACE`]. An element that states a `/NS` whose
    /// dictionary has no `/NS` string of its own has named a namespace this reader cannot identify,
    /// and answers `None` rather than the default — Table 356 makes the entry required, so the
    /// alternative would be to report a document's broken namespace as the standard one.
    #[must_use]
    pub fn namespace(&self, document: &Document, element: &Dictionary) -> Option<String> {
        let stated = document.get_key(element, "NS");
        let Some(space) = stated.as_dict() else {
            return Some(DEFAULT_STANDARD_NAMESPACE.to_owned());
        };
        Namespace::read(document, space).map(|space| space.name)
    }

    /// Whether the element ends, after §14.8.6.2's role mapping, in the standard structure
    /// namespace for PDF 2.0.
    ///
    /// Annex L's population, which is why a writer asks it: "\[e]lements in the standard structure
    /// namespace for PDF 2.0 shall not have child or parent elements in the standard structure
    /// namespace for PDF 2.0 that are not explicitly listed in Table L.2", and "\[t]he containment
    /// rules specified in Table L.2 shall also apply to structure elements that are role mapped
    /// into the standard structure namespace for PDF 2.0". So the namespace asked about is the one
    /// the role map *ends* in, which is `Self::resolved`'s second half — not the `/NS` the
    /// element states, and never the default an element stating none is placed in.
    #[must_use]
    pub fn in_pdf_2_0_namespace(&self, document: &Document, element: &Dictionary) -> bool {
        self.resolved(document, element)
            .and_then(|(_, space)| space)
            .and_then(|space| Namespace::read(document, &space))
            .is_some_and(|space| space.name == STANDARD_NAMESPACE_2_0)
    }

    /// §14.8.6.2's requirement on the *document*: which namespaces its elements end in that the
    /// clause does not permit.
    ///
    /// > In a tagged PDF, all structure elements shall be in at least one of the standard
    /// > structure namespaces or in a namespace identified in 14.8.6.3 , ' Other namespaces '.
    ///
    /// This is the one sentence of §14.8.6 addressed to whoever *wrote* the file rather than to
    /// whoever reads it, and it is the reason both its subclauses stood `partial`: every other
    /// requirement is executed by [`Self::role`], [`Self::standard_role`] and [`Self::namespace`],
    /// and nothing said a word about a document that breaks this one. Each entry here is one
    /// namespace and how many elements ended in it, so a caller can say which vocabulary the
    /// document used rather than only that it used one.
    ///
    /// `None` for a name where the element's `/NS` resolves to a dictionary stating no `/NS` of
    /// its own — Table 356 makes that entry required, so the namespace identifies nothing and
    /// the element is in no namespace this reader can name. That is a violation of the same
    /// sentence and is counted apart from a namespace that *is* named, because the two are
    /// different things to tell a person.
    ///
    /// # Why the root's `/Namespaces` array decides whether the elements are read at all
    ///
    /// The elements have to be walked to answer the clause, and the walk is **151 ms** on the
    /// largest tagged document this tree holds (`MAX_ELEMENTS`'s own note has the measurement),
    /// which is more than the whole launch path is allowed. The clause is what makes it
    /// unnecessary: "\[i]f the structure element is in an explicit namespace, then that namespace
    /// shall be identified in the structure tree root dictionary's Namespaces array entry", so
    /// the array is the set of namespaces a conforming file's elements can name, and a root
    /// declaring none outside §14.8.6.1's two and §14.8.6.3's one has no element to find. That
    /// test is a handful of dictionary lookups and it is what every tagged document pays.
    ///
    /// **The limit that leaves, stated rather than hidden**: an element whose `/NS` names a
    /// namespace the root does not list is not seen here. Such a file breaks the sentence above
    /// as well as the one this answers, and finding it would cost the walk on every tagged
    /// document — so the cheap gate is a reading of the clause rather than an optimisation of it,
    /// and the cost of closing it is a measured number rather than a guess.
    #[must_use]
    pub fn namespaces_outside_the_standard(&self, document: &Document) -> Vec<ForeignNamespace> {
        if !self.declares_a_namespace_outside_the_standard(document) {
            return Vec::new();
        }
        let mut counts: BTreeMap<Option<String>, usize> = BTreeMap::new();
        let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
        let mut budget = MAX_ELEMENTS;
        self.count_foreign(document, None, 0, &mut counts, &mut seen, &mut budget);
        counts
            .into_iter()
            .map(|(name, elements)| ForeignNamespace { name, elements })
            .collect()
    }

    /// §14.8.6.3's requirement on the *document*: how many `MathML` formulae it left unenclosed.
    ///
    /// The subclause's 2020 sentence — which `doc/md/` still carries, and which the conformance
    /// gate therefore still verifies — is
    ///
    /// > When including mathematics structured as `MathML` 3.0, the math structure element type as
    /// > defined in `MathML` 3.0 shall be used, and shall have its namespace explicitly defined
    /// > (see 14.7.4.2, "Namespace dictionary").
    ///
    /// **Errata Collection 3 replaces it with two sentences** (Issues #72 and #719, `/State`
    /// `Review` `Completed`), and this reads the first: it requires the `math` structure element
    /// type, as defined in `MathML`, to be used to enclose the formula under the `Formula`
    /// structure element type. The version goes, because §2's normative reference to `MathML` 3.0
    /// is itself replaced by `MathML` Core; the enclosure is new; and the namespace requirement
    /// widens to all `MathML` types *and their attributes*, which is the second sentence and is
    /// read below rather than implemented.
    ///
    /// The `shall` is on whoever *includes* the mathematics — the sentence opens on the act of
    /// including — so `CLAUDE.md`'s closed authoring exclusion says this tree does not have to
    /// produce such a tagging. It says nothing at all about *reading* one, and a requirement a
    /// file breaks is answered by a report rather than left to a validator, which is where
    /// §7.3.7's row and §14.8.6.2's arrived (ADRs 0784 and 0785). The authoring exclusion does
    /// not reach this clause; ADR 0786 is where that is argued.
    ///
    /// # What the condition is, and what it deliberately is not
    ///
    /// The element counted is one whose type ends at `math` — all lowercase, which the subclause's
    /// NOTE 2 says is "to match the `MathML` 3.0 specification" — **in the `MathML` namespace**,
    /// with no `Formula` anywhere above it. Three readings are folded into that and each is the
    /// narrow one:
    ///
    /// - **The namespace is part of the type.** §14.8.6.2 is the clause that says a name means
    ///   what its namespace makes it mean, so a `math` that ends in some other vocabulary is not
    ///   the type the amended sentence names — which says *as defined in `MathML`* — and is not
    ///   this clause's subject.
    /// - **`under` is read as *anywhere under*.** The sentence does not say *immediately*, so an
    ///   element with a `Formula` ancestor satisfies it on either reading and only one with none
    ///   at all breaks it on both. Reporting the intersection is what keeps this off the condition
    ///   the clause does not state (`doc/traps/instruments-and-reports.md` trap 11).
    /// - **The second `shall` is not answered here**, and that is a reading rather than an
    ///   omission: it requires all `MathML` structure element types and their attributes to have
    ///   the `MathML` namespace explicitly defined, which quantifies over `MathML`'s own vocabulary
    ///   — and ISO 32000-2 states that vocabulary nowhere, because §2's normative reference to
    ///   `MathML` Core holds it. A condition over a list this standard does not print would be this
    ///   reader's invention wearing the clause's number.
    ///
    /// # The cheap gate is [`Self::namespaces_outside_the_standard`]'s, one namespace over
    ///
    /// The walk is 151 ms on the largest tagged document in reach and `viewer_core::notes` runs on
    /// the launch path, so the elements are read only where the root's `/Namespaces` array names
    /// the `MathML` namespace — §14.8.6.2: "\[i]f the structure element is in an explicit namespace,
    /// then that namespace shall be identified in the structure tree root dictionary's Namespaces
    /// array entry", and §14.8.6.3's second `shall` puts every `MathML` element in an explicit one.
    /// So a root that does not list `MathML` has no `math` element for this to find. **The same
    /// limit follows and is stated rather than hidden**: a file that also breaks the `/Namespaces`
    /// sentence is not seen here, and closing that costs the walk on every tagged document.
    #[must_use]
    pub fn mathml_outside_a_formula(&self, document: &Document) -> usize {
        if !self.declares_the_mathml_namespace(document) {
            return 0;
        }
        let mut walk = UnenclosedMathml {
            found: 0,
            seen: BTreeSet::new(),
            budget: MAX_ELEMENTS,
        };
        self.count_unenclosed_mathml(document, None, false, 0, &mut walk);
        walk.found
    }

    /// Whether Table 354's `/Namespaces` array names §14.8.6.3's `MathML` namespace.
    pub(super) fn declares_the_mathml_namespace(&self, document: &Document) -> bool {
        let declared = document.get_key(&self.root, "Namespaces");
        let Some(items) = declared.as_array() else {
            return false;
        };
        items.iter().any(|item| {
            document.resolve(item).as_dict().is_some_and(|dict| {
                Namespace::read(document, dict).is_some_and(|space| space.name == MATHML_NAMESPACE)
            })
        })
    }

    /// Counts the `MathML` `math` elements below `element` with no `Formula` above them.
    ///
    /// `enclosed` is the only state the descent carries, and it is monotone: once a `Formula` is
    /// entered every element beneath it satisfies the clause, so the subtree below one is walked
    /// for its own sake and never counted. Bounded exactly as [`Self::count_foreign`] is.
    fn count_unenclosed_mathml(
        &self,
        document: &Document,
        element: Option<&Dictionary>,
        enclosed: bool,
        depth: usize,
        walk: &mut UnenclosedMathml,
    ) {
        if depth >= MAX_DEPTH {
            return;
        }
        for (child, id) in self.identified_children(document, element) {
            if walk.budget == 0 {
                return;
            }
            walk.budget = walk.budget.saturating_sub(1);
            let Child::Element(dict) = child else {
                continue;
            };
            if !id.is_none_or(|id| walk.seen.insert(id)) {
                continue;
            }
            let Some((name, namespace)) = self.resolved(document, &dict) else {
                continue;
            };
            let is_math = name == MATHML_MATH_TYPE
                && namespace.as_ref().is_some_and(|space| {
                    Namespace::read(document, space)
                        .is_some_and(|space| space.name == MATHML_NAMESPACE)
                });
            if is_math && !enclosed {
                walk.found = walk.found.saturating_add(1);
            }
            // `Formula` is §14.8.4's, so it is read through `standard_role` — a foreign
            // vocabulary's homonym is not the type the clause names, which is §14.8.6.2's whole
            // subject and the reason that method exists.
            let below_a_formula =
                enclosed || self.standard_role(document, &dict) == Some(StandardType::Formula);
            self.count_unenclosed_mathml(
                document,
                Some(&dict),
                below_a_formula,
                depth.saturating_add(1),
                walk,
            );
        }
    }

    /// Whether Table 354's `/Namespaces` array names a namespace §14.8.6.2 does not permit.
    ///
    /// Only a dictionary counts: an array entry that resolves to something else names no
    /// namespace an element's `/NS` could resolve to either, so it is a defect in the array
    /// rather than evidence about an element.
    pub(super) fn declares_a_namespace_outside_the_standard(&self, document: &Document) -> bool {
        let declared = document.get_key(&self.root, "Namespaces");
        let Some(items) = declared.as_array() else {
            return false;
        };
        items.iter().any(|item| {
            document
                .resolve(item)
                .as_dict()
                .is_some_and(|dict| !permitted_namespace(document, dict))
        })
    }

    /// Counts, per namespace, the elements below `element` that end outside §14.8.6.2's set.
    ///
    /// The same descent [`Self::descend`] performs, and bounded the same way — [`MAX_DEPTH`] on
    /// the recursion, the object identity on a cycle, [`MAX_ELEMENTS`] on the whole — but it
    /// keeps a count rather than the tree, because a document reaching this has already been
    /// judged worth walking and nothing here needs the items.
    fn count_foreign(
        &self,
        document: &Document,
        element: Option<&Dictionary>,
        depth: usize,
        counts: &mut BTreeMap<Option<String>, usize>,
        seen: &mut BTreeSet<ObjectId>,
        budget: &mut usize,
    ) {
        if depth >= MAX_DEPTH {
            return;
        }
        for (child, id) in self.identified_children(document, element) {
            if *budget == 0 {
                return;
            }
            *budget = budget.saturating_sub(1);
            let Child::Element(dict) = child else {
                continue;
            };
            // The identity decides the count as well as the descent, which is where this parts
            // company with [`Self::descend`]: that one lists what the walk *reaches*, and this
            // counts *elements*, so an element that is its own descendant is one element and not
            // two. An element written inline in its parent's `/K` has no identity and cannot be
            // reached twice, which is the same reasoning the descent uses.
            if !id.is_none_or(|id| seen.insert(id)) {
                continue;
            }
            // The namespace the element *ends* in, which is §14.8.6.2's third bullet — "they are
            // role mapped into the namespace, either directly or transitively" — and is why this
            // asks `resolved` rather than reading `/NS` off the element.
            if let Some((_, Some(space))) = self.resolved(document, &dict)
                && !permitted_namespace(document, &space)
            {
                let name = Namespace::read(document, &space).map(|space| space.name);
                let seen_here = counts.entry(name).or_insert(0);
                *seen_here = seen_here.saturating_add(1);
            }
            self.count_foreign(
                document,
                Some(&dict),
                depth.saturating_add(1),
                counts,
                seen,
                budget,
            );
        }
    }
}
