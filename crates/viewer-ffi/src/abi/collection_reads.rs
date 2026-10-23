//! A portfolio's collection: columns, folders, order and layouts.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::shared::copy_out;
use crate::answers::Collection;
use crate::kinds::{ColumnTextKind, FolderTextKind};
use crate::session::Session;
use crate::status::Status;
use core::ffi::{CStr, c_char, c_int};

/// §12.3.5's portable collection, as a handle the caller frees.
///
/// The clause is a `shall` on a viewer — "\[i]f this dictionary is present in a PDF document, the
/// interactive PDF processor shall present the document as a portable collection" — so what
/// crosses is everything needed to *arrange* the files `quorra_attachments_read` already lists:
/// Table 153's `/View`, §12.3.5.1's resolved initial document, Table 155's columns in `/O` order,
/// and §12.3.5.2's folder tree flattened depth first. `quorra_collection_folder_of` is the fifth
/// piece and the one a caller could not compute for itself.
///
/// `QUORRA_NO_ANSWER` where the catalogue states no collection, which is every document in this
/// project's corpora.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_read(
    viewer: *const Session,
    collection: *mut *mut Collection,
) -> c_int {
    let (Some(viewer), Some(out)) = (viewer.as_ref(), collection.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.collection() {
        Ok(read) => {
            *out = Box::into_raw(Box::new(read));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases what `quorra_collection_read` produced. Null is a no-op.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_free(collection: *mut Collection) {
    if !collection.is_null() {
        drop(Box::from_raw(collection));
    }
}

/// Table 158's `/Direction` `N`: whether the file navigation view takes the whole window.
///
/// §12.3.5.1, Table 158:
///
/// > N indicates that the window is not split. The entire window region shall be dedicated to the
/// > file navigation view.
///
/// Non-zero where the document asked for it, zero otherwise — including where Table 153's `/View`
/// is `H`, since a view the document asked to hide is not one a window is dedicated to. The rest
/// of the split dictionary does not cross: `H`, `V` and `/Position` state where a splitter bar
/// goes in an initial view this library draws nothing of, which is the platform's furniture and
/// not a fact about the file (ADRs 1168, 1252).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_whole_window(
    collection: *const Collection,
    whole: *mut u32,
) -> c_int {
    let (Some(collection), Some(whole)) = (collection.as_ref(), whole.as_mut()) else {
        return Status::NullArgument.code();
    };
    *whole = u32::from(collection.whole_window());
    Status::Ok.code()
}

/// Table 153's `/View`: how the collection is first presented.
///
/// `QUORRA_COLLECTION_HIDDEN` is the one value that is load-bearing rather than a preference:
/// §7.6.7's unencrypted wrapper document requires it, because the wrapper's own page says the
/// payload is encrypted and showing a file browser over it would hide that.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_view(
    collection: *const Collection,
    view: *mut u32,
) -> c_int {
    let (Some(collection), Some(view)) = (collection.as_ref(), view.as_mut()) else {
        return Status::NullArgument.code();
    };
    *view = collection.view() as u32;
    Status::Ok.code()
}

/// §12.3.5.1's outcome for `/D`, resolved, and the `/EmbeddedFiles` key where it names a file.
///
/// A *resolved* answer rather than the entry: Table 153's `/D` "identifies an entry in the
/// `EmbeddedFiles` name tree, determining the document that shall be initially presented in the
/// user interface", the tree is the document's, and turning a byte string into one of four
/// outcomes is therefore not a caller's to do. The key is empty for the three outcomes that name
/// no file, and is what `quorra_extract` takes for the one that does.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_initial(
    collection: *const Collection,
    kind: *mut u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(collection) = collection.as_ref() else {
        return Status::NullArgument.code();
    };
    let (outcome, name) = collection.initial();
    if let Some(kind) = kind.as_mut() {
        *kind = outcome as u32;
    }
    copy_out(name, out, cap, needed)
}

/// How many columns Table 153's `/Schema` states.
///
/// Zero is a permission rather than a gap: the table says an absent schema lets a processor
/// "choose useful defaults that are known to exist in a file specification dictionary, such as the
/// file name, file size, and modified date", so an empty list is the document declining to choose.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_columns(
    collection: *const Collection,
    count: *mut usize,
) -> c_int {
    let (Some(collection), Some(count)) = (collection.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    *count = collection.columns();
    Status::Ok.code()
}

/// One column's subtype, where its value lives, Table 155's `/O`, `/V` and `/E`.
///
/// The columns are already in `/O` order — "\[t]he relative order of the field name in the user
/// interface" — with a field stating none after every field that states one, which is the only
/// order left when the file says nothing. `in_the_item` is the clause's own division stated rather
/// than left to be derived: the first three subtypes "identify the types of fields in the
/// collection item … dictionary" and the rest "identify the types of file-related fields", so it
/// says whether a caller reads §7.11.6's `/CI` or the file specification it already has.
/// `has_order` is false where the field states no `/O`.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_column(
    collection: *const Collection,
    index: usize,
    kind: *mut u32,
    in_the_item: *mut bool,
    order: *mut i64,
    has_order: *mut bool,
    flags: *mut u32,
) -> c_int {
    let Some(collection) = collection.as_ref() else {
        return Status::NullArgument.code();
    };
    let (subtype, item, stated_order, visible, editable) = match collection.column(index) {
        Ok(facts) => facts,
        Err(status) => return status.code(),
    };
    if let Some(kind) = kind.as_mut() {
        *kind = subtype as u32;
    }
    if let Some(in_the_item) = in_the_item.as_mut() {
        *in_the_item = item;
    }
    if let Some(order) = order.as_mut() {
        *order = stated_order.unwrap_or_default();
    }
    if let Some(has_order) = has_order.as_mut() {
        *has_order = stated_order.is_some();
    }
    if let Some(flags) = flags.as_mut() {
        *flags = u32::from(visible) | (u32::from(editable) << 1_u32);
    }
    Status::Ok.code()
}

/// One of a column's three strings: `QUORRA_COLUMN_NAME`, `QUORRA_COLUMN_KEY`, `QUORRA_COLUMN_SUBTYPE`.
///
/// The third is the whole of what `QUORRA_COLLECTION_FIELD_OTHER` leaves to say: a subtype this
/// standard does not define is still a name the file wrote, and a number a caller cannot resolve
/// beside a name it cannot read would be a silent fallback in a header.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_column_text(
    collection: *const Collection,
    index: usize,
    which: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(collection) = collection.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = ColumnTextKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    match collection.column_text(index, which) {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// How many folders §12.3.5.2's tree holds, counting every level.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_folders(
    collection: *const Collection,
    count: *mut usize,
) -> c_int {
    let (Some(collection), Some(count)) = (collection.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    *count = collection.folders();
    Status::Ok.code()
}

/// One folder's `/ID`, its depth in the tree, and whether it states a `/Thumb`.
///
/// Depth first with a depth on each row, exactly as §12.3.3's outline crosses and for the same
/// reason: a tree is the one shape a C ABI cannot hand over as itself. The `/ID` is "a
/// non-negative integer value representing the unique folder identification number", and it is
/// what `quorra_collection_folder_of` answers with.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_folder(
    collection: *const Collection,
    index: usize,
    id: *mut u32,
    depth: *mut u32,
    has_thumbnail: *mut bool,
) -> c_int {
    let Some(collection) = collection.as_ref() else {
        return Status::NullArgument.code();
    };
    let (number, level, thumbnail) = match collection.folder(index) {
        Ok(facts) => facts,
        Err(status) => return status.code(),
    };
    if let Some(id) = id.as_mut() {
        *id = number;
    }
    if let Some(depth) = depth.as_mut() {
        *depth = level;
    }
    if let Some(has_thumbnail) = has_thumbnail.as_mut() {
        *has_thumbnail = thumbnail;
    }
    Status::Ok.code()
}

/// One of a folder's two strings: `QUORRA_FOLDER_NAME`, `QUORRA_FOLDER_DESCRIPTION`.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_folder_text(
    collection: *const Collection,
    index: usize,
    which: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(collection) = collection.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = FolderTextKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    match collection.folder_text(index, which) {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// How many `/EmbeddedFiles` keys Table 153's `/Sort` put in an order.
///
/// §12.3.5.1's Table 153, on the entry:
///
/// > A collection sort dictionary, which specifies the order in which items in the collection
/// > shall be sorted in the user interface
///
/// Zero where the document states no `/Sort`, which is the collection saying nothing about the
/// order: a caller then shows the files as `quorra_attachments_read` listed them, which is the
/// `/EmbeddedFiles` tree's own order.
///
/// **The order and not the values it was computed from**, which is the same division
/// `quorra_collection_initial` takes: §12.3.5.1's comparisons need §7.11.6's collection item on
/// each file specification's `/CI`, which no caller of this library holds, so the answer crosses
/// and the arithmetic does not (ADR 1168).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_ordered(
    collection: *const Collection,
    count: *mut usize,
) -> c_int {
    let (Some(collection), Some(count)) = (collection.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    *count = collection.ordered();
    Status::Ok.code()
}

/// The `/EmbeddedFiles` key at one place in Table 153's `/Sort` order.
///
/// The tree's key, folder number and all, because that is what `quorra_extract` names a file by
/// and what `quorra_attachment_text`'s `QUORRA_ATTACHMENT_NAME` answers — so a caller matches the
/// two by string and needs nothing else.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_order_key(
    collection: *const Collection,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(collection) = collection.as_ref() else {
        return Status::NullArgument.code();
    };
    match collection.order_key(index) {
        Some(key) => copy_out(key, out, cap, needed),
        None => Status::OutOfRange.code(),
    }
}

/// How many named layouts §12.3.6's navigator states.
///
/// Zero where the collection states no `/Navigator`, which is every collection written before
/// PDF 2.0 and most written since. A caller that gets zero presents the collection according to
/// `quorra_collection_view` alone, which is §12.3.5.1's own instruction.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_layouts(
    collection: *const Collection,
    count: *mut usize,
) -> c_int {
    let (Some(collection), Some(count)) = (collection.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    *count = collection.layouts();
    Status::Ok.code()
}

/// One of them: `QUORRA_NAVIGATOR_*`, and the name the file wrote for a custom one.
///
/// **The list rather than a selection, and that is §12.3.6 rather than a preference**: "\[w]hen
/// multiple names are provided, an interactive PDF processor should present the first one it is
/// capable of displaying in the order present in the array." Which layouts a caller of this
/// library can draw is a fact about that caller, so the choice is the caller's and this is what it
/// needs to make it — a library that chose here would be choosing with the wrong window in mind.
///
/// `out` receives the layout's own name only for `QUORRA_NAVIGATOR_CUSTOM`, whose name §12.3.6
/// leaves to whoever registered it; the seven the table defines write an empty string, because
/// their number *is* the name.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_layout(
    collection: *const Collection,
    index: usize,
    kind: *mut u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(collection) = collection.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some((named, name)) = collection.layout(index) else {
        return Status::OutOfRange.code();
    };
    if let Some(kind) = kind.as_mut() {
        *kind = named as u32;
    }
    copy_out(name, out, cap, needed)
}

/// Which folder an `/EmbeddedFiles` key names, and the file name inside it.
///
/// **The one piece of §12.3.5 a caller could not compute**, and the reason this is a function
/// rather than a note in the header. §12.3.5.2 gives the key of a file in a folder as the folder's
/// identification number in angle brackets followed by the file name; a caller holding a folder
/// tree and a file list has no way to put one inside the other without that grammar. A key with no
/// such prefix is a file at the root, which answers `QUORRA_NO_ANSWER` and copies the key unchanged.
///
/// It takes no viewer, because it is a fact about a string rather than about a document.
///
/// # Safety
///
/// See the module documentation. `key` is NUL-terminated and UTF-8; `out` is writable for `cap`
/// bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_collection_folder_of(
    key: *const c_char,
    id: *mut u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    if key.is_null() {
        return Status::NullArgument.code();
    }
    let Ok(key) = CStr::from_ptr(key).to_str() else {
        return Status::NotUtf8.code();
    };
    let Some((folder, name)) = pdf_model::collection::folder_of(key) else {
        // The key names no folder, so the file is at the root. The name is still copied — it is
        // the key itself — so that a caller may write one loop rather than two.
        let written = copy_out(key, out, cap, needed);
        return if written == Status::Ok.code() {
            Status::NoAnswer.code()
        } else {
            written
        };
    };
    if let Some(id) = id.as_mut() {
        *id = folder;
    }
    copy_out(name, out, cap, needed)
}
