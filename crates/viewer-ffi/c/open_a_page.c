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
 *   ./open_a_page <file.pdf> [<form.pdf>]
 *
 * The second argument is optional and is where §12.7's half runs: a form has to come from a
 * document that has one, and the five-page application note the first argument names does not.
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

/* §8.11.4.3's layers and §7.11.4's files, printed. Both are the same handle. */
static void say_panel(const char *what, pdfv_panel *panel)
{
    size_t rows = pdfv_panel_len(panel);
    printf("%s: %zu row(s)\n", what, rows);
    for (size_t row = 0; row < rows && row < 4; ++row) {
        size_t needed = 0;
        (void)pdfv_panel_text(panel, row, 0, NULL, 0, &needed);
        char *label = malloc(needed);
        if (label == NULL) {
            return;
        }
        (void)pdfv_panel_text(panel, row, 0, label, needed, &needed);
        uint32_t kind = 0;
        uint32_t number = 0;
        uint16_t generation = 0;
        bool on = false;
        bool locked = false;
        uint32_t depth = 0;
        bool expanded = false;
        (void)pdfv_panel_action(panel, row, &kind, &number, &generation, &on, &locked);
        (void)pdfv_panel_depth(panel, row, &depth, &expanded);
        printf("  [%zu] depth %u, %s, object %u %u, on %d, locked %d: %s\n", row, depth,
               pdfv_row_kind_name(kind), number, generation, on ? 1 : 0, locked ? 1 : 0, label);
        free(label);
    }
}

/* One of a field's strings, printed inline. Returns 1 when there was one. */
static int say_field_string(const pdfv_fields *fields, size_t field, uint32_t which,
                            const char *label)
{
    size_t needed = 0;
    int32_t asked = pdfv_field_name(fields, field, which, NULL, 0, &needed);
    if (asked != PDFV_BUFFER_TOO_SMALL && asked != PDFV_OK) {
        return 0;
    }
    char *text = malloc(needed);
    if (text == NULL) {
        return 0;
    }
    if (pdfv_field_name(fields, field, which, text, needed, &needed) == PDFV_OK) {
        printf(" %s=%s", label, text);
    }
    free(text);
    return 1;
}

/*
 * §12.7's form, walked and then *changed*: the one thing a C caller could not do at all before the
 * five-hundred-and-eleventh session. Returns 1 on success.
 *
 * The interesting step is the last: a check box's value is the name Table 170's appearance
 * dictionary is keyed by, and those names are the file's own invention — so a caller has to be told
 * one, which is what `pdfv_field_widget_text(PDFV_TEXT_LABEL)` is for. Sending a guess would tick
 * nothing.
 */
static int exercise_the_form(pdfv_viewer *viewer, const char *path)
{
    size_t len = 0;
    uint8_t *bytes = read_file(path, &len);
    if (bytes == NULL) {
        return 0;
    }
    pdfv_events *events = NULL;
    if (!check("pdfv_open (form)", pdfv_open(viewer, 2, bytes, len, NULL, NULL, &events))) {
        free(bytes);
        return 0;
    }
    free(bytes);
    pdfv_events_free(events);

    pdfv_fields *fields = NULL;
    if (!check("pdfv_fields_read", pdfv_fields_read(viewer, &fields))) {
        return 0;
    }
    size_t count = pdfv_fields_len(fields);
    printf("form: %zu field(s)\n", count);

    /* The check box to tick, and the name that ticks it, both learned from the library. */
    char *ticks = NULL;
    char *ticked_field = NULL;
    for (size_t field = 0; field < count; ++field) {
        uint32_t kind = 0;
        uint32_t flags = 0;
        (void)pdfv_field_control(fields, field, &kind, &flags);
        printf("  [%zu] %s flags %u", field, pdfv_control_kind_name(kind), flags);
        (void)say_field_string(fields, field, PDFV_TEXT_QUALIFIED, "name");
        (void)say_field_string(fields, field, PDFV_TEXT_SHOWN, "shown");
        size_t widgets = 0;
        (void)pdfv_field_widget_count(fields, field, &widgets);
        size_t options = 0;
        (void)pdfv_field_option_count(fields, field, &options);
        printf(" widgets=%zu options=%zu", widgets, options);
        size_t needed = 0;
        int32_t has_value = pdfv_field_value(fields, field, NULL, 0, &needed);
        if (has_value == PDFV_NO_ANSWER) {
            printf(" value=<none>");
        } else {
            char *value = malloc(needed);
            if (value != NULL) {
                if (pdfv_field_value(fields, field, value, needed, &needed) == PDFV_OK) {
                    printf(" value=%s", value);
                }
                free(value);
            }
        }
        printf("\n");
        for (size_t widget = 0; widget < widgets; ++widget) {
            uint32_t number = 0;
            uint16_t generation = 0;
            float quad[8] = {0};
            bool on = false;
            (void)pdfv_field_widget(fields, field, widget, &number, &generation, quad, &on);
            printf("    widget %u %u at %.1f,%.1f on=%d\n", number, generation, (double)quad[0],
                   (double)quad[1], on ? 1 : 0);
            if (kind != PDFV_CONTROL_CHECK || ticks != NULL || on) {
                continue;
            }
            size_t state_needed = 0;
            (void)pdfv_field_widget_text(fields, field, widget, PDFV_TEXT_LABEL, NULL, 0,
                                         &state_needed);
            char *state = malloc(state_needed);
            size_t name_needed = 0;
            (void)pdfv_field_name(fields, field, PDFV_TEXT_QUALIFIED, NULL, 0, &name_needed);
            char *name = malloc(name_needed);
            if (state == NULL || name == NULL) {
                free(state);
                free(name);
                continue;
            }
            if (pdfv_field_widget_text(fields, field, widget, PDFV_TEXT_LABEL, state,
                                       state_needed, &state_needed) == PDFV_OK
                && pdfv_field_name(fields, field, PDFV_TEXT_QUALIFIED, name, name_needed,
                                   &name_needed) == PDFV_OK) {
                ticks = state;
                ticked_field = name;
            } else {
                free(state);
                free(name);
            }
        }
    }
    pdfv_fields_free(fields);

    if (ticks == NULL) {
        fprintf(stderr, "the form fixture has no check box to tick\n");
        return 0;
    }
    printf("ticking %s with the state %s\n", ticked_field, ticks);
    pdfv_events *edited = NULL;
    int32_t sent = pdfv_set_field_text(viewer, ticked_field, ticks, &edited);
    free(ticks);
    free(ticked_field);
    if (!check("pdfv_set_field_text", sent)) {
        return 0;
    }
    pdfv_events_free(edited);

    /* And read it back, which is the rule a host follows after every edit: the field's own answer
     * is the truth, never the string that was sent. */
    if (!check("pdfv_fields_read (again)", pdfv_fields_read(viewer, &fields))) {
        return 0;
    }
    size_t on_now = 0;
    for (size_t field = 0; field < pdfv_fields_len(fields); ++field) {
        size_t widgets = 0;
        (void)pdfv_field_widget_count(fields, field, &widgets);
        for (size_t widget = 0; widget < widgets; ++widget) {
            bool on = false;
            (void)pdfv_field_widget(fields, field, widget, NULL, NULL, NULL, &on);
            if (on) {
                ++on_now;
            }
        }
    }
    pdfv_fields_free(fields);
    printf("after the edit: %zu widget(s) on\n", on_now);

    bool dirty = false;
    (void)pdfv_dirty(viewer, &dirty);
    printf("dirty after the edit: %d\n", dirty ? 1 : 0);

    /* §7.5.6's incremental update: the producer's bytes with the edit appended. */
    pdfv_events *saved = NULL;
    if (!check("pdfv_save", pdfv_save(viewer, &saved))) {
        return 0;
    }
    size_t written = 0;
    for (size_t index = 0; index < pdfv_events_len(saved); ++index) {
        size_t needed = 0;
        if (pdfv_event_bytes(saved, index, NULL, 0, &needed) == PDFV_BUFFER_TOO_SMALL) {
            written = needed;
        }
    }
    pdfv_events_free(saved);
    printf("saved %s than the file it came from: %zu against %zu byte(s)\n",
           written > len ? "more" : "no more", written, len);

    /* And the undo comes *after* the save above, which is why this says 1 rather than 0: the
       edit is in a file now, so taking it back is itself something the file does not have. What
       is unsaved is the distance between the log's cursor and the last save, in either
       direction. */
    pdfv_events *undone = NULL;
    (void)pdfv_undo(viewer, &undone);
    pdfv_events_free(undone);
    (void)pdfv_dirty(viewer, &dirty);
    printf("dirty after the undo: %d\n", dirty ? 1 : 0);

    pdfv_events *closed = NULL;
    (void)pdfv_close(viewer, 2, &closed);
    pdfv_events_free(closed);
    return written > len;
}

int main(int argc, char **argv)
{
    if (argc < 2 || argc > 3) {
        fprintf(stderr, "usage: open_a_page <file.pdf> [<form.pdf>]\n");
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
    /* Table 29's arrangement, counted: one under PDFV_LAYOUT_SINGLE_PAGE, which is what this
     * document opens in, and the index below is into that list. */
    size_t frames = pdfv_frame_count(viewer);
    printf("frames on the screen: %zu\n", frames);
    if (frames != 1) {
        fprintf(stderr, "a single-page arrangement showing %zu page(s)\n", frames);
        pdfv_viewer_free(viewer);
        return 1;
    }
    if (!check("pdfv_frame_info", pdfv_frame_info(viewer, 0, &info))) {
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
    if (pdfv_frame_copy(viewer, 0, &one, 1, NULL) != PDFV_BUFFER_TOO_SMALL) {
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
    if (!check("pdfv_frame_copy", pdfv_frame_copy(viewer, 0, pixels, info.bytes, &written))) {
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
    if (inked == 0) {
        fprintf(stderr, "the page copied out is blank\n");
        pdfv_viewer_free(viewer);
        return 1;
    }

    /* --------------------------------------------------------------------------------------- */
    /* Everything the four-hundred-and-eleventh session left out of the ABI.                     */
    /* --------------------------------------------------------------------------------------- */

    /* The two enumerations this library answers with but does not push, and the name it gives a
     * number it does not define — which is what a caller compiled before a variant gets. */
    printf("control kinds %u (header %u), row kinds %u (header %u), unknown is %s\n",
           pdfv_control_kind_count(), PDFV_CONTROL_KIND_COUNT, pdfv_row_kind_count(),
           PDFV_ROW_KIND_COUNT, pdfv_control_kind_name(PDFV_CONTROL_KIND_COUNT));

    /* §12.5.5's pointer, and the question a caller asks on every move of it. */
    pdfv_events *moved = NULL;
    if (!check("pdfv_pointer", pdfv_pointer(viewer, 100.0f, 100.0f, PDFV_POINTER_MOVED, &moved))) {
        pdfv_viewer_free(viewer);
        return 1;
    }
    pdfv_events_free(moved);
    bool over_a_link = false;
    (void)pdfv_link_at(viewer, 100.0f, 100.0f, &over_a_link);

    /* A selection, as text and as shapes. The shapes are what a caller draws in its own colour;
     * the whole reason they are not baked into the frame. */
    pdfv_events *selected = NULL;
    if (!check("pdfv_select", pdfv_select(viewer, PDFV_SELECT_ALL, &selected))) {
        pdfv_viewer_free(viewer);
        return 1;
    }
    pdfv_events_free(selected);
    size_t text_needed = 0;
    (void)pdfv_selection_text(viewer, NULL, 0, &text_needed);
    pdfv_quads *quads = NULL;
    size_t shapes = 0;
    if (pdfv_selection_quads(viewer, &quads) == PDFV_OK) {
        shapes = pdfv_quads_len(quads);
        float one[8] = {0};
        if (shapes > 0) {
            (void)pdfv_quads_get(quads, 0, one);
        }
        printf("selection: %zu byte(s) over %zu shape(s), first at %.1f,%.1f; over a link: %d\n",
               text_needed > 0 ? text_needed - 1 : 0, shapes, (double)one[0], (double)one[1],
               over_a_link ? 1 : 0);
        pdfv_quads_free(quads);
    }

    /* The other two panels. A document stating neither answers an empty list rather than
     * PDFV_NO_ANSWER — the question was answered — and PDFV_NO_ANSWER is what comes back when no
     * document is focused at all. Both paths are printed, because a caller has to tell them
     * apart: one is a document with no layers and the other is no document. */
    pdfv_panel *panel = NULL;
    int32_t layers = pdfv_layers_read(viewer, &panel);
    if (layers == PDFV_OK) {
        say_panel("layers", panel);
        pdfv_panel_free(panel);
    } else {
        printf("layers: %s\n", pdfv_status_message(layers));
    }
    int32_t files = pdfv_attachments_read(viewer, &panel);
    if (files == PDFV_OK) {
        say_panel("attachments", panel);
        pdfv_panel_free(panel);
    } else {
        printf("attachments: %s\n", pdfv_status_message(files));
    }

    /* What the pages on the screen could not draw, which every layer of this library says out
     * loud — one entry per page Table 29's arrangement is showing, because a column shows
     * several and a note about one of them is not a note about the others. */
    size_t reported = pdfv_reported_pages(viewer);
    printf("reported pages: %zu\n", reported);
    for (size_t entry = 0; entry < reported; ++entry) {
        size_t page = 0;
        size_t reports = 0;
        (void)pdfv_reported_page(viewer, entry, &page);
        (void)pdfv_reports_len(viewer, entry, &reports);
        printf("reports on page %zu: %zu\n", page + 1, reports);
        for (size_t index = 0; index < reports && index < 3; ++index) {
            size_t needed = 0;
            (void)pdfv_report(viewer, entry, index, NULL, 0, &needed);
            char *note = malloc(needed);
            if (note == NULL) {
                break;
            }
            if (pdfv_report(viewer, entry, index, note, needed, &needed) == PDFV_OK) {
                printf("  %s\n", note);
            }
            free(note);
        }
    }

    /* The three policy values and the clock, each of which only a host can supply. None of them
     * has to produce an event, and none of these is checked for one: what is being demonstrated is
     * that a C caller can *say* them. */
    pdfv_events *said = NULL;
    if (!check("pdfv_restrict", pdfv_restrict(viewer, PDFV_RESTRICT_OFF, &said))) {
        pdfv_viewer_free(viewer);
        return 1;
    }
    pdfv_events_free(said);
    if (!check("pdfv_delegate", pdfv_delegate(viewer, PDFV_DELEGATE_DELEGATED, &said))) {
        pdfv_viewer_free(viewer);
        return 1;
    }
    pdfv_events_free(said);
    if (!check("pdfv_present", pdfv_present(viewer, PDFV_PRESENT_ON, &said))) {
        pdfv_viewer_free(viewer);
        return 1;
    }
    pdfv_events_free(said);
    if (!check("pdfv_tick", pdfv_tick(viewer, 1000, &said))) {
        pdfv_viewer_free(viewer);
        return 1;
    }
    /* A page with no /Dur swallows every tick — "[i]f no Dur entry is specified in the page
     * object, the page shall not advance automatically" — so the count printed here is the
     * clause's answer and not a defect. */
    printf("policy: restrict, delegate, present and one tick produced %zu event(s)\n",
           pdfv_events_len(said));
    pdfv_events_free(said);
    (void)pdfv_present(viewer, PDFV_PRESENT_OFF, &said);
    pdfv_events_free(said);

    /* And a number this program refuses to invent: an enumeration this ABI *takes* says so. */
    pdfv_events *refused = NULL;
    if (pdfv_pointer(viewer, 0.0f, 0.0f, 99u, &refused) != PDFV_WRONG_KIND) {
        fprintf(stderr, "a pointer action this build does not define was accepted\n");
        pdfv_viewer_free(viewer);
        return 1;
    }
    printf("an undefined pointer action: %s\n", pdfv_status_message(PDFV_WRONG_KIND));

    /* §12.7's form, on the document that has one. */
    if (argc == 3 && !exercise_the_form(viewer, argv[2])) {
        pdfv_viewer_free(viewer);
        return 1;
    }

    pdfv_viewer_free(viewer);
    printf("ok\n");
    return 0;
}
