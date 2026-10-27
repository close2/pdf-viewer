//! The host's checks that need no display.

use super::input::{key_pressed, modifiers};
use gtk4::gdk::Key as Gdk;
use viewer_host::Tab;

/// This host's whole contribution to what a modifier means, checked without a display.
///
/// Three things, and each was wrong here before ADR 1192: Shift crosses, Control crosses,
/// and a modifier this program does not bind is folded into neither. The last is what makes
/// `viewer_host::ctrl_meaning` answer nothing for an Alt a window manager is about to take.
#[test]
fn both_modifiers_the_table_reads_cross_and_no_others_do() {
    use gtk4::gdk::ModifierType as Held;
    assert_eq!(modifiers(Held::empty()), viewer_host::Modifiers::NONE);
    assert_eq!(modifiers(Held::SHIFT_MASK), viewer_host::Modifiers::SHIFT);
    assert_eq!(modifiers(Held::CONTROL_MASK), viewer_host::Modifiers::CTRL);
    assert_eq!(
        modifiers(Held::SHIFT_MASK | Held::CONTROL_MASK),
        viewer_host::Modifiers {
            shift: true,
            ctrl: true
        }
    );
    assert_eq!(
        modifiers(Held::ALT_MASK | Held::SUPER_MASK | Held::META_MASK),
        viewer_host::Modifiers::NONE,
        "a modifier this program does not bind is not passed off as one it does"
    );
}

/// Every key the shared table states has a `gdk::Key` in this host.
///
/// **This is the instrument the level-hosts decision never had** (ADR 0526). The match is
/// exhaustive over [`viewer_host::Key`], so a binding added to `viewer-host` fails to compile
/// here until this host says which key produces it, and the assertion then checks that the
/// *runtime* translation agrees — a key named here and forgotten in [`key_pressed`] fails
/// rather than drifting. `viewer-ui` and `viewer-qt` carry the same test against their own
/// toolkits.
///
/// It needs no display: a `gdk::Key` is a wrapped keyval and nothing here calls into GTK.
#[test]
fn every_key_the_table_states_has_one_in_this_toolkit() {
    use viewer_host::Key as Stated;
    for stated in Stated::ALL {
        let key = match stated {
            Stated::A => Gdk::a,
            Stated::B => Gdk::b,
            Stated::C => Gdk::c,
            Stated::F => Gdk::f,
            Stated::H => Gdk::h,
            Stated::K => Gdk::k,
            Stated::L => Gdk::l,
            Stated::M => Gdk::m,
            Stated::O => Gdk::o,
            Stated::P => Gdk::p,
            Stated::R => Gdk::r,
            Stated::S => Gdk::s,
            Stated::T => Gdk::t,
            Stated::W => Gdk::w,
            Stated::Y => Gdk::y,
            Stated::Z => Gdk::z,
            Stated::Zero => Gdk::_0,
            Stated::Plus => Gdk::plus,
            Stated::Minus => Gdk::minus,
            Stated::Equals => Gdk::equal,
            Stated::Slash => Gdk::slash,
            Stated::Question => Gdk::question,
            Stated::Escape => Gdk::Escape,
            Stated::Tab => Gdk::Tab,
            Stated::Space => Gdk::space,
            Stated::Enter => Gdk::Return,
            Stated::Home => Gdk::Home,
            Stated::End => Gdk::End,
            Stated::Left => Gdk::Left,
            Stated::Right => Gdk::Right,
            Stated::Up => Gdk::Up,
            Stated::Down => Gdk::Down,
            Stated::PageUp => Gdk::Page_Up,
            Stated::PageDown => Gdk::Page_Down,
        };
        assert_eq!(
            key_pressed(key),
            Some(*stated),
            "{stated:?} is stated by the table and this host does not produce it"
        );
    }
}

/// Every panel the shared list states has a page of this notebook, at the same place.
///
/// **The instrument `doc/todo/30`'s "all three hosts stay level" never had for a *panel***, and
/// the same shape as the key test above (ADR 0526). Two halves: [`super::panels::notebook_page`] is
/// exhaustive over [`viewer_host::Tab`], so a panel added there fails to compile in this host
/// — and in `Host::panel_of`, which is where its answer has to come from — and the assertion
/// then checks that this host's page numbers are the shared list's own order, which is what
/// `Ui::slots` is indexed by. `viewer-ui` and `viewer-qt` carry the same test.
///
/// It needs no display: nothing here calls into GTK.
#[test]
fn every_panel_the_list_states_has_a_page_of_this_notebook() {
    for (place, tab) in Tab::ALL.iter().enumerate() {
        assert_eq!(
            u64::from(super::panels::notebook_page(*tab)),
            place as u64,
            "{tab:?} is the {place}th panel and this host puts it somewhere else"
        );
    }
}

/// §12.3.3's title reaches this host's `GtkLabel` as the document stored it — logical order,
/// nominal letters — which is the text Pango's own bidirectional algorithm and shaping start
/// from (ADR 1417).
///
/// `quorra` joins and orders a label itself, in `viewer-ui`'s chrome and nowhere shared; a row
/// that reached this host already ordered would be reordered a second time by Pango. Driven under
/// `Xvfb`, Pango draws the same joined forms, in the same order, as `quorra`'s chrome does.
#[test]
fn an_arabic_outline_title_reaches_pango_as_stored() {
    let stored =
        "\u{627}\u{644}\u{641}\u{635}\u{644} 12: \u{627}\u{644}\u{633}\u{644}\u{627}\u{645}";
    let outline = pdf_model::outline::Outline {
        items: vec![pdf_model::outline::Item {
            id: pdf_syntax::ObjectId::new(5, 0),
            title: stored.to_owned(),
            destination: None,
            open: false,
            italic: false,
            bold: false,
            colour: [0.0; 3],
            children: Vec::new(),
        }],
        stated_count: None,
    };
    let rows = viewer_host::panel::outline_rows(&outline);
    assert_eq!(rows.first().map(|row| row.label.as_str()), Some(stored));
}

/// A capital letter is the same key, because GDK reports the shifted keyval.
#[test]
fn a_capital_letter_is_the_same_key_as_its_lower_case() {
    assert_eq!(key_pressed(Gdk::A), key_pressed(Gdk::a));
    assert_eq!(key_pressed(Gdk::Z), key_pressed(Gdk::z));
}

/// A key this host does not bind produces nothing rather than a default.
#[test]
fn an_unbound_key_is_nothing_rather_than_something() {
    assert_eq!(key_pressed(Gdk::F1), None);
    assert_eq!(key_pressed(Gdk::g), None);
}

/// A right-to-left paragraph's stops reach Pango from the line's start edge with each side
/// renamed by that edge (ADR 1690): in a label 400 px wide, a stop 75 points (100 px) from the
/// left margin whose text ends at it is 300 px from the right and `Left` there, and one whose
/// text starts at it `Right`; a paragraph read left to right keeps both as they are.
#[test]
fn a_right_to_left_stop_reaches_pango_from_the_right_edge_by_its_name_there() {
    use viewer_host::popup::{TabSide, TabStop};
    let stop = |side, at| TabStop {
        side,
        at,
        leader: None,
    };
    let stops = [stop(TabSide::Right, 75.0), stop(TabSide::Left, 150.0)];
    let handed = |tabs: pango::TabArray| -> Vec<(pango::TabAlign, i32)> {
        (0..tabs.size()).map(|index| tabs.tab(index)).collect()
    };
    assert_eq!(
        handed(super::rich_text::pango_tabs(
            &viewer_host::popup::from_start_edge(&stops, 300.0),
            true
        )),
        vec![(pango::TabAlign::Right, 200), (pango::TabAlign::Left, 300)]
    );
    assert_eq!(
        handed(super::rich_text::pango_tabs(&stops, false)),
        vec![(pango::TabAlign::Right, 100), (pango::TabAlign::Left, 200)]
    );
}
