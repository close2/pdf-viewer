# ADR 0557 — The gate that measured a program without its decoder

Status: accepted, 2026-08-23. Session the seven-hundredth, an instruments round taking
`doc/traps/instruments-and-reports.md` trap 16 — the defect four rounds looked at and three named
wrongly. Adds `pdf_model::image::sandboxed_decoder`; puts a requirement in the six gates of
`doc/todo/02` §2 that lacked one and a reasoned exemption in the two that need none; adds
`tools/conformance/tests/sandbox_gates.rs`. Corrects trap 16, extends trap 10, and corrects this
project's record of sessions 660, 664, 695 and 698. Moves no pixel and changes no floor.

## 1. What was believed, and what is true

Trap 16 said a gate's verdict depends on **how much of the workspace was built**, on four readings
of one commit taken by the six-hundred-and-ninety-eighth session, and attributed it to **Cargo
unifying features across whatever is in the build**. Two of those four readings pass the
accessibility census's ratchet and two fail it, and the difference is nine structure elements.

The attribution is wrong. The variable is neither the directory, nor staleness, nor feature
unification. It is this:

> **`pdf-sandbox-worker` is a separate program, Cargo does not build another package's binaries
> when it tests this one, and a build without it decodes no `CCITTFaxDecode`, `JBIG2Decode` or
> `JPXDecode` image at all.**

Which is **trap 10**, unchanged since the seventh session, wearing trap 16's clothes.

### The measurement, with its conditions named

Trap 16's own rule — *a claim about a defect is a claim about the conditions you reproduced it
under* — so here are the conditions in full. Worktree `r700` at `2ac19e0f`; a `target-dir` of its
own (`/home/AI/cargo-target/pdfv-r700`) that did not exist before this round created it; built by
`cargo test --profile gates -p viewer-core --test accessibility_census --no-run` and nothing else,
so no worker existed anywhere in it; the test binary run directly, twice; then
`cargo build --profile gates -p pdf-sandbox --bins`, and **the same binary — same digest, not
recompiled — run twice again**.

| the one binary, in the one directory | `placed by their own marks` | `with no place` | ratchet |
|---|---|---|---|
| no `pdf-sandbox-worker` beside it | 93 258 | 1345 | **fails** |
| the worker beside it | 93 267 | 1336 | passes |

Deterministic twice each. Nothing about the build differs between the two rows, which is what makes
this decisive where session 698's four rows could not be: 698 varied the *build*, and every one of
its four scopes also varied whether a worker had been produced as a side effect.

**And the nine elements have a name.** Instrumented to print per document, the two runs differ on
exactly one file: `issue5481.pdf`, `derived=9 placeless=0` with the worker and `derived=0
placeless=9` without. It carries a `JPXDecode` image. §14.8.3.3 derives an element's rectangle from
what its marked content *drew*; an image that was refused drew nothing; so nine elements lose the
only place they had. Nothing else in the corpus moves.

## 2. Which number is right — and why it was never a question for the standard

`doc/HANDOVER.md` asked which of 1336 and 1345 the specification says is correct, and warned against
moving a floor before answering. The honest answer is that **they are not two readings of the
standard.** They are one reading, by two programs: one that can decode ISO/IEC 15444-1 and one that
cannot. §14.8.3.3 gives the same answer to both, and both obey it.

So **no floor moves**, and 1336 — the reading of the program a user is given — is the one the
ratchet was set under and stays set under. The other number is not a rival interpretation to be
adjudicated; it is a deployment fault that had learned to look like one.

That is worth keeping as a general shape, because it is what cost four rounds: **two numbers from
one tree are not automatically two readings of the specification.** Ask what the two *programs*
were before asking what the clause says.

## 3. What the shipped binary carries — the third scope, answered

`cargo build --release --bin pdf-viewer` was named as an unexamined third scope. It was examined, by
diffing the resolved unit graphs (`cargo +nightly … --unit-graph -Z unstable-options`, closure of
the named root, features per unit):

- **against the whole-workspace gate build**, the shipped `pdf-viewer` closure differs in exactly
  two crates — `either` gains `std`/`use_std`, `serde` gains `alloc`. Both add trait implementations
  and neither changes a value any code computes.
- **against the census's subset build** the differences are larger and are the ones 698 saw:
  `num-traits` (`{}` against `default, libm, std`), `once_cell` (`race, alloc` against those plus
  `std`), `rustix` and `linux-raw-sys` (extra syscall families), `bytemuck` (`extern_crate_alloc`),
  `log` (`std`), `either`, `enumflags2` (`serde`), and the proc-macro crates `syn` and `proc-macro2`.

**Every one of those was traced to its consumer and none changes what the program computes.**
`num-traits` is reached only by `crypto-bigint`, an integer library that uses none of the `Float`
implementations those two features gate; `once_cell`'s consumer is `read-fonts`, which uses
`once_cell::race`, present either way; `bytemuck`'s `extern_crate_alloc` adds an `alloc` module
nothing in this graph is conditional on; `either`, `log`, `enumflags2`, `rustix` and
`linux-raw-sys` add APIs rather than change behaviour; `syn` and `proc-macro2` are compile-time
only. So the feature-unification hypothesis is not merely unproven here — **the differing features
are enumerated and each is accounted for**, which is a stronger statement than "we did not find
one".

The conclusion that matters for the instrument: **the scope that is odd is the gate's, and the
shipped binary agrees with the whole-workspace build on everything behavioural.** The gate was not
measuring a program the user does not receive *because of features*. It was measuring one the user
does not receive because a component of it had not been built.

**The method is kept rather than the answer**, because the answer decays the moment a dependency
gains a behaviour-changing feature. `doc/verify.md` now carries the two commands and the closure
diff as a thing a round can run in a minute.

## 4. What this makes of sessions 660, 664, 695 and 698

- **660** reported six CCITT tests failing under `cargo nextest run -p pdf-model` alone, black and
  white exchanged, and blamed `hayro-ccitt` feature resolution. The observation was real and the
  attribution wrong twice over: `hayro-ccitt` at the pinned revision **has no `[features]` section
  at all**, so no scope can resolve it differently — and `CCITTFaxDecode` goes through the worker,
  which `-p pdf-model` does not build. It is trap 10, and "black and white exchanged" is trap 10's
  own original sentence from the seventh session.
- **664** recorded it as not reproducing, in a fully-built shared directory. That directory had a
  worker in it. The rule 698 drew from this stands and is the one thing in the chain that was right.
- **695** found the census failing, and did not lower the floor. Correct, and now for a stated
  reason.
- **698** established that the *directory* was a symptom and that build **scope** was the variable.
  Also correct — and one step short: what varies with scope is not only which features Cargo
  unifies but **which binaries Cargo produces**, and only the second of those two was doing
  anything.

## 5. What was changed

**`pdf_model::image::sandboxed_decoder()`** — a probe that starts the worker and returns
`ImageError::Sandboxed` carrying the sandbox's own sentence. It reuses the error variant a refused
image already carries rather than leaking `pdf-sandbox`'s types through `pdf-model`'s surface, so a
caller two crates up (`viewer-core`'s censuses, `render-quorra`'s gate) needs no new dependency to
ask the question. Its non-test use is real: a host can say at startup what it would otherwise say
one image at a time.

**Six gates gained `require_the_sandbox()`** — `accessibility_census`, `selection_census`,
`text_extraction`, `fixed_documents`, `jpeg2000`, and `render-quorra`'s `corpus`. Two had it
already (`pdf-model`'s `corpus` and `oracle`), and their private copies now call the probe.

**What each of the six does when the worker is taken away was measured rather than assumed**, with
the same binary run either side of one `mv`:

| gate | without the worker |
|---|---|
| `accessibility_census` | **passes, with nine elements in the wrong column** — the defect |
| `jpeg2000` | fails, listing the documents that *stopped* differing from `OpenJPEG` |
| `fixed_documents` | fails, naming no cause |
| `selection_census` | every count identical |
| `text_extraction` | every count identical |
| `render-quorra`'s `corpus` | not measured; it is the one that costs a device |

So two of the six were already failing **in the wrong words**, which is an argument for the change
rather than against it: a gate that fails for the right reason in a sentence that reads as *the
decoder improved* costs a round the same hour a silent one does. And two move nothing today — kept
all the same, and the reason is stated rather than assumed: **their numbers are read out of an
`Interpretation`, and what an `Interpretation` contains is exactly what the worker decides.** The
cost of a precondition that never binds is one process spawn; the cost of the one that was missing
was a dozen rounds. Trap 11's rule is about a *report* firing on a condition the clause does not
state, and this is a precondition on an instrument — the asymmetry runs the other way.

**`tools/conformance/tests/sandbox_gates.rs`** — the recurrence gate. It reads `doc/todo/02` §2's
command block, which is the one place the sequence is stated, and requires every
`-p <pkg> --test <target>` line's file either to call `require_the_sandbox` or to carry a line
beginning `// no sandbox worker:` with a reason. `dates` and `xmp` carry the second, because they
read the object graph and interpret no content stream.

Tying the population to the *document that owns the sequence* is deliberate: a list inside the
checker would be a second copy of the sequence, which is the drift ADR 0232 §4 was written about.
A gate line added there is a gate this check immediately demands an answer from.

**Calibrated before it was believed**, which is trap 13. Run against `HEAD`'s versions of
`accessibility_census.rs`, `dates.rs` and `render-quorra/tests/corpus.rs` — the tree as it was
before this round — it names all three, and passes once they are restored. Its two self-checks are
part of that: it fails if the parse yields too few gate lines, and it fails if *no* gate carries the
marker, so a renamed marker cannot pass everything by silence.

## 6. What this does not fix

The requirement is a call each gate makes, and a call can be forgotten — which is why the check
above exists. What the check cannot see is a gate that is **not** in `doc/todo/02` §2's sequence: an
example, a benchmark, or a measurement a round takes by hand. Those still owe the same care, and
`doc/environment.md`'s rule about stale binaries is the closest thing to a guard.

Nor does it make a *missing* image loud in the census's own output. The interpreter reports it —
`Unsupported::Image` carries the sandbox's sentence — and the census does not count reports. That is
a real second instrument and it is not this round's: the requirement makes the run stop, which is
enough to keep a number honest, and counting reports would be a change to what the census measures.
