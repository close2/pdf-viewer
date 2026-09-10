# 0952 — The three largest refusals, and what each of them costs

Session 952. Status: **accepted**. The sixth slice of `quorra-transform archive`
(`doc/questions/A22`): the three requirements that stopped the most conversions, taken in the
corpus sweep's own rank order, plus an argued refusal for the fourth.

The sweep is the instrument, and its ranking is what chose the order rather than a reading of the
standard. What each row cost, measured before and after by
`cargo test -p pdf-transform --test archive_corpus -- --ignored`, is in the sweep's output; this
file records what was decided and why, which no command prints.

## 0. The sweep now authorises every loss, and that changes what it ranks

`Authorisations::default()` authorises nothing, so a requirement answered by a **loss** stopped a
conversion in the sweep exactly as a requirement answered by nothing did. Those are two different
facts: one is a question put to a user, the other a gap in the converter. A ranking that ran them
together would have ranked section 3 of `doc/pdf-a-conversion-limits.md` beside its section 5 —
and the largest single refusal in the corpus turned out to be a section 3 row.

So the sweep authorises all of them and says so. The default run is what a *user* gets; the sweep
is what the next slice reads.

## 1. `metadata/properties-use-known-schemas` — the largest, and it is a loss

ISO 19005-2 section 6.6.2.3.1 requires every property to *use* one of the predefined schemas, and
a packet can name a predefined namespace without using it. `doc/pdf-a-conversion-limits.md`
section 3.9 had already closed two of the three routes out — correcting the value invents content,
because a schema says what *shape* a value has and not what this document meant, and describing a
predefined schema in section 6.6.2.3.2's extension container misrepresents it in the file itself
— which leaves removing the property.

Three decisions inside that, and each was a choice:

- **The judgement is not made twice.** `pdf_archive::properties_outside_their_schema` is the
  requirement's own predicate, factored to answer as a *list* rather than as a verdict and
  exported. A converter with a reading of its own would eventually cut a property the validator
  would have passed, and nobody would have found out from either side.
- **The cut is by span, not by re-serialising.** `pdf_model::xmp::remove` takes the property out
  of the producer's own bytes, in both of ISO 16684-1 section 7.5's spellings, and every other
  byte crosses unchanged — `restate`'s argument, applied to a narrower population. The population
  is what needed the work: `restate` removes a whole *namespace* at any depth, and this must
  remove *named properties of the packet* and nothing that shares a name with one deeper inside
  somebody else's structured value. So the editor learned what an `rdf:Description` directly
  inside the packet's `rdf:RDF` is, which is the reader's own `Kind::Description`.
- **The packet is read back.** A cut that left one of the properties behind is a half-edited
  packet, and a half-edited packet refuses the document. The writer says what it cut; only the
  packet says what it holds, and only the second of those is what a validator will see.

The loss is `Loss::MetadataProperty`, off by default, and its report names every property removed,
its namespace and the value that was there — section 3.9's condition, and the only place a user
can read what the file used to say, because a removed property leaves nothing behind to notice.

## 2. `annotations/flags-entry-present` — read the clause before calling it mechanical

Writing an `/F` where a file states none looks like the emptiest rewrite in the table. It is not.
§12.5.2's Table 166 gives `/F` a default of 0, and §12.5.3's Table 167 says what a clear `Print`
bit means: "If clear, never print the annotation, regardless of whether it is rendered on the
screen." So the file, read as the standard defines it, *says* the annotation is never printed, and
the only value ISO 19005 admits says the opposite. That is a loss —
`Loss::AnnotationPrinting` — and it is `doc/pdf-a-conversion-limits.md` section 3.7's first half.

The value written is `4`: bit position 3 alone, every other flag left at the default the table had
already put in force. **The one bit that changes is the one the requirement is about**, which is
also why the second half of section 6.3.2 — the `Print` bit set and four others clear on an `/F`
the producer *did* write — is a separate row this slice does not answer. Its other future is
removing the annotation, and that is not offered.

What the loss costs is bounded by the same table's next sentence, and it is worth knowing: "If the
annotation does not contain any appearance streams this flag shall be ignored." An annotation with
no appearance prints no differently for this; one with an appearance starts appearing on paper.

## 3. `annotations/appearance-dictionary-present` — a `Stated`, and four refusals inside it

`doc/questions/A21` permits constructing an appearance and makes reporting every one the condition.
§12.5.2's Table 166 is why it is not invention — "A PDF writer shall include an appearance
dictionary when writing or updating the PDF file except for the two cases listed below" — and each
subtype's own clause is what says the marks. So the decision is `Decision::Stated`, carrying the
sentence about what a *different* reader is now told, and `pdf_model::appearance` is the
construction the viewer already draws.

**What is new is the four cases where writing one is refused**, and each is the same principle from
a different side — `doc/questions/A48`'s *state an interpretation the standard defines; never fill
in an absence*:

- a subtype whose clause states no artwork at all (a stamp's legend, a caret, an unapplied
  redaction, a printer's mark, 3D). `appearance.rs`'s arms carry the reading for each;
- a construction this program can complete only **in part**. It draws it for a reader today, and
  freezing a partial rendering into an archive is a different act: a conforming reader would then
  have nothing else to draw from, and §12.5.2 has it ignore the entries the construction failed on;
- a **button field's widget**, whose `/N` §12.7.5.2.3 makes a subdictionary of one appearance per
  state. Which states the button has is exactly what an absent `/AP` does not say;
- an annotation written directly into a page's `/Annots` rather than as an object, which nothing
  that rewrites objects can reach.

An annotation that legitimately draws **nothing** is not one of those. It gets an empty stream:
drawing nothing is what its own entries state, and an empty form `XObject` says exactly that —
the same shape of answer as section 2.2's empty glyph.

### 3.1 One bug this found in the stage that was supposed to be a table

`decide` consulted `Prepared::obstacle` for `Answer::Mechanical` and for the output intent's own
arm, and **not** for a general `Answer::Stated`. Session 951 added that gate for exactly this
reason — a rewrite is an answer only where the document can take it — and a `Stated` row was the
one shape it did not reach. A `Stamp` with no appearance was therefore decided *and reported* as
converted, and refused ten lines later by the stage-3 net, under a sentence about the output not
conforming rather than about the stamp. The net worked; the report lied. It is one arm now.

### 3.2 The interaction to know before running it

A constructed appearance paints in the annotation's own device colours — Table 166's `/C` and
`/IC`, and black where nothing states one. So a document with **no** device colour of its own now
needs section 4.1's output intent for marks the conversion itself wrote.

The converter does not add one on that account, and that is ADR 0947's first rule holding rather
than an oversight: nothing is changed that no *failed* requirement asked for, and the source failed
no colour requirement. Stage 3 refuses such a file and names the colour row. Making the rewrites a
fixpoint — validate, decide, rewrite, validate again, decide the new failures — is the shape that
would close it, and it is a design decision with a cost (a second output intent's worth of
`Decision::Stated` sentences that no requirement in the *input* justifies) rather than a patch.
It is not taken here.

## 4. `metadata/extension-schema-container-fields` — refused, by argument rather than by silence

The corpus's nineteen witnesses all leave out a field ISO 19005-2 section 6.6.2.3.3's tables
require: a name for the schema, a description of what a property means, the category saying whether
a property's value is derived from the document or supplied from outside it. None of those is in
the file anywhere.

It is a `REFUSED_BY_NAME` row with its own sentence now, and it is deliberately
`Because::NotBuiltYet` rather than `Because::TheFence`, which is ADR 0948's distinction taken
seriously in the harder direction. Three of the four fields are prose or a claim only the producer
holds — filling them in is A48's forbidden half. But `doc/pdf-a-conversion-limits.md` section 4.2
already calls emitting such a container a **Default** for the neighbouring row, "authoring in a
small way", and `pdfaSchema:prefix` is genuinely derivable, because the packet itself binds that
namespace to a prefix. So how far that permission reaches is an open question, and a question
nobody has answered is a gap rather than a fence. Saying "never" would have been the easier
sentence and the false one.

## 5. What this did not do

- **`forms/need-appearances-absent-or-false`.** Section 4.4's *interesting case* is an **Ask** and
  no interface exists to ask it. Nothing guards it in the converter and nothing needs to: such a
  document fails this requirement too, the table answers it with nothing, and the conversion stops
  before an appearance is written. That is a guard by construction, and it is recorded here because
  a later reader will look for the explicit one.
- **Section 3.7's second half**, removing an annotation the producer hid.
- **The fixpoint of 3.2.**
