# 0874 — The *ask* level crosses the confinement as a question and an answer

Session 916. Status: **accepted**. The first of this round's two records: the construction that makes
`CLAUDE.md` principle 3's *ask* level askable at all, why it is two round trips rather than a
dialogue on the wire, and what actually crosses. ADR 0875 is the faces on top of it.

## Context

`CLAUDE.md` principle 3 states four levels — `off`, `on`, *ask before the operation*, *warn before
the operation* — and one sentence about how they must be built:

> **A refusal that cannot become an "ask" is the thing to avoid.**

Sessions 373, 872 and 885 built the reading, the four levels, the verdict, the event and the command.
`doc/todo/38` then said that nothing in the core had to change for *ask*, and **round 913 found that
false by building a face**. RFC 0003 §6 puts every byte of parsing in a confined process, and the
restriction decision is taken inside it: `pdf_transform::apply` asks `pdf_model::restriction::decide`
once, at the seam, and under `Level::Ask` answers `Refusal::Unanswered` because — in that function's
own words — a pipe has nobody to put the question to. The confined generator has no channel to a
person **by construction**; that is what confinement is.

So *ask* degraded to a refusal in every face: the viewer answered it through
`viewer_host::unanswerable`, `pdf-fuse` refused because a mount has no dialogue, and the KIO worker
refused *although it has `KIO::WorkerBase::messageBox`*, a real question channel sitting unused. That
is exactly the shape the principle says to avoid.

ADR 0869 §3 priced two ways out and recommended the second:

- **make the wire a dialogue** — a worker that can interrupt an answer with a question, changing the
  protocol both confined workers in this tree speak, `pdf-view-worker`'s included; or
- **ask first, in two round trips** — a query that answers *would this operation be restricted, and
  with what reasons*, put before the operation; the host asks the person; the operation is then
  issued with the answer in hand.

## Decision

**The recommendation stands, and this round implemented it.** Two round trips, no protocol
inversion. The argument for keeping it, having now read the code rather than costed it from outside:

- **A dialogue on the wire makes every worker a state machine.** `confined-transport` is one frame
  each way, and both `Confined` implementations lean on that: a question arriving *instead of* an
  answer means every `ask` call site in both brokers has to be able to be re-entered, and the
  seccomp-confined side has to be able to block on a reply while holding a half-computed render.
  The cost lands on `pdf-view-worker`, which has no restriction question at all.
- **The thing that must not be duplicated is the policy, and asking first does not duplicate it.**
  What crosses is a *question* and an *answer*. `pdf_model::restriction::decide` is still called in
  exactly one place per crate, and in `pdf-transform` it is now called from exactly one function —
  `consult` — which `apply` itself uses. A host that asks and then acts is answered by one reading
  rather than by two that could disagree.
- **The one thing that can be true between the two calls is named and handled.** A consent belongs
  to the generation it was given about; `pdf-vfs` holds it beside the worker for that generation, so
  a document somebody else edited underneath the mount takes the answer with it.

### 1. `pdf_transform::consult` is the question, and `apply` asks it too

```rust
pub fn consult(level: Level, document: &Document, operation: Operation) -> Consulted
```

`Consulted` mirrors `restriction::Verdict` one for one — `Proceed`, `Warn`, `Ask`, `Refuse` — and
carries the operation's word and the document's reasons already worded by `describe_restriction`.
`Consulted::question` is the sentence to put in front of a person, worded **once**, here, so that
four hosts cannot word it four ways; it is `None` for the other three verdicts, because those are
statements rather than questions and showing one as a dialogue would be asking somebody to decide
something already decided.

`apply_borrowed`'s restriction loop is now a `match` on `consult`'s answer. That is what makes the
two calls one reading rather than two.

### 2. What crosses `pdf-vfs`'s wire: one query out, one answer back, one wrapper

- **`Query::Consult { operation }`** → **`Answer::Consulted(Consulted)`**. The only query in the
  crate that changes nothing and reads no page.
- **`Query::Consented(Box<Query>)`** — the operation, with a person's yes behind it. The inner query
  runs at `Level::Off`, which is the level `CLAUDE.md` says "shall always be possible" and is what a
  person consenting to one operation has chosen for it. **The wrapper carries the answer, never a
  second copy of the policy.**

Two wire details are decisions rather than plumbing. A `Consented` inside a `Consented` is **refused
by the decoder** rather than recursed into, because a nesting depth a peer chooses is a stack a peer
chooses. And the encode side has `wire::encode_consented(&Query)`, which writes the tag and then
`encode_query`'s own bytes, so consenting to an insertion does not cost a copy of the document being
inserted.

### 3. The consent lives beside the worker, not beside the broker

`Vfs::consult(path, verb)` and `Vfs::answer(proceed)` are the public shape. Underneath, `Current`'s
worker is wrapped in a `Consenting`, which holds the outstanding question and the standing answer and
spends it in `Worker::ask`.

**Here rather than on `Vfs`, and that is the design rather than an economy.** Two things follow from
it that no other placement gives:

- **A consent is scoped to a generation for free.** A `Current` thrown away because the file changed
  underneath the mount takes the answer with it, and nothing has to remember to.
- **A call site cannot forget to spend it.** Every question this crate asks goes through
  `current.worker`; there are fifteen such call sites and no list of which ones matter. The
  alternative — a `consent` parameter threaded to each — was tried first and is exactly the shape
  where the twelfth site is the one nobody updated.

Spent **once**: a yes to deleting one page is not a yes to the next, and not a yes to a different
operation either. `Worker::ask_consented` is the trait method, defaulted to `Worker::ask` — the safe
direction rather than a convenience, because an implementation that does not override it refuses the
operation again instead of performing something nobody consented to.

### 4. Which operation a path's verb performs is the layout table's answer, held to the seam's

The broker has to name an operation from a path and a verb; the seam names one from the plan it is
about to run. Two mappings that must agree and are only *said* to agree is how they stop agreeing, so
the mapping lives where the meaning already lives — `layout::Write::operation` and
`layout::Generator::operation`, beside the table that already states what a write to each row means —
and the **witness is the tree itself**: at `Level::On`, `a_write.rs`'s
`what_the_layout_says_a_path_performs_is_what_the_seam_asks_about` walks four paths and asserts that
the consultation refuses exactly where the operation refuses. A divergence fails as a disagreement
rather than as a mismatch nobody looks at.

`Verb` gained `Read`, and it is not one of RFC 0003 §5.2's write verbs: taking a page out of the
mount is Table 22 bit 11's assembly, a render is bit 3's printing and an image is bit 5's extraction,
so a face is owed the question before it starts the copy as much as before it starts the write.
`refusal_for`, which words what a *change* earns, refuses a `Read` by name rather than answering it
with a sentence about writing.

## The clauses

§7.6.4.2's Table 22 is what a `Consulted::Ask` is about, and §7.6.4.1 is why obeying it is a
requirement at all:

> PDF readers shall respect the intent of the document creator by restricting user access to an
> encrypted PDF file according to the permissions contained in the file.

That is a `shall` on a reader, and `CLAUDE.md` makes obeying it the reader's own decision with
obeying as the default. §12.8.2.2.1's parenthesis is the other source this round's question can be
about:

> (These changes to the document shall also be prevented if the signature dictionary is referred from
> the DocMDP entry in the permissions dictionary.)

Nothing in either clause moved. What moved is that the reader can now be *asked* rather than only
obeyed or overruled.

## Consequences

- **`Level::Ask` is a level rather than a refusal wearing a level's name**, in every host that has a
  channel. ADR 0875 says which those are.
- **`pdf_transform::Refusal::Unanswered`'s sentence stopped naming a command-line flag.** It read
  "--restrictions=ask was given and this program cannot ask"; four hosts show it and only one has a
  command line. It now says the restriction level is ask and there was nobody to ask.
- **A host that never consults is not broken.** It gets the same refusal it always got, which is the
  honest degradation and not a silent proceed.
- **`Answer::Consulted` is the first answer in `pdf-vfs`'s vocabulary that is a *verdict* rather than
  content**, and `Vfs::answer` is the first call in the crate that is neither a read nor a write.
  Both are named that way rather than folded into an existing shape.
