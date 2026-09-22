//! [`PanelRow`]s in a real platform tree.
//!
//! `doc/todo/30`'s first item: "a platform tree view over `Query::Outline`, `Query::Layers` and
//! `Query::Attachments`, rather than `viewer_ui::chrome`'s own drawn sidebar. The chrome exists to
//! prove the queries answer enough; a native host is the proof they answer enough for somebody
//! else's widgets."
//!
//! The widgets are GTK4's modern list stack — [`gtk4::ListView`] over a
//! [`gtk4::TreeListModel`], with a [`gtk4::TreeExpander`] per row — rather than `GtkTreeView`,
//! which GTK deprecated in 4.10. That is not taste: this workspace turns warnings into errors in
//! CI, so a deprecated widget would not build.
//!
//! **Rows build their own widgets in `bind` rather than recycling ones built in `setup`.** The
//! recycling shape is what a list of a million rows needs, and it costs a signal handler whose
//! lifetime has to be managed by hand — the handler on a layer's switch must forget the row it
//! was bound to. An outline is tens to thousands of rows and only the *visible* ones are ever
//! bound, so the trade goes the other way here: fresh widgets, and a handler that cannot outlive
//! the row it names.

use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{gio, glib, pango};

use viewer_host::panel::{PanelRow, Picture, RowAction};

/// A file's own preview picture, where this host has decoded one.
///
/// §12.3.6's `FilmStrip`, `FreeForm` and `Linear` are built out of pictures of the *attachments*,
/// and this is how one reaches a row: the `/EmbeddedFiles` key in, the attachment's own first
/// page's §12.3.4 `/Thumb` out. `None` is most files and is not a defect (ADR 1251).
pub(crate) type Previews = Rc<dyn Fn(&str) -> Option<gtk4::gdk::MemoryTexture>>;

/// How many rows the initial expansion will open before it stops.
///
/// §12.3.3 lets a document ask for an outline to open — the sign of `/Count` — and ISO 32000-2's
/// own outline is nine hundred items, so "do what the file asked" has to have an end. A document
/// that asks for more than this gets the first of them opened and the rest closed, which is a
/// tree a person can still open by hand.
const EXPANSION_LIMIT: u32 = 4096;

/// A scrollable tree over these rows, with `act` called for every row a person acts on.
///
/// `pictures` answers a file's own preview where the row asks for one, and is called from `bind`
/// — so a collection of a hundred files opens the embedded documents of the rows on the screen
/// and no others, which is the demand-driven rule §12.3.4's page list already follows.
pub(crate) fn tree(
    rows: &[PanelRow],
    act: &Rc<dyn Fn(&RowAction)>,
    pictures: &Previews,
) -> gtk4::Widget {
    let model = gtk4::TreeListModel::new(store(rows), false, false, |parent| {
        let row = parent.downcast_ref::<glib::BoxedAnyObject>()?;
        let children = row.borrow::<PanelRow>().children.clone();
        if children.is_empty() {
            return None;
        }
        Some(store(&children).upcast())
    });
    open_what_the_document_asked_for(&model);

    let factory = gtk4::SignalListItemFactory::new();
    let act = Rc::clone(act);
    let pictures = Rc::clone(pictures);
    factory.connect_bind(move |_, item| bind(item, &act, &pictures));

    let selection = gtk4::SingleSelection::new(Some(model));
    selection.set_autoselect(false);
    selection.set_can_unselect(true);
    let list = gtk4::ListView::new(Some(selection), Some(factory));

    let scroller = gtk4::ScrolledWindow::new();
    scroller.set_child(Some(&list));
    scroller.set_vexpand(true);
    scroller.set_hexpand(true);
    scroller.upcast()
}

/// The rows of one level, as a list model.
fn store(rows: &[PanelRow]) -> gio::ListStore {
    let store = gio::ListStore::new::<glib::BoxedAnyObject>();
    for row in rows {
        store.append(&glib::BoxedAnyObject::new(row.clone()));
    }
    store
}

/// Opens the rows whose document asked for them to be open, bounded by [`EXPANSION_LIMIT`].
///
/// Expanding a row inserts its children after it, so the scan walks forward over a model that
/// grows underneath it — which is what makes one pass enough for a whole subtree.
fn open_what_the_document_asked_for(model: &gtk4::TreeListModel) {
    let mut index = 0_u32;
    while index < model.n_items().min(EXPANSION_LIMIT) {
        if let Some(row) = model.row(index) {
            let wanted = row
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .is_some_and(|held| held.borrow::<PanelRow>().expanded);
            if wanted && !row.is_expanded() {
                row.set_expanded(true);
            }
        }
        index = index.saturating_add(1);
    }
}

/// Builds the widgets for one row and wires what acting on it does.
fn bind(item: &glib::Object, act: &Rc<dyn Fn(&RowAction)>, pictures: &Previews) {
    let Some(item) = item.downcast_ref::<gtk4::ListItem>() else {
        return;
    };
    let Some(tree_row) = item.item().and_downcast::<gtk4::TreeListRow>() else {
        return;
    };
    let Some(held) = tree_row.item().and_downcast::<glib::BoxedAnyObject>() else {
        return;
    };
    let row = held.borrow::<PanelRow>().clone();

    let line = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    // Table 153's `/View T` is "a small icon" and §12.3.6's three further layouts are built out
    // of thumbnails and previews of the attachments themselves. Which *kind* of thing a row is
    // stays `viewer_host::panel::Icon`'s, shared with `viewer-qt`; the icon's picture is the
    // running theme's, because the clause states no artwork at all (ADRs 1215, 1251).
    if let Some(picture) = row.picture {
        line.append(&picture_for(&row, picture, pictures));
    }
    if let RowAction::Toggle { group, on, locked } = row.action {
        let switch = gtk4::CheckButton::new();
        switch.set_active(on);
        // Table 99's `/Locked`: "[t]he state of a locked group cannot be changed through the user
        // interface of an interactive PDF processor". An insensitive switch is the platform's own
        // way of saying so, and it is the clause obeyed rather than merely carried.
        switch.set_sensitive(!locked);
        // Connected *after* the state is set, so that showing the document's own answer is not
        // mistaken for a person changing it.
        let act = Rc::clone(act);
        switch.connect_toggled(move |switch| {
            act(&RowAction::Toggle {
                group,
                on: switch.is_active(),
                locked,
            });
        });
        line.append(&switch);
    }

    let text = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    // The text fills the rest of the row, so that §12.3.3's "[c]licking the text" is the whole
    // row rather than the width of the string — a title of four characters would otherwise be a
    // four-character target.
    text.set_hexpand(true);
    let label = gtk4::Label::new(Some(&row.label));
    label.set_xalign(0.0);
    label.set_ellipsize(pango::EllipsizeMode::End);
    // `PanelRow::note` is a sentence *about* the document rather than a thing in it — §14.3.2's
    // heading, or "this document states no article threads". Adwaita's own dimmed style is what
    // this platform says that with, which is the whole of what a host adds to the decision.
    if row.note {
        label.add_css_class("dim-label");
    }
    // `PanelRow::emphasis` is §12.3.5.1's `/D` — "the document that shall be initially presented
    // in the user interface" — and the clause states no appearance for it. Adwaita's `heading`
    // class is what this platform has for "this row before the others"; `viewer-qt` says it with
    // a bold `Qt::FontRole` and `viewer-ui` with a bold face. Which row it is, is the one shared
    // decision (`viewer_host::panel::collection_rows`).
    if row.emphasis {
        label.add_css_class("heading");
    }
    text.append(&label);
    // Table 153's `/View D` asks for "all information in the Schema dictionary presented in a
    // multi- column format", and `/View T` for a subset of the same fields. A `GtkGrid` of
    // heading-and-value pairs is this toolkit's multi-column format; every other panel carries no
    // cells at all and falls to the one detail line (ADR 1215).
    if row.cells.is_empty() {
        if let Some(detail) = row.detail.as_deref() {
            let second = gtk4::Label::new(Some(detail));
            second.set_xalign(0.0);
            second.set_ellipsize(pango::EllipsizeMode::End);
            second.add_css_class("dim-label");
            text.append(&second);
        }
    } else {
        let columns = gtk4::Grid::new();
        columns.set_column_spacing(12);
        for (at, cell) in row.cells.iter().enumerate() {
            let at = i32::try_from(at).unwrap_or(i32::MAX);
            // Table 155's `/N` is "[t]he textual field name that shall be presented to the user",
            // so the heading is the document's own word above the document's own value.
            let heading = gtk4::Label::new(Some(&cell.heading));
            heading.set_xalign(0.0);
            heading.set_ellipsize(pango::EllipsizeMode::End);
            heading.add_css_class("dim-label");
            heading.add_css_class("caption");
            let value = gtk4::Label::new(Some(&cell.value));
            value.set_xalign(0.0);
            value.set_ellipsize(pango::EllipsizeMode::End);
            columns.attach(&heading, at, 0, 1, 1);
            columns.attach(&value, at, 1, 1, 1);
        }
        text.append(&columns);
    }
    line.append(&text);

    let expander = gtk4::TreeExpander::new();
    expander.set_list_row(Some(&tree_row));
    expander.set_child(Some(&line));

    // §12.3.3: "[c]licking the text of any visible item activates the item". The gesture is on the
    // *text* rather than on the whole row, because a click on a layer's switch must change the
    // layer and not also activate the row the switch sits in.
    if matches!(
        row.action,
        RowAction::Activate(_) | RowAction::Extract { .. }
    ) {
        let click = gtk4::GestureClick::new();
        let act = Rc::clone(act);
        let action = row.action.clone();
        click.connect_released(move |_, _, _, _| act(&action));
        text.add_controller(click);
    }

    item.set_child(Some(&expander));
}

/// The widget a row's [`Picture`] is drawn as.
///
/// The file's own preview where §12.3.6 asks for one and this host has decoded it; the icon
/// theme's picture for the kind otherwise, which is also what Table 153's `/View T` asks for
/// outright. The size is this toolkit's ink — the clause states "a small icon", "thumbnails" and
/// "a large size preview" and no measurement (ADR 1251).
pub(crate) fn picture_for(row: &PanelRow, picture: Picture, pictures: &Previews) -> gtk4::Widget {
    let side = match picture {
        Picture::Icon(_) => 0,
        Picture::Thumbnail(_) => 64,
        Picture::Preview(_) => 128,
    };
    if picture.wants_the_files_own_picture()
        && let RowAction::Extract { name } = &row.action
        && let Some(texture) = pictures(name)
    {
        let image = gtk4::Image::from_paintable(Some(&texture));
        image.set_pixel_size(side);
        return image.upcast();
    }
    let image = gtk4::Image::from_icon_name(picture.kind().theme_name());
    if side == 0 {
        image.set_icon_size(gtk4::IconSize::Large);
    } else {
        image.set_pixel_size(side);
    }
    image.upcast()
}

/// §12.3.6's `FreeForm`, which is a surface rather than a list.
///
/// > The FreeForm layout provides a simple layout, in which thumbnails for each item in the
/// > collection contents are displayed at a random location on the view.
///
/// A `GtkFixed` is this toolkit's surface with places on it, and the places are
/// `viewer_host::panel::scattered`'s so that the three windows put the same file in the same
/// spot. The rows that are sentences rather than files go under it, because a scatter is where
/// the *files* go (ADR 1251).
pub(crate) fn scatter(
    rows: &[PanelRow],
    act: &Rc<dyn Fn(&RowAction)>,
    pictures: &Previews,
) -> gtk4::Widget {
    let surface = gtk4::Fixed::new();
    let mut sentences = Vec::new();
    for row in rows {
        let RowAction::Extract { name } = &row.action else {
            sentences.push(row);
            continue;
        };
        let item = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        if let Some(picture) = row.picture {
            item.append(&picture_for(row, picture, pictures));
        }
        let label = gtk4::Label::new(Some(&row.label));
        label.set_ellipsize(pango::EllipsizeMode::End);
        label.set_max_width_chars(12);
        item.append(&label);
        let click = gtk4::GestureClick::new();
        let act = Rc::clone(act);
        let action = row.action.clone();
        click.connect_released(move |_, _, _, _| act(&action));
        item.add_controller(click);
        let (across, down) = viewer_host::panel::scattered(name);
        let room = f64::from(SCATTER_ROOM);
        surface.put(&item, f64::from(across) * room, f64::from(down) * room);
    }
    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    surface.set_size_request(0, SCATTER_ROOM.saturating_add(SCATTER_ITEM));
    column.append(&surface);
    for row in sentences {
        let label = gtk4::Label::new(Some(&row.label));
        label.set_xalign(0.0);
        label.set_wrap(true);
        label.add_css_class("dim-label");
        column.append(&label);
    }
    let scroller = gtk4::ScrolledWindow::new();
    scroller.set_child(Some(&column));
    scroller.set_vexpand(true);
    scroller.set_hexpand(true);
    scroller.upcast()
}

/// How much room §12.3.6's `FreeForm` scatters its thumbnails over, in logical pixels.
///
/// Ink rather than a reading: the clause asks for "a random location on the view" and states no
/// extent. Six hundred is twice the panel's own width, so a scatter is a scatter and the
/// `GtkScrolledWindow` around it reaches the rest (ADR 1251).
const SCATTER_ROOM: i32 = 600;

/// How much room one of those thumbnails takes under its own place.
const SCATTER_ITEM: i32 = 96;
