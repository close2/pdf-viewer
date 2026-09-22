//! ISO 32000-2 §12.5.6.14: the popup window a markup annotation opens.
//!
//! A popup is the one annotation subtype that is never part of the page's own rendering, and the
//! clause says why in its first sentence:
//!
//! > A popup annotation ( PDF 1.3 ) displays text in a popup window for entry and editing. It
//! > shall not appear alone but is associated with a markup annotation, its parent annotation,
//! > and shall be used for editing the parent's text. It shall have no appearance stream or
//! > associated actions of its own and shall be identified by the Popup entry in the parent's
//! > annotation dictionary
//!
//! No appearance stream, so nothing in `crate::appearance` can construct one and
//! `crate::annotation::decided` answers `Nothing` for the subtype; a *window* over the page is a
//! host's to draw, in the same sense §12.3.3's outline is a host's to list. What this module does
//! is read one — where it is, whether it opens with the page, and the four entries §12.5.6.2 and
//! Table 166 put in its title bar and its body — and hand it over.
//!
//! **The text is the parent's, and Table 186 makes that a `shall`**: "the parent annotation's
//! Contents , M , C , and T entries … shall override those of the popup annotation itself". The
//! popup's own four are the fallback the clause's NOTE 3 describes, for a popup with no parent:
//! "[t]he Contents entry for a popup annotation is relevant only if it has no parent; in that
//! case, it represents the text of the annotation".
//!
//! **Measured over the corpus** (974 documents, first 50 pages each): 45 documents state a popup,
//! 128 popups between them, 7 of which Table 186's `/Open` opens with the page, on 2 documents;
//! 79 have text to show and 4 have no `/Parent`. **No `/Popup` reference names an annotation the
//! page's `/Annots` does not also list**, which is why this walks `/Annots` and does not chase
//! Table 172's entry from the other end: the two routes reach the same 128.
//!
//! **And the parent may not be the annotation whose entries those are**, since the
//! four-hundred-and-eightieth session: §12.5.6.2 makes all four of Table 186's overrides *group
//! attributes*, so where the parent is a subordinate in a group the primary's apply and "the
//! corresponding entries in the subordinate annotations shall be ignored". `crate::markup` is that
//! sentence, and 213 of ISO 32000-2's own 2552 windows hang off a subordinate.
//!
//! **A second entry opens the same window and is §12.5.6.4's**, read here since the
//! four-hundred-and-fifty-ninth session; `opens_with_the_page` has the argument. The corpus
//! cannot rank it — `examples/open_annotation_census` finds 28 text annotations, exactly one
//! stating Table 175's `/Open true`, and that one's popup already states its own — so the pair of
//! fixtures below differ only in the rule, which is trap 8's shape.

use pdf_syntax::{Dictionary, Document, ObjectId};

use crate::Page;

/// One popup window, read.
///
/// Geometry in default user space and text as §7.9.2.2 decodes it: this is what the *document*
/// says, and every decision about pixels — where a window this size goes on a screen, what its
/// title bar is set in — belongs to whatever draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct Popup {
    /// The popup annotation itself.
    pub annotation: ObjectId,
    /// Table 186's `/Parent`: the markup annotation this window belongs to.
    ///
    /// `None` for the case the clause's NOTE 3 names, where the popup's own entries are the
    /// answer. §12.5.6.2 calls that file wrong — "[t]he popup annotation type shall not appear
    /// by itself" — and the note says what to do with it anyway, so it is read rather than
    /// refused. Four of the corpus's 128 popups are like this.
    pub parent: Option<ObjectId>,
    /// Table 166's `/Rect`, normalised to `[x0, y0, x1, y1]` with `x0 <= x1` and `y0 <= y1`.
    pub rect: [f32; 4],
    /// Whether the window opens with the page, which **two** entries can each say.
    ///
    /// Table 186's `/Open` is the popup's own — "whether the popup annotation shall initially be
    /// displayed open" — and Table 175's is the *text annotation's*: "[a] flag specifying whether
    /// the annotation shall initially be displayed open". §12.5.6.4 is what makes the second one
    /// a statement about this window rather than about an icon:
    ///
    /// > When closed, the annotation shall appear as an icon; when open, it shall display a popup
    /// > window containing the text of the note in a font and size chosen by the interactive PDF
    /// > processor.
    ///
    /// See this module's `opens_with_the_page` for why that is a disjunction rather than a
    /// precedence.
    ///
    /// *Initially*, which is why a viewer holds the current state elsewhere: this is the file's
    /// opinion about the moment the page appears, not a fact that survives a click.
    pub open: bool,
    /// §12.5.6.2's `/T`, from the parent where there is one.
    ///
    /// > The text label that shall be displayed in the title bar of the annotation's popup window
    /// > when open and active. This entry shall identify the user who added the annotation.
    pub title: Option<String>,
    /// Table 166's `/Contents`, from the parent where there is one: the text in the window.
    pub text: Option<String>,
    /// Table 166's `/M`, **as the file spells it**.
    ///
    /// Not a `pdf_syntax::Date`, and that is the clause rather than laziness: "[t]he format
    /// should be a date string as described in 7.9.4, "Dates" but interactive PDF processors
    /// shall accept and display a string in any format". A processor that parsed this and
    /// dropped what would not parse would break the `shall`; one that parses it *and* keeps the
    /// original can obey both, so the string is what crosses and [`Popup::modified_date`] is the
    /// half that is a date.
    pub modified: Option<String>,
    /// Table 166's `/C`, converted to RGB — "[t]he title bar of the annotation's popup window".
    ///
    /// `None` for the value the table gives an empty array: "0 No colour; transparent".
    pub colour: Option<pdf_render::Color>,
    /// §12.5.6.2's thread: every reply whose own window this one has absorbed, deepest last.
    ///
    /// Empty for the overwhelming majority of windows, which nobody has replied to. See
    /// [`popups`] for the sentence that makes this a list rather than a second window.
    pub replies: Vec<Comment>,
}

/// One reply in a [`Popup`]'s thread — Table 172's `/RT` `R`, displayed where it belongs.
///
/// ISO 32000-2 §12.5.6.2, Table 172, `/RT`:
///
/// > R The annotation is considered a reply to the annotation specified by IRT . Interactive PDF
/// > processors shall not display replies to an annotation individually but together in the form
/// > of threaded comments.
///
/// So a reply's own popup window is not a window: it is a comment inside the window of what it
/// replies to, and this is that comment. The three text entries are the same three a [`Popup`]
/// carries and are read through the same group rule.
#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    /// The popup annotation whose window this reply would otherwise have opened.
    ///
    /// Kept so that a host can name what it is showing and so that §12.5.1's activation of the
    /// reply itself has something to match; nothing places it, because a comment has no rectangle.
    pub annotation: ObjectId,
    /// The markup annotation whose text this is — Table 186's `/Parent` of [`Self::annotation`].
    pub parent: Option<ObjectId>,
    /// How many `/IRT` hops separate this reply from the window hosting the thread.
    ///
    /// 1 for a reply to the hosting annotation, 2 for a reply to that reply, and so on. The
    /// clause calls the result *threaded*, and a thread with no depth is a list.
    pub depth: usize,
    /// §12.5.6.2's `/T`: who wrote the reply.
    pub title: Option<String>,
    /// Table 166's `/Contents`, or Table 172's `/RC` behind it: what the reply says.
    pub text: Option<String>,
    /// Table 166's `/M`, as the file spells it — [`Popup::modified`]'s rule, for the same reason.
    pub modified: Option<String>,
}

impl Popup {
    /// [`Popup::modified`] where it is a date §7.9.4 states.
    ///
    /// A host that wants to reformat the timestamp for a locale asks this; one that wants to obey
    /// Table 166's "shall accept and display a string in any format" shows the other.
    #[must_use]
    pub fn modified_date(&self) -> Option<pdf_syntax::Date> {
        pdf_syntax::Date::parse(self.modified.as_ref()?)
    }

    /// How wide the window is, in default user space units.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.rect[2] - self.rect[0]
    }

    /// How tall the window is, in default user space units.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.rect[3] - self.rect[1]
    }
}

/// Every popup window the page states, in the `/Annots` array's own order.
///
/// §12.5.3's flags are applied first: a popup whose `/F` sets `Hidden` or `NoView` is not
/// displayed, by the same reading `crate::annotation` gives every other annotation, and
/// §12.6.4.11's hide action reaches it through the same [`crate::view::ViewState`].
///
/// **`/F` is read from the popup and not from its parent**, and that is a decision this corpus
/// argues about: `pr7352.pdf` exists to assert that a popup "should fallback to inherit the
/// annotation flags of the parent annotation", which is another reader's rule. Table 186 lists
/// exactly four entries the parent overrides — `Contents`, `M`, `C`, `T` — and `/F` is not one of
/// them; a table that enumerates four is a table that has said something about the fifth.
#[must_use]
pub fn popups(document: &Document, page: &Page, view: &crate::view::ViewState) -> Vec<Popup> {
    let annotations = document.get_key(&page.dict, "Annots");
    let Some(annotations) = annotations.as_array() else {
        return Vec::new();
    };
    // Two passes, because the second needs to know which windows the page actually shows: the
    // first reads every window the file states and the second threads them.
    let mut read_windows: Vec<(Popup, Option<ObjectId>, usize)> = Vec::new();
    for entry in annotations {
        let Some(id) = entry.as_reference() else {
            continue;
        };
        let resolved = document.resolve(entry);
        let Some(dict) = resolved.as_dict() else {
            continue;
        };
        if document
            .get_key(dict, "Subtype")
            .as_name()
            .map(pdf_syntax::Name::as_bytes)
            != Some(b"Popup")
        {
            continue;
        }
        if !crate::annotation::displayed(document, dict, view.annotation(id)) {
            continue;
        }
        let Some(popup) = read(document, id, dict) else {
            continue;
        };
        // The annotation whose text the window shows — Table 186's `/Parent`, or the window
        // itself for the parentless case this clause's NOTE 3 describes.
        let owner = document.get_key(dict, "Parent");
        let owner = owner.as_dict().unwrap_or(dict);
        let (host, depth) = thread_window(document, owner);
        read_windows.push((popup, host, depth));
    }

    let shown: Vec<ObjectId> = read_windows
        .iter()
        .map(|(popup, _, _)| popup.annotation)
        .collect();
    let mut out: Vec<Popup> = Vec::new();
    let mut folded: Vec<(ObjectId, Comment, bool)> = Vec::new();
    for (popup, host, depth) in read_windows {
        // A reply whose host is not one of this page's shown windows has nothing to be displayed
        // *together with*, so it keeps its own: the clause forbids showing replies separately
        // from what they reply to, not showing them at all. A window is never its own host
        // either, which is what a file whose `/IRT` chain loops back produces — every annotation
        // in a cycle is its own ancestor, and folding one into itself would lose it.
        match host.filter(|host| depth > 0 && *host != popup.annotation && shown.contains(host)) {
            Some(host) => folded.push((
                host,
                Comment {
                    annotation: popup.annotation,
                    parent: popup.parent,
                    depth,
                    title: popup.title,
                    text: popup.text,
                    modified: popup.modified,
                },
                popup.open,
            )),
            None => out.push(popup),
        }
    }
    // Deepest last, `/Annots` order within a depth — which `sort_by_key` keeps, being stable.
    folded.sort_by_key(|(_, comment, _)| comment.depth);
    for (host, comment, open) in folded {
        let Some(window) = out.iter_mut().find(|window| window.annotation == host) else {
            continue;
        };
        // Table 186's `/Open` of a folded reply opens the thread's window, by
        // `opens_with_the_page`'s reading: each entry states a condition under which a window is
        // open and none states one under which it is closed.
        window.open = window.open || open;
        window.replies.push(comment);
    }
    out
}

/// How far a chain of Table 172 `/IRT` replies is followed before a file is read as cyclic.
///
/// The clause bounds a thread to one page — "[b]oth annotations shall be on the same page of the
/// document" — and states no depth. ISO 32000-2's own PDF is the deepest population measured,
/// at four hops (`examples/annotation_group_census`); this is far above it and exists so that a
/// file writing a cycle terminates rather than to express a limit the standard has.
const MAX_THREAD: usize = 64;

/// The window §12.5.6.2 displays this annotation's text in, and how deep in the thread it sits.
///
/// ISO 32000-2 §12.5.6.2, Table 172, the `/RT` value `R`:
///
/// > Interactive PDF processors shall not display replies to an annotation individually but
/// > together in the form of threaded comments.
///
/// `R` is also `/RT`'s default, so an annotation with an `/IRT` and no `/RT` is a reply. This
/// climbs that chain and answers the window of the **highest** ancestor that states one, with the
/// number of hops to it: 0 for an annotation displayed in its own window, which is every
/// annotation in almost every file.
///
/// **The highest ancestor that states a window, rather than the head of the chain**, and the
/// count is why: of ISO 32000-2's own 1752 replies, every one names a `/Popup` of its own and
/// only 1401 have a chain head that names one. Hanging a thread on the head alone would drop the
/// text of the other 351 — and a reply whose ancestors state no window has nothing to be shown
/// *together with*, which is the case this rule leaves alone rather than the rule it breaks.
///
/// `/Popup` is one of the clause's group attributes, so each step reads it through
/// [`crate::markup::group_source`]: a `/RT /Group` subordinate's window is the primary's, and the
/// climb stops there because a group member is not a reply.
fn thread_window(document: &Document, annotation: &Dictionary) -> (Option<ObjectId>, usize) {
    let window_of = |dict: &Dictionary| {
        crate::markup::group_source(document, dict)
            .get("Popup")
            .and_then(pdf_syntax::Object::as_reference)
    };
    let mut node = annotation.clone();
    let mut window = window_of(&node);
    let mut found_at = 0;
    for hop in 1..=MAX_THREAD {
        if !is_reply(document, &node) {
            break;
        }
        let Some(next) = document.get_key(&node, "IRT").as_dict().cloned() else {
            break;
        };
        node = next;
        if let Some(above) = window_of(&node) {
            window = Some(above);
            found_at = hop;
        }
    }
    (window, found_at)
}

/// Whether this annotation is Table 172's *reply* rather than a group's subordinate.
///
/// The entry that says which is `/RT`, "meaningful only if IRT is present", whose two values are
/// `R` — "[t]he annotation is considered a reply to the annotation specified by IRT" — and
/// `Group`, with "Default value: R ". So an `/IRT` with no `/RT` is a reply, which is what makes
/// the default load-bearing rather than cosmetic.
fn is_reply(document: &Document, annotation: &Dictionary) -> bool {
    annotation.get("IRT").is_some()
        && document
            .get_key(annotation, "RT")
            .as_name()
            .map(pdf_syntax::Name::as_bytes)
            != Some(b"Group")
}

/// Table 172's `/Popup`: the window this markup annotation opens, where it states one.
///
/// "An indirect reference to a popup annotation for entering or editing the text associated with
/// this annotation." An indirect reference, so an inline dictionary is not one — and the caller
/// wants the object anyway, because §12.5.1's activation names an annotation rather than a
/// dictionary.
///
/// **`/Popup` is one of §12.5.6.2's group attributes**, so a subordinate annotation's own is
/// ignored and the window a click on it exhibits is the group's — the primary's. That is the
/// clause's own sentence read literally: a group is "a set of annotations … grouped so that they
/// function as a single unit when a user interacts with them", and one window for the unit is what
/// that means for the one interaction §12.5.1 defines.
///
/// **A watermark has none**, whatever its dictionary says: §12.5.6.22's "Watermark annotations
/// shall have no popup window nor other interactive elements" is a sentence about the subtype,
/// and Table 171 makes it no markup annotation either, so a `/Popup` written on one names a
/// window this clause forbids. `None`, as for an annotation that states no entry (ADR 1057).
///
/// **And a reply's window is the thread's**, since §12.5.6.2's other `shall` was implemented:
/// "[i]nteractive PDF processors shall not display replies to an annotation individually but
/// together in the form of threaded comments", so activating a reply exhibits the one window that
/// thread is shown in rather than a second window beside it. [`thread_window`] is that climb and
/// is what [`popups`] folds by, so the two answers cannot disagree.
#[must_use]
pub fn popup_of(document: &Document, annotation: &Dictionary) -> Option<ObjectId> {
    if crate::annotation::is_watermark(document, annotation) {
        return None;
    }
    thread_window(document, annotation).0
}

/// Reads one popup dictionary, or `None` where its `/Rect` states no rectangle — or where its
/// parent is a watermark, which §12.5.6.22 says "shall have no popup window"; see [`popup_of`].
///
/// A window with no rectangle has nowhere to be, and Table 166 makes `/Rect` required — so that
/// is the one refusal here that is the file's.
fn read(document: &Document, id: ObjectId, dict: &Dictionary) -> Option<Popup> {
    let parent = dict
        .get("Parent")
        .and_then(pdf_syntax::Object::as_reference);
    let resolved = document.get_key(dict, "Parent");
    if resolved
        .as_dict()
        .is_some_and(|parent| crate::annotation::is_watermark(document, parent))
    {
        return None;
    }
    let source = resolved.as_dict().unwrap_or(dict);
    // **Two clauses compose here, and the second was unread until the four-hundred-and-eightieth
    // session.** Table 186 makes the parent's `Contents`, `M`, `C` and `T` override the popup's;
    // §12.5.6.2 makes all four *group attributes*, so where the parent is a subordinate in a group
    // "the corresponding entries in the subordinate annotations shall be ignored" and the
    // primary's are what this window shows. 213 of ISO 32000-2's own popups hang off a
    // subordinate, and the erratum text a reader is looking for is in the primary.
    let source = crate::markup::group_source(document, source);
    let rect = rectangle(document, dict)?;
    Some(Popup {
        annotation: id,
        parent,
        rect,
        open: opens_with_the_page(document, dict, resolved.as_dict()),
        title: text(document, &source, "T"),
        // Table 166's `/Contents` first, and Table 172's `/RC` only where there is none: NOTE 1
        // makes the two "textually equivalent" where both are present, and the plain string is
        // the one this crate can hand over without reading a specification it does not have.
        text: text(document, &source, "Contents").or_else(|| rich_text(document, &source)),
        modified: text(document, &source, "M"),
        colour: colour(document, &source),
        replies: Vec::new(),
    })
}

/// Whether the file asks for this window to be open when the page appears.
///
/// **Two entries can each say so, and neither says the opposite.** Table 186 gives the popup its
/// own `/Open`, "whether the popup annotation shall initially be displayed open"; Table 175 gives
/// a *text* annotation an `/Open`, "whether the annotation shall initially be displayed open", and
/// §12.5.6.4 says what an open text annotation is: "when open, it shall display a popup window
/// containing the text of the note". Each entry defaults to `false`, so each states a condition
/// under which the window is open and neither states one under which it is closed — which makes
/// this a disjunction rather than a precedence, and means there is no conflict for Table 186's
/// four-entry override list to have settled.
///
/// **Table 175's half was read nowhere until the four-hundred-and-fifty-ninth session**, on a
/// doc comment in `crate::appearance` saying `/Open` was not read because this program "draws no
/// popup for any subtype" — true when it was written and false since the three-hundred-and-twelfth
/// session, which is `doc/todo/01`'s capability shape. A file saying its sticky note starts open
/// showed no window and said nothing about it. ADR 0294.
///
/// **Only a text annotation's `/Open` counts**, because Table 175 is the only table outside
/// Table 186 that gives an annotation the entry at all. §12.5.6.7, §12.5.6.8, §12.5.6.9,
/// §12.5.6.10 and §12.5.6.13 each say their annotation displays a popup window "when opened" and
/// none of them states an entry that opens it, so an `/Open` on one of those subtypes is a key the
/// standard does not define — of which the corpus has none, counted by
/// `examples/open_annotation_census`.
/// **The parent's half is a group attribute and the subtype is not**, which is why the two are
/// read from different dictionaries: §12.5.6.2 puts `Open` on its list and says nothing about
/// `Subtype`, so a subordinate text annotation is still a text annotation and the entry that opens
/// its group's window is the primary's.
fn opens_with_the_page(
    document: &Document,
    popup: &Dictionary,
    parent: Option<&Dictionary>,
) -> bool {
    if is_open(document, popup) {
        return true;
    }
    parent.is_some_and(|parent| {
        document
            .get_key(parent, "Subtype")
            .as_name()
            .is_some_and(|subtype| subtype.as_bytes() == b"Text")
            && is_open(document, &crate::markup::group_source(document, parent))
    })
}

/// Whether a dictionary states `/Open true`, which both tables spell the same way.
fn is_open(document: &Document, dict: &Dictionary) -> bool {
    document.get_key(dict, "Open") == pdf_syntax::Object::Boolean(true)
}

/// Table 172's `/RC`, as the characters it carries — §12.5.6.2:
///
/// > A rich text string (see Adobe XML Architecture, XML Forms Architecture (XFA) Specification,
/// > version 3.3 ) that shall be displayed in the popup window when the annotation is opened.
///
/// A `shall` about the popup window, and this program has had one since the
/// three-hundred-and-twelfth session — so `doc/todo/01`'s capability shape applies to it, and the
/// row that said these entries "reach a comments pane this program has no panel for" stopped
/// being true then.
///
/// # What is read, and what is deliberately not
///
/// **The characters, and none of the formatting.** The format is XFA's rich text, which is a
/// profile of XHTML, and `CLAUDE.md` excludes XFA — so nothing here interprets a `<span>`'s
/// style, a colour, a size or a face. What it takes is the element content, which is text the
/// clause requires displayed and which needs no specification this tree does not have; the
/// clause's own NOTE 1 says so from the other end, by making `/RC` and `/Contents` "textually
/// equivalent" where a file states both.
///
/// **A paragraph is a break.** §12.5.6.2 states the rule for the plain form — "[w]hen separating
/// text into paragraphs, a CARRIAGE RETURN (0Dh) shall be used" — and the rich form spells a
/// paragraph as an element, so a closing `</p>` and a `<br/>` become the newline the plain form
/// would have carried. Nothing else in the markup changes the text.
///
/// **18 corpus annotations state `/RC` and no `/Contents`** (`examples/markup_text_census`,
/// counted over every page of all 974 documents), which is 18 popups this program opened with
/// nothing in them. 71 state `/RC` at all and 197 state `/Contents`.
///
/// Returns `None` where the entry is absent, is neither a string nor a stream, does not parse as
/// XML, exceeds [`MAX_RICH_TEXT`], or carries no characters at all — a window with nothing to
/// show is the same answer as no entry, and a malformed one may not become a refusal of the
/// window itself.
///
/// # Two tables spell `/RC`, and this reads both
///
/// Table 177 states the entry a second time, on a free text annotation, and its sentence is a
/// different `shall` — §12.5.6.6:
///
/// > A rich text string (see Adobe XML Architecture, XML Forms Architecture (XFA) Specification,
/// > version 3.3 ) that shall be used to generate the appearance of the annotation.
///
/// The same NOTE says why the two cannot be one function's two callers by accident: "[a]s
/// freetext annotations do not have an open state this cannot apply to the popup window as
/// described for the RC key in "Table 172 - Additional entries in an annotation dictionary
/// specific to markup annotations"." So the *characters* are extracted identically and the
/// destination differs — a window here, the page in [`crate::appearance`] — which is why this is
/// `pub(crate)` rather than inlined (ADR 0224).
pub(crate) fn rich_text(document: &Document, dict: &Dictionary) -> Option<String> {
    let value = document.get_key(dict, "RC");
    let bytes: Vec<u8> = match &value {
        pdf_syntax::Object::String(bytes) => bytes.to_vec(),
        // Table 172 gives `/RC` as "text string or text stream", and §7.9.3's text stream is a
        // stream whose decoded bytes are a text string — so the same decoding follows either.
        pdf_syntax::Object::Stream(stream) => document.decoded_stream_data(stream)?.to_vec(),
        _ => return None,
    };
    if bytes.len() > MAX_RICH_TEXT {
        return None;
    }
    // A malformed packet keeps what was read: this is a window's text, and half a comment is
    // better than none of it. The alternative — refusing — would take a popup away over a
    // producer's stray ampersand.
    let read = rich_text_characters(&pdf_syntax::text_string(&bytes));
    let trimmed = read.text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// The character data of a rich text string, and what the walk over its markup found.
///
/// The two callers want the same characters and disagree about a malformed packet, so the walk
/// answers both questions and each decides for itself: [`rich_text`] keeps what it read, because
/// half a comment is better than none, and [`crate::appearance::rich_text_value`] takes a
/// field's value as markup only where the markup is whole, because a value that is not a rich
/// text string is a value to draw as it stands (ADR 1197).
pub(crate) struct RichTextCharacters {
    /// The element content, with a newline where a paragraph or a line break closed.
    pub(crate) text: String,
    /// Whether every token of the markup parsed.
    pub(crate) parsed_whole: bool,
    /// How many elements the markup opened.
    pub(crate) elements: usize,
}

/// Walks a rich text string's markup and takes its character data.
///
/// See [`rich_text`] for what is read and what is deliberately not; this is that walk, shared so
/// that an annotation's `/RC` and a field's rich text value cannot come apart.
pub(crate) fn rich_text_characters(markup: &str) -> RichTextCharacters {
    let mut read = RichTextCharacters {
        text: String::with_capacity(markup.len()),
        parsed_whole: true,
        elements: 0,
    };
    let mut breaks = 0_usize;
    for token in xmlparser::Tokenizer::from(markup) {
        let Ok(token) = token else {
            read.parsed_whole = false;
            break;
        };
        match token {
            xmlparser::Token::Text { text } | xmlparser::Token::Cdata { text, .. } => {
                crate::xmp::unescape(text.as_str(), &mut read.text);
            }
            xmlparser::Token::ElementStart { .. } => {
                read.elements = read.elements.saturating_add(1);
            }
            xmlparser::Token::ElementEnd { end, .. } => {
                let name = match end {
                    xmlparser::ElementEnd::Close(_, local) => local.as_str().to_owned(),
                    xmlparser::ElementEnd::Empty => String::new(),
                    xmlparser::ElementEnd::Open => continue,
                };
                if name.eq_ignore_ascii_case("p") || name.eq_ignore_ascii_case("br") {
                    breaks = breaks.saturating_add(1);
                    if breaks <= MAX_RICH_TEXT_BREAKS {
                        read.text.push('\n');
                    }
                }
            }
            _ => {}
        }
    }
    read
}

/// The most `/RC` markup this module will parse, in bytes.
///
/// A window's text, not a document: `xmp.rs`'s own packet budget is 8 MiB and this is two orders
/// below it because a popup a person reads is a paragraph or two. A larger one is left unread
/// rather than truncated, so what a window shows is always the whole of something.
const MAX_RICH_TEXT: usize = 64 << 10;

/// How many paragraph breaks the text above may carry.
///
/// Bounds the one place [`rich_text`] appends without consuming input: an element with no text is
/// still a `</p>`, so a file of nothing but empty paragraphs would otherwise grow the string
/// without bound in the tokenizer's own terms. The text itself is bounded by [`MAX_RICH_TEXT`].
const MAX_RICH_TEXT_BREAKS: usize = 4096;

/// A text string entry, §7.9.2.2's three encodings decoded.
fn text(document: &Document, dict: &Dictionary, key: &str) -> Option<String> {
    let value = document.get_key(dict, key);
    let bytes = value.as_string()?;
    let decoded = pdf_syntax::text_string(bytes);
    (!decoded.is_empty()).then_some(decoded)
}

/// Table 166's `/Rect`, normalised: "shall be two opposite corners", in either order.
fn rectangle(document: &Document, dict: &Dictionary) -> Option<[f32; 4]> {
    let value = document.get_key(dict, "Rect");
    let array = value.as_array()?;
    let mut corners = [0.0_f32; 4];
    if array.len() < 4 {
        return None;
    }
    for (slot, entry) in corners.iter_mut().zip(array) {
        let number = document.resolve(entry).as_number()?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "page geometry is f32 throughout this tree; see pdf_render::Point"
        )]
        {
            *slot = number as f32;
        }
        if !slot.is_finite() {
            return None;
        }
    }
    Some([
        corners[0].min(corners[2]),
        corners[1].min(corners[3]),
        corners[0].max(corners[2]),
        corners[1].max(corners[3]),
    ])
}

/// Table 166's `/C`, in the space its own length names. ISO 32000-2 §12.5.2:
///
/// > The number of array elements determines the colour space in which the colour shall be
/// > defined: 0 No colour; transparent 1 `DeviceGray` 3 `DeviceRGB` 4 `DeviceCMYK`
///
/// The same sentence `crate::appearance` reads for an icon's background and a link's border, and
/// the same conversion — `crate::colour::ColourSpace`, so a popup's title bar and a link's border
/// do not disagree about what `/C [0 0 0 1]` looks like.
fn colour(document: &Document, dict: &Dictionary) -> Option<pdf_render::Color> {
    let value = document.get_key(dict, "C");
    let array = value.as_array()?;
    let mut components = Vec::with_capacity(array.len());
    for entry in array {
        let number = document.resolve(entry).as_number()?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a colour component is a small number and is clamped here"
        )]
        {
            components.push((number as f32).clamp(0.0, 1.0));
        }
    }
    let space = match components.len() {
        1 => crate::colour::ColourSpace::Gray,
        3 => crate::colour::ColourSpace::Rgb,
        4 => crate::colour::ColourSpace::Cmyk,
        // "0 No colour; transparent", and a length the table does not name is not a colour it
        // describes — the same refusal `crate::appearance::colour` makes, without a report,
        // because nothing here is being drawn *wrong*: the window keeps the interface's colour.
        _ => return None,
    };
    Some(space.to_rgb(&components))
}

#[cfg(test)]
mod tests {
    use pdf_syntax::Document;

    use super::{popup_of, popups};

    /// A one-page document with the annotations the caller spells, as PDF bytes.
    fn document(annotations: &str, objects: &str) -> Document {
        let body = format!(
            "%PDF-1.7\n\
             1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n\
             2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
             3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [{annotations}] \
             >> endobj\n\
             {objects}\
             trailer << /Root 1 0 R /Size 32 >>\n"
        );
        Document::open(body.into_bytes()).expect("the fixture parses")
    }

    fn page(document: &Document) -> crate::Page {
        crate::Pages::new(document).get(0).expect("one page")
    }

    /// §12.5.6.22: "Watermark annotations shall have no popup window nor other interactive
    /// elements." A popup hanging off a watermark is not a window, and the watermark opens none;
    /// the same pair with a square for a parent is the control, since Table 171 makes a square a
    /// markup annotation and §12.5.6.8 gives it a window "[w]hen opened".
    #[test]
    fn a_watermark_has_no_popup_window_and_a_square_has_one() {
        let pair = |subtype: &str| {
            document(
                "4 0 R 5 0 R",
                &format!(
                    "4 0 obj << /Type /Annot /Subtype /{subtype} /Rect [10 10 30 30] /Popup 5 0 R \
                     /Contents (text) /T (author) >> endobj\n\
                     5 0 obj << /Type /Annot /Subtype /Popup /Rect [40 40 200 140] /Parent 4 0 R \
                     /Open true >> endobj\n"
                ),
            )
        };
        for (subtype, windows) in [("Watermark", 0), ("Square", 1)] {
            let document = pair(subtype);
            let view = crate::view::ViewState::of(&document);
            assert_eq!(
                popups(&document, &page(&document), &view).len(),
                windows,
                "{subtype}"
            );
            let parent = document.get(pdf_syntax::ObjectId::new(4, 0));
            let parent = parent.as_dict().expect("the parent");
            assert_eq!(
                popup_of(&document, parent).is_some(),
                windows == 1,
                "{subtype}"
            );
        }
    }

    #[test]
    fn the_parents_four_entries_override_the_popups_own() {
        // Table 186: "the parent annotation's Contents , M , C , and T entries … shall override
        // those of the popup annotation itself".
        let document = document(
            "4 0 R 5 0 R",
            "4 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /Popup 5 0 R \
             /Contents (from the parent) /T (the author) /M (D:20260805120000Z) /C [1 0 0] >> \
             endobj\n\
             5 0 obj << /Type /Annot /Subtype /Popup /Rect [40 40 200 140] /Parent 4 0 R \
             /Open true /Contents (from the popup) /T (nobody) >> endobj\n",
        );
        let view = crate::view::ViewState::of(&document);
        let popups = popups(&document, &page(&document), &view);
        assert_eq!(popups.len(), 1);
        let popup = &popups[0];
        assert_eq!(popup.text.as_deref(), Some("from the parent"));
        assert_eq!(popup.title.as_deref(), Some("the author"));
        assert_eq!(popup.modified.as_deref(), Some("D:20260805120000Z"));
        assert!(popup.open);
        // Exact: the fixture states these four numbers and nothing arithmetical happens to
        // them, so an epsilon here would be hiding a normalisation defect rather than a rounding.
        assert!(
            popup.rect.iter().eq([40.0_f32, 40.0, 200.0, 140.0].iter()),
            "{:?}",
            popup.rect
        );
        assert_eq!(popup.colour.map(|c| (c.r, c.g, c.b)), Some((1.0, 0.0, 0.0)));
        assert_eq!(popup.parent, Some(pdf_syntax::ObjectId::new(4, 0)));
    }

    /// §12.5.6.2's group attributes reach through Table 186's override.
    ///
    /// A **pair** differing only in `/RT`, because the corpus cannot rank this one:
    /// `examples/annotation_group_census` finds a single `/IRT` in the 964 openable documents and
    /// no popup on it (trap 8). The witness is ISO 32000-2's own PDF, where 213 windows hang off a
    /// subordinate and the erratum's words are in the primary — a caret carrying the replacement
    /// text, a strike-out carrying an empty `/RC` and the popup a reader clicks.
    ///
    /// > Some entries in the primary annotation are treated as "group attributes" that shall apply
    /// > to the group as a whole; the corresponding entries in the subordinate annotations shall
    /// > be ignored. These entries are Contents (or RC and DS ), M , C , T , Popup , CreationDate ,
    /// > Subj , and Open .
    #[test]
    fn a_window_on_a_group_shows_the_primarys_words() {
        let grouped = "4 0 obj << /Type /Annot /Subtype /Caret /Rect [10 10 30 30] \
                       /Contents (the replacement) /T (the editor) /C [1 0 0] \
                       /M (D:20260812120000Z) >> endobj\n\
                       5 0 obj << /Type /Annot /Subtype /StrikeOut /Rect [10 10 90 30] \
                       /IRT 4 0 R /RT /Group /Popup 6 0 R /T (nobody) /C [0 1 0] >> endobj\n\
                       6 0 obj << /Type /Annot /Subtype /Popup /Rect [40 40 200 140] \
                       /Parent 5 0 R /Open true >> endobj\n";
        // The reply half of the same entry pair: `/RT /R` is not a group, so nothing is shared and
        // the window is the empty one the file describes.
        let reply = grouped.replace("/RT /Group ", "/RT /R ");
        assert_ne!(grouped, reply, "the pair must differ in the rule alone");

        for (objects, text, title, colour) in [
            (
                grouped,
                Some("the replacement"),
                Some("the editor"),
                (1.0_f32, 0.0_f32, 0.0_f32),
            ),
            (reply.as_str(), None, Some("nobody"), (0.0, 1.0, 0.0)),
        ] {
            let document = document("4 0 R 5 0 R 6 0 R", objects);
            let view = crate::view::ViewState::of(&document);
            let popups = popups(&document, &page(&document), &view);
            assert_eq!(popups.len(), 1, "{objects}");
            assert_eq!(popups[0].text.as_deref(), text, "{objects}");
            assert_eq!(popups[0].title.as_deref(), title, "{objects}");
            assert_eq!(
                popups[0].colour.map(|c| (c.r, c.g, c.b)),
                Some(colour),
                "{objects}"
            );
        }
    }

    /// And Table 172's `/Popup` is itself a group attribute, so a click exhibits one window.
    ///
    /// §12.5.6.2: a group is "a set of annotations … grouped so that they function as a single
    /// unit when a user interacts with them", and `/Popup` is on the list of nine — so the window
    /// a subordinate opens is the primary's and not the one the subordinate names.
    #[test]
    fn a_subordinates_own_popup_entry_is_ignored() {
        let document = document(
            "4 0 R 5 0 R 6 0 R 7 0 R",
            "4 0 obj << /Type /Annot /Subtype /Caret /Rect [10 10 30 30] /Popup 7 0 R \
             /Contents (the replacement) >> endobj\n\
             5 0 obj << /Type /Annot /Subtype /StrikeOut /Rect [10 10 90 30] /IRT 4 0 R \
             /RT /Group /Popup 6 0 R >> endobj\n\
             6 0 obj << /Type /Annot /Subtype /Popup /Rect [40 40 200 140] /Parent 5 0 R >> \
             endobj\n\
             7 0 obj << /Type /Annot /Subtype /Popup /Rect [50 50 210 150] /Parent 4 0 R >> \
             endobj\n",
        );
        let subordinate = document.get(pdf_syntax::ObjectId::new(5, 0));
        let dict = subordinate.as_dict().expect("an annotation dictionary");
        assert_eq!(
            popup_of(&document, dict),
            Some(pdf_syntax::ObjectId::new(7, 0)),
            "the primary's window, not the subordinate's own /Popup 6 0 R"
        );
    }

    #[test]
    fn a_popup_with_no_parent_states_its_own_text() {
        // NOTE 3: "The Contents entry for a popup annotation is relevant only if it has no
        // parent; in that case, it represents the text of the annotation."
        let document = document(
            "5 0 R",
            "5 0 obj << /Type /Annot /Subtype /Popup /Rect [40 40 200 140] \
             /Contents (an orphan) >> endobj\n",
        );
        let view = crate::view::ViewState::of(&document);
        let popups = popups(&document, &page(&document), &view);
        assert_eq!(popups.len(), 1);
        assert_eq!(popups[0].text.as_deref(), Some("an orphan"));
        assert_eq!(popups[0].parent, None);
        // Table 186's default: "Default value: false (closed)."
        assert!(!popups[0].open);
    }

    /// §12.5.6.4's `/Open` opens the window its own popup does not ask for.
    ///
    /// A **pair** differing only in the rule, because the corpus cannot rank this one: the single
    /// text annotation in it that states `/Open true` has a popup that states its own, so a reader
    /// ignoring Table 175 draws that document identically (trap 8, and
    /// `examples/open_annotation_census` is the count).
    #[test]
    fn a_text_annotations_own_open_opens_its_popup() {
        // Table 175: "A flag specifying whether the annotation shall initially be displayed
        // open", and §12.5.6.4: "when open, it shall display a popup window containing the text
        // of the note".
        let with_open = "4 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /Popup 5 0 R \
                         /Open true /Contents (a note) >> endobj\n\
                         5 0 obj << /Type /Annot /Subtype /Popup /Rect [40 40 200 140] \
                         /Parent 4 0 R >> endobj\n";
        let without = with_open.replace("/Open true ", "");
        assert_ne!(with_open, without, "the pair must differ in the rule alone");

        for (objects, expected) in [(with_open, true), (without.as_str(), false)] {
            let document = document("4 0 R 5 0 R", objects);
            let view = crate::view::ViewState::of(&document);
            let popups = popups(&document, &page(&document), &view);
            assert_eq!(popups.len(), 1);
            assert_eq!(popups[0].open, expected, "{objects}");
        }
    }

    /// An `/Open` on a subtype no table gives one to opens nothing.
    #[test]
    fn only_a_text_annotations_open_reaches_its_popup() {
        // Table 182 gives a text markup annotation no `/Open`, and §12.5.6.10's "[w]hen opened,
        // they shall display a popup window" names no entry that opens it. A key the standard
        // does not define for this subtype states nothing about the window.
        let document = document(
            "4 0 R 5 0 R",
            "4 0 obj << /Type /Annot /Subtype /Highlight /Rect [10 10 30 30] /Popup 5 0 R \
             /Open true /Contents (a note) >> endobj\n\
             5 0 obj << /Type /Annot /Subtype /Popup /Rect [40 40 200 140] /Parent 4 0 R >> \
             endobj\n",
        );
        let view = crate::view::ViewState::of(&document);
        let popups = popups(&document, &page(&document), &view);
        assert_eq!(popups.len(), 1);
        assert!(!popups[0].open);
    }

    #[test]
    fn a_hidden_popup_is_not_a_window() {
        // §12.5.3's `Hidden`: "shall not display or print the annotation".
        let document = document(
            "5 0 R",
            "5 0 obj << /Type /Annot /Subtype /Popup /Rect [40 40 200 140] /F 2 /Open true >> \
             endobj\n",
        );
        let view = crate::view::ViewState::of(&document);
        assert!(popups(&document, &page(&document), &view).is_empty());
    }

    #[test]
    fn the_flags_read_are_the_popups_own_and_not_the_parents() {
        // `pr7352.pdf` in the corpus asserts the opposite rule — that a popup inherits its
        // parent's flags — and Table 186 names four entries the parent overrides, none of them
        // `/F`. Here the parent is hidden and the popup is not, so the window stands.
        let document = document(
            "4 0 R 5 0 R",
            "4 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /Popup 5 0 R /F 2 \
             /Contents (a note) >> endobj\n\
             5 0 obj << /Type /Annot /Subtype /Popup /Rect [40 40 200 140] /Parent 4 0 R \
             /Open true >> endobj\n",
        );
        let view = crate::view::ViewState::of(&document);
        let popups = popups(&document, &page(&document), &view);
        assert_eq!(popups.len(), 1);
        assert_eq!(popups[0].text.as_deref(), Some("a note"));
    }

    #[test]
    fn a_popup_with_no_rectangle_is_refused() {
        let document = document(
            "5 0 R",
            "5 0 obj << /Type /Annot /Subtype /Popup /Contents (nowhere) >> endobj\n",
        );
        let view = crate::view::ViewState::of(&document);
        assert!(popups(&document, &page(&document), &view).is_empty());
    }

    #[test]
    fn table_172_s_entry_names_the_window() {
        let document = document(
            "4 0 R 5 0 R",
            "4 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /Popup 5 0 R >> endobj\n\
             5 0 obj << /Type /Annot /Subtype /Popup /Rect [40 40 200 140] /Parent 4 0 R >> \
             endobj\n",
        );
        let parent = document.get(pdf_syntax::ObjectId::new(4, 0));
        let dict = parent.as_dict().expect("an annotation dictionary");
        assert_eq!(
            popup_of(&document, dict),
            Some(pdf_syntax::ObjectId::new(5, 0))
        );
    }

    /// Table 172's `/RC`, where the file states one and no `/Contents` — §12.5.6.2:
    ///
    /// > A rich text string … that shall be displayed in the popup window when the annotation is
    /// > opened.
    ///
    /// The characters and none of the formatting: a `<span>`'s style is XFA's and `CLAUDE.md`
    /// excludes it, while the text is what the clause requires shown and what NOTE 1 makes
    /// equivalent to `/Contents`. A closing paragraph is the newline §12.5.6.2 asks a plain
    /// `/Contents` to spell with a carriage return.
    #[test]
    fn a_rich_text_string_fills_a_window_a_plain_one_would_have_left_empty() {
        let document = document(
            "4 0 R 5 0 R",
            "4 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /Popup 5 0 R \
             /RC (<?xml version=\"1.0\"?><body xmlns=\"http://www.w3.org/1999/xhtml\">\
             <p><span style=\"font-weight:bold\">first</span> line</p><p>second &amp; last</p>\
             </body>) >> endobj\n\
             5 0 obj << /Type /Annot /Subtype /Popup /Rect [40 40 200 140] /Parent 4 0 R >> \
             endobj\n",
        );
        let view = crate::view::ViewState::of(&document);
        let popups = popups(&document, &page(&document), &view);
        assert_eq!(popups.len(), 1);
        assert_eq!(
            popups[0].text.as_deref(),
            Some("first line\nsecond & last"),
            "the characters, the paragraph break and the entity — and no style"
        );
    }

    /// And `/Contents` wins where a file states both, which is §12.5.6.2's NOTE 1 read straight:
    ///
    /// > When both Contents and RC entries are present, it is expected that the contents of both
    /// > entries are textually equivalent.
    #[test]
    fn a_plain_contents_outranks_the_rich_one() {
        let document = document(
            "4 0 R 5 0 R",
            "4 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /Popup 5 0 R \
             /Contents (the plain one) \
             /RC (<body xmlns=\"http://www.w3.org/1999/xhtml\"><p>the rich one</p></body>) >> \
             endobj\n\
             5 0 obj << /Type /Annot /Subtype /Popup /Rect [40 40 200 140] /Parent 4 0 R >> \
             endobj\n",
        );
        let view = crate::view::ViewState::of(&document);
        let popups = popups(&document, &page(&document), &view);
        assert_eq!(popups[0].text.as_deref(), Some("the plain one"));
    }

    /// Malformed markup keeps what it had read rather than emptying the window.
    #[test]
    fn markup_that_stops_parsing_keeps_the_text_before_it() {
        let document = document(
            "4 0 R 5 0 R",
            "4 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /Popup 5 0 R \
             /RC (<body><p>what was written</p><p>and then <<<) >> endobj\n\
             5 0 obj << /Type /Annot /Subtype /Popup /Rect [40 40 200 140] /Parent 4 0 R >> \
             endobj\n",
        );
        let view = crate::view::ViewState::of(&document);
        let popups = popups(&document, &page(&document), &view);
        assert_eq!(
            popups[0].text.as_deref(),
            Some("what was written\nand then"),
            "everything the tokenizer reached before it stopped — half a comment is better than \
             none of it"
        );
    }

    /// §7.9.3's text stream, which is the one form of it any in-scope entry uses.
    ///
    /// > A text stream ( PDF 1.5 ) shall be a PDF stream object (7.3.8, "Stream objects") whose
    /// > unencoded bytes shall meet the same requirements as a text string (7.9.2.2, "Text string
    /// > type") with respect to encoding, byte order, and lead bytes.
    ///
    /// Table 172 types `/RC` "text string or text stream", so reading it means reading both. The
    /// clause's *unencoded* is the load-bearing word: the bytes that have to meet §7.9.2.2's
    /// requirements are the stream's **decoded** ones, so a filter runs before the text decoding
    /// and not after. §7.9.2.2.1's UTF-8 lead bytes are what prove the order here — a reader that
    /// ran the two the other way round would find no prefix at all and fall back to Table D.3,
    /// where the three prefix bytes are three visible characters.
    #[test]
    fn a_rich_text_stream_is_read_as_a_text_string_after_its_filter() {
        // §7.9.2.2.1's UTF-8 prefix, EF BB BF, written as octal escapes so that the fixture
        // stays a `&str`; the payload is XHTML with one non-ASCII character in it.
        let markup = "\u{feff}<body><p>h\u{e9}llo</p></body>";
        let bytes = markup.as_bytes();
        let objects = format!(
            "4 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /Popup 5 0 R /RC 6 0 R >> \
             endobj\n\
             5 0 obj << /Type /Annot /Subtype /Popup /Rect [40 40 200 140] /Parent 4 0 R >> \
             endobj\n\
             6 0 obj << /Length {} >> stream\n{markup}\nendstream endobj\n",
            bytes.len()
        );
        let document = document("4 0 R 5 0 R", &objects);
        let view = crate::view::ViewState::of(&document);
        let popups = popups(&document, &page(&document), &view);
        assert_eq!(popups.len(), 1);
        assert_eq!(
            popups[0].text.as_deref(),
            Some("h\u{e9}llo"),
            "the stream's decoded bytes read as §7.9.2.2's text string, lead bytes and all"
        );
    }

    #[test]
    fn a_date_in_no_format_at_all_is_still_shown() {
        // Table 166's `/M`: "interactive PDF processors shall accept and display a string in any
        // format" — so what crosses is the string, and the parse is a second question.
        let document = document(
            "5 0 R",
            "5 0 obj << /Type /Annot /Subtype /Popup /Rect [0 0 10 10] /M (last Tuesday) >> \
             endobj\n",
        );
        let view = crate::view::ViewState::of(&document);
        let popups = popups(&document, &page(&document), &view);
        assert_eq!(popups[0].modified.as_deref(), Some("last Tuesday"));
        assert!(popups[0].modified_date().is_none());
    }

    /// §12.5.6.2's reply `shall`, in one window rather than three.
    ///
    /// Table 172's `/RT` `R`: "[i]nteractive PDF processors shall not display replies to an
    /// annotation individually but together in the form of threaded comments." The fixture is a
    /// square with a window, a reply to it with a window of its own, and a reply to *that* with a
    /// third — which this program opened as three separate windows until the rule was read.
    ///
    /// `/RT` is absent from both replies on purpose: `R` is the default, so an annotation with an
    /// `/IRT` and nothing else is already a reply, and a rule that only fired on an explicit
    /// `/RT /R` would miss the common case.
    #[test]
    fn replies_are_threaded_into_one_window_rather_than_opened_beside_it() {
        let document = document(
            "4 0 R 5 0 R 6 0 R 7 0 R 8 0 R 9 0 R",
            "4 0 obj << /Type /Annot /Subtype /Square /Rect [10 10 90 90] /Popup 5 0 R \
             /T (the author) /Contents (the original) >> endobj\n\
             5 0 obj << /Type /Annot /Subtype /Popup /Rect [100 10 300 90] /Parent 4 0 R >> \
             endobj\n\
             6 0 obj << /Type /Annot /Subtype /Square /Rect [10 10 90 90] /IRT 4 0 R \
             /Popup 7 0 R /T (a reviewer) /Contents (the reply) >> endobj\n\
             7 0 obj << /Type /Annot /Subtype /Popup /Rect [100 110 300 190] /Parent 6 0 R \
             /Open true >> endobj\n\
             8 0 obj << /Type /Annot /Subtype /Square /Rect [10 10 90 90] /IRT 6 0 R \
             /Popup 9 0 R /T (the author) /Contents (the answer) >> endobj\n\
             9 0 obj << /Type /Annot /Subtype /Popup /Rect [100 210 300 290] /Parent 8 0 R >> \
             endobj\n",
        );
        let page = page(&document);
        let view = crate::view::ViewState::of(&document);
        let windows = popups(&document, &page, &view);
        assert_eq!(windows.len(), 1, "one window for the thread: {windows:?}");
        let window = &windows[0];
        assert_eq!(window.annotation, pdf_syntax::ObjectId::new(5, 0));
        assert_eq!(window.text.as_deref(), Some("the original"));
        let said: Vec<(usize, &str)> = window
            .replies
            .iter()
            .map(|reply| (reply.depth, reply.text.as_deref().unwrap_or_default()))
            .collect();
        assert_eq!(
            said,
            vec![(1, "the reply"), (2, "the answer")],
            "both replies, deepest last, each at its own distance from the window"
        );
        assert_eq!(
            window.replies[0].title.as_deref(),
            Some("a reviewer"),
            "and who wrote it, which is the one thing a thread has that a long note does not"
        );
        // Table 186's `/Open` on the reply's own window opens the thread's, by
        // `opens_with_the_page`'s reading: each entry states a condition under which a window is
        // open and none states one under which it is closed.
        assert!(
            window.open,
            "the reply's /Open opens the window it is shown in"
        );

        // §12.5.1's activation of the reply reaches the same window, so a click on it is not a
        // click on nothing.
        let reply = document.get(pdf_syntax::ObjectId::new(8, 0));
        let reply = reply.as_dict().expect("the reply is a dictionary");
        assert_eq!(
            popup_of(&document, reply),
            Some(pdf_syntax::ObjectId::new(5, 0))
        );
    }

    /// The thread hangs on the **highest ancestor that states a window**, not on the chain's head.
    ///
    /// The head is where a thread most obviously belongs, and the count says it is not enough: of
    /// ISO 32000-2's own 1752 replies every one names a `/Popup` and only 1401 have a head that
    /// names one (`examples/annotation_group_census`). This fixture is that shape — a head with
    /// no window, a reply to it with one, and a reply to *that* — so a head-only rule would leave
    /// the last two in windows of their own, which is the display the clause forbids.
    ///
    /// The second half is the case with nothing above it at all: a reply whose whole chain opens
    /// no window has nothing to be shown *together with*, and it keeps its own rather than being
    /// dropped. Obeying the sentence by losing the text is trap 5's failure.
    #[test]
    fn a_thread_hangs_on_the_highest_window_its_chain_states() {
        let deep = document(
            "4 0 R 6 0 R 7 0 R 8 0 R 9 0 R",
            "4 0 obj << /Type /Annot /Subtype /Square /Rect [10 10 90 90] \
             /Contents (the head, with no window) >> endobj\n\
             6 0 obj << /Type /Annot /Subtype /Square /Rect [10 10 90 90] /IRT 4 0 R /RT /R \
             /Popup 7 0 R /Contents (the reply) >> endobj\n\
             7 0 obj << /Type /Annot /Subtype /Popup /Rect [100 110 300 190] /Parent 6 0 R >> \
             endobj\n\
             8 0 obj << /Type /Annot /Subtype /Square /Rect [10 10 90 90] /IRT 6 0 R /RT /R \
             /Popup 9 0 R /Contents (the answer) >> endobj\n\
             9 0 obj << /Type /Annot /Subtype /Popup /Rect [100 210 300 290] /Parent 8 0 R >> \
             endobj\n",
        );
        let windows = popups(&deep, &page(&deep), &crate::view::ViewState::of(&deep));
        assert_eq!(windows.len(), 1, "{windows:?}");
        assert_eq!(windows[0].annotation, pdf_syntax::ObjectId::new(7, 0));
        assert_eq!(windows[0].text.as_deref(), Some("the reply"));
        let said: Vec<(usize, &str)> = windows[0]
            .replies
            .iter()
            .map(|reply| (reply.depth, reply.text.as_deref().unwrap_or_default()))
            .collect();
        assert_eq!(said, vec![(1, "the answer")]);

        let alone = document(
            "4 0 R 6 0 R 7 0 R",
            "4 0 obj << /Type /Annot /Subtype /Square /Rect [10 10 90 90] \
             /Contents (no window) >> endobj\n\
             6 0 obj << /Type /Annot /Subtype /Square /Rect [10 10 90 90] /IRT 4 0 R /RT /R \
             /Popup 7 0 R /Contents (the reply) >> endobj\n\
             7 0 obj << /Type /Annot /Subtype /Popup /Rect [100 110 300 190] /Parent 6 0 R >> \
             endobj\n",
        );
        let windows = popups(&alone, &page(&alone), &crate::view::ViewState::of(&alone));
        assert_eq!(windows.len(), 1, "{windows:?}");
        assert_eq!(windows[0].text.as_deref(), Some("the reply"));
        assert!(
            windows[0].replies.is_empty(),
            "it hosts a thread of one, which is itself"
        );
    }

    /// A `/RT /Group` subordinate is not a reply, and its window is not folded.
    ///
    /// §12.5.6.2 gives the two values of `/RT` different consequences: `Group` shares the
    /// primary's entries, which `crate::markup` already applies, and `R` is the one the threading
    /// `shall` is about. The pair differs only in that name (trap 8).
    #[test]
    fn only_a_reply_is_threaded_and_a_group_member_is_not() {
        let with = |reply_type: &str| {
            let document = document(
                "4 0 R 5 0 R 6 0 R 7 0 R",
                &format!(
                    "4 0 obj << /Type /Annot /Subtype /Square /Rect [10 10 90 90] /Popup 5 0 R \
                     /Contents (the primary) >> endobj\n\
                     5 0 obj << /Type /Annot /Subtype /Popup /Rect [100 10 300 90] \
                     /Parent 4 0 R >> endobj\n\
                     6 0 obj << /Type /Annot /Subtype /Square /Rect [10 10 90 90] /IRT 4 0 R \
                     {reply_type}/Popup 7 0 R /Contents (the other) >> endobj\n\
                     7 0 obj << /Type /Annot /Subtype /Popup /Rect [100 110 300 190] \
                     /Parent 6 0 R >> endobj\n"
                ),
            );
            let page = page(&document);
            let view = crate::view::ViewState::of(&document);
            popups(&document, &page, &view).len()
        };
        assert_eq!(with("/RT /R "), 1, "a reply is threaded into the primary's");
        assert_eq!(
            with("/RT /Group "),
            2,
            "a group member keeps its window, which §12.5.6.2 fills from the primary"
        );
    }

    /// A file whose `/IRT` chain loops terminates rather than climbing for ever.
    ///
    /// The clause states no depth and nothing stops a producer writing a cycle; §12.6.2 gives the
    /// same answer for the one other structure a file can make self-referential.
    #[test]
    fn a_reply_chain_that_loops_still_answers() {
        let document = document(
            "4 0 R 5 0 R 6 0 R",
            "4 0 obj << /Type /Annot /Subtype /Square /Rect [10 10 90 90] /IRT 6 0 R \
             /Popup 5 0 R /Contents (one) >> endobj\n\
             5 0 obj << /Type /Annot /Subtype /Popup /Rect [100 10 300 90] /Parent 4 0 R >> \
             endobj\n\
             6 0 obj << /Type /Annot /Subtype /Square /Rect [10 10 90 90] /IRT 4 0 R \
             /Contents (two) >> endobj\n",
        );
        let page = page(&document);
        let view = crate::view::ViewState::of(&document);
        assert_eq!(popups(&document, &page, &view).len(), 1);
    }
}
