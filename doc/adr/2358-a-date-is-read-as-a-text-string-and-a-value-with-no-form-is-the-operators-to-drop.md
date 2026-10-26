# 2358 — A date is read as a text string, a value with no form is the operator's to drop, and a dictionary stating nothing goes

Session 1762, the archive slot of batch one hundred and sixteen.
Status: implemented. Amends ADR 1269, whose two ISO 19005-4 section 6.1.3 sites it widens.
Context: `doc/todo/72`; `doc/pdf-a-mitigations.md`'s entry for the two sites; `doc/questions/A48`.
Clauses: ISO 32000-2 §7.9.4, §7.9.2.2, §7.3.9, §14.3.3, §14.3.4; ISO 19005-4 section 6.1.3.

## The census

Under both shipped lower-bound profiles, the most-populated refusal sentence whose remedy is a
conversion — the composite-font refusals wait on Q570 and Q596, the image-smoothing, machine-face
and substitute-stretch losses are the profiles' own `stop`s, the fences are fences, and
`DeviceCMYK` beside an RGB intent is answered by an operator's press profile (ADR 2130) — was
ADR 1269's "this document information dictionary states a CreationDate or ModDate that is not
§7.9.4's date". It refuses only at part 4: part 2 carries the dictionary through untouched.

## 1. A date is a text string first

§7.9.4 makes a date "a text string value", and its NOTE 1 says the grammar describes "the text
string value after appropriate processing". `prepare_information` read the string's raw bytes as
UTF-8, so a date written in UTF-16BE (§7.9.2.2's first encoding) or behind UTF-8's lead bytes was
called malformed. It now decodes through `pdf_syntax::text_string` and then parses, which is what
`pdf_model::collection` already did; `archive::attach::modification_date` had the same defect and
takes the same reading. Nothing is decided: the instant is the one the producer wrote.

## 2. A value no reading makes a date is the operator's to drop

What is left is not a date at all: a year with three digits too many, no `D:` prefix, an English
sentence, an empty string, a month of 42. Each names no instant, so `xmp:CreateDate` cannot hold it,
and reading an instant into it is `doc/questions/A48`'s forbidden repair ("never fill in an
absence"). ISO 19005-4 section 6.1.3 asks only that the dictionary go; moving its values is ADR
1269's preservation, and for this value the preservation has no home. That is the position
`unmapped` already answers for a key Table 349 does not define, so the answer has the same shape —
a new row key, `unreadable = "discard" | "stop"`, read like `undeterminable`, with `stop` what a
row saying nothing states. It also answers §14.3.3's other form failure, a value that is neither a
text string nor `/Trapped`'s name (`/Trapped false`, a boolean `/CreationDate`).

**A new key rather than a wider `unmapped`**, because a profile is a lower bound on what may be
lost: `as-if-printed`, `as-shown` and `only-metadata-loss` authorised dropping a custom key, and
reading that answer as authority over a value of a key Table 349 does define would widen what they
said. The three now say `unreadable = "discard"` beside it; `keep-everything` and
`refuse-any-loss` say nothing and stop. Each dropped value is a row of the report's
`moved_information` with no property and a fate naming the answer.

**A `/ModDate` the clause keeps beside a `/PieceInfo` is never dropped.** It stays as its producer
wrote it and is read only where the packet states `xmp:ModifyDate` too, because only then does
§14.3.4's equivalence bind; a date naming no instant there is refused by its own sentence.

## 3. A dictionary stating nothing goes

An empty `/Info`, or one a reference to a nonexistent object names (§7.3.9: "[a]n indirect object
reference (see 7.3.10, "Indirect objects") to a nonexistent object shall be treated the same as a
null object"), was refused with the sentence for a construction nobody asked for — though the
requirement had failed. It moves nothing into the packet, and the trailer's key goes.

## What it does not do

The three fuzzed documents this lets through at part 4 (two whose root page node's `/Count` is a
stream, one with no `/Kids`) convert into files veraPDF cannot parse, exactly as they already do at
part 2; that is `optimize::refuse_a_document_only_recovery_reads`'s population, not this site's, and
is reported rather than built here.
