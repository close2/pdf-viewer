//! What a copy, a cut and a paste carry: the page's selection and a field's own range, and the platform end both reach.

use viewer_core::{Answer, Command, Query};

use super::spliced;
use crate::app::App;

impl App {
    /// Puts what is selected on the page into this program's clipboard.
    ///
    /// **A consumer of [`Query::LogicalSelection`]**, because a query no host asks does not make
    /// §14.8.2.5's ledger row `implemented` on its own — `doc/todo/01`'s fifth sweep, "the model
    /// implements this, who calls it?" (ADR 0249). A clause reaches a person through a program or
    /// it does not reach one.
    ///
    /// ISO 32000-2 §14.8.2.5 defines the two orders and this host chooses between them:
    ///
    /// > Page content order shall be defined by the sequencing of graphics objects within a
    /// > page's content stream.
    ///
    /// > Logical content order -the ordering for semantic purposes -shall be defined by a
    /// > depth-first traversal of the document's logical structure
    ///
    /// A selection is taken in page content order, because that is the order the shapes are in;
    /// a *copy* off a page whose producer wrote its columns out of order wants the other one.
    /// The core answers [`Answer::None`] where the document states no structure tree or where
    /// the tree does not reach every byte of the selection, and [`viewer_host::copied`] is what
    /// that means here: the order it could give, named out loud, rather than a silent
    /// rearrangement.
    ///
    /// **The choice between the two orders is `viewer_host::copying`'s** (ADR 0519): it is the same
    /// decision in all three windowed hosts, and a copy per host is where two of them stop
    /// agreeing. What stays here is the two questions and the platform, which is what a host
    /// actually owns.
    ///
    /// **And the text now leaves the program.** `viewer_ui::clipboard` is this host's end of the
    /// session's clipboard, connected on this line and no earlier; the in-process string is kept
    /// beside it because it is what Ctrl + V pastes into a field, and a paste that asked the
    /// platform would put another program's response time inside a keystroke. A platform that
    /// refuses is reported rather than swallowed, and the copy still happened *inside* the
    /// program, which is the honest thing to say about it.
    pub(crate) fn copy_selection(&mut self) {
        // **A command rather than two questions**: §7.6.4.2's bit 5 restricts taking text out of
        // the document and a readback can be neither refused, asked about nor warned of, so the
        // gesture is `Command::Copy` and both orders arrive together on `Event::Copied` (ADR 1144).
        // What stays here is the platform and the sentence, which is what a host actually owns.
        if !matches!(self.viewer.query(Query::Selection), Answer::Selected(_)) {
            println!("note: nothing on the page is selected to copy");
            return;
        }
        self.dispatch(Command::Copy);
    }

    /// [`viewer_core::Event::Copied`]: the text the core granted, onto this session's clipboard.
    pub(crate) fn copied(&mut self, logical: Option<String>, page_order: &str) {
        let Some(copied) = viewer_host::copied(logical, page_order) else {
            return;
        };
        println!(
            "note: copied {} characters in {}",
            copied.text.chars().count(),
            copied.order
        );
        if let Err(error) = self.platform_clipboard.put(&copied.text) {
            println!("note: {error}, so the copy stayed inside this program");
        }
        self.clipboard = copied.text;
    }

    /// Ctrl + C, X and V inside a field's value: what the press does to the value, if anything.
    ///
    /// **The finding this round records: copying, cutting and pasting inside a field needed no
    /// message.** The two offsets are into the value `Query::FieldAt` just answered with, so the
    /// characters between them are a slice this host already holds; cutting and pasting are that
    /// slice spliced out or in and sent back as exactly the `Edit::SetField` every keystroke
    /// sends. Nothing crosses the boundary that did not cross it before, which is why ADR 0225
    /// added two questions and no verbs.
    ///
    /// `None` for a character this does not answer, which the caller consumes rather than passing
    /// on. A copy answers `None` for a different reason and says so where it happens: it changes
    /// no value, so there is nothing for the caller to send.
    pub(super) fn clipped(
        &mut self,
        text: &str,
        current: &str,
        range: (usize, usize),
    ) -> Option<(Option<String>, usize, usize)> {
        let (low, high) = range;
        match text {
            "c" | "x" => {
                current
                    .get(low..high)
                    .unwrap_or_default()
                    .clone_into(&mut self.clipboard);
                println!(
                    "note: {} {} bytes out of the field",
                    if text == "c" { "copied" } else { "cut" },
                    self.clipboard.len()
                );
                // **The same platform end a copy off the page takes** (ADR 0519): text a person
                // took out of §12.7.4.3's value is text they meant to paste somewhere, and a
                // viewer whose form fields copy only into themselves is the half-feature this
                // round exists to close. Table 231 bit 14's password field never reaches here —
                // `aim_at_field` refuses the keyboard to one — so nothing this puts on the
                // session's clipboard is a value the document said to obscure.
                if let Err(error) = self.platform_clipboard.put(&self.clipboard) {
                    println!("note: {error}, so the copy stayed inside this program");
                }
                if text == "c" {
                    self.redraw();
                    return Some((None, low, high));
                }
                Some((Some(spliced(current, low, high, "")), low, low))
            }
            "v" => {
                let to = low.saturating_add(self.clipboard.len());
                Some((Some(spliced(current, low, high, &self.clipboard)), to, to))
            }
            _ => None,
        }
    }
}
