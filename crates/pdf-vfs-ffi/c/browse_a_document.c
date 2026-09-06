/*
 * browse_a_document.c — the ABI of `quorra_vfs.h`, driven from C.
 *
 * Everything else in this crate is Rust calling Rust: the entry points are `extern "C"` and the
 * argument types are C's, but no C compiler has read the header and no linker has resolved the
 * symbols. This program is what closes that, and it is also the shortest honest answer to "what
 * does a face have to do", because the C++ KIO worker beside it does exactly these calls with Qt
 * types on the outside.
 *
 * Two documents are given: the one to browse, and a scratch copy to write to. The second is
 * separate because the write verbs change the file, and a gate that mutated a corpus document
 * would be a gate that only passes once.
 *
 * Usage: browse_a_document <document.pdf> <scratch-copy.pdf>
 */

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "quorra_vfs.h"

/* Where a refusal's sentence is read into. Longer than any of them; the two-call idiom says so
 * rather than this program guessing, and `describe` checks. */
#define ROOM 1024

/* A refusal's errno name and sentence, printed and then freed. */
static void describe(const char *verb, quorra_vfs_refusal *why)
{
    int32_t code = 0;
    char sentence[ROOM];
    size_t needed = 0;

    if (why == NULL) {
        printf("%s: refused with no refusal object, which is a bug in the library\n", verb);
        return;
    }
    if (quorra_vfs_refusal_errno(why, &code) != QUORRA_VFS_OK) {
        printf("%s: a refusal that will not say its errno\n", verb);
        quorra_vfs_refusal_free(why);
        return;
    }
    if (quorra_vfs_refusal_message(why, sentence, sizeof sentence, &needed) != QUORRA_VFS_OK) {
        printf("%s: a sentence of %zu bytes did not fit %zu\n", verb, needed, sizeof sentence);
        quorra_vfs_refusal_free(why);
        return;
    }
    printf("%s: %s — %s\n", verb, quorra_vfs_errno_name(code), sentence);
    quorra_vfs_refusal_free(why);
}

/* Prints every name in a directory, and how many there were. */
static int listing(quorra_vfs_mount *mount, const char *path)
{
    quorra_vfs_listing *entries = NULL;
    quorra_vfs_refusal *why = NULL;
    size_t count = 0;
    size_t at = 0;

    if (quorra_vfs_list(mount, path, &entries, &why) != QUORRA_VFS_OK) {
        describe("list", why);
        return 1;
    }
    if (quorra_vfs_listing_len(entries, &count) != QUORRA_VFS_OK) {
        quorra_vfs_listing_free(entries);
        return 1;
    }
    printf("%s: %zu entries:", path, count);
    for (at = 0; at < count; at++) {
        char name[ROOM];
        uint32_t kind = 0;
        size_t needed = 0;

        if (quorra_vfs_listing_name(entries, at, name, sizeof name, &needed) != QUORRA_VFS_OK) {
            printf(" <a name of %zu bytes>", needed);
            continue;
        }
        if (quorra_vfs_listing_kind(entries, at, &kind) != QUORRA_VFS_OK) {
            kind = QUORRA_VFS_KIND_FILE;
        }
        printf(" %s%s", name, kind == QUORRA_VFS_KIND_DIRECTORY ? "/" : "");
    }
    printf("\n");
    quorra_vfs_listing_free(entries);
    return 0;
}

/* Reads a whole virtual file into a buffer the caller frees, and answers its length. */
static uint8_t *whole(quorra_vfs_mount *mount, const char *path, uint64_t *length)
{
    quorra_vfs_file *file = NULL;
    quorra_vfs_refusal *why = NULL;
    uint8_t *bytes = NULL;
    size_t filled = 0;

    *length = 0;
    if (quorra_vfs_open(mount, path, &file, &why) != QUORRA_VFS_OK) {
        describe("open", why);
        return NULL;
    }
    if (quorra_vfs_file_size(file, length) != QUORRA_VFS_OK || *length == 0) {
        quorra_vfs_file_free(file);
        return NULL;
    }
    bytes = malloc((size_t) *length);
    if (bytes == NULL) {
        quorra_vfs_file_free(file);
        return NULL;
    }
    if (quorra_vfs_file_read(file, 0, bytes, (size_t) *length, &filled) != QUORRA_VFS_OK
        || filled != (size_t) *length) {
        free(bytes);
        quorra_vfs_file_free(file);
        return NULL;
    }
    quorra_vfs_file_free(file);
    return bytes;
}

/* The read half: everything RFC 0003 section 5.1 offers, over a document nothing writes to. */
static int reading(const char *document)
{
    quorra_vfs_mount *mount = NULL;
    quorra_vfs_refusal *why = NULL;
    quorra_vfs_attributes attributes;
    uint64_t pages = 0;
    uint64_t length = 0;
    uint8_t *page = NULL;
    uint32_t on_write = 0;
    uint32_t on_delete = 0;
    size_t shortfalls = 0;

    if (quorra_vfs_mount_open(document, QUORRA_VFS_RESTRICT_OFF, &mount, &why) != QUORRA_VFS_OK) {
        describe("mount", why);
        return 1;
    }
    if (quorra_vfs_mount_pages(mount, &pages, &why) != QUORRA_VFS_OK) {
        describe("pages", why);
        quorra_vfs_mount_free(mount);
        return 1;
    }
    printf("the document has %" PRIu64 " page(s)\n", pages);

    if (listing(mount, "/") != 0 || listing(mount, "/pages") != 0) {
        quorra_vfs_mount_free(mount);
        return 1;
    }

    if (quorra_vfs_stat(mount, "/pages/0001.pdf", &attributes, &why) != QUORRA_VFS_OK) {
        describe("stat", why);
        quorra_vfs_mount_free(mount);
        return 1;
    }
    printf("stat /pages/0001.pdf: kind %u, size stated %u, %" PRIu64 " byte(s)\n",
           attributes.kind, attributes.has_size, attributes.size);

    page = whole(mount, "/pages/0001.pdf", &length);
    if (page == NULL || length != attributes.size || length < 5
        || memcmp(page, "%PDF-", 5) != 0) {
        printf("the extracted page is not a PDF\n");
        free(page);
        quorra_vfs_mount_free(mount);
        return 1;
    }
    printf("read /pages/0001.pdf: %" PRIu64 " byte(s), beginning %%PDF-\n", length);
    free(page);

    /* What the CORE says a verb means here, which is what a file manager's access bits are. */
    if (quorra_vfs_write_meaning(mount, "/pages/0001.pdf", &on_write, &on_delete) == QUORRA_VFS_OK) {
        printf("meaning of /pages/0001.pdf: write %u, delete %u\n", on_write, on_delete);
    }
    if (quorra_vfs_write_meaning(mount, "/text/0001.txt", &on_write, &on_delete) == QUORRA_VFS_OK) {
        printf("meaning of /text/0001.txt: write %u, delete %u\n", on_write, on_delete);
    }

    /* RFC 0003 section 5.3's refusals, each with the sentence a person is shown. */
    {
        quorra_vfs_commit *commit = NULL;
        const uint8_t nothing[1] = { 0 };
        why = NULL;
        if (quorra_vfs_write(mount, "/text/0001.txt", nothing, 1, &commit, &why) == QUORRA_VFS_REFUSED) {
            describe("writing into text/", why);
        } else {
            printf("writing into text/ was not refused\n");
            quorra_vfs_commit_free(commit);
        }
    }
    why = NULL;
    if (quorra_vfs_rename(mount, "/pages/0001.pdf", "/pages/0002.pdf", &why) == QUORRA_VFS_REFUSED) {
        describe("rename", why);
    }
    why = NULL;
    if (quorra_vfs_create_directory(mount, "/fonts", &why) == QUORRA_VFS_REFUSED) {
        describe("mkdir", why);
    }

    /* Trap 5 across a boundary: what the layout declares and this build does not do. */
    if (quorra_vfs_mount_shortfall_count(mount, &shortfalls) == QUORRA_VFS_OK) {
        char first[ROOM];
        size_t needed = 0;
        printf("shortfalls: %zu\n", shortfalls);
        if (shortfalls > 0
            && quorra_vfs_mount_shortfall(mount, 0, first, sizeof first, &needed) == QUORRA_VFS_OK) {
            printf("the first shortfall: %s\n", first);
        }
        if (quorra_vfs_mount_shortfall(mount, shortfalls, first, sizeof first, &needed)
            == QUORRA_VFS_OUT_OF_RANGE) {
            printf("a shortfall that is not there is refused rather than answered\n");
        }
    }

    quorra_vfs_mount_free(mount);
    return 0;
}

/* The write half: RFC 0003 section 5.2, over a scratch copy this program is allowed to change. */
/*
 * CLAUDE.md principle 3's *ask* level, in the two round trips ADR 0874 chose.
 *
 * This is what a face does: ask whether the verb would be restricted, put the question to the
 * person by whatever means it has, carry the answer back, then perform the verb unchanged. The
 * "person" here is this function, which says no once and yes once so that both outcomes are
 * driven — and the property that matters most is the first: answering no leaves the document
 * exactly as it was.
 */
static int restricting(const char *restricted)
{
    quorra_vfs_mount *mount = NULL;
    quorra_vfs_refusal *why = NULL;
    quorra_vfs_consultation *asked = NULL;
    quorra_vfs_file *file = NULL;
    uint32_t verdict = QUORRA_VFS_VERDICT_PROCEED;
    uint32_t answered = 0;
    char question[ROOM];
    size_t needed = 0;

    if (quorra_vfs_mount_open(restricted, QUORRA_VFS_RESTRICT_ASK, &mount, &why) != QUORRA_VFS_OK) {
        describe("mount restricted", why);
        return 1;
    }

    /* The question. */
    why = NULL;
    if (quorra_vfs_consult(mount, "/pages/0001.pdf", QUORRA_VFS_VERB_READ, &asked, &why) != QUORRA_VFS_OK) {
        describe("consult", why);
        quorra_vfs_mount_free(mount);
        return 1;
    }
    if (quorra_vfs_consultation_verdict(asked, &verdict) != QUORRA_VFS_OK
        || quorra_vfs_consultation_question(asked, question, sizeof question, &needed) != QUORRA_VFS_OK) {
        fprintf(stderr, "a consultation that would not say what it was\n");
        quorra_vfs_consultation_free(asked);
        quorra_vfs_mount_free(mount);
        return 1;
    }
    quorra_vfs_consultation_free(asked);
    printf("consulted: verdict %u, question '%s'\n", verdict, question);
    if (verdict != QUORRA_VFS_VERDICT_ASK) {
        fprintf(stderr, "this document was supposed to withhold the operation\n");
        quorra_vfs_mount_free(mount);
        return 1;
    }

    /* A no. The page does not come out, and the refusal says a question went unanswered. */
    why = NULL;
    if (quorra_vfs_answer(mount, 0u, &answered, &why) != QUORRA_VFS_OK || answered == 0u) {
        describe("answer no", why);
        quorra_vfs_mount_free(mount);
        return 1;
    }
    why = NULL;
    if (quorra_vfs_open(mount, "/pages/0001.pdf", &file, &why) == QUORRA_VFS_OK) {
        fprintf(stderr, "a no let the operation through\n");
        quorra_vfs_file_free(file);
        quorra_vfs_mount_free(mount);
        return 1;
    }
    describe("after a no", why);

    /* A yes. The question is put again first: an answer answers one question. */
    asked = NULL;
    why = NULL;
    if (quorra_vfs_consult(mount, "/pages/0001.pdf", QUORRA_VFS_VERB_READ, &asked, &why) != QUORRA_VFS_OK) {
        describe("consult again", why);
        quorra_vfs_mount_free(mount);
        return 1;
    }
    quorra_vfs_consultation_free(asked);
    why = NULL;
    if (quorra_vfs_answer(mount, 1u, &answered, &why) != QUORRA_VFS_OK || answered == 0u) {
        describe("answer yes", why);
        quorra_vfs_mount_free(mount);
        return 1;
    }
    why = NULL;
    file = NULL;
    if (quorra_vfs_open(mount, "/pages/0001.pdf", &file, &why) != QUORRA_VFS_OK) {
        describe("after a yes", why);
        quorra_vfs_mount_free(mount);
        return 1;
    }
    {
        uint64_t size = 0;
        if (quorra_vfs_file_size(file, &size) == QUORRA_VFS_OK) {
            printf("after a yes: %" PRIu64 " byte(s) of page\n", size);
        }
    }
    quorra_vfs_file_free(file);
    quorra_vfs_mount_free(mount);
    return 0;
}

static int writing(const char *source, const char *scratch)
{
    quorra_vfs_mount *reader = NULL;
    quorra_vfs_mount *mount = NULL;
    quorra_vfs_refusal *why = NULL;
    quorra_vfs_commit *commit = NULL;
    uint8_t *page = NULL;
    uint64_t length = 0;
    uint64_t pages = 0;
    size_t warnings = 0;

    /* One page, taken out of the source through the same tree: `cp` IS page extraction. */
    if (quorra_vfs_mount_open(source, QUORRA_VFS_RESTRICT_OFF, &reader, &why) != QUORRA_VFS_OK) {
        describe("mount source", why);
        return 1;
    }
    page = whole(reader, "/pages/0001.pdf", &length);
    quorra_vfs_mount_free(reader);
    if (page == NULL) {
        return 1;
    }

    why = NULL;
    if (quorra_vfs_mount_open(scratch, QUORRA_VFS_RESTRICT_OFF, &mount, &why) != QUORRA_VFS_OK) {
        describe("mount scratch", why);
        free(page);
        return 1;
    }

    why = NULL;
    if (quorra_vfs_remove(mount, "/pages/0005.pdf", &commit, &why) != QUORRA_VFS_OK) {
        describe("remove", why);
        free(page);
        quorra_vfs_mount_free(mount);
        return 1;
    }
    if (quorra_vfs_commit_pages(commit, &pages) == QUORRA_VFS_OK
        && quorra_vfs_commit_warning_count(commit, &warnings) == QUORRA_VFS_OK) {
        size_t at = 0;
        printf("deleted a page: %" PRIu64 " page(s) now, %zu warning(s)\n", pages, warnings);
        for (at = 0; at < warnings; at++) {
            char said[ROOM];
            size_t needed = 0;
            if (quorra_vfs_commit_warning(commit, at, said, sizeof said, &needed) == QUORRA_VFS_OK) {
                printf("  warning: %s\n", said);
            }
        }
    }
    quorra_vfs_commit_free(commit);

    commit = NULL;
    why = NULL;
    if (quorra_vfs_write(mount, "/pages/0001.pdf", page, (size_t) length, &commit, &why)
        != QUORRA_VFS_OK) {
        describe("insert", why);
        free(page);
        quorra_vfs_mount_free(mount);
        return 1;
    }
    free(page);
    if (quorra_vfs_commit_pages(commit, &pages) == QUORRA_VFS_OK) {
        printf("inserted a page at 0001: %" PRIu64 " page(s) now\n", pages);
    }
    quorra_vfs_commit_free(commit);

    /* And the tree renumbers, which is what makes an ordinal a position rather than an identity. */
    if (listing(mount, "/pages") != 0) {
        quorra_vfs_mount_free(mount);
        return 1;
    }
    quorra_vfs_mount_free(mount);
    return 0;
}

int main(int argc, char **argv)
{
    size_t document = 0;
    char joined[ROOM];

    if (argc != 4) {
        fprintf(stderr, "usage: %s <document.pdf> <scratch-copy.pdf> <restricted.pdf|->\n",
                argv[0]);
        return 2;
    }

    /* The startup check, which is what a C caller has in place of a build failure. */
    printf("abi %u (header %u), %u errno kind(s) (header %u)\n",
           quorra_vfs_abi_version(), QUORRA_VFS_ABI_VERSION,
           quorra_vfs_errno_kind_count(), QUORRA_VFS_ERRNO_KIND_COUNT);
    if (quorra_vfs_abi_check(QUORRA_VFS_ABI_VERSION, QUORRA_VFS_ERRNO_KIND_COUNT) != QUORRA_VFS_OK) {
        fprintf(stderr, "the header and the library disagree\n");
        return 1;
    }

    /* A URL's path is a document and a path inside it, and only the file system knows where. */
    if (snprintf(joined, sizeof joined, "%s/pages/0001.pdf", argv[1]) < 0) {
        return 1;
    }
    if (quorra_vfs_split(joined, &document) != QUORRA_VFS_OK) {
        fprintf(stderr, "the path would not split\n");
        return 1;
    }
    printf("split: %zu of %zu bytes are the document, and the rest is '%s'\n",
           document, strlen(joined), joined + document);
    if (quorra_vfs_split("/nowhere/at/all/pages/0001.pdf", &document) == QUORRA_VFS_NO_DOCUMENT) {
        printf("a path with no file in it: %s\n",
               quorra_vfs_status_message(QUORRA_VFS_NO_DOCUMENT));
    }

    if (reading(argv[1]) != 0) {
        return 1;
    }
    if (writing(argv[1], argv[2]) != 0) {
        return 1;
    }
    /* A hyphen where the pdf.js corpus is not checked out: the caller says so rather than this
     * program guessing, and the line it prints is what the harness asserts on either way. */
    if (strcmp(argv[3], "-") == 0) {
        printf("restricted: skipped, no corpus\n");
    } else if (restricting(argv[3]) != 0) {
        return 1;
    }
    printf("ok\n");
    return 0;
}
