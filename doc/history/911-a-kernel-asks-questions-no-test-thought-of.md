# 911 — A kernel asks questions no test thought of

2026-09-04. Argued in [ADR 0864](../adr/0864-a-kernel-asks-questions-no-test-thought-of.md) and
[ADR 0865](../adr/0865-two-images-on-a-page-and-a-sentence-about-u32-that-a-five-page-document-could-produce.md).
The **fifth** implementation round of [RFC 0003](../rfc/0003-file-system-faces.md), on round 909's
branch because it continues that landing.

One thing was owed and it was not code: `doc/todo/58` §3's "**nothing has ever mounted it**".

Touched: `crates/pdf-fuse/src/lib.rs`, `src/kernel.rs`, `tests/a_face.rs`;
`crates/pdf-vfs/src/lib.rs`, `src/cache.rs`, `src/commit.rs`, `tests/a_write.rs`,
`tests/confined.rs`; `crates/confined-transport/src/host.rs`;
`crates/pdf-transform/src/structure.rs`, `src/split.rs`, `src/merge.rs`;
`doc/conformance/ledger.toml` (one row), `doc/todo/58-…`; two ADRs, this file.

## 1. It mounted, on the first try

```
$ pdffs r911-doc.pdf mnt/            # doc/PDF20_AN001-BPC.pdf, five pages
$ mount | grep r911
pdffs on …/r911-mnt type fuse.pdf (rw,nosuid,nodev,noexec,noatime,user_id=1001,group_id=1001)
```

`/dev/fuse` is `crw-rw-rw-`, `fusermount3` is setuid root, and nothing in this tree had to change.
`ls`, `ls -l`, `stat`, `find`, `du -a`, `tree`, `cat`, `file`, `qpdf --check`, `grep -r`, `cp` and
`cp -r` all worked over RFC §4's tree. A page taken out is a PDF — `file` says "PDF document,
version 1.7, 1 page(s)", `qpdf --check` says "No syntax or stream encoding errors found" — and a
render is a PNG of the right size: 1241×1754 at 150 dpi, 2480×3508 at 300, which is A4. An
attachment out of `doc/PDF-Declarations.pdf` — two files whose names hold a character the tree
sanitises — is byte-identical to `cat` of the same path, and `cp -r attachments/` works with the
spaces in them.

**The whole of the read side was right.** Everything below was underneath it.

## 2. The seven the kernel found

ADR 0864 has the arguments; these are the observations, because the point of the exercise is that
they are reproducible.

| what | what a shell said |
|---|---|
| `cp new.pdf mnt/pages/0002.pdf` — **RFC §5.2's flagship verb** | `cp: error writing …: Operation not permitted` |
| `> mnt/text/0001.txt` | `echo: write error: Operation not permitted` — at the byte, not at the open |
| `> mnt/renders/150dpi/0001.png` | `EPERM` for a name that exists, `EROFS` for one that does not |
| `: > mnt/pages/0002.pdf` | exit 0, nothing changed, nothing logged |
| `touch mnt/pages/0001.pdf` | exit 0, nothing changed |
| `ls -l mnt/` | every entry `root root`, every one dated `1. Jän 1970` |
| a `.directory`/`.DS_Store`/`.gitignore` probe | one log line per directory, fifteen before anything was asked |
| two `attachments/` writes with both descriptors open | the second lost, exit 0 |

`strace` is what settled the first, and it is the whole diagnosis in two lines:

```
openat(AT_FDCWD, ".../mnt/pages/0002.pdf", O_WRONLY|O_TRUNC) = 4
write(4, "%PDF-1.7\n%\277\367\242\376\n1 0 obj\n<< /Lang "..., 41453) = -1 EPERM
```

`cp` does not issue `create` for a name that exists, and every document with four pages already
has a `pages/0004.pdf`. Afterwards:

```
$ cp r911-one.pdf mnt/pages/0002.pdf && ls mnt/pages/ && qpdf --show-npages r911-doc.pdf
0001.pdf 0002.pdf 0003.pdf 0004.pdf 0005.pdf 0006.pdf
6
$ rm mnt/pages/0002.pdf && qpdf --check r911-doc.pdf | tail -2
No syntax or stream encoding errors found …
$ touch mnt/pages/0001.pdf
touch: setting times of '…/mnt/pages/0001.pdf': Operation not permitted
$ ls -l mnt/pages/ | head -2
-rw-r--r-- 1 AI AI 36265  4. Sep 02:03 0001.pdf
```

## 3. The three that were nothing to do with FUSE

- **`ls mnt/images/0060/` killed the confined worker.** `sig=31 … syscall=257` in the kernel's own
  audit line — `openat`, made by `glibc` creating a per-thread allocator arena on `rayon`'s pool
  thread. Page 60 of `doc/Tagged-PDF-Best-Practice-Guide.pdf` is the only page with **two** images,
  and one item runs on the calling thread. `MALLOC_ARENA_MAX=1` at the spawn, which is the one
  place early enough. It reaches `pdf-view-worker` too.
- **`ls -l mnt/pages/` on ISO 32000-2 produced 1674 `EIO`s**, all saying "the derived document
  needs more objects than one file can number" about a document with 33 000 objects. An `Option`
  had flattened `AssemblyError::AlreadyPlaced` into a sentence about `u32::MAX`; underneath it,
  §14.7's carry copied a content item's `/P` back-reference to its own element before the element
  had a slot. Both fixed; all 1023 pages come out and `qpdf --check` is clean on the piece.
- **A second `ls -l` cost more than the first**: 2 min 45 s, then 4 min 03 s. Sizes now outlive
  their bytes in the cache.

## 4. Where the gates went, and trap 13

`crates/pdf-fuse/tests/a_face.rs` gains three tests and `crates/pdf-vfs/tests/{a_write,confined}.rs`
three between them, one per defect rather than one per fix. Two were calibrated against the tree
without the fix, which is the only way to know:

- `a_page_with_two_images_does_not_kill_the_worker` fails with the `MALLOC_ARENA_MAX` line
  commented out and passes with it.
- `a_stat_after_an_eviction_does_not_generate_again` **failed for real on its first run** — 10
  generations where 5 were expected — because `Cache::put` returns early for an entry larger than
  the whole budget, which is exactly the population whose `stat` is expensive. The note is now
  taken before that return. It needed an instrument first: a size is the same number whether it
  was remembered or recomputed, so the version that asserted only the sizes passed without the
  fix. `Vfs::generated` counts what was actually produced.

## 5. Gates

The full `doc/todo/02` §2 sequence, one corpus walk at a time, because the change reaches
`pdf-transform`'s two writers and `confined-transport`'s spawn — which is the viewer's worker as
well as this one. All green. Two lint findings of the round's own making: a `similar_names` on
`ctime`/`crtime`, which are the FUSE protocol's own field names and are both in the guard, and
four rustfmt diffs.

## 6. What the next round of this stream does first

The **read** side still has no corpus walk — `renders/`, `images/`, `text/` and `meta/` against
four committed documents, where the write side has 974 — and this round is the reason to want it:
every one of §3's three defects was a *read* on a document the tests do not carry. And `doc/todo/58`
§3 now carries the next mount by hand rather than the first, which is a cheaper thing to owe.
