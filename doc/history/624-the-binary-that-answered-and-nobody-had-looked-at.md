# 624 — The binary that answered, and nobody had looked at

Session 623's finding, taken to a line. The line is not in `pdf-model` and not in the tree at all.

## What the round was told to find

621 moved `hayro-jbig2` and `hayro-ccitt` onto pdfium's symbol-instance heuristic and reported
three crawled documents going from blank sheets to correct pages. 623 merged four branches, ran
the whole sequence on `main` with every gate green, and found all three refusing again with the
decoder's own sentence. It eliminated the merge, the pin, the worker, `pdf-sandbox` itself and
the installed copy — each by removing the suspect — and concluded that `pdf-model` must be
sending a shorter buffer, since the new cap is `segment_data_len × 32`.

## What is actually true

**`main` is correct. The three documents draw. They drew all along.** On this branch, which is
`main` plus nothing, `open_one` reports `unsupported []` on all three and the broadsheet in
`1653119.pdf` is a broadsheet.

623 was measuring through a **stale `pdf-sandbox-worker` that nothing in this repository puts
anywhere**:

```
/home/AI/cargo-target/pdf-viewer/release/examples/pdf-sandbox-worker
  1 042 760 bytes, dated hours before any of the four merged rounds' commits
  sha256 b6835b3e…, against the tree's 1 023 504 bytes and 522de66d…
```

`worker_program()` searches **beside the running executable first** and only then one directory
up. Cargo puts an example binary in `target/<profile>/examples/`, so for `examples/open_one` —
which is what a person reproducing one of these documents runs — *beside* is `examples/`, and a
copy left there once outranks every rebuild of `target/<profile>/pdf-sandbox-worker` for as long
as it exists. 623's rebuild, its refresh and its substitution of 621's own worker all went to the
directory above, so none of them was ever loaded.

**Attributed by removing the suspect properly**: one `open_one` binary, one document, the worker
named explicitly with `PDF_SANDBOX_WORKER`.

| worker | `1653119.pdf` |
|---|---|
| `release/examples/pdf-sandbox-worker` | `unsupported [Image { name: "Im0: JBIG2: too many symbol instances" }]`, 0 commands |
| `release/pdf-sandbox-worker` | `unsupported []`, 1 command, the page |

The request each was sent was logged through a wrapper and is **byte-identical at 263 275 bytes**,
which retires the hypothesis directly rather than by argument: nothing about `pdf-model` was in
question. Three copies of the current worker — the merge round's, this round's, and the installed
one under `target/` — are one sha256.

The stale copy is gone from the build directory. That is a five-second fix and it is not the
round.

## The fix, from the clause

`protocol.rs`'s magic has always proved the two processes speak the same **wire format**. It
cannot prove they are the same **build**, and that is the question that was costing pages: a
worker whose decoders are older answers every request perfectly well out of older decoders, and
a decoder's refusal from last week's binary is word for word a decoder's refusal from this one's.

§7.4.7 is why the distinction is not small:

> JBIG2 explicitly defines the requirements of a compliant bitstream, and thus defines decoder
> behaviour.

A conforming bit stream has one decoding and ISO/IEC 14492 is where it is defined, so a refusal
carrying a *number* — ten thousand symbol instances, sixteen thousand rows — is never this
standard's statement about the file. It is a budget belonging to whichever binary answered. Trap
5 says unsupported input stays loud; the other half, which had no mechanism, is that **a refusal
must be attributable to a build**.

So the greeting carries one. `crates/pdf-sandbox/build.rs` hashes the workspace `Cargo.lock` —
which pins `hayro-jbig2`, `hayro-ccitt` and `hayro-jpeg2000`, and is where a fix to one of them
arrives — together with every `.rs` file of the crate, and stamps sixteen hex digits into it.
Both ends of the pipe are this crate, so the constant is equal on both by construction unless the
binaries came from different trees. The magic goes to `PDFSBX04`; a disagreement is
`SandboxError::WorkerMismatch`, which names the worker's path, its identity and ours, and is not
`Undecodable`, whose whole display is the decoder's own words. It is **not a security control**
and says so in three places: the worker is the untrusted side and can send any sixteen bytes.

**One thing was measured rather than assumed, and it changed the design.** A worker of an older
format sends a *shorter* greeting, so a parent reading the whole of the new record at once waits
out the thirty-second request deadline for every image. Run against the stale binary that way, the
check stalled instead of answering. The magic is read on its own first now, and the same run takes
2.5 seconds and prints `the sandbox worker sent a malformed response: the greeting was not this
protocol's` on every affected image.

Demonstrated end to end in both directions:

- **another format**: the stale worker, against this build → every image names the greeting, fast;
- **another build**: two workers differing by one added comment → `the sandbox worker at … was
  built from a different tree than this program (it says fbbb3b7af626fae3, this build is
  b7dbec59ad5e92d3)`, with the healthy pairing green beside it. Repeating that experiment after
  editing `lib.rs` gave a different pair of digits, which is the mechanism rather than a wobble.

## The hole 623 was really about, closed with a program

A fix found by ranking the SafeDocs crawl is measured **once**, by the round that makes it, in a
tree without its neighbours' work; the corpus, oracle and quorra gates all walk `doc/pdf.js` and
name none of these documents. 623 drew the right two rules from that and wrote that
`doc/todo/03` carried them. **It does not** — `git show` on 623's commit is one file, its own
history file — which is the rule failing in the same session that stated it, and is the argument
for making it a program rather than a habit.

- **`doc/checks/fixed-documents.toml`** — one appendable block per document: path, page, the
  session that fixed it, the reports the page must and must not carry, an ink band, and the defect
  in one line with its ADR and clause.
- **`crates/pdf-model/tests/fixed_documents.rs`** — one command over every row, a `doc/todo/02`
  §2 line, and a `tools/state.sh fixed` section. Under half a minute.

**Two observables, and the second is not decoration.** Reports are what caught this; but a third
of the seeded documents were *silent* both before and after their fix — drawn black, blank or
inverted with nothing to say so — and a report-only check could not see one of them come back. The
ink is this tree's own number over its own raster, no reference in the room. On `0100223.pdf` it
reads 225.475 where session 603 recorded 225.476 through ImageMagick, which is corroboration that
the two formulas are the same quantity.

Seeded with **25 documents** from sessions 603, 613, 615, 619 and 621, each *re-measured* here
rather than copied out of a history file — which mattered: the history files state a post-fix
report status for only two of the five sessions, and four of the fixed documents do not close to
zero. Every one of the 25 reports nothing and every one is in band.

**It fails when the defect returns**, on both observables and with the worker named rather than
the file. Against the stale binary: `1653119.pdf` ink 0.000 against a band of 34.707 .. 36.707,
`3252105.pdf` 0.000 against 155.225, `6696861.pdf` **0.267 against 29.987** — the last being a page
that reports nothing either way and that no report-only check could have caught. Against a worker
of *this* format and another build, which is the case the greeting was extended for, **12 of the
25 fail with 32 complaints between them**: every document in the file that needs a sandboxed
codec.

## Ledger

§7.4.7 amended: the row's claim that the decoder's bound is gone **holds**, and now says how it
was checked and what disbelieved it. §7.4.6 and §7.4.9 gain the sentence they share — the same
pipe, the same hole, the same greeting. `spec-errata emit` over `doc/*.pdf` first: §7.4.7 has no
errata, §7.4.6 none, §7.4.9's two are the known editorial `except for` → `excluding` pair and
touch nothing written here.

## The sequence

Whole, because `pdf-sandbox` and `pdf-model` are in `doc/todo/02` §2's first row. `fmt` clean ·
`clippy --workspace --all-targets` silent under `RUSTFLAGS="-D warnings"` · `nextest` **2302
passed, 17 skipped** (six new tests and one new `#[ignore]`) · doctests · corpus **974 documents,
68 incomplete** · oracle **1794 pages, 907 agrees / 66 contradicted / 786 ambiguous** ·
`render-quorra` **957 pages, 932 agree / 23 differ / 2 refused** · text extraction **98.26%** ·
both censuses · dates · XMP · JPEG 2000 **14 identical, 13 differing, 3 not comparable** ·
conformance **9472 citations, 929 quotations**. `cargo deny check` clean, and both cross-target
checks for `pdf-sandbox` under `RUSTFLAGS="-D warnings"`.

**Every number is 623's, unmoved**, which is what a change to a handshake and a new gate line
should produce and is the only reading of it that would have been surprising had it differed.
The new line is `fixed-documents: 25 checked, 0 absent, 25 rows`.

## Owed

- **`viewer-confined`'s `pdf-view-worker` has the same greeting shape and the same staleness
  hazard**, and is not fixed here. The order is arguable: a stale view worker shows up as pixels
  a person is looking at, where a stale decode worker shows up as one image inside a page and a
  sentence that blames the file.
- CI is still unverified, as 623 left it.
