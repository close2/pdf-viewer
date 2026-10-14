//! The window: its widgets built once, the title, the status sentences, Table 29's catalog entries, and the chrome layer drawn over the page.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::Path;
use std::rc::{Rc, Weak};

use gtk4::glib;
use gtk4::prelude::*;
use viewer_core::{Answer, Command, PageTarget, Query, Zoom};

use viewer_host::native_window::NativeWindow;
use viewer_host::panel::Tab;

use super::find::find_bar;
use super::input::listen;
use super::panels::notebook_page;
use super::restrictions::menu_button;
use super::{Chrome, Host, PANEL_WIDTH, Ui, with};

impl Host {
    /// The shapes the chrome layer draws, gathered from the four questions that answer them.
    ///
    /// Its own method rather than the tail of [`Host::refresh`] because all four are asked on
    /// every repaint and none of them touches a widget: what comes back is written into the one
    /// piece of state the draw function reads, and the layer is asked to redraw itself.
    pub(super) fn refresh_chrome(&mut self) {
        let selection = match self.state.viewer.query(Query::Selection) {
            Answer::Selected(selected) => selected.quads.clone(),
            _ => Vec::new(),
        };
        let focus = match self.state.viewer.query(Query::Focus) {
            Answer::Focus { quad, .. } => Some(quad),
            _ => None,
        };
        // Every occurrence on the page being shown. Asked on every repaint because it is a query
        // over a readback that already exists — `Query::Find` is this page and `Command::Find` is
        // the document, and only the second one costs anything.
        let matches = if self.state.needle.is_empty() {
            Vec::new()
        } else {
            match self.state.viewer.query(Query::Find(&self.state.needle)) {
                Answer::Found(occurrences) => occurrences.into_iter().flatten().collect(),
                _ => Vec::new(),
            }
        };
        // Annex O's highlighted rectangle, if the URI this document was opened from named one.
        // Answered from a list the fragment filled when the document opened, so it costs a filter
        // over that list and nothing else.
        let highlights = match self.state.viewer.query(Query::Highlight) {
            Answer::Highlighted(quads) => quads,
            _ => Vec::new(),
        };
        {
            let mut chrome = self.chrome.borrow_mut();
            chrome.selection = selection;
            chrome.matches = matches;
            chrome.highlights = highlights;
            chrome.focus = focus;
            chrome.measuring = self.state.showing.measuring.points().to_vec();
            chrome.scale = f64::from(self.scale);
        }
        self.ui.chrome.queue_draw();
    }

    /// One sentence for a person, in the status bar and on standard output.
    ///
    /// Both, because the status bar is what a person sees and the terminal is what a test reads —
    /// and the `Xvfb` runs this project checks a window with read the terminal.
    pub(super) fn say(&self, what: &str) {
        if what.is_empty() {
            return;
        }
        println!("note: {what}");
        self.ui.status.set_text(what);
        // The label ellipsizes at the end, so a long sentence loses its tail — and the longest
        // sentence this window says is the one that names the way out of a refusal, whose tail is
        // the way out. Found by photographing the window this round's own fix was made for, which
        // is trap 1 one layer out from a page. A tooltip is GTK's own idiom for an elided label
        // and costs nothing when the text fits.
        self.ui.status.set_tooltip_text(Some(what));
    }

    /// Puts the caption in the title bar.
    pub(super) fn retitle(&self) {
        let mark = if self.state.showing.dirty { "• " } else { "" };
        self.ui.window.set_title(Some(&format!(
            "{mark}{} — {}",
            self.named(),
            self.state.showing.caption
        )));
    }

    /// Table 29's two display entries, and §12.2's chrome flags with them.
    ///
    /// `/PageLayout` is the *viewer's* to apply — it read the catalog when the document opened —
    /// and what this host needs from it is the value `l` cycles from, so that the first press
    /// moves off what the document asked for rather than back onto it. `/PageMode` is "how the
    /// document shall be displayed when opened", which is a panel for three of its six names,
    /// nothing for `UseNone`, and since ADR 0470 a full-screen window for `FullScreen`.
    ///
    /// `in_front` is false for a document opened behind the one showing. Its layout is its own tab's
    /// and is taken, but its `/PageMode` is "how the document shall be displayed when opened" and it
    /// is not displayed: the window's full screen, presentation and panel stay the front
    /// document's, so a presentation running when a second document arrives keeps its full screen
    /// (ADR 1303).
    pub(super) fn obey_the_catalog(&mut self, queue: &mut VecDeque<Command>, in_front: bool) {
        let Answer::Opening(opening) = self.state.viewer.query(Query::Opening) else {
            return;
        };
        self.state.showing.layout = opening.layout;
        if opening.layout != pdf_model::viewer_preferences::PageLayout::SinglePage {
            self.say(&format!(
                "this document opens in the {:?} page layout (§7.7.2)",
                opening.layout
            ));
        }
        if !in_front {
            return;
        }
        if let Answer::Preferences(preferences) = self.state.viewer.query(Query::Preferences) {
            self.presenting = viewer_host::Presenting::opening(opening, &preferences);
            // Trap 5: a document asking for something and getting silence. Two of Table 147's
            // three flags name widgets this window hides on request, and the third names one it
            // keeps on purpose — the menu holding this reader's own restriction levels
            // (`viewer_host::restriction::NOT_THE_DOCUMENTS_TO_HIDE`).
            if preferences.hide_menubar {
                self.say(viewer_host::restriction::NOT_THE_DOCUMENTS_TO_HIDE);
            }
            // Table 147's `/CenterWindow` has no call to reach in GTK 4, which gives a program no
            // way to place its own top-level window; said rather than dropped (trap 5, ADR 1429).
            if preferences.center_window {
                self.say(NO_POSITION_TO_SET);
            }
            // §12.2's `/DisplayDocTitle`: the title bar shows the document's own title, by the rule
            // the other two windows take too. The metadata stream is read only for a document
            // that asks, so a document that does not costs its opening nothing here.
            self.state.showing.titled = if preferences.display_doc_title {
                match self.state.viewer.query(Query::Properties) {
                    Answer::Properties {
                        information,
                        metadata,
                    } => viewer_host::documents::document_title(&information, metadata.as_ref()),
                    _ => None,
                }
            } else {
                None
            };
            self.retitle();
        }
        // Queued rather than dispatched: this runs inside `react`, and `dispatch` would start a
        // second `pump` under the first. `queue` is what that parameter is for.
        if self.presenting.full_screen() {
            self.start_the_clock();
            queue.push_back(Command::Present(self.presenting.mode()));
        }
        self.show_page_mode(opening.mode);
        self.apply_chrome();
    }

    /// Shows the document the way one of Table 29's six page modes asks for.
    ///
    /// Two callers and one mapping, which is the point: the document opening, where §7.7.2 states
    /// "how the document shall be displayed when opened", and full screen ending, where §12.2's
    /// `/NonFullScreenPageMode` states "how to display the document on exiting full-screen mode".
    ///
    /// Which panel each of Table 29's names opens is [`Tab::of_page_mode`]'s, shared with the
    /// other two hosts, and **`UseThumbs` is no longer among the names that reach none** — this
    /// host reported it as a mode it had no panel for until §12.3.4's list was built.
    /// `FullScreen` cannot arrive from the second caller, because `pdf_model` refuses that name for
    /// `/NonFullScreenPageMode`.
    pub(super) fn show_page_mode(&mut self, mode: pdf_model::viewer_preferences::PageMode) {
        use pdf_model::viewer_preferences::PageMode;
        if let Some(tab) = Tab::of_page_mode(mode) {
            self.ui.tabs.set_current_page(Some(notebook_page(tab)));
            return;
        }
        if matches!(mode, PageMode::FullScreen) {
            self.say(
                "presenting full screen (§7.7.2's FullScreen page mode) — Escape comes \
                      back",
            );
        }
    }

    /// Puts the window in the state [`viewer_host::Presenting`] says the document asked for.
    ///
    /// Four sentences, four widgets. §12.2's `/HideToolbar` is the header bar's own buttons and
    /// the find bar; `/HideWindowUI` — "scroll bars and navigation controls" — is the status line;
    /// Table 29's "any other window visible" is the notebook of three trees; and full screen is
    /// the window itself, with the titlebar gone because that sentence names the window controls
    /// too. **`/HideMenubar` names the restrictions menu and is deliberately not obeyed** — a
    /// document that could hide the reader's own levels would be taking away the control over
    /// itself — and it is reported rather than ignored when a document asks for it
    /// (`viewer_host::restriction::NOT_THE_DOCUMENTS_TO_HIDE`).
    pub(super) fn apply_chrome(&mut self) {
        let chrome = self.presenting.chrome();
        for button in &self.ui.tool_buttons {
            button.set_visible(chrome.tool_bar);
        }
        if !chrome.tool_bar {
            self.ui.find.set_search_mode(false);
        }
        self.ui.find.set_visible(chrome.tool_bar);
        self.ui.status_bar.set_visible(chrome.window_ui);
        // **Taken out of the splitter rather than hidden**, because a `GtkPaned` carries a
        // `position` that was *set* and a hidden start child leaves that property standing —
        // which is a three-hundred-pixel hole where the panel was, and Table 29 says "no … other
        // window visible" rather than "no other window drawn". A removed child is unambiguous:
        // the end child gets the whole allocation. `Ui::tabs` holds the reference across the
        // unparenting, so the three trees inside it are still there when it goes back.
        //
        // **Two conditions rather than one since ADR 0526**: the clause's permission, and the
        // reader's own wish. `o` moves the second, and composing them here is what makes leaving
        // full screen put back what the *reader* had.
        if chrome.other_windows && self.panel_wanted {
            if self.ui.split.start_child().is_none() {
                self.ui.split.set_start_child(Some(&self.ui.tabs));
                self.ui.split.set_position(PANEL_WIDTH);
            }
        } else if self.ui.split.start_child().is_some() {
            self.ui.split.set_start_child(None::<&gtk4::Widget>);
        }
        self.show_the_tabs();
        // **A layout asked for from outside the allocation**, because the first caller is inside
        // one: page one opens from `resized`, which is `GtkDrawingArea::resize`, and a widget
        // hidden while GTK is allocating is not laid out again until something else asks for a
        // frame. Under Table 29's `FullScreen` that left the page area its pre-full-screen size
        // until the first key press, so the first transition's faces were rasterised at that size
        // and the page jumped as the resize finally landed (ADR 1316).
        let window = self.ui.window.clone();
        glib::idle_add_local_once(move || window.queue_resize());
        if self.presenting.full_screen() {
            self.ui.window.set_decorated(false);
            self.ui.window.fullscreen();
        } else {
            self.ui.window.unfullscreen();
            self.ui.window.set_decorated(true);
        }
    }

    /// The third-party notices this binary is obliged to carry, in a window of their own.
    ///
    /// **A licence obligation with a surface**: `pdf-font` compiles the standard 14 font programs
    /// into every binary in this tree, and both of their licences require a binary distribution to
    /// reproduce their notices. The text is [`viewer_host::NOTICE`], shared with the other two
    /// hosts because a notice that differs between two binaries of one program is two claims about
    /// one obligation.
    ///
    /// Set in a monospace font and **not re-wrapped**: a BSD licence's paragraphs and a font
    /// list's columns are laid out by the file's own line breaks, and re-flowing text this program
    /// is obliged to reproduce would be editing it.
    pub(super) fn show_notices(&self) {
        let text = gtk4::TextView::new();
        text.set_editable(false);
        text.set_cursor_visible(false);
        text.set_monospace(true);
        text.set_left_margin(12);
        text.set_right_margin(12);
        text.set_top_margin(12);
        text.set_bottom_margin(12);
        text.buffer().set_text(viewer_host::NOTICE);
        let scroller = gtk4::ScrolledWindow::new();
        scroller.set_child(Some(&text));
        let window = gtk4::Window::builder()
            .title("Third-party notices")
            .transient_for(&self.ui.window)
            .modal(true)
            .default_width(760)
            .default_height(620)
            .child(&scroller)
            .build();
        window.present();
    }

    /// Where the page area's top-left corner sits in the window, in device pixels.
    ///
    /// The one place AT-SPI and this window agree on: GTK 4 reports its own widgets in the
    /// window's coordinates and in no others, and the document's nodes are the viewport's until the
    /// bridge is told this (ADR 1516). `(0, 0)` for a page area that is not in a window yet.
    pub(crate) fn viewport_origin(&self) -> (f32, f32) {
        let page = &self.ui.fixed;
        let Some((x, y)) = page
            .root()
            .and_then(|root| page.translate_coordinates(&root, 0.0, 0.0))
        else {
            return (0.0, 0.0);
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a widget's place in its window, in device pixels; f32 is exact to 2^24 and \
                      no window is that large"
        )]
        (x as f32, y as f32)
    }
}

/// What to call the document in a title bar: its file name, or the whole path where it has none.
/// What this window says to a document asking for Table 147's `/CenterWindow`.
///
/// GTK 4 removed the call that placed a top-level window, and on Wayland no client places its own
/// window at all; the position is the platform's. A departure the toolkit made rather than one this
/// program decided, and said once when the document opens rather than dropped (ADR 1429).
pub(super) const NO_POSITION_TO_SET: &str = "this document asks for its window in the centre of the screen \
                                  (§12.2's /CenterWindow); GTK 4 gives a program no way to place \
                                  its own window, so it stays where the platform put it";

pub(super) fn named(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// Device pixels as the logical ones GTK lays out in.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a widget extent inside a window is far inside i32's range, and this is the size \
              request GTK itself takes as an i32"
)]
pub(super) fn logical(device: f64, scale: f64) -> i32 {
    (device / scale).round().max(1.0) as i32
}

/// Paints [`pdf_render::SURROUND`] behind the pages, so that a reader can see a page's edge.
///
/// **This is not §11.4.7's 𝑊 and must never be confused with it.** The page's own colour is the
/// standard's, is white, and is imposed by `render-cpu` inside the page's boundary; what lies
/// *outside* every page is not any clause's subject and is this program's documented choice —
/// `pdf_render::medium` has both readings. The colour is taken from there rather than restated
/// here so that all three hosts and all three rasterisers say one thing.
///
/// **Why a native host does not simply take the toolkit's window background.** It sounds like the
/// native answer and it is not one: GTK's default background under Adwaita is within a few levels
/// of white, so a continuous column drew white paper on almost-white ground and the gap between two
/// pages was as good as invisible — measured on the screen, not assumed. The toolkit has no notion
/// of "the surface a document is laid on", so there is no platform value to inherit; picking one is
/// the application's job either way, and picking the same one in all three hosts is `doc/todo/30`'s
/// standing decision that the hosts stay level.
///
/// A failure to load the rule is reported and nothing else: a window whose ground is the
/// toolkit's default still shows every page, so refusing to open would be a worse answer than
/// the faint boundary this exists to sharpen.
fn surround(widget: &impl IsA<gtk4::Widget>) {
    /// The class the rule below names, and the only widget that carries it.
    const CLASS: &str = "pdf-page-surround";
    let level = |component: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a colour component in 0..=1 scaled by 255"
        )]
        {
            (component.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
        }
    };
    let colour = pdf_render::SURROUND;
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(&format!(
        ".{CLASS} {{ background-color: rgb({}, {}, {}); }}",
        level(colour.r),
        level(colour.g),
        level(colour.b)
    ));
    let Some(display) = gtk4::gdk::Display::default() else {
        eprintln!("note: no display to style the page's surround on; it stays the toolkit's");
        return;
    };
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    widget.as_ref().add_css_class(CLASS);
}

/// Everything the window is made of, and the callbacks that reach the host from it.
#[expect(
    clippy::too_many_lines,
    reason = "one window, built once: the widgets, the strip of open documents, the six panels \
              and the signals that reach the host. Splitting it would put half a window's \
              construction in another function and lose the one property it has — that a reader \
              can see the whole widget tree in one place"
)]
pub(super) fn build_window(
    app: &gtk4::Application,
    path: &Path,
    me: &Weak<RefCell<Host>>,
    chrome_state: &Rc<RefCell<Chrome>>,
) -> Ui {
    let window = gtk4::ApplicationWindow::new(app);
    window.set_default_size(1000, 1100);
    window.set_title(Some(&named(path)));
    // Whether a screen reader is told this is the window a person is in (ADR 1565). On the idle
    // queue for `connect_search_mode_enabled_notify`'s reason below: GTK raises the notification
    // from inside calls that can be holding the host borrowed.
    let listener = me.clone();
    window.connect_is_active_notify(move |window| {
        let active = window.is_active();
        let listener = listener.clone();
        glib::idle_add_local_once(move || with(&listener, |host| host.window_focused(active)));
    });

    let (fixed, popups, chrome, overlay) = page_area(chrome_state);

    // One notebook page per `viewer_host::Tab`, in that list's own order and with that list's own
    // wording — six panels rather than the three this host drew until `doc/todo/30`'s item 4, and
    // a seventh added there appears here without a line changing.
    let tabs = gtk4::Notebook::new();
    // **Down the side rather than across the top**, and the screen is what decided it: six labels
    // do not fit across a sidebar, and a `GtkNotebook` that cannot fit its tabs puts the rest
    // behind scroll arrows — so four of `viewer_host::Tab`'s six panels were reachable only by
    // pressing an arrow nobody would look for. A vertical strip holds all six at any width this
    // panel would sensibly have. `set_scrollable` stays on regardless, because a theme with a
    // taller font is nobody's to predict.
    tabs.set_tab_pos(gtk4::PositionType::Left);
    tabs.set_scrollable(true);
    let slots: Vec<gtk4::Box> = Tab::ALL
        .iter()
        .map(|tab| {
            let slot = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
            tabs.append_page(&slot, Some(&gtk4::Label::new(Some(tab.label()))));
            slot
        })
        .collect();
    tabs.set_size_request(PANEL_WIDTH - 20, -1);

    let split = gtk4::Paned::new(gtk4::Orientation::Horizontal);
    split.set_start_child(Some(&tabs));
    split.set_end_child(Some(&overlay.clone()));
    split.set_position(PANEL_WIDTH);
    split.set_resize_start_child(false);

    let status = gtk4::Label::new(None);
    status.set_xalign(0.0);
    status.set_ellipsize(pango::EllipsizeMode::End);
    status.set_margin_start(6);
    status.set_margin_end(6);
    status.set_margin_top(3);
    status.set_margin_bottom(3);

    let (find, find_entry) = find_bar();

    // The separator and the label in one box, because §12.2's `/HideWindowUI` names "scroll bars
    // and navigation controls" as one thing to hide and a rule that left a hairline behind would
    // be obeying half of it.
    let status_bar = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    status_bar.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
    status_bar.append(&status);
    let loading = gtk4::ProgressBar::new();
    loading.set_margin_start(6);
    loading.set_margin_end(6);
    loading.set_margin_bottom(3);
    loading.set_visible(false);
    status_bar.append(&loading);

    let documents = gtk4::Notebook::new();
    documents.set_scrollable(true);
    documents.set_show_border(false);
    // One document, so no strip: a window that opened one file is the window it was.
    documents.set_show_tabs(false);
    let first = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    first.append(&split);
    documents.append_page(&first, Some(&gtk4::Label::new(Some(&named(path)))));

    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    column.append(&find);
    column.append(&documents);
    column.append(&status_bar);
    documents.set_vexpand(true);
    split.set_vexpand(true);
    window.set_child(Some(&column));
    let (bar, tool_buttons) = header(me);
    let menu = menu_button(me);
    bar.pack_end(&menu);
    window.set_titlebar(Some(&bar));

    // Three signals, and each is a different question a find bar answers. Typing changes what is
    // highlighted *on this page*, which is a query and costs nothing; activating asks the core for
    // the next occurrence anywhere; and `GtkSearchEntry`'s own next/previous — Ctrl+G and
    // Ctrl+Shift+G — are the same command in each direction.
    let listener = me.clone();
    find_entry.connect_search_changed(move |entry| {
        let typed = entry.text().to_string();
        with(&listener, |host| host.retype(typed));
    });
    let listener = me.clone();
    find_entry.connect_activate(move |_| {
        with(&listener, |host| host.find(false));
    });
    // Shift with Enter is the previous occurrence, as it is in the other two windows' find bars
    // (ADR 1453). Taken in the capture phase because `GtkSearchEntry` activates on either.
    let shifted = gtk4::EventControllerKey::new();
    shifted.set_propagation_phase(gtk4::PropagationPhase::Capture);
    let listener = me.clone();
    shifted.connect_key_pressed(move |_, key, _, held| {
        let enter = matches!(key, gtk4::gdk::Key::Return | gtk4::gdk::Key::KP_Enter);
        if !enter || !held.contains(gtk4::gdk::ModifierType::SHIFT_MASK) {
            return glib::Propagation::Proceed;
        }
        with(&listener, |host| host.find(true));
        glib::Propagation::Stop
    });
    find_entry.add_controller(shifted);
    let listener = me.clone();
    find_entry.connect_next_match(move |_| {
        with(&listener, |host| host.find(false));
    });
    let listener = me.clone();
    find_entry.connect_previous_match(move |_| {
        with(&listener, |host| host.find(true));
    });
    let listener = me.clone();
    find.connect_search_mode_enabled_notify(move |bar| {
        // **On the idle queue, not here.** This signal fires *synchronously* from
        // `set_search_mode`, which `Host::key` calls while it is holding the host's `RefCell` —
        // so calling in would find it borrowed and `with` would print "the host was busy and an
        // action was dropped", which is what the first run under `Xvfb` printed. Deferring lets
        // the key handler unwind first, and is the same move `viewer-qt` makes with
        // `QTimer::singleShot` for `QDialog::exec`.
        let shown = bar.is_search_mode();
        let listener = listener.clone();
        glib::idle_add_local_once(move || {
            with(&listener, |host| host.find_bar_shown(shown));
        });
    });

    // A person's click on a tab, told apart from this host's own `set_current_page` by the flag
    // the host sets around it — and deferred to the idle queue for the find bar's reason, because
    // the signal is emitted synchronously from inside a call this host makes while borrowed.
    let listener = me.clone();
    documents.connect_switch_page(move |_, _, index| {
        let listener = listener.clone();
        glib::idle_add_local_once(move || {
            with(&listener, |host| host.tab_chosen(index as usize));
        });
    });

    listen(&window, &overlay, &chrome, me);

    window.present();
    // The page has the keyboard as the window opens, for `Host::page_takes_the_keyboard`'s reason:
    // otherwise GTK gives it to the first panel row, which takes Home and End (ADR 1453).
    GtkWindowExt::set_focus(&window, Some(&overlay));

    Ui {
        window,
        fixed,
        popups,
        pictures: Vec::new(),
        chrome,
        slots,
        tabs,
        documents,
        split,
        page_area: overlay,
        tool_buttons,
        menu,
        status_bar,
        status,
        loading,
        find,
        find_entry,
    }
}

/// The four widgets the page is drawn in, and the order they are stacked in.
///
/// Its own function because it is one decision — *what is over what* — and stating it beside the
/// notebook, the header bar and the status bar buried it. Bottom to top: the page's own pictures
/// and §12.7's controls, §12.5.6.14's windows, and the layer the selection and §12.5.1's ring are
/// drawn on.
fn page_area(
    chrome_state: &Rc<RefCell<Chrome>>,
) -> (gtk4::Fixed, gtk4::Fixed, gtk4::DrawingArea, gtk4::Overlay) {
    let fixed = gtk4::Fixed::new();
    fixed.set_overflow(gtk4::Overflow::Hidden);
    surround(&fixed);

    let chrome = gtk4::DrawingArea::new();
    // The chrome layer is over the page and over the controls, so it must not take a click that
    // belongs to either: this is what makes a `GtkEntry` under it still receive the keyboard.
    chrome.set_can_target(false);
    chrome.set_hexpand(true);
    chrome.set_vexpand(true);
    let state = Rc::clone(chrome_state);
    chrome.set_draw_func(move |area, cr, _, _| {
        if let Err(error) = draw_chrome(area, cr, &state.borrow()) {
            eprintln!("note: cannot draw the chrome: {error}");
        }
    });

    // §12.5.6.14's windows go in a layer of their own, and **not** in the `GtkFixed` the page and
    // the form controls are in. A `GtkFixed` measures the union of its children, so a popup whose
    // `/Rect` the document put *beside* the page — which is where every one of `issue14438.pdf`'s
    // six is — asks the `GtkPaned` for more room, which widens the viewport, which moves the
    // window further out, which asks for more room again: measured, the page area walked from 509
    // to 1229 device pixels in nine frames before the geometric series ran out. An overlay child
    // is not measured by `GtkOverlay` unless `set_measure_overlay` says so, and it is allocated
    // the overlay's own size, so a window outside the viewport is clipped instead of moving it.
    let popups = gtk4::Fixed::new();
    popups.set_can_target(false);
    popups.set_overflow(gtk4::Overflow::Hidden);

    // **The page's `GtkFixed` is an overlay child as well, over a ground that is measured in its
    // place**, for the popups' reason and for Table 147's `/FitWindow`: a `GtkFixed` measures the
    // union of its children, so a page centred in a tall viewport makes the viewport's *natural*
    // height its own offset plus its height, and GTK 4 will not size a window below its natural
    // size — a window asked to fit a page 812 pixels tall stopped at 915, and every retry halved
    // the gap rather than closing it. The ground expands and states no size of its own, so the
    // window's size is the window's (ADR 1429). The fixed is still allocated the whole overlay, so
    // its surround and its children are where they were.
    let ground = gtk4::DrawingArea::new();
    ground.set_can_target(false);
    ground.set_hexpand(true);
    ground.set_vexpand(true);
    let overlay = gtk4::Overlay::new();
    overlay.set_child(Some(&ground));
    overlay.add_overlay(&fixed);
    // Under the chrome layer: a selection, a match and §12.5.1's ring are marks *on the page*, and
    // a window that hid them would be furniture eating the document. The window is opaque either
    // way, so what this decides is only what happens where the two meet.
    overlay.add_overlay(&popups);
    overlay.add_overlay(&chrome);
    overlay.set_overflow(gtk4::Overflow::Hidden);
    overlay.set_hexpand(true);
    overlay.set_vexpand(true);
    // Focusable so that a press on the page can give it the keyboard: the key table listens on
    // the window, and only a focused widget that does not bind the key lets it reach there
    // (`Host::page_takes_the_keyboard`, ADR 1453).
    overlay.set_focusable(true);

    (fixed, popups, chrome, overlay)
}

/// The title bar's own buttons, and the buttons apart from it.
///
/// The pair rather than the bar alone, because §12.2's `/HideToolbar` and Table 29's "window
/// controls" name different things and GTK4 puts both in one widget: the buttons are this host's
/// tool bar and the bar itself carries the close button, so a document asking for no tool bar
/// loses the four buttons and keeps its way out of the window.
fn header(me: &Weak<RefCell<Host>>) -> (gtk4::HeaderBar, Vec<gtk4::Button>) {
    let bar = gtk4::HeaderBar::new();
    let mut buttons = Vec::new();
    for (label, target) in [("‹", PageTarget::Previous), ("›", PageTarget::Next)] {
        let button = gtk4::Button::with_label(label);
        let listener = me.clone();
        button.connect_clicked(move |_| {
            with(&listener, |host| host.dispatch(Command::GoTo(target)));
        });
        bar.pack_start(&button);
        buttons.push(button);
    }
    for (label, zoom) in [("−", Zoom::Out), ("+", Zoom::In)] {
        let button = gtk4::Button::with_label(label);
        let listener = me.clone();
        button.connect_clicked(move |_| {
            with(&listener, |host| {
                host.dispatch(Command::Zoom { zoom, at: None });
            });
        });
        bar.pack_end(&button);
        buttons.push(button);
    }
    (bar, buttons)
}

/// The interactive chrome, in the platform's own colour.
///
/// `doc/ui-boundary.md`: "[e]mitting them as quads and points lets a native host draw selection in
/// **macOS's selection colour, KDE's accent, the Windows highlight brush**". The colour here is
/// [`gtk4::prelude::WidgetExt::color`] — the theme's own foreground at this widget, which follows
/// a light or dark theme without this program knowing which is on. It is deliberately *not* the
/// desktop's accent colour, because GTK 4.22 exposes none to application code: there is no
/// `gtk_widget_get_accent_color`, and `@accent_bg_color` is a CSS name libadwaita defines, which
/// is a dependency this crate did not take. ADR 0244.
fn draw_chrome(
    area: &gtk4::DrawingArea,
    cr: &gtk4::cairo::Context,
    chrome: &Chrome,
) -> Result<(), gtk4::cairo::Error> {
    let colour = area.color();
    let scale = if chrome.scale > 0.0 {
        chrome.scale
    } else {
        1.0
    };
    if !chrome.highlights.is_empty() {
        // Annex O's rectangle, under both of the others: it says how the document was *opened*,
        // where a match and a selection say what is happening now. Fainter still, for the same
        // reason the matches are fainter than the selection.
        cr.set_source_rgba(
            f64::from(colour.red()),
            f64::from(colour.green()),
            f64::from(colour.blue()),
            0.18,
        );
        for quad in &chrome.highlights {
            trace_quad(cr, *quad, scale);
        }
        cr.fill()?;
    }
    if !chrome.matches.is_empty() {
        // Fainter than the selection and in the same colour, which is this platform's answer to
        // "what does a match look like": GTK exposes no accent colour to application code, so
        // there is nothing else honest to draw it in. The alpha is what distinguishes *a* match
        // from *the* match a person is on.
        cr.set_source_rgba(
            f64::from(colour.red()),
            f64::from(colour.green()),
            f64::from(colour.blue()),
            0.12,
        );
        for quad in &chrome.matches {
            trace_quad(cr, *quad, scale);
        }
        cr.fill()?;
    }
    if !chrome.selection.is_empty() {
        cr.set_source_rgba(
            f64::from(colour.red()),
            f64::from(colour.green()),
            f64::from(colour.blue()),
            0.25,
        );
        for quad in &chrome.selection {
            trace_quad(cr, *quad, scale);
        }
        cr.fill()?;
    }
    if chrome.measuring.len() > 1 {
        // §12.9's path itself. Solid and on top: every other shape here is something the document
        // or a search put on the page, and this is the one a person is drawing.
        cr.set_source_rgba(
            f64::from(colour.red()),
            f64::from(colour.green()),
            f64::from(colour.blue()),
            0.85,
        );
        cr.set_line_width(1.5);
        for (index, point) in chrome.measuring.iter().enumerate() {
            let (x, y) = (f64::from(point[0]) / scale, f64::from(point[1]) / scale);
            if index == 0 {
                cr.move_to(x, y);
            } else {
                cr.line_to(x, y);
            }
        }
        cr.stroke()?;
    }
    for point in &chrome.measuring {
        // Each press, marked, because the path between two of them is a straight line and a
        // person who put a third point down on top of a second would otherwise see nothing.
        cr.set_source_rgba(
            f64::from(colour.red()),
            f64::from(colour.green()),
            f64::from(colour.blue()),
            0.85,
        );
        cr.arc(
            f64::from(point[0]) / scale,
            f64::from(point[1]) / scale,
            MEASURED_POINT,
            0.0,
            std::f64::consts::TAU,
        );
        cr.fill()?;
    }
    if let Some(quad) = chrome.focus {
        // §12.5.1: an annotation with the input focus. What a focus ring looks like is the
        // platform's, which is why this crosses as one quadrilateral and not as pixels.
        cr.set_source_rgba(
            f64::from(colour.red()),
            f64::from(colour.green()),
            f64::from(colour.blue()),
            0.9,
        );
        cr.set_line_width(2.0);
        trace_quad(cr, quad, scale);
        cr.stroke()?;
    }
    Ok(())
}

/// One `[x0, y0, … x3, y3]` in device pixels, as a closed path in logical ones.
fn trace_quad(cr: &gtk4::cairo::Context, quad: [f32; 8], scale: f64) {
    cr.move_to(f64::from(quad[0]) / scale, f64::from(quad[1]) / scale);
    cr.line_to(f64::from(quad[2]) / scale, f64::from(quad[3]) / scale);
    cr.line_to(f64::from(quad[4]) / scale, f64::from(quad[5]) / scale);
    cr.line_to(f64::from(quad[6]) / scale, f64::from(quad[7]) / scale);
    cr.close_path();
}

/// How large a mark one of §12.9's traced points gets, in logical pixels.
///
/// A choice, and written down as one: the clause states the arithmetic over a path and not one
/// word about what a person sees while they trace it. Small enough not to cover the thing being
/// measured, large enough to find on a dense page.
const MEASURED_POINT: f64 = 2.5;
