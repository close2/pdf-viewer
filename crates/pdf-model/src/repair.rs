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
    /// Inline image data whose `/L` is no length, or whose extent does not end at `EI`.
    INLINE_IMAGE_DATA_EXTENT = "model/inline-image-data-extent", "ISO 32000-2 §8.9.7",
        "inline image data whose /L is no length, or whose stated or computed length does not \
         end at EI, was read to the EI found";

    // ---- images and their soft masks, §8.9.5.1 and §11.6.5.2 ---------------------------------

    /// An image whose depth no codec states and whose `/BitsPerComponent` is not an integer.
    IMAGE_DEPTH_ABSENT = "model/image-depth-absent", "ISO 32000-2 §8.9.5.1",
        "an image or soft mask with no /BitsPerComponent that is a non-negative integer, which \
         Table 87 requires where no codec states the depth, was read at 8 bits";
    /// A soft mask whose data does not decode.
    SOFT_MASK_UNDECODABLE = "model/soft-mask-undecodable", "ISO 32000-2 §11.6.5.2",
        "a soft mask whose data did not decode was not applied, and its image was drawn without \
         it";
    /// A JPEG 2000 image's `/SMaskInData` that is no integer.
    SOFT_MASK_IN_DATA_UNUSABLE = "model/soft-mask-in-data-unusable", "ISO 32000-2 §8.9.5.1",
        "a JPEG 2000 image's /SMaskInData that is no integer was read as 0, and any opacity its \
         codestream encodes ignored";
    /// A `/Luminosity` soft mask's `/BC` that is not as many numbers as its group's `/CS` has
    /// components.
    SOFT_MASK_BACKDROP_UNUSABLE = "model/soft-mask-backdrop-unusable", "ISO 32000-2 §11.6.5.1",
        "a luminosity soft mask's /BC that is not n numbers, n the components of its group's /CS, \
         was read as absent, the space's initial colour its backdrop";

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
    /// Operands left in front of the ones an operator takes.
    OPERANDS_LEFT_OVER = "model/operands-left-over", "ISO 32000-2 §7.8.2",
        "operands left over in front of an operator's own were dropped, and the operator given \
         the last of them";

    // ---- the interpreter's sections and resources, §7.8.2, §8.4.5, §8.10.2, §10.5, §14.6 ------

    /// An `EX` with no `BX` open, or a `BX` its stream leaves open.
    COMPATIBILITY_SECTIONS_UNBALANCED = "model/compatibility-sections-unbalanced",
        "ISO 32000-2 §7.8.2",
        "a BX or EX without its pair was read as if the pair were there: an EX ending nothing \
         was ignored, and a section left open ended with its stream";
    /// An `EMC` with no marked-content sequence open, or a sequence its stream leaves open.
    MARKED_CONTENT_UNBALANCED = "model/marked-content-unbalanced", "ISO 32000-2 §14.6.1",
        "a BMC or BDC without its balancing EMC was read as if the EMC were there: an EMC \
         ending nothing was ignored, and a sequence left open ended with its stream";
    /// A `BDC` naming a property list its resources do not hold as a dictionary.
    PROPERTY_LIST_UNRESOLVED = "model/property-list-unresolved", "ISO 32000-2 §14.6.2",
        "a BDC naming a property list the Properties resources do not hold was read as stating \
         no properties, and an optional-content sequence as shown";
    /// A graphics state parameter dictionary's entry of a type Table 57 does not give it.
    GRAPHICS_STATE_ENTRY_OF_THE_WRONG_TYPE = "model/graphics-state-entry-of-the-wrong-type",
        "ISO 32000-2 §8.4.5",
        "a graphics state parameter dictionary entry of a type Table 57 does not give it was \
         ignored, and the parameter left as it was";
    /// A `/TR` or `/TR2` that is no function, array of four functions or name Table 57 gives.
    TRANSFER_FUNCTION_UNREADABLE = "model/transfer-function-unreadable", "ISO 32000-2 §10.5",
        "a transfer function that could not be read was ignored, and the transfer in force left \
         as it was";
    /// A form's `/Matrix` that is not an array of six numbers.
    FORM_MATRIX_MALFORMED = "model/form-matrix-malformed", "ISO 32000-2 §8.10.2",
        "a form's /Matrix that is not six numbers was read as the identity, or from the first \
         six numbers it holds";
    /// A form's `/BBox` absent, or not an array of four numbers.
    FORM_BBOX_UNUSABLE = "model/form-bbox-unusable", "ISO 32000-2 §8.10.2",
        "a form with no /BBox of four numbers was drawn unclipped, clipped to the first four \
         numbers its /BBox holds, or, as an annotation's appearance, given the annotation \
         rectangle's dimensions at the origin";
    /// A catalog's `/PageMode`, `/PageLayout` or `/ViewerPreferences` of a type or name Table 29
    /// does not give it.
    CATALOG_DISPLAY_ENTRY_UNUSABLE = "model/catalog-display-entry-unusable", "ISO 32000-2 §7.7.2",
        "a catalog's /PageMode, /PageLayout or /ViewerPreferences of a type or a name Table 29 does \
         not give it was read as absent, and the entry's default used";
    /// A viewer preference of a type or name Table 147 does not give it.
    VIEWER_PREFERENCE_UNUSABLE = "model/viewer-preference-unusable", "ISO 32000-2 §12.2",
        "a viewer preference of a type or a name Table 147 does not give it was read as absent, \
         and the entry's default used";
    /// An action's entry of a type or value its table does not give it.
    ACTION_ENTRY_UNUSABLE = "model/action-entry-unusable", "ISO 32000-2 §12.6.4",
        "an action entry of a type or value its table does not give it was read as absent: a \
         boolean, flag word or number as its default, a number past its range as the nearer end, \
         a negative index as naming no target";
    /// A page's `/Annots` that is not an array.
    PAGE_ANNOTS_NOT_AN_ARRAY = "model/page-annots-not-an-array", "ISO 32000-2 §7.7.3.3",
        "a page whose /Annots is neither absent nor an array, which Table 31 requires, was read \
         as listing no annotation";
    /// An annotation's entry of a type or value its table does not give it.
    ANNOTATION_ENTRY_UNUSABLE = "model/annotation-entry-unusable", "ISO 32000-2 §12.5.2",
        "an annotation entry of a type or value its table does not give it was read as absent, \
         or a number outside its range as the nearer end of it";
    /// An interactive form dictionary, or a field's entry, of a type or value its table does not
    /// give it.
    FIELD_ENTRY_UNUSABLE = "model/field-entry-unusable", "ISO 32000-2 §12.7",
        "an /AcroForm, or a field entry, of a type or value its table does not give it was read \
         as absent, and an index naming no option dropped";
    /// An optional content group's `/Name` absent or not a text string.
    OPTIONAL_CONTENT_GROUP_NAME_UNUSABLE = "model/optional-content-group-name-unusable",
        "ISO 32000-2 §8.11.2.1",
        "an optional content group whose required /Name is absent or not a text string was named \
         by the empty string";
    /// A file specification naming no file, or an embedded file stream's `/Params` that is no
    /// dictionary.
    FILE_SPECIFICATION_ENTRY_UNUSABLE = "model/file-specification-entry-unusable",
        "ISO 32000-2 §7.11.3",
        "a file specification in which no /UF, /F, /DOS, /Mac or /Unix is a string, where Table \
         43 requires /F, was shown with the empty name, and an embedded file stream's /Params \
         that is no dictionary was read as absent";
    /// An optional content properties dictionary's `/D`, or a group's `/Usage` or one of its
    /// dictionaries, absent where required or not a dictionary.
    OPTIONAL_CONTENT_ENTRY_UNUSABLE = "model/optional-content-entry-unusable",
        "ISO 32000-2 §8.11.4",
        "an /OCProperties whose required /D is absent or no dictionary was read with every group \
         on, and a group's /Usage, or an entry of it, that is no dictionary was read as absent";
    /// A form's `/Resources` present and not a dictionary.
    FORM_RESOURCES_NOT_A_DICTIONARY = "model/form-resources-not-a-dictionary",
        "ISO 32000-2 §8.10.2",
        "a form's /Resources that is not a dictionary was read as the dictionary of a stream, or \
         else as absent, the form's names then looked up in the page's resources";
    /// A form's, an appearance's or a page's `/Group` that is no dictionary, or a group attributes
    /// dictionary whose required `/S` is absent or no name.
    GROUP_ATTRIBUTES_UNUSABLE = "model/group-attributes-unusable", "ISO 32000-2 §8.10.3",
        "a /Group that is not a dictionary, or a group attributes dictionary whose required /S is \
         absent or not a name, was read as stating no group";
    /// A tiling pattern's required `/BBox` or `/Resources` absent or of a shape Table 74 does not
    /// give it.
    TILING_PATTERN_ENTRY_UNUSABLE = "model/tiling-pattern-entry-unusable", "ISO 32000-2 §8.7.3.1",
        "a tiling pattern whose /BBox is not four finite numbers was tiled from a cell of its step's \
         size at the origin, unclipped, and one whose /Resources is absent or no dictionary was \
         read as the dictionary of a stream, or else as defining nothing";
    /// A graphics state's `/CA` or `/ca` outside the range 0.0 to 1.0.
    ALPHA_CONSTANT_OUT_OF_RANGE = "model/alpha-constant-out-of-range", "ISO 32000-2 §11.3.7.2",
        "an alpha constant outside the range 0.0 to 1.0 was read as the nearer end of the range";
    /// A resource category of a stream's `/Resources` that is not a dictionary.
    RESOURCE_CATEGORY_NOT_A_DICTIONARY = "model/resource-category-not-a-dictionary",
        "ISO 32000-2 §7.8.3",
        "a resource category (/Font, /XObject and the rest) that is not the dictionary Table 34 \
         gives it was read as an empty one";
    /// A `/BM` naming a blend mode neither Table 134 nor Table 135 lists.
    BLEND_MODE_UNKNOWN = "model/blend-mode-unknown", "ISO 32000-2 §11.6.3",
        "a /BM name that neither Table 134 nor Table 135 lists was read as Normal";
    /// A halftone's `/HT` of a type Table 57 does not give it, or naming a halftone other than
    /// `/Default`.
    HALFTONE_UNREADABLE = "model/halftone-unreadable", "ISO 32000-2 §8.4.5",
        "an /HT that is no halftone dictionary, stream or the name Default was ignored, and the \
         halftone's transfer function left as it was";
    /// A halftone's `/TransferFunction` that is no function and not the name `/Identity`.
    HALFTONE_TRANSFER_FUNCTION_UNREADABLE = "model/halftone-transfer-function-unreadable",
        "ISO 32000-2 §10.6.5.2",
        "a halftone's /TransferFunction that could not be read was ignored, and the transfer \
         function of the graphics state used for its component";

    // ---- rich text, §12.7.4.3 and the XFA 3.3 grammar it names ---------------------------------

    /// A field's `/RV` or an annotation's `/RC` that is not well-formed XML.
    RICH_TEXT_MALFORMED = "model/rich-text-malformed", "ISO 32000-2 §12.7.4.3",
        "a rich text string (/RV or /RC) that is not well-formed XML was not read as rich text, \
         and the field's or the annotation's plain text was drawn in its place, or its characters \
         were shown as far as they read; an entity reference in it that XML 1.0 does not define \
         was kept as the text it is written as";
    /// An attribute of a rich text string's markup that does not read as its grammar's value.
    RICH_TEXT_ATTRIBUTE_UNUSABLE = "model/rich-text-attribute-unusable", "ISO 32000-2 §12.7.4.3",
        "an ol start, an li value or a body's xfa:spec in a rich text string that does not read \
         as the number or version its grammar gives it was read as absent";

    // ---- an XFA template, §12.7.3's `/XFA` and the XFA 3.3 grammar it names ------------------

    /// An attribute of an XFA template's `occur` or of a value's content element that does not
    /// read as the number its grammar gives it.
    XFA_TEMPLATE_ATTRIBUTE_UNUSABLE = "model/xfa-template-attribute-unusable",
        "ISO 32000-2 §12.7.3",
        "an occur min or max, or a fracDigits or leadDigits, in an XFA template that does not read \
         as the integer its grammar gives it was read as absent, the grammar's default standing \
         in its place";
    /// A `locale` of an XFA `localeSet` packet stating no `name`.
    XFA_LOCALE_NAME_ABSENT = "model/xfa-locale-name-absent", "ISO 32000-2 §12.7.3",
        "a locale of an XFA localeSet packet with no name, which XFA 3.3 says must be supplied, was \
         not kept, so no picture clause reads its symbols or patterns";
    /// An XFA packet's entity or character reference that XML does not define.
    XFA_REFERENCE_UNDEFINED = "model/xfa-reference-undefined", "ISO 32000-2 §12.7.3",
        "an entity reference in an XFA packet that XML 1.0 does not define, or a character \
         reference naming no character, was kept as the text it is written as";

    // ---- portable collections, §12.3.5 ----------------------------------------------------------

    /// A collection split's `/Position` or a folder's `/ID` of a type or value its table does not
    /// give it.
    COLLECTION_ENTRY_UNUSABLE = "model/collection-entry-unusable", "ISO 32000-2 §12.3.5",
        "a collection split's /Position outside 0 to 100 was read as the nearer end, or of another \
         type as absent, and a folder's required /ID that is no non-negative integer as 0";
    /// An outline item's `/F` or `/C` of a type or value Table 151 does not give it.
    OUTLINE_ENTRY_UNUSABLE = "model/outline-entry-unusable", "ISO 32000-2 §12.3.3",
        "an outline item's /F that is no integer was read as 0, and a /C that is not three numbers \
         from 0.0 to 1.0 as black where it is no array, each other component as 0 and each number \
         outside the range as the nearer end";
    /// A page label dictionary's `/St` that is no integer of at least 1.
    PAGE_LABEL_ENTRY_UNUSABLE = "model/page-label-entry-unusable", "ISO 32000-2 §12.4.2",
        "a page label range's /St that is no integer of at least 1 was read as 1, its default";

    // ---- document requirements, §12.11, and media criteria, §13.2.3 ----------------------------

    /// A requirement dictionary's `/Penalty` of a type or value Table 273 does not give it.
    REQUIREMENT_ENTRY_UNUSABLE = "model/requirement-entry-unusable", "ISO 32000-2 §12.11.1",
        "a requirement's /Penalty outside 0 to 100 was read as the nearer end, and one that is no \
         integer as its default 100";
    /// A media criteria dictionary's `/L` that is not an array of language identifiers.
    MEDIA_CRITERION_UNUSABLE = "model/media-criterion-unusable", "ISO 32000-2 §13.2.3",
        "a media criteria dictionary's /L that is no array, or an element of it that is no text \
         string, was read as absent, so it decided nothing about the object's viability";

    // ---- logical structure and page boundaries, §14.8.5 and §14.11.2.2 -------------------------

    /// A structure element's span or writing mode of a type or value its table does not give it.
    STRUCTURE_ATTRIBUTE_UNUSABLE = "model/structure-attribute-unusable", "ISO 32000-2 §14.8.5",
        "a table cell's /RowSpan or /ColSpan that is no integer of at least 1 was read as 1, a \
         /WritingMode that is none of Table 378's names as LrTb, and a /TextDecorationType, \
         /TextPosition or PrintField /Checked that is none of its table's names as its default";
    /// A page's `/BoxColorInfo`, or one of its box style dictionaries' entries, of a type or value
    /// Table 396 or Table 397 does not give it.
    BOX_COLOUR_INFO_UNUSABLE = "model/box-colour-info-unusable", "ISO 32000-2 §14.11.2.2",
        "a page's /BoxColorInfo, or a box style's /C, /W, /S or /D, of a type or value Tables 396 \
         and 397 do not give it was read as absent, and the guideline drawn in its default";

    // ---- metadata, §14.3.2 -------------------------------------------------------------------

    /// An XMP packet's entity or character reference that XML does not define.
    XMP_REFERENCE_UNDEFINED = "model/xmp-reference-undefined", "ISO 32000-2 §14.3.2",
        "an entity reference in a metadata stream's XMP packet that XML 1.0 does not define, or a \
         character reference naming no character, was kept as the text it is written as";

    // ---- measurement, §12.9 and §12.10.5 -------------------------------------------------------

    /// A number format's or a point data dictionary's entry of a type or value its table does not
    /// give it.
    MEASUREMENT_ENTRY_UNUSABLE = "model/measurement-entry-unusable", "ISO 32000-2 §12.9",
        "a number format's /D that is no positive integer was read at its default, and a point \
         data dictionary's required /Names or /XPTS that is no array was read as an empty one";

    // ---- a coordinate system's Well Known Text, §12.10.3 and ISO 19162 ------------------------

    /// A system, a datum or a unit of a `/WKT` string with no name.
    WKT_NAME_ABSENT = "model/wkt-name-absent", "ISO 32000-2 §12.10.3",
        "a coordinate system, datum or unit that the /WKT string's grammar (ISO 19162) gives a \
         name and the string states without one was named by the empty string";
    /// An `ID` or `AUTHORITY` of a `/WKT` string whose EPSG code is text that is not a number.
    WKT_IDENTIFIER_UNUSABLE = "model/wkt-identifier-unusable", "ISO 32000-2 §12.10.3",
        "an EPSG identifier in a /WKT string whose code is not a whole number was read as no \
         identifier, the component then known by its name alone";
    /// A `Mercator_Auxiliary_Sphere` projection of a `/WKT` string with no
    /// `Auxiliary_Sphere_Type`.
    WKT_AUXILIARY_SPHERE_TYPE_ABSENT = "model/wkt-auxiliary-sphere-type-absent",
        "ISO 32000-2 §12.10.4",
        "a Mercator_Auxiliary_Sphere projection stating no Auxiliary_Sphere_Type was read as \
         type 0, the sphere of the ellipsoid's semi-major axis";
    /// A projection of a `/WKT` string stating no false easting, or no false northing.
    WKT_FALSE_ORIGIN_ABSENT = "model/wkt-false-origin-absent", "ISO 32000-2 §12.10.4",
        "a projection whose /WKT string states no false easting or no false northing, each among \
         the parameters IOGP Guidance Note 7-2 requires to define its method, was evaluated with \
         zero in its place";
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
