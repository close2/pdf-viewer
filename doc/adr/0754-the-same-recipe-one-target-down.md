# ADR 0754 — The same recipe one target down, and the three claims that came with the population

Status: accepted, 2026-08-29. Session 825. Cites ISO 32000-2 §12.8.1, §12.8.3.2, §12.8.3.3.1,
§12.8.3.4.3, §12.8.4.4's Table 262 and §12.8.5 for where a PDF keeps a CMS object, and RFC 5652
section 5 and RFC 3161 section 3.3 for what one is; two conformance ledger rows are corrected and
neither status moves, because nothing here changes what this program does with any clause.
It takes the finding ADR 0751 named and declined, and sits beside 0742 (a fuzz run that exits zero
without fuzzing), 0229 (`x509`, `cms` and why the seeder is a second implementation) and 0747.

ADR 0751 ends on a paragraph naming its own successor:

> **The `cms` block one entry up has the same defect and this round did not fix it.** It names the
> same single submodule, and `pdf_model::cms` is the reader every one of those signatures goes
> through before `x509` sees a certificate at all. It is the same finding, on the same population,
> and it is named here rather than half-done.

This round is that round, and it found the defect is not one thing but three: a recipe that named
one submodule, and **two claims in the tree that were counted over the same submodule and read as
claims about the world**.

## Part one — the recipe, and why it is a script this time

`doc/verify.md`'s `cms` block said to seed the target with "the eleven `/Contents` blobs the nine
signed corpus documents hold". That sentence is a *population* in the imperative mood, which is
exactly the shape ADR 0751 found in `seed_x509.py`'s argument list, and it decayed the same way:
the nine are `doc/pdf.js`'s, and `grep -alr /ByteRange` over every corpus on the disk prints two
orders of magnitude more.

**It was a sentence rather than a script, and that is the part worth reading.** `x509` had a
program with a wrong argument list; `cms` had a paragraph telling a round to go and find eleven
blobs by hand. A paragraph cannot be pointed at a new corpus, cannot be re-run when the crawl
grows, and above all **cannot be calibrated** — there is nothing for trap 13 to plant a defect in.
The recipe is now `fuzz/seed_cms.py`, and the first thing that happened when it was calibrated is
in part three.

### Three routes, and the third one is a clause nobody had read for this

`seed_x509.py`'s three routes were chosen because a *certificate* reaches a PDF three ways. A CMS
object reaches one by three quite different ways, and only the first is obvious:

- **The signature value.** §12.8.3.3.1's CMS object in Table 255's `/Contents`, scanned as
  hexadecimal beside the `/ByteRange` that excludes it. This route keeps **the file's own bytes**,
  because a producer's indefinite-length BER is the one shape a fuzzer will not invent and
  re-encoding it would throw exactly that away.
- **What a signature carries inside itself.** RFC 3161's `TimeStampToken` is a `ContentInfo` in its
  own right, and §12.8.3.4.3's signature-time-stamp attribute puts one inside a `SignerInfo` — as
  do the archive timestamps a long-term signature adds beside it. **No scan of the file can see
  these**, because the file states them in hexadecimal inside another CMS object; only a walk
  reaches them, which is the second implementation ADR 0229 wanted. The walk does *not* enumerate
  attribute identifiers: any attribute value that is itself a `ContentInfo` over `id-signedData` is
  one, whichever of the half-dozen timestamp attributes carried it. Enumerating would have been a
  list this tree cannot check against a document it holds, and it would have gone stale at the next
  profile.
- **What a document states as an object.** §12.8.4.4's Table 262: a signature VRI dictionary's
  `/TS` is "[a] stream containing the DER-encoded timestamp (see Internet RFC 3161 as updated by
  Internet RFC 5816 )". Found by proposing on RFC 5652's opening bytes and disposing by the same
  walk, in the file's bytes and in its inflated streams.

**There is no fourth route out of this tree's own fixtures, and that is a finding rather than an
omission.** `seed_x509.py` has one because `crates/pdf-model/src/*.rs` state their certificates as
hexadecimal constants; `cms.rs`'s `fixtures` module *builds* its signature values in Rust at test
time, so there is no constant to read and no route to write. What follows is written into
`doc/verify.md`'s block: the signature shapes that module constructs reach no corpus, and what the
corpus has of them is whatever the documents have — which is what part two is about.

### One module, because two copies of a parser is how the agreement stops being one

Everything below the structure is the same work in both seeders — X.690's walk with clause
8.1.3.6's indefinite lengths, the `stream` inflation, the `/Contents` hexadecimal, the argument
list — so it is `fuzz/seed_der.py` now and both import it. The alternative was copying about a
hundred and fifty lines of tag-length-value walking into a second file, and **a copied parser
drifts**: the whole value ADR 0229 claims for a Python seeder is that it is a *second*
implementation of `pdf_model::der`, and two forks of one Python reader agreeing with each other
demonstrate nothing at all. The refactor was proved behaviour-preserving before anything else
happened — `seed_x509.py` at `3c259925` and after, over the same 4974 documents, produced a
byte-identical harvest and an identical route tally.

## Part two — two claims in the tree that had this population inside them

The census over the wider population is `examples/signature_algorithm_census`, which already
existed and which `doc/todo/51` already documents pointing at the whole disk. **Running it was
enough to falsify two sentences that had been read for many sessions as facts about signed PDFs**:

- **`cms.rs`'s `fixtures` module** opened "Four of the six signature formats §12.8.3 defines have no
  witness in the 974 — no document timestamp, no `PAdES` signature, no `adbe.x509.rsa_sha1` …". All
  six have witnesses on this disk, and three of the four named have them in numbers.
- **§12.8.5's ledger row** said "**No corpus document carries a document timestamp**, so the witness
  is a fixture (trap 8)", and had been *re-derived* in the six-hundred-and-forty-first session by
  running the census over the 974 and getting zero. That was a correct measurement of the wrong
  denominator, and it is the sharpest instance in this tree of ADR 0403's rule: **a negative claim
  carries its population inside it, and re-deriving it with the same population confirms nothing.**
  §12.8.3.4.2's "four corpus documents write" indefinite lengths is the same shape and is corrected
  beside it.

**Neither fixture is retired, and the reason is trap 8 read from the other side.** A witness found
in a crawl is a file nobody wrote for this purpose: it can *rank* a format — say how much of the
world uses it — and it cannot *define* one, which is what an assertion needs. What the witnesses
change is which of these shapes the `cms` target now sees real examples of, and which claims in the
tree name their denominator.

The census also confirmed, rather than moved, two things `doc/todo/51` already records and asks not
to be re-opened: the two signatures stating BSI TR-03111's `0.4.0.127.0.7.1.1.4.1.3` are outside
what ISO/TS 32002 section 5.1.3's NOTE 2 admits and are correctly reported by their identifier, and
the brainpoolP256r1 key is refused by a package rather than by a clause. A round that reads this
ADR and reaches for either has misread the todo.

## Part three — what the calibration caught, which is the whole argument for having one

Trap 13 says a sweep for a defect must be run against the defect before it is believed. The first
run of the calibration reported **zero** for the route that scans a document's own bytes, on plants
built out of the harvest the other routes had just produced — objects that were provably there.

The cause is worth a sentence of its own. `id-signedData` opens on the octet `0x2A`, and the
candidate pattern was assembled by concatenating those octets into a byte regular expression.
`0x2A` is `*`. The pattern therefore said *zero or more of the preceding length octet*, matched
nothing anywhere, and **every route that used it returned an honest, silent zero** — a harvest that
would have run for an hour and reported a smaller number with no error, no warning and no way to
tell it from a corpus that genuinely held none. `re.escape` is the fix and the comment beside it
says why.

**This is trap 25's shape in a new place**: a hand-written population can name a thing that never
existed, and finding nothing there reads as a pass. The pattern is not a population, but it is a
*proposal* about one, and it failed in the same silent direction. The general rule the two share:
**an instrument that can only under-report has no failing state, so it has to be run against a
positive it cannot miss.** The plants here are the harvest's own objects, which is the cheapest
positive control a seeder can have and the reason the calibration is written the way it is.

## What this does not change

- **`tools/fuzz.sh` is not touched, and no length moves.** The wrapper reads each target's
  invocation out of `doc/verify.md`; this round changes no invocation, so `--list` prints what it
  printed before. ADR 0751 priced this target's search half at half a second and there is nothing
  in this round's figures to reopen that — the figures are in the history file, and what they say
  about seeds against iterations is the same thing 0751 said one target down.
- **No ratchet, and no corpus in the history.** `fuzz/corpus` is gitignored; what is committed is
  the recipe.
- **No crasher.** The runs this round made found none, which is a result rather than an absence for
  ADR 0747's reason: the `INITED → DONE` pair is what makes it a claim about the code.
- **No status moves in the ledger.** Both corrected rows stay `partial`; the correction is to a
  sentence's denominator, and neither sentence was load-bearing for the status.
