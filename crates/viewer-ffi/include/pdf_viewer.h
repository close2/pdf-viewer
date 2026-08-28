/*
 * pdf_viewer.h — the C ABI over viewer-core's vocabulary.
 *
 * `viewer-ffi`, ADR 0247. The argument for every shape here is in `src/lib.rs`; what follows is
 * the declaration and the one-line reason where a reason is not obvious.
 *
 * HAND-WRITTEN, AND CHECKED. No `cbindgen`: a generated header is a derivative of Rust types that
 * a C programmer then has to read anyway, and this one is the artefact rather than a by-product.
 * What a generator buys is that it cannot drift, and that is bought back by
 * `tests/header_and_library_agree.rs`, which reads this file and `src/abi.rs` and asserts that
 * every entry point is declared exactly once in each and that every PDFV_ constant is the number
 * the Rust enumeration gives it.
 *
 * THREADS. No handle may be used from two threads at once. A `pdfv_render_request *` may be MOVED
 * to another thread and rasterised there — which is what the render round trip is for — and a
 * `pdfv_viewer *` may not be shared.
 *
 * MEMORY. Every handle this library returns is owned by the caller and is released with its own
 * `_free`, except where a function is documented as consuming it. Nothing hands out a pointer into
 * the viewer's own memory: pixels are copied into a buffer the caller owns.
 */

#ifndef PDF_VIEWER_H
#define PDF_VIEWER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ------------------------------------------------------------------------------------------- */
/* The identity of this ABI.                                                                     */
/* ------------------------------------------------------------------------------------------- */

/*
 * The revision of everything below that is passed BY VALUE: `pdfv_geometry`, `pdfv_frame` and
 * `pdfv_viewing`, and nothing else. A function added later is a symbol an old caller never looks
 * up; a status or an event kind added later is a number an old caller has a `default:` arm for;
 * a whole struct added later is a shape an old caller never passes. A FIELD added to one of those
 * structs is a size an old caller has already compiled, and no diagnostic anywhere would catch
 * it — so this number moves when one of them does.
 */
#define PDFV_ABI_VERSION 1u

/*
 * How many event kinds the header below declares. Pass it to `pdfv_abi_check` in main().
 *
 * This is what stands in for the Rust rule that a new message fails to compile in every consumer.
 * It cannot fail a build, so it fails a startup instead, once, naming the number that moved.
 */
#define PDFV_EVENT_KIND_COUNT 16u

/* What an entry point returns. `PDFV_OK` is zero; everything else is a refusal. */
#define PDFV_OK                 0
#define PDFV_NULL_ARGUMENT      1
#define PDFV_OUT_OF_RANGE       2
#define PDFV_WRONG_KIND         3
#define PDFV_BUFFER_TOO_SMALL   4
#define PDFV_NO_ANSWER          5
#define PDFV_NOT_UTF8           6
#define PDFV_RENDER_REFUSED     7
#define PDFV_NUMBER_OUT_OF_RANGE 8

/* ISO 32000-2 events, as `viewer_core::Event` names them. */
#define PDFV_EVENT_OPENED             0u
#define PDFV_EVENT_OPEN_FAILED        1u
#define PDFV_EVENT_PASSWORD_REQUIRED  2u
#define PDFV_EVENT_CLOSED             3u
#define PDFV_EVENT_PAGE_CHANGED       4u
#define PDFV_EVENT_NEEDS_RENDER       5u
#define PDFV_EVENT_DAMAGE             6u
#define PDFV_EVENT_OPEN_URI           7u
#define PDFV_EVENT_NEEDS_FILE         8u
#define PDFV_EVENT_TRANSITION         9u
#define PDFV_EVENT_DIRTY             10u
#define PDFV_EVENT_SAVED             11u
#define PDFV_EVENT_EXTRACTED         12u
#define PDFV_EVENT_REFUSED           13u
#define PDFV_EVENT_REPORTED          14u
/* PDF 2.0 Annex O's `search`, and the kind that moved the count from 15 to 16. It had no name
 * here at all until the five-hundred-and-eleventh session — the count moved and the constant did
 * not, so a caller switching on kinds had to write the number 15 by hand. */
#define PDFV_EVENT_SEARCHED          15u

/* §12.5.5's three situations, of which a press is two. What `pdfv_pointer` takes. */
#define PDFV_POINTER_MOVED     0u
#define PDFV_POINTER_PRESSED   1u
#define PDFV_POINTER_DRAGGED   2u
#define PDFV_POINTER_RELEASED  3u

/* What `pdfv_select` asks for. A drag is `pdfv_pointer`'s business. */
#define PDFV_SELECT_ALL   0u
#define PDFV_SELECT_NONE  1u

/* Which way §12.5.1's tab key moves the focus. */
#define PDFV_FOCUS_NEXT      0u
#define PDFV_FOCUS_PREVIOUS  1u
#define PDFV_FOCUS_NONE      2u

/* How much of what a document asserts about its reader this viewer obeys. ON is the default. */
#define PDFV_RESTRICT_ON   0u
#define PDFV_RESTRICT_OFF  1u

/* Table 29's /PageLayout: how the pages are arranged. SINGLE_PAGE is the table's default, and
 * the document's own value is what a session opens in — this is how a reader changes it. */
#define PDFV_LAYOUT_SINGLE_PAGE      0u
#define PDFV_LAYOUT_ONE_COLUMN       1u
#define PDFV_LAYOUT_TWO_COLUMN_LEFT  2u
#define PDFV_LAYOUT_TWO_COLUMN_RIGHT 3u
#define PDFV_LAYOUT_TWO_PAGE_LEFT    4u
#define PDFV_LAYOUT_TWO_PAGE_RIGHT   5u

/* §14.8.2.5's two content orders, which is what pdfv_selection_copy_text says about its text.
 * There is no third and no _COUNT: the clause defines exactly these two. LOGICAL is "a
 * depth-first traversal of the document's logical structure"; PAGE_CONTENT is "the sequencing of
 * graphics objects within a page's content stream", which is also what a selection is measured
 * in and what pdfv_selection_quads covers. */
#define PDFV_ORDER_LOGICAL       0u
#define PDFV_ORDER_PAGE_CONTENT  1u

/* Whether §12.4.4's presentation is running. OFF is the default. */
#define PDFV_PRESENT_OFF  0u
#define PDFV_PRESENT_ON   1u

/* §6.3.2.2's "unless otherwise instructed": who draws §12.7's widget appearances. */
#define PDFV_DELEGATE_DRAWN      0u
#define PDFV_DELEGATE_DELEGATED  1u

/* Which of §12.5.6.10's four text markups `pdfv_markup` adds over what is selected. */
#define PDFV_MARKUP_HIGHLIGHT   0u
#define PDFV_MARKUP_UNDERLINE   1u
#define PDFV_MARKUP_STRIKE_OUT  2u
#define PDFV_MARKUP_SQUIGGLY    3u

/* What a file the document asks for is wanted for. */
#define PDFV_PURPOSE_IMPORT_DATA  0u

/*
 * Which platform control a §12.7 field is — `viewer_host::ControlKind`, which is one variant per
 * control a toolkit has for the job rather than one per §12.7.5 type. `pdfv_control_kind_name`
 * answers for any number, "unknown" for one this build does not define.
 *
 * NOT part of `pdfv_abi_check`, deliberately: an event arrives whether or not the caller asked, so
 * its count has to be right before the first one turns up; a control kind is the answer to a call
 * the caller wrote. `pdfv_control_kind_count()` is there for a caller that wants to check anyway.
 */
#define PDFV_CONTROL_ENTRY      0u
#define PDFV_CONTROL_CHECK      1u
#define PDFV_CONTROL_RADIO      2u
#define PDFV_CONTROL_PUSH       3u
#define PDFV_CONTROL_COMBO      4u
#define PDFV_CONTROL_LIST       5u
#define PDFV_CONTROL_SIGNATURE  6u
#define PDFV_CONTROL_UNSTATED   7u
#define PDFV_CONTROL_KIND_COUNT 8u

/* What acting on a panel row does. `pdfv_row_kind_name` answers for any number. */
#define PDFV_ROW_ACTIVATE  0u
#define PDFV_ROW_TOGGLE    1u
#define PDFV_ROW_EXTRACT   2u
#define PDFV_ROW_INERT     3u
#define PDFV_ROW_KIND_COUNT 4u

/*
 * Which string a `which` argument asks for. One argument rather than a function per string,
 * because they are the same two-call idiom over the same handle and index.
 *
 * QUALIFIED, SHOWN and PARTIAL name a field; LABEL and EXPORT name an option or a widget's state.
 * Asking for one of a kind the accessor does not carry answers PDFV_WRONG_KIND.
 */
#define PDFV_TEXT_QUALIFIED  0u
#define PDFV_TEXT_SHOWN      1u
#define PDFV_TEXT_PARTIAL    2u
#define PDFV_TEXT_LABEL      3u
#define PDFV_TEXT_EXPORT     4u

/* --------------------------------------------------------------------------------------------
 * The numbers the OTHER HALF OF THE QUERIES answers with (ADR 0576).
 *
 * Eleven of `viewer_core::Query`'s variants reached no symbol here at all until the
 * seven-hundred-and-ninth session, and nothing counted them: `PDFV_EVENT_KIND_COUNT` is the right
 * instrument for a message that ARRIVES and no instrument at all for a QUESTION. What replaces it
 * is a test on the library side that matches exhaustively over `Query`, so a question added to the
 * boundary fails to compile there rather than leaving a caller with no symbol and no signal.
 * -------------------------------------------------------------------------------------------- */

/* Table 29's /PageMode: "how the document shall be displayed when opened". `pdfv_opening` answers
 * this beside a PDFV_LAYOUT_*, because §7.7.2 states the two entries separately and a window obeys
 * them separately. COUNTED, unlike the two-valued enumerations below: the table gained /UseOC in
 * PDF 1.5 and /UseAttachments in PDF 1.6. `pdfv_page_mode_name` answers for any number. */
#define PDFV_PAGE_MODE_USE_NONE         0u
#define PDFV_PAGE_MODE_USE_OUTLINES     1u
#define PDFV_PAGE_MODE_USE_THUMBS       2u
#define PDFV_PAGE_MODE_FULL_SCREEN      3u
#define PDFV_PAGE_MODE_USE_OC           4u
#define PDFV_PAGE_MODE_USE_ATTACHMENTS  5u
#define PDFV_PAGE_MODE_COUNT            6u

/*
 * §12.2's Table 147, as keys rather than as a struct or as nineteen symbols.
 *
 * A struct passed by value would put Table 147's SIZE in this ABI, so an entry added by a later
 * part of ISO 32000 would change a type every caller has already compiled — the one hazard
 * PDFV_ABI_VERSION exists for, and this header has exactly two instances of it. A key is a number:
 * an entry added later is a new constant beside `pdfv_preference`, which every compiled caller
 * already links, and `pdfv_preference_key_name` prints a key this build did not have.
 *
 * Every value answers as an int64_t: a boolean is 0 or 1, an enumerated name is its own PDFV_*
 * number, and a count is itself. PDFV_NO_ANSWER is the three entries Table 147 leaves genuinely
 * open — /Duplex, /PickTrayByPDFSize and /NumCopies — where the document states none, because "the
 * document says nothing" and "the document says the default" are different facts and only the
 * first leaves the choice to you. PDFV_PREF_PRINT_PAGE_RANGE is a LIST and answers
 * PDFV_WRONG_KIND: `pdfv_preference_ranges` and `pdfv_preference_range` read it.
 */
#define PDFV_PREF_HIDE_TOOLBAR               0u  /* boolean */
#define PDFV_PREF_HIDE_MENUBAR               1u  /* boolean */
#define PDFV_PREF_HIDE_WINDOW_UI             2u  /* boolean */
#define PDFV_PREF_FIT_WINDOW                 3u  /* boolean */
#define PDFV_PREF_CENTER_WINDOW              4u  /* boolean */
#define PDFV_PREF_DISPLAY_DOC_TITLE          5u  /* boolean */
#define PDFV_PREF_NON_FULL_SCREEN_PAGE_MODE  6u  /* PDFV_PAGE_MODE_* */
#define PDFV_PREF_DIRECTION                  7u  /* PDFV_DIRECTION_* */
#define PDFV_PREF_VIEW_AREA                  8u  /* PDFV_BOUNDARY_* */
#define PDFV_PREF_VIEW_CLIP                  9u  /* PDFV_BOUNDARY_* */
#define PDFV_PREF_PRINT_AREA                10u  /* PDFV_BOUNDARY_* */
#define PDFV_PREF_PRINT_CLIP                11u  /* PDFV_BOUNDARY_* */
#define PDFV_PREF_PRINT_SCALING             12u  /* PDFV_PRINT_SCALING_* */
#define PDFV_PREF_DUPLEX                    13u  /* PDFV_DUPLEX_*, optional */
#define PDFV_PREF_PICK_TRAY_BY_PDF_SIZE     14u  /* boolean, optional */
#define PDFV_PREF_NUM_COPIES                15u  /* a count, optional */
#define PDFV_PREF_ENFORCE_PRINT_SCALING     16u  /* boolean */
#define PDFV_PREF_PRINT_PAGE_RANGE          17u  /* a list — see pdfv_preference_ranges */
#define PDFV_PREF_KEY_COUNT                 18u

/* Table 147's /Direction, "[t]he predominant reading order for text". Two values and no _COUNT,
 * for PDFV_ORDER_*'s reason: the entry states exactly these two names. */
#define PDFV_DIRECTION_L2R  0u
#define PDFV_DIRECTION_R2L  1u

/* Table 147's four page-boundary entries, as one of Table 31's five boxes. /CropBox is the
 * default of all four. */
#define PDFV_BOUNDARY_MEDIA  0u
#define PDFV_BOUNDARY_CROP   1u
#define PDFV_BOUNDARY_BLEED  2u
#define PDFV_BOUNDARY_TRIM   3u
#define PDFV_BOUNDARY_ART    4u

/* Table 147's /PrintScaling and /Duplex. */
#define PDFV_PRINT_SCALING_APP_DEFAULT  0u
#define PDFV_PRINT_SCALING_NONE         1u
#define PDFV_DUPLEX_SIMPLEX             0u
#define PDFV_DUPLEX_FLIP_SHORT_EDGE     1u
#define PDFV_DUPLEX_FLIP_LONG_EDGE      2u

/*
 * §9.10.2's counts, which `pdfv_readback_count` answers and which are DELIBERATELY NOT REPORTS.
 *
 * That clause's own closing sentence is "there is no way to determine what the character code
 * represents", so a code this route ends at is an answer the standard states rather than something
 * this program failed to do — and folding them into `pdfv_report` would say the opposite. What
 * they are for is the thing a person needs: saying that a search found nothing on a page whose
 * text cannot be read, or that a copied selection is short.
 */
#define PDFV_SHORTFALL_EMPTY_MAPPING         0u
#define PDFV_SHORTFALL_INCOMPLETE_TO_UNICODE 1u
#define PDFV_SHORTFALL_UNLISTED_NAME         2u
#define PDFV_SHORTFALL_UNNAMED_CID           3u
#define PDFV_SHORTFALL_UNADDRESSABLE_CID     4u
#define PDFV_SHORTFALL_UNNAMED_GLYPH         5u
#define PDFV_SHORTFALL_UNNAMED_TOTAL         6u  /* the six above, which is what a status bar shows */
#define PDFV_SHORTFALL_WITHOUT_A_GLYPH       7u
#define PDFV_SHORTFALL_BLANK_GLYPH           8u
#define PDFV_SHORTFALL_KIND_COUNT            9u

/* §12.3.4's two producer-side constraints, as bits `pdfv_thumbnail_info` sets when the FILE breaks
 * one. Carried rather than enforced: such a file is wrong and its picture is still what the file
 * says, so the image is decoded either way. Zero is a conformant thumbnail. */
#define PDFV_THUMBNAIL_COLOUR_SPACE_UNPERMITTED  1u
#define PDFV_THUMBNAIL_SUBTYPE_UNPERMITTED       2u

/* Which of a §12.5.6.14 popup window's three strings `pdfv_popup_text` answers. Table 166 makes
 * none of them required, and one the annotation does not state is an empty string. */
#define PDFV_NOTE_TITLE     0u  /* §12.5.6.2's /T, "displayed in the title bar" */
#define PDFV_NOTE_CONTENTS  1u  /* Table 166's /Contents */
#define PDFV_NOTE_MODIFIED  2u  /* Table 166's /M, as the file spells it */

/* Which of a §14.7 structure element's three strings `pdfv_structure_text` answers. ROLE is
 * §14.7.4's /S AFTER §14.7.3's role map, which is a `shall` on us; mapping it onto YOUR platform's
 * vocabulary is a different mapping and is yours. */
#define PDFV_ELEMENT_ROLE      0u
#define PDFV_ELEMENT_NAME      1u
#define PDFV_ELEMENT_LANGUAGE  2u

/* Which of an element's two rectangles `pdfv_structure_box` answers. STATED is what the DOCUMENT
 * says the extent is; DRAWN is where this program drew the element's text. An element whose
 * content is a picture has the first and not the second. */
#define PDFV_BOX_STATED  0u
#define PDFV_BOX_DRAWN   1u

/* Table 384's /Scope: which of a table's axes a header cell describes. */
#define PDFV_SCOPE_ROW     0u
#define PDFV_SCOPE_COLUMN  1u
#define PDFV_SCOPE_BOTH    2u

/* Table 153's /View: how §12.3.5's collection is first presented. HIDDEN is the one value that is
 * load-bearing rather than a preference — §7.6.7's unencrypted wrapper requires it, because the
 * wrapper's own page says the payload is encrypted and a file browser over it would hide that. */
#define PDFV_COLLECTION_DETAILS    0u
#define PDFV_COLLECTION_TILE       1u
#define PDFV_COLLECTION_HIDDEN     2u
#define PDFV_COLLECTION_NAVIGATOR  3u

/* §12.3.5.1's four outcomes for /D, RESOLVED against the /EmbeddedFiles tree — the tree is the
 * document's, so turning a byte string into one of these is not yours to do. EMBEDDED is the only
 * one that names a file, and `pdfv_collection_initial` answers its key beside the number. */
#define PDFV_INITIAL_CONTAINER   0u
#define PDFV_INITIAL_EMBEDDED    1u
#define PDFV_INITIAL_FIRST_FILE  2u
#define PDFV_INITIAL_EMPTY       3u

/* Table 155's /Subtype. The first three "identify the types of fields in the collection item …
 * dictionary" and the rest "identify the types of file-related fields" — which is what
 * `pdfv_collection_column`'s `in_the_item` says, so you do not have to derive it from the number.
 * OTHER is a subtype this standard does not define, and PDFV_COLUMN_SUBTYPE is its name. */
#define PDFV_COLLECTION_FIELD_TEXT             0u
#define PDFV_COLLECTION_FIELD_DATE             1u
#define PDFV_COLLECTION_FIELD_NUMBER           2u
#define PDFV_COLLECTION_FIELD_FILE_NAME        3u
#define PDFV_COLLECTION_FIELD_DESCRIPTION      4u
#define PDFV_COLLECTION_FIELD_MODIFICATION_DATE 5u
#define PDFV_COLLECTION_FIELD_CREATION_DATE    6u
#define PDFV_COLLECTION_FIELD_SIZE             7u
#define PDFV_COLLECTION_FIELD_COMPRESSED_SIZE  8u
#define PDFV_COLLECTION_FIELD_OTHER            9u

/* Which of a collection column's three strings `pdfv_collection_column_text` answers, and the two
 * booleans `pdfv_collection_column` packs into its flag word. */
#define PDFV_COLUMN_NAME      0u  /* Table 155's /N, shown to a person */
#define PDFV_COLUMN_KEY       1u  /* the /Schema key §7.11.6's item addresses values by */
#define PDFV_COLUMN_SUBTYPE   2u  /* Table 155's /Subtype as the file spells it */
#define PDFV_COLUMN_VISIBLE   1u  /* Table 155's /V, "[t]he initial visibility of the field" */
#define PDFV_COLUMN_EDITABLE  2u  /* Table 155's /E */

/* Which of a §12.3.5.2 folder's two strings `pdfv_collection_folder_text` answers. */
#define PDFV_FOLDER_NAME         0u
#define PDFV_FOLDER_DESCRIPTION  1u

/*
 * `pdfv_field_control`'s flags: one bit per boolean Tables 227, 229, 231 and 233 state.
 *
 * A word rather than a field apiece, because sixteen accessors would be sixteen symbols saying one
 * thing and a struct passed by value is the one change this ABI cannot make cheaply. A bit added
 * later is a bit an old caller does not read, which costs it nothing.
 */
#define PDFV_FIELD_READ_ONLY            1u  /* Table 227 bit 1 */
#define PDFV_FIELD_REQUIRED             2u  /* Table 227 bit 2 */
#define PDFV_FIELD_NO_EXPORT            4u  /* Table 227 bit 3 */
#define PDFV_FIELD_MULTILINE            8u  /* Table 231 bit 13 */
#define PDFV_FIELD_PASSWORD            16u  /* Table 231 bit 14 */
#define PDFV_FIELD_FILE_SELECT         32u  /* Table 231 bit 21 */
#define PDFV_FIELD_DO_NOT_SPELL_CHECK  64u  /* Table 231 bit 23, Table 233 bit 23 */
#define PDFV_FIELD_DO_NOT_SCROLL      128u  /* Table 231 bit 24 */
#define PDFV_FIELD_COMB               256u  /* Table 231 bit 25 */
#define PDFV_FIELD_RICH_TEXT          512u  /* Table 231 bit 26 */
#define PDFV_FIELD_NO_TOGGLE_TO_OFF  1024u  /* Table 229 bit 15 */
#define PDFV_FIELD_RADIOS_IN_UNISON  2048u  /* Table 229 bit 26 */
#define PDFV_FIELD_ON                4096u  /* §12.7.5.2: the field is in its on state */
#define PDFV_FIELD_EDITABLE          8192u  /* Table 233 bit 19 */
#define PDFV_FIELD_MULTI_SELECT     16384u  /* Table 233 bit 22 */
#define PDFV_FIELD_COMMIT_ON_SEL    32768u  /* Table 233 bit 27 */
#define PDFV_FIELD_OBSCURED         65536u  /* the value is bit 14's echo, not the characters */

/* Table 164's `/Dm` and `/M`, which `pdfv_event_transition` answers with. */
#define PDFV_DIMENSION_HORIZONTAL  0u
#define PDFV_DIMENSION_VERTICAL    1u
#define PDFV_MOTION_INWARD         0u
#define PDFV_MOTION_OUTWARD        1u

/* Which page `pdfv_go_to_page` means. INDEX and RELATIVE read the argument; the rest ignore it. */
#define PDFV_PAGE_INDEX     0u
#define PDFV_PAGE_FIRST     1u
#define PDFV_PAGE_LAST      2u
#define PDFV_PAGE_NEXT      3u
#define PDFV_PAGE_PREVIOUS  4u
#define PDFV_PAGE_RELATIVE  5u

/* How large the page is drawn. SCALE reads the argument; the rest ignore it. */
#define PDFV_ZOOM_FIT_PAGE    0u
#define PDFV_ZOOM_FIT_WIDTH   1u
#define PDFV_ZOOM_FIT_HEIGHT  2u
#define PDFV_ZOOM_SCALE       3u
#define PDFV_ZOOM_IN          4u
#define PDFV_ZOOM_OUT         5u

/*
 * The pixel layout of a frame. One value, and it is a number rather than an assumption because
 * `pdf_render::RasterFormat` stopped being `#[non_exhaustive]` in ADR 0247 so that a second layout
 * would fail to compile in every consumer — a C caller being the consumer that could not.
 */
#define PDFV_FORMAT_RGBA8 0u

/* ------------------------------------------------------------------------------------------- */
/* Opaque handles. Each is released with its own `_free`.                                        */
/* ------------------------------------------------------------------------------------------- */

typedef struct pdfv_viewer pdfv_viewer;
typedef struct pdfv_events pdfv_events;
typedef struct pdfv_render_request pdfv_render_request;
typedef struct pdfv_raster pdfv_raster;
typedef struct pdfv_outline pdfv_outline;
/* §8.11.4.3's layers and §7.11.4's files. One handle for both, because they differ only in what
 * acting on a row does — which `pdfv_panel_action` says. The outline keeps its own three
 * accessors because a C entry point cannot change shape once it is compiled against. */
typedef struct pdfv_panel pdfv_panel;
/* §12.7's fields on the page being shown. */
typedef struct pdfv_fields pdfv_fields;
/* A list of quadrilaterals in device pixels: a selection, a field's selection. */
typedef struct pdfv_quads pdfv_quads;
/* Every occurrence of a search term on the page being shown. A LIST OF LISTS: one occurrence is
 * several quadrilaterals, because a term wrapped across a line is merged per run of a line — so
 * "next match" is an index into this and never the next shape. `pdfv_matches_quads` hands each
 * occurrence over as a pdfv_quads you free. */
typedef struct pdfv_matches pdfv_matches;
/* §12.3.4's miniature for ONE page, decoded. There is deliberately no list-valued form of this:
 * the clause's NOTE says thumbnails "are not required, and can be included for some pages and not
 * for others", Table 29's /PageMode /UseThumbs opens that panel AS THE DOCUMENT OPENS, and a
 * thousand-page document stating one per page would decode a thousand images to draw eight. Ask
 * for the rows you are about to draw. */
typedef struct pdfv_thumbnail pdfv_thumbnail;
/* §12.5.6.14's open popup windows. The one annotation subtype whose picture is NOT the page's:
 * the clause makes a popup "a window … for entry and editing" with "no appearance stream", so you
 * draw it as chrome in your platform's own window furniture. */
typedef struct pdfv_popups pdfv_popups;
/* §14.7's logical structure, ONE TREE PER PAGE the arrangement is showing. Two indices, and the
 * standard is why: §14.7.5.2's marked-content identifier is unique "within its content stream"
 * and §14.7.5.4 keys the route in from that page's /StructParents, so two pages' trees share no
 * numbering and there is no order between them to renumber by. Every index a node carries — its
 * parent, its header cells — is into THAT PAGE's list. */
typedef struct pdfv_structure pdfv_structure;
/* §12.3.5's portable collection: Table 153's view, §12.3.5.1's resolved initial document, Table
 * 155's columns in /O order and §12.3.5.2's folder tree. The files themselves are
 * `pdfv_attachments_read`'s, and `pdfv_collection_folder_of` is what puts one inside the other. */
typedef struct pdfv_collection pdfv_collection;

/* ------------------------------------------------------------------------------------------- */
/* The structs passed by value.                                                                  */
/* ------------------------------------------------------------------------------------------- */

/* Where a page sits on the screen and how large it is drawn. */
typedef struct pdfv_geometry {
    float    page_width;   /* the page's extent in user space units, after §7.7.3.3's /Rotate */
    float    page_height;
    float    scale;        /* device pixels per user space unit: zoom and display scale together */
    uint32_t width;        /* the rasterised page, in device pixels */
    uint32_t height;
    float    origin_x;     /* where the raster's top-left corner sits in the viewport */
    float    origin_y;
} pdfv_geometry;

/*
 * What the viewer is holding, without the pixels. Ask this, size a buffer, then copy.
 *
 * `pdfv_frame` and not `pdfv_frame_info`: C puts a struct tag and a function in one namespace, so
 * the obvious name collides with `pdfv_frame_info()` below. Found by compiling this header.
 */
typedef struct pdfv_frame {
    size_t   page;         /* zero-based */
    uint32_t width;
    uint32_t height;
    uint32_t format;       /* PDFV_FORMAT_RGBA8 */
    size_t   bytes;        /* exactly what pdfv_frame_copy writes */
    float    origin_x;
    float    origin_y;
} pdfv_frame;

/*
 * Where the reader is looking: the page, the magnification and the scroll, together.
 *
 * READ ONE, HAND IT BACK. `pdfv_view` answers with one and `pdfv_set_view` takes it, and that is
 * the whole point of the type: the commands that make a view are relative and the viewer clamps
 * them, so a caller that issued every one of them still cannot say where the reader ended up.
 * Composing one from numbers of your own is a guess about where that clamp would have left them.
 */
typedef struct pdfv_viewing {
    size_t page;           /* zero-based */
    uint32_t zoom;         /* PDFV_ZOOM_* */
    float    scale;        /* logical pixels per user space unit, for PDFV_ZOOM_SCALE only */
    float    scroll_x;     /* device pixels; positive has moved the content up and left */
    float    scroll_y;
} pdfv_viewing;

/* ------------------------------------------------------------------------------------------- */
/* The identity of the ABI.                                                                      */
/* ------------------------------------------------------------------------------------------- */

uint32_t pdfv_abi_version(void);
uint32_t pdfv_event_kind_count(void);

/*
 * Whether this library agrees with the header the caller compiled against. Pass PDFV_ABI_VERSION
 * and PDFV_EVENT_KIND_COUNT. PDFV_NUMBER_OUT_OF_RANGE means a number moved.
 */
int32_t pdfv_abi_check(uint32_t version, uint32_t event_kinds);

/* One sentence about a status, and the name of an event kind. Static; never freed. */
const char *pdfv_status_message(int32_t status);
const char *pdfv_event_kind_name(uint32_t kind);

/* The same pair for the two enumerations this library answers with but does not push. */
uint32_t    pdfv_control_kind_count(void);
const char *pdfv_control_kind_name(uint32_t kind);
uint32_t    pdfv_row_kind_count(void);
const char *pdfv_row_kind_name(uint32_t kind);

/* ------------------------------------------------------------------------------------------- */
/* The viewer.                                                                                   */
/* ------------------------------------------------------------------------------------------- */

/* `scale` is device pixels per logical pixel: 1.0f ordinarily, 2.0f on a doubled display. */
pdfv_viewer *pdfv_viewer_new(uint32_t width, uint32_t height, float scale);
void pdfv_viewer_free(pdfv_viewer *viewer);

/* ------------------------------------------------------------------------------------------- */
/* Commands. One function each: a union's size is part of an ABI and a symbol is not.             */
/* ------------------------------------------------------------------------------------------- */

/*
 * Opens a document. The bytes are copied, so the caller may free them at once. `password` is
 * §7.6.4.1's and `fragment` is Annex O's; either may be NULL.
 */
int32_t pdfv_open(pdfv_viewer *viewer, uint64_t document, const uint8_t *bytes, size_t len,
                  const char *password, const char *fragment, pdfv_events **events);
int32_t pdfv_close(pdfv_viewer *viewer, uint64_t document, pdfv_events **events);
int32_t pdfv_focus(pdfv_viewer *viewer, uint64_t document, pdfv_events **events);

/* Width and height are DEVICE pixels: a page is rasterised at the resolution it is shown at. */
int32_t pdfv_resize(pdfv_viewer *viewer, uint32_t width, uint32_t height, float scale,
                    pdfv_events **events);

int32_t pdfv_go_to_page(pdfv_viewer *viewer, uint32_t target, int64_t argument,
                        pdfv_events **events);
int32_t pdfv_zoom(pdfv_viewer *viewer, uint32_t zoom, float scale, pdfv_events **events);

/* Positive `dy` moves the content up, which is what a wheel scrolling down does. */
int32_t pdfv_scroll(pdfv_viewer *viewer, float dx, float dy, pdfv_events **events);

/* Puts the reader back where `pdfv_view` said they were. PDFV_WRONG_KIND for a `zoom` this
 * build does not define. */
int32_t pdfv_set_view(pdfv_viewer *viewer, pdfv_viewing view, pdfv_events **events);

/*
 * Annex O's `search`, and a find bar's next/previous. `needle` is NUL-terminated UTF-8.
 *
 * ONE PAGE PER STEP: pdfv_find_start takes the first, then pump pdfv_find_continue until
 * pdfv_event_searched reports `remaining` of zero. A sweep of ISO 32000-2's own 1023 pages is
 * 5.84 s, and this library does not block your event loop for it.
 */
int32_t pdfv_find_start(pdfv_viewer *viewer, const char *needle, int32_t backward,
                        pdfv_events **events);
int32_t pdfv_find_continue(pdfv_viewer *viewer, pdfv_events **events);
int32_t pdfv_find_stop(pdfv_viewer *viewer, pdfv_events **events);

/* §12.3.3: activates an object shown outside the page — an outline row. */
int32_t pdfv_activate(pdfv_viewer *viewer, uint32_t number, uint16_t generation,
                      pdfv_events **events);

/* Both handles are CONSUMED. A stale token is dropped by the viewer, never drawn. */
int32_t pdfv_render_ready_raster(pdfv_viewer *viewer, pdfv_render_request *request,
                                 pdfv_raster *raster, pdfv_events **events);
/* The request is CONSUMED. `why` may be NULL. */
int32_t pdfv_render_ready_failed(pdfv_viewer *viewer, pdfv_render_request *request,
                                 const char *why, pdfv_events **events);

/*
 * §12.5.5: the pointer, in device pixels from the viewport's top-left corner. `action` is one of
 * PDFV_POINTER_*. One button, because that is all the clause assumes — "the term mouse denotes a
 * generic pointing device … [with] at least one button".
 */
int32_t pdfv_pointer(pdfv_viewer *viewer, float x, float y, uint32_t action,
                     pdfv_events **events);
/* Select everything the page reads back as, or nothing. PDFV_SELECT_*. */
int32_t pdfv_select(pdfv_viewer *viewer, uint32_t what, pdfv_events **events);
/* §12.5.1's tab key. The order is the document's (Table 31's /Tabs); the key is yours. */
int32_t pdfv_focused(pdfv_viewer *viewer, uint32_t direction, pdfv_events **events);

/* §12.7.4: the four edits. A name and not a widget: a field's value belongs to the field. */
int32_t pdfv_set_field_text(pdfv_viewer *viewer, const char *field, const char *text,
                            pdfv_events **events);
/* §12.7.5.4: which of Table 234's options are selected, as indices into /Opt. */
int32_t pdfv_set_field_options(pdfv_viewer *viewer, const char *field, const size_t *options,
                               size_t count, pdfv_events **events);
/* §12.7.6.3's "its V entry shall be removed" — not the same state as never having touched it. */
int32_t pdfv_clear_field(pdfv_viewer *viewer, const char *field, pdfv_events **events);
/* §12.5.6.10: mark up WHAT IS SELECTED. Nothing happens where nothing is. Colour in DeviceRGB. */
int32_t pdfv_markup(pdfv_viewer *viewer, uint32_t kind, float red, float green, float blue,
                    pdfv_events **events);
/* §12.5.6.6: a free text annotation over a rectangle DRAWN — that subtype has no text under it. */
int32_t pdfv_free_text(pdfv_viewer *viewer, float from_x, float from_y, float to_x, float to_y,
                       float red, float green, float blue, pdfv_events **events);
/* §12.5.6.6: what one this session added says. Only one this session added. */
int32_t pdfv_set_free_text(pdfv_viewer *viewer, uint32_t number, uint16_t generation,
                           const char *text, pdfv_events **events);
int32_t pdfv_undo(pdfv_viewer *viewer, pdfv_events **events);
int32_t pdfv_redo(pdfv_viewer *viewer, pdfv_events **events);

/* §7.5.6's incremental update, and §7.11.4's embedded file. Both answer with bytes on an event. */
int32_t pdfv_save(pdfv_viewer *viewer, pdfv_events **events);
int32_t pdfv_extract(pdfv_viewer *viewer, const char *name, pdfv_events **events);
/* The answer to a PDFV_EVENT_NEEDS_FILE. A NULL `bytes` is a refusal, which is a fair answer. */
int32_t pdfv_supply(pdfv_viewer *viewer, uint32_t purpose, const uint8_t *bytes, size_t len,
                    pdfv_events **events);

/* §8.11: switch an optional content group on or off. Table 99's /Locked forbids some. */
int32_t pdfv_set_group(pdfv_viewer *viewer, uint32_t number, uint16_t generation, bool on,
                       pdfv_events **events);

/* §12.4.4.1's clock. This library has none, so a presentation is advanced by being told. */
int32_t pdfv_tick(pdfv_viewer *viewer, uint32_t millis, pdfv_events **events);
/* §12.4.4: whether a presentation is running. PDFV_PRESENT_*. Only a host knows. */
int32_t pdfv_present(pdfv_viewer *viewer, uint32_t mode, pdfv_events **events);
/* Table 29's arrangement, as the reader has chosen it. PDFV_LAYOUT_*. */
int32_t pdfv_layout(pdfv_viewer *viewer, uint32_t layout, pdfv_events **events);
/* The reader's policy about the document's restrictions. PDFV_RESTRICT_*. */
int32_t pdfv_restrict(pdfv_viewer *viewer, uint32_t level, pdfv_events **events);
/* §6.3.2.2's "unless otherwise instructed". PDFV_DELEGATE_*. Re-interprets the page. */
int32_t pdfv_delegate(pdfv_viewer *viewer, uint32_t appearances, pdfv_events **events);

/* ------------------------------------------------------------------------------------------- */
/* Events. Owned, so that the viewer's borrow ends before the caller sees anything.               */
/* ------------------------------------------------------------------------------------------- */

void   pdfv_events_free(pdfv_events *events);
size_t pdfv_events_len(const pdfv_events *events);
int32_t pdfv_events_kind(const pdfv_events *events, size_t index, uint32_t *kind);

/*
 * One sentence about the event at `index`, WHATEVER KIND IT IS — including a kind added after this
 * caller was compiled. Two-call idiom: pass out=NULL to learn `needed` (which counts the NUL),
 * then call again. Nothing is written unless the whole sentence fits.
 */
int32_t pdfv_events_describe(const pdfv_events *events, size_t index, char *out, size_t cap,
                             size_t *needed);

/* Typed accessors. Any out-parameter may be NULL. PDFV_WRONG_KIND rather than zeroes. */
int32_t pdfv_event_opened(const pdfv_events *events, size_t index, uint64_t *document,
                          size_t *pages);
int32_t pdfv_event_page_changed(const pdfv_events *events, size_t index, size_t *page,
                                size_t *of);
/*
 * A step of a document-wide search. `found` says whether `page`, `from` and `to` mean anything;
 * `remaining` says whether to call pdfv_find_continue again. `from` and `to` are byte offsets
 * into the page's readback, which is by then also the selection.
 */
int32_t pdfv_event_searched(const pdfv_events *events, size_t index, int32_t *found, size_t *page,
                            size_t *from, size_t *to, size_t *remaining, int32_t *wrapped);
/* An owning handle: hand it back, or release it with pdfv_render_request_free. */
int32_t pdfv_event_render_request(const pdfv_events *events, size_t index,
                                  pdfv_render_request **request);

/* Which document the event is about. PDFV_WRONG_KIND for DAMAGE, which is about the viewport. */
int32_t pdfv_event_document(const pdfv_events *events, size_t index, uint64_t *document);
/*
 * The bytes of a SAVED or an EXTRACTED. A BYTE buffer, not a string: both carry a file, and a
 * file is not text — the NUL-terminated idiom would cut either at its first zero byte. `needed`
 * counts no terminator, and nothing is written unless the whole file fits.
 */
int32_t pdfv_event_bytes(const pdfv_events *events, size_t index, uint8_t *out, size_t cap,
                         size_t *needed);
/* An EXTRACTED's file name, and whether a PERSON asked for it — §O.2.1's own distinction. */
int32_t pdfv_event_extracted(const pdfv_events *events, size_t index, bool *asked, char *out,
                             size_t cap, size_t *needed);
/* §12.6.4.8's resolved URI. Handed over rather than opened: the string is the document's. */
int32_t pdfv_event_open_uri(const pdfv_events *events, size_t index, char *out, size_t cap,
                            size_t *needed);
/* What a file is wanted for and the document's own words for it. Answer with pdfv_supply. */
int32_t pdfv_event_needs_file(const pdfv_events *events, size_t index, uint32_t *purpose,
                              char *out, size_t cap, size_t *needed);
/* [x0, y0, x1, y1] in device pixels: a bound on what changed, not a promise that all of it did. */
int32_t pdfv_event_damage(const pdfv_events *events, size_t index, float *into);
int32_t pdfv_event_dirty(const pdfv_events *events, size_t index, bool *dirty);
/* §12.4.4's Table 164, without /S. `dimension` is PDFV_DIMENSION_*, `motion` PDFV_MOTION_*. */
int32_t pdfv_event_transition(const pdfv_events *events, size_t index, float *seconds,
                              uint32_t *dimension, uint32_t *motion, bool *directed,
                              float *degrees, float *scale, bool *opaque);
/* Table 164's /S as the table spells it — a NAME, because its thirteenth case is one it does not
 * define, "kept as the file wrote it". A number would have had to lose that one. */
int32_t pdfv_event_transition_style(const pdfv_events *events, size_t index, char *out,
                                    size_t cap, size_t *needed);

/* ------------------------------------------------------------------------------------------- */
/* Rendering. The display list stays opaque; what crosses is "draw this" and the pixels.          */
/* ------------------------------------------------------------------------------------------- */

void pdfv_render_request_free(pdfv_render_request *request);
int32_t pdfv_render_request_page(const pdfv_render_request *request, size_t *page,
                                 uint32_t *width, uint32_t *height);
/* Draws it with the processor rasteriser. May be called on a thread of the caller's own. */
int32_t pdfv_render_request_rasterise(const pdfv_render_request *request, pdfv_raster **raster);
void pdfv_raster_free(pdfv_raster *raster);

/* ------------------------------------------------------------------------------------------- */
/* Queries. Synchronous, and they produce no events.                                              */
/* ------------------------------------------------------------------------------------------- */

int32_t pdfv_page_count(const pdfv_viewer *viewer, size_t *pages);
int32_t pdfv_current_page(const pdfv_viewer *viewer, size_t *page, size_t *of);
int32_t pdfv_page_geometry(const pdfv_viewer *viewer, size_t page, pdfv_geometry *geometry);

/* Where the reader is looking. PDFV_NO_ANSWER when no document is focused. */
int32_t pdfv_view(const pdfv_viewer *viewer, pdfv_viewing *view);
/* How many pages Table 29's arrangement is showing: 1 under PDFV_LAYOUT_SINGLE_PAGE, more under
 * a column or a spread, 0 where the viewer holds no pixels. `frame` below indexes this. */
size_t  pdfv_frame_count(const pdfv_viewer *viewer);
int32_t pdfv_frame_info(const pdfv_viewer *viewer, size_t frame, pdfv_frame *info);
/* One copy — what tier 1 costs everywhere in this project. Size `into` from info.bytes. */
int32_t pdfv_frame_copy(const pdfv_viewer *viewer, size_t frame, uint8_t *into, size_t cap,
                        size_t *written);
int32_t pdfv_dirty(const pdfv_viewer *viewer, bool *dirty);
/* §12.5.6.5: whether activating here would follow a link. Asked on every pointer move. */
int32_t pdfv_link_at(const pdfv_viewer *viewer, float x, float y, bool *link);
/* What the pages on the screen could not draw — the same sentences a REPORTED carried, kept so a
 * caller that cleared its status bar can ask again rather than remembering. ONE ENTRY PER PAGE
 * Table 29's arrangement is showing: `entry` below indexes this, and pdfv_reported_page says
 * which page an entry is about. A caller that showed one page's sentences for a column of four
 * would be silent about three pages and talkative about a page nobody is looking at. */
size_t  pdfv_reported_pages(const pdfv_viewer *viewer);
int32_t pdfv_reported_page(const pdfv_viewer *viewer, size_t entry, size_t *page);
int32_t pdfv_reports_len(const pdfv_viewer *viewer, size_t entry, size_t *count);
int32_t pdfv_report(const pdfv_viewer *viewer, size_t entry, size_t index, char *out, size_t cap,
                    size_t *needed);

/* ------------------------------------------------------------------------------------------- */
/* Geometry: what a caller draws over the page in its own colours.                               */
/*                                                                                               */
/* Interactive chrome crosses as SHAPES rather than as pixels, so that a selection is drawn in    */
/* macOS's selection colour, KDE's accent or the Windows highlight brush — and so that a drag     */
/* never forces the page to be drawn again.                                                      */
/* ------------------------------------------------------------------------------------------- */

/* What is selected, as the pages read back. A drag across a continuous /PageLayout may cross a
   page boundary, in which case this is each page's part in page order, joined by a newline. */
int32_t pdfv_selection_text(const pdfv_viewer *viewer, char *out, size_t cap, size_t *needed);
int32_t pdfv_selection_quads(const pdfv_viewer *viewer, pdfv_quads **quads);
/* What to put on YOUR clipboard, which is not the same string as pdfv_selection_text: §14.8.2.5
   gives a page a logical content order as well as a page content order and only recommends that
   they coincide, so a copy off a two-column page wants the first and a highlight wants the
   second. `order` receives PDFV_ORDER_* and may be null; it is written only on PDFV_OK.
   PDFV_NO_ANSWER where nothing is selected — a copy with nothing to copy must not empty your
   clipboard. The clipboard itself is yours: this ABI has no idea what platform you are on. */
int32_t pdfv_selection_copy_text(const pdfv_viewer *viewer, char *out, size_t cap, size_t *needed,
                                 uint32_t *order);
/* Annex O's `highlight`: the rectangles the URI's fragment asked to be shown highlighted, on the
   page being shown. Empty unless a fragment named one — the annex leaves the look to you. */
int32_t pdfv_highlight_quads(const pdfv_viewer *viewer, pdfv_quads **quads);
void    pdfv_quads_free(pdfv_quads *quads);
size_t  pdfv_quads_len(const pdfv_quads *quads);
/* `into` takes eight floats: [x0, y0, … x3, y3], device pixels of the viewport, y downwards. */
int32_t pdfv_quads_get(const pdfv_quads *quads, size_t index, float *into);
/* §12.5.1's focus ring: which annotation has it, and where. The ring itself is yours to draw. */
int32_t pdfv_focused_annotation(const pdfv_viewer *viewer, uint32_t *number,
                                uint16_t *generation, float *quad);

/* ------------------------------------------------------------------------------------------- */
/* §12.7's form, as controls a caller builds rather than as pixels off the raster.                */
/* ------------------------------------------------------------------------------------------- */

/* Walks §12.7.4.1's field tree, so ask it when a page appears and after an edit — not on a click,
 * which asks pdfv_field_at. */
int32_t pdfv_fields_read(const pdfv_viewer *viewer, pdfv_fields **fields);
void    pdfv_fields_free(pdfv_fields *fields);
size_t  pdfv_fields_len(const pdfv_fields *fields);
/* PDFV_TEXT_QUALIFIED addresses the field; PDFV_TEXT_SHOWN is what §14.9.3 says to display. */
int32_t pdfv_field_name(const pdfv_fields *fields, size_t field, uint32_t which, char *out,
                        size_t cap, size_t *needed);
/* PDFV_CONTROL_* and a word of PDFV_FIELD_* bits. */
int32_t pdfv_field_control(const pdfv_fields *fields, size_t field, uint32_t *kind,
                           uint32_t *flags);
/* Table 232's /MaxLen and Table 231 bit 25's cell count; zero where the field states none. */
int32_t pdfv_field_limits(const pdfv_fields *fields, size_t field, uint32_t *max_len,
                          uint32_t *comb_cells);
/* PDFV_NO_ANSWER for a field with no text value AT ALL, which differs from the empty string. */
int32_t pdfv_field_value(const pdfv_fields *fields, size_t field, char *out, size_t cap,
                         size_t *needed);
int32_t pdfv_field_option_count(const pdfv_fields *fields, size_t field, size_t *count);
/* PDFV_TEXT_LABEL or PDFV_TEXT_EXPORT, in /Opt's own order, which Table 233 bit 20 requires. */
int32_t pdfv_field_option(const pdfv_fields *fields, size_t field, size_t option, uint32_t which,
                          char *out, size_t cap, size_t *needed);
int32_t pdfv_field_option_selected(const pdfv_fields *fields, size_t field, size_t option,
                                   bool *selected);
int32_t pdfv_field_widget_count(const pdfv_fields *fields, size_t field, size_t *count);
/* `quad` takes eight floats, as pdfv_quads_get's does. */
int32_t pdfv_field_widget(const pdfv_fields *fields, size_t field, size_t widget, uint32_t *number,
                          uint16_t *generation, float *quad, bool *on);
/* PDFV_TEXT_LABEL is the /AP /N on-state name — what pdfv_set_field_text sends to check the box.
 * PDFV_TEXT_EXPORT is Table 230's /Opt entry, which may differ: §12.7.5.2.3 lets /AP use the
 * widget's numerical position instead, so /AP may say `0` while /Opt says `Rot`. */
int32_t pdfv_field_widget_text(const pdfv_fields *fields, size_t field, size_t widget,
                               uint32_t which, char *out, size_t cap, size_t *needed);

/* What the field at a point is called. PDFV_TEXT_QUALIFIED or PDFV_TEXT_SHOWN. */
int32_t pdfv_field_at(const pdfv_viewer *viewer, float x, float y, uint32_t which, char *out,
                      size_t cap, size_t *needed);
/* Two points, because a caret has no width: how thick one is drawn is your platform's business. */
int32_t pdfv_caret(const pdfv_viewer *viewer, float x, float y, size_t offset, float *from_x,
                   float *from_y, float *to_x, float *to_y);
/* The caret's inverse: `x`,`y` name the field and `point_x`,`point_y` the place to measure. */
int32_t pdfv_offset(const pdfv_viewer *viewer, float x, float y, float point_x, float point_y,
                    size_t *offset);
int32_t pdfv_field_selection(const pdfv_viewer *viewer, float x, float y, size_t from, size_t to,
                             pdfv_quads **quads);
/* §12.5.6.6: the annotation THIS SESSION added at a point, which pdfv_set_free_text names back. */
int32_t pdfv_free_text_at(const pdfv_viewer *viewer, float x, float y, uint32_t *number,
                          uint16_t *generation, char *out, size_t cap, size_t *needed);

/* ------------------------------------------------------------------------------------------- */
/* §8.11.4.3's layers and §7.11.4's files, flattened as the outline is.                           */
/* ------------------------------------------------------------------------------------------- */

int32_t pdfv_layers_read(const pdfv_viewer *viewer, pdfv_panel **panel);
int32_t pdfv_attachments_read(const pdfv_viewer *viewer, pdfv_panel **panel);
void    pdfv_panel_free(pdfv_panel *panel);
size_t  pdfv_panel_len(const pdfv_panel *panel);
/* `detail` non-zero asks for the second line, where the answer carries one. */
int32_t pdfv_panel_text(const pdfv_panel *panel, size_t row, int32_t detail, char *out,
                        size_t cap, size_t *needed);
int32_t pdfv_panel_depth(const pdfv_panel *panel, size_t row, uint32_t *depth, bool *expanded);
/* PDFV_ROW_* and everything the action carries. `on` and `locked` mean something for TOGGLE. */
int32_t pdfv_panel_action(const pdfv_panel *panel, size_t row, uint32_t *kind, uint32_t *number,
                          uint16_t *generation, bool *on, bool *locked);
/* The /EmbeddedFiles key pdfv_extract takes, for a PDFV_ROW_EXTRACT row. */
int32_t pdfv_panel_name(const pdfv_panel *panel, size_t row, char *out, size_t cap,
                        size_t *needed);

/* ------------------------------------------------------------------------------------------- */
/* §12.3.3's outline, flattened: a tree is the one shape a C ABI cannot hand over as itself.       */
/* ------------------------------------------------------------------------------------------- */

int32_t pdfv_outline_read(const pdfv_viewer *viewer, pdfv_outline **outline);
void    pdfv_outline_free(pdfv_outline *outline);
size_t  pdfv_outline_len(const pdfv_outline *outline);
/* Table 151's /Title, in the same two-call idiom pdfv_events_describe uses. */
int32_t pdfv_outline_title(const pdfv_outline *outline, size_t row, char *out, size_t cap,
                           size_t *needed);
/* `depth` is zero at the top level; `expanded` is the sign of §12.3.3's /Count. */
int32_t pdfv_outline_depth(const pdfv_outline *outline, size_t row, uint32_t *depth,
                           bool *expanded);
/* §7.3.10's two numbers, which pdfv_activate takes. */
int32_t pdfv_outline_object(const pdfv_outline *outline, size_t row, uint32_t *number,
                            uint16_t *generation);

/* ------------------------------------------------------------------------------------------- */
/* THE OTHER HALF OF THE QUERIES (ADR 0576).                                                     */
/*                                                                                               */
/* Everything below answers a question `viewer_core::Query` had and this header did not. None of  */
/* it takes or returns a struct by value, so PDFV_ABI_VERSION did not move.                      */
/* ------------------------------------------------------------------------------------------- */

/* Every occurrence of a string on the page in front of the reader, out of a readback that already
 * exists — microseconds, so a find bar may ask on every repaint. `pdfv_find_start` is the other
 * half and searches the DOCUMENT one page per step; a caller had that one and not this, so it
 * could run Annex O's search and not draw a single match. */
int32_t pdfv_find_matches(const pdfv_viewer *viewer, const char *needle, pdfv_matches **matches);
void    pdfv_matches_free(pdfv_matches *matches);
size_t  pdfv_matches_len(const pdfv_matches *matches);
/* One occurrence's shapes, as a pdfv_quads you free with pdfv_quads_free. */
int32_t pdfv_matches_quads(const pdfv_matches *matches, size_t index, pdfv_quads **quads);

/* Table 29's /PageMode and /PageLayout: what the CATALOGUE asks of the window opening it.
 * `pdfv_layout` sets the arrangement a reader chose; this is the one the document opens in. */
int32_t pdfv_opening(const pdfv_viewer *viewer, uint32_t *mode, uint32_t *layout);
uint32_t    pdfv_page_mode_count(void);
const char *pdfv_page_mode_name(uint32_t mode);

/* §12.2's Table 147, one entry at a time. See PDFV_PREF_* above for what each answers with and
 * for why this is a key rather than a struct. */
int32_t pdfv_preference(const pdfv_viewer *viewer, uint32_t key, int64_t *value);
/* /PrintPageRange, the one entry of that table that is a list: "[t]he first and last pages in a
 * sub-range", one-based as the entry states them. */
int32_t pdfv_preference_ranges(const pdfv_viewer *viewer, size_t *count);
int32_t pdfv_preference_range(const pdfv_viewer *viewer, size_t index, int64_t *first,
                              int64_t *last);
uint32_t    pdfv_preference_key_count(void);
const char *pdfv_preference_key_name(uint32_t key);

/* §14.3.3's Table 349 with §14.3.2's metadata stream under it, and §12.4.3's article threads —
 * both as pdfv_panel, read with the pdfv_panel_* accessors and released with pdfv_panel_free.
 * Both tables of the first are shown rather than merged: §14.3.4 leaves a disagreement between
 * them "at the discretion of the PDF processor", so merging would hide one rather than resolve
 * it. Every article row is a PDFV_ROW_ACTIVATE, which is the same message an outline row sends —
 * the DOCUMENT decides what activating a thread means, and following one lands on Table 163's /R
 * rather than on the page its first bead sits on. */
int32_t pdfv_properties_read(const pdfv_viewer *viewer, pdfv_panel **panel);
int32_t pdfv_articles_read(const pdfv_viewer *viewer, pdfv_panel **panel);

/* §12.4.2's label for one page. PDFV_NO_ANSWER for a page that states none, which is most pages
 * of most documents: the clause makes the integer index what identifies a page and the label an
 * addition, so fall back to the number rather than to nothing.
 *
 * SEPARATE from the thumbnail on purpose. A page list needs a name per row and a picture only for
 * the rows it is showing; one call answering both would make listing a thousand pages decode a
 * thousand images — which is the launch-path defect the host that drew its own rows had. */
int32_t pdfv_page_label(const pdfv_viewer *viewer, size_t page, char *out, size_t cap,
                        size_t *needed);

/* §12.3.4's miniature for one page. PDFV_NO_ANSWER for a page with no /Thumb and for one this
 * reader could not decode. `permitted` is a word of PDFV_THUMBNAIL_* bits, zero for a conformant
 * one; `format` is PDFV_FORMAT_RGBA8. Size `into` from `bytes`. */
int32_t pdfv_thumbnail_read(const pdfv_viewer *viewer, size_t page, pdfv_thumbnail **thumbnail);
void    pdfv_thumbnail_free(pdfv_thumbnail *thumbnail);
int32_t pdfv_thumbnail_info(const pdfv_thumbnail *thumbnail, uint32_t *width, uint32_t *height,
                            uint32_t *format, size_t *bytes, uint32_t *permitted);
int32_t pdfv_thumbnail_copy(const pdfv_thumbnail *thumbnail, uint8_t *into, size_t cap,
                            size_t *written);

/* §9.10.2's counts, ONE ENTRY PER PAGE the arrangement is showing, exactly as the reports above
 * are. `entry` indexes this and pdfv_readback_page says which page an entry is about. */
size_t  pdfv_readback_pages(const pdfv_viewer *viewer);
int32_t pdfv_readback_page(const pdfv_viewer *viewer, size_t entry, size_t *page);
int32_t pdfv_readback_count(const pdfv_viewer *viewer, size_t entry, uint32_t which,
                            size_t *count);
uint32_t    pdfv_shortfall_kind_count(void);
const char *pdfv_shortfall_kind_name(uint32_t which);

/* §12.5.6.14's open popup windows on the page being shown, in /Annots order. Only the OPEN ones:
 * Table 186's /Open says which start that way, and pdfv_activate on the parent annotation changes
 * it — §12.5.1's "[w]hen the user activates the annotation by clicking it, it exhibits its
 * associated object, such as by opening a popup window displaying a text note". */
int32_t pdfv_popups_read(const pdfv_viewer *viewer, pdfv_popups **popups);
void    pdfv_popups_free(pdfv_popups *popups);
size_t  pdfv_popups_len(const pdfv_popups *popups);
/* The popup, and Table 186's /Parent where it names one — the first is what pdfv_activate closes
 * the window with, the second is the markup annotation the note belongs to. */
int32_t pdfv_popup_object(const pdfv_popups *popups, size_t index, uint32_t *number,
                          uint16_t *generation, bool *has_parent, uint32_t *parent_number,
                          uint16_t *parent_generation);
/* Eight floats, [x0, y0, … x3, y3], y downwards, in device pixels of the viewport. */
int32_t pdfv_popup_quad(const pdfv_popups *popups, size_t index, float *into);
int32_t pdfv_popup_text(const pdfv_popups *popups, size_t index, uint32_t which, char *out,
                        size_t cap, size_t *needed);
/* Table 166's /C, "[t]he title bar of the annotation's popup window", as three DeviceRGB
 * components. PDFV_NO_ANSWER where none is stated, which is not the same as black. */
int32_t pdfv_popup_colour(const pdfv_popups *popups, size_t index, float *into);

/* §14.7's logical structure for every page the arrangement is showing. Zero nodes for an untagged
 * page is an ANSWER rather than a silence: §14.7 leaves a producer free to state no structure, and
 * inventing a reading order for one would be a guess where a person is entitled to the author's
 * answer. */
int32_t pdfv_structure_read(const pdfv_viewer *viewer, pdfv_structure **structure);
void    pdfv_structure_free(pdfv_structure *structure);
size_t  pdfv_structure_pages(const pdfv_structure *structure);
int32_t pdfv_structure_page(const pdfv_structure *structure, size_t entry, size_t *page,
                            size_t *nodes);
/* `has_parent` is false for a root. `substituted` is the one to ACT on rather than display:
 * §14.9.3 makes /Alt "a complete (or whole) word or phrase substitution for the current element"
 * and §14.9.5 says the same of /E, so an element stating one has said what to speak INSTEAD of its
 * content — descend anyway and it is read twice. `has_scope` is false for everything that is not a
 * TH, and for a TH this reader could place in no grid, which is us saying we do not know. */
int32_t pdfv_structure_node(const pdfv_structure *structure, size_t entry, size_t node,
                            size_t *parent, bool *has_parent, bool *substituted, uint32_t *scope,
                            bool *has_scope);
int32_t pdfv_structure_text(const pdfv_structure *structure, size_t entry, size_t node,
                            uint32_t which, char *out, size_t cap, size_t *needed);
/* Where the element's own text was drawn, as a pdfv_quads you free. Empty for an element whose
 * content drew no text — a figure, a table cell holding an image — which is a statement about our
 * text layer rather than about the element, and is why pdfv_structure_box is beside it. */
int32_t pdfv_structure_quads(const pdfv_structure *structure, size_t entry, size_t node,
                             pdfv_quads **quads);
/* Four floats, [x0, y0, x1, y1]. PDFV_NO_ANSWER where the node has no rectangle of that kind. */
int32_t pdfv_structure_box(const pdfv_structure *structure, size_t entry, size_t node,
                           uint32_t which, float *into);
/* §14.8.4.8.3's header cells for this element, as indices into THIS PAGE's node list. */
int32_t pdfv_structure_headers(const pdfv_structure *structure, size_t entry, size_t node,
                               size_t *count);
int32_t pdfv_structure_header(const pdfv_structure *structure, size_t entry, size_t node,
                              size_t header, size_t *cell);
/* The element's own text again, one line at a time, with each character code's place — which is
 * what AT-SPI's org.a11y.atspi.Text asks for and what PDFV_ELEMENT_NAME cannot answer. A name is
 * one string for a whole paragraph, so a client can read it or not read it; moving a caret through
 * it by character, by word or by line needs to know where each character begins and which
 * characters share a line.
 *
 * NOT §14.9's substitutions, and that is the point: PDFV_ELEMENT_NAME applies /Alt and /E, and
 * these do not — a caret moves over what is on the page, and a phrase that replaces the content has
 * no glyphs whose positions could be reported. So an element stating one has zero lines here, which
 * is the same thing pdfv_structure_node's `substituted` says. */
int32_t pdfv_structure_lines(const pdfv_structure *structure, size_t entry, size_t node,
                             size_t *count);
/* One line's text and how many character codes produced it, in one call: the character byte counts
 * below sum to the length of this string, so an offset into the text and an index into the
 * characters convert into each other without either side guessing. */
int32_t pdfv_structure_line(const pdfv_structure *structure, size_t entry, size_t node, size_t line,
                            size_t *characters, char *out, size_t cap, size_t *needed);
/* One character code's share of a line: how many bytes of the line's text it produced, and where
 * its glyph is as [x0, y0, x1, y1] in the viewport's device pixels. THE UNIT IS THE CODE, not the
 * character: a code mapped through /ToUnicode to a several-character string — a ligature read back
 * as "ffi" — drew one glyph in one place, and splitting its box into thirds would invent positions
 * the file does not state. */
int32_t pdfv_structure_character(const pdfv_structure *structure, size_t entry, size_t node,
                                 size_t line, size_t character, size_t *bytes, float *into);

/* §12.3.5's portable collection, where the catalogue states one — PDFV_NO_ANSWER otherwise, which
 * is every document in this project's corpora. The clause is a `shall` on a viewer: "[i]f this
 * dictionary is present in a PDF document, the interactive PDF processor shall present the
 * document as a portable collection." */
int32_t pdfv_collection_read(const pdfv_viewer *viewer, pdfv_collection **collection);
void    pdfv_collection_free(pdfv_collection *collection);
int32_t pdfv_collection_view(const pdfv_collection *collection, uint32_t *view);
/* §12.3.5.1's outcome, and the /EmbeddedFiles key for PDFV_INITIAL_EMBEDDED — which is what
 * pdfv_extract takes. Empty for the three outcomes that name no file. */
int32_t pdfv_collection_initial(const pdfv_collection *collection, uint32_t *kind, char *out,
                                size_t cap, size_t *needed);
/* Table 155's columns, already in /O order — "[t]he relative order of the field name in the user
 * interface" — with a field stating none after every field that states one. Zero columns is a
 * PERMISSION rather than a gap: Table 153 says an absent schema lets a processor "choose useful
 * defaults that are known to exist in a file specification dictionary". */
int32_t pdfv_collection_columns(const pdfv_collection *collection, size_t *count);
int32_t pdfv_collection_column(const pdfv_collection *collection, size_t index, uint32_t *kind,
                               bool *in_the_item, int64_t *order, bool *has_order,
                               uint32_t *flags);
int32_t pdfv_collection_column_text(const pdfv_collection *collection, size_t index,
                                    uint32_t which, char *out, size_t cap, size_t *needed);
/* §12.3.5.2's folder tree, flattened depth first with a depth on each row, exactly as the outline
 * is. The /ID is "a non-negative integer value representing the unique folder identification
 * number". */
int32_t pdfv_collection_folders(const pdfv_collection *collection, size_t *count);
int32_t pdfv_collection_folder(const pdfv_collection *collection, size_t index, uint32_t *id,
                               uint32_t *depth, bool *has_thumbnail);
int32_t pdfv_collection_folder_text(const pdfv_collection *collection, size_t index,
                                    uint32_t which, char *out, size_t cap, size_t *needed);
/* Which folder an /EmbeddedFiles key names, and the file name inside it — §12.3.5.2's own grammar,
 * and the one piece of the clause you could not compute for yourself: holding a folder tree and a
 * file list is no use without it. A key naming no folder is a file at the root: PDFV_NO_ANSWER,
 * with the key copied out unchanged so that one loop does for both. Takes no viewer, because it is
 * a fact about a string. */
int32_t pdfv_collection_folder_of(const char *key, uint32_t *id, char *out, size_t cap,
                                  size_t *needed);

#ifdef __cplusplus
}
#endif

#endif /* PDF_VIEWER_H */
