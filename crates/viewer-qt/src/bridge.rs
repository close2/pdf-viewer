//! The C++ bridge, and the only place in this tree where `unsafe` is not forbidden.
//!
//! # What crosses, and in which direction
//!
//! Everything the Qt side needs to draw is a value: rows, controls, quadrilaterals, a frame's
//! dimensions and a slice of its pixels. Everything a person does arrives as a call on
//! [`Host`]. There is no path from Rust back into C++ except the one function that starts the
//! application, which is what makes the ownership story simple enough to state in a sentence:
//! **C++ owns the [`Host`] for the life of `QApplication::exec`, and Rust never calls a Qt
//! object.**
//!
//! That is the opposite of `viewer-gtk`, where the Rust side owns the widgets and every callback
//! is a `'static` closure holding the host weakly — which is why that host needs an
//! `Rc<RefCell<Host>>` and a re-entrancy guard and this one needs a plain `Box`. ADR 0246
//! compares the two; the finding is that the boundary did not care, because a `Command` is a
//! value in both.
//!
//! # `#![deny(unsafe_code)]`, not `#![forbid(unsafe_code)]`
//!
//! `#[cxx::bridge]` expands to `unsafe extern "C"` declarations, `unsafe` blocks and
//! `#[export_name]` functions — eighteen lints' worth on this module, counted by trying it. A
//! `forbid` cannot be lifted by an inner `allow`, which is the whole point of `forbid` and the
//! reason it is the attribute every crate touching PDF bytes carries. So the crate root denies
//! and `crate::bridge` is the single `#[allow(unsafe_code)]` in the tree outside a test.
//!
//! **One `unsafe` token in this crate is written by hand**, and it is the `unsafe extern "C++"`
//! header below. `cxx` requires it, and what it requires is a *statement*: that the C++
//! declarations inside are the ones C++ actually has and are safe to call with these types. That
//! is discharged in the smallest way available — `cpp/host.h` declares one function, taking the
//! host and an integer and naming no Qt type at all, and `cpp/window.cpp` defines exactly that
//! one. Everything else the lint sees is the macro's expansion, which no source line contains.
//!
//! `tests/unsafe_position.rs` reads this crate's own sources back and asserts the file and line
//! of that single token — because a `deny` with an exemption is a rule that has to be checked
//! rather than trusted. ADR 0246 states the position and argues that `doc/todo/30`'s "`viewer-ffi`
//! is the only crate permitted `unsafe`" was a rule about promises a reviewer has to check, and
//! that one promise in one place, with a test on it, is what keeps it.

use crate::host::Host;

/// The generated bridge.
///
/// Named types are deliberately plain: `cxx` carries `String`, `Vec<T>`, `&str` and `&[u8]`
/// across, and every enumeration is a `u8` with its meaning stated beside the constant on both
/// sides. A shared `enum` would be closer to this project's habits and `cxx` supports one, but a
/// shared enum in `cxx` is `#[non_exhaustive]` in C++ by construction — it compiles a `switch`
/// with no exhaustiveness guarantee — and `viewer-core`'s rule is that a message added later must
/// fail to compile in every consumer. A `u8` with a documented table does not pretend otherwise.
#[cxx::bridge(namespace = "pdf_viewer_qt")]
pub mod ffi {
    /// One row of a platform tree, in depth-first order with its depth beside it.
    ///
    /// **Flattened, because Qt's model wants it flat and GTK's did not.** `GtkTreeListModel`
    /// pulls a row's children through a closure when a person opens it; `QAbstractItemModel`
    /// answers `index`, `parent` and `rowCount` for the whole tree at any moment, so the host
    /// hands over every row at once and the C++ side rebuilds the parentage from the depths.
    /// ADR 0246.
    #[derive(Debug, Clone)]
    struct QtRow {
        /// What the row says.
        label: String,
        /// A second line, empty where the answer carries none.
        detail: String,
        /// How deep it is; 0 is a top-level row.
        depth: u32,
        /// §12.3.3's `/Count` sign: whether the document asked for this row to start open.
        expanded: bool,
        /// What acting on the row does. 0 is inert; 1 activates an outline item (§12.3.3);
        /// 2 moves an optional content group's switch (§8.11.4.3); and 3 takes an embedded
        /// file out (§7.11.4).
        action: u8,
        /// For action 2 only: whether the group is on.
        on: bool,
        /// For action 2 only: Table 99's `/Locked`, which makes the switch unusable.
        locked: bool,
    }

    /// Where the pixels the viewer is holding belong, and how big they are.
    ///
    /// The pixels themselves are not in here: they come back from `frame_pixels` as a borrowed
    /// slice, so tier 1 costs exactly the one copy `doc/ui-boundary.md` prices it at — the
    /// `QImage`'s — and not two.
    #[derive(Debug, Clone, Copy)]
    struct QtFrame {
        /// Whether the viewer has a frame at all.
        present: bool,
        /// The raster's width in device pixels.
        width: u32,
        /// Its height.
        height: u32,
        /// Where its top-left corner sits in the viewport, in device pixels.
        origin_x: f32,
        /// The same, vertically.
        origin_y: f32,
    }

    /// One §12.7 field's widget, as the control a platform builds over it.
    #[derive(Debug, Clone)]
    struct QtControl {
        /// The axis-aligned bound of the widget's quadrilateral, in device pixels of the
        /// viewport. Left.
        x: f32,
        /// Top.
        y: f32,
        /// Width.
        width: f32,
        /// Height.
        height: f32,
        /// Which control: 0 entry, 1 multiline entry, 2 password entry, 3 check box,
        /// 4 radio button, 5 push button, 6 combo box, 7 list box. A field the clause gives no
        /// control to is not in this list at all.
        kind: u8,
        /// §12.7.4.2's fully qualified name, which is what an edit is addressed by.
        field: String,
        /// The widget annotation's object number, which distinguishes one field's several
        /// widgets from each other.
        annotation: u32,
        /// The value the field now holds, for the controls that show one.
        value: String,
        /// Whether `value` is Table 231 bit 14's echo rather than the field's own characters.
        ///
        /// `pdf_model::view::ShownValue::obscured`, carried across so that the C++ can refuse to
        /// write it back. ADR 0201 has a host write the field's value into its control after every
        /// keystroke; doing that for a password field would replace what a person typed with a row
        /// of bullets. The C++ used to leave the password entry out of the write-back switch,
        /// which was this host inferring from a control's kind what the answer now states
        /// (ADR 0247).
        obscured: bool,
        /// Table 227 bit 1: whether the field may be modified.
        read_only: bool,
        /// For a check box or a radio button: whether it is on.
        on: bool,
        /// Table 232's `/MaxLen`, or -1 where the file states none.
        max_len: i32,
        /// Table 233 bit 22: whether more than one item may be selected.
        multi: bool,
        /// Table 233 bit 19: whether a combo box's text may be typed into.
        editable: bool,
        /// Table 234's `/TI` for a list box: which option the list is scrolled to show first.
        ///
        /// Zero for every other control, which is also the table's own default.
        top: u32,
        /// What a person is shown on hovering, from Table 226's `/TU`.
        tooltip: String,
    }

    /// What one control's rectangle asked for and what the style says it cannot go below.
    ///
    /// **The only measurement in this bridge, and it goes the other way from everything else.**
    /// A minimum size is the *toolkit's* answer and no clause's — `QWidget::minimumSizeHint` is a
    /// Qt type's opinion about a Qt style — so it can only be taken on the C++ side; the
    /// arithmetic over it belongs to `viewer_host::ControlFit`, which `viewer-gtk` already feeds
    /// (ADR 0346). Carrying the pairs across is what stops two hosts computing two different
    /// answers from the same numbers.
    ///
    /// Both pairs are **logical** pixels: `QtControl`'s extents are device pixels of the
    /// viewport and a style's minimum is not, so the division by the device pixel ratio happens
    /// once, where the widget is placed.
    #[derive(Debug, Clone, Copy)]
    struct QtMeasure {
        /// The width the widget's `/Rect` asked the control to occupy.
        asked_width: i32,
        /// Its height.
        asked_height: i32,
        /// `QWidget::minimumSizeHint().width()`.
        minimum_width: i32,
        /// Its height.
        minimum_height: i32,
    }

    /// One quadrilateral of interactive chrome, in device pixels of the viewport.
    ///
    /// `[x0, y0, … x3, y3]` as eight named fields rather than an array, because the C++ reads
    /// them by name and an index would be a place to make a mistake silently.
    #[derive(Debug, Clone, Copy)]
    struct QtQuad {
        /// First corner, horizontally.
        x0: f32,
        /// First corner, vertically.
        y0: f32,
        /// Second corner.
        x1: f32,
        /// Second corner.
        y1: f32,
        /// Third corner.
        x2: f32,
        /// Third corner.
        y2: f32,
        /// Fourth corner.
        x3: f32,
        /// Fourth corner.
        y3: f32,
    }

    /// What has changed since the C++ side last asked.
    ///
    /// A window that rebuilt everything after every keystroke would take the keyboard away from
    /// whatever a person is typing into, which is the same reason `viewer-gtk` compares its
    /// control signature between frames. This is that comparison, made once on the Rust side and
    /// reported rather than repeated.
    #[derive(Debug, Clone, Copy)]
    struct QtUpdate {
        /// The page's pixels or their position moved.
        frame: bool,
        /// The three trees must be rebuilt.
        panels: bool,
        /// The set of controls changed, so the widgets must be rebuilt rather than moved.
        controls: bool,
        /// The selection or §12.5.1's focus ring moved.
        chrome: bool,
        /// The title bar's text changed.
        title: bool,
        /// There is a new sentence for the status bar.
        status: bool,
        /// §7.6.4.1: the document asked for a password.
        password: bool,
    }

    extern "Rust" {
        /// One document, one viewer, and everything a window needs to draw it.
        type Host;

        /// The viewport changed size, in device pixels, at `scale` device pixels per logical one.
        fn resized(self: &mut Host, width: u32, height: u32, scale: f32);
        /// A key was pressed, as `Qt::Key`.
        fn key(self: &mut Host, code: u32);
        /// The pointer moved or a button changed: 0 moved, 1 pressed, 2 dragged, 3 released.
        fn pointer(self: &mut Host, x: f32, y: f32, action: u8);
        /// A row of tree `tree` — 0 outline, 1 layers, 2 files — was activated.
        fn activate_row(self: &mut Host, tree: u8, index: usize);
        /// §8.11.4.3's switch on row `index` of tree `tree` was moved.
        fn toggle_row(self: &mut Host, tree: u8, index: usize, on: bool);
        /// A control's value was typed into.
        fn set_control(self: &mut Host, index: usize, value: &str);
        /// §12.7.5.4: which of Table 234's `/Opt` entries are selected now, by index.
        ///
        /// Separate from `set_control` for the reason Table 233 bit 22 gives: "more than one of
        /// the field's option items may be selected simultaneously", so what a `QListWidget` has
        /// to say is a *set* and a string cannot carry one. Indices rather than labels because
        /// two of `/Opt`'s entries may carry the same name string and only the position says
        /// which was clicked (ADR 0248).
        fn choose_control(self: &mut Host, index: usize, chosen: &[u32]);
        /// §12.7.5.2.3's check box or §12.7.5.2.4's radio button was clicked.
        ///
        /// Separate from `set_control` because the *value* is a name the file invented and only
        /// the Rust side has it: §12.7.5.2.3 makes `/V` select among Table 170's appearance
        /// states by name, so a C++ side that sent "on" would be inventing one.
        fn toggle_control(self: &mut Host, index: usize, on: bool);
        /// §12.7.5.2.2's push button was pressed.
        fn activate_control(self: &mut Host, index: usize);
        /// §7.6.4.1: a person typed a password, or dismissed the prompt with `None`.
        fn supply_password(self: &mut Host, password: &str);
        /// A toolbar button: 0 previous page, 1 next page, 2 zoom out, 3 zoom in, 4 fit page.
        fn command(self: &mut Host, what: u8);
        /// Every control the window has just placed, with the minimum its style gives it.
        ///
        /// Sent once per placement rather than once per control: the answer is the *worst* ratio
        /// over the page, so a call per widget would cross the bridge seventy-six times on
        /// `160F-2019.pdf` to compute one number.
        fn measured(self: &mut Host, controls: &[QtMeasure]);

        /// What has changed since this was last called, which also clears it.
        fn take_update(self: &mut Host) -> QtUpdate;
        /// Where the pixels belong and how big they are.
        fn frame(self: &Host) -> QtFrame;
        /// The pixels themselves, borrowed rather than copied.
        fn frame_pixels(self: &Host) -> &[u8];
        /// Tree `tree`'s rows, depth first.
        fn rows(self: &Host, tree: u8) -> Vec<QtRow>;
        /// Every control the page's form wants.
        fn controls(self: &Host) -> Vec<QtControl>;
        /// Table 234's `/Opt` labels for control `index`, in the array's own order.
        fn control_options(self: &Host, index: usize) -> Vec<String>;
        /// Which of them are selected.
        fn control_selection(self: &Host, index: usize) -> Vec<u32>;
        /// What is selected on the page, as quadrilaterals.
        fn selection(self: &Host) -> Vec<QtQuad>;
        /// §12.5.1's focus ring: one quadrilateral, or none.
        fn focus(self: &Host) -> Vec<QtQuad>;
        /// Every occurrence of the find bar's string on the page being shown.
        fn matches(self: &Host) -> Vec<QtQuad>;
        /// Annex O's `highlight`: what the URI's fragment asked to be shown highlighted.
        fn highlights(self: &Host) -> Vec<QtQuad>;
        /// The find bar's string changed. Highlights this page; searches nothing.
        fn retype(self: &mut Host, needle: &str);
        /// The next occurrence anywhere in the document, or the previous one.
        fn find(self: &mut Host, backward: bool);
        /// One more page of the search in progress. Called from a zero-delay timer.
        fn find_continue(self: &mut Host);
        /// The find bar was closed: forget the plan and the highlights.
        fn find_stop(self: &mut Host);
        /// Whether the search still has pages to read, which is what the timer pumps on.
        fn searching(self: &Host) -> bool;
        /// What the title bar should say.
        fn title(self: &Host) -> String;
        /// The most recent sentence for a person.
        fn status(self: &Host) -> String;
        /// The window's initial size in logical pixels: width, then height.
        fn window_size(self: &Host) -> Vec<i32>;

        /// The C++ side finished putting a frame on the screen, with what the copy cost.
        fn painted(self: &mut Host, bytes: usize, nanos: u64);
        /// One sentence from the C++ side, on the topic a host's window belongs to.
        fn note(self: &Host, what: &str);
    }

    unsafe extern "C++" {
        include!("viewer-qt/host.h");

        /// Builds the window, shows it, and runs `QApplication::exec` until it is closed.
        ///
        /// Takes the host, because C++ owns it for the whole life of the event loop. `quit_after`
        /// is a millisecond count after which the application quits by itself and is what makes
        /// an `Xvfb` run terminate; zero means never, which is what a person gets.
        fn run_qt_host(host: Box<Host>, quit_after: i32) -> i32;
    }
}
