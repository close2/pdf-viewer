/*
 * drive_a_window.c — quorra_window driven from C: the loop a toolkit host runs, one call per thing
 * that happened to the window (RFC 0010 section 3.2a, ADR 2088).
 *
 * Where open_a_page.c answers every viewer-core event itself, this program answers none: the
 * window holds the loop the three toolkit windows share, and what it owes the program — a
 * sentence, a repaint, a title, a question — arrives as a batch after each call. So the event loop
 * below is a resize, a timer, some keys and an answer, and each of them is one call followed by
 * the same `act_on_what_is_owed`.
 *
 *   ./drive_a_window <file.pdf> <form.pdf>
 *
 * The second document is the fixture `tests/a_c_program_drives_the_abi.rs` writes: a push-button
 * whose §12.7.6.2 submission the reader's default level asks about, which is how the question
 * and its answer are exercised.
 *
 * Exit status: 0 if every step succeeded, 1 otherwise, with the failing step named on stderr.
 */

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
    int questions;
    uint64_t prompted;
    char title[256];
    char said[512];
    char question[512];
};

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
        case QUORRA_OWED_REPAINT:
            seen->repaints += 1;
            break;
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
        case QUORRA_OWED_CLIPBOARD:
        case QUORRA_OWED_BEEP:
        case QUORRA_OWED_PANELS:
        case QUORRA_OWED_CHROME:
        case QUORRA_OWED_PLACE:
            break;
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
    if (!check("quorra_window_find_stop", quorra_window_find_stop(window))
        || !check("quorra_window_pointer",
                  quorra_window_pointer(window, 2.0f, 2.0f, QUORRA_POINTER_MOVED, &over))
        || !act_on_what_is_owed(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    printf("window pointer at the corner: over a link %d\n", over ? 1 : 0);

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
    printf("window strip: %zu document(s), %s in front at %zu\n", documents, label,
           quorra_window_focused_document(window));
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
    quorra_window_free(window);
    return again == QUORRA_NO_ANSWER && unasked == QUORRA_NO_ANSWER;
}

int main(int argc, char **argv)
{
    if (argc < 3) {
        fprintf(stderr, "usage: %s <file.pdf> <form.pdf>\n", argv[0]);
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
    quorra_window *nothing = NULL;
    int32_t unreadable = quorra_window_open("/nonexistent/quorra.pdf", NULL, &nothing);
    printf("an unreadable file: %s\n", quorra_status_message(unreadable));
    if (unreadable != QUORRA_UNREADABLE || nothing != NULL) {
        return 1;
    }
    if (!the_note(argv[1], argv[2]) || !the_form(argv[2])) {
        return 1;
    }
    printf("ok\n");
    return 0;
}
