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
    int panels;
    int controls;
    int popups;
    int strips;
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
        case QUORRA_OWED_CLIPBOARD:
        case QUORRA_OWED_BEEP:
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
    if (!read_the_panels(window, &seen)) {
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
        || !check("quorra_window_find_stop", quorra_window_find_stop(window))
        || !read_the_marks(window, "after it closed")
        || !check("quorra_window_pointer",
                  quorra_window_pointer(window, 2.0f, 2.0f, QUORRA_POINTER_MOVED, &over))
        || !act_on_what_is_owed(window, &seen)) {
        quorra_window_free(window);
        return 0;
    }
    printf("window pointer at the corner: over a link %d\n", over ? 1 : 0);
    if (!activate_a_row(window, 0, &seen)) {
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
    quorra_window_free(window);
    return again == QUORRA_NO_ANSWER && unasked == QUORRA_NO_ANSWER && valued == QUORRA_OK;
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
    if (!the_note(argv[1], argv[2]) || !the_form(argv[2])) {
        return 1;
    }
    printf("ok\n");
    return 0;
}
