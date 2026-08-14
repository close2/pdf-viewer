//! The scale a form host draws the page at, and the arithmetic that decides it.
//!
//! **The question ADR 0245 left open, and the answer is that the boundary already had it.** A
//! platform control has a minimum size its theme decides — `gtk_widget_set_size_request` is a
//! *floor* and `QWidget::minimumSizeHint` is one too — so a control placed over a widget's `/Rect`
//! can be larger than the rectangle and cover the page around it. ADR 0244 measured it on GTK and
//! ADR 0246 on Qt, and the two agree that *every* control is taller than its rectangle on some
//! forms, which is what makes it a property of platform controls rather than of a theme.
//!
//! That is not `viewer-core`'s to fix and not a control to shrink. What it means is that a native
//! form host must also choose the **magnification** the page is drawn at, and `doc/todo/30` asked
//! whether choosing it needs a message the vocabulary does not have. It does not, and the whole
//! argument is four steps:
//!
//! 1. `viewer_core::Query::Fields` already answers with every widget's `/Rect` **in device pixels
//!    of the viewport**, which is the rectangle the control was asked to occupy;
//! 2. the control's minimum size is the *toolkit's* answer and no clause's — each host asks its own
//!    and neither `viewer-core` nor `pdf-model` could;
//! 3. a control's minimum does **not** change with the page's magnification and the rectangle's
//!    size does, in proportion — so a widget asked for `w` logical pixels at magnification `m`
//!    needs `m × minimum / w` to fit, and the worst of those over the page is the magnification at
//!    which every control fits. That is [`ControlFit::magnification`], and it is arithmetic;
//! 4. `viewer_core::Command::Zoom` with `viewer_core::Zoom::Scale` applies it.
//!
//! **Nothing in that list is new.** The one piece that did not exist is step 3, and it is here
//! rather than in a host for `panel.rs`'s reason: two hosts measuring the same thing must not be
//! able to compute two different answers from it, and a mapping from rectangles to a number is not
//! a statement about a document.
//!
//! # What it does not decide
//!
//! **When** to apply it. A viewer that silently magnified a page because a form is on it would be
//! answering a question nobody asked, and which gesture asks for it — a key, a menu item, a
//! preference — is chrome and therefore the host's (rule 5). This crate answers *what the number
//! is*; a host decides whether a person wanted it.

/// How the controls placed over one page fit the rectangles the document states for them.
///
/// Fed one control at a time in **logical** pixels, which is what a toolkit lays out in: the device
/// pixels `Query::Fields` answers with are divided by the display's scale before a control is
/// asked for a size, and a minimum measured in logical pixels has to be compared against the same
/// units.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ControlFit {
    /// How many controls were placed.
    placed: usize,
    /// How many are wider than their rectangle.
    wider: usize,
    /// How many are taller.
    taller: usize,
    /// The largest excess width, and the rectangle's own width beside it.
    widest: (i32, i32),
    /// The largest excess height, and the rectangle's own height beside it.
    tallest: (i32, i32),
    /// The largest ratio of a control's minimum to the extent it was asked for, over both axes.
    ///
    /// One number rather than two, because a page is magnified in both directions at once: what a
    /// host needs is the single factor that makes the worst control fit, and which axis it was
    /// worst in is the report's business rather than the zoom's.
    worst_ratio: f32,
}

impl ControlFit {
    /// Measures one control against the rectangle it was asked to occupy.
    ///
    /// Both pairs are logical pixels: `asked` is the widget's `/Rect` at the magnification showing
    /// now, `minimum` is what the toolkit says the control cannot be smaller than.
    pub fn record(&mut self, asked: (i32, i32), minimum: (i32, i32)) {
        self.placed = self.placed.saturating_add(1);
        if minimum.0 > asked.0 {
            self.wider = self.wider.saturating_add(1);
            if minimum.0.saturating_sub(asked.0) > self.widest.0 {
                self.widest = (minimum.0.saturating_sub(asked.0), asked.0);
            }
        }
        if minimum.1 > asked.1 {
            self.taller = self.taller.saturating_add(1);
            if minimum.1.saturating_sub(asked.1) > self.tallest.0 {
                self.tallest = (minimum.1.saturating_sub(asked.1), asked.1);
            }
        }
        for (extent, least) in [(asked.0, minimum.0), (asked.1, minimum.1)] {
            // A rectangle of no extent is one no magnification can make large enough, and a
            // control asking for nothing already fits. Both are skipped rather than made into an
            // infinity a host would then have to defend itself against.
            if extent <= 0 || least <= 0 {
                continue;
            }
            let ratio = ratio_of(least, extent);
            if ratio > self.worst_ratio {
                self.worst_ratio = ratio;
            }
        }
    }

    /// How many controls were placed, how many overflow, and by how much.
    ///
    /// `(placed, wider, taller, worst excess width and its rectangle, worst excess height and
    /// its rectangle)` — the numbers ADR 0244 and ADR 0246 took by hand.
    #[must_use]
    pub const fn counts(&self) -> (usize, usize, usize, (i32, i32), (i32, i32)) {
        (
            self.placed,
            self.wider,
            self.taller,
            self.widest,
            self.tallest,
        )
    }

    /// The magnification at which **every** control fits its rectangle, or `None` where they
    /// already do.
    ///
    /// `showing` is the magnification the page is drawn at now, in the units
    /// `viewer_core::Zoom::Scale` takes — logical pixels per PDF user space unit, where 1.0 is
    /// 72 dpi — so the answer goes straight back as
    /// `Command::Zoom { zoom: Zoom::Scale(that), at: None }`.
    ///
    /// `None` for a page with no controls on it and for one where nothing overflows, which are
    /// two different situations and the same answer: neither is a page to magnify.
    #[must_use]
    pub fn magnification(&self, showing: f32) -> Option<f32> {
        if self.placed == 0 || self.worst_ratio <= 1.0 || !showing.is_finite() || showing <= 0.0 {
            return None;
        }
        let wanted = showing * self.worst_ratio;
        wanted.is_finite().then_some(wanted)
    }
}

/// One control's minimum as a multiple of the extent it was asked for.
///
/// Written out because `clippy::cast_precision_loss` is denied in this workspace and both numbers
/// are widget extents inside a window — far inside what an `f32` represents exactly.
#[expect(
    clippy::cast_precision_loss,
    reason = "a control's extent in logical pixels is at most a few thousand, which f32 holds \
              exactly; the lint is about the general case"
)]
fn ratio_of(minimum: i32, asked: i32) -> f32 {
    minimum as f32 / asked as f32
}

#[cfg(test)]
mod tests {
    use super::ControlFit;

    /// A page whose controls all fit asks for no magnification, which is not the same as asking
    /// for 1.0.
    #[test]
    fn a_form_that_fits_wants_no_magnification() {
        let mut fit = ControlFit::default();
        fit.record((100, 30), (80, 30));
        fit.record((60, 24), (60, 20));
        assert_eq!(fit.magnification(1.5), None);
        assert_eq!(fit.counts().0, 2);
        assert_eq!((fit.counts().1, fit.counts().2), (0, 0));
    }

    /// An empty page asks for none either, and for a different reason.
    #[test]
    fn a_page_with_no_controls_on_it_is_not_a_page_to_magnify() {
        assert_eq!(ControlFit::default().magnification(1.0), None);
    }

    /// The worst control decides, and it decides in whichever axis it is worst in.
    ///
    /// The three here overflow by 1.25× in width, 2× in height and not at all; 2× is the answer,
    /// and at that magnification the first one's width has 1.6× the room it needed.
    #[test]
    fn the_worst_control_decides_the_magnification() {
        let mut fit = ControlFit::default();
        fit.record((80, 40), (100, 20));
        fit.record((120, 12), (60, 24));
        fit.record((200, 50), (50, 20));
        let wanted = fit.magnification(1.0).expect("something overflows");
        assert!(
            (wanted - 2.0).abs() < 1e-6,
            "the tallest control is half the height it needs: {wanted}"
        );
        // And the answer scales with the magnification showing, because a rectangle in logical
        // pixels does and a control's minimum does not.
        let doubled = fit.magnification(2.0).expect("something overflows");
        assert!((doubled - 4.0).abs() < 1e-6, "{doubled}");
    }

    /// At the magnification it answers with, every control fits — which is the property, checked
    /// rather than argued.
    #[test]
    fn at_the_magnification_it_answers_with_every_control_fits() {
        let controls = [
            ((80, 40), (100, 20)),
            ((120, 12), (60, 24)),
            ((200, 50), (50, 20)),
            ((18, 14), (84, 34)),
        ];
        let mut fit = ControlFit::default();
        for (asked, minimum) in controls {
            fit.record(asked, minimum);
        }
        let showing = 1.0_f32;
        let wanted = fit.magnification(showing).expect("something overflows");
        let grown = wanted / showing;
        for (asked, minimum) in controls {
            let width = f64::from(asked.0) * f64::from(grown);
            let height = f64::from(asked.1) * f64::from(grown);
            assert!(
                width + 1e-3 >= f64::from(minimum.0) && height + 1e-3 >= f64::from(minimum.1),
                "{asked:?} at {grown}× is {width}x{height}, and the control wants {minimum:?}"
            );
        }
    }

    /// A rectangle of no extent is skipped rather than made into an infinity.
    ///
    /// §12.5.6.19 lets a widget state a degenerate `/Rect` and 147 corpus widgets are empty text
    /// fields; a host that received `inf` here would have to defend itself against it, and no
    /// magnification makes a control fit a rectangle of zero width anyway.
    #[test]
    fn a_rectangle_with_no_extent_asks_for_nothing() {
        let mut fit = ControlFit::default();
        fit.record((0, 0), (34, 34));
        assert_eq!(fit.magnification(1.0), None);
        fit.record((10, 10), (20, 10));
        let wanted = fit.magnification(1.0).expect("the second one overflows");
        assert!((wanted - 2.0).abs() < 1e-6, "{wanted}");
    }
}
