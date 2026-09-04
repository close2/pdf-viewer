/*
 * pdf_vfs.h — the C ABI over pdf-vfs: one PDF document as a directory tree.
 *
 * `pdf-vfs-ffi`, ADR 0868. RFC 0003 designs the tree; `src/lib.rs` argues every shape below;
 * what follows is the declaration and the one-line reason where a reason is not obvious.
 *
 * HAND-WRITTEN, AND CHECKED. No `cbindgen`: a generated header is a derivative of Rust types that
 * a C programmer then has to read anyway, and this one is the artefact rather than a by-product.
 * What a generator buys is that it cannot drift, and that is bought back by
 * `tests/header_and_library_agree.rs`, which reads this file and `src/abi.rs` and asserts that
 * every entry point is declared exactly once in each and that every PDFVFS_ constant is the
 * number the Rust gives it.
 *
 * THE TREE. `pdf:/home/u/doc.pdf/pages/0007.pdf` is one path with two halves: a document to open
 * and a path inside it. `pdfvfs_split` finds the boundary (the longest prefix that is a file);
 * `pdfvfs_mount_open` takes the first half and every other call takes the second. Paths inside
 * begin with a solidus, and "/" is the root.
 *
 *     doc.pdf/pages/0001.pdf      one extractable single-page PDF per page
 *     doc.pdf/renders/150dpi/…    the same pages rendered
 *     doc.pdf/images/0035/01.png  the embedded image XObjects, a directory per page
 *     doc.pdf/text/0001.txt       the extraction, per page and whole
 *     doc.pdf/attachments/…       §7.11.4's embedded files
 *     doc.pdf/meta/…              §14.3.3's /Info, §14.3.2's metadata, §12.3.3's outline
 *
 * THREADS. No handle may be used from two threads at once. A `pdfvfs_file *` may be MOVED to
 * another thread and read there; a `pdfvfs_mount *` may not be shared.
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
 * A REFUSAL IS AN OBJECT. Every call the *tree* can refuse takes a `pdfvfs_refusal **why`, and
 * writes it exactly when it answers PDFVFS_REFUSED — never on any other status. It carries the
 * `errno` the core states and the sentence RFC 0003 section 5.3 requires; free it with
 * `pdfvfs_refusal_free`. Passing NULL for `why` is allowed and loses the sentence.
 *
 * THE CONFINED GENERATOR. Not one byte of PDF is parsed in this library or in the process that
 * loads it. Every question that needs a parser is answered by `pdf-vfs-worker`, a separate
 * program that puts itself under seccomp-BPF, Landlock and an address-space ceiling before it
 * reads anything (RFC 0003 section 6). It is looked for beside the running executable, or at
 * $PDF_VFS_WORKER; `pdfvfs_worker_program()` and `pdfvfs_worker_variable()` are those two names,
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
 * The revision of everything below that is passed BY VALUE, which is `pdfvfs_attributes` and
 * nothing else. A function added later is a symbol an old caller never looks up; a status, a kind
 * or an errno added later is a number an old caller has a `default:` arm for. A FIELD added to
 * that struct is a size an old caller has already compiled, and no diagnostic anywhere would
 * catch it — so this number moves when that happens and at no other time.
 */
#define PDFVFS_ABI_VERSION 1u

/*
 * How many errno kinds the core states. Pass it to `pdfvfs_abi_check` before serving anything.
 *
 * This is what stands in for the Rust rule that a new refusal fails to compile in every consumer.
 * It cannot fail a build, so it fails a startup instead, once, naming the number that moved.
 */
#define PDFVFS_ERRNO_KIND_COUNT 13u

/* What an entry point returns. PDFVFS_OK is zero; everything else is a refusal or a mistake. */
#define PDFVFS_OK                    0
#define PDFVFS_NULL_ARGUMENT         1
#define PDFVFS_OUT_OF_RANGE          2
#define PDFVFS_BUFFER_TOO_SMALL      3
#define PDFVFS_NOT_UTF8              4
/* The tree refused. The `pdfvfs_refusal **why` the call was given has been written. */
#define PDFVFS_REFUSED               5
/* A fair question with no answer: the layout names no row for that path. */
#define PDFVFS_NO_ANSWER             6
/* No component of the path is a file, so there is no document there. `pdfvfs_split` only. */
#define PDFVFS_NO_DOCUMENT           7
#define PDFVFS_NUMBER_OUT_OF_RANGE   8

/* Directory or file: what a listing entry and a `pdfvfs_attributes` say. */
#define PDFVFS_KIND_DIRECTORY  0u
#define PDFVFS_KIND_FILE       1u

/*
 * How much of what a document asserts over its reader this mount obeys — CLAUDE.md principle 3's
 * four levels, chosen once at `pdfvfs_mount_open` because that is the place a host can supply it.
 * OFF is the default everywhere in this tree: the program is the reader's.
 *
 * ASK reaches a caller today as EACCES with a sentence saying a question went unanswered. The
 * question is decided inside the confined generator, which RFC 0003 section 6 gives no channel to
 * a person at all; ADR 0869 says what the wire owes before `KIO::WorkerBase::messageBox` can put
 * it. WARN proceeds and says the reasons afterwards, as a commit's warnings — and that one a KIO
 * worker can already show.
 */
#define PDFVFS_RESTRICT_OFF   0u
#define PDFVFS_RESTRICT_ON    1u
#define PDFVFS_RESTRICT_ASK   2u
#define PDFVFS_RESTRICT_WARN  3u

/*
 * What writing to or deleting a path means: RFC 0003 section 5.2's five verbs, and NOTHING for a
 * row that refuses. Which refusal it is, is a sentence rather than a number — ask for the verb
 * and read the `pdfvfs_refusal` the attempt produces.
 */
#define PDFVFS_MEANS_NOTHING             0u
#define PDFVFS_MEANS_INSERT_PAGES        1u
#define PDFVFS_MEANS_DELETE_PAGE         2u
#define PDFVFS_MEANS_EMBED_FILE          3u
#define PDFVFS_MEANS_REMOVE_ATTACHMENT   4u
#define PDFVFS_MEANS_SET_INFORMATION     5u

/* ------------------------------------------------------------------------------------------- */
/* The five handles, all opaque.                                                                 */
/* ------------------------------------------------------------------------------------------- */

typedef struct pdfvfs_mount pdfvfs_mount;
typedef struct pdfvfs_listing pdfvfs_listing;
typedef struct pdfvfs_file pdfvfs_file;
typedef struct pdfvfs_commit pdfvfs_commit;
typedef struct pdfvfs_refusal pdfvfs_refusal;

/*
 * What a `stat` answers. The only thing here passed by value, which is what PDFVFS_ABI_VERSION is
 * about — and it is called `pdfvfs_attributes` rather than `pdfvfs_stat` because C puts a struct
 * tag and a function in one namespace.
 *
 * `size` is the file's TRUE size, never an estimate: RFC 0003 section 5.5 makes a stat generate
 * the file, because a stated size that is too small silently truncates it for every reader.
 */
typedef struct {
    uint32_t kind;      /* PDFVFS_KIND_DIRECTORY or PDFVFS_KIND_FILE */
    uint32_t has_size;  /* one for a file, zero for a directory */
    uint64_t size;
} pdfvfs_attributes;

/* ------------------------------------------------------------------------------------------- */
/* Identity, and the words for a number.                                                         */
/* ------------------------------------------------------------------------------------------- */

uint32_t pdfvfs_abi_version(void);
uint32_t pdfvfs_errno_kind_count(void);
/* PDFVFS_OK when the header a caller compiled against and the library agree. */
int32_t pdfvfs_abi_check(uint32_t version, uint32_t errno_kinds);
/* One sentence for a status, including a status this build does not define. Never freed. */
const char *pdfvfs_status_message(int32_t status);
/* "EPERM", "ENOENT", … for any number, including one this build does not name. Never freed. */
const char *pdfvfs_errno_name(int32_t code);
/* The confined generator's program name, and the variable that names it explicitly. */
const char *pdfvfs_worker_program(void);
const char *pdfvfs_worker_variable(void);

/* ------------------------------------------------------------------------------------------- */
/* Where the document ends and the tree begins.                                                  */
/* ------------------------------------------------------------------------------------------- */

/*
 * Writes the length of the prefix of `url_path` that names the document; the tree inside is the
 * rest of the string, and an empty rest is the root. PDFVFS_NO_DOCUMENT where no prefix is a file.
 * Nothing but the file system can say where the boundary is, which is why this asks it.
 */
int32_t pdfvfs_split(const char *url_path, size_t *document_length);

/* ------------------------------------------------------------------------------------------- */
/* A refusal.                                                                                    */
/* ------------------------------------------------------------------------------------------- */

int32_t pdfvfs_refusal_errno(const pdfvfs_refusal *why, int32_t *out);
int32_t pdfvfs_refusal_message(const pdfvfs_refusal *why, char *out, size_t cap, size_t *needed);
void pdfvfs_refusal_free(pdfvfs_refusal *why);

/* ------------------------------------------------------------------------------------------- */
/* The mount.                                                                                    */
/* ------------------------------------------------------------------------------------------- */

/*
 * Opens a document as a tree at one of the four PDFVFS_RESTRICT_ levels. Nothing is parsed here:
 * the file is checked to be a regular file and the document is read on the first question.
 *
 * A `pdfvfs_mount *` is not const anywhere below, and that is honest rather than conservative: a
 * read generates and the mount remembers what it generated, so every operation can change what is
 * inside it.
 */
int32_t pdfvfs_mount_open(const char *document, uint32_t restrictions,
                          pdfvfs_mount **out, pdfvfs_refusal **why);
void pdfvfs_mount_free(pdfvfs_mount *mount);
/* How many pages §7.7.3.2's tree holds. The first call that reads the document. */
int32_t pdfvfs_mount_pages(pdfvfs_mount *mount, uint64_t *out, pdfvfs_refusal **why);
/* What the layout declares and this build does not do — print these, do not discover them. */
int32_t pdfvfs_mount_shortfall_count(pdfvfs_mount *mount, size_t *out);
int32_t pdfvfs_mount_shortfall(pdfvfs_mount *mount, size_t index,
                               char *out, size_t cap, size_t *needed);

/* ------------------------------------------------------------------------------------------- */
/* Reads — RFC 0003 section 5.1.                                                                 */
/* ------------------------------------------------------------------------------------------- */

int32_t pdfvfs_list(pdfvfs_mount *mount, const char *path,
                    pdfvfs_listing **out, pdfvfs_refusal **why);
int32_t pdfvfs_listing_len(const pdfvfs_listing *listing, size_t *out);
int32_t pdfvfs_listing_name(const pdfvfs_listing *listing, size_t index,
                            char *out, size_t cap, size_t *needed);
int32_t pdfvfs_listing_kind(const pdfvfs_listing *listing, size_t index, uint32_t *out);
void pdfvfs_listing_free(pdfvfs_listing *listing);

/* Generates the file, because the size has to be true. See `pdfvfs_attributes`. */
int32_t pdfvfs_stat(pdfvfs_mount *mount, const char *path,
                    pdfvfs_attributes *out, pdfvfs_refusal **why);

/*
 * What writing to and deleting this path would each mean, as PDFVFS_MEANS_. PDFVFS_NO_ANSWER for
 * a path the layout does not name. The CORE decides this, so the access bits a file manager shows
 * are the document's own shape rather than a list a face keeps.
 */
int32_t pdfvfs_write_meaning(pdfvfs_mount *mount, const char *path,
                             uint32_t *on_write, uint32_t *on_delete);

int32_t pdfvfs_open(pdfvfs_mount *mount, const char *path,
                    pdfvfs_file **out, pdfvfs_refusal **why);
int32_t pdfvfs_file_size(const pdfvfs_file *file, uint64_t *out);
/* Short at the end, empty past it, exactly as read(2) answers. */
int32_t pdfvfs_file_read(const pdfvfs_file *file, uint64_t offset,
                         uint8_t *buffer, size_t capacity, size_t *filled);
void pdfvfs_file_free(pdfvfs_file *file);

/* ------------------------------------------------------------------------------------------- */
/* Writes — RFC 0003 section 5.2, and the refusals of section 5.3.                               */
/* ------------------------------------------------------------------------------------------- */

/*
 * One whole file into the tree, as one transaction — which is the shape KIO's own `put` has, and
 * why the staged four a kernel needs (create/write/flush/release) are not on this boundary.
 */
int32_t pdfvfs_write(pdfvfs_mount *mount, const char *path,
                     const uint8_t *bytes, size_t length,
                     pdfvfs_commit **out, pdfvfs_refusal **why);
int32_t pdfvfs_remove(pdfvfs_mount *mount, const char *path,
                      pdfvfs_commit **out, pdfvfs_refusal **why);
int32_t pdfvfs_commit_pages(const pdfvfs_commit *commit, uint64_t *out);
/* Principle 3's *warn* level arrives here, and so does §7.5.6's "a deletion keeps the bytes". */
int32_t pdfvfs_commit_warning_count(const pdfvfs_commit *commit, size_t *out);
int32_t pdfvfs_commit_warning(const pdfvfs_commit *commit, size_t index,
                              char *out, size_t cap, size_t *needed);
void pdfvfs_commit_free(pdfvfs_commit *commit);

/* Both of these always answer PDFVFS_REFUSED, with the core's own sentence saying why. */
int32_t pdfvfs_rename(pdfvfs_mount *mount, const char *from, const char *to,
                      pdfvfs_refusal **why);
int32_t pdfvfs_create_directory(pdfvfs_mount *mount, const char *path, pdfvfs_refusal **why);

#ifdef __cplusplus
}
#endif

#endif /* PDF_VFS_H */
