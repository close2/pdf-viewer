//! The half of a native host that is not the toolkit.
//!
//! # Why this crate exists, and what discovered it
//!
//! `crates/viewer-gtk` was written in the four-hundred-and-eighth session as eight modules, and
//! four of them named no GTK type at all: the three panel answers turned into one row shape, a
//! §12.7 field decided into the control it is, §12.7.6.4's file policy, and a launch timeline.
//! They were kept toolkit-free deliberately — it is the only part of a native host a workspace
//! test suite can see without a display — but nothing said whether that was a fact about GTK or a
//! fact about *hosts*.
//!
//! The four-hundred-and-tenth session answered it by writing the second host. `crates/viewer-qt`
//! is Qt 6 through a C++ bridge: a different widget model, a different ownership model, a
//! different language on the far side — and it wanted all four modules unchanged. So they are
//! here, depended on by both hosts and by neither toolkit, and `viewer-gtk`'s public interface is
//! now `Host` and `HostError` and nothing else.
//!
//! **This is not [`viewer_core`]'s work and deliberately not in it.** That crate is a
//! *vocabulary* — `Command` in, `Event` out, `Query` → `Answer` beside them — and a mapping from
//! three of its answers into one row shape is a convenience for whoever draws a tree, not a
//! statement about a document. Putting it in the core would make the core answer a question no
//! clause asks. Putting it in each host would have it written twice, and the second copy is where
//! two hosts stop agreeing about what §12.3.3's `/Count` sign means.
//!
//! # What is in it
//!
//! - [`arrangement`] — moving between Table 29's six `/PageLayout` values, which the clause states
//!   and says nothing about cycling. The third host would have been the third copy.
//! - [`panel`] — §12.3.3's outline, §8.11.4.3's `/Order` and §7.11.4's embedded files, as one
//!   [`PanelRow`] tree with a [`RowAction`] per row. Three answers, three types, one shape a
//!   platform tree can hold.
//! - [`form`] — §12.7.5's field, decided into the [`ControlKind`] a platform builds. One variant
//!   per *control* rather than one per clause type, because the clause's choice field is two
//!   controls and its button field is three.
//! - [`presentation`] — Table 29's `/PageMode /FullScreen`, §12.2's three chrome flags and the
//!   page mode §12.2 says to come back to. The *toolkit call* differs in all three hosts and
//!   which sentence a window is obeying does not, which is this crate's test applied to a window.
//! - [`clock`] — §12.4.4.1's own clock: how often a presentation is ticked, what a tick carries,
//!   and how long Table 164's transition has left to run. `viewer-core` has none by rule 3, and
//!   the three event loops that could supply one — `glib::timeout_add_local`, `QTimer`, winit's
//!   `ControlFlow::WaitUntil` — agree about every question the clause asks and differ in every
//!   letter of how it is asked.
//! - [`copying`] — §14.8.2.5's two content orders, and which of them the text a person copies out
//!   of the program is in. The *clipboard* is a platform surface and there are four of them here;
//!   which order to hand it, and what to say about the one it did not get, is one decision.
//! - [`fit`] — the magnification at which every §12.7 control fits the `/Rect` its document states
//!   for it, from the rectangles `Query::Fields` answers with and the minimum sizes a toolkit
//!   measures. ADR 0245's scale question, answered with the messages that already existed.
//! - [`policy`] — §12.7.6.4's import-data file, under the narrowest policy that still performs
//!   the action, and §O.2.1's embedded file, which a URI may name and a person may not have.
//!   `viewer_core`'s rule 2 is that the crate has no filesystem, so this is where that rule
//!   reaches a person.
//! - [`status`] — what the pages on the screen could not draw, worded for a status bar. One
//!   sentence, three widgets: `Query::Reports` answers per page since Table 29's arrangements
//!   were obeyed, and a note that did not say which page it was about would be a note about one
//!   of four.
//! - [`trace`] — `--trace=<topics>`, in the line format `viewer-ui` prints, so that two hosts'
//!   launch timelines can be read side by side. `CLAUDE.md` makes the launch path a measured
//!   thing and a host is a program a person runs.
//!
//! # What is *not* in it, and why
//!
//! No widget, no window, no event loop, and no pixel format. Each of those is where the two hosts
//! genuinely differ — `gdk::MemoryTexture` against `QImage`, `GtkTreeListModel`'s lazy children
//! against `QAbstractItemModel`'s eager ones — and a shared abstraction over them would be an
//! invention rather than a finding. ADR 0246 records what the second host asked of the boundary
//! and what it asked of this crate, which was nothing.

#![forbid(unsafe_code)]

pub mod arrangement;
pub mod clock;
pub mod copying;
pub mod fit;
pub mod form;
pub mod panel;
pub mod policy;
pub mod presentation;
pub mod status;
pub mod trace;

pub use arrangement::next_layout;
pub use clock::{Clock, face_target};
pub use copying::{ContentOrder, Copied, copied};
pub use fit::ControlFit;
pub use form::{ControlKind, control_kind};
pub use panel::{PanelRow, RowAction, attachment_rows, layer_rows, outline_rows};
pub use policy::{
    ImportRefusal, may_open_extracted, may_write_extracted, read_import, resolve_import,
};
pub use presentation::{Chrome, Presenting};
pub use trace::{Topic, Trace, parse_topics};
