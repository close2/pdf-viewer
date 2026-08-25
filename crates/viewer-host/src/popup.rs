//! ISO 32000-2 §12.5.6.14: what a popup window says, and where a host puts it.
//!
//! The clause's first sentence is why a window is a *host's* rather than a page's:
//!
//! > A popup annotation ( PDF 1.3 ) displays text in a popup window for entry and editing. It
//! > shall not appear alone but is associated with a markup annotation, its parent annotation,
//! > and shall be used for editing the parent's text. It shall have no appearance stream or
//! > associated actions of its own
//!
//! No appearance stream, so nothing this program rasterises can draw one; `pdf_model::popup`
//! reads the entries and [`viewer_core::Query::Popups`] places them, and what is left is the
//! window furniture — which is a platform's. `viewer-ui` draws its own, `viewer-gtk` places
//! GTK widgets and `viewer-qt` places Qt ones, and all three of them need the same three
//! strings and the same rectangle.
//!
//! **That is the whole of what is here.** The title bar's label and its timestamp, the body, and
//! the upright box the window occupies — one reading of §12.5.6.2 and Table 166, made once. The
//! *look* is deliberately absent: a title bar's height, its font and its border are the toolkit's
//! and this crate has no widget in it.

use viewer_core::PopupWindow;

/// One of §12.5.6.14's windows, as a host is about to put it on the screen.
///
/// The strings are borrowed from the answer, so building this list costs no allocation at all —
/// which matters because a host asks [`viewer_core::Query::Popups`] on every repaint, exactly as
/// it asks for the selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window<'a> {
    /// The popup annotation, which is what [`viewer_core::Command::Activate`] closes.
    pub annotation: pdf_syntax::ObjectId,
    /// §12.5.6.2's `/T`, which goes at the left of the title bar.
    ///
    /// > The text label that shall be displayed in the title bar of the annotation's popup window
    /// > when open and active. This entry shall identify the user who added the annotation.
    ///
    /// **Empty where the file states none, and that is a choice rather than a fallback.** The
    /// entry is optional and names *a person*; a window headed `Untitled` or `Annotation` would
    /// be this program asserting an author the document does not claim, which principle 5's last
    /// clause is about. An untitled window still has a title bar, because Table 166's `/M` and
    /// the frame belong there whatever `/T` says.
    pub title: &'a str,
    /// Table 166's `/M`, as a person reads it, at the right of the title bar.
    ///
    /// [`crate::stamp`]'s answer, which is §7.9.4's date where the string parses as one and the
    /// file's own characters where it does not — the table makes displaying it in *any* format a
    /// `shall`, so a string this program cannot parse is still shown.
    ///
    /// `None` where the annotation states no `/M`.
    pub modified: Option<&'a str>,
    /// Table 166's `/Contents`: the text in the window.
    ///
    /// Empty where the annotation states none, which is a window with a title bar and nothing in
    /// it. The clause permits it and a good third of the corpus's popups are like that
    /// (`pdf-model`'s `popup` module records the census).
    pub text: &'a str,
    /// The title bar's colour, from Table 166's `/C` — "[t]he title bar of the annotation's popup
    /// window".
    ///
    /// `None` where the file states none *and* for the empty array the table gives the meaning
    /// "0 No colour; transparent", which `pdf_model::popup` has already folded together. A host
    /// puts its own platform colour there.
    pub colour: Option<pdf_render::Color>,
    /// Where the window goes, in device pixels of the viewport: `(left, top, width, height)`.
    ///
    /// [`crate::bounds`] of [`viewer_core::PopupWindow::quad`], because a window is upright in
    /// every toolkit and §7.7.3.3's `/Rotate` can turn the rectangle the file states.
    pub place: (f32, f32, f32, f32),
}

/// Every window in an answer that has somewhere to go, in the order the page listed them.
///
/// **A window with no area is left out**, and it is the one refusal here. Table 166 makes `/Rect`
/// required and §12.5.6.14 gives the popup no appearance stream to fall back on, so a rectangle
/// whose corners coincide describes a window a person could not see and a widget a toolkit would
/// still put in its layout. Leaving it out is not silence: the annotation is still on the page and
/// its parent still opens it, and there is nothing here that a host could draw instead.
///
/// The list is short by nature — a page states as many windows as it has open comments — so this
/// allocates one vector per repaint and borrows every string in it.
#[must_use]
pub fn windows(popups: &[PopupWindow]) -> Vec<Window<'_>> {
    popups
        .iter()
        .filter_map(|popup| {
            let place = crate::bounds(popup.quad);
            (place.2 > 0.0 && place.3 > 0.0).then_some(Window {
                annotation: popup.annotation,
                title: popup.title.as_deref().unwrap_or_default(),
                modified: popup.modified.as_deref(),
                text: popup.text.as_deref().unwrap_or_default(),
                colour: popup.colour,
                place,
            })
        })
        .collect()
}

/// Table 166's `/M` as [`Window::modified`] should be shown, or `None` where there is none.
///
/// Separate from [`windows`] because it allocates and the rest of a window does not: a host asks
/// for the whole list on every repaint and for the timestamp only where it has room to draw one.
#[must_use]
pub fn modified(window: &Window<'_>) -> Option<String> {
    let written = window.modified?.to_owned();
    crate::stamp(pdf_syntax::Date::parse(&written), Some(&written))
}

#[cfg(test)]
mod tests {
    use super::{modified, windows};
    use viewer_core::PopupWindow;

    /// One window whose `/Rect` is an upright box `wide` by `tall` at the origin.
    fn window(wide: f32, tall: f32) -> PopupWindow {
        PopupWindow {
            annotation: pdf_syntax::ObjectId::new(1, 0),
            parent: None,
            quad: [0.0, 0.0, wide, 0.0, wide, tall, 0.0, tall],
            title: Some("A Reader".to_owned()),
            text: Some("a note".to_owned()),
            modified: Some("D:20240102030405Z".to_owned()),
            colour: None,
        }
    }

    #[test]
    fn a_window_with_area_is_placed_where_the_answer_put_it() {
        let answer = [window(120.0, 60.0)];
        let placed = windows(&answer);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].place, (0.0, 0.0, 120.0, 60.0));
        assert_eq!(placed[0].title, "A Reader");
        assert_eq!(placed[0].text, "a note");
    }

    /// Table 166 requires `/Rect`; a rectangle with no area is a window a person cannot see.
    #[test]
    fn a_window_with_no_area_is_not_placed() {
        assert!(windows(&[window(0.0, 60.0)]).is_empty());
        assert!(windows(&[window(120.0, 0.0)]).is_empty());
    }

    /// §12.5.6.2's `/T` is optional and names a person, so nothing is invented for it.
    #[test]
    fn an_untitled_window_is_headed_with_nothing() {
        let bare = PopupWindow {
            title: None,
            text: None,
            ..window(120.0, 60.0)
        };
        let placed = windows(std::slice::from_ref(&bare));
        assert_eq!(placed[0].title, "");
        assert_eq!(placed[0].text, "");
    }

    /// §7.9.4 where it parses, and Table 166's "any format" where it does not.
    #[test]
    fn the_timestamp_is_the_date_where_there_is_one_and_the_string_where_there_is_not() {
        let answer = [window(120.0, 60.0)];
        let placed = windows(&answer);
        assert_eq!(modified(&placed[0]).as_deref(), Some("2024-01-02 03:04"));
        let odd = PopupWindow {
            modified: Some("last Tuesday".to_owned()),
            ..window(120.0, 60.0)
        };
        let placed = windows(std::slice::from_ref(&odd));
        assert_eq!(modified(&placed[0]).as_deref(), Some("last Tuesday"));
        let none = PopupWindow {
            modified: None,
            ..window(120.0, 60.0)
        };
        let placed = windows(std::slice::from_ref(&none));
        assert_eq!(modified(&placed[0]), None);
    }
}
