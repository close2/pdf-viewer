//! ISO 32000-2 §12.5.6.3: the author-specific state a reviewer has put on an annotation.
//!
//! A reviewer marks somebody else's comment *accepted*, *rejected* or *completed*, and the
//! clause's opening says where that fact lives — which is not where a reader would first look
//! for it:
//!
//! > Beginning with PDF 1.5, annotations may have an author-specific state associated with them.
//! > The state is not specified in the annotation itself but in a separate text annotation that
//! > refers to the original annotation by means of its IRT ("in reply to") entry
//!
//! So the state of an annotation is a fact about the *other* annotations on its page, and
//! [`states`] is the walk that recovers it. Nothing here marks the page: the §12.5.6.4 text
//! annotation carrying the state draws its own icon like any other, and what this module produces
//! is the review status a comments panel shows beside a comment — a document fact handed over,
//! in the same division [`crate::popup`] makes for a window.
//!
//! # Table 174 is the whole vocabulary
//!
//! "States shall be grouped into a number of state models", two of them — `Marked`, whose states
//! are `Marked` and `Unmarked`, and `Review`, whose states are `Accepted`, `Rejected`,
//! `Cancelled`, `Completed` and `None`. [`StateModel`] and [`State`] are that table, and each
//! carries an `Other` case rather than discarding a name the table does not have: both entries
//! are `text string`s with no enumeration constraining them, so a producer can write anything,
//! and dropping what it wrote would be the silent swallowing `CLAUDE.md` principle 1 forbids.
//!
//! # Three entries, and only one of them is in Table 175
//!
//! §12.5.6.3 names them together, the third being the one this module is about:
//!
//! > State and StateModel (see "Table 175 -Additional entries specific to a text annotation")
//! > shall update the state of the original annotation for the specified user.
//!
//! The other two are Table 172's. `/T` "shall specify the user" and is read through
//! [`crate::markup`], because §12.5.6.2 makes it a group attribute: a state annotation that is
//! itself a group's subordinate names its user in the primary. `/IRT` "shall refer to the
//! original annotation" — and this clause's two citations for it name Table 176, a *link*
//! annotation's, which Errata Collection 3's Issue #479 corrects to Table 172 in both places.
//! The correction moves nothing here: the entry was always read as the markup annotation's.
//!
//! # The chain, and why the deepest reply wins
//!
//! §12.5.6.3 closes with the sentence that makes a state a *sequence*:
//!
//! > Additional state changes shall be made by adding text annotations in reply to the previous
//! > reply for a given user.
//!
//! A user's second opinion is a reply to their own first one, so the chain's *depth* is its
//! chronology and the clause states no other order — not `/M`, not `/CreationDate`, neither of
//! which a reader may assume a producer kept honest. [`states`] walks outwards from the original
//! breadth-first, so a state deeper in the chain replaces a shallower one for the same user and
//! the same model, while two users who never replied to each other both keep their own answer.
//!
//! Table 172 bounds the walk to one page — "[b]oth annotations shall be on the same page of the
//! document" — so the population is the page's own `/Annots` and no other page is opened. A file
//! whose chain loops has each annotation visited once, which is §12.6.2's rule about a
//! self-referential structure applied to the one structure this clause builds.
//!
//! # What is deliberately not read
//!
//! Table 172 gives `/IRT` a second form: "[i]f this entry is present in an FDF file … its type
//! shall not be a dictionary but a text string containing the contents of the NM entry of the
//! annotation being replied to". That is §12.7.8's forms data format rather than a page, and what
//! this module is given is a page.

use std::collections::BTreeSet;

use pdf_syntax::{Dictionary, Document, ObjectId};

use crate::Page;

/// Table 174's state models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateModel {
    /// Whether the user has marked the annotation.
    Marked,
    /// A reviewer's verdict on a change.
    Review,
    /// A model name Table 174 does not have, as the file spells it.
    Other(String),
}

impl StateModel {
    /// The model Table 174 spells this way, or [`StateModel::Other`].
    fn read(written: &str) -> Self {
        match written {
            "Marked" => Self::Marked,
            "Review" => Self::Review,
            other => Self::Other(other.to_owned()),
        }
    }

    /// This model's default state, which Table 175's `/State` row gives for each model:
    /// `Unmarked` under `Marked`, `None` under `Review`.
    ///
    /// `None` for a model Table 174 does not have, which has no default because it has no states.
    fn default_state(&self) -> Option<State> {
        match self {
            Self::Marked => Some(State::Unmarked),
            Self::Review => Some(State::None),
            Self::Other(_) => None,
        }
    }
}

/// Table 174's states, across both models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// "The annotation has been marked by the user."
    Marked,
    /// "The annotation has not been marked by the user (the default)."
    Unmarked,
    /// "The user agrees with the change."
    Accepted,
    /// "The user disagrees with the change."
    Rejected,
    /// "The change has been cancelled."
    Cancelled,
    /// "The change has been completed."
    Completed,
    /// "The user has indicated nothing about the change (the default)."
    None,
    /// A state name Table 174 does not have, as the file spells it.
    Other(String),
}

impl State {
    /// The state Table 174 spells this way, or [`State::Other`].
    fn read(written: &str) -> Self {
        match written {
            "Marked" => Self::Marked,
            "Unmarked" => Self::Unmarked,
            "Accepted" => Self::Accepted,
            "Rejected" => Self::Rejected,
            "Cancelled" => Self::Cancelled,
            "Completed" => Self::Completed,
            "None" => Self::None,
            other => Self::Other(other.to_owned()),
        }
    }

    /// The model Table 174 puts this state under, where the table has the state at all.
    ///
    /// Every state name in that table appears under exactly one model, so this is the table read
    /// backwards rather than an inference. It answers the file that breaks Table 175's "Required
    /// if State is present, otherwise optional" and states a `/State` with no `/StateModel`:
    /// the state names its own model, and refusing the whole state change over the missing entry
    /// would lose a fact the file did state.
    fn model(&self) -> Option<StateModel> {
        match self {
            Self::Marked | Self::Unmarked => Some(StateModel::Marked),
            Self::Accepted | Self::Rejected | Self::Cancelled | Self::Completed | Self::None => {
                Some(StateModel::Review)
            }
            Self::Other(_) => None,
        }
    }
}

/// One user's state for one model, as the page's chain of replies leaves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationState {
    /// Table 172's `/T` on the annotation that set it: "shall specify the user".
    ///
    /// `None` where that annotation states none. The clause makes the state "author-specific" and
    /// the entry naming the author optional, so a file can state a state belonging to nobody in
    /// particular; that is one user as far as this walk is concerned, and naming it anything
    /// would be inventing an author.
    pub user: Option<String>,
    /// Which of Table 174's models the state belongs to.
    pub model: StateModel,
    /// The state itself.
    pub state: State,
    /// The text annotation that set it — the last one in this user's chain.
    pub set_by: ObjectId,
}

/// §12.5.6.3's states of one annotation, one per user and model, in the order they were first set.
///
/// `annotation` is the *original* — the annotation a reviewer replied to — and the answer is empty
/// for the overwhelming majority, which are the annotations nobody has replied to at all.
///
/// # What a caller does with it
///
/// The states are a comments panel's, in the sense [`crate::popup::Popup`] is a window's: this
/// crate reads what the document says and a host decides whether to show it. No clause states a
/// mark on the page for any of them, which is why nothing in [`crate::appearance`] asks.
///
/// # Cost
///
/// One pass over the page's `/Annots` to index the replies, holding two object numbers apiece and
/// resolving a dictionary only for the ones the walk reaches. A page whose annotations state no
/// `/IRT` — which is every page of the curated corpus, counted by
/// `examples/annotation_state_census` — builds an empty index and returns without walking
/// anything. The index is per call, so a caller wanting the states of every annotation on a page
/// asks only about the ones something replies to, as that census does.
#[must_use]
pub fn states(document: &Document, page: &Page, annotation: ObjectId) -> Vec<AnnotationState> {
    let replies = replies(document, page);
    if replies.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<AnnotationState> = Vec::new();
    let mut visited = BTreeSet::new();
    visited.insert(annotation);
    let mut frontier = vec![annotation];
    // Breadth-first, so that a reply deeper in one user's chain is read after the shallower one it
    // supersedes and `record` can simply overwrite.
    while !frontier.is_empty() {
        let mut next = Vec::new();
        for target in frontier {
            for reply in replies.iter().filter(|reply| reply.target == target) {
                if !visited.insert(reply.id) {
                    continue;
                }
                if let Some(state) = state_change(document, reply.id) {
                    record(&mut out, state);
                }
                next.push(reply.id);
            }
        }
        frontier = next;
    }
    out
}

/// One text annotation on the page that replies to another.
struct Reply {
    /// The replying annotation.
    id: ObjectId,
    /// Table 172's `/IRT`: the annotation it is "in reply to".
    target: ObjectId,
}

/// Every `Text` annotation on the page that states an `/IRT`, in the `/Annots` array's own order.
///
/// A list rather than a map keyed by the target: the population is tiny — a page states as many
/// replies as it has review comments — and the order the page listed them in is what makes two
/// states set at the same depth deterministic.
fn replies(document: &Document, page: &Page) -> Vec<Reply> {
    let annotations = document.get_key(&page.dict, "Annots");
    let Some(annotations) = annotations.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in annotations {
        let Some(id) = entry.as_reference() else {
            continue;
        };
        let resolved = document.resolve(entry);
        let Some(dict) = resolved.as_dict() else {
            continue;
        };
        // "State changes made by a user shall be indicated in a text annotation": a reply of any
        // other subtype is a reply and not a state change, so it is not indexed here at all.
        if document
            .get_key(dict, "Subtype")
            .as_name()
            .map(pdf_syntax::Name::as_bytes)
            != Some(b"Text")
        {
            continue;
        }
        let Some(target) = dict.get("IRT").and_then(pdf_syntax::Object::as_reference) else {
            continue;
        };
        out.push(Reply { id, target });
    }
    out
}

/// The state change this text annotation states, or `None` where it states none.
///
/// Table 175 makes both entries optional, so most replies are comments rather than state changes.
/// Either one alone is enough: a `/StateModel` on its own takes that model's default state, and a
/// `/State` on its own takes the model Table 174 puts that state under.
fn state_change(document: &Document, id: ObjectId) -> Option<AnnotationState> {
    let object = document.get(id);
    let dict = object.as_dict()?;
    let state = text(document, dict, "State").map(|written| State::read(&written));
    let model = match text(document, dict, "StateModel") {
        Some(written) => StateModel::read(&written),
        None => state.as_ref()?.model()?,
    };
    let state = match state {
        Some(state) => state,
        None => model.default_state()?,
    };
    Some(AnnotationState {
        // §12.5.6.2 makes `/T` a group attribute, so a state annotation that is a subordinate in a
        // group names its user in the primary.
        user: text(document, &crate::markup::group_source(document, dict), "T"),
        model,
        state,
        set_by: id,
    })
}

/// Files one state change, replacing this user's earlier one for the same model.
///
/// The clause's chain is per user — "in reply to the previous reply for a given user" — and the
/// two models are independent, so a user who marked an annotation *and* accepted it has two states
/// and neither supersedes the other.
fn record(out: &mut Vec<AnnotationState>, state: AnnotationState) {
    if let Some(held) = out
        .iter_mut()
        .find(|held| held.user == state.user && held.model == state.model)
    {
        *held = state;
    } else {
        out.push(state);
    }
}

/// A `text string` entry, decoded by §7.9.2.2, or `None` where there is none.
fn text(document: &Document, dict: &Dictionary, key: &str) -> Option<String> {
    let value = document.get_key(dict, key);
    let bytes = value.as_string()?;
    let decoded = pdf_syntax::text_string(bytes);
    (!decoded.is_empty()).then_some(decoded)
}

#[cfg(test)]
mod tests {
    use pdf_syntax::{Document, ObjectId};

    use super::{State, StateModel, states};

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

    /// The original annotation every fixture below replies to, with no state of its own.
    const ORIGINAL: &str = "4 0 obj << /Type /Annot /Subtype /Highlight /Rect [10 10 90 30] \
                            /T (the primary) /Contents (the change) >> endobj\n";

    /// A state set by a reply reaches the annotation it replies to.
    ///
    /// The clause's own sentence: the state "is not specified in the annotation itself but in a
    /// separate text annotation that refers to the original annotation by means of its IRT".
    /// Both directions are asserted — the original carries the state and the *reply* carries
    /// none of its own — because a reader that answered from the annotation's own dictionary
    /// would return nothing for the first and nothing for the second alike.
    #[test]
    fn a_reply_sets_the_state_of_the_annotation_it_replies_to() {
        let document = document(
            "4 0 R 5 0 R",
            &format!(
                "{ORIGINAL}\
                 5 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 4 0 R \
                 /T (Ada) /State (Accepted) /StateModel (Review) >> endobj\n"
            ),
        );
        let page = page(&document);
        let set = states(&document, &page, ObjectId::new(4, 0));
        assert_eq!(set.len(), 1);
        assert_eq!(set[0].user.as_deref(), Some("Ada"));
        assert_eq!(set[0].model, StateModel::Review);
        assert_eq!(set[0].state, State::Accepted);
        assert_eq!(set[0].set_by, ObjectId::new(5, 0));
        assert!(
            states(&document, &page, ObjectId::new(5, 0)).is_empty(),
            "the reply is not in reply to itself"
        );
    }

    /// A user's later reply supersedes their earlier one; another user's stands beside it.
    ///
    /// "Additional state changes shall be made by adding text annotations in reply to the
    /// previous reply for a given user", so depth is the chronology. Grace's verdict is a reply
    /// to the *original*, so it is at the same depth as Ada's first and is not touched by Ada's
    /// second — which is what makes this two assertions rather than one.
    #[test]
    fn the_deeper_reply_in_a_users_chain_supersedes_the_shallower() {
        let document = document(
            "4 0 R 5 0 R 6 0 R 7 0 R",
            &format!(
                "{ORIGINAL}\
                 5 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 4 0 R \
                 /T (Ada) /State (Rejected) /StateModel (Review) >> endobj\n\
                 6 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 4 0 R \
                 /T (Grace) /State (Completed) /StateModel (Review) >> endobj\n\
                 7 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 5 0 R \
                 /T (Ada) /State (Accepted) /StateModel (Review) >> endobj\n"
            ),
        );
        let states = states(&document, &page(&document), ObjectId::new(4, 0));
        assert_eq!(states.len(), 2, "one per user: {states:?}");
        assert_eq!(states[0].user.as_deref(), Some("Ada"));
        assert_eq!(states[0].state, State::Accepted);
        assert_eq!(states[0].set_by, ObjectId::new(7, 0));
        assert_eq!(states[1].user.as_deref(), Some("Grace"));
        assert_eq!(states[1].state, State::Completed);
    }

    /// The two models are independent: one user can hold a state in each.
    #[test]
    fn a_user_holds_one_state_per_model() {
        let document = document(
            "4 0 R 5 0 R 6 0 R",
            &format!(
                "{ORIGINAL}\
                 5 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 4 0 R \
                 /T (Ada) /State (Marked) /StateModel (Marked) >> endobj\n\
                 6 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 5 0 R \
                 /T (Ada) /State (Cancelled) /StateModel (Review) >> endobj\n"
            ),
        );
        let states = states(&document, &page(&document), ObjectId::new(4, 0));
        assert_eq!(states.len(), 2, "{states:?}");
        assert_eq!(states[0].model, StateModel::Marked);
        assert_eq!(states[0].state, State::Marked);
        assert_eq!(states[1].model, StateModel::Review);
        assert_eq!(states[1].state, State::Cancelled);
    }

    /// Either entry alone states a state, and Table 174 supplies the other half.
    ///
    /// A `/StateModel` with no `/State` takes Table 175's default for that model; a `/State` with
    /// no `/StateModel` takes the model Table 174 puts it under, which is the file breaking
    /// "Required if State is present" and still having said something.
    #[test]
    fn one_of_the_two_entries_is_enough() {
        for (entries, model, state) in [
            ("/StateModel (Marked)", StateModel::Marked, State::Unmarked),
            ("/StateModel (Review)", StateModel::Review, State::None),
            ("/State (Rejected)", StateModel::Review, State::Rejected),
            ("/State (Unmarked)", StateModel::Marked, State::Unmarked),
        ] {
            let document = document(
                "4 0 R 5 0 R",
                &format!(
                    "{ORIGINAL}\
                     5 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 4 0 R \
                     {entries} >> endobj\n"
                ),
            );
            let states = states(&document, &page(&document), ObjectId::new(4, 0));
            assert_eq!(states.len(), 1, "{entries}");
            assert_eq!(states[0].model, model, "{entries}");
            assert_eq!(states[0].state, state, "{entries}");
            assert_eq!(states[0].user, None, "{entries}");
        }
    }

    /// A reply that states neither entry is a comment and not a state change.
    #[test]
    fn a_reply_with_no_state_states_none() {
        let document = document(
            "4 0 R 5 0 R",
            &format!(
                "{ORIGINAL}\
                 5 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 4 0 R \
                 /T (Ada) /Contents (I disagree) >> endobj\n"
            ),
        );
        assert!(
            states(&document, &page(&document), ObjectId::new(4, 0)).is_empty(),
            "a reply is not a state change"
        );
    }

    /// A name outside Table 174 is carried rather than dropped.
    #[test]
    fn a_name_the_table_does_not_have_is_kept_as_the_file_spells_it() {
        let document = document(
            "4 0 R 5 0 R",
            &format!(
                "{ORIGINAL}\
                 5 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 4 0 R \
                 /T (Ada) /State (Approved) /StateModel (Approval) >> endobj\n"
            ),
        );
        let states = states(&document, &page(&document), ObjectId::new(4, 0));
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].model, StateModel::Other("Approval".to_owned()));
        assert_eq!(states[0].state, State::Other("Approved".to_owned()));
    }

    /// A state annotation that is a group's subordinate names its user in the primary.
    ///
    /// §12.5.6.2 makes `/T` a group attribute — "the corresponding entries in the subordinate
    /// annotations shall be ignored" — and §12.5.6.3 makes `/T` the entry that "shall specify the
    /// user", so the two clauses compose. The pair differs in `/RT` alone, which is the whole
    /// rule: with `/RT /R` the subordinate's own `/T` is its own.
    #[test]
    fn a_subordinates_user_is_the_primarys() {
        for (reply_type, user) in [
            ("/RT /Group ", "the primary"),
            ("/RT /R ", "the subordinate"),
        ] {
            // The state annotation is a subordinate of 5 0 R and replies to 4 0 R: its `/IRT`
            // names the primary of its group, which is what §12.5.6.2 requires of a subordinate,
            // and 4 0 R is the annotation whose state it sets.
            let document = document(
                "4 0 R 5 0 R",
                &format!(
                    "{ORIGINAL}\
                     5 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 4 0 R \
                     {reply_type}/T (the subordinate) /State (Accepted) /StateModel (Review) >> \
                     endobj\n"
                ),
            );
            let states = states(&document, &page(&document), ObjectId::new(4, 0));
            assert_eq!(states.len(), 1, "{reply_type}");
            assert_eq!(states[0].user.as_deref(), Some(user), "{reply_type}");
        }
    }

    /// A chain that loops is walked once and terminates.
    ///
    /// Two replies each naming the other leave the clause behind — a chain is a chain — and
    /// §12.6.2's rule for a self-referential structure is that it is not executed twice.
    #[test]
    fn a_looping_chain_terminates() {
        let document = document(
            "4 0 R 5 0 R 6 0 R",
            &format!(
                "{ORIGINAL}\
                 5 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 6 0 R \
                 /T (Ada) /State (Accepted) /StateModel (Review) >> endobj\n\
                 6 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 5 0 R \
                 /T (Ada) /State (Rejected) /StateModel (Review) >> endobj\n"
            ),
        );
        assert!(
            states(&document, &page(&document), ObjectId::new(4, 0)).is_empty(),
            "neither reply names the original"
        );
        let from_one = states(&document, &page(&document), ObjectId::new(5, 0));
        assert_eq!(from_one.len(), 1);
        assert_eq!(from_one[0].state, State::Rejected);
    }

    /// A reply of a subtype other than `Text` sets no state.
    ///
    /// "State changes made by a user shall be indicated in a text annotation", so a `Highlight`
    /// carrying the two entries has not made one — the entries are Table 175's and Table 175 is
    /// a text annotation's.
    #[test]
    fn only_a_text_annotation_sets_a_state() {
        let document = document(
            "4 0 R 5 0 R",
            &format!(
                "{ORIGINAL}\
                 5 0 obj << /Type /Annot /Subtype /Highlight /Rect [10 10 30 30] /IRT 4 0 R \
                 /T (Ada) /State (Accepted) /StateModel (Review) >> endobj\n"
            ),
        );
        assert!(states(&document, &page(&document), ObjectId::new(4, 0)).is_empty());
    }
}
