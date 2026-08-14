# ADR 0334 — The save read back by three readers

Status: accepted, 2026-08-14. Session 499. Builds ADR 0323's instrument 2; the numbers of the
first run are in `doc/history/499-the-save-read-back-by-three-readers.md`.

## Context

§7.5.6's incremental update is the one form of writing `CLAUDE.md` permits, and until this
round it had been judged at corpus scale by nobody but its own author: `saving.rs` and the
headless harness assert the construction on a handful of fixtures, and the one precedent for
what an independent reader finds in this program's saves is the password Table 231 bit 14 said
must never be stored and was (ADR 0247). ADR 0323 designed the instrument — one synthetic edit
per document, three **exact** assertions, refusals counted by reason — and priced nothing but
the shape. This round builds it: `crates/pdf-model/tests/save_round_trip.rs`, with the two
witness scripts beside it under `tests/save_round_trip/`.

## Decisions

### Its own test binary, by §2's own mechanism

`doc/todo/02` §2's gate lines hand `-- --ignored` to a **whole binary**, so an ignored
corpus-scale test placed in `corpus.rs`'s or `text_extraction.rs`'s binary joins that gate's
run and output immediately — and ADR 0323's rule is that an instrument's numbers enter §2 only
after they have held across rounds. (Instrument 1 is *designed* to land inside
`text_extraction` for the opposite reason — to ride an existing line.) A binary of its own keeps the
instrument invokable by name today, and keeps the eventual gate line running exactly this:

```sh
cargo test --profile gates -p pdf-model --test save_round_trip -- --ignored --nocapture
```

The three exact assertions bind from the first run — prefix, readback, and every reference
answer agreeing or in a **named, diagnosed** exception list (empty today). The census counts
are printed by reason and deliberately not ratcheted yet.

### What each reference is asked, and through what

*"Open this file and say what is in it"* — never to render. Two routes, chosen so that trap 3
has as little as possible to bite on:

- **mupdf answers from its raw object layer** — `mutool run` over
  `tests/save_round_trip/mupdf_witness.js`, which walks the trailer, page one's `/Annots` and
  §12.7.3's field tree with `/V` inherited per Table 226. Its coordinates are the file's own
  `/Rect` in default user space, so **no page-space convention of mupdf's is in the answer at
  all** — the frame is left out rather than audited. What this exercises is exactly §7.5.6:
  the appended section found, its entries winning over the ones beneath, the new objects
  parsed and (for an encrypted document) decrypted with the document's own key.
- **poppler has no CLI that prints an annotation or a field value** — `pdftotext` reads page
  content streams, `pdftoppm` renders — so the question goes through **poppler-glib**
  (`python3` + `gi`, both already on the oracle machine), which is the same reader underneath.
  Its annotation-mapping area was **measured before it was compared**, against hand-built
  fixtures whose crop box differs from their media box, at every `/Rotate`: the area is the
  `/Rect` translated by the **crop box's origin**, y still upward, with the page's rotation
  then applied. `poppler_area` in the test is that measurement as arithmetic, and the
  comparison happens in poppler's own frame — never poppler's answer re-projected into ours.
  The rectangle tolerances (0.05 pt) are representational, not judgemental: mupdf parses
  reals into 32-bit floats and the script prints four decimals.

### The witness rectangle is fixed in the page's own frame

ADR 0323 said "a fixed rectangle", and fixed *in default user space* would place the
annotation off the visible page wherever the boxes exclude the origin — `160F-2019.pdf`'s
crop box starts at x = −11.96 — leaving "where it was put" unwitnessed. So the rectangle is
fixed relative to the one corner every page has: sixteen points in from the crop box's
lower-left, 180 × 60, the same for every document.

### The policy census, and the two levels in one run

The sweep runs under the default `Restrict(On)`, asking the question exactly where
`viewer-core` asks it — `pdf_model::restriction::asserted`, once per operation — and a
document that withholds an operation is **refused by policy**, counted by clause, never
folded into a failure count: that is the policy working (trap 11). The same documents are
then swept again as `Restrict(Off)`, which `CLAUDE.md` says shall always be possible, so the
three assertions still bind them; the population that differs between the levels is exactly
the policy's own count, and running `Off` over only that population is deliberate — anywhere
else it would repeat the same save byte for byte and judge nothing new.

### A reference that cannot read the original cannot witness the save

The first run manufactured nine false disagreements, and the classification that dissolved
them is the round's main structural addition: **on any reference failure or missing witness,
the same script is pointed at the original document.** A reference that refuses the original,
cannot reach its first page, or cannot authenticate it is excluded from assertion 3's
denominator for that document, printed by reason — mupdf and poppler do not perform this
reader's §7.7.3.3 page-tree recovery, and a fuzzed file they refuse outright is still one
this reader opens and honestly appends to. The exclusion list is trap 11's arithmetic made
visible; a reference that reads the original fine and still misses the edit stays a real
disagreement, and the gate fails on it undiagnosed.

### Two reference passwords, each a measured gap in the reference

- **`pr6531_2.pdf`**: the empty string is this file's *owner* password — §7.6.4.4.11's
  Algorithm 12, the corpus's one file exercising that branch (ADR 0247's neighbourhood;
  `pdf-syntax`'s `an_empty_password_may_be_the_owner_password`). This reader authenticates
  it; **mupdf 1.28 refuses it** (`mutool draw` refuses the file with no password and opens it
  with the user password pdf.js pull request 6531 records). The references are handed that
  user password.
- **`saslprep-r6.pdf`**: the password is one §7.6.4.3.3's `SASLprep` *changes*, and neither
  reference implements the preprocessing — both refuse the stated password and both accept
  its normalised form, measured this round. The normalised form is the same key.

Both live in `REFERENCE_PASSWORDS` with the diagnosis beside them, apart from
`KNOWN_PASSWORDS` (which is what *this* reader authenticates with), because the two lists
answer different questions and merging them would change the policy census —
`print_protection.pdf`'s owner password grants what its `/P` withholds.

### Nothing is cached, and what bounds the runtime instead

`pdfref`'s cache keys on the invocation, the renderer's version and the document's SHA-256
(trap 10a). A freshly-written file's hash changes whenever the writer changes — most rounds
that would run this — and on **every** run for an encrypted document (§7.6.3.2's fresh
initialisation vectors, ADR 0129). A cache keyed on bytes that are new each time never hits,
so the instrument uses none and says so in its module comment. What bounds the runtime
instead is that the questions are object reads rather than renders — tens of milliseconds of
reference processor time per document against the seconds a render costs, three orders under
the oracle's 1020 s of reference CPU — so the whole population runs in minutes at worst,
**no sampling is needed and the denominator is everything**. ADR 0323's sampling clause was
for a cost that did not materialise. (No wall-clock figure is recorded: the round ran beside
nine parallel ones and a wall clock under that load measures the load.)

### The oracle-over-saved-files half: priced, not built

ADR 0323 ordered it "last, priced when built". The price, from the oracle's own accounting
(`pdfref`'s cache module: ~1020 s of reference-renderer CPU against 46 s of ours for the
corpus, ~319 MB of cache for 1794 pages):

- Reference renders of saved files can **never** amortise the way the corpus's do. The cache
  key contains the saved file's SHA-256, so every change to the writer flushes every entry,
  and an encrypted document's save is new bytes on every run. The steady state is the
  *uncached* rate, paid per writer-change rather than once.
- Over ADR 0323's bounded sample — every document with a field (80 fillable today) plus a
  fixed sample of the rest, say 200 documents — that is roughly 200/1794 of the corpus's
  reference cost: **about two minutes of reference CPU per writer-change**, plus ~35 MB of
  cache that expires with the next writer-change if cached at all.
- What it buys beyond this instrument: a verdict on the *appearance streams* this writer
  generates (§12.7.4.3's layout, §12.5.6.6's construction) as pixels, judged by the existing
  consensus machinery with its measured bounds. That is a real question — the witness scripts
  read `/V` and `/Contents`, not the marks — and it is the next instrument-2 increment for a
  round that wants it, at the price above.

## What the first run established

The distribution is in the history file. The shape of it: the three exact assertions hold
over the whole population that saves, under both levels, with zero undiagnosed disagreements;
every document that does not save says why, and the reasons partition into the policy census,
`UpdateError`'s construction refusals (dominated by tables rebuilt by scanning — §7.5.6's
update has nothing honest to chain to), the two encryptions this reader refuses, and the
pageless. The nine reference exclusions are all files this reader recovers and the references
refuse — including `issue19484_1/2.pdf`, whose self-contradictory key statement (a 40-bit
`/Length` under `/V 4` with no `/CF`; `corpus.rs`'s `MAX_PAGELESS` note has the reading) makes
every reader's answer a function of which of the file's two claims it believed. Our appended
strings are consistent with our stated reading; theirs with Acrobat's padding; the *file* is
what disagrees.

## Consequences

- The save path now has what the raster has had since ADR 0005: an independent judge at
  corpus scale, with the denominator stated and every exclusion named.
- `doc/todo/05`'s second build item is done; the third (the accessibility ratchet) is the
  remaining round.
- A later round ratchets the census counts once they have held, and the gate line joins §2 by
  the standing rule — nothing in this ADR pre-empts either.
