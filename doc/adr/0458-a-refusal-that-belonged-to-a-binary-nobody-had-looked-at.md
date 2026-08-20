# ADR 0458 — A refusal that belonged to a binary nobody had looked at

Status: accepted, 2026-08-20. Session 624. Answers session 623's finding, which is
`doc/history/623-a-clean-merge-that-was-not-a-correct-one.md`, and discharges the two rules
`doc/todo/03` took from it. Amends §7.4.6's, §7.4.7's and §7.4.9's ledger rows and the note in
`doc/traps/oracle-and-references.md` about a missing worker.

## What was believed

Session 621 moved `hayro-jbig2` and `hayro-ccitt` onto the revision carrying pdfium's
symbol-instance heuristic in place of a flat 10 000, and reported three crawled documents going
from blank sheets to correct pages. Session 623 merged four branches, ran the whole sequence on
`main` — every gate green, every number identical to what the four branches had measured — and
then found all three documents refusing again with the decoder's own sentence, `JBIG2: too many
symbol instances`.

623's diagnosis removed suspects one at a time and was right about every one of them: the merge
had not dropped the dependency, the pinned checkout carried the heuristic, the worker was not
stale, `pdf-sandbox` was byte-identical between the two branches, and substituting session 621's
own worker changed nothing. What it concluded from that was that **`pdf-model` must be sending
the decoder a different buffer** — the cap is `segment_data_len × 32`, so a shorter buffer is a
lower ceiling — and it stopped there rather than name a line, which was the right call on the
evidence it had.

**The conclusion was wrong, and the last elimination is where it went wrong.** Substituting a
worker only removes the worker as a suspect if the substituted file is the one that runs.

## What is true

`main` is correct. The three documents draw. They drew all along.

The tree was measured with a **stale copy of `pdf-sandbox-worker` that nothing in the repository
puts there**: `/home/AI/cargo-target/pdf-viewer/release/examples/pdf-sandbox-worker`, 1 042 760
bytes, dated hours before any of the four merged rounds' commits, against the 1 023 504 bytes the
tree builds. `worker_program()` searches **beside the running executable first** and only then one
directory up:

```rust
let beside = directory.join(&name);
if beside.is_file() {
    return Ok(beside);
}
```

Cargo puts an example binary in `target/<profile>/examples/`, so for `examples/open_one` —
which is what a person reproducing one of these documents runs — *beside* is `examples/`, and a
copy left in that directory once outranks every rebuild of `target/<profile>/pdf-sandbox-worker`
for as long as it exists. Session 623's refresh, its rebuild and its substitution all went to the
directory above.

**Attributed by removing the suspect properly.** One `open_one` binary, one document, the worker
named explicitly:

| `PDF_SANDBOX_WORKER` | `1653119.pdf` |
|---|---|
| `…/release/examples/pdf-sandbox-worker` (the stale copy) | `unsupported [Image { name: "Im0: JBIG2: too many symbol instances" }]`, 0 commands |
| `…/release/pdf-sandbox-worker` (the tree's) | `unsupported []`, 1 command, the broadsheet |

Both binaries, and the request bytes each was sent, were compared: the request is byte-identical
at 263 275 bytes, so nothing about `pdf-model` was ever in question. Three copies of the *current*
worker — the merge round's, this round's, and the one installed under `target/` — are one sha256.

## The decision

**A worker that is not this build says so, in its own words, at the greeting.**

`protocol.rs`'s magic has always proved the two processes speak the same wire format, and it is
bumped when the format changes. It cannot prove they are the same *build*, and that is the
question that was costing pages: a worker whose decoders are older answers every request
perfectly well, out of older decoders, and **a decoder's refusal from last week's binary is word
for word a decoder's refusal from this one's**. There is nothing in the sentence that reaches the
page to say which.

The clause says why that is not a small distinction. §7.4.7 on JBIG2:

> JBIG2 explicitly defines the requirements of a compliant bitstream, and thus defines decoder
> behaviour.

A conforming bit stream has one decoding, defined outside this document by ISO/IEC 14492. So a
refusal with a *number* in it — ten thousand symbol instances, sixteen thousand rows — is never
something the standard states about the file; it is a budget belonging to whatever binary
happened to answer. Trap 5's rule is that unsupported input stays loud, and a loud sentence that
attributes a bound to the wrong party is only half of it. **A refusal must be attributable to a
build.**

So:

- `crates/pdf-sandbox/build.rs` hashes what decides the worker's answers — the workspace
  `Cargo.lock`, which pins `hayro-jbig2`, `hayro-ccitt` and `hayro-jpeg2000` and is where a fix to
  one of them arrives, and every `.rs` file of this crate — and stamps the result into the crate
  as sixteen hex digits. Both ends of the pipe are this crate, so the constant is equal on both by
  construction unless the two binaries were built from different trees.
- The greeting carries it, and the magic goes to `PDFSBX04` because the record's length changed.
- A disagreement is `SandboxError::WorkerMismatch`, which names the worker's path, its identity
  and ours, and says where to look. It is not `Undecodable`, whose whole display is the decoder's
  own words.
- **The magic is read on its own before the rest of the greeting.** A worker of an older format
  sends a shorter record, and a parent asking for the whole of this one at once waits out the
  thirty-second request deadline per image instead of answering. That was measured here, on the
  stale binary itself, and turned a two-second diagnosis into a stall.

**FNV-1a rather than a real digest, and that is a decision rather than laziness.** A build script
may not buy a dependency for an accident detector, and nobody is choosing the collisions: **this
is not a security control and must not be read as one.** The worker is the untrusted side of that
pipe and can send any sixteen bytes it likes. What it detects is a mistake.

**What the identity does not cover is stated rather than implied**: the compiler, its version and
the profile. Two builds of the same sources by different compilers agree here — which is right,
because they also agree about every image — and a stale binary always differs in one of the two
inputs that *are* covered.

## The second decision, which is worth more than the three documents

623's real subject is that **two branches touching no common line can defeat each other with
every gate green**, because the corpus, oracle and quorra gates walk `doc/pdf.js` and these
documents are in the SafeDocs crawl, which no gate names. A crawl fix is measured once, by the
round that makes it, in a tree that does not yet contain its neighbours'.

`doc/todo/03` took two rules from that, and a rule a round has to remember is a rule a round
forgets. They are a program now:

- **`doc/checks/fixed-documents.toml`** — one appendable block per document: the path, the page,
  the session that fixed it, the reports the page must and must not carry, an ink band, and the
  defect in one line with its ADR and clause.
- **`crates/pdf-model/tests/fixed_documents.rs`** — one command that runs every row, and a
  `doc/todo/02` §2 line that the merge round runs, since the merge is the only place the
  combination exists.

**Two observables rather than one, and the second is not decoration.** Reports are what caught
this. But a third of the seeded documents were *silent* both before and after their fix — drawn
black, blank or inverted with nothing to say so — so a report-only check could not see one of them
come back. The ink is this tree's own number over its own raster, with no reference in the room:
mean of `255 − luma` over page one at scale 1.0. It agrees with `doc/todo/00` step 7's ImageMagick
recipe to the thousandth where a history file records one, which is corroboration the formula is
the same quantity.

The bands are the measured value plus and minus 1.0 of 255. Every defect they stand against moved
a page by between 3.8 and 245 levels, so the band is about the defect rather than about
antialiasing; a round that legitimately changes what these pages draw moves the number and says
why in its own history file, which is the discipline any ratchet in this tree already asks for.

It is seeded with the documents sessions 603, 613, 615, 619 and 621 fixed, each verified against
this tree at seeding rather than copied out of a history file. Rows whose document is absent are
skipped and counted — `corpus-cache/` is a machine-local crawl — and a run that finds none of them
fails rather than passing quietly, which is the failure mode a check like this has.

## What was considered and not done

- **Changing the search order** so that `examples/` cannot shadow `target/<profile>/`. It would
  have fixed this instance and nothing else: the same hazard is a deployment upgrading a viewer
  and leaving the old worker beside it, which is exactly the arrangement the "beside the
  executable" rule exists to serve. The greeting catches both.
- **Bumping the magic whenever a decoder changes.** That is what failed. A format constant is
  bumped when the format changes, by a person who is thinking about the format; session 621
  changed the decoders and had no reason to touch it.
- **Doing the same for `viewer-confined`**, whose `pdf-view-worker` has the same greeting shape
  and the same staleness hazard. It is owed rather than done, and the argument for the order is
  that a stale *view* worker shows up as pixels a person is looking at, where a stale *decode*
  worker shows up as one image inside a page and a sentence that blames the file.

## Consequences

- Any `pdf-sandbox` change now rebuilds both ends, which they always did; what is new is that
  running only one of them is an error rather than a silence.
- One more build script in the tree, on the same footing as `pdf-font`'s and `pdf-spec`'s.
- `doc/todo/02` §2 gains a line, and the merge round gains a program instead of a memory.
