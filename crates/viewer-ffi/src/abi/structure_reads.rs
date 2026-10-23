//! The structure tree, flattened into pages, nodes and lines.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::shared::copy_out;
use crate::answers::Structure;
use crate::kinds::{BoxKind, ElementKind};
use crate::session::Session;
use crate::shapes::Quads;
use crate::status::Status;
use core::ffi::{c_char, c_int};

/// §14.7's logical structure for every page the arrangement is showing, as a handle the caller
/// frees.
///
/// **Two indices, and the standard is why.** §14.7.5.2's marked-content identifier "uniquely
/// identifies the marked-content sequence within its content stream" and §14.7.5.4 keys the route
/// in from that page's `/StructParents`, so two pages' trees share no numbering and there is no
/// order between them to renumber by. A caller walks pages with `quorra_structure_page`, then nodes;
/// every index a node carries is into **that page's** list.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_structure_read(
    viewer: *const Session,
    structure: *mut *mut Structure,
) -> c_int {
    let (Some(viewer), Some(out)) = (viewer.as_ref(), structure.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.structure() {
        Ok(tree) => {
            *out = Box::into_raw(Box::new(tree));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases what `quorra_structure_read` produced. Null is a no-op.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_structure_free(structure: *mut Structure) {
    if !structure.is_null() {
        drop(Box::from_raw(structure));
    }
}

/// How many pages the arrangement is showing. Zero for a null pointer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_structure_pages(structure: *const Structure) -> usize {
    structure.as_ref().map_or(0, Structure::len)
}

/// Which page an entry is about, and how many nodes its tree has.
///
/// Zero nodes is an answer rather than a silence: §14.7 leaves a producer free to state no
/// structure, and a reader that invented a reading order for an untagged page would be presenting
/// a guess where a person is entitled to the author's answer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_structure_page(
    structure: *const Structure,
    entry: usize,
    page: *mut usize,
    nodes: *mut usize,
) -> c_int {
    let Some(structure) = structure.as_ref() else {
        return Status::NullArgument.code();
    };
    let (index, count) = match structure.page(entry) {
        Ok(answer) => answer,
        Err(status) => return status.code(),
    };
    if let Some(page) = page.as_mut() {
        *page = index;
    }
    if let Some(nodes) = nodes.as_mut() {
        *nodes = count;
    }
    Status::Ok.code()
}

/// A node's parent, §14.9.3's substitution, and Table 384's `/Scope` for a header cell.
///
/// `has_parent` is false for a root. `substituted` is the one a client acts on rather than
/// displays: §14.9.3 makes `/Alt` "a complete (or whole) word or phrase substitution for the
/// current element" and §14.9.5 says the same of `/E`, so an element stating one has said what to
/// speak *instead of* its content and a client stops there rather than descending — descending
/// anyway reads the element twice. `has_scope` is false for every element that is not a `TH` and
/// for a `TH` this reader could place in no grid, which is a caller being told we do not know.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_structure_node(
    structure: *const Structure,
    entry: usize,
    node: usize,
    parent: *mut usize,
    has_parent: *mut bool,
    substituted: *mut bool,
    scope: *mut u32,
    has_scope: *mut bool,
) -> c_int {
    let Some(structure) = structure.as_ref() else {
        return Status::NullArgument.code();
    };
    let (encloser, substitution, header_scope) = match structure.shape(entry, node) {
        Ok(facts) => facts,
        Err(status) => return status.code(),
    };
    if let Some(parent) = parent.as_mut() {
        *parent = encloser.unwrap_or_default();
    }
    if let Some(has_parent) = has_parent.as_mut() {
        *has_parent = encloser.is_some();
    }
    if let Some(substituted) = substituted.as_mut() {
        *substituted = substitution;
    }
    if let Some(scope) = scope.as_mut() {
        *scope = header_scope.map_or(0, |scope| scope as u32);
    }
    if let Some(has_scope) = has_scope.as_mut() {
        *has_scope = header_scope.is_some();
    }
    Status::Ok.code()
}

/// One of a node's three strings: `QUORRA_ELEMENT_ROLE`, `QUORRA_ELEMENT_NAME`,
/// `QUORRA_ELEMENT_LANGUAGE`.
///
/// The role is §14.7.4's `/S` **after §14.7.3's role map**, which is the file's own statement
/// about its own names and a `shall`: "\[a] structure type shall always be mapped to its
/// corresponding name in the role map, if there is one, even if the original name is one of the
/// standard types." Mapping it onto a *platform's* vocabulary is the caller's, and is a different
/// mapping.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_structure_text(
    structure: *const Structure,
    entry: usize,
    node: usize,
    which: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(structure) = structure.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = ElementKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    match structure.text(entry, node, which) {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// Where the element's own text was drawn, as a `quorra_quads *` the caller frees.
///
/// Empty for an element whose content drew no text — a figure, a table cell holding an image —
/// which is a statement about this program's text layer rather than about the element, and is why
/// `quorra_structure_box` exists beside it.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_structure_quads(
    structure: *const Structure,
    entry: usize,
    node: usize,
    quads: *mut *mut Quads,
) -> c_int {
    let (Some(structure), Some(out)) = (structure.as_ref(), quads.as_mut()) else {
        return Status::NullArgument.code();
    };
    match structure.quads(entry, node) {
        Ok(shapes) => {
            *out = Box::into_raw(Box::new(Quads::new(shapes)));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One of a node's two rectangles: `[x0, y0, x1, y1]` in device pixels of the viewport.
///
/// `QUORRA_BOX_STATED` is what the **document** says the element's extent is — Table 379's `/BBox`,
/// which §14.8.5.4.3 makes "the rectangle that completely encloses its visible content" — and
/// `QUORRA_BOX_DRAWN` is where **this program** drew its text. Two kinds of statement, carried side
/// by side rather than merged, because an element whose content is a picture has the first and not
/// the second. `QUORRA_NO_ANSWER` where the node has no rectangle of that kind.
///
/// # Safety
///
/// See the module documentation. `into` is writable for four floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_structure_box(
    structure: *const Structure,
    entry: usize,
    node: usize,
    which: u32,
    into: *mut f32,
) -> c_int {
    let Some(structure) = structure.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = BoxKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    if into.is_null() {
        return Status::NullArgument.code();
    }
    match structure.rectangle(entry, node, which) {
        Ok(rectangle) => {
            core::slice::from_raw_parts_mut(into, rectangle.len()).copy_from_slice(&rectangle);
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// How many header cells §14.8.4.8.3 associates with this element.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_structure_headers(
    structure: *const Structure,
    entry: usize,
    node: usize,
    count: *mut usize,
) -> c_int {
    let (Some(structure), Some(count)) = (structure.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    match structure.headers(entry, node) {
        Ok(number) => {
            *count = number;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One of those header cells, as an index into **this page's** node list.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_structure_header(
    structure: *const Structure,
    entry: usize,
    node: usize,
    header: usize,
    cell: *mut usize,
) -> c_int {
    let (Some(structure), Some(cell)) = (structure.as_ref(), cell.as_mut()) else {
        return Status::NullArgument.code();
    };
    match structure.header(entry, node, header) {
        Ok(index) => {
            *cell = index;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// How many lines of text this element's own content items drew.
///
/// **What AT-SPI's `Text` interface is built on.** `QUORRA_ELEMENT_NAME` is what the element is
/// *called* and these are what it *says*: a name is one string for a whole paragraph, so a client
/// can read the paragraph or not read it, and moving a caret through it by character, by word or by
/// line needs to know where each character begins and which characters share a line. That is what
/// `org.a11y.atspi.Text`'s `GetCharacterExtents`, `GetOffsetAtPoint` and `GetTextAtOffset` ask for
/// and what no string can answer.
///
/// **Not §14.9's substitutions**, deliberately, and the difference is the point: `QUORRA_ELEMENT_NAME`
/// applies §14.9.3's `/Alt` and §14.9.5's `/E`, and this does not — a caret moves over what is on
/// the page, and a phrase that substitutes for the content has no glyphs to report positions for.
/// Zero lines for an element that states one, which is also what `quorra_structure_node`'s
/// `substituted` says, and zero for an element whose content drew no text.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_structure_lines(
    structure: *const Structure,
    entry: usize,
    node: usize,
    count: *mut usize,
) -> c_int {
    let (Some(structure), Some(count)) = (structure.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    match structure.lines(entry, node) {
        Ok(number) => {
            *count = number;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One line's text, with the number of character codes that produced it.
///
/// The text follows this ABI's string convention — `out` and `cap`, with `needed` receiving the
/// length including the terminator — and `characters` receives the count in the same call, because
/// the invariant between them is what a text interface rests on: the character byte counts sum to
/// the text's length, so an offset into the string and an index into the characters convert into
/// each other without either side guessing.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_structure_line(
    structure: *const Structure,
    entry: usize,
    node: usize,
    line: usize,
    characters: *mut usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let (Some(structure), Some(characters)) = (structure.as_ref(), characters.as_mut()) else {
        return Status::NullArgument.code();
    };
    match structure.line(entry, node, line) {
        Ok((text, codes)) => {
            *characters = codes;
            copy_out(text, out, cap, needed)
        }
        Err(status) => status.code(),
    }
}

/// One character code's share of a line: its byte count, and where its glyph is.
///
/// `bytes` is how much of the line's text this code produced — **the unit is the code and not the
/// character**, because a code mapped through `/ToUnicode` to a several-character string drew one
/// glyph in one place and splitting its box would invent positions the file does not state.
/// `into` receives `[x0, y0, x1, y1]` in the device pixels of the viewport, the same space every
/// other shape in this ABI is in.
///
/// # Safety
///
/// See the module documentation. `into` is writable for four floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_structure_character(
    structure: *const Structure,
    entry: usize,
    node: usize,
    line: usize,
    character: usize,
    bytes: *mut usize,
    into: *mut f32,
) -> c_int {
    let (Some(structure), Some(bytes)) = (structure.as_ref(), bytes.as_mut()) else {
        return Status::NullArgument.code();
    };
    if into.is_null() {
        return Status::NullArgument.code();
    }
    match structure.character(entry, node, line, character) {
        Ok((produced, rectangle)) => {
            *bytes = produced;
            core::slice::from_raw_parts_mut(into, rectangle.len()).copy_from_slice(&rectangle);
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}
