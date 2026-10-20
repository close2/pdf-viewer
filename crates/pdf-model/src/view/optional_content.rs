//! Optional content: §8.11.4.5's initial configuration, §8.11.4.4's usage application under the
//! reader's audience and purpose, the snapshot a replay restores, and §12.6.4.13's group state.

use pdf_syntax::ObjectId;

use crate::action::Change;
use crate::optional_content::OptionalContent;

use super::ViewState;

impl ViewState {
    /// §8.11.4.5's reapplication against whatever the three factors now are.
    pub(super) fn reapply_usage(&mut self) -> bool {
        let (magnification, purpose) = (self.magnification, self.purpose);
        self.optional_content
            .as_mut()
            .is_some_and(|content| content.reapply(magnification, &self.audience, purpose))
    }

    /// The optional content configuration, as the state currently stands.
    #[must_use]
    pub fn optional_content(&self) -> Option<&OptionalContent> {
        self.optional_content.as_ref()
    }

    /// §12.4.4.2 NOTE 2: the optional content state, to be given back when a presentation ends.
    ///
    /// > Interactive PDF processors need to save the state of optional content groups when a user
    /// > enters presentation mode and restore it when presentation mode ends.
    ///
    /// and the NOTE says what for: "transient changes to bullets do not affect the printing of the
    /// document". A presentation's steps are §12.6.4.13's actions on §8.11's groups (NOTE 1), so
    /// what a person saw during the slide show would otherwise be the document's state afterwards.
    ///
    /// The whole configuration rather than a map of states, because it is a function of the
    /// document and of what has been performed against it and nothing else, so giving it back *is*
    /// restoring the groups. `None` is a document with no `/OCProperties`, which §8.11.4.2 makes
    /// decisive and which has no state to save.
    #[must_use]
    pub fn optional_content_snapshot(&self) -> Option<OptionalContent> {
        self.optional_content.clone()
    }

    /// Puts back what [`Self::optional_content_snapshot`] took.
    ///
    /// Returns whether anything changed, so a caller can decide whether the page has to be
    /// interpreted again rather than comparing two configurations itself — the same shape
    /// [`Self::set_widget_appearances`] has, and for the same reason: §8.11 decides what is drawn.
    ///
    /// A configuration carries §8.11.4.4's event overlay with it, so the restored one is put back
    /// against whatever [`Self::purpose`] is *now* rather than whatever it was when the snapshot
    /// was taken — which is what keeps the two in step wherever a state is written.
    pub fn restore_optional_content(&mut self, snapshot: Option<OptionalContent>) -> bool {
        let changed = self.optional_content != snapshot;
        self.optional_content = snapshot;
        self.reapply_usage() || changed
    }

    /// Sets a group's state the way a *layer panel* does, and answers whether it changed.
    ///
    /// This is §8.11.4.5's other half — "\[t]he user may manipulate optional content group states
    /// manually or by triggering set-OCG-state actions" — and the two differ in exactly one
    /// respect, which is why they are two functions. Table 99's `/Locked` says "\[t]he state of a
    /// locked group cannot be changed through the user interface of an interactive PDF
    /// processor", and the clause's next sentence permits the other route: "\[a]n interactive PDF
    /// processor may allow the states of optional content groups to be changed by means other
    /// than the user interface, such as ECMAScript or items in the AS entry". So a lock stops
    /// this and does not stop [`crate::action::Action::SetOcgState`].
    ///
    /// Radio-button collections apply either way. `/PreserveRB` is a set-OCG-state action's
    /// entry and has no counterpart for a person; Table 99 states the paradigm unconditionally,
    /// so a panel that let two members of one collection be on would be showing a state the
    /// configuration says cannot exist.
    ///
    /// `false` for a group the document never declared, one the configuration's `/Intent` does
    /// not cover, and one `/Locked` names — three different reasons a switch does nothing, and
    /// a panel that wants to distinguish them has [`OptionalContent::is_locked`] and
    /// [`OptionalContent::state`].
    pub fn set_group(&mut self, group: ObjectId, on: bool) -> bool {
        let Some(content) = self.optional_content.as_mut() else {
            return false;
        };
        if content.is_locked(group) || content.state(group).is_none_or(|was| was == on) {
            return false;
        }
        content.apply(&[(group, if on { Change::On } else { Change::Off })], true);
        // A manual change writes the view-side states, and an event's overlay stands over those
        // — so it is recomputed here rather than left holding the value the change replaced.
        self.reapply_usage();
        true
    }
}
