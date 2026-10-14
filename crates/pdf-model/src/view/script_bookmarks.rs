//! The outline a document's realm is told of, and the activations a script's `Bookmark` object asks
//! for (ADR 1934).
//!
//! Adobe's *JavaScript for Acrobat API Reference*, "Bookmark", "Bookmark properties" and "Bookmark
//! methods", cited and never quoted, is what a script asks; every answer is read from the entry
//! ISO 32000-2 states for it — Table 151's `/Title`, `/Count` and `/C`, Table 152's bits — through
//! [`crate::outline::Outline::read`], the reading the outline panel draws.
//!
//! **The outline is read on first use, not at the open.** A realm is told it once a script of the
//! document spells `bookmarkRoot`, or an outline item's own script runs and its `event.target` is
//! the item — the rule the pages' words keep (ADR 1762), and for the same reason: a request is data
//! handed across the process boundary before the script runs (ADR 1591), so what a script will read
//! is decided from its text. A document whose scripts never name the outline never pays for it.
//!
//! **`execute` is a person's click, carried out by the host.** §12.3.3 makes activating an item
//! "jump to a destination or trigger an action associated with the item"; a view state holds the
//! item a script executed and a host activates it as it activates a row of its panel (ADR 1904), so
//! that the item's `/Dest`, its `/A` chain and the scripts in it run as they run at a click. The
//! item is checked against the outline the realm was told: a realm that names any other object
//! activates nothing, and says so.

use pdf_syntax::{Document, ObjectId};

use super::ViewState;
use super::script_model::{BookmarkState, MAX_BOOKMARKS};

/// Most outline items scripts may ask a host to activate before it takes them: a script that
/// executes items in a loop has asked for more than a person could follow, and the rest are refused
/// with a sentence (ADR 1934).
const MAX_BOOKMARK_REQUESTS: usize = 16;

/// The outline as the realm is told it, and the items scripts executed.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Bookmarks {
    /// Every outline item in the order a reader meets them, once a script has needed them: `None`
    /// until then.
    told: Option<Vec<BookmarkState>>,
    /// The items scripts executed, in the order asked, until a host takes them; at most
    /// [`MAX_BOOKMARK_REQUESTS`].
    requests: Vec<ObjectId>,
}

impl ViewState {
    /// Reads the document's outline for its scripts, once, where `script` spells `bookmarkRoot` or
    /// runs at an outline item, and says where the outline is longer than a realm is told.
    pub(super) fn read_outline_for(&mut self, document: &Document, script: &str, at_item: bool) {
        if self.scripting.bookmarks.told.is_some() || !(at_item || spells_bookmarks(script)) {
            return;
        }
        let (states, whole) = bookmark_states(document);
        if !whole {
            self.report(format!(
                "the document's scripts read its outline, which holds more than {MAX_BOOKMARKS} \
                 items: bookmarkRoot answers the first {MAX_BOOKMARKS} in the order a reader \
                 meets them, and no later one (ADR 1934)"
            ));
        }
        self.scripting.bookmarks.told = Some(states);
    }

    /// Whether the outline has been read for the document's scripts.
    pub(super) fn outline_read(&self) -> bool {
        self.scripting.bookmarks.told.is_some()
    }

    /// The outline as a realm is told it, where it has been read.
    pub(super) fn bookmarks_told(&self) -> Option<Vec<BookmarkState>> {
        self.scripting.bookmarks.told.clone()
    }

    /// A script's `Bookmark.execute()`, held for a host where it names an item of the outline the
    /// realm was told, and refused with a sentence where it does not.
    pub(super) fn ask_bookmark(&mut self, number: u32, generation: u16) {
        let item = self.scripting.bookmarks.told.as_ref().and_then(|told| {
            told.iter()
                .find(|item| (item.number, item.generation) == (number, generation))
        });
        let Some(item) = item else {
            self.report(format!(
                "a script asked for the object {number} {generation} to be activated as an outline \
                 item, and it is not an item of the outline its realm was told, so nothing is \
                 activated (ADR 1934)"
            ));
            return;
        };
        let name = item.name.clone();
        let requests = &mut self.scripting.bookmarks.requests;
        if requests.len() < MAX_BOOKMARK_REQUESTS {
            requests.push(ObjectId::new(number, generation));
        } else {
            self.report(format!(
                "the document's scripts asked for more than {MAX_BOOKMARK_REQUESTS} outline items \
                 to be activated before the window could carry them out, and the outline item \
                 {name:?} is not activated (ADR 1934)"
            ));
        }
    }

    /// Every outline item a script executed since a host last took them, in the order asked
    /// (ADR 1934).
    ///
    /// A host activates each as it activates an item a person clicked in its panel — §12.3.3's
    /// destination or action, the item's scripts among them (ADR 1904) — after any call that ran
    /// scripts, as it carries out a script's page turn. Each is an item of the outline the realm
    /// was told, checked when the script's run was applied.
    pub fn take_bookmark_requests(&mut self) -> Vec<ObjectId> {
        std::mem::take(&mut self.scripting.bookmarks.requests)
    }
}

/// Whether a script's text spells `bookmarkRoot`, the one member that reaches the outline from a
/// script that is not an outline item's own (ADR 1934).
///
/// Read off the text for [`super::scripts`]' reason about the pages' words: a script that builds
/// the name at run time is told no outline, and its read is refused with a sentence saying the
/// outline was not read.
fn spells_bookmarks(script: &str) -> bool {
    script.contains("bookmarkRoot")
}

/// Every item of §12.3.3's outline in the order a reader meets them, each after its parent, at most
/// [`MAX_BOOKMARKS`]; and whether that is the whole outline (ADR 1934).
///
/// Read through [`crate::outline::Outline::read`], so that a script walks the tree a person sees,
/// with that reading's bounds on depth and on cycles.
fn bookmark_states(document: &Document) -> (Vec<BookmarkState>, bool) {
    let outline = crate::outline::Outline::read(document, &crate::page::Pages::new(document));
    let mut states = Vec::new();
    let mut waiting: Vec<(Option<u32>, &crate::outline::Item)> = outline
        .items
        .iter()
        .rev()
        .map(|item| (None, item))
        .collect();
    while let Some((parent, item)) = waiting.pop() {
        if states.len() >= MAX_BOOKMARKS {
            return (states, false);
        }
        let place = u32::try_from(states.len()).unwrap_or(u32::MAX);
        states.push(BookmarkState {
            number: item.id.number,
            generation: item.id.generation,
            parent,
            name: item.title.clone(),
            open: item.open,
            // Table 152 numbers its bits "from low-order to high-order bits, with the lowest-order
            // bit numbered 1": italic is the value 1 and bold the value 2, the reference's own
            // numbers for the two styles.
            style: u8::from(item.italic) | (u8::from(item.bold) << 1),
            colour: item.colour.map(f64::from),
        });
        waiting.extend(item.children.iter().rev().map(|child| (Some(place), child)));
    }
    (states, true)
}
