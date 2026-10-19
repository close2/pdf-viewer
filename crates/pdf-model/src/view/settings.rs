//! The settings a window states about how the document is read: widget appearances, magnification,
//! audience, purpose, paper, separation simulation and the modification time.
//!
//! None of these is written back to the file. Each is one input to the next render, held beside the
//! document in [`ViewState`] (see this module's parent), and each setter says whether the setting
//! changed so that a host re-renders only when it must.

use crate::optional_content::{Audience, Purpose};

use super::{Magnified, TargetMedia, ViewState, WidgetAppearances};

impl ViewState {
    /// Whether §12.7's form widgets are drawn on the page, or left to the host.
    #[must_use]
    pub fn widget_appearances(&self) -> WidgetAppearances {
        self.widget_appearances
    }

    /// Says who draws §12.7's form widgets.
    ///
    /// [`WidgetAppearances::Drawn`] until a host says otherwise, so a caller that never calls
    /// this draws the page §6.3.2.2 describes — which is what makes the corpus gate, the oracle
    /// and every other existing caller produce the display list they produced before this
    /// existed, by construction rather than by comparison.
    ///
    /// Returns whether the value changed, so a caller can decide whether the page has to be
    /// interpreted again — the same shape [`ViewState::set_magnification`] has, and for the same
    /// reason: this decides what is *drawn*, so a change invalidates a display list rather than
    /// only the pixels made from it.
    pub fn set_widget_appearances(&mut self, appearances: WidgetAppearances) -> bool {
        let changed = self.widget_appearances != appearances;
        self.widget_appearances = appearances;
        changed
    }

    /// How large the page is being drawn, where a caller has said.
    ///
    /// See [`ViewState::set_magnification`] for why `None` is not 1.0.
    #[must_use]
    pub fn magnification(&self) -> Option<f32> {
        self.magnification
    }

    /// Says how large the page is being drawn, in logical pixels per default user space unit.
    ///
    /// Two clauses read it. §12.5.3's `NoZoom` moves one annotation — which is why a host that
    /// never zooms need never call this and a host that does may call it on every frame, and
    /// `Interpretation::view_dependent` says whether this page has anything that would notice.
    /// And §8.11.4.4's `Zoom` category decides whether a layer is drawn at all, which §8.11.4.5
    /// requires to be asked again every time this moves: "[w]henever there is a change to a
    /// factor that the usage application dictionaries with event type View depend on (such as
    /// zoom level), the corresponding dictionaries shall be reapplied".
    ///
    /// **The reapplication happens here rather than in a second call a caller has to remember**,
    /// which is what makes it a property of the magnification changing rather than of a host
    /// being diligent. It costs one empty-vector test on a document that states no `/AS`.
    ///
    /// The answer separates the two, because they oblige a caller differently; see
    /// [`Magnified`].
    pub fn set_magnification(&mut self, magnification: Option<f32>) -> Magnified {
        if self.magnification == magnification {
            return Magnified::Unchanged;
        }
        self.magnification = magnification;
        if self.reapply_usage() {
            Magnified::Visibility
        } else {
            Magnified::Placement
        }
    }

    /// §8.11.4.4's answers about this processor, as a host has given them.
    #[must_use]
    pub fn audience(&self) -> &Audience {
        &self.audience
    }

    /// Says who is reading and in what language, for Table 100's `/User` and `/Language`.
    ///
    /// Returns whether any group's state moved, which is what a caller needs: §8.11 decides
    /// what is *drawn*, so an answer that changes a state supersedes the ink the page already
    /// produced. A host that never calls this leaves both categories unanswered, which is the
    /// position every caller of [`ViewState::of`] is in.
    pub fn set_audience(&mut self, audience: Audience) -> bool {
        if self.audience == audience {
            return false;
        }
        self.audience = audience;
        self.reapply_usage()
    }

    /// What the output being produced is for; see [`Self::set_purpose`].
    #[must_use]
    pub fn purpose(&self) -> Purpose {
        self.purpose
    }

    /// Says what the output being produced is for, for the duration of producing it.
    ///
    /// Two clauses read it, and they are the two the answer exists for. §8.11.4.4's `Print` and
    /// `Export` usage application dictionaries are "applied over the current states of optional
    /// content groups" and their changes "persist only for the duration" of the operation; and
    /// §8.9.5.4 step c) selects an image's `/DefaultForPrinting` alternate when "the PDF is being
    /// printed".
    ///
    /// **An operation states it, holds it while it runs, and states [`Purpose::View`] again** —
    /// or, as `quorra-transform`'s writers do, works through a state of its own that never was a
    /// reader's. The revert the clause requires costs nothing either way, because the event is
    /// an overlay rather than a write: see [`crate::optional_content::OptionalContent::reapply`].
    /// Nothing in this crate infers the answer, which is `CLAUDE.md` principle 3's rule and
    /// [`Audience`]'s (ADR 1106): a renderer that decided for itself what a person was doing would
    /// be answering a question nobody asked it.
    ///
    /// Returns whether any §8.11 group's state moved, which is what a caller that shows a layer
    /// panel needs. **It is not the test for whether the page has to be drawn again**, and the
    /// second clause is why: §8.9.5.4's step c) selects a different image without any group's
    /// state moving, so a change of purpose supersedes the ink whatever this answers. ADR 1173.
    pub fn set_purpose(&mut self, purpose: Purpose) -> bool {
        if self.purpose == purpose {
            return false;
        }
        self.purpose = purpose;
        self.reapply_usage()
    }

    /// §12.5.6.22's target media, as the operation producing the output knows it.
    ///
    /// Table 193 makes the dimensions of that media the thing a fixed print watermark is placed
    /// against, and states what to do without them:
    ///
    /// > If the dimensions of the target media are not known at the time of drawing, drawing
    /// > shall be done relative to the dimensions specified by the page's MediaBox entry
    ///
    /// So `None` is *not known*, which is every caller that renders for a screen — §12.5.6.22
    /// requires that one by name, "[w]hen displaying a watermark annotation on-screen,
    /// interactive PDF processors shall use the dimensions of the media box ... so that the
    /// scroll and zoom behaviour is the same as for other annotations" — and `Some` is a sheet
    /// somebody chose, in default user space units.
    ///
    /// **Stated beside [`Self::set_purpose`] and read only under [`Purpose::Print`]**, because
    /// the sentence the substitution comes from is about a printed page: a paper size stated
    /// while the purpose is still [`Purpose::View`] would move a mark on the screen that
    /// §12.5.6.22's on-screen sentence fixes to the media box. ADR 1179.
    ///
    /// Returns nothing to redraw by itself: the caller that states a paper size is the caller
    /// that is about to ask for the pages, and a change of purpose supersedes the ink already
    /// produced whatever this says.
    pub fn set_paper(&mut self, paper: Option<TargetMedia>) {
        self.paper = paper;
    }

    /// §12.5.6.22's target media, or `None` where nobody has stated one; see [`Self::set_paper`].
    #[must_use]
    pub fn paper(&self) -> Option<TargetMedia> {
        self.paper
    }

    /// Says whether §10.8.3's separation simulation is being asked for.
    ///
    /// **A person's request, which is what the clause conditions itself on**: the simulation is
    /// for when "it is important for the colours of the display for a PDF, on a device that
    /// normally would not be used to produce separations, to more closely match those produced
    /// when using separations", and no document states that. So it arrives the way
    /// [`Self::set_audience`] and [`Self::set_purpose`] do and never by inference here, which is
    /// `CLAUDE.md` principle 3's rule: the policy is asked once, in a place a host can supply.
    ///
    /// Returns whether the answer moved, which is what a caller needs — §10.8.3 decides what
    /// colour every mark on the page is, so an answer that changes supersedes the ink already
    /// produced. ADR 1228.
    pub fn set_separation_simulation(&mut self, simulate: bool) -> bool {
        if self.separation_simulation == simulate {
            return false;
        }
        self.separation_simulation = simulate;
        true
    }

    /// Whether §10.8.3's separation simulation is being asked for; see
    /// [`Self::set_separation_simulation`].
    #[must_use]
    pub fn separation_simulation(&self) -> bool {
        self.separation_simulation
    }

    /// What a host's clock says, for Table 166's `/M` on what [`Self::save`] writes.
    ///
    /// The entry is "[t]he date and time when the annotation was most recently modified", and
    /// `CLAUDE.md`'s rule 3 is why it arrives this way rather than being read: a renderer with no
    /// clock cannot know when anything happened, and an instant invented here would be a claim
    /// about a machine dressed as a fact about a document.
    ///
    /// **The instant written is the one last stated, and that is the whole of the contract.** So a
    /// host reads its clock immediately before the save and states what it read; a host that
    /// states nothing writes no `/M` at all, which is what every caller of [`Self::of`] does until
    /// one does otherwise. Nothing is derived from it, nothing expires, and no page is drawn
    /// differently for it. ADR 1160.
    pub fn set_modification_time(&mut self, at: Option<pdf_syntax::Date>) {
        self.modified = at;
    }

    /// The instant a host last stated, for Table 166's `/M`.
    #[must_use]
    pub fn modification_time(&self) -> Option<pdf_syntax::Date> {
        self.modified
    }
}
