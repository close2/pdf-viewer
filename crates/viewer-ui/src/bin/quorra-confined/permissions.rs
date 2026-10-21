//! The machine-side launch policy, adjustable without a round trip to the worker.

use viewer_host::{Act, ActLevel, Restrictions};
use winit::keyboard::{Key, NamedKey};

use super::Host;

impl Host {
    fn permissions_rows(&self) -> Vec<viewer_host::Row> {
        Restrictions::new(viewer_core::RestrictionPolicy::default())
            .with(ActLevel::LaunchedDocuments(self.launched_documents))
            .act_rows(Act::LaunchedDocuments)
    }

    /// Modal cards and text entry keep their keys; a menu choice never waits on the worker.
    pub(super) fn permissions_key(&mut self, key: &Key) -> bool {
        if self.password.shown || self.question.shown || self.typing.is_some() {
            return false;
        }
        if matches!(key.as_ref(), Key::Character("r" | "R")) {
            self.permissions.toggle(self.permissions_rows());
        } else if self.permissions.shown {
            match key {
                Key::Named(NamedKey::Escape) => self.permissions.hide(),
                Key::Named(NamedKey::ArrowDown) => {
                    self.permissions.move_by(1);
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.permissions.move_by(-1);
                }
                Key::Named(NamedKey::Enter) => self.permissions_choose(),
                _ => {}
            }
        } else {
            return false;
        }
        self.redraw();
        true
    }

    fn permissions_choose(&mut self) {
        if let Some(ActLevel::LaunchedDocuments(level)) = self.permissions.act_level() {
            self.launched_documents = level;
            eprintln!(
                "{}",
                viewer_host::act_chosen(ActLevel::LaunchedDocuments(level))
            );
            self.permissions.refill(self.permissions_rows());
        }
    }

    pub(super) fn permissions_pointer(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let extent = window.inner_size();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale is a small ratio"
        )]
        let scale = window.scale_factor() as f32;
        if self
            .permissions
            .press(self.cursor, extent.width, extent.height, scale)
        {
            self.permissions_choose();
            self.redraw();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, time::Instant};

    #[test]
    fn launch_menu_changes_the_policy_without_a_worker() {
        let mut host = Host::new(
            PathBuf::from("unopened.pdf"),
            0,
            true,
            false,
            Instant::now(),
        );
        assert!(host.permissions_key(&Key::Character("r".into())));
        assert!(host.permissions.shown);
        for level in viewer_host::LaunchedDocuments::ALL {
            assert!(host.permissions_key(&Key::Named(NamedKey::Enter)));
            assert_eq!(host.launched_documents, level);
            host.permissions_key(&Key::Named(NamedKey::ArrowDown));
        }
        host.permissions_key(&Key::Named(NamedKey::Escape));
        assert!(!host.permissions.shown);
    }
}
