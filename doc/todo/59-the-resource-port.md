# 59 — The resource port: what a confined worker may be given, and by whom

Status: **built for fonts** by session 920 (ADRs 0880, 0881); the item stays open for what "What is
still owed" below names. Originally **accepted** by the project owner on 2026-09-04, in these
words: *"I think we need to rethink our 'no access to the filesystem' policy. what do you think about a clean layer, which
every implementation must (can?) overwrite. the cli would wrap the access with a flag. GUIs could
either have a setting, or ask the user. access to fonts might be reasonable without user
intervention."*
Priority: 50-band. Companions: [60](60-paths-a-document-names.md) (the tier this item deliberately
excludes), [61](61-what-a-library-asks-the-machine.md) (the class this item must not absorb),
`doc/todo/58` §4 (where ADR 0870 named the broker as the place a face could come from).
Clauses: §9.6.5.4, §9.8.1, §9.10.2 on substitution and its report; §14.11.5 for a later profile
provider.
Code: `crates/pdf-font/src/substitute.rs`, `crates/pdf-vfs/src/worker.rs`,
`crates/confined-transport/`, `crates/viewer-confined/`, `crates/pdf-sandbox/`.

## Why this exists

The confinement admits two syscalls beyond the interpreter's set (ADR 0812), and a worker that
looks for a font dies on the second of them. Sessions 902, 911, 914 and 917 each met that kill, and
914's fix — `pdf_font::substitute::no_machine_fonts()` before the lockdown — bought a live worker
with a **stated fidelity cost**: a confined mount draws a face the document names and does not
embed from the compiled-in Latin faces. Four documents in the first sixty of the corpus are in that
population (`XiaoBiaoSong.pdf`, `SimFang-variant.pdf`, `90ms_rksj_h_sample.pdf`,
`ThuluthFeatures.pdf`), and **the same code is `pdf-view-worker`'s**, so the confined viewer loses
whole pages rather than a glyph. That is the demand.

## The shape, which is the part that binds

**The layer is a port, not a permission.** The worker's syscall set does not change and no host can
change it: a worker asks the broker for a resource **by description** — a family, a weight, a set
of code points — and the broker matches, opens, and passes a **descriptor** over the channel
§7.5.6's document already crosses (`SCM_RIGHTS`, ADR 0812), which the worker reads positionally
with the `pread64` it already has. A font name out of an untrusted file therefore never becomes a
path lookup inside the process that parses untrusted bytes, and session 917's ten-class matrix keeps
meaning what it means.

**`can`, not `must`** — the owner's own parenthetical, answered. The trait's default
implementation provides nothing, so a host that ignores the layer is exactly the host we ship
today. Enabling is a deliberate act: a flag on the command line, a setting or a question in a
window.

**The floor stays.** A worker whose host provides nothing still renders, still substitutes from the
compiled-in faces, and still reports the shortfall under §9.10.2. Nothing here may turn a
substitution into a failure.

**Laziness is a requirement, not an optimisation.** `CLAUDE.md` principle 2 forbids system font
enumeration on the launch path. Resolution happens on the first miss; any index the broker keeps is
built when it is first needed and never at startup.

## Fonts without asking, and why that is defensible rather than merely convenient

This program has no script engine (`CLAUDE.md`'s JavaScript exclusion) and the worker has no
network, so **a document cannot observe which face matched or report anything about the machine**.
The usual objection to letting a document reach the font set is fingerprinting, and fingerprinting
needs a channel back. There is none. What remains is a real trade to write into the ADR rather than
leave implied: **the broker would parse the user's own fonts in an unconfined process**, which
moves attack surface in the wrong direction, though the input is the user's rather than the
document's.

## What landed, in session 920

1. **The port itself** — `crates/pdf-font/src/provider.rs`: the description (a family, a weight, a
   slope, the characters a script needs, and how many answers to pass over — never a path), the
   answer (the face's program and the file's own *name*), `faces_come_from` to arm a worker,
   `MachineFaces` for a host to state, and a default that provides nothing. The wire is
   `confined-transport`'s `frame::RESOURCE_REQUEST` / `RESOURCE_ANSWER`, answered inside
   `Host::read_frame`, so **neither protocol's vocabulary gained an arm and no broker call site is
   re-entered** — the cost ADR 0874 declined to pay for the *ask* level is avoided rather than
   paid.
2. **One matcher** — `substitute::machine_face`, which is this module's own families-by-endings walk
   and its covering search, factored so that the broker runs the same order an unconfined process
   runs. `provider::open_a_face` is a decode, that call, and a read.
3. **The hosts** — `pdffs --machine-fonts`, `pdf-viewer-confined --machine-fonts` (or
   `PDF_VIEWER_MACHINE_FONTS=on`, which is `viewer_host::MACHINE_FONTS_VARIABLE` and is what a window
   started from a desktop entry has), and `PDF_VFS_MACHINE_FONTS=on` for the KIO face, whose C
   boundary is an ABI of thirty-five functions (ADR 0868) and whose only channel is the environment
   (ADR 0875 says the same of its restriction level). Every one of them is **off** by default.
4. **The measurement** — `crates/pdf-vfs/examples/faces_on_the_port.rs`, over the four documents
   above and over any population a caller names, with the unconfined process as the reference and
   byte identity as the comparison. ADR 0881 has the figures.

**One thing did not land the way this file specified it, and the reason is a finding.** The resource
crosses as **bytes on the frame**, not as a descriptor: a descriptor works and then kills every debug
build, because `OwnedFd::drop` asks `fcntl(fd, F_GETFD)` before `close` and `fcntl` is not on the
allow-list. ADR 0880 §6 has the `strace`, and trap 32 is the general shape. Nothing about the port's
security property depends on which of the two it is — the broker opens, the worker never does.

## What is still owed

1. **The other resources.** ICC profiles are the obvious second (§14.11.5, and RFC 0006 §5.3's
   converter needs one). The port is shaped for more than one kind — the transport's request and
   answer carry opaque bytes and know nothing about fonts — but nothing but fonts speaks it yet.
2. **A way for a person to *choose*, rather than a flag and two environment variables.**
   `doc/todo/38`'s sentence binds here too: no user interface until the owner asks for one. What
   exists is what a command line and a desktop entry can carry.
3. **`installed_wider`'s walk over the port is bounded at `MAX_OFFERS` round trips**, which is past
   the preference list's length on every family and is a bound rather than a policy. If a family
   list ever grows past it the port would answer fewer candidates than the machine would.
4. **`CLAUDE.md` principle 3's amendment**, which is the owner's sentence to write:
   `doc/questions/Q24` proposes the exact wording. This round deliberately did not touch
   `CLAUDE.md`.
