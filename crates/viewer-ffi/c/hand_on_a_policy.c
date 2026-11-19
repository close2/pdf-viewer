/*
 * hand_on_a_policy.c — a signature policy's copy, fetched by the caller, handed to the document's
 * validation from C.
 *
 * ISO 32000-2 §12.8.3.4.4 has a signature handler "enforce signature policy constraints", and the
 * constraints are in the policy's document, which a signature names by URL and the caller fetches
 * at its reader's network level. This program does what such a caller does once it has the
 * octets: it asks for the document's report, finds the policy the signature publishes, binds the
 * copy (quorra_event_policy_bind), hands it on (quorra_policy_copy), and prints what the
 * validation said and the trusted list the bound policy references — which a policy no copy was
 * handed for does not reference, because its rules are unknown until then (ADR 3169).
 *
 *   ./hand_on_a_policy <signed.pdf> <policy-copy>
 *
 * `tests/a_c_program_drives_the_abi.rs` writes both files, builds and runs this, and checks what
 * it printed. Exit status: 0 if every step succeeded, 1 otherwise, with the step named on stderr.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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

/* Reads a whole file. Returns NULL and says why on failure. */
static uint8_t *read_file(const char *path, size_t *len)
{
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        fprintf(stderr, "cannot open %s\n", path);
        return NULL;
    }
    uint8_t *bytes = NULL;
    long size = -1;
    if (fseek(file, 0, SEEK_END) == 0) {
        size = ftell(file);
    }
    if (size >= 0) {
        rewind(file);
        /* One byte more than the file, so an empty file is still an allocation. */
        bytes = malloc((size_t)size + 1);
    }
    if (bytes != NULL && fread(bytes, 1, (size_t)size, file) != (size_t)size) {
        free(bytes);
        bytes = NULL;
    }
    fclose(file);
    if (bytes == NULL) {
        fprintf(stderr, "cannot read %s\n", path);
        return NULL;
    }
    *len = (size_t)size;
    return bytes;
}

/* Prints every event under `when`, one line each, in the two-call idiom. Returns 1 on success. */
static int say(const quorra_events *events, const char *when)
{
    size_t count = quorra_events_len(events);
    printf("%s: %zu event(s)\n", when, count);
    for (size_t index = 0; index < count; ++index) {
        size_t needed = 0;
        (void)quorra_events_describe(events, index, NULL, 0, &needed);
        char *said = malloc(needed);
        if (said == NULL) {
            return 0;
        }
        if (!check("quorra_events_describe",
                   quorra_events_describe(events, index, said, needed, &needed))) {
            free(said);
            return 0;
        }
        printf("  %s: %s\n", when, said);
        free(said);
    }
    return 1;
}

/* Prints how many trusted lists the focused document's bound policies reference, and each. */
static int references(const quorra_viewer *viewer, const char *when)
{
    size_t count = 0;
    if (!check("quorra_trusted_list_references", quorra_trusted_list_references(viewer, &count))) {
        return 0;
    }
    printf("%s: %zu reference(s)\n", when, count);
    for (size_t index = 0; index < count; ++index) {
        size_t needed = 0;
        (void)quorra_trusted_list_reference(viewer, index, NULL, 0, &needed);
        char *location = malloc(needed);
        if (location == NULL) {
            return 0;
        }
        if (!check("quorra_trusted_list_reference",
                   quorra_trusted_list_reference(viewer, index, location, needed, &needed))) {
            free(location);
            return 0;
        }
        printf("  reference: %s\n", location);
        free(location);
    }
    return 1;
}

/* The steps, each of which ends the run on its first failure; `main` owns what they allocate. */
static int run(quorra_viewer *viewer, const uint8_t *document, size_t document_len,
               const uint8_t *copy, size_t copy_len)
{
    quorra_events *opened = NULL;
    if (!check("quorra_open",
               quorra_open(viewer, 1, document, document_len, NULL, NULL, &opened))) {
        return 0;
    }
    quorra_events_free(opened);
    if (!references(viewer, "before")) {
        return 0;
    }

    quorra_events *published = NULL;
    if (!check("quorra_document_report", quorra_document_report(viewer, &published))) {
        return 0;
    }
    int ok = 0;
    size_t at = (size_t)-1;
    size_t other = (size_t)-1;
    for (size_t index = 0; index < quorra_events_len(published); ++index) {
        uint32_t kind = 0;
        if (!check("quorra_events_kind", quorra_events_kind(published, index, &kind))) {
            goto done;
        }
        if (kind == QUORRA_EVENT_SIGNATURE_POLICIES_PUBLISHED) {
            at = index;
        } else {
            other = index;
        }
    }
    if (at == (size_t)-1) {
        fprintf(stderr, "the report published no signature policy\n");
        goto done;
    }
    size_t policies = 0;
    if (!check("quorra_event_policy_count", quorra_event_policy_count(published, at, &policies))) {
        goto done;
    }
    printf("published: %zu policy copy(ies)\n", policies);

    /* Whether the copy is the signer's, and opens beside — the sentence a window says. */
    bool opens = true;
    size_t needed = 0;
    (void)quorra_event_policy_bind(published, at, 0, copy, copy_len, &opens, NULL, 0, &needed);
    char *bound = malloc(needed);
    if (bound == NULL) {
        goto done;
    }
    if (!check("quorra_event_policy_bind",
               quorra_event_policy_bind(published, at, 0, copy, copy_len, &opens, bound, needed,
                                        &needed))) {
        free(bound);
        goto done;
    }
    printf("bound: opens %d: %s\n", opens ? 1 : 0, bound);
    free(bound);

    /* A policy the event does not list, and an event of another kind, are refused and send
     * nothing. */
    quorra_events *refused = NULL;
    int32_t past = quorra_policy_copy(viewer, published, at, policies, copy, copy_len, &refused);
    int32_t wrong = other == (size_t)-1
                        ? QUORRA_WRONG_KIND
                        : quorra_policy_copy(viewer, published, other, 0, copy, copy_len, &refused);
    printf("refused: past the list %d, another kind %d, events %s\n", past, wrong,
           refused == NULL ? "none" : "some");
    if (refused != NULL) {
        quorra_events_free(refused);
    }

    quorra_events *handed = NULL;
    if (!check("quorra_policy_copy",
               quorra_policy_copy(viewer, published, at, 0, copy, copy_len, &handed))) {
        goto done;
    }
    ok = say(handed, "handed");
    quorra_events_free(handed);
    ok = ok && references(viewer, "after");

done:
    quorra_events_free(published);
    return ok;
}

int main(int argc, char **argv)
{
    if (argc != 3) {
        fprintf(stderr, "usage: %s <signed.pdf> <policy-copy>\n", argv[0]);
        return 1;
    }
    size_t document_len = 0;
    size_t copy_len = 0;
    uint8_t *document = read_file(argv[1], &document_len);
    uint8_t *copy = read_file(argv[2], &copy_len);
    quorra_viewer *viewer = quorra_viewer_new(200, 200, 1.0f);
    int ok = document != NULL && copy != NULL && viewer != NULL &&
             run(viewer, document, document_len, copy, copy_len);
    if (viewer != NULL) {
        quorra_viewer_free(viewer);
    }
    free(copy);
    free(document);
    return ok ? 0 : 1;
}
