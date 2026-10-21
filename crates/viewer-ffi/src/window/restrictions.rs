//! The shared permissions menu, read when a host opens it and changed by a chosen row.

use viewer_host::native_window::NativeWindow;

use super::Window;
use crate::Status;

impl Window {
    /// The shared menu rows, including headings and checked levels, in their display order.
    #[must_use]
    pub fn restriction_rows(&self) -> Vec<viewer_host::Row> {
        self.state.restrictions.rows()
    }

    /// Chooses a level from the shared menu. Headings never change a policy.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] for a row outside the menu, [`Status::WrongKind`] for a heading.
    pub fn choose_restriction(&mut self, index: usize) -> Result<(), Status> {
        match self
            .restriction_rows()
            .get(index)
            .copied()
            .ok_or(Status::OutOfRange)?
        {
            viewer_host::Row::Level(entry) => {
                let command = self.state.restrictions.chose(entry.chose);
                self.dispatch(command);
                self.say(&viewer_host::chosen(entry.chose));
            }
            viewer_host::Row::ActLevel(entry) => {
                self.state.restrictions.set(entry.level);
                if let Some(command) = viewer_host::told(entry.level) {
                    self.dispatch(command);
                }
                self.say(&viewer_host::act_chosen(entry.level));
            }
            _ => return Err(Status::WrongKind),
        }
        self.settle();
        Ok(())
    }
}
