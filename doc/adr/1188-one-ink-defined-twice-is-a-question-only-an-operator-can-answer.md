# 1188 — One ink defined twice is a question only an operator can answer

Status: accepted and **built**. Session 1175.
Context: `crates/pdf-transform/src/archive/{config,remedies,rewrite,sites,mod}.rs`,
`crates/pdf-archive/src/table/graphics.rs`, `crates/pdf-transform/tests/archive.rs`,
`crates/pdf-transform/tests/archive_corpus.rs`.
Answers: `doc/todo/66`'s top refusal at PDF/A-2b and PDF/A-4, and
`doc/pdf-a-mitigations.md` section 4.2's `graphics/separations-of-one-name-agree`, which this moves
from *catalogued* to *built*.
Builds: `doc/rfc/0007` sections 1 and 2 (a refusal is a question, and `stop` is one answer among
several), `doc/questions/A48` (state an interpretation the standard defines; never fill in an
absence), `doc/adr/0947` (nothing changes which no failed requirement asked to change).
Clauses: ISO 32000-2 §8.6.6.4, §7.10; ISO 19005-2 section 6.2.4.4, ISO 19005-4 section 6.2.4.4.

## 1. What the clause asks, and why it cannot be answered mechanically

Both parts require every `Separation` array in a file that names one colourant to state the same
alternate space and the same tint transform. A file that states two has defined one ink twice, and
**nothing in the file says which its producer meant**.

§8.6.6.4 is why that matters on a screen rather than only on a press. A `Separation` space is a
tint through a named colourant, and a device that has the colourant paints it directly — but:

> The preceding paragraph applies only to subtractive output devices such as printers and
> imagesetters. For an additive device such as a computer display, a Separation colour space never
> applies a process colourant directly; it always reverts to the alternate colour space as
> described below.

So on the device this program draws to, the alternate space and the tint transform *are* the ink.
Choosing one definition over another repaints every mark drawn through the loser.

**The two elements move together, and §8.6.6.4 says why**: the tint transform "shall be called with
the tint value and shall return the corresponding colour component values", and "the number of
components and the interpretation of their values shall depend on the alternate colour space". A
transform written beside a different space is a function whose output nothing can read, so element 2
and element 3 are replaced as one statement or not at all.

## 2. The decision: a `supply`, and what the operator supplies is a choice, not a definition

`doc/pdf-a-mitigations.md` section 4.2 calls this the model case for `supply`, and it is right; what
this ADR settles is *what is supplied*. The catalogue's entry offered an explicit table —
`colourants = { "PANTONE 293 C" = "…" }` — and that shape cannot be built honestly: a tint transform
is a §7.10 function, in general a sampled stream or a PostScript calculator program, and no
configuration file holds one. A converter that accepted a colour there would be writing a definition
the document never stated, which is the far side of `A48`'s line.

So the configuration states **which of the document's own definitions wins**:

```toml
[site."graphics/separations-of-one-name-agree"]
remedy = "supply"
winner = "first"          # or "most-used"
```

Every byte the rewrite writes is then the producer's: the alternate space and tint transform put
into the losing arrays are the objects a winning array already stated. What the operator
contributed is which of the file's own answers survives — a fact their ink book holds and the file
does not.

- **`first`** is the first definition in the document's own object order, which is the one the
  validator reports the others *against*.
- **`most-used`** is the definition the most `Separation` arrays state, ties going to the first.
  **Counted by definitions written, not by marks painted**, and an operator has to know that: the
  file says how many times each definition appears and says nothing about how much of any page each
  one covers. Naming it otherwise would promise a weighting nothing in the file supports.

A site the configuration does not name stops, as every site does; a row naming the site without a
`winner` states no fact and is listed by `Configuration::unbuilt` rather than guessed at.

## 3. Whether this has to be an *ask* rather than a `supply` — measured

`doc/veraPDF-corpus` holds four witnesses at PDF/A-2b and four at PDF/A-4. Page one of each was
drawn by the correctness oracle at three times nominal size, before and after the conversion
(`crates/pdf-transform/examples/r1175_separation_pixels.rs`, a scratch instrument, not a gate):

| | `6-2-4-4-t03-fail-a` | `fail-b` | `fail-c`, `fail-d` |
|---|---|---|---|
| source vs converted, worst tile | **39.62** | 24.58 | 0.00 |
| source vs converted, max channel | **242** | 187 | 0 |
| `first` vs `most-used`, worst tile | 0.00 | 0.00 | 0.00 |

The pages are not blank — about 43,200 of 4,362,336 pixels carry ink — so the zeros are agreement
rather than emptiness.

Two things follow, and they point in different directions:

- **The remedy changes the page, and by a lot where it changes it at all.** A worst tile of 39.6 and
  a maximum channel difference of 242 is not a rounding artefact; it is a mark that is a different
  colour. So this may never be a default, and `stop` staying the default is not caution but the
  correct answer.
- **The two words agree on every witness**, because each of these files states each definition
  exactly once: `most-used` ties at one apiece and the tie goes to the first. That is a fact about
  this corpus and not about the requirement, and it is written here so that a later round does not
  read the zeros as a licence to drop one of the words.

**So no fourth mode is owed.** `doc/rfc/0007` section 1's reframe is that a refusal *is* the
question and the configuration is where it is answered in advance; `stop` is the ask, and it is what
a caller who says nothing gets. `CLAUDE.md`'s four levels — off, on, ask, warn — are about the
restrictions a *document* asserts over its reader, which is the opposite direction and a different
subject. What this site owes beyond `stop` is that the report names, per colourant, how many
definitions were found and which won, and that the file's own `xmpMM:History` records that a person
rather than the document decided it. Both are built, under `doc/rfc/0007` section 5b.1's obligation.

## 4. Two smaller decisions, recorded so they are not re-taken

**Sameness is `pdf_archive`'s, not a second reading.** Grouping a colourant's definitions needs the
notion of *the same* that the requirement is failed on — indirection set aside, and the compression
of a function stream set aside with it. That reading lives in the validator, so it is exported
(`pdf_archive::same_parameter`) rather than made twice. A converter grouping differently would write
a file that still fails the rule it was answering.

**`All` and `None` are not special-cased, and could have been.** §8.6.6.4 says a processor
"shall ignore the alternateSpace and tintTransform parameters" for those two names, so unifying
their definitions changes nothing any reader shows and would be `Mechanical`. It is not built,
because the `supply` already answers those files correctly and safely — both definitions are ignored
whichever wins — and a second `Answer` shape would exist to spare an operator a line of
configuration on a document nothing in the corpus exhibits. If a witness turns up, the narrowing is
sound and this paragraph is where it starts.

**Every array of a decided colourant is rewritten, including one ISO 19005-2 section 6.2.2 exempts.**
The exemption puts a named resource nothing references outside the requirement's *population*, so
such an array is not what failed. Rewriting it all the same costs no mark — nothing draws through
it — and the alternative is a converter that has to reconstruct the validator's exemption set to
decide which copies of one ink to leave disagreeing. The clause's own sentence is about every array
in the file, and that is what the rewrite does.
