# 623 — A clean merge that was not a correct one

The second merge round, and the first one that found something. `doc/todo/02` §2's rule is that
green in a worktree establishes nothing about `main`; session 618 ran the sequence and everything
held, which made the rule look like a formality. It is not.

## What was merged

`round-619`, `round-620`, `round-621`, `round-622`, branched from `c34f46ed`. **Four clean merges,
no textual conflicts** — the four files they shared (`doc/conformance/ledger.toml`, `doc/todo/03`,
`doc/todo/_image-codecs-and-the-sandbox.md`, `doc/traps/oracle-and-references.md`) each took every
branch's edit without one.

## The whole sequence on `main`, after the merge

`fmt` clean · `clippy --workspace --all-targets` silent under **`RUSTFLAGS="-D warnings"`** ·
`nextest` **2296 passed, 16 skipped** · doctests · corpus **974 documents, 68 incomplete** · oracle
**1794 pages, 907 agrees / 66 contradicted / 786 ambiguous** · `render-quorra` **957 pages, 932
agree / 23 differ / 2 refused** · text extraction 98.26% · both censuses · dates · XMP · JPEG 2000 ·
conformance. **Every gate green and every number identical to what the four branches measured
separately.**

And that is exactly why the finding below matters: **no gate in this project could see it.**

## The finding: 621's fix does not survive the merge, and nothing failed

Session 621 moved `hayro-jbig2` and `hayro-ccitt` onto the revision this tree already pins for
`hayro-jpeg2000`, because it carries pdfium's heuristic in place of a flat 10 000 symbol-instance
cap. It reported three crawled documents going from blank sheets to correct pages, with
`unsupported []` on all three.

On the merged tree **all three refuse again**, with the same sentence they refused with before:

```
1653119.pdf  unsupported [Image { name: "Im0: JBIG2: too many symbol instances" }]
3375154.pdf  unsupported [Image { name: "ForeGround: … /Mask did not decode: JBIG2: too many symbol instances" }]
3252105.pdf  unsupported [Image { name: "Im2: … /Mask did not decode: JBIG2: too many symbol instances" }]
```

**What was ruled out, in order, because a diagnosis this project trusts is one that removed the
suspect:**

- **Not the merge dropping the dependency.** `git diff round-621 HEAD -- Cargo.toml Cargo.lock` is
  empty. Both files are byte-identical to 621's own branch.
- **Not the pin being wrong about the fix.** The pinned checkout's
  `hayro-jbig2/src/decode/text.rs` carries the heuristic, commented with pdfium's own URL:
  `let max_instances = header.segment_data_len.saturating_mul(32);`. The flat cap is gone.
- **Not a stale worker** — trap 10's shape, and it did fire once here on the first attempt, which is
  why the check was repeated after an explicit `cargo build --release -p pdf-sandbox --bins`. It
  still refuses with a worker built after the merge.
- **Not the worker binary at all.** `pdf-sandbox` is byte-identical between `round-621` and `main`
  (`git diff` empty), the two release workers are the same size, and **substituting 621's own
  worker into `main` changes nothing.**
- **Not the installed copy shadowing the built one.** `target/pdf-sandbox-worker` was refreshed and
  re-tested.

What is left is the **caller**: `pdf-model` sends the decoder something different on `main` from
what it sends on `round-621`. The cap is `segment_data_len × 32`, so a shorter buffer is a lower
ceiling — and `crates/pdf-model/src/image.rs` and `inline_image.rs` were changed by **619** as well
as by 621, 619's change being about a derived length a window was too short to check. Reverting
619's two files onto `main` does not compile — they are coupled to the rest of that round — so the
attribution stops at *the image and stream path in `pdf-model`, where two branches met*, and does
not name a line. **That is where it is left, because naming a line would be a guess.**

**`main` is not worse than it was**: these three documents refused before 621 as well. What is lost
is an improvement, and what is worth more than the improvement is the shape.

## The shape, which is the round

**Two branches that touch no common line can still defeat each other, and every gate stays green.**
The corpus and the oracle walk `doc/pdf.js`; these three documents are in the SafeDocs crawl, which
no gate names. The ranking that found them is run by hand, once a chunk, in the round that takes the
chunk — so a crawl fix is measured **once**, by the round that makes it, in a tree that does not yet
contain its neighbours.

Two rules follow and both are cheap:

- **A round that fixes a document no gate covers records the check, not just the result** — the
  command and the expected line — so the merge round can re-run it. 621's report said
  `unsupported []`; that sentence is what made this findable in ten minutes.
- **The merge round re-runs those checks.** It is the only place the combination exists.

`doc/todo/03` carries both, and the three documents are named there as owed.

## Also settled here

**`cargo deny check advisories` was failing on a yanked `arrayref 0.3.9`** reached through
`tiny-skia`, which 621 reported and correctly declined to attribute to itself. It is **clean now
with `Cargo.lock` unchanged** — `cargo update -p arrayref` locked zero packages and the check went
green, so the failure was a **stale crates.io index** rather than a yank. Worth knowing about the
instrument: an advisories result is only as current as the index it reads, and a failure can be an
artefact of not having refreshed one. `deny` was the one CI job that had been passing.

## Owed

- The three JBIG2 documents, with the attribution above.
- CI is **still unverified**. 614's five fixes have never faced a run; the token in the tree is
  read-only for contents.
