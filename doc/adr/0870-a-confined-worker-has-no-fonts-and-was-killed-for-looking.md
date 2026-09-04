# 0870 — A confined worker has no fonts, and was killed for looking

Session 914. Status: **accepted**. The first of this round's two records: the defect the read
side's corpus walk found on its first sixty documents, which is the viewer's as well as the
mount's.

## Context

RFC 0003 §6 and ADR 0847 put every byte of PDF parsing behind a process with no filesystem and no
network, and `doc/todo/58` §4 makes that the condition on shipping a face at all: "a mount is
entered by anything that touches a folder, and a file manager will open a document nobody chose to
open". The confinement is `pdf_sandbox::lockdown::Profile::Interpreter`, unchanged from the
viewer's, and `crates/pdf-sandbox/src/lockdown_linux.rs` states what it is: three mechanisms, of
which the third is "an allow-list of system call numbers — anything not on it **kills** the
process".

`crates/pdf-font/src/substitute.rs` is the other half of the story, and it was written for a world
where reading a directory is allowed to fail:

> Discovery is deferred behind a `OnceLock` because it walks the filesystem, which is exactly the
> kind of work `CLAUDE.md` forbids on the launch path.

and, one screen down, `let Ok(entries) = std::fs::read_dir(dir) else { return; };` — a machine with
no fonts installed is a *supported deployment*, and `find` "never fails" there because
`crate::standard` compiles §9.6.2.2's fourteen faces into the binary.

Those two designs are individually right and were never asked about each other.

## What the walk found

The nine-hundred-and-fourteenth session's first run of `crates/pdf-vfs/tests/read_corpus.rs` — the
read side's corpus walk, ADR 0871 — was limited to the first sixty documents of `doc/pdf.js`, as a
smoke test before the full population. Four of them killed the confined generator outright:

```
XiaoBiaoSong.pdf:      /renders/150dpi/0001.png: the confined worker stopped without answering
                       (killed by signal 31 (SIGSYS: a system call the confinement forbids))
SimFang-variant.pdf:   /text/0001.txt:            … the same
90ms_rksj_h_sample.pdf: /renders/300dpi/0001.png: … the same
ThuluthFeatures.pdf:   /text/document.txt:        … the same
```

and the kernel's own audit line names the call, as it did in ADR 0865:

```
audit: type=1326 … comm="pdf-vfs-worker" sig=31 arch=c000003e syscall=257 …
```

`syscall=257` is `openat`. **This is not ADR 0865's allocator arena**: that one is `glibc`'s, is
made by a `rayon` pool thread at its first allocation, and was fixed by `MALLOC_ARENA_MAX=1` at the
spawn. This one is the program's own, and the documents say which program: three CJK faces and an
Arabic one, each *named* by a `/BaseFont` and not embedded. `pdf_font::substitute::find` is asked
for a stand-in, `catalogue()` walks `/usr/share/fonts`, `read_dir` issues `openat`, and the filter's
action is `SECCOMP_RET_KILL_PROCESS`.

**The `else` branch never runs.** That is the whole finding, and it generalises past fonts: code
written to shrug off an `Err` from the filesystem is *not* robust inside a confinement, because a
confinement of this kind does not produce an `Err`. It produces a corpse. `pdf_vfs::Vfs` then
throws the generation away, which is exactly right and no consolation: the mount lost the page, the
text and every other file of that document's generation, and the next question paid for a fresh
worker that would die the same way.

**It reaches the viewer.** `viewer_confined::worker::confine` has the identical shape, and
`pdf-view-worker` is what draws a page for the confined viewer. A user opening a document that
names an uninstalled CJK face loses the *page*, not a glyph. Nothing in `viewer-confined`'s own
suite could see it: its documents embed their fonts.

## Decision

**A process that is about to confine itself states that the machine's fonts are unreachable, and
`pdf-font` obeys the statement.**

`pdf_font::substitute::no_machine_fonts()` sets a process-global flag; `catalogue()` answers with
an empty slice under it, and `read_cached` refuses to open a path. Both `pdf_vfs::confine` and
`viewer_confined::confine` call it in the block that already holds
`pdf_sandbox::set_isolation(Isolation::InProcess)`, `available_parallelism` and
`address_space_in_use` — the three things those functions already ask *before* the lockdown for
exactly this reason, each with a comment saying "this is the one place it can be asked". The font
question is the fourth, and it belongs in the same paragraph.

### Why a stated flag rather than the alternatives

- **Widening the filter to permit `openat`** would give a confined process a filesystem, which is
  the one thing the confinement exists to remove. `lockdown_linux.rs` says it in its own words:
  "`openat` and `socket` are simply not reachable, so there is no filesystem and no network
  irrespective of what any path or address would have permitted". A Landlock rule for
  `/usr/share/fonts` cannot help, because seccomp is decided by call number and never sees the
  path.
- **Warming the catalogue before the confinement** would fix the walk and not the read: the
  catalogue holds paths, and the *bytes* are read lazily by `read_cached` — inside the
  confinement, at the same `openat`. Warming the bytes means reading the machine's whole font
  collection at every worker start, on the viewer's time-to-first-page path, to answer a question
  most documents never ask.
- **Catching the signal** is not available: `SECCOMP_RET_KILL_PROCESS` is not a signal a handler
  can see.

### What it costs, and the cost is real

A confined worker now behaves exactly like a machine with no fonts installed. For a document whose
fonts are embedded, or whose non-embedded fonts are §9.6.2.2's fourteen, nothing changes at all —
the compiled-in faces are already what `find` answers first there (ADR 0133), and the corpus walk
holds every one of those pages byte-for-byte identical between the two transports.

For a document naming an uninstalled face, the page is drawn from the compiled-in Latin faces
instead of from the machine's wider one. **This is a fidelity loss and it is not silent**:
`pdf_model::interpret` already reports it, per font, with the count of characters lost —

> font /{name} is substituted and the face this machine offers draws none of the {n} character(s)
> it is asked for (§9.10.2)

— which is the loudness trap 5 asks for, and it is the same report the same document produces
today on a machine that has no CJK font installed. What is gone is the *kill*.

**The fix that keeps the fidelity is the broker's**, and it is named rather than done: the broker
is unconfined, it already opens the document and passes the descriptor across with `SCM_RIGHTS`
(ADR 0812), and a face that can hand over a document can hand over a font file. `doc/todo/58`
carries it as the shortfall this round created and did not close.

### The transport claim is now qualified, and the walk says so

ADR 0841 §2's "the confinement is a transport change and nothing else" is still true of every
question in `crate::worker::Query` — but only when both ends are in the same font posture, because
a substituted face is read from the *machine* and the two ends no longer have the same machine.
`tests/read_corpus.rs` therefore calls `no_machine_fonts()` in its own process before it reads
anything, and says why: comparing a confined answer with an unconfined one would otherwise be
comparing two machines, and would report a *fidelity* difference in the column meant for a
*transport* difference.

## Consequences

- `pdf-vfs` and `viewer-confined` both take a direct `pdf-font` dependency, with the reason in the
  manifest beside it. Both already reached `pdf-font` transitively through `pdf-model`; what is
  new is that each now *states* something about substitution, which is a thing a reader of those
  manifests should be told.
- Two probes, one per confined worker, calibrated against the tree without the fix (trap 13):
  `pdf-vfs`'s `a_confined_generator_can_stand_in_for_a_font_it_cannot_look_up` and
  `viewer-confined`'s `a_confined_interpreter_can_stand_in_for_a_font_it_cannot_look_up`. With the
  `no_machine_fonts` line commented out, each is killed by `SIGSYS` — `ExitStatus(unix_wait_status(159))`,
  which is signal 31 — and with it in place each exits `ALLOWED`. They ask for a `Request` whose
  `standard` is `false`, because that is the flag that decides whether the machine is consulted
  before the compiled-in faces or after them.
- `no_machine_fonts` cannot be undone, deliberately: it stands beside a confinement that cannot be
  undone either, and a switch that could be turned back on would be a way to reach the syscall
  after the filter is installed.
- **The general lesson is bigger than fonts and belongs to every crate a confined worker links.**
  A fallible filesystem call is not a *safe* filesystem call inside this confinement. Anything
  reached from `pdf-model`, `pdf-font`, `pdf-render` or `render-cpu` that opens a path on a rare
  input is a latent kill, and the population is not "code that unwraps" — it is "code that opens".
  `doc/traps/instruments-and-reports.md` gains this as trap 31.
