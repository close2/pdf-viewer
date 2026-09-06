/*
 * quorra_vfs.h — the C ABI over pdf-vfs: one PDF document as a directory tree.
 *
 * `pdf-vfs-ffi`, ADR 0868. RFC 0003 designs the tree; `src/lib.rs` argues every shape below;
 * what follows is the declaration and the one-line reason where a reason is not obvious.
 *
 * HAND-WRITTEN, AND CHECKED. No `cbindgen`: a generated header is a derivative of Rust types that
 * a C programmer then has to read anyway, and this one is the artefact rather than a by-product.
 * What a generator buys is that it cannot drift, and that is bought back by
 * `tests/header_and_library_agree.rs`, which reads this file and `src/abi.rs` and asserts that
 * every entry point is declared exactly once in each and that every QUORRA_VFS_ constant is the
 * number the Rust gives it.
 *
 * THE TREE. `pdf:/home/u/doc.pdf/pages/0007.pdf` is one path with two halves: a document to open
 * and a path inside it. `quorra_vfs_split` finds the boundary (the longest prefix that is a file);
 * `quorra_vfs_mount_open` takes the first half and every other call takes the second. Paths inside
 * begin with a solidus, and "/" is the root.
 *
 *     doc.pdf/pages/0001.pdf      one extractable single-page PDF per page
 *     doc.pdf/renders/150dpi/…    the same pages rendered
 *     doc.pdf/images/0035/01.png  the embedded image XObjects, a directory per page
 *     doc.pdf/text/0001.txt       the extraction, per page and whole
 *     doc.pdf/attachments/…       §7.11.4's embedded files
 *     doc.pdf/meta/…              §14.3.3's /Info, §14.3.2's metadata, §12.3.3's outline
 *
 * THREADS. No handle may be used from two threads at once. A `quorra_vfs_file *` may be MOVED to
 * another thread and read there; a `quorra_vfs_mount *` may not be shared.
 *
 * MEMORY. Every handle this library returns is owned by the caller and is released with its own
 * `_free`. Nothing hands out a pointer into the library's memory: bytes are copied into a buffer
 * the caller owns, and every string uses the two-call idiom below.
 *
 * THE TWO-CALL IDIOM. A string-valued call takes `(char *out, size_t cap, size_t *needed)`.
 * Pass `out = NULL` to learn the size — `*needed` counts the terminating NUL — then call again
 * with a buffer that large. Nothing is written unless the whole string fits, so a truncated
 * sentence can never look like a short one.
 *
 * A REFUSAL IS AN OBJECT. Every call the *tree* can refuse takes a `quorra_vfs_refusal **why`, and
 * writes it exactly when it answers QUORRA_VFS_REFUSED — never on any other status. It carries the
 * `errno` the core states and the sentence RFC 0003 section 5.3 requires; free it with
 * `quorra_vfs_refusal_free`. Passing NULL for `why` is allowed and loses the sentence.
 *
 * THE CONFINED GENERATOR. Not one byte of PDF is parsed in this library or in the process that
 * loads it. Every question that needs a parser is answered by `pdf-vfs-worker`, a separate
 * program that puts itself under seccomp-BPF, Landlock and an address-space ceiling before it
 * reads anything (RFC 0003 section 6). It is looked for beside the running executable, or at
 * $PDF_VFS_WORKER; `quorra_vfs_worker_program()` and `quorra_vfs_worker_variable()` are those two names,
 * so a caller installed elsewhere can say what is missing rather than reporting a broken file.
 */

#ifndef PDF_VFS_H
#define PDF_VFS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ------------------------------------------------------------------------------------------- */
/* The identity of this ABI.                                                                     */
/* ------------------------------------------------------------------------------------------- */

/*
 * The revision of everything below that is passed BY VALUE, which is `quorra_vfs_attributes` and
 * nothing else. A function added later is a symbol an old caller never looks up; a status, a kind
 * or an errno added later is a number an old caller has a `default:` arm for. A FIELD added to
 * that struct is a size an old caller has already compiled, and no diagnostic anywhere would
 * catch it — so this number moves when that happens and at no other time.
 */
#define QUORRA_VFS_ABI_VERSION 2u

/*
 * How many errno kinds the core states. Pass it to `quorra_vfs_abi_check` before serving anything.
 *
 * This is what stands in for the Rust rule that a new refusal fails to compile in every consumer.
 * It cannot fail a build, so it fails a startup instead, once, naming the number that moved.
 */
#define QUORRA_VFS_ERRNO_KIND_COUNT 13u

/* What an entry point returns. QUORRA_VFS_OK is zero; everything else is a refusal or a mistake. */
#define QUORRA_VFS_OK                    0
#define QUORRA_VFS_NULL_ARGUMENT         1
#define QUORRA_VFS_OUT_OF_RANGE          2
#define QUORRA_VFS_BUFFER_TOO_SMALL      3
#define QUORRA_VFS_NOT_UTF8              4
/* The tree refused. The `quorra_vfs_refusal **why` the call was given has been written. */
#define QUORRA_VFS_REFUSED               5
/* A fair question with no answer: the layout names no row for that path. */
#define QUORRA_VFS_NO_ANSWER             6
/* No component of the path is a file, so there is no document there. `quorra_vfs_split` only. */
#define QUORRA_VFS_NO_DOCUMENT           7
#define QUORRA_VFS_NUMBER_OUT_OF_RANGE   8

/* Directory or file: what a listing entry and a `quorra_vfs_attributes` say. */
#define QUORRA_VFS_KIND_DIRECTORY  0u
#define QUORRA_VFS_KIND_FILE       1u

/*
 * How much of what a document asserts over its reader this mount obeys — CLAUDE.md principle 3's
 * four levels, chosen once at `quorra_vfs_mount_open` because that is the place a host can supply it.
 * OFF is the default everywhere in this tree: the program is the reader's.
 *
 * ASK is askable since ADR 0874, in two round trips: `quorra_vfs_consult` asks whether the operation
 * would be restricted and hands back the question, the face puts that question to a person by
 * whatever means it has (`KIO::WorkerBase::messageBox` is one), `quorra_vfs_answer` carries the
 * answer back, and the verb is then performed unchanged. A face that never calls those two still
 * gets the honest degradation — EACCES with a sentence saying a question went unanswered — which
 * is what a FUSE mount gets, because a mount has nowhere to put a question. WARN proceeds and
 * says the reasons afterwards, as a commit's warnings.
 */
#define QUORRA_VFS_RESTRICT_OFF   0u
#define QUORRA_VFS_RESTRICT_ON    1u
#define QUORRA_VFS_RESTRICT_ASK   2u
#define QUORRA_VFS_RESTRICT_WARN  3u

/*
 * What writing to or deleting a path means: RFC 0003 section 5.2's five verbs, and NOTHING for a
 * row that refuses. Which refusal it is, is a sentence rather than a number — ask for the verb
 * and read the `quorra_vfs_refusal` the attempt produces.
 */
#define QUORRA_VFS_MEANS_NOTHING             0u
#define QUORRA_VFS_MEANS_INSERT_PAGES        1u
#define QUORRA_VFS_MEANS_DELETE_PAGE         2u
#define QUORRA_VFS_MEANS_EMBED_FILE          3u
#define QUORRA_VFS_MEANS_REMOVE_ATTACHMENT   4u
#define QUORRA_VFS_MEANS_SET_INFORMATION     5u

/*
 * Which verb a `quorra_vfs_consult` is about. READ is not one of RFC 0003 section 5.2's write verbs
 * and never a refusal a write mapping words: it is here because taking a page out of the mount is
 * Table 22 bit 11's assembly, a render is bit 3's printing and an image is bit 5's extraction, so
 * a face is owed the question before it starts the copy as much as before it starts the write.
 */
#define QUORRA_VFS_VERB_READ     0u
#define QUORRA_VFS_VERB_WRITE    1u
#define QUORRA_VFS_VERB_DELETE   2u

/*
 * What a consultation came back with. Only ASK is a question; the other three are statements, and
 * a face that showed one as a dialogue would be asking somebody to decide something already
 * decided. PROCEED also covers a mount at QUORRA_VFS_RESTRICT_OFF, so a face may consult before every
 * verb and cost one round trip and no dialogue.
 */
#define QUORRA_VFS_VERDICT_PROCEED   0u
#define QUORRA_VFS_VERDICT_WARN      1u
#define QUORRA_VFS_VERDICT_ASK       2u
#define QUORRA_VFS_VERDICT_REFUSE    3u

/* ------------------------------------------------------------------------------------------- */
/* The six handles, all opaque.                                                                  */
/* ------------------------------------------------------------------------------------------- */

typedef struct quorra_vfs_mount quorra_vfs_mount;
typedef struct quorra_vfs_listing quorra_vfs_listing;
typedef struct quorra_vfs_file quorra_vfs_file;
typedef struct quorra_vfs_commit quorra_vfs_commit;
typedef struct quorra_vfs_refusal quorra_vfs_refusal;
typedef struct quorra_vfs_consultation quorra_vfs_consultation;

/*
 * What a `stat` answers. The only thing here passed by value, which is what QUORRA_VFS_ABI_VERSION is
 * about — and it is called `quorra_vfs_attributes` rather than `quorra_vfs_stat` because C puts a struct
 * tag and a function in one namespace.
 *
 * `size` is the file's TRUE size, never an estimate: RFC 0003 section 5.5 makes a stat generate
 * the file, because a stated size that is too small silently truncates it for every reader.
 */
typedef struct {
    uint32_t kind;      /* QUORRA_VFS_KIND_DIRECTORY or QUORRA_VFS_KIND_FILE */
    uint32_t has_size;  /* one for a file, zero for a directory */
    uint64_t size;
} quorra_vfs_attributes;

/* ------------------------------------------------------------------------------------------- */
/* Identity, and the words for a number.                                                         */
/* ------------------------------------------------------------------------------------------- */

uint32_t quorra_vfs_abi_version(void);
uint32_t quorra_vfs_errno_kind_count(void);
/* QUORRA_VFS_OK when the header a caller compiled against and the library agree. */
int32_t quorra_vfs_abi_check(uint32_t version, uint32_t errno_kinds);
/* One sentence for a status, including a status this build does not define. Never freed. */
const char *quorra_vfs_status_message(int32_t status);
/* "EPERM", "ENOENT", … for any number, including one this build does not name. Never freed. */
const char *quorra_vfs_errno_name(int32_t code);
/* The confined generator's program name, and the variable that names it explicitly. */
const char *quorra_vfs_worker_program(void);
const char *quorra_vfs_worker_variable(void);

/* ------------------------------------------------------------------------------------------- */
/* Where the document ends and the tree begins.                                                  */
/* ------------------------------------------------------------------------------------------- */

/*
 * Writes the length of the prefix of `url_path` that names the document; the tree inside is the
 * rest of the string, and an empty rest is the root. QUORRA_VFS_NO_DOCUMENT where no prefix is a file.
 * Nothing but the file system can say where the boundary is, which is why this asks it.
 */
int32_t quorra_vfs_split(const char *url_path, size_t *document_length);

/* ------------------------------------------------------------------------------------------- */
/* A refusal.                                                                                    */
/* ------------------------------------------------------------------------------------------- */

int32_t quorra_vfs_refusal_errno(const quorra_vfs_refusal *why, int32_t *out);
int32_t quorra_vfs_refusal_message(const quorra_vfs_refusal *why, char *out, size_t cap, size_t *needed);
void quorra_vfs_refusal_free(quorra_vfs_refusal *why);

/* ------------------------------------------------------------------------------------------- */
/* The mount.                                                                                    */
/* ------------------------------------------------------------------------------------------- */

/*
 * Opens a document as a tree at one of the four QUORRA_VFS_RESTRICT_ levels. Nothing is parsed here:
 * the file is checked to be a regular file and the document is read on the first question.
 *
 * A `quorra_vfs_mount *` is not const anywhere below, and that is honest rather than conservative: a
 * read generates and the mount remembers what it generated, so every operation can change what is
 * inside it.
 */
int32_t quorra_vfs_mount_open(const char *document, uint32_t restrictions,
                          quorra_vfs_mount **out, quorra_vfs_refusal **why);
void quorra_vfs_mount_free(quorra_vfs_mount *mount);
/* How many pages §7.7.3.2's tree holds. The first call that reads the document. */
int32_t quorra_vfs_mount_pages(quorra_vfs_mount *mount, uint64_t *out, quorra_vfs_refusal **why);
/* What the layout declares and this build does not do — print these, do not discover them. */
int32_t quorra_vfs_mount_shortfall_count(quorra_vfs_mount *mount, size_t *out);
int32_t quorra_vfs_mount_shortfall(quorra_vfs_mount *mount, size_t index,
                               char *out, size_t cap, size_t *needed);

/* ------------------------------------------------------------------------------------------- */
/* Reads — RFC 0003 section 5.1.                                                                 */
/* ------------------------------------------------------------------------------------------- */

int32_t quorra_vfs_list(quorra_vfs_mount *mount, const char *path,
                    quorra_vfs_listing **out, quorra_vfs_refusal **why);
int32_t quorra_vfs_listing_len(const quorra_vfs_listing *listing, size_t *out);
int32_t quorra_vfs_listing_name(const quorra_vfs_listing *listing, size_t index,
                            char *out, size_t cap, size_t *needed);
int32_t quorra_vfs_listing_kind(const quorra_vfs_listing *listing, size_t index, uint32_t *out);
void quorra_vfs_listing_free(quorra_vfs_listing *listing);

/* Generates the file, because the size has to be true. See `quorra_vfs_attributes`. */
int32_t quorra_vfs_stat(quorra_vfs_mount *mount, const char *path,
                    quorra_vfs_attributes *out, quorra_vfs_refusal **why);

/*
 * What writing to and deleting this path would each mean, as QUORRA_VFS_MEANS_. QUORRA_VFS_NO_ANSWER for
 * a path the layout does not name. The CORE decides this, so the access bits a file manager shows
 * are the document's own shape rather than a list a face keeps.
 */
int32_t quorra_vfs_write_meaning(quorra_vfs_mount *mount, const char *path,
                             uint32_t *on_write, uint32_t *on_delete);

int32_t quorra_vfs_open(quorra_vfs_mount *mount, const char *path,
                    quorra_vfs_file **out, quorra_vfs_refusal **why);
int32_t quorra_vfs_file_size(const quorra_vfs_file *file, uint64_t *out);
/* Short at the end, empty past it, exactly as read(2) answers. */
int32_t quorra_vfs_file_read(const quorra_vfs_file *file, uint64_t offset,
                         uint8_t *buffer, size_t capacity, size_t *filled);
void quorra_vfs_file_free(quorra_vfs_file *file);

/* ------------------------------------------------------------------------------------------- */
/* Writes — RFC 0003 section 5.2, and the refusals of section 5.3.                               */
/* ------------------------------------------------------------------------------------------- */

/*
 * One whole file into the tree, as one transaction — which is the shape KIO's own `put` has, and
 * why the staged four a kernel needs (create/write/flush/release) are not on this boundary.
 */
int32_t quorra_vfs_write(quorra_vfs_mount *mount, const char *path,
                     const uint8_t *bytes, size_t length,
                     quorra_vfs_commit **out, quorra_vfs_refusal **why);
int32_t quorra_vfs_remove(quorra_vfs_mount *mount, const char *path,
                      quorra_vfs_commit **out, quorra_vfs_refusal **why);
int32_t quorra_vfs_commit_pages(const quorra_vfs_commit *commit, uint64_t *out);
/* Principle 3's *warn* level arrives here, and so does §7.5.6's "a deletion keeps the bytes". */
int32_t quorra_vfs_commit_warning_count(const quorra_vfs_commit *commit, size_t *out);
int32_t quorra_vfs_commit_warning(const quorra_vfs_commit *commit, size_t index,
                              char *out, size_t cap, size_t *needed);
void quorra_vfs_commit_free(quorra_vfs_commit *commit);

/*
 * CLAUDE.md principle 3's *ask* level, in the two round trips ADR 0874 chose. Call `quorra_vfs_consult`
 * before the verb; where the verdict is QUORRA_VFS_VERDICT_ASK, show the question and call
 * `quorra_vfs_answer` with what the person said; then perform the verb unchanged — the yes is spent
 * by the one operation it was given for, and by no other.
 */
int32_t quorra_vfs_consult(quorra_vfs_mount *mount, const char *path, uint32_t verb,
                       quorra_vfs_consultation **out, quorra_vfs_refusal **why);
int32_t quorra_vfs_consultation_verdict(const quorra_vfs_consultation *consultation, uint32_t *out);
/* Empty for every verdict but QUORRA_VFS_VERDICT_ASK. */
int32_t quorra_vfs_consultation_question(const quorra_vfs_consultation *consultation,
                                     char *out, size_t cap, size_t *needed);
void quorra_vfs_consultation_free(quorra_vfs_consultation *consultation);
int32_t quorra_vfs_answer(quorra_vfs_mount *mount, uint32_t proceed,
                      uint32_t *answered, quorra_vfs_refusal **why);

/* Both of these always answer QUORRA_VFS_REFUSED, with the core's own sentence saying why. */
int32_t quorra_vfs_rename(quorra_vfs_mount *mount, const char *from, const char *to,
                      quorra_vfs_refusal **why);
int32_t quorra_vfs_create_directory(quorra_vfs_mount *mount, const char *path, quorra_vfs_refusal **why);

#ifdef __cplusplus
}
#endif

#endif /* PDF_VFS_H */
