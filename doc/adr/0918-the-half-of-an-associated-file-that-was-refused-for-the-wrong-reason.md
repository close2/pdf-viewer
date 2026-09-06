# 0918 — The half of an associated file that was refused, and the reason that was two claims

Session 939. Status: **accepted**. §14.13.2 states two forms of associated file and this reader
read one of them, under a refusal that belonged to the other half of the sentence.

## Context

§14.13.2's first sentence is the whole of what the clause asks a reader:

> The file specification for an associated file represents either a file external to the PDF file
> or an embedded file stream (see 7.11.4, "Embedded file streams") within the PDF file.

There is no modal verb in it, which is why every sweep in `doc/todo/01` was blind to it and why the
row sat in `--bin permitted`'s *no modal verb* bucket — the bucket session 933 said must be read by
hand and never moved on the flag (ADR 0907). Read by hand, it is a requirement stated in the
indicative, exactly as §12.7.8.3.2's is.

`attachment::associated` read the second form and skipped the first, and the comment above the skip
said why:

> a specification with no `/EF` is skipped: §14.13.2 permits an external associated file … and an
> external one is §7.11.1's refusal, which this program has no filesystem to lift.

**That is two claims in one coat, and only one of them is true.** §7.11's own ledger row states the
distinction it elides, in a sentence written for exactly this failure: "what this row used to
record as unread was the difference between refusing a file and being unable to name it", and it
goes on to list "§14.13's associated ones" among the places where a file's *name* now reaches a
person. It did not. A conforming PDF 2.0 file that associates an external spreadsheet with the
chart drawn from it produced, from this reader, nothing at all — not the name, not the
`/AFRelationship` the producer asserted, not a word to say something had been dropped.

**The reasoning is older than the comment**, and `doc/todo/01`'s fourth sweep found where: ADR 0077
— *one array in seven places*, the round that wrote `associated` — states it as a decision, "§14.13.2
permits them … and an external one is §7.11.1's refusal, which no filesystem here can lift". That
ADR is not edited, on ADR 0232 §2's rule that a decision record is not rewritten to follow the tree
that moved under it; this one is where a reader following that sentence arrives.

The cousin-pair shape is ADR 0205's seventh: two rows about one mechanism, one giving a
*capability* reason where the other names code. §7.11.1 and §7.11 are `implemented` on the reading
that naming is done; §14.13.2 was `partial` on the reading that it is refused.

## Decision

**Read both forms, refuse only the bytes, and say so when a document uses the form we cannot
follow.**

- `attachment::external_associated` is the second reader, deliberately the mirror of
  `associated_under` with the test inverted, so that one `/AF` array cannot become two
  populations: every item is either an `Attachment` or an `ExternalAssociatedFile`, and an item
  that is not a dictionary is neither.
- `ExternalAssociatedFile` carries what an external specification actually holds — the file's own
  name and `Relationship`. There is no stream, so there are no `/Params`, no `/Size`, no media
  type, and the type says so by having no room for them.
- `viewer_core::notes::about` says it once when the document opens, beside §7.11.4's embedded-file
  sentences. That module's whole subject is claims about the *file* rather than about a page, and
  "what you are looking at is not the whole of what the producer assembled" is one.

**What is deliberately not done.** Following the file stays refused, and A24 does not change that:
the broker may hand the renderer a resource it opened itself, but no host offers such a port for a
document's own external references and none is proposed here. And the report's scope is the
catalog, which is the scope the embedded list beside it already has — §14.13.4 to §14.13.9's other
carriers are read by the same function and reach no caller. That is the fifth sweep's shape (a
reader every caller *could* reach is not a reader every caller does) and it is written into
§14.13.2's row rather than left to be found.

## The population, measured before the report was written

`crates/pdf-model/examples/associated_file_census.rs` counts the two forms over a corpus by walking
the object graph rather than §14.13.1's eight carriers, because an `/AF` array means the same thing
wherever it hangs and enumerating the carriers would measure this program's reach instead of the
corpus's contents. `/MCAF` is counted beside it, for §14.13.5's property-list form.

| population | documents stating an array | arrays | specifications | embedded | external |
|---|---|---|---|---|---|
| `doc/pdf.js`, the 974 | 7 | 36 | 44 | 44 | **0** |
| `CC-MAIN-2021-31`, 65 944 | 23 | 47 | 45 | 45 | **0** |

**Not one witness in either.** So both tests are built rather than borrowed (trap 8), and the
condition the report fires on has no corpus member to fire on — which is the point rather than an
objection: this is a requirement the corpus cannot rank, the same shape as Annex O's fragment
identifiers, which no document can contain at all. `CLAUDE.md`'s two denominators are the argument;
coverage found what robustness cannot see.

The census is also why the *report* is safe (trap 11): it fires on a specification with no `/EF`
under an `/AF`, and no document in 66 918 states one, so no page's diagnosis moves.

## Calibration

Both tests were run against the defect they deny (trap 13).

- `external_associated_under`'s push replaced by a `continue` — the behaviour it replaces:
  `an_associated_file_outside_the_document_is_named_rather_than_dropped` fails, the other eleven
  `attachment` tests pass.
- the note's loop replaced by an empty one:
  `a_file_the_document_associates_but_does_not_carry_is_named_when_it_opens` fails,
  the other 115 `headless` tests pass.

## Consequences

§14.13.2 moves `partial` → `implemented`, and what is left in the subclause is stated in the row
rather than dropped: an embedded associated file's `/Params` with a `/ModDate`, and a valid MIME
type for `/Subtype` with `application/octet-stream` where it is unknown, are `shall`s on whoever
*writes* an associated file. Nothing in `crates/`, `tools/` or `fuzz/` writes an `/AF` —
`pdf_transform`'s `Attach` files a §7.11.4 stream in §7.7.4's tree and asserts no relationship — so
they are writer-side today and become live the day this program associates a file. That is
`doc/habits.md`'s standing rule about a clause gaining a new verb, and it is the second time this
tree has had to apply it since the writer arrived.

`Relationship::as_str` exists because a person is shown Table 43's own name; a registered
second-class name is given back as the producer spelled it, on the table's NOTE 2, which is the
same reason `Relationship::Other` exists at all.
