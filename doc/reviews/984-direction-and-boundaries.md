# 984 — Direction and boundaries: an architectural review of the viewer, the validator and the converter

Status: **review** — read-only over `crates/` and `tools/` at commit `0dde6224`, commissioned by the
project owner in these words: *"analyze the pdf-viewer, the validator and the converter for their
overall direction. Is the code going into the right direction, does the structure look good? Are the
boundaries at the right positions … I want to avoid that we are fixing problems with 'hacks' and
overlook a big picture problem."*
Date: 2026-09-12. Session 984, with five sibling rounds live in the same tree (986–990).
Proposals: ADR 1005. Record: `doc/history/984-direction-and-boundaries.md`.

Every claim below carries a file and line, a command with its output, or a count. Where a number is
quoted it was produced in this session by the command beside it; none is copied from a document.
`§N` is ISO 32000-2 and nothing else; quotation marks mean verbatim.

---

## Verdict

**The code is going in the right direction and its structure is sound; the big-picture problems are
shapes rather than hacks, and there are five of them.** The crate graph is acyclic and layered
(`cargo metadata`: the three rasterisers depend on `pdf-render` alone, `pdf-render` and
`pdf-syntax` on nothing internal, `viewer-core` on no external package at all); the display list
boundary holds; the viewer, the validator and the converter genuinely read one document model and
one reading of ISO 19005; and the hunt for hacks came back nearly empty — no producer name in any
condition, no constant tuned to a corpus (the two suspicious ones, `90.51` and `0.525`, are derived
and their history says so), eight "workaround" markers in half a million lines and all of them
describing somebody else's, and exactly one recovery justified in code by what other readers do.
What the evidence does show is this. **First**, `pdf-model` is five crates wearing one name —
191,001 lines *including its tests and examples* — 93,413 of source, 67,772 of tests; the merge that read this review re-counted, and the source alone is 18% of the tree's Rust, not 37% — 76 modules, with a 9,906-line signature-verification
stack of ASN.1/X.509/PKCS#1/PSS/DSA/ECDSA/EdDSA that the crate map itself says holds "no PDF at all",
a 13,098-line colour engine and a 16,982-line interpreter beside the page tree — so the crate whose
header states one responsibility carries six, and every consumer links twelve cryptographic packages
to read a page. **Second**, the tree's knowledge is indexed to a chronology: 1,353 session ordinals
and 3,744 ADR references inside Rust comments, 855 ADRs of which 132 amend another, 241,995 lines
of Markdown against roughly 520,000 of Rust, comment blocks of up to 768 lines, corrections appended
to sentences rather than replacing them — which is a codebase that *needs* the thousand ADRs to be
read, the opposite of principle 4's aim. **Third**, the instruments that catch the gravest defects
are not in the gate sequence: the converter's corpus walk that found ADR 1006's lying signature, the
validator's veraPDF comparison, the viewer's own save round-trip — seven of the twenty-seven
`#[ignore]`d test files are named by no line of `doc/todo/02` §2 or `tools/state.sh` — and there is
no golden of this program's own output at all, so on the 835 oracle pages that are `ambiguous` (43%
of 1,956) a regression is visible only if it moves ink by a level. **Fourth**, the validator's
`Examination` is a per-object contract and cannot answer a whole-file question, which is why
`doc/todo/62` is blocked and why ADR 1006's identity conversion had to be bolted in front of the
pipeline as a special case rather than falling out of the rule the verb rests on; and the validator
carries a second content-stream state machine (`survey.rs`, 3,715 lines) beside the interpreter's.
**Fifth**, RFC 0007 — a configuration format, an external-tool API and content-deriving remedies for
the converter — is the first real direction risk in the tree, and it is correctly parked on eight
unanswered owner questions; the risk is that it is built as drafted. None of these is a hack, and
none is fixed by a hundred small commits; each needs one decision, and ADR 1005 puts the five to
the owner with their costs.

---

## The ranked findings

### 1. `pdf-model` is five crates wearing one name

**Evidence.**

| measure | value | command |
|---|---|---|
| lines / files in `crates/pdf-model` | 191,001 / 273 (**93,413 in `src/`**, 67,772 in `tests/`; the `find` counts both) | `find crates/pdf-model -name '*.rs' \| xargs cat \| wc -l` |
| share of all Rust under `crates/` + `tools/` | 37% with tests (191,001 of ~520,000); **18% for source alone** (93,413 of 528,584) | same, summed; corrected at the merge |
| `src/` modules | 76 | `find crates/pdf-model/src -name '*.rs' \| wc -l` |
| stated responsibility | "Document and page-tree model built over parsed PDF objects." | `crates/pdf-model/src/lib.rs:1` |
| signature stack (`signature cms x509 der pkcs1 pss dsa ecdsa eddsa bigint`) | 9,906 lines | `cat … \| wc -l` |
| colour stack (`colour icc function shading mesh`) | 13,098 lines | same |
| content interpreter (`content.rs` + `content/*`) | 16,982 lines | same |
| XML parser (`xmp.rs`) | 2,795 lines | `wc -l` |
| external dependencies | 17, of which **12 cryptographic**: `p256 p384 p521 ecdsa ed25519-dalek crypto-bigint const-oid sha1 sha2 sha3 shake md-5 ripemd` | `cargo metadata` |
| crates that depend on it | 17 (`corpus-classes hayro-compare pdf-archive pdf-retrieve pdf-transform pdf-vfs pdf-vfs-ffi safedocs spec-errata viewer-accessibility viewer-confined viewer-core viewer-ffi viewer-gtk viewer-host viewer-qt viewer-ui`) | `cargo metadata` |

`doc/crate-map.md:168` describes the crate's own contents in these words: "`der.rs` and `cms.rs` hold
no PDF either and are the tree's only ASN.1", "the five modules that answer §12.8.1's *second*
question and hold no PDF either". The lesson is already written down in `doc/PLAN.md:329-337` —
"Their crate boundaries are worth copying where ours are missing … a `CMap` parser is
self-contained, independently testable and independently fuzzable, which is the argument that made
`pdf-syntax` separate from `pdf-model` in the first place" — and was applied to fonts and not to
this.

**Cost of leaving it.** Every consumer of the page tree links elliptic-curve arithmetic
(`tools/spec-errata`, which reads errata out of PDFs, compiles `p521`). `doc/todo/02` §2 rule 2
makes any change in `pdf-model` a whole-sequence round, so a change to `cms.rs` costs the oracle,
the text gates and every corpus walk. The gate map's "seven crates are under everything" is true
because one of the seven is a third of the tree. A student asked "where is the document model"
opens a directory of 76 files and finds `pss.rs`.

**First step.** Two mechanical extractions, in this order, each one round: `pdf-signature`
(`signature cms x509 der pkcs1 pss dsa ecdsa eddsa bigint`; consumers outside the crate are exactly
four — `viewer-core/src/notes.rs`, `viewer-core/tests/headless.rs`, `pdf-transform/src/lib.rs`,
`pdf-archive/src/table/interaction.rs` — and `signature.rs` needs `pdf-syntax` for `/ByteRange`,
not `pdf-model`), then `pdf-colour` (`colour icc function shading mesh`, which `pdf-render`'s
`MeshRaster` and `ShadingProgram` already sit beside). The priced cost is the ledger: 72 rows name a
signature-stack path and 110 a colour-stack path, and `tools/conformance` verifies that a row's code
site exists, so each move is one commit that rewrites those rows — a `sed` over `ledger.toml`, held
by the gate that already checks it. ADR 1005 §1.

### 2. The knowledge is indexed to a chronology, and a student cannot read a function without it

**Evidence.**

| measure | value | command |
|---|---|---|
| session ordinals in Rust source ("hundred-and-") | 1,353 | `grep -rE "hundred-and-" --include=*.rs crates tools \| wc -l` |
| ADR references in Rust source ("ADR 0") | 3,744 | same pattern |
| "session" in Rust source | 1,803 | same |
| corrections appended in code comments ("used to say" / "This comment said" / "this sentence") | 29 / 14 / 39 | same |
| comment lines as a share of `src/` lines | `pdf-model` 38%, `pdf-syntax` 38%, `pdf-render` 38%, `viewer-core` 44%, `render-cpu` 37% | `grep -cE "^\s*(//\|///\|//!)"` over each `src/` |
| comment blocks ≥ 50 lines / ≥ 100 lines | 261 / 54 | scratchpad script over `crates` + `tools` |
| the longest comment block | 768 lines, `crates/pdf-model/tests/oracle.rs:2313` | same |
| `oracle.rs` | 16,273 lines, one test file | `wc -l` |
| ADRs / their lines | 855 / 122,770 | `ls doc/adr \| wc -l`; `cat doc/adr/*.md \| wc -l` |
| ADRs that amend or supersede another | 132 | `grep -lE "amends ADR\|supersedes ADR\|Amends\|Supersedes" doc/adr/*.md \| wc -l` |
| ADRs carrying a "used to say / was wrong" correction | 156 | same shape |
| Markdown under `doc/` (excluding `doc/md/`) | 241,995 lines | `cat doc/*.md doc/adr/*.md doc/todo/*.md … \| wc -l` |
| `doc/todo/00-ambiguous-bucket.md` | 1,595 lines, for one todo item | `wc -l` |
| recent churn, last 150 commits | 49% `src`, 15% tests, 3% `tools/`, **26% `doc/`** | `git log -150 --numstat` binned |

Three navigational documents contradict themselves or the tree: `doc/PLAN.md` says Qt was dropped
(`:43-52`) while `crates/viewer-qt` exists; lists a workspace of ten crates (`:75-91`) against
thirty-two; says "Not yet a git repository" (`:687`); says the conformance gate is "Not built"
(`:148`) and "built in the ninth session" (`:539`). `doc/crate-map.md` opens "one row per crate"
and has no row for `pdf-archive` (31,207 lines) while its row for the third rasteriser names a
crate that does not exist (`render-quorra`; the crate is `render-raster`). `crates/viewer-ui/src/lib.rs:4-5`
names `ashpd` and `AccessKit` as what the crate does; neither is a dependency
(`cargo metadata`: `arboard log softbuffer thiserror winit`).

The mechanism is a style, not carelessness: a correction is *appended* — `crates/pdf-model/src/lib.rs:24`:
"**This paragraph opened "[t]ext and images are not yet drawn" until the two-hundred-and-
twenty-first session**" — so the sentence a reader meets first is the retired one, followed by
its retirement. `doc/state-of-play.md` is 554 lines of this, in sentences that run twenty lines.

**Cost of leaving it.** The reading cost of every function is the history of that function. The
project has answered by building instruments that police prose — `tools/conformance` has twenty
sweep binaries (`retired`, `overtaken`, `overstated`, `pointers`, `unread`, `unpriced`, …), 22,042
lines, to find claims that went stale — which is an arms race the prose is winning: the last
150 commits put more lines into `doc/` than into tests and tools together. For principle 4 this is
the decisive question: a student should be able to read `run_reader` and learn how a content
stream is interpreted, not learn that session 535 moved a classification into a table.

**First step.** One rule, stated once: **a comment in code carries the current reason and nothing
about how it got there**; the "how" is the ADR's, which the comment cites by number. A retired
sentence is deleted, not annotated. Then a bounded rewrite of four documents as *what is* texts —
`doc/PLAN.md`, `doc/crate-map.md`, `doc/state-of-play.md`, `doc/HANDOVER.md` — with their
histories left in `doc/history/`. ADR 1005 §2 prices it.

### 3. The instruments that catch the gravest defects are not in the sequence

**Evidence.** Matching `-p <package> --test <name>` against `doc/todo/02-every-round.md` §2 and
`tools/state.sh` for every file carrying `#[ignore]`:

```
UNGATED pdf-archive    --test corpus          (crates/pdf-archive/tests/corpus.rs)
UNGATED pdf-model      --test actions
UNGATED pdf-model      --test save_round_trip (crates/pdf-model/tests/save_round_trip.rs)
UNGATED pdf-syntax     --test on_disk
UNGATED pdf-transform  --test archive_corpus  (crates/pdf-transform/tests/archive_corpus.rs)
UNGATED viewer-confined --test awkward_classes
UNGATED viewer-confined --test confined
GATED   (the other twenty)
```

- `archive_corpus` is the walk ADR 1006 credits with finding "a signature that lied" — found on the
  *merge*, by a sibling, because nothing runs it per round (`doc/adr/1006:10`).
- `pdf-archive --test corpus` is the validator's clause-by-clause comparison against the veraPDF
  corpus. `doc/state-of-play.md:333` says "`tools/state.sh` prints where the comparison stands";
  `tools/state.sh --list` has no such section, and `grep -in "archive\|vera" tools/state.sh` finds
  nothing but a line about `Command::Close`. The validator — 31,207 lines, 191 tests — has **no line
  in the state script at all**.
- `save_round_trip` is the viewer's own save, read back by three readers; the one form of writing
  `CLAUDE.md` permitted for six hundred sessions is in no gate.

**Cost of leaving it.** ADR 1006's shape repeats: the catalogue says one thing, the code another,
and the instrument that could compare them is run when somebody remembers. A round that changes
`pdf-archive` runs "the core" (`doc/todo/02` §2 rule 1) and nothing that opens a veraPDF file.

**First step.** Add the three corpus walks to §2's sequence (they are corpus walks and run under
`tools/bounded.sh` like the rest) and an `archive` section to `tools/state.sh`; then make the
population a checked claim — `tools/conformance/tests/sandbox_gates.rs` already reads §2's command
block, and the same reader can assert that every `#[ignore]`d test file is either named there or
carries a line saying why it is not a gate. ADR 1005 §3.

### 4. There is no golden of our own output, so the oracle sees disagreement and not change

**Evidence.** The last recorded oracle run (`doc/history/985-…md`): 1,956 pages — 990 agree, 62
contradicted, **835 ambiguous**, 47 not comparable, 17 no render. 835 of 1,956 is 42.7%. On an
`ambiguous` page nothing holds our pixels: the `AMBIGUOUS_*` groups hold a *diagnosis* by name,
and the only measurement is step 7's ink sweep (`tools/pdfref/src/bin/undrawn.rs`), whose alarm is
one level of 255 and which ADR 0945 records could not see a page drawn in the wrong *place* at the
right weight for hundreds of sessions. `doc/PLAN.md:276-279` promised "Goldens — Snapshots of *our
own* output, separate from reference comparison, catching commit-to-commit regressions including in
deliberately-divergent areas." Two examples exist — `crates/pdf-model/examples/display_list_digest.rs`
and `raster_digest.rs` — and `grep -n digest tools/state.sh` finds nothing. `render-raster`'s corpus
gate compares the third rasteriser against the CPU one, so an interpreter change moves both sides
together.

**Cost of leaving it.** On 43% of the population an interpreter regression is invisible unless it
moves ink. This is the *next* defect class the apparatus cannot see, in the sense the brief asks
for: the last four it found were a guard over a gitignored population (ADR 0962), a sweep with no
denominator (ADR 0985), an instrument that always fired (ADR 0970 / trap 39), and a signature that
lied (ADR 1006) — every one a case of an instrument that existed and measured the wrong thing. This
one is an instrument that was planned and never became a gate.

**First step.** A per-page digest of the CPU raster over the corpus, held **by name** in a file the
way ADR 0970 holds the accessibility ceiling and the way `ambiguous_undiagnosed.txt` is held in both
directions: a page whose digest moves fails the round that moved it, and the round that changes
pixels on purpose updates the list in the same commit — which is what trap 1 already asks every
pixel round to do by eye. ADR 1005 §4.

### 5. The validator's contract is per-object, and the converter's identity case shows the same seam

**Evidence.**

- `crates/pdf-archive/src/examination.rs:50-67`: `Examination` holds `survey`, `objects`,
  `annotations`, `pages`, `spans` — each a `OnceCell` over a flat enumeration. `objects()` is "every
  object a cross-reference section names" (`:104-113`). Nothing answers *what reaches this object*.
- `doc/todo/62-the-exemption-no-row-states.md:6-8`: "blocked on an infrastructure this crate does
  not have, a whole-file reachability answer available to every row, which is a change to
  `Examination`'s contract rather than to any one predicate."
- `crates/pdf-transform/src/archive/mod.rs:237-249`: `if input.verdict() == Verdict::Conforms { … copy_the_source }`
  in front of the three stages. ADR 0947 §2 had claimed the rule "makes 'a document already
  conforming is not rewritten' true by construction rather than by a special case"; ADR 1006 found
  that "the converter re-serialises every document it converts, including one it changes nothing
  in", for the whole life of the verb. The reason is structural: `Rewrite::WholeFileRewritten`
  (`rewrite.rs:53`) is "the file", so the converter has no representation of *which objects change*
  and cannot check its own rule — it re-validates instead (the net), which is right, and it is
  also why the identity case could only be a special case.
- The validator carries a second content-stream state machine. `crates/pdf-archive/src/lib.rs:5-8`
  says it "adds no reader of its own"; `crates/pdf-archive/src/survey.rs` (3,715 lines) reuses
  `pdf_model::content::reader::ContentReader` for tokens (`:145`) and re-derives everything above
  them — its own `State` (`:926`), its own `q`/`Q` (`:1129-1130`), its own resource-in-force and
  form/pattern/Type 3 traversal — beside the interpreter's `run_reader`
  (`crates/pdf-model/src/content/run.rs:235`). `crates/pdf-render/src/lib.rs:12-16` states the
  principle that this violates one layer up: a state machine reimplemented twice has to be
  reimplemented "*identically*, or the backends would disagree". The consumers of `ContentReader`
  outside the reader itself are exactly three: `pdf-model/src/page.rs:542`, `pdf-model/src/content.rs:1146`,
  `pdf-archive/src/survey.rs:1324`.

**Cost of leaving it.** The `survey` and the interpreter drift in the one place they must agree
— which resource dictionary is in force — and a validator that judges a resource the renderer
never selects is exactly the false failure `survey.rs:27` says the crate is written to avoid.
`doc/todo/62` stays blocked. Every future whole-file requirement (unreferenced resources, orphaned
objects, the signature's byte range against the file's structure) is one more example that
computes reachability by itself.

**First step.** Two things, and the first is small: give `Examination` a `reaches()` — one walk
from `/Root` and `/Info` recording, per object, the entries it was reached through — computed once,
which `examples/unreferenced.rs` already does by hand. The second is a design question for the
owner rather than a task: whether `pdf-model`'s interpreter grows an *observer* — the resolved
state at each operator, without drawing — so that the survey reads the interpreter's resource and
graphics state instead of re-deriving it. ADR 1005 §5 prices both.

### 6. RFC 0007 is the first real direction risk, and it is correctly parked

**Evidence.** `doc/rfc/0007` proposes a TOML configuration keyed by refusal site (`:83-110`), a
declared external program per remedy (`program = "/usr/bin/soffice"`), five remedy kinds including
`derive` (a new representation made from content — a transcript, screenshots) and `supply` (the
operator states a fact the document does not hold). Its own §5b records that the format already
needs a `default` key, a `prefer` ordering, a bounded `on-failure` chain, per-site `keep` lists and
a shape qualifier beside the target qualifier — and says the thing itself: "The remedy vocabulary
has to be closed and small, or a configuration becomes a program" (`:741`). It strains RFC 0002 §5's
second rule ("No filesystem, no clock, no environment", `doc/rfc/0002:225`), which is what made the
seam confinable, and principle 3. `decision.rs` is already 2,301 lines. Eight owner questions are
open and unanswered — `doc/questions/Q53` to `Q60` (`ls doc/questions`: no `A53`–`A60`) — and ADR
0954 says nothing is built until they are.

**Cost of leaving it.** None today. The cost is in building it as drafted: a configuration format is
an interface the day one file is written against it (`doc/rfc/0007:678`).

**First step.** Answer three of the eight in the direction that keeps the seam: Q54 — `apply`
returns a request and the binary runs the tool; Q55 — `derive` is not this verb's, a separate
program produces the derived document and the converter takes it as an input; Q58 — appending
pages is its own amendment. ADR 1005 §6 says why each.

### 7. Small inversions and duplicates, listed because they are the kind that accumulate

Each is minor alone; together they are the direction the small fixes drift in.

| what | where | evidence |
|---|---|---|
| a batch library depends on the viewer's application crate for one type | `crates/pdf-transform/src/lib.rs:74`: `pub use viewer_core::Secret;` | `cargo metadata`: `pdf-transform -> viewer-core`. The type is a password buffer and belongs beside decryption in `pdf-syntax` |
| the font crate depends on the rendering interface for geometry | `crates/pdf-font/src/loading.rs:19`: `use pdf_render::{Path, PathCommand, Point}` | `pdf-font -> pdf-render` |
| the validator's front door is a tool named for another job | `tools/pdf-retrieve/src/main.rs:296-367` (`archive_places`, `archive_row`) | `pdf-retrieve -> pdf-archive`; the converter's CLI is in `pdf-transform` |
| one clause's number, three times | `CONSISTENT = 0.001` at `pdf-archive/src/table/fonts.rs:1292`, `pdf-transform/src/archive/fonts.rs:90`, `pdf-font/src/restate.rs:65` | the second carries an argument for *not* sharing it ("a converter that read the validator's constant would silently follow it") — which inverts the usual reason for a constant |
| one workaround, two backends | `CUTOFF = 0.0005` at `render-cpu/src/shading.rs:28` and `render-gpu/src/shading.rs:23` | documented as kept "in step" (`render-gpu/src/shading.rs:14`) |
| two ADR series with colliding numbers | `raster/doc/adr/0053-a-paint-the-device-evaluates…` vs `doc/adr/0053-the-component-the-ledger-found` | `raster/crates/raster-function-conformance/src/lib.rs:4` cites "`doc/adr/0053`" meaning the first; `tools/conformance/src/pointers.rs` resolves only the second series (`grep raster`: no hit) |
| an instrument that has never fired | `clippy.toml:13`: `cognitive-complexity-threshold = 20` | `clippy::cognitive_complexity` is a nursery lint; `grep -rn "nursery\|cognitive"` over every manifest and `lib.rs` finds only the threshold. Its comment promises exceptions "with an explicit `#[allow]`", against trap 7 |
| trap 7 in the flagship binary | `crates/viewer-ui/src/bin/quorra.rs:205`, `crates/viewer-ui/src/bin/quorra/arguments.rs:152` | two `#[allow(clippy::too_many_lines)]`; the tree's other 53 are `#[expect]` |
| a recovery justified by other readers | `crates/pdf-syntax/src/lexer.rs:896-897`: "both Acrobat and pdf.js read it as -5" | the ledger's §7.3.3 row justifies the same salvage as "toward reading files that exist", which is the honest reason and the one principle 5 admits; the code should carry it. `lexer.rs:923-927` also holds two consecutive sentences that contradict each other ("is not ignored" / "terminates it") |
| a refusal citing an informative annex as a clause | `crates/pdf-transform/src/archive/mod.rs:206`: "§C.4's recovery"; `optimize.rs:341` | `doc/md/…:19038`: "Annex C (informative) Advice on maximising portability". The refusal is a project choice — a sound one — and should say so |
| the crate map | `doc/crate-map.md` | no row for `pdf-archive`; a row for `render-quorra`, which is not a crate |

---

## What is right, to the same standard

- **The layering.** `cargo metadata --no-deps` over 32 workspace packages: no cycle (cargo would
  refuse one); `pdf-syntax` and `pdf-render` depend on no internal crate; `render-cpu`, `render-gpu`
  and `render-raster` depend on `pdf-render` and nothing else internal in normal dependencies, so
  "backends contain no PDF semantics" is a fact of the graph and not a comment; `viewer-core` has
  **zero** external dependencies and its five rules (`crates/viewer-core/src/lib.rs:35-53`) are what
  let it be confined for free; `confined-transport` carries no vocabulary; the two faces
  (`pdf-fuse`, `pdf-vfs-ffi`) hold no layout knowledge. `#![forbid(unsafe_code)]` on 28 of 32
  `lib.rs`, `deny` on the four that need a bridge with a test asserting the token's position.
  No `[features]` table in any manifest — there is no configuration matrix to reason about.
- **No hacks of the kind the owner feared.** Producer names: none in a condition (the only hits are
  metadata field names and corpus paths). Tuned constants: 103 float constants in non-test source
  were listed; the two that looked tuned are derived — `BEVELLED_BY_THE_STROKER = 90.51`
  (`render-cpu/src/lib.rs:2532`) is `1 / sqrt((1/4096) / 2)` from `tiny-skia`'s own angle test, one
  commit in its history; `PARAMETER = 0.525` (`pdf-render/src/shading.rs:1819`) is a barycentric
  coordinate written out. `for now` / `workaround` / `hack` / `TODO` / `FIXME` / `todo!` /
  `unimplemented!` outside tests and examples: 8 lines, every one describing somebody else's
  workaround or a retired sentence. `#[allow(`: 12, of which 3 are the argued `unsafe_code` lifts, 2 are
  Finding 7's, and the rest are a comment and the strings inside the tests that count them.
- **Refusals are typed, named, and their reading direction is right.** `pdf_model::content::report::Unsupported`
  has 21 variants; `pdf_transform::archive::Because` separates a fence, a gap, a target and a
  choice (`decision.rs:187-218`); `pdf_archive::Outcome` separates `Unchecked`, `Processor` and
  `OutsideValidation` from `Met`/`Failed` (`lib.rs:82-97`). A validator that names what it did not
  check is the rare thing.
- **The ledger is complete and honest.** 883 rows: 469 `implemented`, 207 `partial`, 114
  `out-of-scope` (each naming its exclusion), 67 `inapplicable`, 17 `reported`, 9 `writer-side`,
  **0 `unreviewed`, 0 `silent`** (`grep -oE '^status = "[a-z-]+"' doc/conformance/ledger.toml`).
  `pdf-archive/src/coverage.rs` does the same for ISO 19005 at sentence level, with the standard as
  the denominator — `CLAUDE.md`'s two-denominator rule is real machinery, not a slogan.
- **The converter's shape.** Validate → decide → apply → *re-validate* (`archive/mod.rs:224-289`),
  with the decision a `const` table (`decision.rs:547`) a non-programmer can review, and the net
  under it is what caught ADR 1006. `Authorisations` is one field per `Loss` so a new loss is a
  compile error everywhere it must be answered (`decision.rs:130-140`).
- **Tests are behaviour over real documents.** 4,370 workspace tests pass (`doc/history/985`);
  every writer has a corpus walk and a foreign-reader walk (`foreign_corpus`: qpdf, poppler and
  mupdf read the output); the confined workers are probed by the kernel rather than the source;
  fuzzing has its own workspace and a gate that checks it still compiles. The documents themselves
  are held by gates (`sandbox_gates.rs`, `workspaces.rs`, `questions.rs`, `submodules.rs`), which is
  unusual and right.
- **Measurement discipline.** Counts over clocks wherever a count exists (ADR 0894); the launch gate
  judges what a machine cannot move and prints `NOT JUDGED` otherwise (ADR 0884); the oracle's
  independence is in the type (`Reference::independence`).

---

## Per-question evidence

### Q1 — Layer boundaries and the crate graph

`cargo metadata --format-version 1 --no-deps`, internal edges only (normal dependencies):

```
pdf-spec, pdf-syntax, pdf-render, pdf-sandbox, conformance, raster-scene  -> (nothing internal)
pdf-font        -> pdf-render, pdf-syntax
pdf-model       -> pdf-font, pdf-render, pdf-sandbox, pdf-spec, pdf-syntax
render-cpu      -> pdf-render
render-gpu      -> pdf-render
render-raster   -> pdf-render, raster-gpu, raster-scene
pdf-archive     -> pdf-font, pdf-model, pdf-syntax
pdf-transform   -> pdf-archive, pdf-font, pdf-model, pdf-render, pdf-syntax, render-cpu, viewer-core
pdf-vfs         -> confined-transport, pdf-font, pdf-model, pdf-sandbox, pdf-syntax, pdf-transform
viewer-core     -> pdf-model, pdf-render, pdf-syntax
viewer-host     -> pdf-model, pdf-render, pdf-syntax, render-cpu, viewer-core
viewer-ui       -> (13 internal crates)
```

Where `pdf-model` ends and `pdf-render` begins is stated correctly and held: `pdf-render/src/lib.rs:6-20`
("the content-stream interpreter resolves the state machine once … Backends therefore contain no
PDF semantics at all"). Two qualifications the display list carries that a reader should know:

- **It is not plain data.** `ImageSource::AtDeviceScale(DeferredImage)` and `DeferredColours`
  (`paint.rs:963-968`, `shading.rs:466`) carry `Arc<dyn …>` producers back into `pdf-model` code,
  resolved at draw time. `paint.rs:885-902` argues why (a 2×2 image with a 34862×4332 mask), and the
  cost is paid honestly: `viewer-confined/src/protocol/display_list.rs` has to refuse both "by name
  into the raster arm" because a closure cannot cross a pipe. The boundary is right; the
  exception is documented; a reader should know the list has two variants that are behaviour.
- **It carries a small interpreter.** `ShadingProgram`/`ProgramStep`/`ProgramOperator`
  (`program.rs:68-202`) put §7.10.5's calculator on the device by ADR 0053's decision. That is a
  chosen leak of one clause into the backend contract, argued, and `raster-function-conformance`
  exists to hold it; it is the one place a backend evaluates something PDF-shaped.

`pdf-model -> pdf-sandbox` means the document model *spawns a confined process* when it meets a
JBIG2, JPX or CCITT stream (`image.rs:430`: `pdf_sandbox::Sandbox::shared()`). Documented, and
right for the viewer; a library consumer (`pdf-vfs`, `spec-errata`) inherits a process arrangement
by linking a model.

### Q2 — The shared spine

The three programs share `pdf-syntax::Document` (immutable), `pdf-model`'s readers, and — for the
validator and converter — one requirement table: `pdf_archive::check` (`lib.rs:69-77`) is the only
reading of ISO 19005, and `pdf-transform/src/archive/decision.rs:9-11` states "Nothing here reads
the standard". That is the right thing to share, and it is shared correctly.

What the validator's `Examination` gives the converter is a `Report` of judgements with `places`
(`Findings::kept()`), and the converter's `sites.rs` reads those places to decide *which objects*
to rewrite (ADR 0957 §"The shape they share"). So the converter can *decide* from the report. What
it cannot do from the report is know what it *changed*: stage 3 is a whole-file walk
(`rewrite.rs:685`, `convert`) with `replace`d objects, and the rule "nothing is changed that no
failed requirement asked for" is enforced only by re-validating the output. ADR 1006's identity
conversion is therefore a shape, not an accident: a converter whose unit of change is the file
cannot express "no change" except as a bypass. Finding 5 above; the fix is a representation of the
change set (which objects, from which decision), not another special case.

`doc/todo/62`'s missing reachability answer is a symptom of the same contract: `Examination` is a
bag of enumerations, and every requirement about *relationships between objects* has to build its
own graph.

### Q3 — Hacks

Searched, with the command and the result:

| search | result |
|---|---|
| producer / reader names in non-comment Rust | 0 in a condition (`grep -rnE "Acrobat\|pdf\.js\|poppler\|mupdf\|Ghostscript\|Producer\|Creator"` excluding comments and tests: metadata keys, corpus paths, census examples only) |
| `for now`, `workaround`, `hack`, `TODO`, `FIXME`, `todo!(`, `unimplemented!(` in non-test source | 8 lines; each names somebody else's workaround (`render-gpu/src/shading.rs:14`, `render-cpu/src/scan.rs:24` on `tiny-skia`'s own comment, `viewer-qt/build.rs:29` on `cxx-qt`'s ordering) or a retired sentence (`pdf-model/src/content.rs:2001`) |
| `#[expect]` reasons that are workarounds | 651 `#[expect(` in total; by lint: `cast_precision_loss` 225, `cast_possible_truncation` 221, `cast_sign_loss` 101, `too_many_lines` 53, `arithmetic_side_effects` 43, `float_cmp` 31, `too_many_arguments` 22 … — the shape of a rasteriser, not of a workaround |
| constants tuned to a corpus | 103 float `const`s in non-test source read; `git log -S` on the two that looked tuned (`90.51`, `0.525`) shows one commit each, both derivations |
| `salvage` / `recover` / `tolerate` / `repair` paths with no clause | `lexer.rs:903 salvage_number` (§7.3.3 row records it as a departure; code justifies it by Acrobat/pdf.js — Finding 7); `filter.rs:736 salvage` (§7.4.4's row, damage vs bound separated, ADR 0306); `xref.rs:205 recovered_by_scan`, `document.rs:1427 recover_compressed_objects` (Annex C.4's informative advice, reported as `was_recovered`); `sfnt.rs:108-443 repaired_loca_*` (a malformed font program repaired from its own lengths, reported as `repair_shortfall`). Every one reports; none is silent |
| a `LimitReached` / `Unsupported` that is really a bug | not found by reading; `Unsupported::Text { operations }` (`report.rs:16-20`, raised at `content.rs:1343`) is the one variant whose doc comment says less than it means — "Text-showing operators were present" is what it said when nothing drew text; today it counts show operations met with no font in the graphics state (`content/text.rs:318-322`) |
| two code paths doing one job | the validator's `survey.rs` beside the interpreter (Finding 5); `CONSISTENT` ×3 and `CUTOFF` ×2 (Finding 7); the archive `rewrite.rs` walk is described as "`crate::optimize`'s closure walk with one difference" (`rewrite.rs:5-7`) and is a second copy of it (`optimize.rs:380 copy_closure`, `rewrite.rs:786 walk`) |

### Q4 — Complexity concentration

Ranked by a brace-matching scanner over every `fn` in `crates/` and `tools/` (13,512 functions; 227
over 100 lines, 22 over 200, 6 over 400):

| function | lines | `=>` arms | depth | essential or accidental |
|---|---|---|---|---|
| `crates/pdf-model/src/content/run.rs:235 run_reader` | 908 | 75 | 9 | the operator dispatch is essential (§8.2 Table 50 is a table of 73 operators); one 908-line function is accidental — `survey.rs:1129` turns the same table into an `Operator` enum and reads it, which is the shape this one could take |
| `tools/conformance/src/citation.rs:363 read_citations` | 874 | 5 | 6 | accidental, and an instrument: one scanner that grew a case per shape of citation |
| `crates/pdf-model/src/function.rs:2300 apply_operator` | 369 | 54 | 5 | essential — §7.10.5's calculator has 42 operators and each is an arm |
| `crates/pdf-model/src/content/text.rs:311 show_text` | 338 | 6 | 8 | mixed — §9.4.4's positioning is essential; the knockout, selection-readback and Type 3 branches inside one function are accidental (depth 8) |
| `crates/pdf-transform/src/range.rs:285 split_items` | 306 | 25 | 7 | accidental — a grammar hand-parsed in one function |
| `crates/viewer-confined/src/protocol.rs` (`encode_answer` 270/36, `decode_command_holding` 186/62, `encode_command` 192/54) | | | | essential by design: exhaustive matches so a new message fails to compile here |

`clippy::too_many_lines` is lifted 53 times, and the threshold the project *meant* to enforce —
`cognitive-complexity-threshold = 20` — is configured for a lint that is never enabled (Finding 7).
The three worst places are one function each in the interpreter, the calculator and the citation
scanner; the first is the one a student meets first.

### Q5 — Direction: instruments versus the program

The instruments are the right investment *for this project's definition of done* — "every PDF that
exists renders as its producer specified" cannot be measured without an oracle, a ledger, a
citation checker and a coverage audit, and each of the last four defects the tree found was found
by one of them. Two observations against that:

- **The instruments are now larger than some of the programs they measure.** `tools/conformance` is
  22,042 lines in 51 files with 20 sweep binaries; `crates/pdf-model/tests/oracle.rs` alone is
  16,273 lines; `pdf-archive/src/coverage.rs` is 5,177. `render-gpu`, a shipped rasteriser, is 4,651.
- **The refusal-as-configuration model (RFC 0007) is a program feature waiting on eight owner
  questions**, and correctly so — Finding 6.

Is the ADR/trap/habit process producing a codebase a student could learn from? The *structure* is —
the crate graph, the message boundary, the display list, the validate-decide-apply shape are each
the kind of thing a course would draw on a board. The *text* is not, for Finding 2's reasons: a
student reading `crates/pdf-model/src/lib.rs` is told in its second paragraph what the file said
in session 221. The process is producing the right decisions and recording them in a form that
requires the record to be read.

### Q6 — The exclusion list and the fence

Where the line actually runs, against where `CLAUDE.md` says it runs:

| code | side of the line | argued by |
|---|---|---|
| `pdf-font/src/restate.rs` — rewrites `hmtx`/`hhea` and CFF charstring widths of an embedded font program | *on* the line: touches a producer's font bytes, invents no mark; §9.2.4 makes the dictionary the source of positioning and §9.9.1 forbids a processor to read `vhea`/`vmtx` at all | `restate.rs:1-40`; `doc/pdf-a-conversion-limits.md` §4.9; the test `an_outline_survives_its_advance_being_restated` |
| `pdf-transform/src/archive/fonts.rs` — embeds a face the file never carried | past the old line, sanctioned by `doc/questions/A47` and reported per font | `archive/mod.rs:53-60` |
| `pdf-model/src/variable_text.rs` — writes a content stream for a field appearance | on the line, sanctioned by §12.7.4.3 and named in `CLAUDE.md` | `CLAUDE.md` exclusion text |
| `pdf-transform/src/archive/prepare.rs` — constructs an `/AP` from a subtype's entries | on the line, sanctioned by `A21` on condition every appearance is reported | `archive/mod.rs:43-45` |
| the `F` → `f` respelling | one byte inside a content stream, sanctioned by `A50` on a closed list | `doc/pdf-a-conversion-limits.md:956-971` |
| appending pages as a `preserve` remedy (RFC 0007 §4.6.1) | **the far side**, not built, Q58 open | `doc/rfc/0007:239-253` |

So the fence is where the document says it is, and every step onto it is argued and reported. The
one thing to say is that the line is now crossed by *five* sanctioned constructions, each with its
own answer file, and `CLAUDE.md` names two of them. A reader of the principle should be able to
find all five from it.

### Q7 — Tests and gates

Behaviour or implementation? Overwhelmingly behaviour: the corpus gates ask what a document draws,
reads, writes and re-reads; `foreign_corpus` asks three other readers; the confined tests ask the
kernel. The implementation-shaped tests are deliberate and few — the `unsafe_position` tests
(`viewer-ffi`, `viewer-qt`, `pdf-vfs-ffi`) read their own crate's source to count tokens;
`every_group_of_pages_carries_a_diagnosis_naming_one_of_them` reads `oracle.rs` to check its own
comments; `sandbox_gates.rs` and `workspaces.rs` parse `doc/todo/02`'s command block. These are
tests over the *repository*, and they are the right shape for what they check.

What the whole apparatus cannot see, and the next one: Findings 3 and 4. An instrument outside the
sequence (seven walks) and a self-golden that was planned and never built. Both are the same
lesson the last four defects taught — the population, not the predicate — arriving at the sequence
itself rather than at one gate.

---

## Next steps, in priority order

1. **Put the seven walks in the sequence and make the population checked** (Finding 3). One round;
   no design.
2. **A self-golden over the CPU raster, held by name** (Finding 4). One round; the construction is
   ADR 0970's.
3. **Extract `pdf-signature`, then `pdf-colour`** (Finding 1). Two rounds; the ledger rewrite is
   priced at 72 + 110 rows.
4. **The comment rule, and four documents rewritten as *what is*** (Finding 2). One rule now; the
   rewrite is a round per document and can be spread.
5. **`Examination::reaches()`**, and the observer question put to the owner (Finding 5).
6. **Answer Q54, Q55 and Q58** before any line of RFC 0007 is written (Finding 6).
7. The small list in Finding 7, each a line: `Secret` to `pdf-syntax`; enable or delete the
   cognitive-complexity threshold; two `#[allow]` to `#[expect]`; the lexer's reason; the Annex C
   wording; a `pdf-archive` row and the right name in the crate map; a prefix for `raster/`'s ADR
   series.
