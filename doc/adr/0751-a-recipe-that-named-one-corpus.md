# ADR 0751 — A recipe that named one corpus, and a length chosen from a curve rather than its end

Status: accepted, 2026-08-29. Session 821. Cites ISO 32000-2 §12.8.3.3.1, §12.8.4.3's Table 261,
Table 255 and Table 238 for where a certificate reaches a PDF from, and RFC 5280 §4.1 and §5.1 for
what one is and what it is easily mistaken for; the conformance ledger is untouched, because
nothing here changes what this program does with any of them.
It continues ADR 0747, which left three findings in writing and named this round to take them, and
sits beside 0742 (a fuzz run that exits zero without fuzzing), 0229 (`x509`, `cms` and why the
seeder is a second implementation) and 0264 (`page`, and why a target over documents needs
documents).

ADR 0747 recorded three things and acted on none, deliberately:

> **Two documented lengths are too short** […] `display_list` gained +884 edges and +3549 features
> in its ten minutes […] and `confined_wire` gained +660 and +3189 while finishing its million runs
> in **156 seconds**. Both were still climbing when they stopped. […] the round that changes one
> should be able to say what the new one buys.
>
> **`x509` has 533 seeds, the fewest of any target** […] the target's fastest path to more coverage
> is more certificates rather than more runs.

This round is that round. **Neither of the two lengths is raised for the reason 0747 gave, and one
of them is raised for a different reason it could not have given.**

- `display_list` **stays at ten minutes.** Its 0747 figure was a property of the corpus it ran
  against, and 0747's own run changed that corpus.
- `confined_wire` **goes from a million runs to four million** — not because it is "still climbing",
  which out to eight million it also is, but because it is the *cheapest* target in this tree per
  unit of coverage and its budget was small relative to that.
- the eight saturated targets **stay exactly as they are**, and the pricing says why in one figure:
  the search half of all eight together is under thirteen seconds.
- `x509`'s corpus was thin because the recipe named one of the tree's PDF sources. Fixing the
  recipe and the script **more than doubled that target's edge coverage before a single mutation**,
  and bought about nineteen times the edges 0747's million-run campaign on it bought.

**A round that measures and declines is not a round that did nothing**, and this project already
holds the reason: `doc/habits.md`'s *Measuring* section says a price is a claim that decays, and
three of the four wrong prices it records were wrong in the direction of *doing the work*.

## Part one — the two lengths, re-priced against the corpus 0747's own campaign left behind

**The measurement 0747 could not make is the one that mattered: what a length buys is a property of
the corpus it is run against, and its campaign changed that corpus.** `display_list` went into that
campaign with 861 seeds and came out with 1512; `confined_wire` with 8075 and came out with 8458.
The `INITED → DONE` pair ADR 0747 built is exactly the instrument for asking what the *same* length
buys now, and the answer is that it is not the same number.

The figures, and the load each ran under, are in
`doc/history/821-a-recipe-that-named-one-corpus.md`. What belongs here is what they mean:

**Neither target was under-run in the sense 0747 meant. Both were under-seeded, and 0747's own
campaign was the cure.** Run at the documented length against the corpus that campaign produced,
`display_list` buys a twenty-fifth of the features it bought then and `confined_wire` a fifth.

**A target still climbing at the end of its budget is evidence of exactly one thing** — that there
was somewhere left to go — and it does not distinguish *this run needs longer* from *this corpus
needed this run*. Run the same length again after the seeds have landed and the two come apart,
because only the first predicts the same slope. **And "still climbing" is not even evidence of the
first**, which is what the `confined_wire` curve makes plain: measured at one, two, four and eight
million runs it is still climbing at every one of them, because the curve is logarithmic and a
logarithmic curve is always still climbing. A length can only be chosen against a budget.

**So the rule that generalises is trap 24's sentence pointed the other way**: a length is priced
against a corpus, so **a length priced immediately after a campaign that grew that corpus is priced
against a tree that no longer exists.** ADR 0747 was right to decline the change and right about
why — what it could not know is that the change it declined had already been made obsolete by its
own run.

**What `confined_wire`'s length is raised on instead is its rate.** At the documented million runs
it adds a couple of hundred edges in tens of seconds on a quiet machine, where `display_list` adds a
few dozen in ten minutes and `page` needs an hour; it is by a wide margin the cheapest coverage in
this tree, and a target that cheap was being given the smallest budget of the fifteen. Four million
is where the return per doubling halves, and it costs under three minutes. That is a reason 0747
could not have given, because it needs the curve rather than its last point.

**`display_list`'s length is the only one in `doc/verify.md` stated in wall clock, and it stays that
way — but a round reading its figures has to know what that means here.** Inside one run of it this
round the reported rate moved by nearly a factor of two as three sibling rounds built, so a budget
in seconds is not a fixed amount of work on this machine, which is `doc/todo/02` §2's own sentence
about a gate that spawns another program arriving in a second place.

The unit stays because the alternative gives up more than it buys. A run count is a fixed amount of
work and a variable amount of *time*, and what this target executes per input scales with the size
of the display lists its corpus happens to hold — so a run count that is ten minutes today is an
unbounded promise tomorrow, and `doc/verify.md` is a checklist a person works through. **The
guarantee worth keeping is the one on what a round spends**, and the question a run count would have
answered — how much work did this actually do — is the one `INITED → DONE` already answers, in
counters that are sets rather than rates and so do not move with the load at all.

### And the eight saturated targets, whose length is not worth an argument either

ADR 0747's saturation finding was that eight of the fifteen add under a hundred features over their
documented length and one of them adds none, and it left open what follows for the length. Priced
by running each of the eight twice — once at `-runs=0`, which loads the corpus, executes every seed
and stops, and once at the documented length — **the search half of those runs costs seconds**, so
there is nothing to save by shortening one and no reason to lengthen one.

The half that is *not* free is the replay, and it is the half worth keeping: `-runs=0` over a
mature corpus is every seed executed against today's code, which is a regression check on the
parser that no `#[test]` in the tree performs. **A saturated target is not a target doing nothing.
It is a target whose value has moved from the search to the replay**, and the length is what carries
the replay along with it.

## Part two — the recipe named one corpus, and the tree holds several

`x509` was the thinnest-seeded target in the tree, and the reason was one argument list:

```
python3 fuzz/seed_x509.py fuzz/corpus/x509 doc/pdf.js/test/pdfs/*.pdf
```

**`doc/pdf.js` is one of the tree's PDF sources and by far the smallest source of *signatures*
among them** — `doc/oracle-and-corpus.md` lists the submodule corpora and `corpus-cache/` holds the
crawl beside them. Point the *unchanged* script at every PDF the tree holds instead and it collects
what `grep -alr /ByteRange` over all of them says it should, which is two orders of magnitude more
documents than that line names and a harvest of certificates to match. Not one line of the walk had
to change: the population was the whole defect.

**This is `doc/habits.md`'s rule about negative claims, and nothing in this tree was watching for it
in a *recipe*.** That rule was written for a ledger row — *no corpus document does X* carries its
population inside it whether or not it says so — and a round after ADR 0403 re-ran every such
sentence in the ledger and found five false. **A seeding recipe makes the same claim in the
imperative mood**: *these are the files this corpus comes from* is a statement about a population,
it decays the moment the tree gains a corpus, and no sweep in this project reads one. Every
`doc/verify.md` block naming a seed source is the same shape.

### Two routes the script did not have, and one it now cannot do without

**A certificate reaches a PDF in three unrelated ways, and the script knew one.** §12.8.3.3.1's CMS
object is where a signature carries its signer's chain, and that route stays exactly as ADR 0229
built it — a hand-written walk of X.690, RFC 5652's `ContentInfo` and `SignedData`, and the
`certificates [0] IMPLICIT` member, which is the second implementation that argument wanted.

*What a document states as an object of its own.* §12.8.4.3's document security store puts an entire
validation chain in `/DSS`'s `/Certs`, one stream per certificate, and Table 255's `/Cert` and Table
238's `/Subject` state certificates directly. **Reaching those through the file's structure would
mean writing a PDF reader in Python** — a cross-reference table, object streams, a `/Filter`
pipeline — over files chosen for being malformed. A DER certificate is self-delimiting, so the new
route proposes by RFC 5280 §4.1's opening bytes and disposes by the walk the first route already
has.

*What this tree states in hexadecimal.* `crates/pdf-model/src/{x509,dsa,pss,ecdsa,eddsa}.rs` carry
the certificates their `fixtures` modules verify against, and until now `doc/verify.md` asked a
round to re-make the interesting ones with `openssl req -new -x509` by hand. **That is the route
that makes a clone's corpus complete rather than merely large**: the DSA certificate is the only
input that reaches `dsa::verify` at all, and the P-384, P-521, brainpoolP256r1 and Ed25519 ones are
the only inputs that reach the arms ADR 0689 added. A corpus is not recoverable from the history
because it was never in it (ADR 0742); this closes the last part of it that was not recoverable from
the tree either.

### The near miss, which is the part worth reading

**RFC 5280 §5.1's `CertificateList` has a certificate's outer shape exactly.** Three members —
a `SEQUENCE`, an `AlgorithmIdentifier` and a `BIT STRING` — and it repeats its algorithm identifier
inside the first of them the same way a certificate does, so §4.1.1.2's equality rule does not
separate the two either; every revocation list this round's scan found satisfies it. It is also, by
§12.8.4.3's Table 261, in `/CRLs` **immediately beside** the certificates in `/Certs`, so a route
that scans for certificates in a document security store is scanning a place that holds both.

A check of the outer shape alone therefore harvests revocation lists, and on this disk the largest
of them is 1.5 MB — a seed two orders of magnitude past any certificate in the corpus, in a target
whose inputs are otherwise kilobytes. **What separates the two is a field**: a certificate's
`Validity` is two `Time`s where a revocation list's `thisUpdate` is one. So `is_certificate` reads
RFC 5280 §4.1's field list as far as `subjectPublicKeyInfo` and checks that.

**The general shape: a structure recognised by its opening bytes is recognised by its *shape*, and a
format usually has a sibling with the same one.** What it costs to miss the sibling is not a wrong
answer — every byte string is a legal input to a fuzz target — but a corpus quietly filling with
megabyte seeds that no longer say what the target is for.

### What it was worth, and the sentence it earns

The figures are in the history file. The one that matters is the shape: **the harvest more than
doubled this target's edge coverage with no fuzzing at all**, where ADR 0747's million-run campaign
on the same target moved it by under eight per cent of that. Seeds beat iterations by more than an
order of magnitude here, on a target where the seeds were a *recipe's argument list* away.

**And the seeds made the search worth more too**, which is the half a "seeds, not iterations"
slogan leaves out: run at the documented length afterwards, the same million runs bought three
times the features they bought against the old corpus. A fuzzer mutates what it has, so a corpus
that states more is a corpus a run can go further from. The two are not alternatives.

### The gate on inflation, and what it gives up

A `/Certs` entry is a stream, so this route inflates each file's stream bodies and scans those too;
and inflating every stream of every file in the crawl is most of a day. The script therefore
inflates only where the file names one of the keys a certificate collection is reached through, and
**the trade is measured in both directions in the comment beside it**, because CLAUDE.md's rule on
optimisation asks for the number that justifies one and the cost it carries. The cost is real and
stated: certificates inside a Flate stream of a document that names none of those keys are not
harvested, and there are some.

## What this does not change

- **`tools/fuzz.sh` is not touched, and that is the point of how it was built.** It reads each
  target's invocation out of `doc/verify.md`, so the one length this round changes is changed in
  one place and the wrapper follows it. `tools/fuzz.sh --list` printing the new figure is the
  check, and it was run.
- **No ratchet, and no corpus in the history.** ADR 0742 argued both and ADR 0747 restated them; a
  campaign's figures are a record of a run, and `fuzz/corpus` is gitignored. What is committed is
  the recipe, and this round's whole contribution to `x509` is a better one.
- **No crasher.** The runs this round made found none, which is a result rather than an absence for
  the reason ADR 0747 gives — the `INITED → DONE` pair is what makes it a claim about the code.
- **The `cms` block one entry up has the same defect and this round did not fix it.** It names the
  same single submodule, and `pdf_model::cms` is the reader every one of those signatures goes
  through before `x509` sees a certificate at all. It is the same finding, on the same population,
  and it is named here rather than half-done.
