# 1005 — Five shapes, and the decision each one needs

Session 984. Status: **proposed** — every section below is a proposal for the project owner, with
its cost, and none is taken here. This round was read-only over `crates/` and `tools/`.
Context: `doc/reviews/984-direction-and-boundaries.md` is the review these come from and carries
the evidence; this file carries only the decisions and what each costs.

The owner asked whether the code is going in the right direction, whether the boundaries are in
the right places, and whether small fixes are papering over a big-picture problem. The review's
answer is that the direction and the boundaries are right and that five *shapes* — not hacks —
are what a hundred small fixes would paper over. Each is one decision.

## 1. `pdf-model` is split along the seams it already has

**Proposal.** Two extractions, mechanical, one round each:

1. `crates/pdf-signature` — `signature.rs`, `cms.rs`, `x509.rs`, `der.rs`, `bigint.rs`,
   `pkcs1.rs`, `pss.rs`, `dsa.rs`, `ecdsa.rs`, `eddsa.rs` (9,906 lines), depending on
   `pdf-syntax` for `/ByteRange` and the object reader, and taking the twelve cryptographic
   packages with it. `pdf-model` depends on it for `Signature`; the four external consumers
   (`viewer-core/src/notes.rs`, `viewer-core/tests/headless.rs`, `pdf-transform/src/lib.rs`,
   `pdf-archive/src/table/interaction.rs`) change an import path.
2. `crates/pdf-colour` — `colour.rs`, `icc.rs`, `function.rs`, `shading.rs`, `mesh.rs` (13,098
   lines), depending on `pdf-syntax` and `pdf-render` (whose `MeshRaster`, `ShadingProgram` and
   `BlendingSpace` it already produces for).

`xmp.rs` (2,795 lines, the tree's one XML parser) and the interpreter (`content/`, 16,982 lines)
stay: the first is small, the second *is* the model's job.

**Why.** Principle 4 asks for one stated responsibility per crate; `crates/pdf-model/src/lib.rs:1`
states one and the crate carries six. `doc/PLAN.md:329-337` already records the argument for a
crate boundary ("self-contained, independently testable and independently fuzzable") and applied
it to CMaps. Every consumer of a page tree links `p521`; every change to `cms.rs` is a
whole-sequence round under `doc/todo/02` §2 rule 2.

**Cost.** The conformance ledger names code sites and the gate verifies they exist: 72 rows name a
signature-stack path, 110 a colour-stack path (`grep -cE` over `doc/conformance/ledger.toml`).
Each extraction is therefore one commit that moves files and rewrites those rows — a `sed`, held
by the gate that already checks the result. `doc/crate-map.md` gains two rows; `doc/todo/02` §2
rule 2's list of seven crates becomes nine (both are under everything the corpus gates rasterise,
the colour one certainly). Fuzz targets that name `pdf_model::cms` move with it. No behaviour
changes, which the whole sequence can confirm in one run.

## 2. A comment carries the current reason; the history lives in the ADR

**Proposal.** One rule, added to `doc/todo/02` §7 beside the three habits that bind every round:

> A comment in code states the reason the code is as it is, today, and cites the ADR that argued
> it. It does not record what the comment used to say, which session changed it, or that an
> earlier reading was wrong: that is the ADR's and `doc/history/`'s. A retired sentence is
> deleted, not annotated.

And a bounded rewrite of the four navigational documents as *what is* texts, one round each,
spread over the fifty-round run: `doc/PLAN.md` (which today says Qt was dropped, lists ten crates,
and says the tree is not a git repository), `doc/crate-map.md` (no row for `pdf-archive`; a row for
a crate that does not exist), `doc/state-of-play.md`, `doc/HANDOVER.md`.

**Why.** 1,353 session ordinals and 3,744 ADR references inside Rust comments; 261 comment blocks
of fifty lines or more; corrections appended to the sentence they correct (`crates/pdf-model/src/lib.rs:24`).
A student cannot read a function without the chronology, which is the opposite of principle 4's
aim, and the project is answering with prose-policing instruments (`tools/conformance` has twenty
sweep binaries) — an arms race the prose is winning: the last 150 commits put 26% of their lines
into `doc/`.

**Cost.** The rule costs nothing to state and is checkable by the existing `retired` sweep in the
other direction (a comment that names a session is a comment carrying history). The rewrite costs
four rounds of reading and writing, and it costs something real: the sweeps that find retired
claims by their phrasing will find fewer, because the phrasing goes. That is the intended effect,
not a loss of coverage — the claims move to `doc/history/`, which no round reads to do its work
(`doc/history/README.md:4-6`).

## 3. Every `#[ignore]`d walk is in the sequence or says why it is not

**Proposal.** Add to `doc/todo/02` §2's sequence and to `tools/state.sh`:

```sh
cargo test --profile gates -p pdf-archive   --test corpus          -- --ignored --nocapture   # the validator over the veraPDF corpus, clause by clause
cargo test --profile gates -p pdf-transform --test archive_corpus  -- --ignored --nocapture   # the converter over the same corpus: conforms in, conforms out, no glyph moves
cargo test --profile gates -p pdf-model     --test save_round_trip -- --ignored --nocapture   # the viewer's save, read back by three readers
```

with an `archive` section in `tools/state.sh` printing the validator's and the converter's own
summary lines. Then make the population a checked claim: `tools/conformance/tests/sandbox_gates.rs`
already reads §2's command block; extend it to assert that every test file in the tree carrying
`#[ignore]` is either named by a line of that block or carries a `// not a gate:` line stating why
(`on_disk`, `actions`, `signature.rs`'s priced measurement, and the two `viewer-confined` files
each have a reason today; it is not written where a check can read it).

**Why.** Seven of the twenty-seven `#[ignore]`d test files are in no gate line; the converter's
walk is the one that found ADR 1006's lying signature, on a merge, because nothing ran it per
round; the validator has no line in `tools/state.sh` while `doc/state-of-play.md:333` says it has.

**Cost.** Three corpus walks per whole-sequence round; `doc/veraPDF-corpus` is 239 MB and a
submodule, so each line skips loudly without it as `archive_corpus` already does. The
`sandbox_gates.rs` extension is an afternoon and will fail the build the day a walk is added
without a line — which is the point.

## 4. A golden of our own output, held by name

**Proposal.** A per-page digest of the CPU raster over the corpus — `examples/raster_digest.rs`
already computes it — held in a file by *name* in both directions, the way `ambiguous_undiagnosed.txt`
and ADR 0970's `NO_PARENT_KEY_SILENT` are: a page whose digest moves fails the round that moved it
and names the page; a round that changes pixels on purpose updates the entry in the same commit,
which is what trap 1 already asks it to do by eye. Population: every corpus page the oracle judges,
including the 835 `ambiguous` ones, which today nothing holds. A name absent from the file proves
nothing and fails nothing, so a machine without the submodule passes.

**Why.** `doc/PLAN.md:276-279` promised it in the first month; it was never built as a gate. On
42.7% of the oracle's population (835 of 1,956 pages) the only hold on this program's output is an
ink sweep with an alarm at one level of 255, and ADR 0945 records a page that was wrong in *where*
at the right *weight* for hundreds of sessions. The last four defects the apparatus found were
each an instrument that measured the wrong population; this is the instrument that was planned
and does not exist.

**Cost.** One gate line, one file of about two thousand names, and a discipline: every pixel round
edits the file. That discipline is the cost to weigh, because trap 39's shape is near — a signal
that fires on every pixel change is only a signal if a round reads which pages moved and says why.
The mitigation is the file's own diff: a round that moves a hundred pages has to name them in its
history file, which is what a pixel round owes today under trap 1 and does by hand.

## 5. `Examination` learns what reaches an object; the interpreter's observer is a question

**Proposal, part one (a task).** `Examination::reaches()` — one walk from `/Root` and `/Info`,
computed once like the survey, recording per object the entries it was reached through. It is what
`crates/pdf-archive/examples/unreferenced.rs` computes by hand and what `doc/todo/62` says the
crate lacks. It unblocks that item and every future requirement about a relationship between
objects.

**Proposal, part two (a question for the owner).** Whether `pdf-model`'s interpreter grows an
*observer* — the resolved resource dictionary and graphics state at each operator, without
drawing — so that `crates/pdf-archive/src/survey.rs` (3,715 lines, its own `State`, its own
`q`/`Q`, its own form/pattern/Type 3 traversal) reads the interpreter's state instead of
re-deriving it. Today the validator carries the second content-stream state machine in the tree,
against `crates/pdf-render/src/lib.rs:12-16`'s own argument that such a machine exists once or the
two disagree.

**Why.** The validator's contract is per-object, and the converter's identity case (ADR 1006) is
the same seam seen from the other side: a converter whose unit of change is the file cannot
express "nothing changed" except as a bypass in front of its pipeline. Neither is a hack; both are
a contract one notion short.

**Cost.** Part one: a walk of every object once per report — `Examination` already walks them
(`objects()`, 346 ms on ISO 32000-2's 110,000 objects) and the reachability walk is the same
order. Part two is the expensive one and the reason it is a question: an observer is a public
surface on the interpreter that every future operator has to feed, and the survey's own
under-report-rather-than-mis-report rule (`survey.rs:23-27`) would have to be re-derived over what
the interpreter does at a soft mask, an `SMask` image and a shading function — the three things the
survey deliberately does not walk. The honest alternative is to leave two machines and hold them to
each other with a test over the corpus: every resource the survey reports selected is one the
interpreter selected, in both directions.

## 6. RFC 0007: three answers before a line is written

**Proposal.** The owner answers three of the eight open questions in the direction that keeps the
seam, and the other five wait on those:

- **Q54 — `apply` returns a request; the binary runs the tool.** RFC 0002 §5's second rule (no
  filesystem, no process) is what made the seam confinable and its determinism claim checkable;
  a library that spawns `/usr/bin/soffice` gives both up. The cost is a round trip per invocation
  and a harder streaming story, which the RFC already prices.
- **Q55 — `derive` is not this verb's.** A movie's screenshots or an office file's PDF are
  produced by a separate program and handed to the converter as an input document; the converter
  attaches or appends what it is given and reports it. The archival verb keeps `A48`'s line —
  state an interpretation the standard defines; never fill in an absence — whole.
- **Q58 — appending pages is its own amendment**, because `CLAUDE.md`'s exclusion text says so in
  as many words, and the amendment record has been kept twice by argument.

**Why.** RFC 0007 is a configuration format, an external-tool API and two content-creating remedy
kinds proposed for a converter whose middle stage is a 2,301-line table; its own §5b lists five
things the format already lacks and says "or a configuration becomes a program". It is correctly
parked. The risk is that it is built as drafted, and a configuration format is an interface the day
one file is written against it.

**Cost.** Nothing today. Answering Q54 the recommended way costs the RFC's streaming design a
revision; answering Q55 that way costs the RFC its `derive` kind and twelve of its catalogue
entries move to a separate program's remit.

## 7. The small list

Each is a line or a file, and each is the kind of thing that accumulates into a direction if left:

- `viewer_core::Secret` moves to `pdf-syntax` beside decryption; `pdf-transform` drops its
  dependency on the viewer's application crate.
- `clippy.toml`'s `cognitive-complexity-threshold` is either enabled (`clippy::cognitive_complexity`
  named in `[workspace.lints.clippy]`, with the exceptions it then demands) or deleted; its comment
  promises `#[allow]` against trap 7 either way.
- `crates/viewer-ui/src/bin/quorra.rs:205` and `quorra/arguments.rs:152`: `#[allow]` to `#[expect]`.
- `crates/pdf-syntax/src/lexer.rs:896-897` carries the ledger's reason for the salvage ("toward
  reading files that exist") instead of Acrobat's and pdf.js's agreement; `:923-927` loses the
  sentence it contradicts.
- `crates/pdf-transform/src/archive/mod.rs:206` and `optimize.rs:341` say that refusing a
  document only Annex C.4's recovery reads is this project's choice — the annex is informative.
- `doc/crate-map.md` gains a `pdf-archive` row and names `render-raster` by its name.
- `raster/doc/adr/` takes a prefix (`R-0053`) so that a pointer to an ADR names one series;
  `tools/conformance/src/pointers.rs` learns the prefix.
- `crates/viewer-ui/src/lib.rs:4-5` stops naming `ashpd` and `AccessKit`.
- `CONSISTENT = 0.001` becomes one constant in `pdf-font` that the validator and the converter
  both read, with the sentence at `pdf-transform/src/archive/fonts.rs:84-88` retired: a converter
  that reads the validator's constant *should* follow it, because they are the same clause.

## What this ADR does not propose

- Moving the display list's two deferred producers (`ImageSource::AtDeviceScale`,
  `DeferredColours`) out of `pdf-render`. They are the argued exception to "a display list is data"
  and their cost is paid where it falls (`viewer-confined`'s raster arm). The review names them so a
  reader knows; it does not ask for a change.
- Splitting the interpreter's `run_reader` (908 lines, 75 arms). It is the operator table, it is
  essential, and a mechanical split into per-operator functions is the kind of change a pixel round
  makes with the oracle beside it, not a decision for the owner.
- Any change to the exclusion list. The fence is where `CLAUDE.md` says it is; the five sanctioned
  constructions on it are each argued and reported. What the review asks is only that the
  principle name all five where it names two.
