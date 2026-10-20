//! Which element owns an object, a widget, an appearance or an identifier.
//!
//! ISO 32000-2 §14.7.5.4 gives the structural parent tree, §14.7.5.3 the objects that are
//! content items. These are the lookups that start from something on a page and end at an
//! element, or from an identifier and end at one.

use std::collections::BTreeSet;

use pdf_syntax::{Dictionary, Document, Object, ObjectId, tree};

use super::{
    Child, MAX_ANCESTRY, MAX_CHILDREN, MAX_DEPTH, MAX_OBJECT_SEARCH, OwnerSearch, Reading, Tree,
};

impl Tree {
    /// The structure element a `/ID` names, through §14.7.2's Table 354 `/IDTree`.
    ///
    /// > A name tree (see 7.9.6, "Name trees") that maps element identifiers (see "Table 355 -
    /// > Entries in a structure element dictionary") to the structure elements they denote.
    ///
    /// The identifiers are the `/ID` entries §14.7.2 puts on elements, and the tree is
    /// `pdf-syntax`'s: §7.9.6's keys are byte strings compared by code, which is what
    /// [`pdf_syntax::tree::lookup`] does. §14.7.7's worked example carries one, mapping `Chap1`,
    /// `Sec1.1`, `Sec1.2` and `Sec1.3` to four elements.
    ///
    /// `None` for a document that states no `/IDTree`, or for an identifier it does not hold.
    /// **This line used to say "89 of the corpus's 89 tagged ones" state none, and it was wrong
    /// from the round that wrote it**: 12 of those 89 state one, and over the wider corpus 22 of
    /// 151 tagged documents do — which is where all 475 of the identifiers Table 384's `/Headers`
    /// names are found. `examples/cell_header_census` is what counts it.
    #[must_use]
    pub fn element_by_id(&self, document: &Document, id: &[u8]) -> Option<Dictionary> {
        let tree = document.get_key(&self.root, "IDTree");
        let tree = tree.as_dict()?;
        let found = tree::lookup(tree, &tree::TreeKey::Name(id), &|object| {
            document.resolve(object)
        })?;
        document.resolve(&found).as_dict().cloned()
    }

    /// The structure elements this page's content items name, through §14.7.5.4's parent tree.
    ///
    /// §14.7.5.4 is the *only* route from content to structure the standard states — a stream
    /// cannot contain object references, so a marked-content sequence has no way of naming the
    /// element it belongs to, and the structural parent tree is the mechanism the clause provides
    /// instead. What it holds for a page is:
    ///
    /// > For a content stream containing marked-content sequences that are content items, the
    /// > value shall be an array of indirect references to the sequences' parent structure
    /// > elements.
    ///
    /// So a consumer asking what is on *one* page does not have to walk down from the root: the
    /// document has already written the answer down, keyed by the page's own `/StructParents`.
    /// [`Self::ancestry`] is the other half — the elements above these — and the two together are
    /// the subtree a page occupies.
    ///
    /// # The kinds of content item this looks for
    ///
    /// §14.7.5.1.1 makes a content item a marked-content sequence or a whole object, and Table 359
    /// keys each kind differently: the sequences of one content stream share the stream's
    /// `/StructParents`, and each object carries its own `/StructParent`. The objects a page can
    /// hold are its annotations and the `XObject`s its resources name — the clause's own EXAMPLE 1
    /// is a form `XObject` — so all are asked, and a form's resources are followed for the
    /// `XObject`s drawn inside it.
    ///
    /// **The fourth is a form's own `/StructParents`.** §14.7.5.2 gives a form two ways into the
    /// structure: a form that is a content item *in its entirety*
    /// carries `/StructParent`, and a form whose stream "may contain one or more marked-content
    /// sequences that are associated with structure elements" carries `/StructParents` instead —
    /// which Table 359 says in so many words, quoted under `Self::stream_owners`.
    /// An element reached only that way would otherwise be pruned as belonging to another page, so
    /// a figure tagged inside a form would reach no screen reader at all. `Self::stream_owners`
    /// is the lookup; ADR 0488.
    ///
    /// **The fifth is an annotation's appearance streams.** §12.5.5 makes an appearance stream a
    /// form `XObject`, so Table 359's words already covered it — but the `XObject` walk below
    /// starts from the page's *resources*, and an `/AP` entry is not a resource.
    /// `Self::appearance_owners` is the walk; ADR 0719.
    ///
    /// # Why a generous answer costs nothing, and why `None` is not an empty one
    ///
    /// An `XObject` a page's resources name may never be drawn, and an annotation may belong to
    /// another element than the one this returns. Neither matters: this answers which elements a
    /// page *may* reach, and whether one of them has a content item on the page is decided
    /// afterwards by the content items themselves — Table 355's and Table 358's `/Pg`. Missing an
    /// element is what would cost, which is why the question is asked three ways.
    ///
    /// Which is exactly why a page stating no `/StructParents` answers `None` rather than the
    /// annotations' elements alone. §14.7.5.4's Table 359 makes the entry
    ///
    /// > Required for all content streams containing marked-content sequences that are structural
    /// > content items
    ///
    /// so a page without one has said nothing about its sequences, and a set built from its
    /// annotations would be a *complete-looking* answer that silently omitted every paragraph on
    /// the page. Found by comparing this route against the whole-tree walk over the corpus, on a
    /// document whose pages carry widget annotations and no `/StructParents` at all; ADR 0325.
    ///
    /// **A page that states the entry answers `None` too where the parent tree holds no array
    /// under that key**, and a caller that reports the first reason for the second is saying
    /// something false about the file. `Self::parent_tree_array` is where the two forms
    /// §7.3.10 allows the array to be written are read as one.
    ///
    /// **The sixth route runs backwards, and only for a file that broke a `shall`**:
    /// [`Self::unkeyed_widget_owners`] asks §14.7.5.3's `/K` for the elements of this page's
    /// widget annotations that state no `/StructParent`, which Table 359 requires of them. It is
    /// the only route here that searches rather than looks up, and the clause that makes it a
    /// recovery rather than an invention is Table 368's — quoted there — so it is spent only on
    /// the pages that state such a widget.
    #[must_use]
    pub fn elements_on_page(
        &self,
        document: &Document,
        page: &Dictionary,
    ) -> Option<BTreeSet<ObjectId>> {
        // "For a content stream containing marked-content sequences that are content items, the
        // value shall be an array of indirect references to the sequences' parent structure
        // elements."
        let key = document.get_key(page, "StructParents").as_integer()?;
        let items = self.parent_tree_array(document, key)?;
        let mut out: BTreeSet<ObjectId> = items
            .iter()
            .take(MAX_CHILDREN)
            .filter_map(Object::as_reference)
            .collect();
        // "For an object identified as a content item by means of an object reference …, the
        // value shall be an indirect reference to the parent structure element."
        let mut visited = BTreeSet::new();
        if let Object::Array(annotations) = document.get_key(page, "Annots") {
            for annotation in annotations.iter().take(MAX_CHILDREN) {
                let resolved = document.resolve(annotation);
                let Some(dict) = resolved.as_dict() else {
                    continue;
                };
                out.extend(self.object_owner(document, dict));
                // The annotation's appearance streams are the fifth route, and Table 359 is why
                // they are one: `/StructParents` "may appear … in the stream dictionary of a form
                // or image XObject", §12.5.5 makes an appearance stream a form `XObject`, and
                // Table 357's `/Stm` row names "12.5.5, "Appearance streams"" beside §8.10's
                // forms as the streams a marked-content reference reaches. An element whose only
                // content item is a sequence inside an appearance stream is on the page and is
                // reachable no other way — the walk below only sees streams the page's
                // *resources* name, and an `/AP` entry is not a resource.
                self.appearance_owners(document, dict, &mut visited, &mut out);
            }
        }
        self.xobject_owners(document, page, 0, &mut visited, &mut out);
        // The sixth route. Its own answer is empty for every page whose widgets state the key
        // Table 359 requires, which is every page of a conforming file, so nothing is searched
        // unless the file has already contradicted itself.
        out.extend(self.unkeyed_widget_owners(document, page).items);
        Some(out)
    }

    /// The elements §14.7.5.3 names for this page's widget annotations that state no
    /// `/StructParent` — the route that is left when Table 359's key is missing.
    ///
    /// # Why a widget, and not any annotation
    ///
    /// Because §14.8.4.7.2's Table 368 states one `shall` about annotations and structure
    /// elements, and it is about this one alone:
    ///
    /// > In a tagged PDF, Form shall be used for each PDF widget annotation that belongs to the
    /// > real content of the document.
    ///
    /// Its neighbours in the same table are permissions rather than obligations. `Annot` is
    /// "\[e]ither an association between the content enclosed by the Annot structure element and
    /// one or more corresponding PDF annotations …, or a mechanism to include one or more PDF
    /// annotations in the structure tree", and "\[a]ll other annotation types may be referenced by
    /// this structure element"; `Link` is "\[a]n association between content enclosed by the Link
    /// structure element and a corresponding link annotation", which says what a `Link` element
    /// means and not that one has to exist. So a link or a text annotation that states no
    /// `/StructParent` is a file saying it is not a content item — and a *widget* that states
    /// none is a file that has broken a requirement, which is what makes looking for its element
    /// the other way round a recovery of something the standard says is there rather than an
    /// invention of something it does not.
    ///
    /// # The route
    ///
    /// §14.7.5.3 states the association in the direction the parent tree cannot:
    ///
    /// > When a structure element's content consists of an entire PDF object, such as an XObject
    /// > directly or indirectly referenced by a page description or an annotation, the object
    /// > shall be identified in the structure element's K entry by an object reference dictionary
    /// > ("Table 358 -Entries in an object reference dictionary").
    ///
    /// and that is a statement the *element* makes, needing no parent tree. §14.7.5.4 asks for
    /// both halves — "\[t]he tree shall contain an entry for each object that is a content item of
    /// at least one structure element" — so a file missing the key has contradicted itself, and
    /// the half still standing is the `/K`.
    ///
    /// # What it costs
    ///
    /// Finding an element by its content is a search, which is the cost ADR 0325 took off a page
    /// turn, so this is spent only where the page states such a widget and is bounded at
    /// `MAX_OBJECT_SEARCH` elements. An element the search cannot name by identity is passed
    /// over rather than descended into, because [`Self::ancestry`] and the walk that consumes
    /// this both work in identities: an element inside its parent's `/K` is one no page-scoped
    /// walk could enter even if this found something below it.
    #[must_use]
    pub fn unkeyed_widget_owners(
        &self,
        document: &Document,
        page: &Dictionary,
    ) -> Reading<ObjectId> {
        let nothing = Reading {
            items: Vec::new(),
            truncated: false,
        };
        let wanted = Self::unkeyed_widgets(document, page);
        if wanted.is_empty() {
            return nothing;
        }
        let mut search = OwnerSearch {
            wanted: &wanted,
            entered: BTreeSet::new(),
            budget: MAX_OBJECT_SEARCH,
            found: nothing,
        };
        self.search_owners(document, None, 0, &mut search);
        search.found
    }

    /// The page's `/Widget` annotations that state none of §14.7.5.4 Table 359's `/StructParent`,
    /// which that table makes "\[r]equired for all objects that are structural content items".
    fn unkeyed_widgets(document: &Document, page: &Dictionary) -> BTreeSet<ObjectId> {
        let mut out = BTreeSet::new();
        let Object::Array(annotations) = document.get_key(page, "Annots") else {
            return out;
        };
        for annotation in annotations.iter().take(MAX_CHILDREN) {
            let Some(id) = annotation.as_reference() else {
                continue;
            };
            let resolved = document.get(id);
            let Some(dict) = resolved.as_dict() else {
                continue;
            };
            if document
                .get_key(dict, "Subtype")
                .as_name()
                .is_some_and(|subtype| subtype.as_bytes() == b"Widget")
                && document
                    .get_key(dict, "StructParent")
                    .as_integer()
                    .is_none()
            {
                out.insert(id);
            }
        }
        out
    }

    /// [`Self::unkeyed_widget_owners`]'s descent: the identity of every element whose `/K` holds
    /// an object reference naming one of the wanted objects.
    fn search_owners(
        &self,
        document: &Document,
        element: Option<(&Dictionary, ObjectId)>,
        depth: usize,
        search: &mut OwnerSearch,
    ) {
        if depth >= MAX_DEPTH {
            return;
        }
        let here = element.map(|(_, id)| id);
        let node = element.map(|(dict, _)| dict);
        for (child, id) in self.identified_children(document, node) {
            match child {
                Child::Object { object, .. } => {
                    if let Some(here) = here
                        && search.wanted.contains(&object)
                        && !search.found.items.contains(&here)
                    {
                        search.found.items.push(here);
                    }
                }
                Child::Element(dict) => {
                    // An element with no identity of its own is passed over rather than
                    // descended into, for the reason the page-scoped walk skips one: Table 355
                    // makes `/P` an indirect reference, so a walk pruned to a set of identities
                    // could not enter it even if this found something below it.
                    let Some(id) = id else {
                        continue;
                    };
                    if !search.entered.insert(id) {
                        continue;
                    }
                    if search.budget == 0 {
                        search.found.truncated = true;
                        return;
                    }
                    search.budget = search.budget.saturating_sub(1);
                    self.search_owners(
                        document,
                        Some((&dict, id)),
                        depth.saturating_add(1),
                        search,
                    );
                }
                Child::MarkedContent { .. } => {}
            }
        }
    }

    /// The elements owed to one annotation's appearance streams (§12.5.5, §14.7.5.4).
    ///
    /// §12.5.5's Table 170 gives an annotation up to three appearances — `/N`, `/R`, `/D` —
    /// and each entry is "a single appearance stream or an appearance subdictionary" of one
    /// stream per state. Every one of those streams is a form `XObject` and may carry Table
    /// 359's entries like any other; its resources may name further forms, which is what the
    /// [`Self::xobject_owners`] call walks.
    fn appearance_owners(
        &self,
        document: &Document,
        annotation: &Dictionary,
        visited: &mut BTreeSet<ObjectId>,
        out: &mut BTreeSet<ObjectId>,
    ) {
        let appearances = document.get_key(annotation, "AP");
        let Some(appearances) = appearances.as_dict() else {
            return;
        };
        for key in ["N", "R", "D"] {
            let entry = document.get_key(appearances, key);
            let mut streams: Vec<Object> = Vec::new();
            match entry {
                Object::Stream(_) => streams.push(entry),
                Object::Dictionary(states) => {
                    for (_, state) in states.iter().take(MAX_CHILDREN) {
                        streams.push(document.resolve(state));
                    }
                }
                _ => {}
            }
            for stream in streams {
                let Object::Stream(stream) = stream else {
                    continue;
                };
                out.extend(self.object_owner(document, &stream.dict));
                self.stream_owners(document, &stream.dict, out);
                self.xobject_owners(document, &stream.dict, 1, visited, out);
            }
        }
    }

    /// Every element at or above one of `elements`, following §14.7.2's Table 355 `/P`:
    ///
    /// > The structure element or the structure tree root that is the immediate parent of this
    /// > structure element in the structure hierarchy.
    ///
    /// The entry is *required* and "shall be an indirect reference", and that second half is what
    /// makes this answerable at all: a chain of identities rather than of dictionaries, so a walk
    /// coming down from the root can recognise the elements it must descend into. Bounded by
    /// `MAX_ANCESTRY`, and a chain that meets an element already in the set stops there — the
    /// set is the union of the paths to one root.
    ///
    /// The root itself is not in the answer, which is the other half of the sentence quoted above:
    /// it is one of the two things a `/P` may name, it is not a structure element, and no walk
    /// needs to be told to enter it.
    #[must_use]
    pub fn ancestry(
        &self,
        document: &Document,
        elements: &BTreeSet<ObjectId>,
    ) -> BTreeSet<ObjectId> {
        let mut out: BTreeSet<ObjectId> = BTreeSet::new();
        for element in elements {
            let mut at = *element;
            for _ in 0..MAX_ANCESTRY {
                let resolved = document.get(at);
                let Some(dict) = resolved.as_dict() else {
                    break;
                };
                // Table 354 makes `/Type` required on the structure tree root and Table 355 makes
                // it optional on an element, so this is the one comparison that can tell the top
                // of the chain from the rest of it.
                if document
                    .get_key(dict, "Type")
                    .as_name()
                    .is_some_and(|kind| kind.as_bytes() == b"StructTreeRoot")
                {
                    break;
                }
                if !out.insert(at) {
                    break;
                }
                let Some(above) = dict.get("P").and_then(Object::as_reference) else {
                    break;
                };
                at = above;
            }
        }
        out
    }

    /// The parent tree entry a `/StructParent` or `/StructParents` key names, **unresolved**.
    ///
    /// Unresolved because the entry's whole content is identity: §14.7.5.4 makes both forms of
    /// value references to structure elements, and a resolved dictionary cannot be told from an
    /// equal one somewhere else in the tree.
    fn parent_tree_entry(&self, document: &Document, key: i64) -> Option<Object> {
        let parent_tree = document.get_key(&self.root, "ParentTree");
        let parent_tree = parent_tree.as_dict()?;
        tree::lookup_unresolved(parent_tree, &tree::TreeKey::Number(key), &|object| {
            document.resolve(object)
        })
    }

    /// §14.7.5.4's *array* form of a parent tree entry, resolved.
    ///
    /// > For a content stream containing marked-content sequences that are content items, the
    /// > value shall be an array of indirect references to the sequences' parent structure
    /// > elements.
    ///
    /// The array is the entry's **value**, and §7.3.10 puts no condition on how a value is
    /// written — "\[a]ny object in a PDF file may be labelled as an indirect object" — so
    /// §14.7.5.4's own EXAMPLE 2 spells one `/Nums` both ways, `6 [1 0 R]` beside `0 101 0 R`.
    /// A reference to the array and the array itself are therefore two spellings of one entry,
    /// and this resolves before matching.
    ///
    /// [`Self::parent_tree_entry`] stays unresolved, and the difference is what each form's
    /// content is: the object form's whole content is an identity, which resolution destroys,
    /// while here the identities are the array's *elements* and the array is their container.
    /// ADR 1151 measured what matching the direct spelling alone cost: **77 of the 109 corpus
    /// documents that state a structure tree write at least one `/Nums` array as an object of its
    /// own**, `structure_simple.pdf` among them, so [`Self::elements_on_page`] answered `None` for
    /// most of the tagged corpus and ADR 0325's page-scoped walk never ran on it.
    fn parent_tree_array(&self, document: &Document, key: i64) -> Option<Vec<Object>> {
        match document.resolve(&self.parent_tree_entry(document, key)?) {
            Object::Array(items) => Some(items),
            _ => None,
        }
    }

    /// Whether §14.7.5.3's object-reference route reaches this object.
    ///
    /// §14.7.5.1.1 makes "\[c]omplete PDF objects such as annotations and `XObjects`" one of the two
    /// kinds of content item, and §14.7.5.4 Table 359 gives such an object its own `/StructParent`
    /// — "\[f]or an object identified as a content item by means of an object reference …, the value
    /// shall be an indirect reference to the parent structure element". So this is the whole of
    /// what the *object* side of §14.8.2.2.2's "all page content as well as annotations" asks: is
    /// this annotation in the tree.
    ///
    /// One number-tree lookup and no resolution of the element, which is what lets
    /// [`crate::content::interpret`] ask it per annotation on a tagged page.
    #[must_use]
    pub fn claims_object(&self, document: &Document, object: &Dictionary) -> bool {
        self.object_owner(document, object).is_some()
    }

    /// The element an object content item belongs to, from its own `/StructParent`.
    fn object_owner(&self, document: &Document, object: &Dictionary) -> Option<ObjectId> {
        let key = document.get_key(object, "StructParent").as_integer()?;
        self.parent_tree_entry(document, key)?.as_reference()
    }

    /// The elements the sequences of *one content stream* belong to, from its `/StructParents`.
    ///
    /// The plural entry, which is the other of ISO 32000-2 §14.7.5.4 Table 359's two and the one
    /// a page uses:
    ///
    /// > Required for all content streams containing marked-content sequences that are structural
    /// > content items
    ///
    /// and Table 359 says where it may sit:
    ///
    /// > Depending on the type of content item, this entry may appear in the page object of a page
    /// > containing marked-content sequences, in the stream dictionary of a form or image XObject,
    /// > or in an annotation dictionary.
    ///
    /// A form `XObject` carrying §14.7.5.2's second arrangement states one — the clause's own
    /// words are quoted at [`Self::elements_on_page`] — and its array names elements that are on
    /// this page and reachable no other way.
    fn stream_owners(
        &self,
        document: &Document,
        stream: &Dictionary,
        out: &mut BTreeSet<ObjectId>,
    ) {
        let Some(key) = document.get_key(stream, "StructParents").as_integer() else {
            return;
        };
        let Some(items) = self.parent_tree_array(document, key) else {
            return;
        };
        out.extend(
            items
                .iter()
                .take(MAX_CHILDREN)
                .filter_map(Object::as_reference),
        );
    }

    /// [`Self::object_owner`] for every `XObject` a resource dictionary names, and for those the
    /// forms among them name in turn.
    ///
    /// §14.7.5.4's EXAMPLE 1 is exactly this shape — a form `XObject` stating `/StructParent 6`,
    /// drawn by the page's content stream and named by the page's resource dictionary — and a form
    /// may draw another. The visited set is what makes a form that names itself terminate, which
    /// §8.10.1 does not forbid a file from writing.
    fn xobject_owners(
        &self,
        document: &Document,
        node: &Dictionary,
        depth: usize,
        visited: &mut BTreeSet<ObjectId>,
        out: &mut BTreeSet<ObjectId>,
    ) {
        if depth >= MAX_DEPTH {
            return;
        }
        let resources = document.get_key(node, "Resources");
        let Some(resources) = resources.as_dict() else {
            return;
        };
        let xobjects = document.get_key(resources, "XObject");
        let Some(xobjects) = xobjects.as_dict() else {
            return;
        };
        for (_, value) in xobjects.iter().take(MAX_CHILDREN) {
            if let Some(id) = value.as_reference()
                && !visited.insert(id)
            {
                continue;
            }
            let resolved = document.resolve(value);
            let Object::Stream(stream) = &resolved else {
                continue;
            };
            // Table 359's two entries, and the clause forbids both at once: "[a]t most one of
            // these two entries shall be present in a given object. An object may be either a
            // content item in its entirety or a container for marked-content sequences that are
            // content items, but not both." So asking for each costs one dictionary lookup and
            // the second can only answer where the first did not.
            out.extend(self.object_owner(document, &stream.dict));
            self.stream_owners(document, &stream.dict, out);
            self.xobject_owners(
                document,
                &stream.dict,
                depth.saturating_add(1),
                visited,
                out,
            );
        }
    }
}
