# 1006 — A source that already conforms is copied, not rewritten — and the converter had never honoured section 3.6

Status: accepted. Session 985 (the merge of rounds 979–984).
Context: `crates/pdf-transform/src/archive/mod.rs` (`run`, `copy_the_source`),
`crates/pdf-archive/src/table/interaction.rs` (`digest_covers_the_whole_file`, ADR 1003),
`doc/pdf-a-conversion-limits.md` section 3.6, ADR 0947, `crates/pdf-transform/tests/archive_corpus.rs`.

## What failed

Merging the batch, `archive_corpus` stopped in 0.02 s on the first signed document it met:

```
veraPDF test suite 6-1-12-t02-pass-a.pdf: conformed already and no file was written
```

Run through the verb by hand, the converter said why:

```
still failed: signatures/digest-covers-the-whole-file — which this document met, so the conversion broke it
```

## What that is

Session 982 made ISO 19005-2 Annex B.1's range requirement checkable (ADR 1003). The output check
that ADR 0947 built — *a file leaves this verb only if it conforms, and a regression is named
separately* — then did exactly its job on the first signed document: the source met the
requirement, the output did not.

The output did not because **the converter re-serialises every document it converts, including
one it changes nothing in.** A conversion is a rewrite; RFC 0002 §10's serializer emits a new
object table, new streams and a new cross-reference; every byte offset moves. A signature covers a
byte range of a specific file. So the signature dictionary carried into the rewritten file no longer
covers the bytes it sits in — **a signature that lies** — and it had been written that way, into
files claiming PDF/A, for as long as the verb has existed, because no predicate could see a range.

`doc/pdf-a-conversion-limits.md` section 3.6 has said all along that *"a converted document does
not carry its source's signatures"* and classed it *"Ask, loudly"*: name each signature, its signer,
whether it validates, keep the fields and their appearances, state that the cryptographic assertion
is gone. **None of that was built.** The catalogue and the code disagreed for the whole life of the
verb, and the disagreement was invisible because the one requirement that could expose it was
`Unchecked`.

## The decision

**A source whose verdict is `Conforms` is copied to the sink byte for byte.** `run` short-circuits
after stage 1; the report's `conformed` list is filled as before, `achieved` says it conforms with
nothing failing, and the output's origin records zero changes.

This is not a workaround for the test. It is the answer to a question the verb had not asked: *what
is the conversion of a document that needs no conversion?* The only rewrite that cannot regress a
requirement is the one that changes no byte, and a conforming source is exactly the case where
nothing asked for a byte to change — `prepare.rs` already says *"nothing is prepared that no failed
requirement asked for"*, and then the serializer changed every offset anyway. The identity keeps
the signature valid, keeps the file's `/ID`, keeps its history, and keeps a claim the source had
already earned.

**A non-conforming signed source is refused, with the regression named.** That is the honest state
of the tree: the rewrite it needs cannot preserve the signature, section 3.6's report that would say
so is not built, and until it is, the output's own verdict refuses the file rather than writing one
that lies. The refusal sentence names `signatures/digest-covers-the-whole-file` as *the conversion
broke it*, which is true and is the whole of what a reader needs.

## What is owed, precisely

Section 3.6's mechanism, as a converter round: a loss kind for the signature's cryptographic
assertion, asked for by name; a rewrite that removes each signature field's value and keeps the
field and its appearance (§12.7.5.5, and ISO 19005-2 6.3.3's requirement that annotations carry
appearances); and the report line per signature — its `/Name`, its `/M`, and whether
`pdf_model::signature` validated it over the *source* — so that a range the source got wrong is
stated rather than dropped with the rest. `SIGNATURE_RANGE_IS_THE_SIGNERS` in `decision.rs` already
says this is what the row waits on. The population is small — the corpus holds eight signed
documents, three of them pass-cases — and every one of the five fail-cases is refused correctly
today.

## A trap, for the record

**A catalogue that classes a loss "Ask, loudly" is a claim that the code asks.** The limits document
had the right answer for ninety-odd sessions and the converter never read it, and nothing compared
the two — the same shape as ADR 0989's ledger rows quoting a retired sentence of `CLAUDE.md`. The
instrument that finally compared them was a *predicate*, written for a different reason. When a
requirement moves from `Unchecked` to `Implemented`, the converter's output check is the first
reader of every claim the catalogue made about that requirement, and it should be run over the
corpus before the promotion is believed.
