//! The restrictions menu: the reader's levels for what a document asserts, a scope per heading,
//! and the choice applied to the window or to the document in front (ADR 1145; filed here by ADR
//! 1908).

use super::Host;

impl Host {
    /// The headings of the restrictions menu bar: one per group the menu's rows open, the two
    /// scopes and then what a document may ask this machine to do.
    pub(crate) fn restriction_scopes(&self) -> Vec<String> {
        self.restrictions
            .headings()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    /// Every entry of the restrictions menu, flat, as the C++ side nests it.
    ///
    /// Built when the menu is opened and never held, which is `CLAUDE.md` section 2's rule: what
    /// it holds is a function of two policies that change while the window is up.
    pub(crate) fn restriction_menu(&self) -> Vec<crate::bridge::ffi::QtMenuEntry> {
        self.restrictions
            .rows()
            .into_iter()
            .map(|row| match row {
                viewer_host::Row::Scope { label, note, .. }
                | viewer_host::Row::Machine { label, note } => crate::bridge::ffi::QtMenuEntry {
                    depth: 0,
                    label: label.to_owned(),
                    note: note.to_owned(),
                    chosen: false,
                },
                viewer_host::Row::Operation { label, note, .. }
                | viewer_host::Row::Act { label, note } => crate::bridge::ffi::QtMenuEntry {
                    depth: 1,
                    label: label.to_owned(),
                    note: note.to_owned(),
                    chosen: false,
                },
                viewer_host::Row::Level(entry) => crate::bridge::ffi::QtMenuEntry {
                    depth: 2,
                    label: entry.label.to_owned(),
                    note: String::new(),
                    chosen: entry.chosen,
                },
                viewer_host::Row::ActLevel(entry) => crate::bridge::ffi::QtMenuEntry {
                    depth: 2,
                    label: entry.label.to_owned(),
                    note: String::new(),
                    chosen: entry.chosen,
                },
            })
            .collect()
    }

    /// A person picked one of [`Host::restriction_menu`]'s entries.
    ///
    /// By index, which is sound because the *set* of entries and their order do not depend on the
    /// policy — two scopes, every operation, four levels and one way back — and only what is
    /// ticked does. A heading's index chooses nothing, which is what a `QMenu` title is.
    pub(crate) fn chose_restriction(&mut self, entry: usize) {
        let picked = match self.restrictions.rows().get(entry).copied() {
            Some(viewer_host::Row::Level(picked)) => picked,
            // An act's row sets a level this host reads, and tells the viewer only of the act it
            // does itself — a script runs in its view state (ADRs 1291, 1331, 1616).
            Some(viewer_host::Row::ActLevel(act)) => {
                self.restrictions.set(act.level);
                if let Some(command) = viewer_host::told(act.level) {
                    self.dispatch(command);
                }
                self.say(&viewer_host::act_chosen(act.level));
                return;
            }
            _ => return,
        };
        let command = self.restrictions.chose(picked.chose);
        self.dispatch(command);
        self.say(&viewer_host::chosen(picked.chose));
    }
}
