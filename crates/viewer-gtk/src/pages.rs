//! ISO 32000-2 §12.3.4's panel: one row per page, with the producer's miniature where there is one.
//!
//! > A PDF document may contain thumbnail images representing the contents of its pages in
//! > miniature form. An interactive PDF processor may display these images on the screen, allowing
//! > the user to navigate to a page by clicking its thumbnail image
//!
//! Both halves of that sentence are here: the pictures, and the click.
//!
//! # The list is virtual, and that is the requirement rather than the technique
//!
//! `CLAUDE.md` section 2 forbids thumbnail generation on the launch path by name, and
//! [`viewer_core::Query::Thumbnail`] answers one page at a time so that a host can obey it — *"the
//! panel knows which eight it is showing; this crate does not"*. A [`gtk4::ListView`] binds a row
//! only when it is about to be shown, so the decode happens in [`bind`] and nowhere else: opening
//! this tab on a thousand-page document asks for the dozen rows on the screen, and scrolling asks
//! for the rows it scrolls onto. What is held afterwards is [`viewer_host::Miniatures`]' business
//! and is bounded there.
//!
//! **A page with no `/Thumb` is still a row.** The clause's NOTE says thumbnails "are not required,
//! and can be included for some pages and not for others", so a list of only the pages carrying one
//! would be a list of the document's *thumbnails* rather than of its pages — and the row still
//! navigates, because §12.4.2's label is what identifies a page to a reader whatever the producer
//! chose to draw.

use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};

use viewer_host::Held;

/// How tall a row's picture is allowed to be, in logical pixels.
///
/// A choice, and the same one the tier-2 host's sidebar makes: about 140 logical pixels shows a
/// portrait page's miniature at roughly the size a producer writes one — Table 87's examples are a
/// few score samples on a side — with a line for the page's label under it. A `GtkPicture` scales
/// to fit within it, so a landscape miniature is narrower and not cropped.
const PICTURE_HEIGHT: i32 = 140;

/// What one row needs, once the host has decoded it.
pub(crate) type Row = Rc<Held<gdk::MemoryTexture>>;

/// A scrollable list of `count` pages.
///
/// `row` is asked for a page's label and miniature the first time GTK binds that row, and `show` is
/// called with the page index when a person activates one. Both are the host, reached weakly, so
/// neither keeps the window alive.
pub(crate) fn page_list(
    count: usize,
    row: &Rc<dyn Fn(usize) -> Option<Row>>,
    show: &Rc<dyn Fn(usize)>,
) -> gtk4::Widget {
    let store = gio::ListStore::new::<glib::BoxedAnyObject>();
    for index in 0..count {
        store.append(&glib::BoxedAnyObject::new(index));
    }

    let factory = gtk4::SignalListItemFactory::new();
    let row = Rc::clone(row);
    factory.connect_bind(move |_, item| bind(item, &row));

    let selection = gtk4::SingleSelection::new(Some(store));
    selection.set_autoselect(false);
    selection.set_can_unselect(true);
    let list = gtk4::ListView::new(Some(selection), Some(factory));
    // §12.3.4's own sentence: "allowing the user to navigate to a page by clicking its thumbnail
    // image". `activate` is GTK's name for that gesture, and it carries the row's position, which
    // *is* the page index — a thumbnail is not a destination to resolve, it is the page.
    let show = Rc::clone(show);
    list.connect_activate(move |_, position| {
        show(position as usize);
    });

    let scroller = gtk4::ScrolledWindow::new();
    scroller.set_child(Some(&list));
    scroller.set_vexpand(true);
    scroller.set_hexpand(true);
    scroller.upcast()
}

/// Builds the widgets for one row, asking the host for that page and no other.
///
/// A row the host could not answer for — which is a re-entrant call and nothing else, since every
/// page of an open document has a label — draws its number rather than nothing, because a blank row
/// in a list of pages is a page a reader cannot find their way to.
fn bind(item: &glib::Object, row: &Rc<dyn Fn(usize) -> Option<Row>>) {
    let Some(item) = item.downcast_ref::<gtk4::ListItem>() else {
        return;
    };
    let Some(held) = item.item().and_downcast::<glib::BoxedAnyObject>() else {
        return;
    };
    let index: usize = *held.borrow::<usize>();

    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    column.set_margin_top(4);
    column.set_margin_bottom(4);
    let answered = row(index);
    if let Some(texture) = answered.as_ref().and_then(|row| row.picture.as_ref()) {
        let picture = gtk4::Picture::for_paintable(texture);
        picture.set_size_request(-1, PICTURE_HEIGHT);
        picture.set_can_shrink(true);
        column.append(&picture);
    }
    let named = answered.as_ref().map_or_else(
        || format!("Page {}", index.saturating_add(1)),
        |row| row.label.clone(),
    );
    let label = gtk4::Label::new(Some(&named));
    label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    column.append(&label);
    item.set_child(Some(&column));
}
