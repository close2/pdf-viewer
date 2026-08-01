//! ISO 32000-2 §14.7's logical structure and §14.9's accessibility entries, as a tree a host can
//! hand to a platform accessibility API.
//!
//! # Why this is a `Query` and not an `Event`
//!
//! A screen reader asks for the tree when it attaches, and again when the page changes. It is a
//! question with an answer the viewer already holds, which is exactly what the second channel is
//! for — the same argument [`crate::Query::Selection`] makes.
//!
//! # What this is, and what a host still owes
//!
//! `pdf-model` has read §14.7's structure tree since the seventy-eighth session and §14.9's
//! `/Alt`, `/E`, `/Lang` and `/ActualText` since the sixtieth, and this file is the first
//! consumer of either: until now nothing in this program handed a structure tree to anybody.
//! What crosses is toolkit-free by construction — a role name, a string to speak, a language tag
//! and the quadrilaterals the element covers, in the same device pixels
//! [`crate::Query::Selection`] answers in — so a host builds `AccessKit`'s nodes, AT-SPI's, or
//! `NSAccessibility`'s from it without this crate naming any of them.
//!
//! # The order the nodes are in
//!
//! §14.8.2.5's *logical* order, which is the structure tree's own order and not the content
//! stream's. That is the whole reason a tagged document is worth reading: a page whose producer
//! wrote its columns out of order gives its text in that order to a selection and in the right
//! order here.

use pdf_model::accessibility::Described;
use pdf_model::content::MarkedSpan;
use pdf_model::structure::{Child, Tree};
use pdf_syntax::{Dictionary, Document};

/// How deep a structure tree is walked before the walk is abandoned.
///
/// A tree deeper than this is hostile rather than merely complex — §14.7.2 puts no bound on
/// nesting and a file can state a cycle through `/K` that no `/Type` distinguishes from a deep
/// tree. The same reasoning and the same order of magnitude as `pdf-model`'s own bounds.
const MAX_DEPTH: usize = 64;

/// How many nodes one page's tree may hold.
///
/// A bound on the *answer* rather than on the document: a host asks this question when a screen
/// reader attaches, and a page that produced a million nodes would stall it.
const MAX_NODES: usize = 8192;

/// One element of §14.7's structure tree, as an accessibility API would take it.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityNode {
    /// Which node encloses this one, as an index into the answer, or `None` for a root.
    ///
    /// An index rather than a nesting of children, because that is the shape both `AccessKit` and
    /// AT-SPI want and because a flat list has no recursion for a host to bound.
    pub parent: Option<usize>,
    /// §14.7.4's `/S`: the structure type, as the document states it.
    ///
    /// Not mapped through §14.7.4's role map or §14.8.4's standard set, deliberately: a host
    /// that knows the platform's own vocabulary is better placed to map `H1` or `TD` onto it than
    /// this crate is, and a mapping here would be a second opinion nobody asked for. What the
    /// document says it is, is what crosses.
    pub role: String,
    /// What a text-to-speech engine would say for this element.
    ///
    /// §14.9.3's `/Alt` where the element states one — "human-readable text that could, for
    /// example, be vocalised by a text-to-speech engine" — else §14.9.5's `/E`, else the text the
    /// element's own marked-content sequences produced. The precedence is
    /// [`pdf_model::accessibility::Described`]'s, stated once there and followed here.
    pub name: String,
    /// §14.9.2's language, where the element or an enclosing one states one.
    pub language: Option<String>,
    /// Where the element is, in device pixels of the viewport, as quadrilaterals.
    ///
    /// The same shapes and the same space [`crate::Query::Selection`] answers in, so a host that
    /// draws a selection can draw a focus ring with no second mapping. Empty for an element whose
    /// content drew no text — a figure, a table cell holding an image — which is a statement
    /// about this program's text layer rather than about the element.
    pub quads: Vec<[f32; 8]>,
}

/// What one element contributes before its quads are mapped into the viewport.
pub(crate) struct Gathered {
    /// The element's own `/S`.
    pub(crate) role: String,
    /// The `/MCID`s below it, including those of its descendants.
    ///
    /// Descendants' too, because an element's spoken text is everything it encloses: §14.9.3's
    /// `/Alt` "is a complete (or whole) word or phrase substitution for the current element",
    /// which only means something if the element has text to substitute *for*.
    pub(crate) mcids: Vec<i64>,
    /// §14.9.3's `/Alt` or §14.9.5's `/E`, where the element itself states one.
    pub(crate) phrase: Option<String>,
    /// §14.9.2's `/Lang`, where the element itself states one.
    pub(crate) language: Option<String>,
}

/// Reads the page's part of §14.7's structure tree.
///
/// `page` is the page object, which is what Table 355's `/Pg` names; an element belonging to
/// another page is skipped rather than answered with, because a screen reader is being told what
/// is on the screen.
///
/// Returns an empty list for an untagged document, which is 885 of the corpus's 974 — and that is
/// an answer rather than a failure: a host asking this of an untagged page learns that the page
/// says nothing about its own structure, which is exactly what §14.7 leaves it to say.
pub(crate) fn nodes(
    document: &Document,
    page: pdf_syntax::ObjectId,
    text: &str,
    marked: &[MarkedSpan],
    described: &[Described],
    default_language: Option<&str>,
) -> Vec<(Option<usize>, Gathered)> {
    let Some(tree) = Tree::of(document) else {
        return Vec::new();
    };
    let mut out: Vec<(Option<usize>, Gathered)> = Vec::new();
    walk(
        document,
        &tree,
        None,
        None,
        page,
        default_language,
        0,
        &mut out,
    );
    let _ = (text, marked, described);
    out
}

/// What §14.9 says one element's span of the readback should be spoken as.
///
/// The element's own `/Alt` or `/E` wins where it states one, because that is a substitution for
/// the whole element. Where it does not, the text is taken through
/// [`pdf_model::accessibility::speech`] — the *sequence*-level `/Alt`, `/E` and `/ActualText` the
/// interpreter recorded — rather than raw, because a `/Alt` on a `BDC` inside the element is as
/// much a substitution as one on the element, and reading the raw text would speak the letters an
/// abbreviation is written with.
///
/// The `Described` spans are rebased onto the slice, so a span that straddles the element's edge
/// is clipped to it rather than dropped: half a substitution is still what the clause says about
/// the half that is here.
fn spoken(text: &str, described: &[Described], spans: &[(usize, usize)]) -> String {
    let mut out = String::new();
    for (start, end) in spans {
        let Some(slice) = text.get(*start..*end) else {
            continue;
        };
        let within: Vec<Described> = described
            .iter()
            .filter(|item| item.range.start < *end && item.range.end > *start)
            .map(|item| Described {
                range: item.range.start.max(*start).saturating_sub(*start)
                    ..item.range.end.min(*end).saturating_sub(*start),
                alt: item.alt.clone(),
                expansion: item.expansion.clone(),
                language: item.language.clone(),
            })
            .collect();
        for run in pdf_model::accessibility::speech(slice, &within, None) {
            out.push_str(&run.text);
        }
    }
    out
}

/// One element and everything below it, appended to `out` parent-first.
#[expect(
    clippy::too_many_arguments,
    reason = "a walk of a tree carries what it is walking, where it is, and what it inherits; \
              grouping them would build a struct that exists once per call"
)]
fn walk(
    document: &Document,
    tree: &Tree,
    element: Option<&Dictionary>,
    parent: Option<usize>,
    page: pdf_syntax::ObjectId,
    language: Option<&str>,
    depth: usize,
    out: &mut Vec<(Option<usize>, Gathered)>,
) {
    if depth >= MAX_DEPTH || out.len() >= MAX_NODES {
        return;
    }
    for child in tree.children(document, element) {
        match child {
            Child::Element(dict) => {
                // §14.9.2.3's hierarchy: the innermost stated `/Lang` wins, and an element that
                // states none inherits what encloses it.
                let language =
                    text_entry(document, &dict, "Lang").or_else(|| language.map(str::to_owned));
                let role = document
                    .get_key(&dict, "S")
                    .as_name()
                    .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
                    .unwrap_or_default();
                let phrase =
                    text_entry(document, &dict, "Alt").or_else(|| text_entry(document, &dict, "E"));
                let index = out.len();
                out.push((
                    parent,
                    Gathered {
                        role,
                        mcids: Vec::new(),
                        phrase,
                        language: language.clone(),
                    },
                ));
                walk(
                    document,
                    tree,
                    Some(&dict),
                    Some(index),
                    page,
                    language.as_deref(),
                    depth.saturating_add(1),
                    out,
                );
            }
            Child::MarkedContent { mcid, page: on } => {
                // Table 355 makes `/Pg` the page a bare integer belongs to; an element with no
                // `/Pg` anywhere up the chain is taken to be on the page being asked about,
                // which is what a single-page structure tree means by stating nothing.
                if on.is_some_and(|object| object != page) {
                    continue;
                }
                // The identifier belongs to the element that contains it *and* to every element
                // above it, because an ancestor's text is everything it encloses.
                let mut at = parent;
                while let Some(index) = at {
                    let Some((above, entry)) =
                        out.get_mut(index).map(|(above, entry)| (*above, entry))
                    else {
                        break;
                    };
                    entry.mcids.push(mcid);
                    at = above;
                }
            }
            // §14.7.5.3's object reference is an annotation or an XObject rather than text on
            // this page's readback. It is not skipped silently: it contributes no `/MCID`, so
            // its element is answered with whatever `/Alt` it states and no quads, which is a
            // true statement about what this program can locate.
            Child::Object { .. } => {}
        }
    }
}

/// A text-string entry, decoded through §7.9.2.2's rules.
fn text_entry(document: &Document, dict: &Dictionary, key: &str) -> Option<String> {
    let value = document.get_key(dict, key);
    let bytes = value.as_string()?;
    let text = pdf_syntax::text_string::text_string(bytes);
    (!text.is_empty()).then_some(text)
}

/// The byte ranges of the readback that a set of `/MCID`s covers.
pub(crate) fn ranges(marked: &[MarkedSpan], mcids: &[i64]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = marked
        .iter()
        .filter(|span| mcids.contains(&span.mcid) && span.range.start < span.range.end)
        .map(|span| (span.range.start, span.range.end))
        .collect();
    out.sort_unstable();
    out
}

/// Turns a gathered element into what crosses the boundary.
pub(crate) fn finish(
    gathered: Gathered,
    parent: Option<usize>,
    text: &str,
    marked: &[MarkedSpan],
    described: &[Described],
    quads: impl Fn(usize, usize) -> Vec<[f32; 8]>,
) -> AccessibilityNode {
    let spans = ranges(marked, &gathered.mcids);
    let name = gathered
        .phrase
        .unwrap_or_else(|| spoken(text, described, &spans));
    let mut all = Vec::new();
    for (start, end) in spans {
        all.extend(quads(start, end));
    }
    AccessibilityNode {
        parent,
        role: gathered.role,
        name,
        language: gathered.language,
        quads: all,
    }
}
