//! Marked-content sections: §8.11's optional-content visibility and §14.9's accessibility
//! entries.
//!
//! A `BMC`/`BDC` … `EMC` section hangs three independent things off one nesting — whether
//! what follows marks the page, what a reader copies from it, and what a screen reader says
//! about it — and the types here are that nesting's state.

use pdf_render::display_list::Command;
use pdf_render::geom::{Rect, Transform};
use std::sync::Arc;

use pdf_syntax::{Dictionary, Object, ObjectId};

use super::Interpreter;
use super::report::Unsupported;

/// One open marked-content section, `BMC`/`BDC` to `EMC`.
///
/// Three independent things hang off the same nesting, which is why this is a struct rather
/// than a counter: §8.11.3.2's optional content decides whether what follows *marks the page*,
/// §14.9.4's replacement text decides what a reader *reads back* from it, and §14.9.3's and
/// §14.9.5's decide what a screen reader *says* about it. A section may be any, all or none.
#[derive(Debug, Clone, Default)]
pub(super) struct Marked {
    /// Whether this section's optional content is turned off.
    pub(super) hides: bool,
    /// Where in the readback this section's text began, for the two rules that need its extent.
    pub(super) starts_at: usize,
    /// §14.9.4's `/ActualText`, which replaces what the section reads back.
    pub(super) actual_text: Option<String>,
    /// §14.9.3's `/Alt` and §14.9.5's `/E`, which replace what it is *spoken* as, and
    /// §14.9.2's `/Lang`, which says in what language.
    ///
    /// `None` where the section states none of the three, which is what keeps an untagged page
    /// from allocating anything.
    pub(super) described: Option<Accessible>,
    /// §14.8.2.2's `/Artifact` tag, with Table 363's property list where the section has one.
    pub(super) artifact: Option<crate::structure::Artifact>,
    /// §14.13.5's `/AF` tag: the files associated with the graphics objects it encloses.
    ///
    /// Empty for every other section, which is all of them in 967 of the 974 corpus documents.
    pub(super) associated: Vec<crate::attachment::Attachment>,
    /// §14.7.5.2's `/MCID`, where this section's property list states one.
    ///
    /// The identifier is what ties a run of page content to a structure element, and therefore
    /// to §14.8.2.5.1's logical content order — which is a different order from the one this
    /// interpreter reads the stream in, and the only reason it is recorded.
    pub(super) mcid: Option<i64>,
    /// Whether this section's tag is §14.8.2.5.3's `ReversedChars`.
    ///
    /// A flag per section rather than one on the interpreter, because the sections nest and
    /// what has to be undone at `EMC` is this one's contribution.
    pub(super) reversed: bool,
}

/// What [`Interpreter::clip_extent`] has worked out about one of the display list's clips.
///
/// Three states rather than two nested `Option`s, because the middle one is the surprising one:
/// **a clip chain that admits nothing is an answer** — §8.5.4 makes each `W` intersect what is
/// already there, so two paths that do not meet leave no pixel — and it is worth keeping so that
/// the walk is not repeated for every command inside it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ClipExtent {
    /// No command inside a marked-content sequence has been clipped by this one yet.
    Unasked,
    /// The region it admits, or `None` where it admits none.
    Known(Option<Rect>),
}

/// One open marked-content sequence that stated an `/MCID`.
///
/// The two fields are the two questions §14.8.3.3 turns out to ask of a sequence, and they are
/// kept together because the second is only meaningful over the first's lifetime: what the
/// enclosed content *drew*, and whether any of what it enclosed this program **declined** to
/// draw. See [`Closed`] for why the second one is worth carrying.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct OpenSequence {
    /// §14.8.3.3's content rectangle so far, or `None` for a sequence that has drawn nothing.
    pub(super) extent: Option<Rect>,
    /// [`Interpreter::notes_raised`] at the moment this sequence opened.
    ///
    /// A high-water mark rather than a flag: a note raised anywhere between here and the `EMC`
    /// was raised *inside* this sequence, and one raised before it was not.
    notes_at_open: usize,
}

/// What one marked-content sequence turned out to be, at its `EMC`.
///
/// # Why the second field exists
///
/// §14.8.3.3 derives a structure element's content rectangle "from the shape of the enclosed
/// content", so an element whose enclosed content this program refused to draw has **no**
/// rectangle — not because the producer stated nothing and not because the sequence was empty,
/// but because the drawing did not happen. Those are three different facts and the extent alone
/// reports all three as `None`.
///
/// It is not a hypothetical distinction: with `pdf-sandbox-worker` absent, `issue5481.pdf`'s
/// `JPXDecode` image is refused, nine of its structure elements lose the only place they had, and
/// every instrument that counted them counted a smaller number with nothing able to say why
/// (ADR 0557, `doc/traps/instruments-and-reports.md` trap 16). ADR 0573.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Closed {
    /// §14.8.3.3's content rectangle, `[x0, y0, x1, y1]`, or `None` where nothing was drawn.
    pub(super) drawn: Option<[f32; 4]>,
    /// Whether any content the sequence enclosed produced an [`super::report::Unsupported`].
    pub(super) enclosed_a_refusal: bool,
}

/// §14.9's three spoken-form entries as one section states them.
#[derive(Debug, Clone, Default)]
pub(super) struct Accessible {
    /// §14.9.3's `/Alt`.
    pub(super) alt: Option<String>,
    /// §14.9.5's `/E`.
    pub(super) expansion: Option<String>,
    /// §14.9.2's `/Lang`, already resolved through a structure element's ancestry.
    pub(super) language: Option<String>,
}

impl Accessible {
    /// `None` where nothing was stated, so that a section with no accessibility entries costs
    /// no allocation and produces no span.
    fn or_nothing(self) -> Option<Self> {
        let stated = self.alt.is_some() || self.expansion.is_some() || self.language.is_some();
        stated.then_some(self)
    }
}

impl Interpreter<'_> {
    /// Appends a drawing command, and records what it contributes to the sequence enclosing it.
    ///
    /// **The one route from the interpreter into the display list.** ISO 32000-2 §14.8.3.3 gives
    /// every block- and inline-level structure element a content rectangle and says where it comes
    /// from:
    ///
    /// > The content rectangle shall be derived from the shape of the enclosed content and defines
    /// > the bounds used for the layout of any included child elements.
    ///
    /// §14.8.5.4.5 states the derivation for the two cases that are marks rather than layout — a
    /// table cell's is "determined from the bounding box of all graphics objects in the cell's
    /// content", an illustration's the same — so unioning each command's bound as it is emitted is
    /// the standard's own construction and not an invention of this program. What it yields is
    /// recorded as [`super::report::MarkedSpan::drawn`], which is a *derived* extent and stays
    /// separate from Table 379's `/BBox`, the producer's own statement.
    ///
    /// # Two things the union is narrowed by, and one it is not
    ///
    /// The command's bound is intersected with its clip chain, because §14.8.5.4.3's rectangle
    /// encloses "visible" content and a mark outside the clip is painted nowhere. It is **not**
    /// narrowed by the page boundary here: §14.11.2.1's crop box is a fact about the page rather
    /// than about the command, and `viewer_core` already applies it to the stated rectangle in the
    /// one place that holds both (ADR 0301).
    ///
    /// And it is a **bound**: [`Command::device_bounds`] counts a curve's control points and a
    /// mitre's reach, so the rectangle is never smaller than the ink. That is the direction a
    /// focus ring has to err in.
    ///
    /// # What it costs
    ///
    /// One `Vec::is_empty` per command on an untagged page, which is nearly every page, and that
    /// is the whole of what 885 of the corpus's 974 documents pay.
    ///
    /// On a tagged one it is a rectangle per command, and the A/B is this function with its body
    /// short-circuited, rebuilt, under callgrind — a stopwatch is the wrong instrument for a
    /// change this size (ADR 0312). Fifty interpretations of ISO 32000-2's page 101, a dense
    /// tagged page of text and vector graphics: **1236.7 M instructions without it and 1294.3 M
    /// with**, which is **1.15 M per page, 4.7% of what interpreting that page costs**. The same
    /// page interpreted once is 184.7 M against 185.8 M, and page *one* of the same document —
    /// the launch page, and the number `CLAUDE.md`'s startup rules bind — is 170.69 M against
    /// 170.93 M, **+0.14%**.
    ///
    /// Two thirds of the 1.15 M is the command's own bound and one third was the clip chain,
    /// before [`Interpreter::clip_extent`] stopped re-walking one that three hundred consecutive
    /// runs of a page share: with the chain walked per command it was 1.46 M rather than 1.15 M.
    /// The walk under both is [`pdf_render::geom::Path::hull`], computed once per distinct path
    /// and kept, so a page repeating one glyph outline pays for it once.
    pub(super) fn draw(&mut self, command: Command) {
        if !self.marking.is_empty() {
            let bounds = command.device_bounds(Transform::IDENTITY);
            let bounds = match (bounds, command.clip()) {
                (Some(bounds), None) => Some(bounds),
                (Some(bounds), Some(clip)) => self
                    .clip_extent(clip)
                    .and_then(|region| bounds.intersection(region)),
                (None, _) => None,
            };
            if let Some(bounds) = bounds
                && let Some(slot) = self.marking.last_mut()
            {
                slot.extent = Some(match slot.extent {
                    None => bounds,
                    Some(open) => open.union(bounds),
                });
            }
        }
        self.list.push(command);
    }

    /// [`pdf_render::DisplayList::clip_bounds`], answered once per clip rather than per command.
    ///
    /// A clipping region is immutable once `add_clip` has returned its identifier and the table
    /// only grows, so an answer is good for the rest of the page — and a page states far fewer
    /// clips than it does commands: ISO 32000-2's page 6 wraps `q`/`W n`/`Q` around **303** text
    /// runs and states **one** region between them (ADR 0132 measured the same ratio for a
    /// different reason). Without this the chain was walked and mapped once per command, which
    /// was a third of what [`Interpreter::draw`] costs on a tagged page.
    ///
    /// Indexed rather than keyed, because [`pdf_render::ClipId`] *is* an index into that table.
    fn clip_extent(&mut self, id: pdf_render::ClipId) -> Option<Rect> {
        let index = id.index();
        if self.clip_extents.len() <= index {
            self.clip_extents
                .resize(index.saturating_add(1), ClipExtent::Unasked);
        }
        if let Some(&ClipExtent::Known(known)) = self.clip_extents.get(index) {
            return known;
        }
        let answer = self.list.clip_bounds(id);
        if let Some(slot) = self.clip_extents.get_mut(index) {
            *slot = ClipExtent::Known(answer);
        }
        answer
    }

    /// Opens a sequence's extent, where the sequence stated an `/MCID`.
    pub(super) fn open_marking(&mut self, mcid: Option<i64>) {
        if mcid.is_some() {
            self.marking.push(OpenSequence {
                extent: None,
                notes_at_open: self.notes_raised,
            });
        }
    }

    /// Closes it, and hands the enclosing sequence what this one enclosed.
    ///
    /// The inner extent goes to the outer one because §14.8.3.3's rectangle is derived from *the
    /// enclosed* content, and a sequence that encloses another encloses its marks. §14.7.5.1.1
    /// forbids that nesting between two structure content items, so on a conforming file this
    /// branch never runs; it is what keeps a file that does it anyway from losing the marks.
    ///
    /// The refusal travels the same way and needs no propagation of its own: an enclosing
    /// sequence opened *before* the inner one, so a note raised inside the inner one is already
    /// past the outer one's mark.
    pub(super) fn close_marking(&mut self, mcid: Option<i64>) -> Option<Closed> {
        mcid?;
        let mine = self.marking.pop()?;
        if let Some(extent) = mine.extent
            && let Some(slot) = self.marking.last_mut()
        {
            slot.extent = Some(match slot.extent {
                None => extent,
                Some(open) => open.union(extent),
            });
        }
        Some(Closed {
            drawn: mine
                .extent
                .map(|rect| [rect.min.x, rect.min.y, rect.max.x, rect.max.y]),
            enclosed_a_refusal: self.notes_raised > mine.notes_at_open,
        })
    }

    /// Whether the content being interpreted right now belongs to a hidden layer.
    ///
    /// What this suppresses is *marking the page*, and nothing else. §8.11.3.1 is explicit
    /// that hiding changes what is drawn and not what the graphics state becomes: colour,
    /// transformation and clipping "shall still be applied", the text position is updated
    /// "even for text wrapped in optional content", and the state after the section is the
    /// same whether it was drawn or not. Suppressing at the point a command enters the
    /// display list is what makes that true by construction rather than by care.
    pub(super) fn is_hidden(&self) -> bool {
        self.hidden > 0
    }

    /// Whether content governed by `oc` is drawn, reporting what cannot be decided.
    ///
    /// `oc` is what a `BDC /OC`'s name finds in the page's `/Properties`, or the `/OC` entry
    /// of an `XObject` or an annotation — **as written**, reference and all. An optional
    /// content group is identified by which object it is (§8.11.2.2), so resolving it before
    /// this point loses the only identity it has.
    pub(super) fn shows_optional_content(&mut self, oc: &Object) -> bool {
        use crate::optional_content::Visibility;

        let Some(optional_content) = &self.optional_content else {
            // §8.11.4.2: with no `/OCProperties`, "a PDF processor shall ignore any optional
            // content structures in the document".
            return true;
        };
        match optional_content.visibility(self.document, oc) {
            Visibility::Visible => true,
            Visibility::Hidden => false,
            Visibility::TooDeep => {
                self.note(Unsupported::OptionalContent {
                    detail: "a /VE visibility expression nested past the interpreter's bound"
                        .to_owned(),
                });
                true
            }
        }
    }

    /// §14.9's four entries for a marked-content sequence, from either place each may live.
    ///
    /// The clause puts `/ActualText` (§14.9.4), `/Alt` (§14.9.3), `/E` (§14.9.5) and `/Lang`
    /// (§14.9.2.1) on a `Span` property list *and* on a structure element, and says the same
    /// thing of both. A sequence carrying an `/MCID` names its element through §14.7.5.4's
    /// parent tree, so both are reachable from a `BDC`; **the property list is asked first for
    /// each entry independently**, because it is the more specific statement — attached to
    /// this sequence rather than to the element the sequence belongs to — and because a file
    /// may put one entry in each place. Falling back per entry rather than per dictionary is
    /// what makes §14.9.3's own example legal: `/Span <</Lang (en-us) /Alt (six-point star)>>`
    /// beside an element that states only the language.
    ///
    /// The element is resolved at most once, and only for a sequence that named one. That
    /// matters: `structure.rs` records that following every entry of the specification's own
    /// first page costs 96 M instructions, so a lookup per entry would pay it four times.
    pub(super) fn accessibility(
        &self,
        resources: &Dictionary,
        operand: Option<&Object>,
    ) -> (Option<String>, Option<Accessible>) {
        let Some(list) = self.property_list(resources, operand) else {
            return (None, None);
        };
        let stated = |key: &str| match self.document.get_key(&list, key) {
            Object::String(bytes) => {
                let text = pdf_syntax::text_string(&bytes);
                (!text.is_empty()).then_some(text)
            }
            _ => None,
        };
        let mut actual_text = stated("ActualText");
        let mut described = Accessible {
            alt: stated("Alt"),
            expansion: stated("E"),
            language: stated("Lang"),
        };

        // The structure element supplies only what the property list did not state, and is
        // read only where one of the four is still missing *and* an `/MCID` names it.
        let missing = actual_text.is_none()
            || described.alt.is_none()
            || described.expansion.is_none()
            || described.language.is_none();
        if missing
            && let Some(mcid) = self.document.get_key(&list, "MCID").as_integer()
            && let Some(element) = self.structure.element(self.document, mcid)
        {
            let document = self.document;
            actual_text = actual_text.or_else(|| crate::structure::actual_text(document, &element));
            described.alt = described
                .alt
                .or_else(|| crate::structure::alternate_description(document, &element));
            described.expansion = described
                .expansion
                .or_else(|| crate::structure::expansion(document, &element));
            // The one of the four that is inherited: §14.9.2.3 has an element take its
            // language from "any parent element that has one".
            described.language = described
                .language
                .or_else(|| crate::structure::language(document, &element));
        }
        (actual_text, described.or_nothing())
    }

    /// Swaps in a content stream's own §14.7.5.4 parent tree, for as long as that stream runs.
    ///
    /// The clause makes the route back from a mark to its element **per stream** by construction:
    ///
    /// > The tree shall contain an entry for each object that is a content item of at least one
    /// > structure element and for each content stream containing at least one marked-content
    /// > sequence that is a content item.
    ///
    /// and Table 359 puts the key on the stream itself:
    ///
    /// > Depending on the type of content item, this entry may appear in the page object of a page
    /// > containing marked-content sequences, in the stream dictionary of a form or image XObject,
    /// > or in an annotation dictionary.
    ///
    /// So a form's `/MCID 3` indexes the *form's* array, and reading the page's array with it
    /// answers with whatever element happens to sit at index 3 of somebody else's content.
    ///
    /// A stream stating no `/StructParents` keeps the enclosing tree, which is what §14.7.5.2's
    /// *first* method means: a `Do` inside the page's own sequence makes the whole form part of
    /// that sequence, so the page's array is still the right one.
    ///
    /// Answers what to hand [`Self::leave_stream_structure`], and `None` where nothing was
    /// swapped — which is every stream of every untagged document and costs one dictionary lookup.
    pub(super) fn enter_stream_structure(
        &mut self,
        named: Option<ObjectId>,
        dict: &Dictionary,
    ) -> Option<Arc<crate::structure::ParentTree>> {
        if self.document.get_key(dict, "StructParents").as_integer()? < 0 {
            // Table 359 makes it "[t]he integer key of this object's entry", and a number tree's
            // keys are what §7.9.7 admits; a negative one names nothing, and taking it as "no
            // entry" is the same reading `ParentTree::for_page` gives a missing one.
            return None;
        }
        // A form drawn a hundred times states the same `/StructParents` a hundred times, and the
        // tree behind it is a number-tree walk plus an array copy. Memoised by the object the
        // stream is, which is exactly what makes two drawings of one form the same stream.
        let known = named.and_then(|object| self.stream_structures.get(&object));
        let read = if let Some(known) = known {
            Arc::clone(known)
        } else {
            let read = Arc::new(crate::structure::ParentTree::for_page(self.document, dict));
            if let Some(object) = named {
                self.stream_structures.insert(object, Arc::clone(&read));
            }
            read
        };
        Some(std::mem::replace(&mut self.structure, read))
    }

    /// Puts back what [`Self::enter_stream_structure`] took away.
    pub(super) fn leave_stream_structure(
        &mut self,
        outer: Option<Arc<crate::structure::ParentTree>>,
    ) {
        if let Some(outer) = outer {
            self.structure = outer;
        }
    }

    /// The property list a `BDC` operand names, inline or through `/Properties`.
    ///
    /// §14.6.2 gives the operand two forms, and which one a producer may use is decided by the
    /// values: a list of direct objects "may be written inline in the content stream as a
    /// direct object", and one holding an indirect reference "shall be defined as a named
    /// resource in the Properties subdictionary". Both are read here, which is what lets
    /// §14.9.4's `/ActualText` be found wherever a producer put it —
    /// §8.11.3.2's optional content is the one caller that cannot use this, because it needs
    /// the group's *identity* rather than its value.
    pub(super) fn property_list(
        &self,
        resources: &Dictionary,
        operand: Option<&Object>,
    ) -> Option<Dictionary> {
        match operand? {
            Object::Dictionary(list) => Some(list.clone()),
            Object::Name(name) => self
                .resource(resources, "Properties", name)
                .and_then(|list| list.as_dict().cloned()),
            _ => None,
        }
    }
}
