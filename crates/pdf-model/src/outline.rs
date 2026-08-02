//! ISO 32000-2 §12.3.3's document outline.
//!
//! The outline — bookmarks, to most people — is "a tree-structured hierarchy of outline items
//! … which serve as a visual table of contents to display the document's structure to the
//! user". `CLAUDE.md` names it in scope, and 176 of the 974 corpus documents have one.
//!
//! # A tree written as four linked lists
//!
//! The clause does not store children in an array. Each level is a doubly-linked list threaded
//! through `/Prev` and `/Next`, entered by the parent's `/First` and `/Last`, and every item
//! points back at its `/Parent`:
//!
//! > The items at each level of the hierarchy form a linked list, chained together through
//! > their Prev and Next entries and accessed through the First and Last entries in the parent
//! > item (or in the outline dictionary in the case of top-level items).
//!
//! Six references per item, all indirect, all writable by a producer that got one of them
//! wrong — so this module follows `/First` and `/Next` only, bounds the walk, and refuses to
//! visit an object twice. `/Prev`, `/Last` and `/Parent` are redundant with what it already
//! has; reading them could only disagree.
//!
//! # What is here and what is not
//!
//! An outline is a *panel* in a viewer that has none, so this module answers the question a
//! viewer without a panel can still ask: **which item covers the page being shown**.
//! [`Outline::section_at`] is that, and it is the whole of what `viewer-ui` uses.

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use crate::destination::Destination;
use crate::page::Pages;

/// Deepest nesting followed.
///
/// A table of contents nests as deep as a book has heading levels. Anything approaching this
/// is malformed, and the visited set below already catches the cycle case — this bounds the
/// stack.
const MAX_DEPTH: usize = 32;

/// Most items read from one document.
///
/// A bound on the work a malformed file can ask for, in the same spirit as the page tree's.
/// The largest outline in the corpus is two orders of magnitude below it.
const MAX_ITEMS: usize = 1 << 16;

/// One entry in the outline.
#[derive(Debug, Clone)]
pub struct Item {
    /// The item's own object, which is how a caller asks for it to be *activated*.
    ///
    /// §12.3.3: "[c]licking the text of any visible item activates the item, causing the
    /// interactive PDF processor to jump to a destination or trigger an action associated with
    /// the item." [`Self::destination`] is the first half; the second is `/A`, which may be any
    /// of §12.6's types and a `/Next` chain of them — so what a caller needs is not a payload
    /// but a *name* for the thing to activate, and the object is it. Every item is reached
    /// through an indirect reference, which Table 151 requires of `/First`, `/Next` and `/Last`
    /// alike, so this is always known.
    pub id: ObjectId,
    /// Table 151's `/Title`, "[t]he text that shall be displayed on the screen for this item".
    ///
    /// A *text string*, so §7.9.2.2's three encodings apply and `pdf_syntax::text_string`
    /// decodes it — the same route a page label's prefix and a field's value take.
    pub title: String,
    /// Where the item goes: its `/Dest`, or the `/D` of a go-to action in its `/A`.
    ///
    /// Table 151 makes those mutually exclusive — `/Dest` "shall not be present if an A entry
    /// is present" — so a file writing both has said something the clause forbids, and `/Dest`
    /// wins here because it is the older and simpler statement of the same thing.
    pub destination: Option<Destination>,
    /// Whether the item is open, from the sign of `/Count`.
    ///
    /// "If the outline item is open, Count is the sum of the number of visible descendent
    /// outline items at all levels… If the outline item is closed, Count is negative". An item
    /// with no descendants states no `/Count` and is neither; it reads as closed, which is
    /// what a leaf looks like on a screen.
    pub open: bool,
    /// Table 152's bit 1: display the item in italic.
    pub italic: bool,
    /// Table 152's bit 2: display the item in bold.
    pub bold: bool,
    /// Table 151's `/C`, "the components in the `DeviceRGB` colour space of the colour that
    /// shall be used for the outline entry's text. Default value: [0.0 0.0 0.0]."
    pub colour: [f32; 3],
    /// The item's immediate children, in the order the linked list holds them.
    pub children: Vec<Item>,
}

/// A document's outline, read once.
#[derive(Debug, Clone, Default)]
pub struct Outline {
    /// The top-level items.
    pub items: Vec<Item>,
    /// The outline dictionary's own `/Count`, where it states one.
    ///
    /// Kept as the file wrote it rather than as a computed value, because the clause defines
    /// it by an algorithm this module can run — so the two can be compared, which is the only
    /// way to find out whether a producer ran it too.
    pub stated_count: Option<i64>,
}

impl Outline {
    /// Reads the catalog's `/Outlines`, which most documents do not have.
    ///
    /// An absent entry is a document with no outline, not a defect, and produces an empty
    /// outline rather than an error.
    #[must_use]
    pub fn read(document: &Document, pages: &Pages<'_>) -> Self {
        let Ok(catalog) = document.catalog() else {
            return Self::default();
        };
        let root = document.get_key(&catalog, "Outlines");
        let Some(root) = root.as_dict() else {
            return Self::default();
        };
        let mut visited = std::collections::BTreeSet::new();
        let mut budget = MAX_ITEMS;
        Self {
            items: level(document, pages, root, 0, &mut visited, &mut budget),
            stated_count: document.get_key(root, "Count").as_integer(),
        }
    }

    /// Whether the document states an outline at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The visible-item count §12.3.3 defines, run over what was read.
    ///
    /// The clause states this as an algorithm rather than as a number, which is what makes it
    /// checkable:
    ///
    /// > Step 1. Initialize Count to zero. Step 2. Add to Count the number of immediate
    /// > children. During repetitions of this step, update only the Count of the original
    /// > outline item. Step 3. For each of those immediate children whose Count is positive and
    /// > non-zero, repeat steps 2 and 3.
    ///
    /// "[T]hose immediate children whose Count is positive" are the *open* ones, so this counts
    /// every item reachable without opening anything that is closed. A file states the same
    /// number in its outline dictionary, and the two agreeing is the file agreeing with itself.
    #[must_use]
    pub fn visible_count(&self) -> usize {
        fn walk(items: &[Item]) -> usize {
            items.iter().fold(items.len(), |total, item| {
                if item.open {
                    total.saturating_add(walk(&item.children))
                } else {
                    total
                }
            })
        }
        walk(&self.items)
    }

    /// The title of the innermost item covering a page, or `None`.
    ///
    /// The question a viewer with no outline panel can still answer: *where in the document am
    /// I*. An item covers a page when its destination names that page or an earlier one and no
    /// later item does — which is the reading order of a table of contents, and it is a
    /// property of the outline rather than a rule the clause states, so it is a **documented
    /// choice**: §12.3.3 describes a panel a person clicks, and says nothing about mapping a
    /// page back to an item.
    ///
    /// Items with no resolvable destination are skipped rather than treated as covering
    /// nothing, because a heading whose link is broken still names the section that follows it.
    #[must_use]
    pub fn section_at(&self, document: &Document, pages: &Pages<'_>, index: usize) -> Option<&str> {
        fn walk<'a>(
            items: &'a [Item],
            document: &Document,
            pages: &Pages<'_>,
            indices: &BTreeMap<ObjectId, usize>,
            index: usize,
            best: &mut Option<(usize, &'a str)>,
        ) {
            for item in items {
                if let Some(page) = item
                    .destination
                    .and_then(|destination| destination.page_index_with(document, pages, indices))
                    && page <= index
                    && best.is_none_or(|(at, _)| page >= at)
                {
                    *best = Some((page, item.title.as_str()));
                }
                walk(&item.children, document, pages, indices, index, best);
            }
        }
        // **One walk of the page tree for the whole outline, rather than one per item.**
        // `Pages::index_of` cannot skip a subtree, so resolving every item's destination
        // separately is quadratic in the document: ISO 32000-2's 988 items over its 1023 pages
        // cost 344 ms, on a path a person takes by pressing an arrow key. See `Pages::indices`.
        let indices = pages.indices();
        let mut best = None;
        walk(&self.items, document, pages, &indices, index, &mut best);
        best.map(|(_, title)| title)
    }
}

/// Reads one level's linked list, from a node whose `/First` starts it.
fn level(
    document: &Document,
    pages: &Pages<'_>,
    parent: &Dictionary,
    depth: usize,
    visited: &mut std::collections::BTreeSet<u32>,
    budget: &mut usize,
) -> Vec<Item> {
    if depth > MAX_DEPTH {
        return Vec::new();
    }
    let mut items = Vec::new();
    let mut next = parent.get("First").and_then(Object::as_reference);
    while let Some(id) = next {
        if *budget == 0 || !visited.insert(id.number) {
            // Either the file is asking for more items than any outline has, or its `/Next`
            // chain points back at something already read. Both end the level rather than the
            // whole outline: what was read before is still the document's own list.
            break;
        }
        *budget = budget.saturating_sub(1);
        let item = document.get(id);
        let Some(item) = item.as_dict() else {
            break;
        };
        items.push(read_item(document, pages, id, item, depth, visited, budget));
        next = item.get("Next").and_then(Object::as_reference);
    }
    items
}

/// Reads one item and its children.
fn read_item(
    document: &Document,
    pages: &Pages<'_>,
    id: ObjectId,
    dict: &Dictionary,
    depth: usize,
    visited: &mut std::collections::BTreeSet<u32>,
    budget: &mut usize,
) -> Item {
    let count = document.get_key(dict, "Count").as_integer();
    let flags = document.get_key(dict, "F").as_integer().unwrap_or(0);
    Item {
        id,
        title: match document.get_key(dict, "Title") {
            Object::String(bytes) => pdf_syntax::text_string(&bytes),
            _ => String::new(),
        },
        destination: destination(document, dict),
        open: count.is_some_and(|count| count > 0),
        // Table 152 numbers its bits "from low-order to high-order bits, with the lowest-order
        // bit numbered 1", so bit 1 is the value 1 and bit 2 the value 2.
        italic: flags & 1 != 0,
        bold: flags & 2 != 0,
        colour: colour(document, dict),
        children: level(
            document,
            pages,
            dict,
            depth.saturating_add(1),
            visited,
            budget,
        ),
    }
}

/// Table 151's `/Dest`, or the destination inside `/A`'s go-to action.
///
/// §12.6.4.2 is the only action type that states a view of *this* document; a `/GoToR` names a
/// page in another file and an ECMAScript action is on `CLAUDE.md`'s exclusion list. An item
/// with any other action has no destination here, which is the truth about it rather than a
/// gap: nothing this program does could follow one.
fn destination(document: &Document, dict: &Dictionary) -> Option<Destination> {
    if let Some(dest) = dict.get("Dest")
        && let Some(destination) = Destination::read(document, dest)
    {
        return Some(destination);
    }
    let action = document.get_key(dict, "A");
    let action = action.as_dict()?;
    let kind = document.get_key(action, "S");
    if kind.as_name()?.as_bytes() != b"GoTo" {
        return None;
    }
    Destination::read(document, action.get("D")?)
}

/// Table 151's `/C`, defaulting to black.
///
/// "An array of three numbers in the range 0.0 to 1.0" — a value outside that range is not a
/// colour the clause describes, and clamping is what every other `DeviceRGB` component in this
/// tree gets, so it is what this one gets.
fn colour(document: &Document, dict: &Dictionary) -> [f32; 3] {
    let array = document.get_key(dict, "C");
    let Some(items) = array.as_array() else {
        return [0.0; 3];
    };
    let mut out = [0.0f32; 3];
    for (slot, item) in out.iter_mut().zip(items) {
        if let Some(value) = document.resolve(item).as_number() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a colour component is between 0 and 1 and is clamped to it here"
            )]
            {
                *slot = (value as f32).clamp(0.0, 1.0);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::Outline;
    use crate::page::Pages;
    use pdf_syntax::Document;

    /// Builds a document from object bodies numbered from 1.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// The clause's own example from H.6, cut to the shape that matters.
    ///
    /// Two top-level items, the first open with two children, the second closed with one — so
    /// the visible count is 2 + 2 = 4 and *not* 5, which is the whole content of the clause's
    /// three-step algorithm. The nesting also proves the walk descends: a reader that followed
    /// only `/Next` would return two items and count two.
    fn outlined() -> Document {
        document(&[
            "<< /Type /Catalog /Pages 2 0 R /Outlines 5 0 R >>",
            "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /Outlines /First 6 0 R /Last 9 0 R /Count 4 >>",
            "<< /Title (Chapter 1) /Parent 5 0 R /Next 9 0 R /First 7 0 R /Last 8 0 R \
             /Count 2 /Dest [3 0 R /Fit] >>",
            "<< /Title (Section 1.1) /Parent 6 0 R /Next 8 0 R /Dest [3 0 R /XYZ null 700 null] >>",
            "<< /Title (Section 1.2) /Parent 6 0 R /Prev 7 0 R /Dest [3 0 R /XYZ null 400 null] >>",
            "<< /Title (Chapter 2) /Parent 5 0 R /Prev 6 0 R /First 10 0 R /Last 10 0 R \
             /Count -1 /Dest [4 0 R /Fit] /C [1 0 0] /F 2 >>",
            "<< /Title (Section 2.1) /Parent 9 0 R /Dest [4 0 R /Fit] >>",
        ])
    }

    /// The hierarchy is read as a tree, with titles, destinations, colour and style.
    #[test]
    fn the_linked_lists_read_as_a_tree() {
        let doc = outlined();
        let pages = Pages::new(&doc);
        let outline = Outline::read(&doc, &pages);

        assert_eq!(outline.items.len(), 2);
        let first = outline.items.first().expect("chapter 1");
        assert_eq!(first.title, "Chapter 1");
        assert!(first.open, "a positive /Count is an open item");
        assert_eq!(
            first
                .children
                .iter()
                .map(|item| item.title.as_str())
                .collect::<Vec<_>>(),
            ["Section 1.1", "Section 1.2"],
            "the children come back in the order the /Next chain holds them"
        );

        let second = outline.items.get(1).expect("chapter 2");
        assert!(!second.open, "a negative /Count is a closed item");
        assert!(second.bold && !second.italic, "Table 152's bit 2");
        // `[1 0 0]` reaches the array unchanged: every component is exactly representable and
        // nothing scales it, so an exact comparison is the assertion the clause supports.
        #[expect(clippy::float_cmp, reason = "the file's own three integers, unscaled")]
        {
            assert_eq!(second.colour, [1.0, 0.0, 0.0]);
        }
        assert_eq!(second.children.len(), 1, "a closed item still has children");
    }

    /// The clause's three-step count, and it is not the number of items.
    ///
    /// Five items exist and four are visible, because chapter 2 is closed. The distinction is
    /// the entire content of the algorithm, and a reader that returned five would agree with
    /// no producer.
    #[test]
    fn the_visible_count_skips_a_closed_items_descendants() {
        let doc = outlined();
        let pages = Pages::new(&doc);
        let outline = Outline::read(&doc, &pages);
        assert_eq!(outline.visible_count(), 4);
        assert_eq!(
            outline.stated_count,
            Some(4),
            "and the document says so itself"
        );
    }

    /// A `/Next` chain that points back at an earlier item terminates.
    ///
    /// Every one of Table 151's six links is an indirect reference a producer can get wrong,
    /// and a reader that followed `/Next` without a visited set would not return.
    #[test]
    fn a_cycle_in_the_next_chain_terminates() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /Outlines /First 5 0 R >>",
            "<< /Title (A) /Next 6 0 R >>",
            "<< /Title (B) /Next 5 0 R >>",
        ]);
        let pages = Pages::new(&doc);
        let outline = Outline::read(&doc, &pages);
        assert_eq!(outline.items.len(), 2, "both items, and then it stops");
    }

    /// The page a reader is on names the innermost item at or before it.
    #[test]
    fn a_page_finds_the_section_that_covers_it() {
        let doc = outlined();
        let pages = Pages::new(&doc);
        let outline = Outline::read(&doc, &pages);
        assert_eq!(
            outline.section_at(&doc, &pages, 0),
            Some("Section 1.2"),
            "the last item on page one, not the first"
        );
        assert_eq!(outline.section_at(&doc, &pages, 1), Some("Section 2.1"));
    }
}
