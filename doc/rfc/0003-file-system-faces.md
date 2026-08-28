# RFC 0003 — File-system faces: a KIO worker and a FUSE filesystem over one core

Status: **draft**
Round: 786, commissioned by the owner
Companions: RFC 0002 (the transform layer — every write below goes through it), RFC 0004
(print), RFC 0005 (text editing). Numbering was reconciled at merge (round 788); the
numbers here are final and the index in `README.md` matches them.

**The owner's framing, verbatim in force**: this RFC is not limited by the project's
current rules; where a rule is relevant it is named as a current restriction with its
rationale, and the unconstrained design is proposed beside it.

**Registers**: everything cited from KDE, FUSE and other tools below is evidence about
*interface convention* — what users of file managers expect a document-as-folder to do.
None of it bears on what a page means; extraction and rendering semantics stay governed
by the specification alone.

## 1. Motivation

The owner's idea: expose PDF functionality abstracted as file operations. The argument is
the oldest one in Unix: once a document is a directory, every tool the user already has
becomes a PDF tool. `cp doc.pdf/pages/0007.pdf ~/` extracts a page. `ls doc.pdf/images/`
inventories the artwork. `grep -r` searches the text. A file manager becomes a page
organiser. No new UI has to be learned, scripted workflows come free, and the transform
layer (RFC 0002) gets a second, involuntary user interface — which is the best possible
test of its API being honest.

Two frontends, one virtual layout:

- **A KIO worker**, so Dolphin (and every KIO-aware application) browses into a PDF the
  way it browses into an archive today.
- **A FUSE filesystem**, so *every* program on the system — shells, scripts, non-KDE file
  managers, build systems — sees the same tree as a real mount.

Both are thin faces over one shared core; neither contains PDF logic of its own.

## 2. Prior art

- **kio_archive** (kio-extras): Dolphin's archives-as-folders. One plugin serves
  `tar:/`, `zip:/`, `archive:/`; the mime type→protocol association (protocol metadata's
  `archiveMimetype`) plus Dolphin's "open archives as folder" setting rewrites a click on
  an archive into a worker URL. It is **read-only**: its base class implements exactly
  `listDir`, `stat` and `get`.
  <https://github.com/KDE/kio-extras/tree/master/archive>
- **audiocd:/** — the strongest formats-as-directories precedent: the worker synthesises
  one virtual folder per encoder (FLAC, MP3, Ogg, WAV), one virtual file per track, and
  encodes *while streaming* a `get`; listed sizes are estimates. Directories that name a
  format are an established KIO idiom, not an invention of ours.
  <https://docs.kde.org/stable_kf6/en/audiocd-kio/kioworker6/audiocd/index.html>,
  <https://github.com/KDE/audiocd-kio/blob/master/HACKING>
- **kio_msits** (Okular): a CHM document container served as browsable virtual pages —
  document-as-folder, shipped for years.
  <https://invent.kde.org/graphics/okular/-/blob/master/generators/chm/kio-msits/msits.cpp>
- **kio_krarc** (Krusader): archive protocol *with write support*, gated behind an
  explicit "enable write support" option — precedent both for writes through a worker and
  for making them opt-in. <https://phabricator.kde.org/D1394>
- **kio-fuse** (KDE): mounts any KIO protocol as FUSE for KIO-unaware apps; since
  KIO 5.66 this routing is automatic. Design notes: lowlevel libfuse API, own inode
  table, and for protocols without random access, **whole-file caching** — the entire
  virtual file is materialised on first access.
  <https://github.com/KDE/kio-fuse/blob/master/DESIGN.md>
- **fuser** (Rust): the standard Rust FUSE crate; implements the kernel protocol in pure
  Rust by default (libfuse linkage optional since 0.16), callback API mirroring
  `fuse_lowlevel_ops`, kernel-cache invalidation notifications since 0.14. Actively
  maintained; a major typed-API overhaul landed in 0.17 (2026-02).
  <https://github.com/cberner/fuser>
- **ffmpegfs / mp3fs**: transcoding filesystems — each media file appears as its
  converted counterpart, generated on open into a growing per-file buffer. Their
  documented central problem is ours too: **stat must state a size before the bytes
  exist**, and the kernel clamps reads at the stat size, so a wrong size truncates the
  file for every reader. <https://nschlia.github.io/ffmpegfs/html/md_README.html>
- **archivemount / fuse-zip**: read-write archive mounts; both defer the expensive
  rewrite to unmount and warn about it — the negative precedent our commit-on-flush
  design avoids (§5.4). <https://manpages.ubuntu.com/manpages/questing/man1/archivemount.1.html>
- **PDF specifically: no maintained precedent found.** The niche is open.

## 3. Current restrictions, named

- **The authoring exclusion, as amended** (`CLAUDE.md`): incremental update of an open
  document is the permitted form of writing. Every write below is an append —
  §7.5.6: "changes shall be appended to the end of the file, leaving its original
  contents intact" — so the *mechanism* needs no new permission. What is new is the
  *operation set* (page insertion and deletion are RFC 0002's subject; this RFC only
  maps file verbs onto that layer's operations and inherits its scope decisions).
- **`pdf_syntax::Document` stays immutable**: the core below is read-model plus
  transform calls; nothing here wants document mutation. The rule's rationale (the
  oracle's purity) is untouched and untouchable by this design.
- **The sandbox** (`CLAUDE.md` principle 3) is not negotiable and is *load-bearing*
  here — §6. A KIO worker and a FUSE daemon are long-lived processes fed hostile bytes
  by anything that touches a folder; they are the most exposed surface this project
  would ship.

## 4. The virtual layout — one tree, both faces

The same tree whether reached as `pdf:/home/u/doc.pdf/…` (KIO) or as a mount
(`pdffs doc.pdf mnt/ && ls mnt/…`). All names are ASCII, generated, and stable within a
generation of the document (§5.4).

    doc.pdf/
    ├── pages/                    one extractable single-page PDF per page
    │   ├── 0001.pdf              (zero-padded ordinal; width from page count)
    │   ├── 0002.pdf
    │   └── …
    ├── renders/                  the same pages as rendered images
    │   ├── 150dpi/               one directory per offered resolution —
    │   │   ├── 0001.png            the audiocd formats-as-directories idiom
    │   │   └── …
    │   └── 300dpi/ …
    ├── images/                   the embedded image XObjects
    │   ├── 0001-01.jpg           original bytes where the stream is a
    │   ├── 0001-01.jp2             self-contained format (DCT → .jpg, JPX → .jp2)
    │   ├── 0003-02.png           decoded losslessly otherwise (Flate/LZW/CCITT/JBIG2)
    │   └── …                     (named page-ordinal – index within the page)
    ├── text/
    │   ├── 0001.txt              per page, UTF-8, byte-for-byte the extraction
    │   ├── …                       identity `pdf-retrieve` already asserts (ADR 0257)
    │   └── document.txt          the concatenation
    ├── attachments/              §7.11.4's embedded files, by their stated names;
    │   └── …                       a /Collection document gets its folder schema
    │                               instead of a flat list (the sidebar's rule, ADR 0202)
    └── meta/
        ├── info.json             §14.3.3's /Info, as pdf-retrieve JSON
        ├── xmp.xml               §14.3.2's metadata stream, raw bytes
        └── outline.json          §12.3.3, as pdf-retrieve JSON

Format decisions, made rather than left open:

- **`pages/*.pdf`** are complete single-page PDF files produced by the transform layer's
  page extraction: resources, fonts and inherited attributes carried, so the extracted
  file renders alone. This is the flagship: `cp` *is* page extraction.
- **`renders/`** offers fixed named resolutions rather than a mount-time knob, because a
  KIO URL has no mount options and two faces must show one tree. 150 dpi (poppler's
  `pdftoppm` default) and 300 dpi (the de-facto print-grade default, RFC 0004 §3) to
  start; the set is the core's to define.
- **`images/`**: pass the original stream through untouched where it is already a
  complete image file (DCTDecode, JPXDecode — re-encoding would be a lie about the
  bytes), decode to PNG where it is not. One file per image XObject; inline images
  (§8.9.7) in a later revision, stated not silent.
- **`text/`** is the extraction identity, deliberately: a caller that greps the mount is
  grepping the same bytes the oracle's text gates measure.
- Names use ordinals, not page labels: §12.4.2 labels ("iv", "A-2") collide and may
  repeat, and a filename must not. Labels appear inside `meta/outline.json` and can join
  the tree later as a symlink directory if wanted (open question 6).

## 5. Operations — what each verb means, and what is refused

### 5.1 Reads

Everything listed above supports `stat`, `open`, `read`, `listDir`. All content is
generated lazily and cached (§5.5); nothing walks the whole document to answer `ls
doc.pdf/` (the launch-path principle applies to a mount too: listing the root names six
directories and reads nothing but the page count).

### 5.2 Writes — supported, with their meanings

| verb | meaning | mechanism |
|---|---|---|
| copy a PDF **into `pages/`** | **insert its pages** at the position the target name states: `cp new.pdf pages/0004.pdf` inserts before the current fourth page (the incumbent 0004 and everything after shift up on the next listing) | transform layer: page insertion, saved as §7.5.6 append |
| **delete `pages/NNNN.pdf`** | delete that page | transform layer: page deletion |
| copy a file **into `attachments/`** | embed it (§7.11.4 name tree gains an entry) | incremental update — the operation the viewer's sidebar already performs in reverse |
| **delete `attachments/X`** | remove the embedded file from the name tree | incremental update |
| **overwrite `meta/info.json`** | set the Info entries it states | incremental update (open question 4 — include in v1 or not) |

Ordinal names are **positions, not identities** — the rule that makes insertion and
deletion coherent: after any write, the next listing renumbers. A file manager showing a
stale listing sees it refreshed by the change notification (§5.4).

### 5.3 Writes — refused, each with its reason, all four loud

- **`mv` within `pages/`** (reorder): refused in v1. Rename semantics under
  position-names are ambiguous (is `mv 0007 0002` "before old 0002" or "become new
  0002"?), and a file manager's drag-reorder emits rename storms this tree cannot make
  atomic. Reorder belongs to the transform CLI (RFC 0002) until a deliberate design
  exists (open question 3).
- **writes into `text/`**: refused. Editing text through a byte stream has no honest
  in-place semantics — that is RFC 0005's subject, with a caret, a line box and a font
  answer. The refusal message names that route.
- **writes into `images/`**: refused in v1. Replacing an image is a plausible transform
  operation later; the refusal says "not supported yet" rather than pretending.
- **writes into `renders/` and `meta/xmp.xml`**: refused; derived artefacts.

KIO reports refusals as `ERR_UNSUPPORTED_ACTION` / `ERR_WRITE_ACCESS_DENIED` with the
sentence; FUSE returns `EROFS` for the derived directories and `EPERM` with no message
channel — which is FUSE's poverty, and why the mount also logs each refusal's sentence to
its own stderr/journal. Trap 5's rule (unsupported input stays loud) applied to a mount.

One consequence said out loud rather than discovered: **§7.5.6 deletion does not destroy
bytes.** A deleted page or attachment is unreferenced, not erased — the producer's bytes
remain in the file under the append. The viewer's own appearance machinery already
documents this edge (the one edit an incremental update cannot express). Anyone deleting
an attachment *to remove its content* needs a rewrite ("vacuum"), which is a
generator-side operation and RFC 0002's to offer or refuse — this RFC only insists the
refusal/behaviour be stated where the user deletes.

### 5.4 Consistency — the file changes under the mount

The backing file is the single source of truth, and it can change three ways: our own
committed write, another program's incremental update, or a full rewrite.

- **Generation key**: (mtime, size, last `startxref` offset). Every operation validates
  the key before answering; a changed key rebuilds the virtual tree.
- **Commit point**: a KIO `put` commits when the worker's `put` completes (KIO's verb is
  already transactional). A FUSE write buffers; **validation and commit happen on
  `flush`**, whose error return reaches the application's `close()` — `release` reaches
  nobody, which is why it is only cleanup. (The archivemount/fuse-zip
  rewrite-on-unmount design is exactly what this avoids: no deferred surprise, every
  `close` an answered transaction.)
- **Invalidation**: the FUSE face watches the backing file (inotify) and issues
  `notify_inval_entry`/`notify_inval_inode` from a separate task — separate because
  issuing them synchronously from a request handler can deadlock against the kernel
  (documented libfuse hazard). The KIO face is per-request and needs only the key check
  plus a `KDirNotify` emission so file managers refresh.
- **Mid-read change**: an open virtual file keeps the generation it was opened under
  (its bytes are already materialised in the cache); the *next* open sees the new
  generation. No reader ever receives a splice of two generations.

### 5.5 Sizes and the cache

FUSE `stat` must state sizes for files whose bytes do not exist yet, and the kernel
clamps reads at the stated size (the ffmpegfs lesson, §2). Estimating is therefore not
an option — an under-estimate silently truncates a page.

- **Rule: no virtual file is stat'd before it is generated.** `stat` on
  `pages/0007.pdf` generates (or finds cached) the bytes and reports the true size.
  Directory listings are cheap — names and types come from the document's structure —
  and file managers stat lazily, so browsing stays fast and the cost lands on the first
  touch of each file, where the user expects work to happen.
- **Cache**: content-addressed by (generation key, path), bounded (memory plus optional
  disk bound, both explicit budgets in principle-3 style), shared between the two faces
  through the core.
- `text/document.txt` and other whole-document concatenations are the expensive corner;
  their generation streams page by page, and the same lazy-stat rule applies.

## 6. Sandbox posture — the broker and the confined parser

The frontend processes are, by construction, fed hostile bytes by anything that opens a
folder. KIO runs workers out of process (the `kioworker` host executable) but **applies
no sandbox to them** — no seccomp, no Landlock; a worker has the user's full privileges.
A FUSE daemon likewise. So the posture is the one this tree already built for its
windows (ADR 0713, `viewer-confined`):

    Dolphin / any app          shell / any program
          │ KIO socket               │ /dev/fuse
    ┌─────▼─────────┐          ┌─────▼─────────┐
    │ C++ KIO shim  │          │ pdffs daemon   │   two thin, privileged FRONTENDS:
    │ (WorkerBase)  │          │ (fuser, Rust)  │   file I/O, caching, verb mapping —
    └─────┬─────────┘          └─────┬─────────┘    and NOT ONE BYTE of PDF parsing
          │        C ABI / socket    │
          └──────────┬───────────────┘
              ┌──────▼───────┐
              │  pdf-vfs core │  layout, generation cache, transform calls
              └──────┬───────┘
              ┌──────▼──────────────┐
              │ confined worker      │  seccomp-BPF + Landlock + address ceiling:
              │ (pdf-view-worker     │  ALL parsing, rendering, extraction happens
              │  pattern, ADR 0713)  │  here; Query/Answer over a pipe
              └─────────────────────┘

- The frontends and the core never parse PDF bytes. They hold paths, verbs, caches and
  the wire protocol. The confined worker parses, renders, extracts — and is killable,
  budgeted and refusal-loud exactly as the window's worker already is.
- Writes: the confined worker *computes* the transform output (the §7.5.6 append bytes);
  the broker validates the frame (length, magic) and performs the actual file append —
  the worker keeps no filesystem, the broker keeps no parser.
- This also answers KIO's process model cleanly: the KIO worker process is our *broker*,
  and confinement does not depend on anything KDE does or does not provide.

## 7. The shared core, and the two thin faces

**`pdf-vfs`** (the core): defines the layout (one declarative table: path pattern →
generator → write mapping), the generation cache, the generation-key consistency rules,
and the broker side of the confined-worker protocol. Consumes the transform layer
(RFC 0002) for every write and for page extraction; consumes the existing readers
(interpretation, text, images, attachments, metadata) through the confined worker. The
faces contain *no* layout knowledge — adding `fonts/` one day is a core change that both
faces grow simultaneously.

**The FUSE face**: a Rust binary (`pdffs <file.pdf> <mountpoint>`, plus `--foreground`,
`--allow-other` off by default) on `fuser`'s default pure-Rust `/dev/fuse` path — no C
linkage at all. Toolchain risk: **low**. (Note: fuser's 0.17 typed-API rework is recent;
pin and vendor-watch.)

**The KIO face**: KF6 admits no Rust worker — no binding exists (the one experiment,
cxx-kde-frameworks, was archived in 2024 at 19 commits), and `WorkerBase` is a C++
class. So: a deliberately dumb C++ `MODULE` plugin subclassing `KIO::WorkerBase`,
embedded JSON metadata (`"protocol": "pdf"`), built with CMake + extra-cmake-modules,
linking `Qt6::Core` + `KF6::KIOCore`, installed to `lib/qt6/plugins/kf6/kio/`; every
operation forwards over a **C ABI** into the core — the `viewer-ffi` precedent, which is
exactly the boundary this tree already knows how to freeze and test. The shim owns the
Qt types (`QUrl`, `UDSEntry`); the core never sees them. Toolchain risk: **moderate** —
CMake, Qt 6 and KF6 headers enter the build of that one component; it lives outside the
cargo workspace (a `kio/` subdirectory with its own build), so the workspace stays pure
Rust and CI treats the shim as an optional artefact built where KF6 exists (the Arch
packages are plainly named `kio`, `kconfig`, … — `doc/environment.md` already notes the
naming).

**Registration/entry**: mime association `application/pdf` → protocol `pdf` (the
`archiveMimetype` mechanism), so Dolphin's "open archives as folder" behaviour can enter
a PDF exactly as it enters a tar. And one bonus the research surfaced: **kio-fuse
auto-bridges any KIO protocol for KIO-unaware apps** — so the KIO worker alone already
yields a (whole-file-cached) POSIX view on KDE systems. That is not a substitute for our
FUSE face (non-KDE systems, streaming semantics, our cache) but it means the KIO face
compounds.

**Order of construction — recommendation**: core + FUSE face first (pure Rust, testable
in this tree's own harnesses, no external toolchain), KIO shim second (thin by then, and
its layout is already fixed by the core).

## 8. Difficulty

| piece | grade | why |
|---|---|---|
| layout + read generation for `text/`, `meta/`, `attachments/` | **easy** | the readers exist (`pdf-retrieve`, the sidebar); this is plumbing |
| `renders/` | **easy** | the raster path exists; DPI is a parameter |
| `images/` | **moderate** | decode paths exist (incl. sandboxed codecs); passthrough-vs-decode policy and naming are new, careful code |
| `pages/*.pdf` generation | **moderate** | entirely the transform layer's page extraction (RFC 0002); this RFC's part — the face — is easy, but it *depends* on that layer's hardest read operation |
| FUSE face (fuser, stat/gen/cache, flush-commit, inotify invalidation) | **moderate** | well-trodden semantics, real care at sizes, generations and the notify deadlock |
| confined-broker split | **moderate** | the pattern is built (ADR 0713); adapting the wire to vfs queries is work, not research |
| KIO shim + packaging | **moderate** | small C++, but a new toolchain (CMake/Qt6/KF6) and a new install surface; risk is integration, not logic |
| write verbs (insert/delete pages, attachments) | **moderate–hard** | the transform layer bears the PDF-side burden; the face's burden is transactional honesty (flush-commit, renumbering, notifications) — subtle, testable |
| reorder-by-rename, image replacement, `text/` writes | **hard / declined v1** | ambiguous or dishonest semantics at the file-verb grain; stated refusals instead |

## 9. Open questions for the owner

1. **Scheme name**: `pdf:/` (specific, honest) or something format-neutral (`doc:/`)
   should other formats ever join? Recommendation: `pdf:/`.
2. **Write support default**: on, or opt-in per krarc's precedent ("enable write
   support")? Recommendation: reads always; writes opt-in at mount/config until the
   transform layer has soaked.
3. **Reorder**: is `mv`-reorder wanted enough to design its semantics (a `.order` file?
   accepting rename storms?), or does reorder stay with RFC 0002's CLI? Recommendation:
   stay with the CLI.
4. **`meta/info.json` writes** in v1: yes/no? (Small, symmetric, low risk — but the
   first write verb outside pages/attachments.)
5. **The deletion-privacy stance** (§5.3): is a content-destroying rewrite something
   RFC 0002 should offer (a deliberate, named generator-side operation), so this face
   can refer to it — or refused project-wide? Needs an owner sentence either way.
6. **Page-label aliases** (`pages/by-label/iv.pdf` as symlinks): wanted, or clutter?
7. **Resolution set for `renders/`**: 150 + 300 as proposed, or owner's own list?

## 10. Recommendation

Build `pdf-vfs` as the one core with the broker/confined split from day one; ship the
FUSE face first and the KIO shim second; reads everywhere, the five write verbs of §5.2
behind an opt-in, every refusal loud with its reason. The layout table of §4 is the
contract to review hardest — it is the piece both faces and every user script will
calcify around.
