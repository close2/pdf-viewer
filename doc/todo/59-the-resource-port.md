# 59 — The resource port: what a confined worker may be given, and by whom

Status: **accepted** by the project owner on 2026-09-04, in these words: *"I think we need to
rethink our 'no access to the filesystem' policy. what do you think about a clean layer, which
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

## What is owed

1. The port itself: the request type (a description, never a path), the answer (a descriptor plus
   what identifies the face), the default implementation that provides nothing, and the wire arms
   in `confined-transport`.
2. The broker's matcher, lazy and cached, and where it lives so that both `pdf-vfs` and
   `viewer-confined` use one implementation rather than two.
3. The hosts: a flag for the command line, a setting for the windows, and whatever the KIO and FUSE
   faces can honestly offer.
4. The measurement `no_machine_fonts()` cost us, taken again with the port on: the four documents
   above, and the population `crates/corpus-classes` names — which both confined sweeps now walk,
   so the instrument that will say the port works is already there (ADRs 0878, 0879).
5. An amendment to `CLAUDE.md` principle 3's sentence — **as a clarification of what the broker may
   do, not a weakening of what the worker may** — once the port exists.

ICC profiles are the obvious second resource (§14.11.5, and RFC 0006 §5.3's converter needs one).
Not in this item; named so the port is designed for more than one kind.
