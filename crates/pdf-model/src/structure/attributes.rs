//! An element's attribute objects, and the entries read through them.
//!
//! ISO 32000-2 §14.7.6 defines the `/A` attribute objects and §14.8.5.2 their owners; §14.8.5.3
//! gives the values and their inheritance, §14.8.5.6 the print field's entries and §14.8.5.8
//! the artifact's. Everything here asks "what does this element say about itself, and who said
//! it".

use pdf_syntax::{Dictionary, Document, Object};

use super::{
    Artifact, ArtifactKind, AttributeObject, Checked, FieldRole, MAX_ANCESTRY, MAX_CHILDREN, Owner,
    PrintField, Tree, paired, ranked,
};

impl Tree {
    /// Every attribute object attached to an element, in **increasing precedence order**.
    ///
    /// §14.7.6 attaches attributes by two routes and states which wins in each. Within an
    /// element's `/A`, "\[i]f a given attribute is specified more than once, the later (in array
    /// order) entry shall take precedence"; between the two routes, "\[i]f both the A and C
    /// entries are present and a given attribute is specified by both, the one specified by
    /// the A entry shall take precedence". So the list is the `/C` classes first and the `/A`
    /// objects after them, each in array order, and the *last* object stating an attribute is
    /// the one that holds.
    ///
    /// **That order decides within one of §14.8.5.3's priorities and not between them**, which
    /// is why each object carries the route it came by ([`AttributeObject::from_class`]): the
    /// clause ranks an `/A` object above a class-map one whatever their array positions were,
    /// and ranks some owners above other `/A` objects again. [`Self::attribute`] applies that
    /// ranking to this list.
    ///
    /// Both entries carry §14.7.6.3's revision numbers, and reading them is not optional even
    /// though the mechanism is deprecated in PDF 2.0 and useless to a reader that does not
    /// edit: the revision is "the second (when present)" element of a *pair* inside the same
    /// array, so a reader that did not know about it would take an integer for an attribute
    /// object. The clause's own reason is in its NOTE 3 — "since an attribute object reference
    /// is distinct from an integer, that distinction is used to determine whether the
    /// attribute object is represented in the array by a single or a pair of entries".
    #[must_use]
    pub fn attributes(&self, document: &Document, element: &Dictionary) -> Vec<AttributeObject> {
        let mut out = Vec::new();
        for (name, revision) in paired(document, &document.get_key(element, "C")) {
            let Object::Name(class) = name else { continue };
            let map = document.get_key(&self.root, "ClassMap");
            let Some(map) = map.as_dict() else { continue };
            let named = document.get_key_by_name(map, &class);
            // "The corresponding value shall be an attribute object or an array of such
            // objects" — and this array is a plain list, since §14.7.6.3 puts the revision
            // beside the class *name* rather than beside the object it names.
            match named {
                Object::Array(items) => {
                    for item in items.iter().take(MAX_CHILDREN) {
                        if let Some(object) =
                            AttributeObject::read(document, &document.resolve(item), revision, true)
                        {
                            out.push(object);
                        }
                    }
                }
                other => {
                    if let Some(object) = AttributeObject::read(document, &other, revision, true) {
                        out.push(object);
                    }
                }
            }
        }
        for (object, revision) in paired(document, &document.get_key(element, "A")) {
            if let Some(object) = AttributeObject::read(document, &object, revision, false) {
                out.push(object);
            }
        }
        out
    }

    /// One attribute's value, by §14.8.5.3's priority, **without** the inherited step.
    ///
    /// The clause states five priorities and this applies the first three, which are the ones
    /// about *this* element. [`Priority`] is that ranking as a type and carries each band's
    /// own words; what this function adds is the search:
    ///
    /// - **between bands, the band decides.** A band-1 value beats a band-2 one and both beat
    ///   the class map's, whatever order the objects were written in.
    /// - **within one band, §14.7.6's two rules decide**, and [`Self::attributes`] has already
    ///   put the objects in the order that makes the last match the winner — which is why this
    ///   walks the list in reverse and keeps the first value found at the best band so far.
    /// - **an object the clause does not admit at all is not consulted**, which is
    ///   [`AttributeObject::priority`] answering `None`: an export format's attributes, because
    ///   §14.8.5.2 says they "shall be applied only when processing PDF content based on that
    ///   format" and this program processes to no format, and a namespace owner whose name this
    ///   standard gives no owner for. Both are refused *by name* rather than in silence:
    ///   [`Self::unranked_owners`] is the list of what was skipped on an element.
    ///
    /// The fourth priority — "\[t]he resolved value of the parent structure element, if the
    /// attribute is inheritable" — is [`Self::inherited_attribute`], because whether an
    /// attribute *is* inheritable is stated per attribute in §14.8.5.4's tables and is a
    /// property of the attribute rather than of the element. The fifth, a default, is the same
    /// kind of statement and belongs to whoever knows the attribute.
    ///
    /// [`Priority`]: crate::structure::Priority
    #[must_use]
    pub fn attribute(
        &self,
        document: &Document,
        element: &Dictionary,
        name: &str,
    ) -> Option<Object> {
        let attached = self.attributes(document, element);
        ranked(document, &attached, name, |_| true)
    }

    /// The attribute objects on this element whose owner §14.8.5.3 gives no applicable rank.
    ///
    /// One string per object, as the file wrote the owner: the `/O` name, or — where `/O` is
    /// `NSO` — the namespace name §14.7.4.2 says "shall be considered as identifying the owner".
    /// These are the objects [`Self::attribute`] skips, and naming them is the difference
    /// between a reader that declined an owner it does not process and one that dropped
    /// attributes without saying so (`doc/traps/parsers-and-streams.md` trap 5).
    ///
    /// An export-format owner is in this list for a reason the clause states and is not a defect
    /// in the file; a namespace owner is in it because the clause's equivalence has no member for
    /// that name. `examples/attribute_owner_census` asks it of every element of every document
    /// this project holds.
    #[must_use]
    pub fn unranked_owners(&self, document: &Document, element: &Dictionary) -> Vec<String> {
        self.attributes(document, element)
            .into_iter()
            .filter(|object| object.priority().is_none())
            .map(|object| object.owner_as_written())
            .collect()
    }

    /// §14.8.5.3's fourth priority: the value this element or its nearest ancestor states.
    ///
    /// > Inheritable attributes propagate down the structure tree; that is, an attribute that is
    /// > specified for an element shall apply to all the descendants of the element in the
    /// > structure tree unless a descendent element specifies an explicit value for the attribute.
    ///
    /// The walk goes up `/P`, which is the same chain §14.9.2.3's language inheritance uses and
    /// is bounded the same way — a `/P` is a reference a document controls. **Whether to call
    /// this or [`Self::attribute`] is the caller's decision**, because the clause makes
    /// inheritability a property of each attribute: §14.8.5.4's tables say which are inheritable,
    /// and a reader that inherited every attribute would give a paragraph its table's
    /// `/ColSpan`.
    ///
    /// The clause's own closing sentence is why this returns the same kind of answer as the
    /// direct one: "\[t]here is no semantic distinction between attributes that are specified
    /// explicitly and ones that are inherited."
    #[must_use]
    pub fn inherited_attribute(
        &self,
        document: &Document,
        element: &Dictionary,
        name: &str,
    ) -> Option<Object> {
        let mut current = element.clone();
        for _ in 0..MAX_ANCESTRY {
            if let Some(value) = self.attribute(document, &current, name) {
                return Some(value);
            }
            current = document.get_key(&current, "P").as_dict()?.clone();
        }
        None
    }

    /// §14.8.5.6's `PrintField` attributes, for a `Form` element of a non-interactive form.
    ///
    /// The clause explains what the attribute is *for*, and it is the only place in §14.8.5 that
    /// describes something a reader could act on: a form
    ///
    /// > may have originally contained interactive fields such as text fields and radio buttons
    /// > but were then converted into non-interactive PDF files, or they may have been designed
    /// > to be printed out and filled in manually.
    ///
    /// So the widget is gone and the marks are all that is left, and this attribute is what says
    /// the marks were a check box, whether it was ticked, and what the field was called. A
    /// screen reader that skipped it would read a form as a page of unlabelled boxes.
    ///
    /// `None` where the element states no `PrintField` attributes, which is every element of
    /// every corpus document — measured.
    ///
    /// Which attribute objects may carry it is stated by the clause and applied by
    /// [`Self::attribute`] already: it "may only be defined in attribute objects whose O (owner)
    /// entry has the value `PrintField` or whose owner is any other owner excluding `Layout`,
    /// `List`, `Table` and `Artifact`" — a paraphrase in this crate's own spelling of the names.
    #[must_use]
    pub fn print_field(&self, document: &Document, element: &Dictionary) -> Option<PrintField> {
        let role = self
            .attribute(document, element, "Role")
            .and_then(|value| value.as_name().map(|name| name.as_bytes().to_vec()));
        // Table 383 gives this entry two spellings and deprecates one of them: "Checked,
        // checked … lower case form is deprecated in PDF 2.0", with NOTE 2 explaining that the
        // old case "did not conform to the same conventions used elsewhere in this standard".
        // Both are read, the current spelling first, because deprecation tells a *writer* what
        // to stop doing — the same reading §12.2's `/ViewArea` gets.
        let checked = self
            .attribute(document, element, "Checked")
            .or_else(|| self.attribute(document, element, "checked"))
            .and_then(|value| value.as_name().map(|name| name.as_bytes().to_vec()));
        let description = match self.attribute(document, element, "Desc") {
            Some(Object::String(bytes)) => Some(pdf_syntax::text_string(&bytes)),
            _ => None,
        };
        if role.is_none() && checked.is_none() && description.is_none() {
            return None;
        }
        Some(PrintField {
            role: role.as_deref().and_then(FieldRole::read),
            // "Default value: off ." — applied here rather than left to a caller, because the
            // clause states it and an unticked box is what a form full of them looks like.
            checked: checked
                .as_deref()
                .and_then(Checked::read)
                .unwrap_or(Checked::Off),
            description,
        })
    }

    /// Table 385's artifact attributes on this element: §14.8.5.8's form of §14.8.2.2's artifact.
    ///
    /// # Why this returns the same type the content stream's artifacts do
    ///
    /// §14.8.2.2.2 gives an artifact two forms, "\[b]y enclosing it in a marked-content sequence
    /// with the tag Artifact" and "\[b]y inclusion in the logical structure tree through the use
    /// of the Artifact structure element type", and they carry the same facts under two tables:
    /// Table 363's property list entries and Table 385's attributes. [`Artifact::read`] answers
    /// the first; this answers the second, into the same [`Artifact`], so a consumer deciding
    /// what to do with a running head does not have to ask which form its producer chose.
    ///
    /// # Where the two tables differ, and it is not a copy
    ///
    /// **The fourth type name.** Table 363's is `Background`, "content ... from document
    /// templates that are often repeated unchanged across many pages"; Table 385's is `Inline`
    /// (PDF 2.0), which "enclose\[s] artifact content that has context in the document's logical
    /// structure". Each reader takes its own table's four names and nothing else, which is what
    /// keeps [`ArtifactKind`] one enum rather than a union of two vocabularies.
    ///
    /// **What `/Subtype` may sit beside.** Table 385's cell says the entry "should appear only
    /// when the Type entry has a value of Pagination or Inline" where Table 363's names
    /// `Pagination` alone. It is a `should` in both, so neither reader conditions on it: a
    /// document that states a subtype under some other type has stated one, and dropping it
    /// would be answering with less than the file says.
    ///
    /// **`/Attached` is Table 363's alone**, so [`Artifact::attached`] is four falses here. That
    /// is the table's own silence rather than an unread entry.
    ///
    /// # The owner restriction is §14.8.5.8's first sentence
    ///
    /// > The artifact attributes described in "Table 385 -Standard artifact attributes" may only
    /// > be defined in attribute objects whose O (owner) entry has the value Artifact or whose
    /// > owner is any other owner excluding Layout, List, PrintField and Table .
    ///
    /// So `Artifact` and `NSO` are the two owners [`Self::attribute`]'s §14.8.5.3 ranking admits
    /// that this sentence also admits; the other four PDF-native owners are excluded by name, and
    /// a format-specific owner is priority 1's, conditioned on "processing based on the format
    /// indicated by the owner value", which this program does not do. `/BBox` is the exception
    /// and is [`Self::bounds`]'s: Table 379 states the same entry under `Layout`, and which of
    /// the two owners a producer wrote it under is not a fact about the rectangle.
    ///
    /// # Every entry may be absent, and the element is still an artifact
    ///
    /// The same answer [`Artifact::read`] gives for `/Artifact BMC`: §14.8.2.2.2 calls that "a
    /// generic artifact", and the type of the *element* is what said it is one. Which elements to
    /// ask about is therefore the caller's, exactly as it is for [`Self::table_summary`] and
    /// [`Self::header_short`] — §14.8.4.8.7's `Artifact` is the type these attributes describe,
    /// and that type is §14.7.3's mapped one, which this function is not given.
    #[must_use]
    pub fn artifact(&self, document: &Document, element: &Dictionary) -> Artifact {
        let kind = self
            .artifact_attribute(document, element, "Type")
            .as_ref()
            .and_then(Object::as_name)
            .and_then(|name| match name.as_bytes() {
                b"Pagination" => Some(ArtifactKind::Pagination),
                b"Layout" => Some(ArtifactKind::Layout),
                b"Page" => Some(ArtifactKind::Page),
                b"Inline" => Some(ArtifactKind::Inline),
                _ => None,
            });
        let subtype = self
            .artifact_attribute(document, element, "Subtype")
            .as_ref()
            .and_then(Object::as_name)
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned());
        Artifact {
            kind,
            subtype,
            bbox: self.bounds(document, element),
            attached: [false; 4],
        }
    }

    /// One Table 385 attribute, under the owners §14.8.5.8's first sentence permits.
    ///
    /// [`Self::attribute`] with one filter added: that clause names four of the five PDF-native
    /// owners and excludes them, so what is left is `Artifact` and whatever §14.7.4.2's
    /// equivalence leaves an `NSO` object owned by — which is why the filter is applied to
    /// [`AttributeObject::effective_owner`] rather than to `/O` as written. `UserProperties` and
    /// an export format are already out, the first because §14.7.6.4 makes that owner's `/P`
    /// array the attributes rather than the entries beside it, the second because §14.8.5.2
    /// applies it only to a processor translating to that format.
    ///
    /// The ranking is [`Self::attribute`]'s unchanged: the clause narrows *who may state* these
    /// attributes and says nothing about which of two statements wins, so §14.8.5.3 still does.
    fn artifact_attribute(
        &self,
        document: &Document,
        element: &Dictionary,
        name: &str,
    ) -> Option<Object> {
        let attached = self.attributes(document, element);
        ranked(document, &attached, name, |owner| {
            !matches!(
                owner,
                Owner::Layout | Owner::List | Owner::PrintField | Owner::Table
            )
        })
    }
}
