# Traps: instruments, reports, and reading what a gate prints

Status: **standing** — each is a mistake somebody actually made in this tree.
Read by: a round that runs a gate, believes a number, adds a report, sweeps the tree for a class of
defect, or adds a lint exception.
`doc/todo/02-every-round.md` §2 owns the gate sequence; this file is what the gates and their
numbers do wrong.

`doc/HANDOVER.md` is the index and names which group holds which trap. **Every trap keeps its
number**, because `crates/`, `tools/`, `doc/conformance/ledger.toml` and dozens of ADRs cite them
by number and an ADR is not edited to follow a file that moved underneath it (ADR 0232 §2).

## How to read what `tools/state.sh` prints

**The numbers are not in any document** — `tools/state.sh` prints them, and that is deliberate
rather than tidy: a round told to *measure* something must not be able to read the answer in a
document, because a table of gate figures is exactly what lets a round write "unchanged" without
running anything (ADR 0281).

- **Counts are ratcheted**: they may only improve, except where a rise is a *new report* and is
  written down as one (trap 5).
- **A gate's own number, never arithmetic beside it.** This project has twice carried a sum that
  was stale while the gate figure two lines above it was current, which is why `state.sh` filters
  a gate's output and adds nothing up. If you need a total, print it.

## Traps

### 7. `#[expect]`, never `#[allow]`

Every lint exception is `#[expect(..., reason = "...")]`. It errors when it stops being necessary,
which has already removed several. A bare `allow` hides that forever.

### 10. The sandbox worker is a separate binary, and Cargo will not rebuild it for you

`cargo test -p pdf-model` builds pdf-sandbox's *library*, not its `pdf-sandbox-worker` binary —
Cargo never builds another package's binaries. So the tests run against whatever worker was last
compiled. Not hypothetical: the seventh session inverted the black-and-white sense of every JBIG2
sample and the test passed. `cargo test --workspace` or `cargo build -p pdf-sandbox --bins` builds
it. Both gates fail loudly if the worker is *missing* — and a missing worker and a stale one look
nothing alike.

**And there are now two of these, in a profile of their own.** Since the three-hundred-and-eighty-fifth
the corpus gates run under `--profile gates`, so the worker they spawn is `target/gates/pdf-sandbox-worker`
and not the release one; and `pdfref-hayro`, which the oracle spawns for a fourth reading, is a second
program under the same rule. That one is worse than trap 10's original shape rather than better: it
**fails silently**. `Reference::Hayro` votes on nothing, so its absence leaves every verdict intact and
only removes a picture — which is how it went unbuilt by `doc/todo/02` §2 for its whole life and was
noticed by a reference-render count falling 861 with nothing else moving (ADR 0222). Both are lines in
§2 now, and the tell is the same one trap 10a names: the hit rate.

### 10a. A cached reference render is a fourth thing that can be stale

The key is built from the invocation itself plus the renderer's version and the document's
SHA-256, so **a flag not in the key is a flag not passed to the renderer either**. What it cannot
see is a renderer whose output changes while its version string does not. **The variable names a
*directory* and only the literal `off` disables it** — `PDFREF_CACHE=on` silently starts a fresh
319 MB cache in a directory called `on`. **The hit rate is printed and it is the tell**: under 99%
on an unchanged tree means the corpus or a renderer moved. A remembered *timeout* is the one entry
whose truth decays, counted separately and expiring after a week.

### 11. A report is only as good as the condition it fires on

Trap 5's other edge. The reflex is to report whenever the unimplemented thing *could* be involved.
Four instances: §9.3.8's text knockout named 7 documents on one of the clause's two conditions and
took **three agreeing pages out of the gated set**; §11.6.2 named six, three of which set an alpha
to *zero* so there are no two portions to composite; §11.7.4's overprinting was 63 documents and
six `silent` rows and the honest condition has **no members** on this device; §12.5.6.19 fired
where the clause asks for nothing at all, naming 23 documents.

**Derive the condition from the clause, print what it matched before trusting the count, and cost
it in gated pages** — a page that reports is a page the oracle stops judging. **Both of §9.3.8's
conditions outlived the report**: they are what decides whether the implementation builds a group.
A condition worked out for a report is worth keeping when the feature lands. And the reverse worry
is real: **a report can hide another report** — `knockout_smask.pdf`'s knockout gap was covered by
its soft-mask report for four sessions.

### 13. A sweep for a defect must be run against the defect before it is believed

A round told to look for a class of defect writes a grep, gets a handful of hits, reads them and
reports the tree clean. **That is a measurement with an instrument nobody calibrated**, and the
six-hundred-and-fourth session calibrated one and watched it fail.

The class was ADR 0438's: a byte string made into text by a lossy route and then used to *decide*
something. The obvious sweep looks for the conversion — `from_utf8_lossy` inside a `get`, a `==`, a
`match`. Run against a scratch copy of the very files the defect had lived in, at the revision
before the fix, **it prints nothing**: the conversion was in one function and the lookup was in
another, and no grep over a single line joins two functions.

The sweep that worked inverted the question and looked at the *decision* — every dictionary lookup
whose key is not a string literal — because ISO's own keys are literals in this source, so a key
that is not one came out of a file. That sweep names the planted defect five times, and it found
six more sites the first one could not see.

So: **plant the defect back and confirm the sweep names it.** A scratch copy and `git show
<commit>^:<path>` is the whole cost, and without it "the sweep came back clean" is a sentence about
a grep rather than about the tree. ADR 0439 has both sweeps as commands.

## Things worth knowing

- **The sandbox is a flag and the default is the safe one.** `--no-sandbox` trades panic
  containment and a memory ceiling, not memory safety. There is deliberately no path that falls
  back to in-process decoding when the worker fails to start.
- **Debug builds are ~15× slower here**, and it changes what a test can assert: the corpus gate is
  2 s in release and minutes in debug. Run timing assertions in release and say so.
