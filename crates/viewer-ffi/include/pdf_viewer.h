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
