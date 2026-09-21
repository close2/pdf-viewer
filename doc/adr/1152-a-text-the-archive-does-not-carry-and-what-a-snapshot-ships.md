# 1152 — A text the archive does not carry, and what a snapshot ships

Status: accepted and **built** in session 1156.
Context: `.github/workflows/ci.yml` (`snapshot`, `publish-snapshot`),
`crates/pdf-archive/src/editions.rs` (`optional_markdown`,
`every_citation_resolves_in_the_edition_a_part_two_file_adheres_to`).
Builds: ADR 0187 (the specifications are encrypted, not redistributed), ADR 0194 (three
platforms), ADR 0222 (one invocation, not three), ADR 0450 (a red `main` nobody was watching),
ADR 0869 (the KIO build takes its library's path rather than searching).

## 1. Four pushes went red on a text CI cannot obtain

`main` has been red since 2026-09-12 — runs 34707900980, 34932151457 and 35574732327, three
pushes and every one of them the same job, the same test and the same sentence:

> `…/doc/md/ISO_32000-1_2008.md` is not readable (No such file or directory); unpack
> `doc/specifications.zip`

The advice is the defect. `doc/specifications.zip` holds fourteen texts and ISO 32000-1:2008 is
not one of them: the owner downloaded it separately on 2026-09-07 (`doc/third-party-data.md`),
`/doc/*.pdf` and `/doc/md/` ignore the PDF and its conversion alike, and nothing has ever put it
into the archive. **No unpacking produces that file**, so the step the message asks for had
already been taken and had already succeeded.

The rule the test was written to — a test that cannot find `doc/md/` **fails loudly** rather than
skipping (`doc/habits/reading-the-specification.md`) — is right, and it is about the texts the
archive supplies: there, an absence *is* a developer who skipped the unpacking, and saying so is
better than a green run that checked nothing. It was applied to the one text the archive does not
supply, where the same absence means the opposite, and the test then accused CI of an omission CI
could not repair.

**So the rule keeps its subject and gains its complement.** `markdown` still panics, for the
fourteen. `optional_markdown` returns `None` for ISO 32000-1:2008, the test prints which file is
missing and why the archive does not hold it, and the half of the test that reads no
specification — the sweep that forbids a `§` after the spelled-out earlier edition — is moved
above the skip so that it runs everywhere. `coverage.rs`'s `doc/pdfa/` test and the KIO worker's
build test are the same shape, in the same crate.

The alternative was to put the text into `doc/specifications.zip`, which would restore the loud
failure and is the better end state. It needs the archive's password, which is a repository
secret; it is the owner's to make, and this ADR is not in its way.

## 2. The snapshot shipped two programs out of ten

The `snapshot` job built `quorra` and `pdf-sandbox-worker` and called that "both executables".
`doc/todo/02-every-round.md` §5 — what a person runs — names ten binaries and two libraries. The
eight that were missing include every program that is not a viewer: the transform suite whose
`archive` verb is the PDF/A conversion, the retriever whose `archive-check` is the PDF/A
validation, the FUSE mount, the two native hosts and two of the three workers.

**What a target ships is now a matrix line**, read off in one place rather than reconstructed from
a `case` in the packaging script, and each omission was *measured* with `cargo check --target <t>
-p <crate> --bins` rather than assumed:

| | Linux x86_64 | macOS aarch64 | Windows x86_64 |
|---|---|---|---|
| `quorra`, `quorra-confined`, `quorra-transform`, `quorra-retrieve` | yes | yes | yes |
| `pdf-sandbox-worker`, `pdf-view-worker` | yes | yes | yes |
| `pdf-vfs-worker` | yes | yes | **no** |
| `quorrafs` | yes | **no** | **no** |
| `quorra-gtk`, `quorra-qt` | yes | **no** | **no** |
| `kio/` | library and header always; plugin where KF6 is | **no** | **no** |

`pdf-vfs-worker`'s absence on Windows is the code's own and not a package's: `pdf-vfs` hands an
open document to its worker as a *descriptor* over a Unix socket, so `FileBytes::descriptor` and
`File: From<ReceivedDescriptor>` do not exist there and the crate does not compile. `quorrafs`
fails earlier still — `fuser`'s build script asks `pkg_config` for a kernel interface neither of
the other two runners has. The native hosts are built where their toolkits' development files
are, which is the same reason the `check` and `test` jobs install them.

## 3. Flat, because a worker is looked for beside the executable

Three of the ten are workers their programs *spawn*, and
`confined_transport::program_beside_executable` looks beside the running binary and then one
directory above it. So the archive is flat and its README says to keep it that way. Verified by
unpacking the staged directory and mounting a document through it: `quorrafs` served `pages`,
`text`, `renders`, `images`, `meta` and `attachments`; the same binary alone in a directory
answered `EIO` and named the worker it could not find.

**The MANIFEST is written from what is in the staging directory**, never from a list kept in the
workflow. A list kept beside the packaging drifts from what was built, and this tree has already
paid for that: three of §5's ten names were pre-rename for months and nothing failed, because the
stale artefacts were still lying in the shared build directory for `install` to find. A manifest
derived from `ls` cannot name a file the archive does not hold, and a binary added to the matrix
without a description shows up as a row with an empty one.

## 4. The KIO plugin ships as its library, and as itself only where KDE was

RFC 0003 section 7 puts the KIO shim outside the cargo workspace and says CI should treat it as
"an optional artefact built where KF6 exists". Two facts decide the shape. A KF6 plugin is bound
to the ABI of the KDE it was compiled against, so `pdf.so` from a runner is of use to somebody
running that runner's distribution and of none to anybody else; and `kio/CMakeLists.txt` takes the
path to `libpdf_vfs_ffi.so` as a *required* variable rather than searching for it, precisely so
that a revision mismatch is loud (ADR 0869) — which means the library is what a local build needs.

So the Linux archive always carries `kio/libpdf_vfs_ffi.so` and `kio/quorra_vfs.h`, the half that
needs no KDE and that somebody rebuilding the plugin links against, and carries `kio/pdf.so` as
well when the runner's package set could build it. The step is guarded end to end: the
development packages are *tried* under both names Debian and Ubuntu have used, a miss prints a
sentence, and the job carries on. **It cannot fail the snapshot**, which is what "optional
artefact" has to mean in a job whose output is a published release.
