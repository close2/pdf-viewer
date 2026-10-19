//! A table's and a block's layout facts: spans, header scope, a cell's headers, the spacing and
//! the writing mode its attributes state.
//!
//! ISO 32000-2 §14.8.5.4 (layout attributes) and §14.8.5.7 (table attributes) are the home of
//! these entries; the list continuation is here because it is read from the same attribute
//! objects (§14.8.5.5).

use pdf_syntax::{Dictionary, Document, Object};

use super::{
    BlockSpacing, CellFacts, HeaderScope, ListContinuation, MAX_CHILDREN, Tree, WritingMode,
    allocation_rectangle, normalised_rectangle,
};

impl Tree {
    /// Table 384's `/RowSpan` and `/ColSpan` for a `TH` or `TD`, with the table's own defaults.
    ///
    /// ISO 32000-2 §14.8.5.7:
    ///
    /// > (Optional; not inheritable) The number of rows in the enclosing table that shall be
    /// > spanned by the cell.
    ///
    /// and the same sentence for columns, both with "Default value: 1". *Not inheritable* is why
    /// this asks [`Self::attribute`] rather than [`Self::inherited_attribute`] — a cell inside a
    /// spanned cell's table would otherwise take its enclosing cell's span.
    ///
    /// A value below 1 is read as 1: the entry is a number of rows or columns, and a cell
    /// occupies at least the one it is in.
    ///
    /// Both entries are read for a `TD` as well as a `TH`, because Table 384 states them for both
    /// and because a `TD`'s span is what pushes the *next* row's header cell off column zero.
    #[must_use]
    pub fn cell_span(&self, document: &Document, element: &Dictionary) -> (usize, usize) {
        let span = |name: &str| {
            self.attribute(document, element, name)
                .and_then(|value| value.as_integer())
                .and_then(|value| usize::try_from(value).ok())
                .filter(|value| *value >= 1)
                .unwrap_or(1)
        };
        (span("RowSpan"), span("ColSpan"))
    }

    /// Table 384's `/Scope`, where a `TH` states one.
    ///
    /// `None` for an element that states none — for which [`HeaderScope::assumed`] is the
    /// standard's own answer and needs the cell's place in the grid — and for a value outside the
    /// table's three names, which is a document stating something §14.8.5.7 does not define.
    ///
    /// The clause's condition on which attribute objects may carry it is
    /// [`Self::attribute`]'s already: the table attributes "may only be defined in attribute
    /// objects whose `O` (owner) entry has the value `Table` or whose owner is any other owner
    /// excluding `Layout`, `List`, `PrintField` and `Artifact`" — a paraphrase in this crate's own
    /// spelling of the names — and a format-specific owner's value is never consulted.
    #[must_use]
    pub fn header_scope(&self, document: &Document, element: &Dictionary) -> Option<HeaderScope> {
        self.attribute(document, element, "Scope")
            .and_then(|value| value.as_name().map(|name| name.as_bytes().to_vec()))
            .as_deref()
            .and_then(HeaderScope::read)
    }

    /// Table 379's `/BBox` — or Table 385's, which is the same rectangle under a second owner —
    /// in **default user space**, where the element states one.
    ///
    /// ISO 32000-2 §14.8.5.4.3:
    ///
    /// > An array of four numbers in default user space units that shall give the coordinates of
    /// > the left, bottom, right, and top edges, respectively, of the structure element's bounding
    /// > box (the rectangle that completely encloses its visible content).
    ///
    /// The other twelve entries of that table describe the layout process that produced an
    /// appearance this reader already has. This one describes a *result*, and it is the only
    /// statement the standard makes about where an element is that does not require re-running
    /// that process — which matters for exactly the elements whose extent cannot be recovered from
    /// the text layer, because they mark no text. The clause's own condition says which those are:
    ///
    /// > The BBox attribute should be present for structure elements whose content does not lend
    /// > itself to reflow or any other visual rearrangement of the content inside it.
    ///
    /// with NOTE 1 naming `Figure` and `Formula`, and 60 of the 61 corpus elements that state one
    /// *and* mark no text are `Figure`s.
    ///
    /// **Default user space, not the display list's**: §7.7.3.3's `/Rotate` and the crop box's
    /// origin stand between the two, and [`crate::content::page_space_at`] is the map. A caller
    /// that took these numbers for pixels would place a rotated page's figures off the page.
    ///
    /// Not inheritable, which is why this asks [`Self::attribute`]: a paragraph inside a figure has
    /// its own extent and the figure's rectangle is not a statement about it. **Table 379 is not
    /// the only table that states the entry**: §14.8.5.3's priority is applied over every
    /// PDF-native owner, so §14.8.5.8's Table 385 answers here too, for an attribute object owned
    /// by `Artifact` — the same rectangle, described in the same words, around content §14.8.2.2
    /// calls an artifact rather than the document's own. Table 379's cell says *not inheritable*
    /// outright; Table 385's says it only under Errata Collection 3's Issue #346, whose caret puts
    /// the same two words into it, so the choice made here now rests on the cell of whichever owner
    /// the document wrote rather than on one of the two.
    ///
    /// `None` for an element stating none, and for a value that is not four finite numbers —
    /// which no corpus document states.
    #[must_use]
    pub fn bounds(&self, document: &Document, element: &Dictionary) -> Option<[f32; 4]> {
        normalised_rectangle(document, &self.attribute(document, element, "BBox")?)
    }

    /// §14.8.5.4.5's **allocation rectangle** for this element, in default user space.
    ///
    /// §14.8.3.3 gives every block- and inline-level element two rectangles, and this is the
    /// second of them:
    ///
    /// > The allocation rectangle includes any additional borders or spacing surrounding the
    /// > element, affecting how it shall be positioned with respect to adjacent elements and the
    /// > enclosing content rectangle or reference area.
    ///
    /// §14.8.5.4.5 states the derivation outright, and it is two sentences rather than an
    /// algorithm:
    ///
    /// > For a BLSE, the allocation rectangle shall be equal to the content rectangle with its
    /// > before and after edges adjusted by the element's SpaceBefore and SpaceAfter attributes,
    /// > if any, but with no changes to the start and end edges.
    ///
    /// > For an ILSE, the allocation rectangle is the same as the content rectangle.
    ///
    /// **Which edges those are is [`WritingMode`]'s answer**, which is why §14.8.3.3 is the
    /// clause this reads and not only §14.8.5.4.5: the standard names the edges "by terms that
    /// are neutral with respect to the progression order rather than by familiar terms such as
    /// up, down, left, and right", so a `TbRl` document's before edge is on the right and
    /// adjusting the top of its rectangle would move the wrong side.
    ///
    /// # The content rectangle it starts from is the **stated** one
    ///
    /// [`Self::bounds`], which is Table 379's `/BBox`. The derived content rectangle — the union
    /// of what the element's marked-content sequences drew — is `viewer_core`'s, and it is in the
    /// viewport's pixels rather than in the space Table 379's two spacing numbers are "measured in
    /// default user space units" in. So this answers for the population that states a rectangle,
    /// and `None` for the rest, which is the same population [`Self::bounds`] answers for.
    ///
    /// # An ILSE needs no arm of its own
    ///
    /// §14.8.5.4.3 addresses Table 379's attributes to block-level elements — "layout attributes
    /// that shall apply only to block-level structure elements (BLSEs)" — so an ILSE states
    /// neither, [`Self::block_spacing`] answers with the table's own default of 0 for both, and
    /// the arithmetic below returns the content rectangle unchanged. That is the clause's ILSE
    /// sentence, arrived at rather than branched on, which is what lets this function answer
    /// without knowing §14.8.4.1's category — a fact that depends on where the element *sits* and
    /// that only a caller walking the tree has.
    #[must_use]
    pub fn allocation(&self, document: &Document, element: &Dictionary) -> Option<[f32; 4]> {
        let content = self.bounds(document, element)?;
        Some(allocation_rectangle(
            content,
            self.block_spacing(document, element),
            self.writing_mode(document, element),
        ))
    }

    /// Table 379's `/SpaceBefore` and `/SpaceAfter` for this element, with the table's defaults.
    ///
    /// §14.8.5.4.3:
    ///
    /// > The amount of extra space preceding the before edge of the BLSE, measured in default
    /// > user space units in the block-progression direction.
    ///
    /// and the same sentence for `/SpaceAfter` with *following* and *after*. Both are
    /// "(Optional; not inheritable)" with "Default value: 0", which is why this asks
    /// [`Self::attribute`] and not [`Self::inherited_attribute`]: a paragraph inside a `Div` that
    /// leaves room around itself has not asked for the same room around each of its lines.
    ///
    /// # Two sentences in the cells that are a layout engine's and are not applied here
    ///
    /// "If the preceding BLSE has a `SpaceAfter` attribute, the greater of the two attribute
    /// values shall be used" collapses the gap *between* two elements, and "[t]his attribute
    /// shall be disregarded for the first BLSE placed in a given reference area" drops it at the
    /// edge of an area. Both are about a sequence of elements being stacked into a reference
    /// area — §14.8.3.2's, which this program does not lay out — rather than about one element's
    /// own two rectangles, and applying either would mean answering for a neighbour this function
    /// has not been given.
    #[must_use]
    pub fn block_spacing(&self, document: &Document, element: &Dictionary) -> BlockSpacing {
        BlockSpacing {
            before: self.spacing_number(document, element, "SpaceBefore"),
            after: self.spacing_number(document, element, "SpaceAfter"),
        }
    }

    /// One of Table 379's two spacing numbers, or its default of 0.
    ///
    /// A value that is not a finite number is the default too: the table's type is `number`, and
    /// an infinity would push an allocation rectangle off every page rather than describe a gap.
    fn spacing_number(&self, document: &Document, element: &Dictionary, name: &str) -> f32 {
        let Some(number) = self
            .attribute(document, element, name)
            .as_ref()
            .and_then(Object::as_number)
            .filter(|number| number.is_finite())
        else {
            return 0.0;
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a spacing is a page coordinate, bounded by §14.11.2's 14 400 units"
        )]
        {
            number as f32
        }
    }

    /// Table 378's `/WritingMode` for this element, or §14.8.3.3's default.
    ///
    /// "(Optional; inheritable)", and the cell says how far it reaches: "[t]he specified layout
    /// directions shall apply to the given structure element and all of its descendants" —
    /// §14.8.5.3's fourth priority, which is [`Self::inherited_attribute`]. The cell's default is
    /// `LrTb`, which §14.8.3.3 describes as the Western one: "the block direction is from top to
    /// bottom and the inline direction is from left to right".
    ///
    /// A name the table does not define answers with the default as well, and that is a reading
    /// of the cell rather than a shrug: the value "shall be one of the following" eight, so a
    /// ninth is a document that has not stated a writing mode, and the entry's own default is
    /// what a document that states nothing means.
    #[must_use]
    pub fn writing_mode(&self, document: &Document, element: &Dictionary) -> WritingMode {
        self.inherited_attribute(document, element, "WritingMode")
            .as_ref()
            .and_then(Object::as_name)
            .and_then(|name| WritingMode::read(name.as_bytes()))
            .unwrap_or_default()
    }

    /// Table 384's `/Headers` for a `TH` or `TD`: the element identifiers the cell states.
    ///
    /// ISO 32000-2 §14.8.5.7:
    ///
    /// > An array of byte strings, where each string shall be the element identifier (see the ID
    /// > entry in "Table 355 - Entries in a structure element dictionary") for a TH structure
    /// > element that shall be used as a header associated with this cell.
    ///
    /// The identifiers are returned **unresolved**, in the document's own order, because the
    /// clause makes that order part of the answer: "[t]he order in which the entries in the
    /// Headers array are listed shall be row IDs followed by column IDs" and those "shall be
    /// ordered from most specific to most general". Resolving them is [`TableStack`]'s, which is
    /// the only thing that knows which cell a walk gave each identifier to.
    ///
    /// `Some(vec![])` for a cell stating an **empty** array, and that is not the same answer as
    /// `None`: §14.8.4.8.3's search runs only where "the Headers attribute … is not specified",
    /// so a cell that specified an empty one has said it has no headers rather than asked for
    /// them to be worked out. 2 of the corpus's 281 cells that state the entry state an empty
    /// array — measured by `examples/cell_header_census`.
    ///
    /// Not inheritable, which is why this asks [`Self::attribute`]: a cell inside a cell's own
    /// table would otherwise take the enclosing cell's headers.
    #[must_use]
    pub fn cell_headers(&self, document: &Document, element: &Dictionary) -> Option<Vec<Vec<u8>>> {
        let stated = self.attribute(document, element, "Headers")?;
        let items = stated.as_array()?;
        Some(
            items
                .iter()
                .take(MAX_CHILDREN)
                .filter_map(|item| {
                    document
                        .resolve(item)
                        .as_string()
                        .map(<[u8]>::to_vec)
                        .filter(|id| !id.is_empty())
                })
                .collect(),
        )
    }

    /// Table 384's `/Summary`, where the element states one.
    ///
    /// ISO 32000-2 §14.8.5.7:
    ///
    /// > (Optional; not inheritable; PDF 1.7) A summary of the table's purpose and structure.
    ///
    /// and the entry's own NOTE says who it is for: "[f]or use in non-visual rendering such as
    /// speech or braille." It is the one entry of the table that describes the *table* rather
    /// than a cell — "[t]his entry shall only be used within Table structure elements" — and
    /// that condition is the caller's to apply, exactly as a `TH`-only entry's is: which
    /// standard type an element maps to is §14.7.3's question, answered where the role is
    /// already in hand, and this reader would otherwise re-derive it per ask.
    ///
    /// *Not inheritable* is why this asks [`Self::attribute`] rather than
    /// [`Self::inherited_attribute`]: a table nested inside a summarised table's cell has not
    /// been summarised by its host's sentence.
    ///
    /// `None` for an element stating none, for a value that is not a string — the entry is a
    /// "text string", so a name or a number here is a statement §14.8.5.7 does not define — and
    /// for one whose text is empty, because the value exists to be spoken and an empty sentence
    /// in a listener's ear is a pause rather than an answer.
    #[must_use]
    pub fn table_summary(&self, document: &Document, element: &Dictionary) -> Option<String> {
        match self.attribute(document, element, "Summary") {
            Some(Object::String(bytes)) => {
                Some(pdf_syntax::text_string(&bytes)).filter(|text| !text.is_empty())
            }
            _ => None,
        }
    }

    /// Table 384's `/Short`, where the element states one.
    ///
    /// ISO 32000-2 §14.8.5.7:
    ///
    /// > (Optional; not inheritable; PDF 2.0) Contains a short form of the content of a TH
    /// > structure element's content.
    ///
    /// The entry's EXAMPLE is what it is for: when a cell's header cells are read to a person,
    /// "[i]t can become cumbersome for a user to repeatedly have to listen to the full contents
    /// of a TH structure element", and this is the author's own abbreviation of that repetition.
    /// "This entry shall only have an effect for structure elements of type of TH" is the
    /// caller's condition to apply, for the reason [`Self::table_summary`] gives.
    ///
    /// *Not inheritable*, so this asks [`Self::attribute`]: a `TH` under a shortened one has
    /// its own content and the enclosing cell's abbreviation is not a statement about it.
    ///
    /// `None` for the same three shapes as [`Self::table_summary`], for the same reasons.
    #[must_use]
    pub fn header_short(&self, document: &Document, element: &Dictionary) -> Option<String> {
        match self.attribute(document, element, "Short") {
            Some(Object::String(bytes)) => {
                Some(pdf_syntax::text_string(&bytes)).filter(|text| !text.is_empty())
            }
            _ => None,
        }
    }

    /// Table 382's `/ContinuedList` and `/ContinuedFrom`: what earlier list this `L` carries on.
    ///
    /// ISO 32000-2 §14.8.5.5 addresses these two of Table 382's three entries to whoever reads an
    /// `L` element rather than to whoever lays one out:
    ///
    /// > The ContinuedList and the ContinuedFrom attributes described in "Table 382 -Standard list
    /// > attributes" control the interpretation of the L element as it relates to other L elements
    /// > that are not its immediate parent.
    ///
    /// which is why they are read here and `/ListNumbering` — the numbering scheme whose labels are
    /// already on the page as `Lbl` elements — is not. Without them a listener is told a fresh list
    /// where the document said the items carry on from one it has already heard, and the numbering
    /// restarting is exactly what makes that confusing rather than merely incomplete.
    ///
    /// # What states a continuation
    ///
    /// `/ContinuedList` is "[a] flag specifying whether the list is a continuation of a previous
    /// list in the structure tree ( true ), or not ( false )", with "Default value: false".
    ///
    /// **A list stating `/ContinuedFrom` and no flag is read as continuing too, and that is a
    /// choice rather than a reading.** The entry is "[t]he ID … of the list for which this list is
    /// a continuation", so its presence asserts what the flag would; the standard does not say
    /// which of the two an inconsistent file means, and discarding the only statement it made
    /// would be the silence this reader exists to end. An explicit `/ContinuedList false` wins over
    /// it, because that is the producer saying `no` in the entry whose job is to say it.
    ///
    /// # Which list
    ///
    /// [`ListContinuation::From`] where `/ContinuedFrom` names one, unresolved — the identifier is
    /// Table 355's `/ID` of another element, and resolving it needs the walk, which is
    /// [`list_predecessors`]. Otherwise [`ListContinuation::Preceding`], because the clause names
    /// the predecessor itself in that case: "[i]f the ContinuedFrom attribute is not present, the
    /// continuation is from the preceding list at the same level in the structure hierarchy."
    ///
    /// # Not inheritable, and only since an erratum said so
    ///
    /// Both cells read `(Optional; PDF 2.0)` as printed. Errata Collection 3's Issue #346 carets
    /// *not inheritable; * into each of them — and into neither of Table 382's `/ListNumbering`,
    /// which stays inheritable — so this asks [`Self::attribute`] rather than
    /// [`Self::inherited_attribute`]. A list nested inside a continuing list has not been said to
    /// continue anything.
    ///
    /// Whether the element is an `L` at all is the caller's condition to apply, for the reason
    /// [`Self::table_summary`] gives: §14.7.3's mapped type is answered where the role is in hand.
    #[must_use]
    #[expect(
        clippy::doc_markdown,
        reason = "a verbatim quotation: Table 382 spells its own entries without backticks, and \
                  adding them inside the quotation marks would make the conformance gate's \
                  quotation check fail"
    )]
    pub fn list_continuation(
        &self,
        document: &Document,
        element: &Dictionary,
    ) -> Option<ListContinuation> {
        let flag = self.attribute(document, element, "ContinuedList");
        let from = self.attribute(document, element, "ContinuedFrom");
        match flag {
            Some(Object::Boolean(true)) => {}
            None if from.is_some() => {}
            _ => return None,
        }
        match from {
            Some(Object::String(id)) if !id.is_empty() => Some(ListContinuation::From(id.to_vec())),
            _ => Some(ListContinuation::Preceding),
        }
    }

    /// Everything one `TH` or `TD` says about itself that §14.8.4.8.3's search needs.
    ///
    /// Read in one call because [`TableStack`] wants all of it for the same element and each
    /// entry costs a walk of §14.7.6's attribute objects and §14.7.6.2's class map; a tagged
    /// document's elements are overwhelmingly paragraphs and spans, so this is asked only where
    /// the element has turned out to be a cell.
    #[must_use]
    pub fn cell_facts(&self, document: &Document, element: &Dictionary) -> CellFacts {
        let (row_span, column_span) = self.cell_span(document, element);
        CellFacts {
            row_span,
            column_span,
            scope: self.header_scope(document, element),
            // Table 355's `/ID` is on the element itself rather than in an attribute object:
            // "[t]he element identifier, a byte string designating this structure element."
            id: document
                .get_key(element, "ID")
                .as_string()
                .map(<[u8]>::to_vec)
                .filter(|id| !id.is_empty()),
            headers: self.cell_headers(document, element),
        }
    }
}
