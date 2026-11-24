//! The classes of repair this crate's readers make, on `pdf_syntax`'s one channel (ADR 3477,
//! ADR 3505, ADR 3541).
//!
//! # Why this crate declares its own
//!
//! `pdf_syntax::repair` holds the channel — [`pdf_syntax::RepairClass`], [`pdf_syntax::Site`] and
//! the log on the [`pdf_syntax::Document`] — and the classes of the syntax layer. The page tree,
//! the content reader and the interpreter repair at a level that layer cannot name: a `/MediaBox`
//! that is a name, a `/Rotate` of 45, a `Q` with nothing saved. Their classes are declared here,
//! listed in [`CLASSES`], and recorded on the document the reader reads from, so a validator reads
//! one log and maps every class by its stable name.
//!
//! Where a repair is recorded:
//!
//! - **the page tree's** at [`pdf_syntax::Site::Object`] of the node or page that stated the
//!   entry (or [`pdf_syntax::Site::WholeFile`] where no object names it);
//! - **a content stream's** — what lexing it, reading its inline objects and interpreting its
//!   operators needed — at [`pdf_syntax::Site::Content`] of the page or form `XObject` whose stream
//!   it is, so a converter can tell a repair that crosses a rewrite byte for byte (ADR 3521).
//!
//! What is **not** a class is said beside the code, with a `// not a repair:` comment naming the
//! clause that makes the reading the specification's own.

use pdf_syntax::RepairClass;

macro_rules! classes {
    ($($(#[$doc:meta])* $item:ident = $name:literal, $clause:literal, $instead:literal;)*) => {
        $(
            $(#[$doc])*
            pub static $item: RepairClass = RepairClass {
                name: $name,
                clause: $clause,
                read_instead: $instead,
            };
        )*
        /// Every class this crate records, in declaration order (ADR 3477).
        pub static CLASSES: &[&RepairClass] = &[$(&$item),*];
    };
}

classes! {
    // ---- the page tree, §7.7.2 to §7.7.3.4 and §14.11.2 ------------------------------------

    /// A document catalog that could not be read, so no page tree was reached through it.
    CATALOG_UNREADABLE = "model/catalog-unreadable", "ISO 32000-2 §7.7.2",
        "a document catalog that could not be read was read as one naming no page tree";
    /// A root `/Count` that is absent, no integer, or no positive count of leaves.
    PAGE_COUNT_UNUSABLE = "model/page-count-unusable", "ISO 32000-2 §7.7.3.2",
        "a page tree root whose /Count is absent or not a positive integer was counted by \
         walking its leaves";
    /// A page tree that reaches no page, replaced by the page objects a scan of the file finds.
    PAGE_TREE_FOUND_BY_SCANNING = "model/page-tree-found-by-scanning", "ISO 32000-2 §7.7.3.2",
        "a page tree that reaches no page was replaced by the page objects a scan of the file \
         found";
    /// A page object whose dictionary stops part way, read as the entries before the damage.
    PAGE_DICTIONARY_DAMAGED = "model/page-dictionary-damaged", "ISO 32000-2 §7.7.3.3",
        "a page object whose dictionary does not parse was read as the entries before the damage";
    /// A `/Kids` element that is neither a reference nor a dictionary.
    KIDS_ENTRY_NOT_A_NODE = "model/kids-entry-not-a-node", "ISO 32000-2 §7.7.3.2",
        "a /Kids element that is not a reference to a page tree node or page was skipped";
    /// A `/Kids` element naming the node it is in, or a node above it.
    PAGE_TREE_CYCLE = "model/page-tree-cycle", "ISO 32000-2 §7.7.3.2",
        "a /Kids element naming a node above it in the tree was skipped";
    /// A page tree deeper, or with more nodes, than this reader walks.
    PAGE_TREE_PAST_A_BOUND = "model/page-tree-past-a-bound", "ISO 32000-2 §7.7.3.2",
        "a page tree deeper or wider than this reader walks was read as far as the bound";
    /// A `/Resources` that is not a dictionary, read as the one an ancestor states.
    RESOURCES_NOT_A_DICTIONARY = "model/resources-not-a-dictionary", "ISO 32000-2 §7.7.3.3",
        "a /Resources entry that is not a dictionary was read as the one an ancestor states, or \
         as none";
    /// A page whose whole ancestry states no `/Resources`.
    RESOURCES_ABSENT = "model/resources-absent", "ISO 32000-2 §7.7.3.4",
        "a page whose ancestry states no /Resources, which Table 31 requires, was read with an \
         empty resource dictionary";
    /// A page whose ancestry states no usable `/MediaBox`, drawn on this reader's own box.
    MEDIA_BOX_SUBSTITUTED = "model/media-box-substituted", "ISO 32000-2 §7.7.3.3",
        "a page whose ancestry states no /MediaBox that is a rectangle with an area was read on \
         this reader's own box";
    /// A `/CropBox`, `/BleedBox`, `/TrimBox` or `/ArtBox` that is no rectangle with an area.
    PAGE_BOUNDARY_UNUSABLE = "model/page-boundary-unusable", "ISO 32000-2 §14.11.2.1",
        "a /CropBox, /BleedBox, /TrimBox or /ArtBox that is not a rectangle with an area was \
         read as absent";
    /// A `/Rotate` that is not an integer.
    ROTATE_NOT_AN_INTEGER = "model/rotate-not-an-integer", "ISO 32000-2 §7.7.3.3",
        "a /Rotate that is not an integer was read as the value an ancestor states, or 0";
    /// A `/Rotate` that is an integer and not a multiple of 90.
    ROTATE_NOT_A_MULTIPLE_OF_NINETY = "model/rotate-not-a-multiple-of-ninety",
        "ISO 32000-2 §7.7.3.3",
        "a /Rotate that is not a multiple of 90 was read as the multiple of 90 below it";
    /// A `/UserUnit` that is not a positive number.
    USER_UNIT_UNUSABLE = "model/user-unit-unusable", "ISO 32000-2 §7.7.3.3",
        "a /UserUnit that is not a positive finite number was read as 1";

    // ---- a content stream: its parts, its inline objects, §7.7.3.3, §7.8.2, §8.9.7 ----------

    /// An entry of `/Contents` that is not a stream.
    CONTENTS_ENTRY_NOT_A_STREAM = "model/contents-entry-not-a-stream", "ISO 32000-2 §7.7.3.3",
        "an entry of a page's /Contents that is not a stream was read as no content";
    /// A content stream, a token in it or an object written inside it past this reader's bounds.
    CONTENT_PAST_A_BOUND = "model/content-past-a-bound", "ISO 32000-2 §7.8.2",
        "a content stream, a token or an inline object larger or deeper than this reader holds \
         was read as far as the bound";
    /// An inline dictionary or array holding a token that is no object where one belongs.
    INLINE_OBJECT_MALFORMED = "model/inline-object-malformed", "ISO 32000-2 §7.8.2",
        "a dictionary or array written inside a content stream holding a token that is no \
         object where one belongs was read without it";
    /// An inline image dictionary naming one key twice, abbreviated and in full.
    INLINE_IMAGE_KEY_TWICE = "model/inline-image-key-twice", "ISO 32000-2 §8.9.7",
        "an inline image dictionary stating one key twice was read at one of them";
    /// An inline image `/CS` naming a space the resources do not define.
    INLINE_IMAGE_COLOUR_SPACE_UNRESOLVED = "model/inline-image-colour-space-unresolved",
        "ISO 32000-2 §8.9.7",
        "an inline image /ColorSpace naming no space the resources define was read as a device \
         space";
    /// Inline image data whose extent does not end at `EI`.
    INLINE_IMAGE_DATA_EXTENT = "model/inline-image-data-extent", "ISO 32000-2 §8.9.7",
        "inline image data whose stated or computed length does not end at EI was read to the \
         EI found";

    // ---- the interpreter's operands, §7.8.2, §8.4.2, §8.6.8, §9.4.1 -------------------------

    /// An operator given an operand of a type its table does not state.
    OPERAND_OF_THE_WRONG_TYPE = "model/operand-of-the-wrong-type", "ISO 32000-2 §7.8.2",
        "an operator given an operand of a type its table does not state was not carried out";
    /// An operator given fewer operands than its table states.
    OPERATOR_SHORT_OF_OPERANDS = "model/operator-short-of-operands", "ISO 32000-2 §7.8.2",
        "an operator given fewer operands than its table states was carried out with what was \
         there, or not at all";
    /// A `Q` with no `q` before it in the same stream.
    RESTORE_WITH_NOTHING_SAVED = "model/restore-with-nothing-saved", "ISO 32000-2 §8.4.2",
        "a Q with no q before it in the same content stream was ignored";
    /// A text-showing or text-positioning operator outside `BT` … `ET`.
    TEXT_OPERATOR_OUTSIDE_A_TEXT_OBJECT = "model/text-operator-outside-a-text-object",
        "ISO 32000-2 §9.4.1",
        "a text operator outside BT and ET was carried out as if inside a text object";
    /// Colour operands whose count is not the current colour space's.
    COLOUR_OPERANDS_DO_NOT_FIT_THE_SPACE = "model/colour-operands-do-not-fit-the-space",
        "ISO 32000-2 §8.6.8",
        "a colour operator given a number of operands other than the current colour space's \
         components was read in the device space with that many, or not at all";
}

#[cfg(test)]
mod tests {
    use super::CLASSES;

    /// Every name is this crate's, unique, and every clause is ISO 32000-2's.
    #[test]
    fn every_class_is_named_once_under_this_crate() {
        let mut names = std::collections::BTreeSet::new();
        for class in CLASSES {
            assert!(class.name.starts_with("model/"), "{}", class.name);
            assert!(
                class.clause.starts_with("ISO 32000-2 \u{a7}"),
                "{}",
                class.name
            );
            assert!(names.insert(class.name), "{} twice", class.name);
        }
    }
}
