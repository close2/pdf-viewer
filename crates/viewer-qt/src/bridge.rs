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
//! and `crate::bridge` is one of the `#[allow(unsafe_code)]` the workspace names — the others are
//! the two C ABIs, `viewer_ffi::abi` and `pdf_vfs_ffi::abi`, and
//! `only_the_three_named_crates_in_the_tree_lift_the_denial` is the list. This line said *the
//! single* one for as long as a second has existed, which is why it names the test rather than a
//! number.
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
        /// Whether the row is a sentence *about* the document rather than a thing *in* it.
        ///
        /// `viewer_host::PanelRow::note` — §14.3.2's heading, or "this document states no article
        /// threads". Qt says it by clearing `Qt::ItemIsEnabled`, which is the platform's own way
        /// of saying a row is not a thing to act on; GTK says it with a `dim-label`.
        note: bool,
        /// Whether the document asked for this row to be presented first.
        ///
        /// `viewer_host::PanelRow::emphasis` — §12.3.5.1's `/D`, "the document that shall be
        /// initially presented in the user interface". The clause states no appearance, so Qt
        /// says it with a bold `Qt::FontRole` and GTK with a `heading` class.
        emphasis: bool,
    }

    /// One row of §12.3.4's panel: a page's label, and its miniature where the page states one.
    ///
    /// **The pixels are owned here and borrowed in `QtFrame`**, and the difference is the size: a
    /// miniature is tens of kilobytes asked for once per row, a frame is megabytes asked for once
    /// per present. Copying the first buys a bridge with one cache in it rather than two.
    ///
    /// `width` and `height` are zero for a page carrying no `/Thumb`, which is most pages of most
    /// documents and is not a defect — §12.3.4's NOTE says thumbnails "are not required, and can
    /// be included for some pages and not for others". The row is drawn either way.
    #[derive(Debug, Clone)]
    struct QtPage {
        /// §12.4.2's label for the page, or the fallback `viewer_host::page_entry` chose.
        label: String,
        /// The miniature's width in samples, or 0 where the page states none.
        width: u32,
        /// Its height.
        height: u32,
        /// Row-major RGBA8 samples, top row first, with no row padding — the same layout a frame's
        /// pixels arrive in, because `pdf_model::thumbnail` decodes a `/Thumb` into the raster
        /// every other image in this tree becomes.
        pixels: Vec<u8>,
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

    /// One of ISO 32000-2 §12.5.6.14's popup windows, as the window furniture Qt places.
    ///
    /// **A rectangle and three strings rather than pixels**, because the clause gives a popup "no
    /// appearance stream or associated actions of its own": there is nothing on the page to copy
    /// and a window belongs to the platform. What decides the three texts and the box is
    /// `viewer_host::popup`, shared with the other two hosts; what is Qt's is the frame, the
    /// fonts and the palette.
    #[derive(Debug, Clone)]
    struct QtPopup {
        /// The window's left edge in device pixels of the viewport.
        x: f32,
        /// Its top edge.
        y: f32,
        /// Its width.
        width: f32,
        /// Its height.
        height: f32,
        /// §12.5.6.2's `/T`, at the left of the title bar. Empty where the file states none.
        title: String,
        /// Table 166's `/M` as `viewer_host::stamp` shows a date, at the right of the title bar.
        /// Empty where the annotation states none.
        modified: String,
        /// Table 166's `/Contents`: the text in the window.
        text: String,
        /// Whether Table 166's `/C` gave the title bar a colour of its own.
        ///
        /// A flag beside the three components rather than an absent value, because `cxx` carries
        /// no `Option` — and the distinction matters: a file stating no colour gets the platform's
        /// own title bar, and one stating black gets black.
        coloured: bool,
        /// `/C`'s red component in 0..=255, where `coloured` is true.
        red: u8,
        /// Its green.
        green: u8,
        /// Its blue.
        blue: u8,
    }

    /// Where a window is on the screen, in the screen's own pixels.
    ///
    /// A shared struct rather than four arguments, and the difference from the C ABI's rule about
    /// structs by value is worth stating: `cxx` regenerates both sides from this file in one
    /// build, so a field added here is a compile error in C++ rather than a silently different
    /// layout — which is precisely what `PDFV_ABI_VERSION` exists to catch where a caller compiles
    /// separately (ADR 0576).
    #[derive(Debug, Clone, Copy)]
    struct QtPlace {
        /// The rectangle's left edge in screen pixels.
        x: f32,
        /// Its top edge.
        y: f32,
        /// Its width.
        width: f32,
        /// Its height.
        height: f32,
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
        /// §12.5.6.14's set of open windows changed, or one of them moved.
        ///
        /// Its own flag rather than `chrome`'s, because these are *widgets* and that one is a
        /// repaint: rebuilding a window on every pointer move during a drag would churn the widget
        /// tree at pointer speed for a list that changes when a page turns.
        popups: bool,
        /// The pointer crossed into or out of §12.5.6.5's activation region.
        ///
        /// A flag beside `over_link` rather than the boolean itself, which is the shape `window`
        /// and `clipboard` already have and is here for the reason this module's own documentation
        /// gives: Rust never calls a Qt object, and `QWidget::setCursor` is one.
        cursor: bool,
        /// The title bar's text changed.
        title: bool,
        /// There is a new sentence for the status bar.
        status: bool,
        /// §7.6.4.1: the document asked for a password.
        password: bool,
        /// Table 29's and §12.2's chrome changed: full screen was entered or left, or a document
        /// stated what its window should hide.
        ///
        /// A flag beside a getter rather than the four booleans themselves, which is the shape
        /// every other answer in this bridge already has: what changed is cheap to say on every
        /// keystroke and what it changed *to* is asked for only when it did (ADR 0470).
        window: bool,
        /// §14.8.2.5's text is waiting to be put on the session's clipboard.
        ///
        /// **The clipboard is Qt's and this bridge does not reverse** (ADR 0519). Everything a
        /// person does arrives as a call on `Host` and Rust never calls a Qt object, which is the
        /// property this module's own documentation states and the reason it needs one
        /// hand-written `unsafe` token rather than a table of them. So a copy is not Rust reaching
        /// for `QClipboard`: it is this flag, and `take_clipboard` beside it, which is exactly the
        /// shape `window` above has and for the same reason.
        clipboard: bool,
        /// The find bar should be shown and given the keyboard.
        ///
        /// `f` and `/` mean this in all three hosts since ADR 0526, and it is a flag for
        /// `clipboard`'s reason: the bar is a `QToolBar` and Rust does not call one.
        find_bar: bool,
        /// The third-party notices should be put on the screen.
        ///
        /// `?`, and the same shape again. The *text* is `viewer_host::NOTICE`, shared with the
        /// other two hosts, because a notice that differs between two binaries of one program is
        /// two claims about one obligation.
        notices: bool,
    }

    /// Which pieces of this window's chrome may be shown — `viewer_host::Chrome`, and the window
    /// itself.
    ///
    /// Table 29's `FullScreen` is "[f]ull-screen mode, with no menu bar, window controls, or any
    /// other window visible"; §12.2's `/HideToolbar`, `/HideMenubar` and `/HideWindowUI` are the
    /// same subject in the smaller. Which sentence is in force is decided on the Rust side, shared
    /// with the other two hosts, and what crosses is the answer rather than the reasoning.
    #[derive(Debug, Clone, Copy)]
    struct QtChrome {
        /// Whether the window should be full screen.
        full_screen: bool,
        /// `/HideMenubar`: whether the menu bar may be shown.
        menu_bar: bool,
        /// `/HideToolbar`: whether the tool bars may be shown.
        tool_bar: bool,
        /// `/HideWindowUI`: whether the status bar and the window's own controls may be shown.
        window_ui: bool,
        /// Table 29's "any other window visible": whether the panel may be shown.
        other_windows: bool,
    }

    extern "Rust" {
        /// One document, one viewer, and everything a window needs to draw it.
        type Host;

        /// The viewport changed size, in device pixels, at `scale` device pixels per logical one.
        fn resized(self: &mut Host, width: u32, height: u32, scale: f32);
        /// A key was pressed, as `Qt::Key`, with Shift's state beside it.
        ///
        /// The modifier crosses because §12.5.1's tab key needs a direction and no other row of
        /// `viewer_host::keys` looks at one. Qt also reports Shift and Tab together as
        /// `Qt::Key_Backtab`, which this side folds back in.
        fn key(self: &mut Host, code: u32, shift: bool);
        /// The pointer moved or a button changed: 0 moved, 1 pressed, 2 dragged, 3 released.
        fn pointer(self: &mut Host, x: f32, y: f32, action: u8);
        /// The wheel turned, in device pixels of the viewport.
        fn scrolled(self: &mut Host, dx: f32, dy: f32);
        /// A row of panel `tree` was activated. The number is a place in `viewer_host::Tab::ALL`.
        fn activate_row(self: &mut Host, tree: u8, index: usize);
        /// §8.11.4.3's switch on row `index` of panel `tree` was moved.
        fn toggle_row(self: &mut Host, tree: u8, index: usize, on: bool);
        /// §12.3.4: a person clicked a page's miniature, so that page is the one to show.
        fn show_page(self: &mut Host, index: usize);
        /// A control's value was typed into.
        fn set_control(self: &mut Host, index: usize, value: &str);
        /// §12.7.5.4: which of Table 234's `/Opt` entries are selected now, by index.
        ///
        /// Separate from `set_control` for the reason Table 233 bit 22 gives: "more than one of
        /// the field's option items may be selected simultaneously", so what a `QListWidget` has
        /// to say is a *set* and a string cannot carry one. Indices rather than labels because
        /// two of `/Opt`'s entries may carry the same label and only the position says
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
        /// §7.6.4.1: a person typed a password, or dismissed the prompt with an empty one.
        ///
        /// **An empty string is a decline and not the default user password**, which the reader
        /// has already tried by the time `QtUpdate::password` was set — so a dialogue closed with
        /// Cancel, with Escape or with the close button sends this with nothing in it, and the
        /// person is told why the document is not on the screen (`viewer_host::password`).
        fn supply_password(self: &mut Host, password: &str);
        /// §7.6.4.1: what to put above the entry, including which attempt this is.
        ///
        /// A function rather than a string built in C++ for `notices`' reason: the words are
        /// `viewer_host::password`'s so that three hosts ask the same question, and only Rust has
        /// them. Asked once, when `QtUpdate::password` says so.
        fn password_prompt(self: &Host) -> String;
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
        /// Which pieces of chrome this window may show, and whether it is full screen.
        fn chrome(self: &Host) -> QtChrome;
        /// The text a copy is putting on the clipboard, which this also clears.
        ///
        /// Asked only where `QtUpdate::clipboard` said so, and in §14.8.2.5's logical content
        /// order where the document's structure tree could give one — which is
        /// `viewer_host::copied`'s decision and the same one the other two hosts take.
        fn take_clipboard(self: &mut Host) -> String;
        /// The third-party notices this binary is obliged to carry, for the window `?` opens.
        ///
        /// A function rather than a constant in the header because the text is
        /// `viewer_host::NOTICE` and only Rust has it; asked once, when the flag says so.
        fn notices(self: &Host) -> String;
        /// Which of `viewer_host::Tab`'s panels the document asks to be showing, as a tab index.
        ///
        /// `-1` for a page mode that names no panel — `UseNone` and `FullScreen`. `UseThumbs` was
        /// among them until §12.3.4's panel was built. Answered
        /// rather than pushed, because the same question is asked twice for two different clauses:
        /// §7.7.2 when the document opens and §12.2's `/NonFullScreenPageMode` when full screen
        /// ends.
        fn panel_wanted(self: &Host) -> i32;
        /// How many pages Table 29's arrangement is showing pixels for.
        fn frame_count(self: &Host) -> usize;
        /// Where one page's pixels belong and how big they are.
        fn frame(self: &Host, index: usize) -> QtFrame;
        /// The pixels themselves, borrowed rather than copied.
        fn frame_pixels(self: &Host, index: usize) -> &[u8];
        /// Panel `tree`'s rows, depth first. §12.3.4's panel has none — see `page_row`.
        fn rows(self: &Host, tree: u8) -> Vec<QtRow>;
        /// What panel `tab` is called, from the one place the six words are written down.
        fn panel_label(self: &Host, tab: u8) -> String;
        /// Which of the panels is §12.3.4's — the one that is a list of pictures rather than rows.
        fn pages_panel(self: &Host) -> u8;
        /// How many rows §12.3.4's panel has, which is the document's page count.
        fn page_count(self: &Host) -> usize;
        /// One row of §12.3.4's panel, asked for when the model is about to draw that row.
        ///
        /// **Not in a loop over `page_count`.** `CLAUDE.md` section 2 forbids thumbnail generation on the
        /// launch path by name and `viewer_core::Query::Thumbnail` answers one page at a time so
        /// that a host can obey it; a `QAbstractListModel` asks `data` for the rows a view is
        /// laying out and no others, which is what makes that obedience the model's own shape.
        fn page_row(self: &Host, index: usize) -> QtPage;
        /// How many of §12.3.4's miniatures the panel may keep — `viewer_host::KEPT_MINIATURES`.
        fn kept_miniatures(self: &Host) -> usize;
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
        /// §12.5.6.14's open popup windows, placed. Asked when `QtUpdate::popups` says so.
        fn popups(self: &Host) -> Vec<QtPopup>;
        /// Whether the pointer is over §12.5.6.5's activation region.
        ///
        /// Asked when `QtUpdate::cursor` says so, which is when the answer *changed*: what a
        /// reader sees over a link is a convention no clause states, and this is the one this
        /// program has had in `viewer-ui` since ADR 0166.
        fn over_link(self: &Host) -> bool;
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
        /// The window moved or was resized, in the screen's own pixels: its frame, then its
        /// contents.
        ///
        /// **AT-SPI reports a node's extents in screen coordinates and this program's are the
        /// viewport's**, so an adapter needs the window's own origin to add. Qt has it —
        /// `QWidget::frameGeometry` and `QWidget::geometry` are both in screen coordinates — and
        /// GTK4 has no such API at all, which is the one place the two native hosts genuinely
        /// differ on §14.7 (ADR 0623). It crosses as eight numbers on a `moveEvent` rather than
        /// being asked for per page: `viewer-ui` measured the same two questions at 1.8 to 3.2 ms
        /// of synchronous X11 round trips when it asked them on every page turn (ADR 0228).
        fn window_placed(self: &mut Host, outer: QtPlace, inner: QtPlace);
        /// How long to wait before draining what an assistive technology has asked for, in
        /// milliseconds, or `-1` while nothing is listening.
        ///
        /// **`presentation_wait`'s shape, and for a related reason** (ADR 0473): the interval is
        /// pulled rather than pushed, so C++ owns the timer and Rust owns the decision. What is
        /// different is what the answer depends on — a clock's interval depends on §12.4.4.1 and
        /// this one depends on whether anybody has attached to the accessibility bus at all.
        /// `accesskit_unix` calls this program back from its own D-Bus thread and there is no
        /// path from a foreign thread into `QApplication::exec` that would not cost this crate a
        /// second hand-written `unsafe` token, so a request is drained on a timer that does not
        /// exist until a client arrives.
        fn accessibility_wait(self: &Host) -> i32;
        /// One drain of that queue. Called from a `QTimer`.
        fn accessibility_pump(self: &mut Host);
        /// How long to wait before asking the drawing thread whether a page has finished, in
        /// milliseconds, or `-1` while nothing is being drawn.
        ///
        /// **`presentation_wait`'s shape a third time, and here it is the whole reason the
        /// arrangement is a pull** (ADR 0668). A page is rasterised on a thread of its own since
        /// the seven-hundred-and-fifty-fourth session, so that a document written to be expensive
        /// cannot take `QApplication::exec` with it — and a finished page has to reach this window
        /// somehow. There is no path from that thread into a Qt object that would not cost this
        /// crate a second hand-written `unsafe` token, which is the property this module's own
        /// documentation states. So the interval is `viewer_host::Drawing`'s decision, shared with
        /// `viewer-gtk`, and the timer is Qt's — and it does not exist at all while the thread is
        /// idle.
        fn drawing_wait(self: &Host) -> i32;
        /// One look at that thread. Called from a `QTimer`.
        fn drawing_pump(self: &mut Host);
        /// ISO 32000-2 §12.4.4.1: how long to wait before the next turn of the presentation
        /// clock, in milliseconds, or `-1` where nothing is presenting.
        ///
        /// Asked after every pump rather than set once, because the interval a transition wants
        /// and the interval a still page wants are different numbers — `viewer_host::Clock`
        /// decides both, so all three hosts wake at the same rate for the same reason.
        fn presentation_wait(self: &Host) -> i32;
        /// One turn of that clock. Called from a `QTimer`.
        fn presentation_tick(self: &mut Host);
        /// What the title bar should say.
        fn title(self: &Host) -> String;
        /// The most recent sentence for a person.
        fn status(self: &Host) -> String;
        /// The window's initial size in logical pixels: width, then height.
        fn window_size(self: &Host) -> Vec<i32>;
        /// What the page area shows where no page lies: red, green, blue, in 0..=255.
        ///
        /// **Not §11.4.7's 𝑊**, which is the page's own colour and is imposed by the rasteriser
        /// inside the page's boundary. This is the ground the pages are laid on, which the
        /// standard says nothing about at all — `pdf_render::medium` has both readings and owns
        /// the value, so that the three hosts and the three rasterisers state one fact once.
        fn surround(self: &Host) -> Vec<u8>;

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
