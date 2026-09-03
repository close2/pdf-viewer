//! `pages` — one document's page list edited, RFC 0002 section 6.2.
//!
//! # One input, and why that is the boundary between this verb and `merge`
//!
//! RFC 0002 section 6.2 leaves one question open — "whether `pages` and `merge` are truly two
//! verbs or one (`merge` with a single input and edit flags subsumes `pages`)" — and this
//! module answers it by the count of files rather than by the kind of edit: **`pages` reads one
//! document and `merge` reads several**, which is what section 4.1 already says of both ("one
//! input, one PDF output" against "many inputs, one PDF output"). So `--insert` takes a range
//! of *this* document and puts a second copy of those pages somewhere else in it; a range of
//! another file is a usage refusal naming `merge`, because cross-file renumbering is the thing
//! `merge` exists to do and a second implementation of it here would be two answers to one
//! question.
//!
//! The engine underneath is one, and that is the point: [`merge::write`] takes a list of
//! [`Placement`]s and writes a document, and the two verbs differ only in how they build the
//! list. Every document-level reconciliation session 888 derived — §8.11's optional content,
//! §12.7's form, §7.9.6's name trees, §12.3.3's outline, §12.4.2's labels, §14.11.5's output
//! intents, §12.8.1's signatures — therefore applies to a page *leaving* exactly as it applies
//! to a page arriving, with no second construction to keep in step.
//!
//! # The operations, and what each rests on
//!
//! Operations compose **left to right over the current page list**, which is RFC 0002 section
//! 6.2's own rule, so every range is read against the list as the operations before it left it:
//! `--delete 3 --delete 3` takes out the third page and then the page that moved into its
//! place. `--help` says so, because the alternative reading — every range against the source's
//! original numbering — is equally defensible and the file is what decides between them.
//!
//! ## `--rotate [+|-]angle:range` — the rotation, and what it composes with
//!
//! §7.7.3.3's Table 31 entry is the whole definition:
//!
//! > ( Optional; inheritable ) The number of degrees by which the page shall be rotated
//! > clockwise when displayed or printed. The value shall be a multiple of 90. Default value: 0
//! > .
//!
//! **An unsigned angle is absolute and a signed one is relative** (qpdf's spelling, RFC 0002
//! section 6.2). A relative rotation composes with the page's ***effective*** rotation — the
//! one §7.7.3.4 gives it — and not with what its own dictionary happens to state:
//!
//! > If such an attribute is omitted from a page object, its value shall be inherited from an
//! > ancestor node in the page tree.
//!
//! > If the attribute is optional and no inherited value is specified, the default value shall
//! > be used.
//!
//! So `+90` on a page that states nothing and inherits `/Rotate 90` writes 180, and on a page
//! that inherits nothing writes 90. That is the only reading that makes `--rotate +90` mean
//! "turn this page a quarter turn from how it is displayed", which is what the clause's own
//! words are about; composing with the page's *stated* value would make the same flag do
//! different things to two pages a reader shows identically.
//!
//! **The value written is reduced modulo a full turn, into 0, 90, 180 or 270.** `-90` and `270`
//! name the same displayed page, and the clause's constraint is "a multiple of 90" rather than
//! a range, so both are legal to write and the smallest non-negative one is chosen. A
//! documented choice, not a derivation.
//!
//! **A source page whose effective rotation is not a multiple of 90** has broken the clause's
//! `shall`. Where no rotation is asked of it, it crosses as its producer wrote it — this verb
//! does not correct a file it was not asked to touch. Where one *is* asked of it, an absolute
//! angle replaces it outright, and a relative angle is composed against 0 with a warning naming
//! the page and the value, because there is no quarter-turn from 45 degrees.
//!
//! **Annotations are not touched, and §12.5.3 is why.** An annotation's `/Rect` is stated "in
//! default user space units" (Table 172), and `/Rotate` does not change default user space — it
//! is a display instruction, "when displayed or printed". The flag that looks like it bears on
//! this is Table 167's bit 5:
//!
//! > (PDF 1.3) If set, do not rotate the annotation's appearance to match the rotation of the
//! > page. The upper-left corner of the annotation rectangle shall remain in a fixed location
//! > on the page, regardless of the page rotation.
//!
//! and §12.5.3 states the same thing the other way round: "if the `NoRotate` flag is set, the
//! annotation shall retain its original orientation on the screen when the page is rotated (by
//! changing the Rotate entry in the page object; see 7.7.3, "Page tree")". The clause names
//! *this very edit* — changing `/Rotate` — and puts the whole consequence on the **viewer**:
//! nothing about the annotation's stored `/Rect` changes, the compensating transform is applied
//! at display time, and it pivots around the rectangle's upper-left corner. So `/NoRotate`
//! bears on display and not on the file, and a `pages --rotate` that rewrote an annotation's
//! `/Rect` would be moving an annotation the standard says stays where it is.
//!
//! ## `--delete range` — and what leaves with the page
//!
//! A deleted page is a page the output does not hold, and every reference to it becomes
//! §7.3.10's null, counted and warned — `merge`'s construction, on the clause's own words
//! ("[a]n indirect reference to an undefined object shall not be considered an error by a PDF
//! processor; it shall be treated as a reference to the null object"). §12.3.2.2 is what makes
//! that the right answer rather than a lost link: "in each case, page is an indirect reference
//! to a page object", so a destination *is* a reference and a reference to a page that is not
//! there is a null. An outline item keeps its place in the chain with a null destination rather
//! than being deleted, because deleting it would rebuild a chain the source stated.
//!
//! §12.4.2's labels are rebuilt one entry per output page, so the surviving pages keep the
//! labels they had — the clause numbers by *position* ("the indices shall be fixed, running
//! consecutively through the document starting from 0 for the first page"), so a deletion moves
//! every later index and no range of the source's tree survives it. §12.7's fields, §8.11's
//! groups and §7.9.6's trees each cross by session 888's reconciliation.
//!
//! ## `--move from:to` and the reorder
//!
//! A permutation of the list. Nothing else in the document has to move with it: §12.3.2.2 makes
//! a destination in this document an indirect reference to a page object, so it follows its
//! page through any permutation. What does move is §12.4.2's labels, which are positional, and
//! they are rebuilt per page as above — a page reordered keeps the label it had rather than
//! taking the label of the position it moved into, which is the only answer that keeps a
//! document's own identification of its pages.
//!
//! One destination form does **not** follow, and it is not this document's to fix: §12.3.2.2's
//! NOTE says that for a remote or embedded go-to action "the page parameter specifies an
//! integer page number within the remote document instead of a page object in the current
//! document". Such an integer names a position in *another* file and is carried unchanged.
//!
//! ## `--insert range@position` — a page twice, and Table 31
//!
//! §7.7.3.3's Table 31, on a page's `/Parent`:
//!
//! > ( Required; shall be an indirect reference ) The page tree node that is the immediate
//! > parent of this page object
//!
//! One `/Parent`, so one place in the tree: a page that appears twice in the output is **two
//! page objects**, and [`Duplicates::Copy`] is that. Everything below the page — its content
//! stream, its resources, its fonts — is shared by reference, because nothing in a page's
//! closure points back at the page. Its annotations do: Table 172's `/P` is "[a]n indirect
//! reference to the page object with which this annotation is associated", one page, so a
//! duplicated page gets its own annotation objects, and an annotation naming another —
//! §12.5.6.14's `/Popup`, §12.5.6.10's `/IRT` — names the copy beside it rather than the
//! original on the other page.
//!
//! **A page carrying a §12.7 widget is refused by name**, [`Refusal::DuplicateWidget`]. A
//! widget is a field's representation on a page, and §12.7.4.2 makes the fully qualified field
//! name the field's identity; a duplicated widget is either a second field under that name —
//! which the clause forbids unless "the same field type ( FT ), value ( V ), and default value
//! ( DV )" agree, and which would be a field this program invented — or a second representation
//! of the same field, which needs an entry in its field's own `/Kids` that this program would
//! have to write into an object the plan never asked it to touch. Neither is a page duplicated;
//! both are a form edited. So the operation is declined and says which page and which clause.
//!
//! # §14.7's structure tree, said plainly
//!
//! **No verb of this suite carries it**, `pages` included, and this is the honest statement of
//! what that costs rather than a half-carry. §14.7.1:
//!
//! > A PDF document's logical structure shall be stored separately from its visible content,
//! > with pointers from each to the other.
//!
//! The catalog's `/StructTreeRoot` is in `merge`'s `NOT_CARRIED`, so a source that states one is
//! named in a warning and the output states none. The *other* half of that pair — the pointers
//! from the content back to the structure — is Table 31's `/StructParents`, "[t]he integer key
//! of the page's entry in the structural parent tree", and a carried page still states the
//! integer its producer wrote. With no structure tree in the output, §14.7.5.4's parent tree
//! does not exist, so that integer **names nothing at all**; it does not name the wrong
//! element, which is the failure it would be if a partial tree were written. It is left as the
//! producer stated it — dropping it would be a second edit to the page dictionary in service of
//! a construct this verb does not write — and the warning says the tree is gone.
//!
//! A tagged document therefore loses its tagging to any of these verbs, with a warning to say
//! so. `doc/todo/57` names it as the largest single thing the suite owes.
//!
//! # Determinism
//!
//! The output is a function of the source and the plan: RFC 0002 section 9's first layer, with
//! no flag. Nothing here reads a clock, and the operations are applied in the order the caller
//! wrote them.

use pdf_model::Pages;
use pdf_model::page_label::PageLabels;
use pdf_syntax::Document;
use pdf_syntax::object::Object;

use crate::merge::{Duplicates, Placement, inherited};
use crate::pattern::Pattern;
use crate::range::Selection;
use crate::{Origin, Output, Refusal, Report, Sinks, Warning, merge};

/// One document's pages deleted, inserted, moved and rotated.
#[derive(Debug, Clone, PartialEq)]
pub struct PagesPlan {
    /// Which source.
    pub source: usize,
    /// The edits, applied left to right over the running page list.
    pub edits: Vec<Edit>,
    /// How the one output is named.
    pub names: Pattern,
}

/// One page edit.
///
/// Every range is resolved against the **current** list, which is RFC 0002 section 6.2's
/// composition rule.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// Take these pages out.
    Delete(Selection),
    /// Move these pages so that the first of them lands at `to`, counted from 1.
    Move {
        /// Which pages.
        pages: Selection,
        /// Where the block goes, counted from 1; one past the end appends.
        to: usize,
    },
    /// Put a second copy of these pages before `at`, counted from 1.
    Insert {
        /// Which pages.
        pages: Selection,
        /// Where the copies go, counted from 1; one past the end appends.
        at: usize,
    },
    /// §7.7.3.3's `/Rotate` on these pages.
    Rotate {
        /// Absolute, or relative to the page's effective rotation.
        angle: Angle,
        /// Which pages.
        pages: Selection,
    },
}

/// An angle as the caller wrote it: `90` is absolute, `+90` and `-90` are relative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Angle {
    /// The page's `/Rotate` becomes this.
    Absolute(i64),
    /// This much is added to the page's effective `/Rotate`.
    Relative(i64),
}

impl Angle {
    /// The degrees the caller wrote, whichever form.
    #[must_use]
    pub fn degrees(self) -> i64 {
        match self {
            Self::Absolute(degrees) | Self::Relative(degrees) => degrees,
        }
    }
}

/// A quarter turn, and the modulus a whole turn is taken to.
const QUARTER: i64 = 90;
/// A whole turn.
const TURN: i64 = 360;

/// One page of the running list: where it came from, and what it is rotated by.
#[derive(Debug, Clone, Copy)]
struct Item {
    /// The source page, zero-based.
    page: usize,
    /// Its effective §7.7.3.3 rotation, as the document leaves it or an edit set it.
    rotate: i64,
    /// Whether an edit decided that rotation, which is what makes it worth writing.
    edited: bool,
}

/// Edits one document's page list and writes the result.
///
/// `at` is the document's position among the opened ones — one, for this verb.
pub(crate) fn run(
    plan: &PagesPlan,
    at: usize,
    documents: &[Document],
    sinks: &dyn Sinks,
    report: &mut Report,
) -> Result<(), Refusal> {
    let document = documents.get(at).ok_or(Refusal::NoSuchSource {
        at: plan.source,
        count: documents.len(),
    })?;
    let pages = Pages::new(document);
    let labels = PageLabels::read(document);
    let mut list: Vec<Item> = (0..pages.len())
        .map(|page| Item {
            page,
            rotate: effective_rotation(document, &pages, page).unwrap_or(0),
            edited: false,
        })
        .collect();

    let mut warnings = Vec::new();
    for edit in &plan.edits {
        apply(edit, &mut list, plan.source, &labels, &mut warnings)?;
    }
    if list.is_empty() {
        return Err(Refusal::Assembly(
            "every page was deleted, and §7.7.3.2 makes /Kids \"an array of indirect references \
             to the immediate children\" of a node that has some"
                .to_owned(),
        ));
    }
    refuse_duplicated_widgets(document, &list, plan.source)?;

    let order: Vec<Placement> = list
        .iter()
        .map(|item| Placement {
            at,
            page: item.page,
            rotate: item.edited.then_some(item.rotate),
        })
        .collect();
    report.warnings.append(&mut warnings);
    let assembled = merge::write(
        &order,
        documents,
        &[plan.source],
        Duplicates::Copy,
        &plan.names,
        sinks,
        report,
    )?;
    report.outputs.push(Output {
        name: assembled.name,
        bytes: assembled.bytes,
        sanitised: assembled.sanitised,
        origin: Origin::Edited {
            source: plan.source,
            pages: order.len(),
            objects: assembled.objects,
        },
    });
    Ok(())
}

/// One edit over the running list.
fn apply(
    edit: &Edit,
    list: &mut Vec<Item>,
    source: usize,
    labels: &PageLabels,
    warnings: &mut Vec<Warning>,
) -> Result<(), Refusal> {
    // A range is read against the list as the edits before it left it, so the label of
    // position *p* is the label of whichever source page is standing there now.
    let label = |position: usize| list.get(position).and_then(|item| labels.label(item.page));
    match edit {
        Edit::Delete(pages) => {
            let chosen = resolve(pages, list.len(), label, source)?;
            let mut position = 0;
            list.retain(|_| {
                let keep = !chosen.contains(&position);
                position = position.saturating_add(1);
                keep
            });
        }
        Edit::Move { pages, to } => {
            let chosen = resolve(pages, list.len(), label, source)?;
            let moved: Vec<Item> = chosen
                .iter()
                .filter_map(|at| list.get(*at).copied())
                .collect();
            *list = spliced(list, &chosen, &moved, *to, source)?;
        }
        Edit::Insert { pages, at } => {
            let chosen = resolve(pages, list.len(), label, source)?;
            let copies: Vec<Item> = chosen
                .iter()
                .filter_map(|at| list.get(*at).copied())
                .collect();
            *list = spliced(list, &[], &copies, *at, source)?;
        }
        Edit::Rotate { angle, pages } => {
            let chosen = resolve(pages, list.len(), label, source)?;
            for position in chosen {
                let Some(item) = list.get_mut(position) else {
                    continue;
                };
                let turned = match angle {
                    Angle::Absolute(degrees) => *degrees,
                    Angle::Relative(degrees) => {
                        if item.rotate % QUARTER == 0 {
                            item.rotate.saturating_add(*degrees)
                        } else {
                            // §7.7.3.3: "The value shall be a multiple of 90." This one is not,
                            // so there is no quarter turn from it and the relative angle is
                            // composed against the default instead.
                            warnings.push(Warning {
                                source,
                                page: Some(item.page.saturating_add(1)),
                                detail: format!(
                                    "§7.7.3.3 makes /Rotate \"a multiple of 90\" and this page's \
                                     effective rotation is {}; a relative rotation was composed \
                                     against 0",
                                    item.rotate
                                ),
                            });
                            *degrees
                        }
                    }
                };
                if turned % QUARTER != 0 {
                    return Err(Refusal::Rotation { degrees: turned });
                }
                item.rotate = turned.rem_euclid(TURN);
                item.edited = true;
            }
        }
    }
    Ok(())
}

/// A selection resolved against the running list, as positions counted from 0.
fn resolve(
    pages: &Selection,
    count: usize,
    label: impl Fn(usize) -> Option<String>,
    source: usize,
) -> Result<Vec<usize>, Refusal> {
    pages
        .resolve(count, label)
        .map_err(|error| Refusal::Selection { at: source, error })
}

/// The list with `removed` taken out and `block` put in front of position `to`, counted from 1.
///
/// One rule covers both moving and inserting: walk the list, emit the block before the item
/// that stands at `to`, and skip whatever was removed. A `to` one past the end appends, which
/// is what lets a page be moved to the back.
fn spliced(
    list: &[Item],
    removed: &[usize],
    block: &[Item],
    to: usize,
    source: usize,
) -> Result<Vec<Item>, Refusal> {
    let Some(before) = to.checked_sub(1) else {
        return Err(Refusal::Position {
            at: source,
            position: to,
            count: list.len(),
        });
    };
    if before > list.len() {
        return Err(Refusal::Position {
            at: source,
            position: to,
            count: list.len(),
        });
    }
    let mut out = Vec::with_capacity(list.len().saturating_add(block.len()));
    for (position, item) in list.iter().enumerate() {
        if position == before {
            out.extend_from_slice(block);
        }
        if !removed.contains(&position) {
            out.push(*item);
        }
    }
    if before >= list.len() {
        out.extend_from_slice(block);
    }
    Ok(out)
}

/// §7.7.3.4's effective `/Rotate` for one page: its own, else its nearest ancestor's, else 0.
fn effective_rotation(document: &Document, pages: &Pages<'_>, index: usize) -> Option<i64> {
    let id = pages.get(index).and_then(|page| page.id)?;
    let stated = document
        .get_key_of(id, "Rotate")
        .map(|value| document.resolve(&value))
        .or_else(|| inherited(document, id, "Rotate").map(|value| document.resolve(&value)))?;
    stated.as_integer()
}

/// [`Refusal::DuplicateWidget`] where an edit would put a §12.7 widget on two pages.
fn refuse_duplicated_widgets(
    document: &Document,
    list: &[Item],
    source: usize,
) -> Result<(), Refusal> {
    let pages = Pages::new(document);
    let mut seen = std::collections::BTreeSet::new();
    for item in list {
        if seen.insert(item.page) {
            continue;
        }
        if has_widget(document, &pages, item.page) {
            return Err(Refusal::DuplicateWidget {
                at: source,
                page: item.page.saturating_add(1),
            });
        }
    }
    Ok(())
}

/// Whether a page's `/Annots` holds a §12.5.6.19 widget annotation.
fn has_widget(document: &Document, pages: &Pages<'_>, index: usize) -> bool {
    let Some(id) = pages.get(index).and_then(|page| page.id) else {
        return false;
    };
    let Some(Object::Array(items)) = document
        .get_key_of(id, "Annots")
        .map(|value| document.resolve(&value))
    else {
        return false;
    };
    items.iter().any(|item| {
        let resolved = document.resolve(item);
        resolved
            .as_dict()
            .and_then(|dict| dict.get("Subtype"))
            .and_then(|value| value.as_name().map(|name| name.as_bytes() == b"Widget"))
            .unwrap_or(false)
    })
}
