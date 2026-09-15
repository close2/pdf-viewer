//! §12.5.6.2's `/IRT` and `/RT`, counted: how many annotations belong to a group, and how many
//! of them state a group attribute of their own.
//!
//! The clause makes a set of annotations behave as one unit:
//!
//! > In PDF 1.6, a set of annotations may be grouped so that they function as a single unit when
//! > a user interacts with them. The group consists of a primary annotation, which shall not have
//! > an IRT entry, and one or more subordinate annotations, which shall have an IRT entry that
//! > refers to the primary annotation and an RT entry whose value is Group .
//!
//! (Errata Collection 3's Issue #297 strikes that sentence's *PDF 1.6* and writes *PDF 1.5*,
//! inserting *(PDF 1.6)* after the `RT` instead: the grouping is PDF 1.5's, the `/RT` entry is
//! the later half. The blockquote keeps the published wording the quotation gate reads, and this
//! census counts what documents state rather than what their headers claim.)
//!
//! and then says what a subordinate's own entries are worth:
//!
//! > Some entries in the primary annotation are treated as "group attributes" that shall apply to
//! > the group as a whole; the corresponding entries in the subordinate annotations shall be
//! > ignored. These entries are Contents (or RC and DS ), M , C , T , Popup , CreationDate ,
//! > Subj , and Open .
//!
//! So the population that *ranks* the rule is not "annotations with an `/IRT`" but the narrower
//! one where obeying it changes an answer: a subordinate that states one of those nine itself.
//! Anything else reads identically whether the rule is applied or not, which is trap 8's shape and
//! the reason this counts three things rather than one.
//!
//! `/RT`'s default is `R`, so an annotation with an `/IRT` and no `/RT` is a **reply** and not a
//! group member — counted separately here, with the popup window it would open on its own, because
//! the clause's other `shall` is about those: "[i]nteractive PDF processors shall not display
//! replies to an annotation individually but together in the form of threaded comments."
//!
//! ```sh
//! cargo run --release -p pdf-model --example annotation_group_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_syntax::{Document, Object};

/// The nine entries §12.5.6.2 makes group attributes.
///
/// `/DS` is in the clause's parenthesis beside `/RC` — "Contents (or RC and DS )" — so the list of
/// keys is ten where the list of attributes is nine.
const GROUP_ATTRIBUTES: [&str; 10] = [
    "Contents",
    "RC",
    "DS",
    "M",
    "C",
    "T",
    "Popup",
    "CreationDate",
    "Subj",
    "Open",
];

/// What one document contributes to the census.
#[derive(Default)]
struct Counts {
    /// Annotations seen at all.
    seen: usize,
    /// Annotations stating an `/IRT`.
    in_reply_to: usize,
    /// Of those, the ones stating `/RT /Group`: §12.5.6.2's subordinate annotations.
    subordinate: usize,
    /// Of those, the ones whose `/IRT` names an annotation this page's `/Annots` also lists.
    subordinate_on_page: usize,
    /// Of those, the ones stating at least one group attribute of their own.
    subordinate_states_one: usize,
    /// Group attributes stated by a subordinate whose primary states the key differently.
    ///
    /// The count that ranks the rule: every one of these is an entry this program reads from the
    /// wrong dictionary, and each is a title, a colour, a date or a window's text.
    disagreeing: usize,
    /// The same count broken down by key, in [`GROUP_ATTRIBUTES`]' order.
    ///
    /// Which of the ten disagree decides what a reader *sees*: a `/T` or a `/Contents` is a
    /// window's title and body, a `/C` is ink, and a `/Subj` is neither until there is a panel.
    by_key: [usize; GROUP_ATTRIBUTES.len()],
    /// Subordinates with no `/AP`, whose `/C` is therefore a mark on the page.
    ///
    /// §12.5.2's rendering rule ignores `/C` where a stored appearance exists, so this is the
    /// population where reading the entry from the wrong dictionary changes a raster rather than
    /// a window.
    subordinate_synthesised: usize,
    /// Popup windows whose `/Parent` is a subordinate.
    ///
    /// Table 186 makes the parent's `Contents`, `M`, `C` and `T` override the popup's own, and
    /// §12.5.6.2 makes all four group attributes — so every one of these is a window this program
    /// fills from a dictionary the clause says to ignore.
    popup_of_subordinate: usize,
    /// Replies — an `/IRT` with no `/RT`, or `/RT /R`.
    reply: usize,
    /// Of those, the ones naming a popup window of their own through Table 172's `/Popup`.
    ///
    /// Every one of these is a window the clause's other `shall` forbids opening on its own:
    /// "[i]nteractive PDF processors shall not display replies to an annotation individually but
    /// together in the form of threaded comments."
    reply_with_popup: usize,
    /// Of those, the ones whose thread **head** names a `/Popup` of its own.
    ///
    /// The head is the annotation reached by following `/IRT` until one states none, which is
    /// where a thread has to be displayed if it is to be displayed once. A reply whose head has
    /// no window of its own would have nowhere to be shown — this is the count that says whether
    /// that case is real.
    reply_head_with_popup: usize,
    /// The deepest `/IRT` chain seen, in hops from a reply to its head.
    deepest_thread: usize,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, other: &Self) {
        self.seen = self.seen.saturating_add(other.seen);
        self.in_reply_to = self.in_reply_to.saturating_add(other.in_reply_to);
        self.subordinate = self.subordinate.saturating_add(other.subordinate);
        self.subordinate_on_page = self
            .subordinate_on_page
            .saturating_add(other.subordinate_on_page);
        self.subordinate_states_one = self
            .subordinate_states_one
            .saturating_add(other.subordinate_states_one);
        self.disagreeing = self.disagreeing.saturating_add(other.disagreeing);
        for (mine, theirs) in self.by_key.iter_mut().zip(other.by_key) {
            *mine = mine.saturating_add(theirs);
        }
        self.subordinate_synthesised = self
            .subordinate_synthesised
            .saturating_add(other.subordinate_synthesised);
        self.popup_of_subordinate = self
            .popup_of_subordinate
            .saturating_add(other.popup_of_subordinate);
        self.reply = self.reply.saturating_add(other.reply);
        self.reply_with_popup = self.reply_with_popup.saturating_add(other.reply_with_popup);
        self.reply_head_with_popup = self
            .reply_head_with_popup
            .saturating_add(other.reply_head_with_popup);
        self.deepest_thread = self.deepest_thread.max(other.deepest_thread);
    }
}

fn main() {
    let mut total = Counts::default();
    let mut opened = 0_usize;
    let mut lines: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        let counts = document_counts(&document);
        if counts.in_reply_to > 0 {
            let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
            lines.push(format!(
                "  {name}: {} /IRT ({} /RT /Group, {} of them on the same page, {} stating a \
                 group attribute, {} disagreeing with the primary), {} replies ({} with a popup)",
                counts.in_reply_to,
                counts.subordinate,
                counts.subordinate_on_page,
                counts.subordinate_states_one,
                counts.disagreeing,
                counts.reply,
                counts.reply_with_popup
            ));
        }
        total.absorb(&counts);
    }

    println!("{opened} document(s) opened, {} annotation(s)", total.seen);
    println!("  {} state Table 172's /IRT", total.in_reply_to);
    println!(
        "  {} of those state /RT /Group, {} naming a primary on the same page",
        total.subordinate, total.subordinate_on_page
    );
    println!(
        "  {} subordinate(s) state a group attribute of their own, {} entr(ies) disagreeing with \
         the primary",
        total.subordinate_states_one, total.disagreeing
    );
    let breakdown: Vec<String> = GROUP_ATTRIBUTES
        .iter()
        .zip(total.by_key)
        .filter(|(_, count)| *count > 0)
        .map(|(key, count)| format!("/{key} {count}"))
        .collect();
    println!("    disagreeing by key: {}", breakdown.join(", "));
    println!(
        "  {} subordinate(s) state no /AP, so their /C marks the page; {} popup window(s) hang \
         off a subordinate",
        total.subordinate_synthesised, total.popup_of_subordinate
    );
    println!(
        "  {} repl(ies) — /RT /R or absent — {} of which name a /Popup of their own, {} whose \
         thread head names one; deepest chain {} hop(s)",
        total.reply, total.reply_with_popup, total.reply_head_with_popup, total.deepest_thread
    );
    for line in &lines {
        println!("{line}");
    }
}

/// The annotation a chain of `/IRT`s ends at, and how many hops away it is.
///
/// Table 172 bounds the relationship to one page — "[b]oth annotations shall be on the same page
/// of the document" — but says nothing that stops a file writing a cycle, so the walk is bounded
/// and returns whatever it is standing on when the bound runs out.
fn thread_head<'a>(
    document: &'a Document,
    annotation: &'a pdf_syntax::Dictionary,
) -> (std::borrow::Cow<'a, pdf_syntax::Dictionary>, usize) {
    let mut node = std::borrow::Cow::Borrowed(annotation);
    for depth in 0..MAX_THREAD {
        let grouped = document
            .get_key(&node, "RT")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Group");
        // Only a *reply* is climbed: a `/RT /Group` subordinate already shares the primary's
        // `/Popup` as a group attribute, so the head of a thread is the first annotation that is
        // not a reply, and its window is whichever one §12.5.6.2 says its entries come from.
        let next = document.get_key(&node, "IRT");
        let Some(next) = next.as_dict() else {
            return (node, depth);
        };
        if grouped {
            return (std::borrow::Cow::Owned(next.clone()), depth);
        }
        node = std::borrow::Cow::Owned(next.clone());
    }
    (node, MAX_THREAD)
}

/// How far a reply chain is followed before the file is treated as having written a cycle.
const MAX_THREAD: usize = 64;

/// Walks every annotation on every page of one document.
fn document_counts(document: &Document) -> Counts {
    let mut counts = Counts::default();
    let pages = pdf_model::Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let entry = document.get_key(&page.dict, "Annots");
        let Some(list) = entry.as_array() else {
            continue;
        };
        let on_page: Vec<pdf_syntax::ObjectId> =
            list.iter().filter_map(Object::as_reference).collect();
        for item in list {
            let object = document.resolve(item);
            let Some(annotation) = object.as_dict() else {
                continue;
            };
            counts.seen = counts.seen.saturating_add(1);
            let irt = document.get_key(annotation, "IRT");
            let Some(primary) = irt.as_dict() else {
                continue;
            };
            counts.in_reply_to = counts.in_reply_to.saturating_add(1);
            let grouped = document
                .get_key(annotation, "RT")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Group");
            if !grouped {
                counts.reply = counts.reply.saturating_add(1);
                if annotation.get("Popup").is_some() {
                    counts.reply_with_popup = counts.reply_with_popup.saturating_add(1);
                }
                let (head, depth) = thread_head(document, annotation);
                counts.deepest_thread = counts.deepest_thread.max(depth);
                if head.get("Popup").is_some() {
                    counts.reply_head_with_popup = counts.reply_head_with_popup.saturating_add(1);
                }
                continue;
            }
            counts.subordinate = counts.subordinate.saturating_add(1);
            if annotation.get("AP").is_none() {
                counts.subordinate_synthesised = counts.subordinate_synthesised.saturating_add(1);
            }
            if annotation.get("Popup").is_some() {
                counts.popup_of_subordinate = counts.popup_of_subordinate.saturating_add(1);
            }
            if annotation
                .get("IRT")
                .and_then(Object::as_reference)
                .is_some_and(|id| on_page.contains(&id))
            {
                counts.subordinate_on_page = counts.subordinate_on_page.saturating_add(1);
            }
            let mut states_one = false;
            for key in GROUP_ATTRIBUTES {
                let own = annotation.get(key);
                if own.is_none() {
                    continue;
                }
                states_one = true;
                if own != primary.get(key) {
                    counts.disagreeing = counts.disagreeing.saturating_add(1);
                    if let Some(slot) = GROUP_ATTRIBUTES
                        .iter()
                        .position(|name| *name == key)
                        .and_then(|at| counts.by_key.get_mut(at))
                    {
                        *slot = slot.saturating_add(1);
                    }
                }
            }
            if states_one {
                counts.subordinate_states_one = counts.subordinate_states_one.saturating_add(1);
            }
        }
    }
    counts
}
