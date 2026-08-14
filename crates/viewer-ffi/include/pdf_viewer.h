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
 * The revision of everything below that is passed BY VALUE, which is `pdfv_geometry` and
 * `pdfv_frame` and nothing else. A function added later is a symbol an old caller never looks
 * up; a status or an event kind added later is a number an old caller has a `default:` arm for.
 * A field added to one of those two structs is a size an old caller has already compiled, and no
 * diagnostic anywhere would catch it — so this number moves when one of them does.
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

/* ------------------------------------------------------------------------------------------- */
/* The two structs passed by value.                                                              */
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
int32_t pdfv_frame_info(const pdfv_viewer *viewer, pdfv_frame *info);
/* One copy — what tier 1 costs everywhere in this project. Size `into` from info.bytes. */
int32_t pdfv_frame_copy(const pdfv_viewer *viewer, uint8_t *into, size_t cap, size_t *written);
int32_t pdfv_dirty(const pdfv_viewer *viewer, bool *dirty);
/* §12.5.6.5: whether activating here would follow a link. Asked on every pointer move. */
int32_t pdfv_link_at(const pdfv_viewer *viewer, float x, float y, bool *link);
/* What the page could not draw — the same sentences a REPORTED carried, kept so a caller that
 * cleared its status bar can ask again rather than remembering. */
int32_t pdfv_reports_len(const pdfv_viewer *viewer, size_t *count);
int32_t pdfv_report(const pdfv_viewer *viewer, size_t index, char *out, size_t cap,
                    size_t *needed);

/* ------------------------------------------------------------------------------------------- */
/* Geometry: what a caller draws over the page in its own colours.                               */
/*                                                                                               */
/* Interactive chrome crosses as SHAPES rather than as pixels, so that a selection is drawn in    */
/* macOS's selection colour, KDE's accent or the Windows highlight brush — and so that a drag     */
/* never forces the page to be drawn again.                                                      */
/* ------------------------------------------------------------------------------------------- */

int32_t pdfv_selection_text(const pdfv_viewer *viewer, char *out, size_t cap, size_t *needed);
int32_t pdfv_selection_quads(const pdfv_viewer *viewer, pdfv_quads **quads);
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

#ifdef __cplusplus
}
#endif

#endif /* PDF_VIEWER_H */
