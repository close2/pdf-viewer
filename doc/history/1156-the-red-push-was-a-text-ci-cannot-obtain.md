# 1156 — The red push was a text CI cannot obtain, and the snapshot shipped two programs of ten

2026-09-21. Files: `.github/workflows/ci.yml`, `pdf-archive/src/editions.rs`, ADR 1152, this record.

## The root cause, which was the same three times and is four now
`gh run view --log-failed` over every red `main` push. **2026-09-07** (34113007400) was
`render-raster`'s `dashed_close`: on a second device `wgpu-hal` reaches its GLES backend, whose EGL
call answers `BadDisplay` and is `unwrap`ped — the next push set `WGPU_BACKEND: vulkan` and was
green. **2026-09-12, 09-15 and today's 09-21** are one failure: `pdf-archive`'s
`every_citation_resolves_in_the_edition_a_part_two_file_adheres_to` panicking
"`doc/md/ISO_32000-1_2008.md` is not readable; unpack `doc/specifications.zip`".

The advice is the defect. The archive holds fourteen texts and that is not one of them — the owner
downloaded ISO 32000-1:2008 separately and `/doc/md/` ignores its conversion — so no unpacking
produces it and the step the message asks for had already succeeded. The loud-failure rule
(`doc/habits/reading-the-specification.md`) is about the texts the archive supplies; here it accused
CI of an omission CI could not repair. `optional_markdown` skips with the path and the reason
printed, the sweep that needs no specification moved above it so it still runs, and ADR 1152 keeps
open the better end state: the text in the encrypted archive, whose password is the owner's.

**Verified by reproducing CI's `doc/md` exactly**: the worktree's symlink replaced by a directory of
the zip's fourteen files. Before, the panic; after, the skip, and `cargo test --workspace
--no-fail-fast` under that mask reached every other test binary — which the red runs never did.

## Two programs of ten
The `snapshot` job built `quorra` and `pdf-sandbox-worker`; `doc/todo/02` §5 names ten binaries.
Linux now ships all ten plus `kio/`, macOS seven, Windows six. Each omission measured with `cargo
check --target <t> -p <crate> --bins`, not assumed: `pdf-vfs` does not compile for Windows (its
worker takes a descriptor over a Unix socket), `fuser`'s build script fails off Linux, the native
hosts need their toolkits. The KIO step is guarded end to end and cannot fail the job — the C ABI
library and header always, the plugin where the runner's KF6 built it. MANIFEST is generated from
the staging directory, never from a list in the workflow. **The layout was run, not argued**: the
staged directory mounted a document through `quorrafs` and served `pages`, `text`, `renders`,
`images`, `meta`, `attachments`; the same binary alone answered `EIO` naming `pdf-vfs-worker`.

## Two surprises for a later round
`viewer-confined`'s `a_host_drawing_marks_that_will_not_finish_interrupts_its_own_draw` fails here
nine runs in ten: it asserts a draw of 10 000 fills is unfinished after 2 s and 24 cores finish it.
Not CI's — a two-core runner is slower — and not mine. And `pdf-fuse` is still `pdffs` throughout,
usage line, diagnostics and `MountOption::FSName`, while its binary is `quorrafs`.
