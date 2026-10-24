/*
 * drive_a_window.c — quorra_window driven from C: the loop a toolkit host runs, one call per thing
 * that happened to the window (RFC 0010 section 3.2a, ADRs 2088 and 2104).
 *
 * Where open_a_page.c answers every viewer-core event itself, this program answers none: the
 * window holds the loop the three toolkit windows share, and what it owes the program — a
 * sentence, a repaint, a title, a question — arrives as a batch after each call. So the event loop
 * below is a resize, a timer, some keys and an answer, and each of them is one call followed by
 * the same `act_on_what_is_owed`.
 *
 *   ./drive_a_window <file.pdf> <form.pdf> <script.pdf> <directory> <audience.pdf> <placed.pdf>
 *
 * The second document is the fixture `tests/a_c_program_drives_the_abi.rs` writes: a push-button
 * whose §12.7.6.2 submission the reader's default level asks about, which is how the question
 * and its answer are exercised — and, opened again under levels this program states, how a level
 * a host sets is shown to be the one in force (ADR 2120). The third is a text field whose
 * validation asks with app.alert, which a window at the scripts' PROCEED level puts to this
 * program as a record (ADR 2121). The fourth is a directory holding one file that is neither a
 * certificate nor a PDF (ADR 2156), the fifth one layer for a named reader (ADR 2173), and the
 * sixth a page stating Table 147's /FitWindow and /CenterWindow (ADR 2308).
 *
 * Exit status: 0 if every step succeeded, 1 otherwise, with the failing step named on stderr.
 */

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "quorra.h"

/* Every call goes through this, so that no status is dropped. */
static int check(const char *what, int32_t status)
{
    if (status != QUORRA_OK) {
        fprintf(stderr, "%s: %s (%d)\n", what, quorra_status_message(status), status);
        return 0;
    }
    return 1;
}

/* Waits `millis` milliseconds on the C11 clock: a toolkit's timer, without a toolkit. */
static void wait_ms(int32_t millis)
{
    struct timespec start;
    struct timespec at;
    if (timespec_get(&start, TIME_UTC) != TIME_UTC) {
        return;
    }
    do {
        if (timespec_get(&at, TIME_UTC) != TIME_UTC) {
            return;
        }
    } while ((double)(at.tv_sec - start.tv_sec) * 1e3
                 + (double)(at.tv_nsec - start.tv_nsec) / 1e6
             < (double)millis);
}

/* What the program saw in the batches it took, kept for the steps that ask about it. */
struct seen {
    int repaints;
    uint32_t repainted;
    size_t fresh;
    int questions;
    int panels;
    int controls;
    int popups;
    int strips;
    uint64_t prompted;
    int archives;
    size_t archive_target;
    size_t archive_profile;
    int scripts;
    int withdrawn;
    uint64_t script_document;
    uint32_t script_icon;
    uint32_t script_buttons;
    uint32_t script_dismissed;
    char script_message[128];
    int prints;
    size_t print_pages;
    bool print_degraded;
    char title[256];
    char said[512];
    char question[512];
    /* A window of this program's own for Table 147's placement: its inner size, which is the
     * viewport and a chrome of PLACED_CHROME below it, and what quorra_window_place asked of it.
     * A zero width is a scenario that measures no window, and the record is only counted. */
    int32_t inner_width;
    int32_t inner_height;
    int resizes;
    int moves;
    int32_t moved_x;
    int32_t moved_y;
};

/* The pretend window's chrome: a frame of ten by thirty and a bar forty tall under the page area,
 * on a 1920 by 1080 work area at the origin. */
#define PLACED_CHROME 40

/* Takes what the window owes and acts on each record in order, as a toolkit window would: a
 * sentence on the status line (here, standard output), the title bar, the page area drawn again.
 * A kind this program has no arm for is named rather than dropped. */
static int act_on_what_is_owed(quorra_window *window, struct seen *seen)
{
    quorra_owed *owed = NULL;
    if (!check("quorra_window_take_owed", quorra_window_take_owed(window, &owed))) {
        return 0;
    }
    for (size_t index = 0; index < quorra_owed_count(owed); ++index) {
        uint32_t kind = 0;
        if (!check("quorra_owed_kind", quorra_owed_kind(owed, index, &kind))) {
            quorra_owed_free(owed);
            return 0;
        }
        /* C's two-call idiom: the size first, then the text into a buffer of that size. A kind
         * that carries no text answers QUORRA_WRONG_KIND, and its text is empty. */
        size_t needed = 0;
        int32_t sized = quorra_owed_text(owed, index, NULL, 0, &needed);
        char *text = calloc(sized == QUORRA_BUFFER_TOO_SMALL ? needed : 1, 1);
        if (text == NULL || (sized != QUORRA_BUFFER_TOO_SMALL && sized != QUORRA_WRONG_KIND)
            || (sized == QUORRA_BUFFER_TOO_SMALL
                && !check("quorra_owed_text", quorra_owed_text(owed, index, text, needed, NULL)))) {
            free(text);
            quorra_owed_free(owed);
            return 0;
        }
        switch (kind) {
        case QUORRA_OWED_SAID:
            printf("said: %s\n", text);
            snprintf(seen->said, sizeof seen->said, "%s", text);
            break;
        case QUORRA_OWED_TITLE:
            snprintf(seen->title, sizeof seen->title, "%s", text);
            break;
        case QUORRA_OWED_REPAINT: {
            /* What changed, and which pictures are new: a toolkit copies only those. */
            uint32_t changed = 0;
            bool fresh = false;
            seen->repaints += 1;
            if (!check("quorra_owed_repaint", quorra_owed_repaint(owed, index, &changed))) {
                free(text);
                quorra_owed_free(owed);
                return 0;
            }
            seen->repainted |= changed;
            for (size_t frame = 0;
                 quorra_owed_repaint_fresh(owed, index, frame, &fresh) == QUORRA_OK; ++frame) {
                seen->fresh += fresh ? 1 : 0;
            }
            break;
        }
        case QUORRA_OWED_QUESTION:
            seen->questions += 1;
            snprintf(seen->question, sizeof seen->question, "%s", text);
            break;
        case QUORRA_OWED_PASSWORD:
            if (!check("quorra_owed_document", quorra_owed_document(owed, index, &seen->prompted))) {
                free(text);
                quorra_owed_free(owed);
                return 0;
            }
            break;
        case QUORRA_OWED_PANELS:
            seen->panels += 1;
            break;
        case QUORRA_OWED_CONTROLS:
            seen->controls += 1;
            break;
        case QUORRA_OWED_POPUPS:
            seen->popups += 1;
            break;
        case QUORRA_OWED_STRIP:
            seen->strips += 1;
            break;
        case QUORRA_OWED_ARCHIVE:
            seen->archives += 1;
            printf("archive chooser: %s\n", text);
            if (!check("quorra_owed_archive",
                       quorra_owed_archive(owed, index, &seen->archive_target,
                                           &seen->archive_profile))) {
                free(text);
                quorra_owed_free(owed);
                return 0;
            }
            break;
        case QUORRA_OWED_SCRIPT: {
            uint32_t asked = 0;
            size_t length = 0;
            seen->scripts += 1;
            printf("script dialogue: %s\n", text);
            if (!check("quorra_owed_document",
                       quorra_owed_document(owed, index, &seen->script_document))
                || !check("quorra_owed_script_question",
                          quorra_owed_script_question(owed, index, &asked, &seen->script_icon,
                                                      &seen->script_buttons,
                                                      &seen->script_dismissed, NULL))
                || !check("quorra_owed_script_text",
                          quorra_owed_script_text(owed, index, QUORRA_SCRIPT_TEXT_MESSAGE,
                                                  seen->script_message,
                                                  sizeof seen->script_message, &length))
                || asked != QUORRA_SCRIPT_QUESTION_ALERT) {
                free(text);
                quorra_owed_free(owed);
                return 0;
            }
            break;
        }
        case QUORRA_OWED_SCRIPT_WITHDRAWN:
            seen->withdrawn += 1;
            break;
        case QUORRA_OWED_PRINT:
            seen->prints += 1;
            printf("print: %s\n", text);
            if (!check("quorra_owed_print",
                       quorra_owed_print(owed, index, &seen->print_pages, &seen->print_degraded))) {
                free(text);
                quorra_owed_free(owed);
                return 0;
            }
            break;
        case QUORRA_OWED_CLIPBOARD:
        case QUORRA_OWED_BEEP:
            break;
        case QUORRA_OWED_CHROME: {
            int32_t panel = -99;
            int32_t status = quorra_owed_chrome_panel(owed, index, &panel);
            if ((status != QUORRA_OK && status != QUORRA_NO_ANSWER)
                || (status == QUORRA_NO_ANSWER && panel != -99)
                || (status == QUORRA_OK && (panel < -1 || panel >= (int32_t)quorra_tab_count()))
                || quorra_owed_chrome_panel(owed, index, NULL) != QUORRA_NULL_ARGUMENT
                || quorra_owed_chrome_panel(NULL, index, &panel) != QUORRA_NULL_ARGUMENT) {
                free(text);
                quorra_owed_free(owed);
                return 0;
            }
            break;
        }
        case QUORRA_OWED_PLACE: {
            if (seen->inner_width == 0) {
                break;
            }
            quorra_window_extents extents = {seen->inner_width, seen->inner_height, 10, 30, 0, 0,
                                             1920, 1080, 1u};
            quorra_window_placement placed = {0};
            int32_t status = quorra_window_place(window, &extents, &placed);
            if (status == QUORRA_NO_ANSWER) {
                break;
            }
            if (!check("quorra_window_place", status)
                || quorra_window_place(window, &extents, NULL) != QUORRA_NULL_ARGUMENT) {
                free(text);
                quorra_owed_free(owed);
                return 0;
            }
            if (placed.resize) {
                seen->resizes += 1;
                seen->inner_width = placed.width;
                seen->inner_height = placed.height;
                if (!check("quorra_window_resize",
                           quorra_window_resize(window, (uint32_t)placed.width,
                                                (uint32_t)(placed.height - PLACED_CHROME),
                                                1.0f))) {
                    free(text);
                    quorra_owed_free(owed);
                    return 0;
                }
            }
            if (placed.move_to) {
                seen->moves += 1;
                seen->moved_x = placed.x;
                seen->moved_y = placed.y;
            }
            break;
        }
        default:
            printf("owed a kind this program has no arm for: %s\n", quorra_owed_kind_name(kind));
            break;
        }
        free(text);
    }
    quorra_owed_free(owed);
    return 1;
}

/* The window's timer: ticked at the interval it asks for, until it asks for none — bounded, so a
 * window that never settles fails the program rather than hanging it. */
static int run_the_timer(quorra_window *window, struct seen *seen)
{
    for (int ticks = 0; ticks < 20000; ++ticks) {
        int32_t interval = quorra_window_interval(window);
        if (interval < 0) {
            return 1;
        }
        wait_ms(interval);
        if (!check("quorra_window_tick", quorra_window_tick(window))
            || !act_on_what_is_owed(window, seen)) {
            return 0;
        }
    }
    fprintf(stderr, "the window never stopped asking for its timer\n");
    return 0;
}

/* One key, and what the window owed for it. */
static int press(quorra_window *window, uint32_t key, bool command, uint32_t *act,
                 struct seen *seen)
{
    float by = 0.0f;
    return check("quorra_window_press", quorra_window_press(window, key, false, command, act, &by))
           && act_on_what_is_owed(window, seen) && run_the_timer(window, seen);
}

/* The new output is an additive, fixed-layout snapshot; null pointers never write it. */
static int chrome_contract(quorra_window *window)
{
    quorra_window_chrome_state chrome = {7, 7, 7, 7};
    if (quorra_window_chrome(NULL, &chrome) != QUORRA_NULL_ARGUMENT
        || chrome.menu_bar != 7 || chrome.other_windows != 7
        || quorra_window_chrome(window, NULL) != QUORRA_NULL_ARGUMENT
        || !check("quorra_window_chrome", quorra_window_chrome(window, &chrome))) return 0;
    printf("window chrome: %u %u %u %u\n", chrome.menu_bar, chrome.tool_bar,
           chrome.window_ui, chrome.other_windows);
    return sizeof chrome == 16 && chrome.menu_bar <= 1 && chrome.tool_bar <= 1
        && chrome.window_ui <= 1 && chrome.other_windows <= 1;
}

/* Copies the page area's first picture out and counts the pixels that are not white. */
static int copy_the_page(quorra_window *window)
{
    quorra_frame frame;
    uint32_t shown_width = 0;
    uint32_t shown_height = 0;
    size_t count = quorra_window_frame_count(window);
    if (count == 0) {
        fprintf(stderr, "the window shows no picture\n");
        return 0;
    }
    if (!check("quorra_window_frame",
               quorra_window_frame(window, 0, &frame, &shown_width, &shown_height))) {
        return 0;
    }
    uint8_t *pixels = malloc(frame.bytes);
    size_t written = 0;
    if (pixels == NULL
        || !check("quorra_window_frame_copy",
                  quorra_window_frame_copy(window, 0, pixels, frame.bytes, &written))) {
        free(pixels);
        return 0;
    }
    size_t inked = 0;
    for (size_t at = 0; at + 3 < written; at += 4) {
        if (pixels[at] != 255 || pixels[at + 1] != 255 || pixels[at + 2] != 255) {
            ++inked;
        }
    }
    free(pixels);
    printf("window frame: %zu picture(s), page %zu, %ux%u shown %ux%u, %zu byte(s); %zu inked\n",
           count, frame.page + 1, frame.width, frame.height, shown_width, shown_height, written,
           inked);
    return check("quorra_window_presented", quorra_window_presented(window));
}

/* The panels' rows, read on the PANELS record the open owed, through the handles the vocabulary
 * layer hands over: the outline, the information, the page list's count, label and thumbnail. */
static int read_the_panels(quorra_window *window, struct seen *seen)
{
    quorra_outline *outline = NULL;
    quorra_panel *properties = NULL;
    size_t pages = 0;
    if (!check("quorra_window_outline_read", quorra_window_outline_read(window, &outline))
        || !check("quorra_window_properties_read",
                  quorra_window_properties_read(window, &properties))
        || !check("quorra_window_page_count", quorra_window_page_count(window, &pages))) {
        quorra_outline_free(outline);
        quorra_panel_free(properties);
        return 0;
    }
    char label[64] = {0};
    size_t needed = 0;
    int32_t labelled = quorra_window_page_label(window, 0, label, sizeof label, &needed);
    quorra_thumbnail *thumbnail = NULL;
    int32_t pictured = quorra_window_thumbnail_read(window, 0, &thumbnail);
    quorra_thumbnail_free(thumbnail);
    printf("window panels: %d owed, %zu outline row(s), %zu property row(s), %zu page(s); page 1 "
           "is labelled \"%s\" (%s), its thumbnail: %s\n",
           seen->panels, quorra_outline_len(outline), quorra_panel_len(properties), pages,
           labelled == QUORRA_OK ? label : "", quorra_status_message(labelled),
           quorra_status_message(pictured));
    quorra_panel_free(properties);
    quorra_outline_free(outline);
    return 1;
}

/* The menu's four launch levels round-trip through the same window state. */
static int choose_permissions(quorra_window *window)
{
    size_t count = quorra_window_restriction_count(window), first = 0, restore = 0;
    uint32_t depth = 0;
    bool chosen = false;
    char label[192];
    size_t needed = 0;
    for (size_t row = 0; row < count; row++) {
        if (!check("restriction read", quorra_window_restriction_read(window, row, 0,
                    &depth, &chosen, label, sizeof(label), &needed))) return 0;
        if (strcmp(label, "opening a launched document in another program") == 0) first = row + 1;
    }
    if (first == 0 || first + 4 > count
        || quorra_window_restriction_count(NULL) != 0
        || quorra_window_restriction_choose(window, 0) != QUORRA_WRONG_KIND
        || quorra_window_restriction_choose(window, count) != QUORRA_OUT_OF_RANGE
        || quorra_window_restriction_choose(NULL, 0) != QUORRA_NULL_ARGUMENT) return 0;
    for (size_t row = first; row < first + 4; row++) {
        if (!check("restriction read", quorra_window_restriction_read(window, row, 0,
                    &depth, &chosen, label, sizeof(label), &needed))) return 0;
        if (chosen) restore = row;
    }
    for (size_t row = first; row < first + 4; row++) {
        if (!check("restriction choose", quorra_window_restriction_choose(window, row))) return 0;
        for (size_t other = first; other < first + 4; other++) {
            if (!check("restriction tick", quorra_window_restriction_read(window, other, 0,
                        &depth, &chosen, label, sizeof(label), &needed))
                || depth != 2 || chosen != (row == other)) return 0;
        }
    }
    if (restore == 0 || !check("restriction restore", quorra_window_restriction_choose(window, restore))) return 0;
    printf("window permissions: all four launch levels selected and read back; initial level restored\n");
    return 1;
}

/* A thumbnail row names a page directly. The invalid index must not move the page. */
static int choose_a_page(quorra_window *window, struct seen *seen)
{
    size_t pages = 0;
    quorra_frame frame = {0};
    if (!check("quorra_window_page_count", quorra_window_page_count(window, &pages)) || pages < 2
        || !check("quorra_window_go_to_page", quorra_window_go_to_page(window, pages - 1))
        || !act_on_what_is_owed(window, seen) || !run_the_timer(window, seen)
        || !check("quorra_window_frame", quorra_window_frame(window, 0, &frame, NULL, NULL))
        || frame.page != pages - 1) {
        return 0;
    }
    if (quorra_window_go_to_page(window, pages) != QUORRA_OUT_OF_RANGE
        || quorra_window_go_to_page(NULL, 0) != QUORRA_NULL_ARGUMENT
        || !check("quorra_window_frame", quorra_window_frame(window, 0, &frame, NULL, NULL))
        || frame.page != pages - 1
        || !check("quorra_window_go_to_page", quorra_window_go_to_page(window, 0))
        || !act_on_what_is_owed(window, seen) || !run_the_timer(window, seen)
        || !check("quorra_window_frame", quorra_window_frame(window, 0, &frame, NULL, NULL))
        || frame.page != 0) {
        return 0;
    }
    printf("window thumbnails: last page and first selected; invalid index kept the page\n");
    return 1;
}

/* An outline row clicked: its object, activated, is §12.3.3's destination followed. */
static int activate_a_row(quorra_window *window, size_t row, struct seen *seen)
{
    quorra_outline *outline = NULL;
    uint32_t number = 0;
    uint16_t generation = 0;
    char title[128] = {0};
    size_t needed = 0;
    if (!check("quorra_window_outline_read", quorra_window_outline_read(window, &outline))
        || !check("quorra_outline_object",
                  quorra_outline_object(outline, row, &number, &generation))
        || !check("quorra_outline_title",
                  quorra_outline_title(outline, row, title, sizeof title, &needed))) {
        quorra_outline_free(outline);
        return 0;
    }
    quorra_outline_free(outline);
    if (!check("quorra_window_activate", quorra_window_activate(window, number, generation))
        || !act_on_what_is_owed(window, seen) || !run_the_timer(window, seen)) {
        return 0;
    }
    printf("window outline row \"%s\" activated: %s\n", title, seen->title);
    return 1;
}

/* What the page area draws over the frames on a repaint: the find bar's occurrences, the selection
 * and Annex O's highlight, each a set of quadrilaterals the caller colours. */
static int read_the_marks(quorra_window *window, const char *when)
{
    quorra_matches *matches = NULL;
    quorra_quads *selection = NULL;
    quorra_quads *highlight = NULL;
    int ok = check("quorra_window_find_matches", quorra_window_find_matches(window, &matches))
             && check("quorra_window_selection_quads",
                      quorra_window_selection_quads(window, &selection))
             && check("quorra_window_highlight_quads",
                      quorra_window_highlight_quads(window, &highlight));
    if (ok) {
        printf("window marks %s: %zu match(es), %zu selection quad(s), %zu highlight(s)\n", when,
               quorra_matches_len(matches), quorra_quads_len(selection),
               quorra_quads_len(highlight));
    }
    quorra_matches_free(matches);
    quorra_quads_free(selection);
    quorra_quads_free(highlight);
    return ok;
}

/* Control and the wheel about the viewport's middle: three events of two fifths of a line each are
 * one step in, the fraction carried between calls as the toolkit windows carry it, and a touchpad's
 * sixty pixels the other way are one step out (ADR 2152). A unit this build does not define and a
 * travel that is not a number are refused by status. */
static int zoom_with_the_wheel(quorra_window *window, struct seen *seen)
{
    const float x = 400.0f;
    const float y = 500.0f;
    int32_t steps = 0;
    int32_t inwards = 0;
    seen->repainted = 0;
    seen->fresh = 0;
    for (int event = 0; event < 3; ++event) {
        if (!check("quorra_window_zoom_wheel",
                   quorra_window_zoom_wheel(window, 0.4f, QUORRA_WHEEL_LINES, x, y, &steps))
            || !act_on_what_is_owed(window, seen)) {
            return 0;
        }
        inwards += steps;
    }
    if (!run_the_timer(window, seen) || !copy_the_page(window)) {
        return 0;
    }
    /* The page drawn larger is new pixels in a new place, which the repaints said (ADR 2168). */
    printf("window wheel repaints: frames %d, places %d, %s fresh picture\n",
           (seen->repainted & QUORRA_REPAINT_FRAMES) != 0,
           (seen->repainted & QUORRA_REPAINT_PLACES) != 0, seen->fresh > 0 ? "a" : "no");
    int32_t unknown = quorra_window_zoom_wheel(window, 1.0f, 7u, x, y, &steps);
    int32_t nonsense = quorra_window_zoom_wheel(window, NAN, QUORRA_WHEEL_LINES, x, y, &steps);
    int32_t outwards = 0;
    if (!check("quorra_window_zoom_wheel",
               quorra_window_zoom_wheel(window, -60.0f, QUORRA_WHEEL_PIXELS, x, y, &outwards))
        || !act_on_what_is_owed(window, seen) || !run_the_timer(window, seen)
        || !copy_the_page(window)) {
        return 0;
    }
    printf("window wheel: 1.2 lines in three events, %d step(s); sixty pixels back, %d; unit 7: "
           "%s; not a number: %s\n",
           inwards, outwards, quorra_status_message(unknown), quorra_status_message(nonsense));
    return 1;
}

/* A repaint says what changed (ADR 2168): the find bar closed takes its occurrences away and
 * draws no pixel again, and a pointer moved where it already was changes nothing and owes no
 * repaint at all. */
static int the_repaint_says_what_changed(quorra_window *window, struct seen *seen, bool *over)
{
    seen->repaints = 0;
    seen->repainted = 0;
    seen->fresh = 0;
    if (!check("quorra_window_find_stop", quorra_window_find_stop(window))
        || !act_on_what_is_owed(window, seen)) {
        return 0;
    }
    int closed = seen->repaints;
    uint32_t closed_changed = seen->repainted;
    size_t closed_fresh = seen->fresh;
    if (!check("quorra_window_pointer",
               quorra_window_pointer(window, 2.0f, 2.0f, QUORRA_POINTER_MOVED, over))
        || !act_on_what_is_owed(window, seen)) {
        return 0;
    }
    seen->repaints = 0;
    if (!check("quorra_window_pointer",
               quorra_window_pointer(window, 2.0f, 2.0f, QUORRA_POINTER_MOVED, over))
        || !act_on_what_is_owed(window, seen)) {
        return 0;
    }
    printf("window repaint: the find bar closed owed %d repaint(s) changing %u with %zu fresh "
           "picture(s); the pointer where it was owed %d\n",
           closed, closed_changed, closed_fresh, seen->repaints);
    return closed == 1 && closed_changed == QUORRA_REPAINT_MARKS && closed_fresh == 0
           && seen->repaints == 0;
}

/* The application note: opened, drawn, presented, turned with a key, two keys that are not the
 * window's to finish, a search, the pointer, and a second document beside it in the strip. */
static int the_note(const char *path, const char *beside)
{
    quorra_window *window = NULL;
    struct seen seen = {0};
    if (!check("quorra_window_open", quorra_window_open(path, NULL, &window))
        || !check("quorra_window_resize", quorra_window_resize(window, 800, 1000, 1.0f))
        || !act_on_what_is_owed(window, &seen) || !run_the_timer(window, &seen)
        || !copy_the_page(window) || !act_on_what_is_owed(window, &seen)
        || !run_the_timer(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    printf("window title: %s\n", seen.title);
    if (!chrome_contract(window) || !read_the_panels(window, &seen)
        || !choose_a_page(window, &seen) || !choose_permissions(window)) {
        quorra_window_free(window);
        return 0;
    }

    uint32_t act = QUORRA_ACT_NOTHING;
    if (!press(window, QUORRA_KEY_PAGE_DOWN, false, &act, &seen) || !copy_the_page(window)) {
        quorra_window_free(window);
        return 0;
    }
    printf("after Page Down: %s, act %s, %d repaint(s) owed so far\n", seen.title,
           quorra_act_name(act), seen.repaints);

    uint32_t find = QUORRA_ACT_NOTHING;
    uint32_t markup = QUORRA_ACT_NOTHING;
    if (!press(window, QUORRA_KEY_F, true, &find, &seen)
        || !press(window, QUORRA_KEY_H, false, &markup, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    printf("window keys: Control + F is %s, H is %s and said \"%.40s\"\n", quorra_act_name(find),
           quorra_act_name(markup), seen.said);

    /* The find bar's search, read a page per tick until it lands, then closed. */
    if (!check("quorra_window_find", quorra_window_find(window, "Black Point", false))
        || !act_on_what_is_owed(window, &seen) || !run_the_timer(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    printf("window search: %s\n", seen.said);
    bool over = true;
    if (!read_the_marks(window, "with the find bar open")
        || !the_repaint_says_what_changed(window, &seen, &over)
        || !read_the_marks(window, "after it closed")) {
        quorra_window_free(window);
        return 0;
    }
    printf("window pointer at the corner: over a link %d\n", over ? 1 : 0);
    if (!activate_a_row(window, 0, &seen) || !zoom_with_the_wheel(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }

    /* A file chooser's answer opens in a tab of its own; the strip says so, and closing both
     * documents ends at the last, which is the window's to close. */
    if (!check("quorra_window_open_beside", quorra_window_open_beside(window, beside))
        || !act_on_what_is_owed(window, &seen) || !run_the_timer(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    char label[256] = {0};
    size_t needed = 0;
    size_t documents = quorra_window_document_count(window);
    if (!check("quorra_window_document_label",
               quorra_window_document_label(window, documents - 1, label, sizeof label, &needed))) {
        quorra_window_free(window);
        return 0;
    }
    printf("window strip: %zu document(s), %s in front at %zu, the strip owed %s\n", documents,
           label, quorra_window_focused_document(window), seen.strips > 0 ? "yes" : "no");
    bool last = true;
    if (!check("quorra_window_choose_document", quorra_window_choose_document(window, 0))
        || !check("quorra_window_close_document", quorra_window_close_document(window, &last))
        || !act_on_what_is_owed(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    printf("window closed one: %s; the last %d\n", seen.said, last ? 1 : 0);
    if (!check("quorra_window_close_document", quorra_window_close_document(window, &last))) {
        quorra_window_free(window);
        return 0;
    }
    printf("window closing the other: the last %d, %zu document(s) left open\n", last ? 1 : 0,
           quorra_window_document_count(window));
    quorra_window_free(window);
    return 1;
}

/* The measured fit (ADR 2169): a control asked for 100 by 10 logical pixels that cannot be less
 * than 20 high fits at twice the magnification, which W then sends; once every control fits, W
 * says so and sends nothing. */
static int the_measured_fit(quorra_window *window, struct seen *seen)
{
    const int32_t taller[4] = {100, 10, 100, 20};
    const int32_t fitting[4] = {100, 20, 60, 20};
    float wanted = 0.0f;
    float none = -1.0f;
    uint32_t act = QUORRA_ACT_NOTHING;
    char fitted[sizeof seen->said] = {0};
    if (!check("quorra_window_controls_measured",
               quorra_window_controls_measured(window, taller, 1, &wanted))
        || !press(window, QUORRA_KEY_W, false, &act, seen)) {
        return 0;
    }
    snprintf(fitted, sizeof fitted, "%s", seen->said);
    if (!check("quorra_window_controls_measured",
               quorra_window_controls_measured(window, fitting, 1, &none))
        || !press(window, QUORRA_KEY_W, false, &act, seen)) {
        return 0;
    }
    int32_t unsized = quorra_window_controls_measured(window, NULL, 1, NULL);
    printf("window fit: %s; W said \"%.24s\", act %s; once they fit %.1f and \"%s\"; no sizes: "
           "%s\n",
           wanted > 0.0f ? "a magnification" : "none", fitted, quorra_act_name(act), none,
           seen->said, quorra_status_message(unsized));
    return wanted > 0.0f && none == 0.0f && unsized == QUORRA_NULL_ARGUMENT;
}

/* §12.9's traced path (ADR 2168): M starts measuring, two presses put down two points the repaint
 * calls marks, and M again ends it and the path with it. */
static int the_traced_path(quorra_window *window, struct seen *seen)
{
    uint32_t act = QUORRA_ACT_NOTHING;
    bool over = false;
    float points[8] = {0};
    size_t traced = 0;
    size_t after = 99;
    if (!press(window, QUORRA_KEY_M, false, &act, seen)) {
        return 0;
    }
    seen->repainted = 0;
    if (!check("quorra_window_pointer",
               quorra_window_pointer(window, 100.0f, 100.0f, QUORRA_POINTER_PRESSED, &over))
        || !check("quorra_window_pointer",
                  quorra_window_pointer(window, 100.0f, 100.0f, QUORRA_POINTER_RELEASED, &over))
        || !check("quorra_window_pointer",
                  quorra_window_pointer(window, 200.0f, 100.0f, QUORRA_POINTER_PRESSED, &over))
        || !check("quorra_window_pointer",
                  quorra_window_pointer(window, 200.0f, 100.0f, QUORRA_POINTER_RELEASED, &over))
        || !act_on_what_is_owed(window, seen)
        || !check("quorra_window_traced_path",
                  quorra_window_traced_path(window, points, 8, &traced))) {
        return 0;
    }
    uint32_t marked = seen->repainted;
    if (!press(window, QUORRA_KEY_M, false, &act, seen)
        || !check("quorra_window_traced_path",
                  quorra_window_traced_path(window, NULL, 0, &after))) {
        return 0;
    }
    printf("window traced path: %zu float(s), from %.0f,%.0f to %.0f,%.0f, the marks %s; %zu after "
           "the mode ended\n",
           traced, points[0], points[1], points[2], points[3],
           (marked & QUORRA_REPAINT_MARKS) != 0 ? "repainted" : "not repainted", after);
    return traced == 4 && after == 0 && (marked & QUORRA_REPAINT_MARKS) != 0;
}

/* The fixture's push-button, reached with Tab and pressed with Space: the reader's default level
 * asks before a form leaves this machine, the program answers no, and the window says so. */
static int the_form(const char *path)
{
    quorra_window *window = NULL;
    struct seen seen = {0};
    if (!check("quorra_window_open", quorra_window_open(path, NULL, &window))
        || !check("quorra_window_resize", quorra_window_resize(window, 800, 1000, 1.0f))
        || !act_on_what_is_owed(window, &seen) || !run_the_timer(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    /* The form's first field is a text field, a control placed over the page — so the open owed
     * the controls once — which toggles nothing; a field the page does not show refuses an edit. */
    quorra_fields *fields = NULL;
    if (!check("quorra_window_fields_read", quorra_window_fields_read(window, &fields))) {
        quorra_window_free(window);
        return 0;
    }
    char name[128] = {0};
    size_t needed = 0;
    uint32_t number = 0;
    uint16_t generation = 0;
    float quad[8] = {0};
    bool on = false;
    if (quorra_fields_len(fields) == 0
        || !check("quorra_field_name",
                  quorra_field_name(fields, 0, QUORRA_TEXT_QUALIFIED, name, sizeof name, &needed))
        || !check("quorra_field_widget",
                  quorra_field_widget(fields, 0, 0, &number, &generation, quad, &on))) {
        quorra_fields_free(fields);
        quorra_window_free(window);
        return 0;
    }
    size_t count = quorra_fields_len(fields);
    quorra_fields_free(fields);
    int32_t toggled = quorra_window_toggle_field(window, number, generation, true);
    int32_t unknown = quorra_window_set_field_text(window, "no such field", "text");
    printf("window form: %zu field(s), %s; %d control record(s); toggling it: %s; an unknown "
           "field: %s\n",
           count, name, seen.controls, quorra_status_message(toggled),
           quorra_status_message(unknown));
    if (toggled != QUORRA_WRONG_KIND || unknown != QUORRA_NO_ANSWER) {
        quorra_window_free(window);
        return 0;
    }
    uint32_t act = QUORRA_ACT_NOTHING;
    for (int tabs = 0; tabs < 8 && seen.questions == 0; ++tabs) {
        if (!press(window, QUORRA_KEY_TAB, false, &act, &seen)
            || !press(window, QUORRA_KEY_SPACE, false, &act, &seen)) {
            quorra_window_free(window);
            return 0;
        }
    }
    if (seen.questions != 1) {
        fprintf(stderr, "the push-button's submission was not asked about: %d\n", seen.questions);
        quorra_window_free(window);
        return 0;
    }
    /* The question's two paragraphs, the reasons and the choice, joined by an empty line. */
    const char *choice = strstr(seen.question, "\n\n");
    printf("window question: %.50s | %.60s\n", seen.question, choice == NULL ? "" : choice + 2);
    if (!check("quorra_window_answer", quorra_window_answer(window, false))
        || !act_on_what_is_owed(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    printf("window answered no: %s\n", seen.said);
    int32_t again = quorra_window_answer(window, false);
    printf("a second answer: %s\n", quorra_status_message(again));
    int32_t unasked = quorra_window_password(window, 1, "secret");
    printf("a password nobody asked for: %s\n", quorra_status_message(unasked));
    /* The placed text control typed into and finished with, and the value read back. */
    char value[64] = {0};
    if (!check("quorra_window_set_field_text", quorra_window_set_field_text(window, name, "typed in C"))
        || !check("quorra_window_commit_field", quorra_window_commit_field(window, name))
        || !act_on_what_is_owed(window, &seen)
        || !check("quorra_window_fields_read", quorra_window_fields_read(window, &fields))) {
        quorra_window_free(window);
        return 0;
    }
    int32_t valued = quorra_field_value(fields, 0, value, sizeof value, &needed);
    quorra_fields_free(fields);
    printf("window control committed: %s is \"%s\" (%s)\n", name, value,
           quorra_status_message(valued));
    int fitted = the_measured_fit(window, &seen) && the_traced_path(window, &seen);
    quorra_window_free(window);
    return again == QUORRA_NO_ANSWER && unasked == QUORRA_NO_ANSWER && valued == QUORRA_OK
           && fitted;
}

/* The three requests a toolkit answers with a dialogue, on the note at the default levels: ISO
 * 19005's chooser closed with nothing chosen, a print let through and ended, and a script's answer
 * with no question out (ADR 2121). */
static int the_requests(const char *path)
{
    quorra_window *window = NULL;
    struct seen seen = {0};
    if (!check("quorra_window_open", quorra_window_open(path, NULL, &window))
        || !check("quorra_window_resize", quorra_window_resize(window, 800, 1000, 1.0f))
        || !act_on_what_is_owed(window, &seen) || !run_the_timer(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    size_t targets = 0;
    size_t profiles = 0;
    char first[32] = {0};
    size_t needed = 0;
    if (!check("quorra_archive_targets", quorra_archive_targets(&targets))
        || !check("quorra_archive_profiles", quorra_archive_profiles(&profiles))
        || !check("quorra_archive_target", quorra_archive_target(0, first, sizeof first, &needed))) {
        quorra_window_free(window);
        return 0;
    }
    /* Shift + A with no conversion chosen yet: the chooser, owed as one record. */
    uint32_t act = QUORRA_ACT_NOTHING;
    float by = 0.0f;
    if (!check("quorra_window_press",
               quorra_window_press(window, QUORRA_KEY_A, true, false, &act, &by))
        || !act_on_what_is_owed(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    int32_t past = quorra_window_archive(window, (int32_t)targets, 0);
    if (seen.archives != 1 || past != QUORRA_OUT_OF_RANGE
        || !check("quorra_window_archive", quorra_window_archive(window, -1, -1))
        || !act_on_what_is_owed(window, &seen)) {
        fprintf(stderr, "the chooser: %d record(s), past its list %d\n", seen.archives, past);
        quorra_window_free(window);
        return 0;
    }
    int32_t twice = quorra_window_archive(window, 0, 0);
    printf("window chooser: %zu target(s) from %s, %zu profile(s), starting on %zu and %zu, act "
           "%s; past the list: %s; closed: %s; again: %s\n",
           targets, first, profiles, seen.archive_target, seen.archive_profile,
           quorra_act_name(act), quorra_status_message(past), seen.said,
           quorra_status_message(twice));
    /* Control + P: §7.6.4.2's bit 3 asked, at the default level let through, and ended. */
    if (!check("quorra_window_press",
               quorra_window_press(window, QUORRA_KEY_P, false, true, &act, &by))
        || !act_on_what_is_owed(window, &seen) || seen.prints != 1
        || !check("quorra_window_print_finish", quorra_window_print_finish(window))
        || !act_on_what_is_owed(window, &seen)) {
        fprintf(stderr, "the print: %d record(s)\n", seen.prints);
        quorra_window_free(window);
        return 0;
    }
    int32_t finished = quorra_window_print_finish(window);
    int32_t unasked = quorra_window_answer_script(window, 1, QUORRA_SCRIPT_ANSWER_PRESSED,
                                                  QUORRA_ALERT_BUTTON_OK, NULL);
    printf("window print: act %s, %zu page(s), degraded %d; finished again: %s; a script answer "
           "nobody asked for: %s\n",
           quorra_act_name(act), seen.print_pages, seen.print_degraded ? 1 : 0,
           quorra_status_message(finished), quorra_status_message(unasked));
    quorra_window_free(window);
    return twice == QUORRA_NO_ANSWER && finished == QUORRA_NO_ANSWER && unasked == QUORRA_NO_ANSWER
           && targets == 6 && act == QUORRA_ACT_NOTHING;
}

/* The levels a host states: words read as the four windows read theirs, a level and a
 * restriction set by number, and the form opened under them — its push-button's submission
 * refused without a question, where the default level asked one. */
static int the_settings(const char *path)
{
    quorra_settings *settings = NULL;
    bool taken = false;
    bool other = true;
    char refusal[256] = {0};
    size_t needed = 0;
    if (!check("quorra_settings_new", quorra_settings_new(&settings))
        || !check("quorra_settings_take",
                  quorra_settings_take(settings, "--submissions=refuse", &taken))
        || !check("quorra_settings_take", quorra_settings_take(settings, "--cpu", &other))
        || !check("quorra_settings_restrict",
                  quorra_settings_restrict(settings, QUORRA_RESTRICTED_PRINT, QUORRA_RESTRICT_ON))
        || !check("quorra_settings_set",
                  quorra_settings_set(settings, QUORRA_POLICY_LINKS, QUORRA_LEVEL_REFUSE))) {
        quorra_settings_free(settings);
        return 0;
    }
    int32_t wrong = quorra_settings_take(settings, "--links=sometimes", NULL);
    int32_t said = quorra_settings_refusal(settings, refusal, sizeof refusal, &needed);
    int32_t between =
        quorra_settings_set(settings, QUORRA_POLICY_SEPARATIONS, QUORRA_LEVEL_ASK);
    printf("settings: --submissions=refuse taken %d, --cpu taken %d; --links=sometimes: %s, "
           "\"%s\"; separations at ask: %s\n",
           taken ? 1 : 0, other ? 1 : 0, quorra_status_message(wrong),
           said == QUORRA_OK ? refusal : "", quorra_status_message(between));
    quorra_window *window = NULL;
    struct seen seen = {0};
    int ok = taken && !other && wrong == QUORRA_WRONG_KIND && said == QUORRA_OK
             && between == QUORRA_WRONG_KIND
             && check("quorra_window_open_under",
                      quorra_window_open_under(path, NULL, settings, &window));
    quorra_settings_free(settings);
    if (!ok || !check("quorra_window_resize", quorra_window_resize(window, 800, 1000, 1.0f))
        || !act_on_what_is_owed(window, &seen) || !run_the_timer(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    uint32_t act = QUORRA_ACT_NOTHING;
    char refused[512] = {0};
    for (int tabs = 0; tabs < 8 && refused[0] == 0; ++tabs) {
        if (!press(window, QUORRA_KEY_TAB, false, &act, &seen)
            || !press(window, QUORRA_KEY_SPACE, false, &act, &seen)) {
            quorra_window_free(window);
            return 0;
        }
        if (strstr(seen.said, "submit-form") != NULL) {
            snprintf(refused, sizeof refused, "%s", seen.said);
        }
    }
    printf("window under the stated levels: %d question(s); %s\n", seen.questions, refused);
    quorra_window_free(window);
    return seen.questions == 0 && refused[0] != 0;
}

/* The reader's other words, stated by word and by number (ADR 2156): a value taken with the word
 * after it, a flag leaving the word after it, a word with no value refused in the command line's
 * sentence; then `directory` — one file that is neither a certificate nor a PDF — as the anchors
 * and the target documents, said by name as SAID records, and a conversion chosen, which the
 * chooser's second key starts on. And the print key, which ends the print that stands (ADR 2157). */
static int the_reader(const char *path, const char *directory)
{
    quorra_settings *settings = NULL;
    size_t named = 0;
    size_t flag = 0;
    char refusal[256] = {0};
    size_t needed = 0;
    if (!check("quorra_settings_new", quorra_settings_new(&settings))
        || !check("quorra_settings_take_next",
                  quorra_settings_take_next(settings, "--reader-name", "Ada", &named))
        || !check("quorra_settings_take_next",
                  quorra_settings_take_next(settings, "--accept-unknown-revocation", "doc.pdf",
                                            &flag))
        || !check("quorra_settings_reader",
                  quorra_settings_reader(settings, QUORRA_READER_ORGANISATION, "Acme"))
        || !check("quorra_settings_accept_unknown_revocation",
                  quorra_settings_accept_unknown_revocation(settings, false))
        || !check("quorra_settings_reader",
                  quorra_settings_reader(settings, QUORRA_READER_TRUST_ANCHORS, directory))
        || !check("quorra_settings_reader",
                  quorra_settings_reader(settings, QUORRA_READER_REFERENCE_FILES, directory))
        || !check("quorra_settings_archive", quorra_settings_archive(settings, 1, 0))) {
        quorra_settings_free(settings);
        return 0;
    }
    int32_t bare = quorra_settings_take(settings, "--trust-anchors", NULL);
    int32_t said = quorra_settings_refusal(settings, refusal, sizeof refusal, &needed);
    int32_t nowhere =
        quorra_settings_reader(settings, QUORRA_READER_REFERENCE_FILES, "/nonexistent/quorra");
    int32_t unknown = quorra_settings_reader(settings, 6, "Ada");
    int32_t past = quorra_settings_archive(settings, 99, 0);
    printf("reader: --reader-name used %zu, --accept-unknown-revocation used %zu; "
           "--trust-anchors alone: %s, \"%s\"; no such directory: %s; word 6: %s; target 99: %s\n",
           named, flag, quorra_status_message(bare), said == QUORRA_OK ? refusal : "",
           quorra_status_message(nowhere), quorra_status_message(unknown),
           quorra_status_message(past));
    quorra_window *window = NULL;
    struct seen seen = {0};
    int ok = named == 2 && flag == 1 && bare == QUORRA_WRONG_KIND && said == QUORRA_OK
             && nowhere == QUORRA_WRONG_KIND && unknown == QUORRA_WRONG_KIND
             && past == QUORRA_OUT_OF_RANGE
             && check("quorra_window_open_under",
                      quorra_window_open_under(path, NULL, settings, &window));
    quorra_settings_free(settings);
    if (!ok || !check("quorra_window_resize", quorra_window_resize(window, 800, 1000, 1.0f))
        || !act_on_what_is_owed(window, &seen) || !run_the_timer(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    /* Control + Shift + A: the chooser again, on the conversion the handle stated. */
    uint32_t act = QUORRA_ACT_NOTHING;
    float by = 0.0f;
    if (!check("quorra_window_press",
               quorra_window_press(window, QUORRA_KEY_A, true, true, &act, &by))
        || !act_on_what_is_owed(window, &seen) || seen.archives != 1
        || !check("quorra_window_archive", quorra_window_archive(window, -1, -1))
        || !act_on_what_is_owed(window, &seen)) {
        fprintf(stderr, "the chooser again: %d record(s)\n", seen.archives);
        quorra_window_free(window);
        return 0;
    }
    printf("window under the reader's words: the chooser starts on %zu and %zu\n",
           seen.archive_target, seen.archive_profile);
    /* Control + P twice: the print let through, then ended by the same key, so the caller's
     * own end finds nothing to end. */
    if (!check("quorra_window_press",
               quorra_window_press(window, QUORRA_KEY_P, false, true, &act, &by))
        || !act_on_what_is_owed(window, &seen) || seen.prints != 1
        || !check("quorra_window_press",
                  quorra_window_press(window, QUORRA_KEY_P, false, true, &act, &by))
        || !act_on_what_is_owed(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    int32_t finished = quorra_window_print_finish(window);
    printf("window print key again: \"%s\"; then finished: %s\n", seen.said,
           quorra_status_message(finished));
    quorra_window_free(window);
    return seen.archive_target == 1 && seen.archive_profile == 0 && seen.prints == 1
           && finished == QUORRA_NO_ANSWER;
}

/* How many pixels of the page area's first picture are not white. */
static int count_the_ink(quorra_window *window, size_t *inked)
{
    quorra_frame frame;
    uint32_t shown_width = 0;
    uint32_t shown_height = 0;
    if (quorra_window_frame_count(window) == 0
        || !check("quorra_window_frame",
                  quorra_window_frame(window, 0, &frame, &shown_width, &shown_height))) {
        return 0;
    }
    uint8_t *pixels = malloc(frame.bytes);
    size_t written = 0;
    if (pixels == NULL
        || !check("quorra_window_frame_copy",
                  quorra_window_frame_copy(window, 0, pixels, frame.bytes, &written))) {
        free(pixels);
        return 0;
    }
    *inked = 0;
    for (size_t at = 0; at + 3 < written; at += 4) {
        if (pixels[at] != 255 || pixels[at + 1] != 255 || pixels[at + 2] != 255) {
            ++*inked;
        }
    }
    free(pixels);
    return 1;
}

/* The audience a host states, as a named reader sees it (§8.11.4.4, ADR 2173): `path` is one
 * layer whose /Usage /User names Ada and whose /AS manages it, over a filled rectangle. Opened
 * under --reader-name Ada, under --reader-name Grace and under nobody, the PANELS record's layer
 * row says whether it is on, and the page area's picture is drawn under the same answer: on for
 * Ada, off for Grace, and the configuration's own state where nobody was named. */
static int the_audience(const char *path)
{
    static const char *const readers[3] = {"Ada", "Grace", NULL};
    int on[3] = {-1, -1, -1};
    size_t inked[3] = {0, 0, 0};
    int panels[3] = {0, 0, 0};
    char name[64] = {0};
    for (int reader = 0; reader < 3; ++reader) {
        quorra_settings *settings = NULL;
        size_t used = 0;
        if (!check("quorra_settings_new", quorra_settings_new(&settings))
            || (readers[reader] != NULL
                && !check("quorra_settings_take_next",
                          quorra_settings_take_next(settings, "--reader-name", readers[reader],
                                                    &used)))) {
            quorra_settings_free(settings);
            return 0;
        }
        quorra_window *window = NULL;
        struct seen seen = {0};
        int opened = check("quorra_window_open_under",
                           quorra_window_open_under(path, NULL, settings, &window));
        quorra_settings_free(settings);
        quorra_panel *layers = NULL;
        uint32_t kind = 0;
        uint32_t number = 0;
        uint16_t generation = 0;
        bool shown = false;
        bool locked = false;
        size_t needed = 0;
        if (!opened || !check("quorra_window_resize", quorra_window_resize(window, 400, 200, 1.0f))
            || !act_on_what_is_owed(window, &seen) || !run_the_timer(window, &seen)
            || !check("quorra_window_layers_read", quorra_window_layers_read(window, &layers))
            || quorra_panel_len(layers) != 1
            || !check("quorra_panel_text",
                      quorra_panel_text(layers, 0, 0, name, sizeof name, &needed))
            || !check("quorra_panel_action", quorra_panel_action(layers, 0, &kind, &number,
                                                                 &generation, &shown, &locked))
            || kind != QUORRA_ROW_TOGGLE || !count_the_ink(window, &inked[reader])) {
            quorra_panel_free(layers);
            quorra_window_free(window);
            return 0;
        }
        quorra_panel_free(layers);
        quorra_window_free(window);
        on[reader] = shown ? 1 : 0;
        panels[reader] = seen.panels;
    }
    printf("window audience: \"%s\" on %d for Ada, %zu inked; on %d for Grace, %zu inked; on %d "
           "for nobody, %zu inked; %d, %d and %d panels record(s)\n",
           name, on[0], inked[0], on[1], inked[1], on[2], inked[2], panels[0], panels[1],
           panels[2]);
    return on[0] == 1 && on[1] == 0 && on[2] == 1 && inked[0] > 0 && inked[1] == 0
           && inked[2] == inked[0] && panels[0] > 0 && panels[1] > 0 && panels[2] > 0;
}

/* One commit of the fixture's field, whose validation asks with app.alert. */
static int commit_the_question(quorra_window *window, const char *value, struct seen *seen)
{
    return check("quorra_window_set_field_text",
                 quorra_window_set_field_text(window, "kept", value))
           && check("quorra_window_commit_field", quorra_window_commit_field(window, "kept"))
           && act_on_what_is_owed(window, seen);
}

/* A script's question, at the scripts' PROCEED level a host set by number: owed as a record in
 * Adobe's numbers, answered once, and a second one left unanswered until its wait withdraws it
 * (ADRs 1628, 1643). */
static int the_script(const char *path)
{
    quorra_settings *settings = NULL;
    quorra_window *window = NULL;
    struct seen seen = {0};
    int opened = check("quorra_settings_new", quorra_settings_new(&settings))
                 && check("quorra_settings_set",
                          quorra_settings_set(settings, QUORRA_POLICY_SCRIPTS,
                                              QUORRA_LEVEL_PROCEED))
                 && check("quorra_window_open_under",
                          quorra_window_open_under(path, NULL, settings, &window));
    quorra_settings_free(settings);
    if (!opened || !check("quorra_window_resize", quorra_window_resize(window, 800, 1000, 1.0f))
        || !act_on_what_is_owed(window, &seen) || !run_the_timer(window, &seen)
        || !commit_the_question(window, "5", &seen) || seen.scripts != 1) {
        fprintf(stderr, "the script's question: %d record(s)\n", seen.scripts);
        quorra_window_free(window);
        return 0;
    }
    printf("window script: \"%s\", icon %u, buttons %u, closed answers %u\n", seen.script_message,
           seen.script_icon, seen.script_buttons, seen.script_dismissed);
    if (!check("quorra_window_answer_script",
               quorra_window_answer_script(window, seen.script_document,
                                           QUORRA_SCRIPT_ANSWER_PRESSED, QUORRA_ALERT_BUTTON_YES,
                                           NULL))
        || !act_on_what_is_owed(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    printf("window script answered: %s\n", seen.said);
    int32_t again = quorra_window_answer_script(window, seen.script_document,
                                                QUORRA_SCRIPT_ANSWER_PRESSED,
                                                QUORRA_ALERT_BUTTON_YES, NULL);
    if (!commit_the_question(window, "6", &seen) || seen.scripts != 2
        || !run_the_timer(window, &seen)) {
        fprintf(stderr, "the second question: %d record(s)\n", seen.scripts);
        quorra_window_free(window);
        return 0;
    }
    int32_t late = quorra_window_answer_script(window, seen.script_document,
                                               QUORRA_SCRIPT_ANSWER_PRESSED,
                                               QUORRA_ALERT_BUTTON_YES, NULL);
    printf("window script again: %s; the second withdrawn %d, then answered: %s\n",
           quorra_status_message(again), seen.withdrawn, quorra_status_message(late));
    quorra_window_free(window);
    return again == QUORRA_NO_ANSWER && seen.withdrawn == 1 && late == QUORRA_NO_ANSWER;
}

/* Table 147's /FitWindow and /CenterWindow, answered to a window this program pretends to be: the
 * page area 800 by 600 under no chrome but a bar forty tall, resized until the page area is the
 * first displayed page, then moved to the centre of the work area, then owed nothing (ADR 2308). */
static int the_placement(const char *path)
{
    quorra_window *window = NULL;
    struct seen seen = {0};
    seen.inner_width = 800;
    seen.inner_height = 600 + PLACED_CHROME;
    if (!check("quorra_window_open", quorra_window_open(path, NULL, &window))
        || !check("quorra_window_resize", quorra_window_resize(window, 800, 600, 1.0f))
        || !act_on_what_is_owed(window, &seen) || !run_the_timer(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    quorra_window_extents extents = {seen.inner_width, seen.inner_height, 10, 30, 0, 0, 1920,
                                     1080, 1u};
    quorra_window_placement placed = {0};
    int32_t after = quorra_window_place(window, &extents, &placed);
    bool full_screen = true;
    if (!check("quorra_window_full_screen", quorra_window_full_screen(window, &full_screen))
        || quorra_window_full_screen(window, NULL) != QUORRA_NULL_ARGUMENT) {
        quorra_window_free(window);
        return 0;
    }
    printf("window placement: %d resize(s) to %dx%d, %d move(s) to %d,%d, then %s; full screen %d\n",
           seen.resizes, seen.inner_width, seen.inner_height, seen.moves, seen.moved_x,
           seen.moved_y, quorra_status_message(after), full_screen ? 1 : 0);
    quorra_window_free(window);
    return seen.resizes >= 1 && seen.moves == 1 && after == QUORRA_NO_ANSWER && !full_screen;
}

int main(int argc, char **argv)
{
    if (argc < 7) {
        fprintf(stderr,
                "usage: %s <file.pdf> <form.pdf> <script.pdf> <directory> <audience.pdf> "
                "<placed.pdf>\n",
                argv[0]);
        return 1;
    }
    if (!check("quorra_abi_check", quorra_abi_check(QUORRA_ABI_VERSION, QUORRA_EVENT_KIND_COUNT))) {
        return 1;
    }
    printf("owed kinds %u (header %u), unknown is %s\n", quorra_owed_kind_count(),
           QUORRA_OWED_KIND_COUNT, quorra_owed_kind_name(QUORRA_OWED_KIND_COUNT));
    if (quorra_owed_kind_count() != QUORRA_OWED_KIND_COUNT) {
        fprintf(stderr, "this program was compiled against another set of owed kinds\n");
        return 1;
    }
    /* The panels every window offers, by the library's count and its one wording for each. */
    printf("panels offered %u (header %u):", quorra_tab_count(), QUORRA_TAB_COUNT);
    for (uint32_t tab = 0; tab < quorra_tab_count(); ++tab) {
        char label[32] = {0};
        size_t needed = 0;
        if (!check("quorra_tab_label", quorra_tab_label(tab, label, sizeof label, &needed))) {
            return 1;
        }
        printf(" %s", label);
    }
    printf("\n");
    if (quorra_tab_count() != QUORRA_TAB_COUNT) {
        fprintf(stderr, "this program was compiled against another set of panels\n");
        return 1;
    }
    quorra_window *nothing = NULL;
    int32_t unreadable = quorra_window_open("/nonexistent/quorra.pdf", NULL, &nothing);
    printf("an unreadable file: %s\n", quorra_status_message(unreadable));
    if (unreadable != QUORRA_UNREADABLE || nothing != NULL) {
        return 1;
    }
    if (!the_note(argv[1], argv[2]) || !the_form(argv[2]) || !the_requests(argv[1])
        || !the_settings(argv[2]) || !the_reader(argv[1], argv[4]) || !the_script(argv[3])
        || !the_audience(argv[5]) || !the_placement(argv[6])) {
        return 1;
    }
    printf("ok\n");
    return 0;
}
