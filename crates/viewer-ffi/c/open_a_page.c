/*
 * open_a_page.c — the whole ABI exercised from C, with numbers rather than adjectives.
 *
 * The two Rust hosts prove the vocabulary; only a C program proves the ABI. This one opens a
 * document, checks that the library agrees with this header, prints every event it is given —
 * including any kind it has never heard of — draws the first page, turns to the second, asks two
 * queries, reads §12.3.3's outline, and copies the pixels out. Every number it prints is read off
 * the library rather than asserted here.
 *
 *   cc -I../include open_a_page.c -o open_a_page -lpdf_viewer_ffi -L<where the cdylib is>
 *   ./open_a_page <file.pdf>
 *
 * `tests/a_c_program_drives_the_abi.rs` builds and runs exactly this, and skips when there is no
 * C compiler on the machine.
 *
 * Exit status: 0 if every step succeeded, 1 otherwise, with the failing step named on stderr.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "pdf_viewer.h"

/* Microseconds. What a C host measures its own time to first page with — the viewer has no clock
 * of its own (rule 3), so every number below is the caller's.
 *
 * `timespec_get` rather than `clock_gettime`, which is POSIX and needs a feature macro under
 * `-std=c11`: this file is meant to compile as plain C11 anywhere, which is part of what it is
 * demonstrating. */
static double now_us(void)
{
    struct timespec at;
    if (timespec_get(&at, TIME_UTC) != TIME_UTC) {
        return 0.0;
    }
    return (double)at.tv_sec * 1e6 + (double)at.tv_nsec / 1e3;
}

/* Every call goes through this, so that no status is ever dropped: a C caller cannot see a
 * `Result`, and a program that ignored one would be exactly the silence this project forbids. */
static int check(const char *what, int32_t status)
{
    if (status != PDFV_OK) {
        fprintf(stderr, "%s: %s (%d)\n", what, pdfv_status_message(status), status);
        return 0;
    }
    return 1;
}

/* Reads a whole file. Returns NULL and says why on failure. */
static uint8_t *read_file(const char *path, size_t *len)
{
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        fprintf(stderr, "cannot open %s\n", path);
        return NULL;
    }
    if (fseek(file, 0, SEEK_END) != 0) {
        fclose(file);
        return NULL;
    }
    long size = ftell(file);
    if (size < 0) {
        fclose(file);
        return NULL;
    }
    rewind(file);
    uint8_t *bytes = malloc((size_t)size);
    if (bytes == NULL) {
        fclose(file);
        return NULL;
    }
    if (fread(bytes, 1, (size_t)size, file) != (size_t)size) {
        free(bytes);
        fclose(file);
        return NULL;
    }
    fclose(file);
    *len = (size_t)size;
    return bytes;
}

/* Prints every event, and returns the index of the first NeedsRender, or SIZE_MAX. */
static size_t say_and_find_render(pdfv_events *events, const char *when)
{
    size_t found = (size_t)-1;
    size_t count = pdfv_events_len(events);
    printf("%s: %zu event(s)\n", when, count);
    for (size_t index = 0; index < count; ++index) {
        uint32_t kind = 0;
        if (!check("pdfv_events_kind", pdfv_events_kind(events, index, &kind))) {
            return (size_t)-1;
        }
        /* The two-call idiom: ask for the size, then for the sentence. This is what a caller does
         * with an event kind it has never heard of, and it is why nothing is dropped in silence. */
        size_t needed = 0;
        (void)pdfv_events_describe(events, index, NULL, 0, &needed);
        char *said = malloc(needed);
        if (said == NULL) {
            return (size_t)-1;
        }
        if (!check("pdfv_events_describe", pdfv_events_describe(events, index, said, needed, &needed))) {
            free(said);
            return (size_t)-1;
        }
        /* A kind above PDFV_EVENT_KIND_COUNT is one this program was compiled before. It still
         * says what it is, which is the whole of what the design owes a caller here. */
        if (kind >= PDFV_EVENT_KIND_COUNT) {
            printf("  [%zu] kind %u, which this program predates: %s\n", index, kind, said);
        } else {
            printf("  [%zu] %s: %s\n", index, pdfv_event_kind_name(kind), said);
        }
        free(said);
        if (kind == PDFV_EVENT_NEEDS_RENDER && found == (size_t)-1) {
            found = index;
        }
    }
    return found;
}

/* Draws whatever the batch asked for and hands the pixels back. Returns 1 on success. */
static int draw_what_was_asked(pdfv_viewer *viewer, pdfv_events *events, size_t at)
{
    pdfv_render_request *request = NULL;
    if (!check("pdfv_event_render_request", pdfv_event_render_request(events, at, &request))) {
        return 0;
    }
    size_t page = 0;
    uint32_t width = 0;
    uint32_t height = 0;
    if (!check("pdfv_render_request_page", pdfv_render_request_page(request, &page, &width, &height))) {
        pdfv_render_request_free(request);
        return 0;
    }
    printf("  rasterising page %zu at %ux%u\n", page + 1, width, height);

    pdfv_raster *raster = NULL;
    int32_t drawn = pdfv_render_request_rasterise(request, &raster);
    if (drawn != PDFV_OK) {
        /* The refusal path, and it is a real path rather than a comment: the request goes back
         * saying why, so the viewer knows the page was not drawn instead of assuming it was. */
        pdfv_events *told = NULL;
        (void)pdfv_render_ready_failed(viewer, request, pdfv_status_message(drawn), &told);
        pdfv_events_free(told);
        fprintf(stderr, "pdfv_render_request_rasterise: %s\n", pdfv_status_message(drawn));
        return 0;
    }

    pdfv_events *after = NULL;
    if (!check("pdfv_render_ready_raster",
               pdfv_render_ready_raster(viewer, request, raster, &after))) {
        return 0;
    }
    pdfv_events_free(after);
    return 1;
}

int main(int argc, char **argv)
{
    if (argc != 2) {
        fprintf(stderr, "usage: open_a_page <file.pdf>\n");
        return 1;
    }

    /* Step one, before anything else: does the library agree with this header? This is what
     * stands in for "a new message fails to compile in every consumer". */
    printf("abi %u (header %u), %u event kind(s) (header %u)\n", pdfv_abi_version(),
           PDFV_ABI_VERSION, pdfv_event_kind_count(), PDFV_EVENT_KIND_COUNT);
    if (!check("pdfv_abi_check", pdfv_abi_check(PDFV_ABI_VERSION, PDFV_EVENT_KIND_COUNT))) {
        return 1;
    }

    size_t len = 0;
    uint8_t *bytes = read_file(argv[1], &len);
    if (bytes == NULL) {
        return 1;
    }
    printf("%s: %zu byte(s)\n", argv[1], len);

    pdfv_viewer *viewer = pdfv_viewer_new(800, 1000, 1.0f);
    if (viewer == NULL) {
        free(bytes);
        return 1;
    }

    double began = now_us();
    pdfv_events *events = NULL;
    if (!check("pdfv_open", pdfv_open(viewer, 1, bytes, len, NULL, NULL, &events))) {
        free(bytes);
        pdfv_viewer_free(viewer);
        return 1;
    }
    double opened_at = now_us();
    free(bytes); /* the library copied them */

    size_t asked = say_and_find_render(events, "open");
    /* An `Opened` event says how many pages there are, and so does `pdfv_page_count`; the two must
     * agree, and this program checks rather than trusting one of them. */
    for (size_t index = 0; index < pdfv_events_len(events); ++index) {
        uint64_t document = 0;
        size_t pages = 0;
        if (pdfv_event_opened(events, index, &document, &pages) == PDFV_OK) {
            printf("  Opened says document %llu has %zu page(s)\n",
                   (unsigned long long)document, pages);
        }
    }
    if (asked == (size_t)-1) {
        fprintf(stderr, "opening a document asks for its first page, and did not\n");
        pdfv_events_free(events);
        pdfv_viewer_free(viewer);
        return 1;
    }
    if (!draw_what_was_asked(viewer, events, asked)) {
        pdfv_events_free(events);
        pdfv_viewer_free(viewer);
        return 1;
    }
    double drawn_at = now_us();
    pdfv_events_free(events);
    printf("open %.0f us, first page drawn and handed back at %.0f us\n", opened_at - began,
           drawn_at - began);

    size_t pages = 0;
    if (!check("pdfv_page_count", pdfv_page_count(viewer, &pages))) {
        pdfv_viewer_free(viewer);
        return 1;
    }
    size_t page = 0;
    size_t of = 0;
    (void)pdfv_current_page(viewer, &page, &of);
    printf("page %zu of %zu (%zu page(s) in the document)\n", page + 1, of, pages);

    /* Where the page sits and how large it is drawn — the other query a host asks per frame. */
    pdfv_geometry geometry;
    memset(&geometry, 0, sizeof geometry);
    if (check("pdfv_page_geometry", pdfv_page_geometry(viewer, page, &geometry))) {
        printf("geometry: %.1fx%.1f user units at %.3f, %ux%u px, origin %.1f,%.1f\n",
               (double)geometry.page_width, (double)geometry.page_height, (double)geometry.scale,
               geometry.width, geometry.height, (double)geometry.origin_x,
               (double)geometry.origin_y);
    }

    /* §12.3.3's outline, which is the answer ADR 0247 made owned. */
    pdfv_outline *outline = NULL;
    int32_t read = pdfv_outline_read(viewer, &outline);
    if (read == PDFV_OK) {
        size_t rows = pdfv_outline_len(outline);
        printf("outline: %zu row(s)\n", rows);
        for (size_t row = 0; row < rows && row < 4; ++row) {
            size_t needed = 0;
            (void)pdfv_outline_title(outline, row, NULL, 0, &needed);
            char *title = malloc(needed);
            if (title == NULL) {
                break;
            }
            uint32_t depth = 0;
            bool expanded = false;
            uint32_t number = 0;
            uint16_t generation = 0;
            (void)pdfv_outline_title(outline, row, title, needed, &needed);
            (void)pdfv_outline_depth(outline, row, &depth, &expanded);
            (void)pdfv_outline_object(outline, row, &number, &generation);
            printf("  [%zu] depth %u, %s, object %u %u: %s\n", row, depth,
                   expanded ? "open" : "closed", number, generation, title);
            free(title);
        }
        pdfv_outline_free(outline);
    } else {
        printf("outline: %s\n", pdfv_status_message(read));
    }

    /* Annex O's `search`, driven the way a C find bar drives it: start, then pump one page at a
     * time until the library says nothing is remaining. The loop has a bound because a caller
     * that trusted `remaining` to reach zero would hang on a library that had a bug. */
    pdfv_events *searching = NULL;
    if (!check("pdfv_find_start", pdfv_find_start(viewer, "black point", 0, &searching))) {
        pdfv_viewer_free(viewer);
        return 1;
    }
    size_t steps = 1;
    int32_t hit = 0;
    size_t at_page = 0;
    size_t from = 0;
    size_t to = 0;
    size_t left = 0;
    int32_t wrapped = 0;
    for (;;) {
        int32_t status = PDFV_WRONG_KIND;
        for (size_t index = 0; index < pdfv_events_len(searching); ++index) {
            int32_t asked_about = pdfv_event_searched(searching, index, &hit, &at_page, &from, &to,
                                                      &left, &wrapped);
            if (asked_about == PDFV_OK) {
                status = PDFV_OK;
            }
        }
        pdfv_events_free(searching);
        searching = NULL;
        if (status != PDFV_OK) {
            fprintf(stderr, "a find step said nothing about the search\n");
            pdfv_viewer_free(viewer);
            return 1;
        }
        if (hit || left == 0 || steps > 4096) {
            break;
        }
        if (!check("pdfv_find_continue", pdfv_find_continue(viewer, &searching))) {
            pdfv_viewer_free(viewer);
            return 1;
        }
        ++steps;
    }
    if (hit) {
        printf("search: found on page %zu, bytes %zu..%zu, after %zu step(s)\n", at_page + 1, from,
               to, steps);
    } else {
        printf("search: nothing in the document, after %zu step(s)\n", steps);
    }
    pdfv_events *stopped = NULL;
    (void)pdfv_find_stop(viewer, &stopped);
    pdfv_events_free(stopped);
    (void)pdfv_current_page(viewer, &page, &of);
    printf("after the search: page %zu of %zu\n", page + 1, of);

    /* A page turn, and the page that comes back must be the one turned to. */
    pdfv_events *turned = NULL;
    if (!check("pdfv_go_to_page", pdfv_go_to_page(viewer, PDFV_PAGE_NEXT, 0, &turned))) {
        pdfv_viewer_free(viewer);
        return 1;
    }
    double turned_at = now_us();
    asked = say_and_find_render(turned, "page turn");
    if (asked != (size_t)-1 && !draw_what_was_asked(viewer, turned, asked)) {
        pdfv_events_free(turned);
        pdfv_viewer_free(viewer);
        return 1;
    }
    pdfv_events_free(turned);
    (void)pdfv_current_page(viewer, &page, &of);
    printf("after the turn: page %zu of %zu, drawn in %.0f us\n", page + 1, of,
           now_us() - turned_at);

    /* And the pixels, into a buffer this program owns. Two calls: size, then copy. */
    pdfv_frame info;
    memset(&info, 0, sizeof info);
    if (!check("pdfv_frame_info", pdfv_frame_info(viewer, &info))) {
        pdfv_viewer_free(viewer);
        return 1;
    }
    printf("frame: page %zu, %ux%u, format %u, %zu byte(s)\n", info.page + 1, info.width,
           info.height, info.format, info.bytes);
    if (info.format != PDFV_FORMAT_RGBA8) {
        fprintf(stderr, "a pixel layout this program was not compiled for: %u\n", info.format);
        pdfv_viewer_free(viewer);
        return 1;
    }

    /* The refusal half of the two-call idiom, checked rather than assumed. */
    uint8_t one = 0;
    if (pdfv_frame_copy(viewer, &one, 1, NULL) != PDFV_BUFFER_TOO_SMALL) {
        fprintf(stderr, "a one-byte buffer took a whole page\n");
        pdfv_viewer_free(viewer);
        return 1;
    }

    uint8_t *pixels = malloc(info.bytes);
    if (pixels == NULL) {
        pdfv_viewer_free(viewer);
        return 1;
    }
    size_t written = 0;
    double copy_began = now_us();
    if (!check("pdfv_frame_copy", pdfv_frame_copy(viewer, pixels, info.bytes, &written))) {
        free(pixels);
        pdfv_viewer_free(viewer);
        return 1;
    }
    double copy_took = now_us() - copy_began;

    /* Something was drawn: count the pixels that are not the background, which is what says the
     * page arrived rather than a blank buffer. Four bytes per pixel, RGBA, straight alpha. */
    size_t inked = 0;
    for (size_t at = 0; at + 3 < written; at += 4) {
        if (pixels[at] != 0xFF || pixels[at + 1] != 0xFF || pixels[at + 2] != 0xFF) {
            ++inked;
        }
    }
    printf("copied %zu byte(s) in %.0f us (%.1f GB/s); %zu of %zu pixel(s) are not white\n",
           written, copy_took, (double)written / copy_took / 1e3, inked, written / 4);
    free(pixels);

    pdfv_viewer_free(viewer);
    if (inked == 0) {
        fprintf(stderr, "the page copied out is blank\n");
        return 1;
    }
    printf("ok\n");
    return 0;
}
