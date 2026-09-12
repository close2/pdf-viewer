# 1007 — A signature is reported before it goes, and the question is the conversion's own

Status: accepted. Session 986.
Context: `crates/pdf-transform/src/archive/{signatures,decision,prepare,rewrite,report,mod}.rs`,
`crates/pdf-transform/tests/archive.rs`, `doc/pdf-a-conversion-limits.md` section 3.6,
`doc/pdf-a-mitigations.md` section 2's permissions entry, ADRs 1003 and 1006; ISO 32000-2
§12.7.3 (Table 225), §12.7.5.5, §12.8.1, §12.8.2.3, §12.8.6 (Table 263); ISO 32000-1:2008,
12.7.4.5 and 12.8.1; ISO 19005-2 sections 6.1.12, 6.3.3, 6.4.3 and Annex B.1; ISO 19005-4
section 6.1.11.

## 1. What was owed

ADR 1006 found that this verb had re-serialised every signed document it converted, carrying each
signature dictionary into bytes it no longer covered — a signature that lies — for as long as the
verb existed, while `doc/pdf-a-conversion-limits.md` section 3.6 had said all along that a
converted document carries no signature, that the loss is *Ask, loudly*, and that the report names
each signature, its signer and whether it validates. The merge made a conforming source the
identity conversion; a non-conforming signed source was refused by the output's own verdict with
`signatures/digest-covers-the-whole-file — the conversion broke it`, which was honest and said
nothing a person could act on. This round builds section 3.6.

## 2. The question is the conversion's, not a requirement's

The decision table answers *failed requirements*. Two rows name a signature —
`file-structure/document-signature-states-no-digest` and
`signatures/digest-covers-the-whole-file` — and both bind PDF/A-2 alone. ISO 19005-4 has no
Annex B.1 and its section 6.1.11 admits `DocMDP`, so **a signed source converted to a part 4
target fails no row about its signature**, and a converter that removed signatures only where a
row asked would have gone on writing the lying kind under part 4 — where nothing would ever have
caught it, because the output's verdict has no row to catch it with either. That was the state of
the tree before this round for every part 4 conversion of a signed, non-conforming source.

So the question is put by the conversion itself: **whenever a source that will be rewritten
carries a signature**, `Conversion::signatures` holds a decision of the same three kinds a
`Loses` row gets — `Authorised`, `Unauthorised` or `Refused` — and stops the conversion the same
way. It is a field beside `decided` rather than a synthetic row in it, because `Decided.requirement`
is a `pdf_archive` identifier and this is not one. The two part 2 rows are answered by the same
rewrite, `Rewrite::SignatureValueRemoved`, under the same loss, `Loss::SignatureAssertion`, so a
document failing them and a document failing nothing about its signature get one answer.

A conforming source never reaches this: ADR 1006's identity conversion keeps every signature it
has, and `Prepared` walks and hashes nothing for it.

## 3. Each part of the rewrite, decided by a sentence

- **The field's `/V` goes.** §12.7.5.5: "the field value ( V ), if present, shall be a signature
  dictionary containing the signature". The dictionary is the assertion.
- **The field, its widget and its `/AP` stay.** §12.7.5.5 NOTE 2 says signing updated "at least
  the V entry and usually also the AP entry"; only the first is undone, because the same clause
  makes the appearance "strictly for the purpose of providing a way for a human verifier to
  perform their own verification of the visual representation" and forbids a processor to
  "incorporate the validation status of a signature … into the appearance" — so an appearance
  kept after its signature is gone asserts nothing false — and because ISO 19005-2 section 6.3.3
  requires an annotation to carry one. The signature widgets in the corpus's fail-cases have no
  `/AP` and a zero-size `/Rect`; the fixture in `tests/archive.rs` has both and keeps both.
- **`/Perms`'s `DocMDP` and `UR3` entries go.** Table 263 makes each one a signature: a processor
  "shall enforce the permissions specified by the P entry … and shall also validate the
  corresponding signature", and `UR3`'s rights are granted "[i]f the signature is valid". Neither
  rests on anything once the signature is gone, and §12.8.2.3 says of `UR3` outright that a
  processor which modifies the document beyond the rights "should remove that signature prior to
  writing the newly modified PDF". `CLAUDE.md` principle 3 is honoured in the direction it points:
  the restriction `DocMDP` enforced is *stated in the report* — "whose DocMDP entry had every
  processor permit form filling and signing and nothing else" — rather than silently dropped.
- **The permissions dictionary stays, empty.** Table 263 makes every entry optional; an empty one
  states nothing, and removing the key would be a change no requirement asked for.
- **Table 225's `AppendOnly` bit is cleared.** "If set, the document contains signatures that may
  be invalidated if the PDF file is saved (written) in a way that alters its previous contents":
  the output contains none. `SignaturesExist` stays, because the *fields* do.
- **`/Lock` and `/SV` stay** (Table 235): they constrain the next signing rather than describe the
  last. **`/DSS` and `/Legal` stay** (§12.8.4.3, §12.8.7): certificates, revocation data and
  feature counts name no byte of this file and are as true after the rewrite as before.
- **A key of `/Perms` outside Table 263 is removed mechanically**, not under the loss. §12.8.6:
  "[e]ach entry in this dictionary … shall specify the name of a permission handler", and Table
  263 lists "the currently defined entries" — two. A handler the standard does not define is one
  no conforming processor of either edition can consult, so nothing any reader did ever turned on
  it, and removing it changes what no reader could see. Both corpus fail-cases for the row carry
  `/XX (value)` — a string, not even a dictionary — and no signature; the catalogue had lumped the
  row in with the signature rows and the census reported one `signatures` entry where the row's
  subject is not a signature.

## 4. The report line, over the source

Section 3.6 asks for each signature's signer and "whether it currently validates". The line is
computed over the *source's* bytes before anything moves — the only file the signature was ever a
statement about — and says: where it was reached (field, widget, `DocMDP`, `UR3`), its `/Name`,
its `/M`, its `/Reason`, what its `DocMDP` permitted, what its `/ByteRange` covered of the source,
`Signature::integrity`'s answer and `Signature::authenticity`'s. Nothing in it says *valid*, for
the reason `pdf_model::signature` gives.

What the corpus's three certification signatures say under it is worth recording, because it is
the case ADR 1003 wanted stated rather than dropped: **"the bytes it covers no longer hash to the
SHA256 digest it records, so the source was modified after it was signed; it does verify under the
1024-bit RSA (PKCS #1 v1.5) key in a certificate the source itself carries, but what it signs is
the digest the source no longer produces."** veraPDF built its `t02-fail-*` files by editing a
signed `pass` file, and the report says so without anybody having to know that.

## 5. The proof

The converted output is walked the same three ways the source was — the form's field tree, every
page's `/Annots`, and `/Perms` — and refused by name if a signature remains. Session 971's rule,
applied to a removal: a report that says the assertion is gone may not sit beside an output that
still states it. The preparation records an obstacle rather than a plan where a signed field or
widget is written directly into its array, because the rewrite edits objects; that document is
refused with the sentence, and its signatures are still named in the report.

## 6. The eight documents

Before: the three `6-1-12-t02-fail-*` files and both `t01-fail-a` files (2b and 4) were refused
with `SIGNATURE_STRUCTURE_NOT_BUILT`; the three `pass` files were copied. After: the two
`t01-fail-a` files convert with nothing authorised (`Mechanical`, `/XX` removed); the three
`t02-fail-*` files are refused by default with the line above and convert under
`--authorise signature-assertion`, each output held to PDF/A-2b again and conforming; the three
`pass` files are copied as before. veraPDF, run as evidence and not read, passes all five outputs
(`-f 2b` for four, `-f 4` for one).

## 7. What is not built

`doc/pdf-a-mitigations.md`'s `xmpMM:History` entry, appended page and attached source for a
signature are `doc/rfc/0007`'s configuration and wait on it; the catalogue entry now says so. The
identity path does not list the signatures it keeps.

## 8. A trap, and a habit

**A loss the rewrite itself causes has no requirement to be keyed by, and a target without a row
hides it entirely.** Every loss this converter knew before this round was the answer to a failed
requirement; the decision table is a function of the requirement's identifier, and the census
counts what the table answers. A signature's loss is caused by *serialising*, which every
conversion does, so no identifier names it — and under part 4 no row would ever have failed to
show that the file written was wrong. The place to look for the next one of these is any
property of the source that is a function of its bytes rather than of its objects: a signature,
a `/ID` derived from content, a linearisation hint table.

**The report reads the source, never the output.** The output's signature would say the range
covers nothing and the digest fails, every time, for every file — which is true and useless. The
report's job is to say what the source asserted, so that a person who has only the output and the
report can still know it.
