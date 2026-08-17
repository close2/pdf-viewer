# ADR 0403 — The second entry of a plural array, and the grep that classed a corpus as binary

Status: accepted.
Session: the five-hundred-and-sixty-eighth, a general improvement round.

## 1. What this decides

Three things, and the second is the one that would have been found without writing any code.

1. **§12.8.2.4's FieldMDP transform is read**, and it becomes a restriction of its own rather than a
   second name for §12.7.5.5's signature field lock, because the two clauses state different things
   about the same list of fields.
2. **A `/Reference` array is plural and is read to its end.** The corpus's one certification
   signature carries two signature reference dictionaries, and this tree read the first for its whole
   life.
3. **`grep` over a corpus of PDFs needs `-a`**, and a ledger row that had been `reported` on the
   strength of a grep without it was making a claim about grep.

## 2. What the clause asks, and what it does not

§12.8.2.4 is short. Its first sentence is the requirement:

> The FieldMDP transform method shall be used to detect changes to the values of a list of form
> fields.

Table 259 states the list as an `/Action` — `All`, `Include` or `Exclude` — and a `/Fields` array of
text strings, and the clause's last sentence sends validation elsewhere: "FieldMDP signatures shall
be validated in a similar manner to DocMDP signatures", which is §12.8.2.2.2's comparison of the
signed revision against the current one. That comparison is not done here and §12.8.2.2's own row has
been `partial` for it since the round that recomputed the byte range digest; this row is now `partial`
for the same reason and no other.

What *is* doable without the signed revision is everything before it: which fields a signature says
it covers, and telling a person before they change one.

## 3. Why it is not the field lock under another name

Table 259's vocabulary is Table 236's, exactly, and the standard says so:

> The Action and Fields entries in the transform parameters dictionary shall be copied from the
> corresponding fields in the signature field lock dictionary.

So one type reads both — `signature::FieldSelection`, which was `FieldLock` and is renamed for
carrying two tables — and a second enum with the same three variants would have claimed a
distinction the vocabulary does not have.

**The distinction that is real is in what each clause says about the fields it names**, and it is
the reason `Restriction::FieldCovered` is not `Restriction::FieldLocked`:

| | says | shape |
|---|---|---|
| §12.7.5.5 | the values "shall no longer be changed after this signature has been signed" | a prohibition on a reader |
| §12.8.2.4 | "any modifications to specific form fields shall invalidate that recipient's signature" | a consequence of a change |

A person deciding whether to fill in a field is owed which of the two they are being told. One says
the document forbids the edit; the other says the edit costs a signature, and a reader who would
accept the second may well refuse the first. `CLAUDE.md`'s four levels — `off`, `on`, ask, warn — are
answered differently by each, which is exactly the argument `crates/pdf-model/src/restriction.rs`
exists to make: this crate hands back reasons and a host decides.

Where a document states both, both are returned. That is §12.8.6's composition rule doing its job —
"[f]or a permission to be actually granted for a document, it shall be allowed by each permission
handler that is present" — rather than a duplicate.

## 4. The `/Reference` array is plural, and this tree read one entry of it

`xfa_filled_imm1344e.pdf` has been named in this project's documents for hundreds of rounds as the
corpus's one certification signature: `/Perms /DocMDP`, Table 257's `/P 2`, 2.5 MB of filled-in form
appended after it. Its signature dictionary states a `/Reference` array of **two** signature reference
dictionaries — a `DocMDP` and a `FieldMDP` — and every reader this tree had written for that array
returned on the first match it wanted:

- `modification` returns the moment it finds a `DocMDP`;
- `usage_rights` returns the moment it finds a `UR`;
- `has_transform` answers a boolean about one name.

None of those is wrong for what it asks. What was wrong is that nothing asked the array as a whole,
and §12.8.2.1's own framing is plural — "[t]ransform methods, along with transform parameters, shall
determine which objects are included and excluded in revision comparison". A signature can state more
than one transform because it can restrict more than one thing, and the file that proves it is the
one file this project had been citing about signatures all along.

`signature::field_mdp` therefore reads every entry of the array rather than the first that matches.

## 5. The measurement that said zero was a measurement of grep

The §12.8.2.4 row read `reported`, and its note said the transform's protection "is not absent from
the program … it is simply not this transform doing it" — resting on the belief that no corpus
document states one. §12.7.5.5's row said the same about `/Lock`, and `forms_data.rs`'s field-lock
test says in its own comment that the fixture "is the only witness there is".

For `/Lock` that is true. For FieldMDP it was not, and the instrument is the whole story:

```
$ grep -rl  FieldMDP doc/pdf.js/test/pdfs            # nothing
$ grep -arl FieldMDP doc/pdf.js/test/pdfs
doc/pdf.js/test/pdfs/xfa_filled_imm1344e.pdf
```

Same walk, same pattern, same `-l`. Without `-a`, grep classes these files as binary and the listing
never names them. A second witness from the same producer is
`doc/corpora/format-corpus/jhove-errors/PDF-HUL-114/MOZILLA-1671648-0.pdf`, and across every PDF
under the pdf.js corpus, the four submodules in `doc/corpora/` and this project's own fixtures those
two are the whole population — measured with this tree's own reader, which is what found them.

`doc/HANDOVER.md` trap 8 already says a census whose predicate is the code under test is not
independent of it. This is the other side of the same coin: the *independent* instrument was the one
that lied, and the code under test was right. The rule that survives is neither "trust the reader" nor
"trust the grep" but **run both and require them to agree**, which is what
`signatures.rs::the_corpus_states_the_fields_one_signature_covers` now pins.

`doc/habits.md`'s *Measuring* section carries the one-line form.

## 6. What the witness settled that no argument could

`FieldSelection::covers` matches §12.7.4.2's **fully qualified** name, and the reason written down
when §12.7.5.5 landed was an argument: a partial name repeats across a field tree, and locking every
`Total` in a document because one was named would refuse edits the file never asked to refuse. No
document was available to check it against.

This one checks it. Its transform names `form1[0].SignatureField3[0]`, and `pdf_model::form::fields`
independently derives exactly that string as the fully qualified name of the document's one field —
two readers, two clauses, one name. A partial-name reading would have matched here too, which is why
the argument still carries the general case; but the fully qualified reading is now the one a
producer's own file agrees with.

## 7. What this does not claim

- **Nothing is validated.** §12.8.2.2.2's comparison of the signed and current revisions is not done,
  for FieldMDP or for DocMDP, and both rows say so. `Signature::integrity` answers only whether the
  bytes the signature covers still hash to what it recorded.
- **Table 256's `/Data` is unread**, deliberately. It is "(Required when TransformMethod is
  FieldMDP)" and it names "the object in the document upon which the object modification analysis
  should be performed" — the scope of an analysis nothing here performs. Reading it would record a
  fact no code acts on.
- **The restriction withholds nothing this program currently does.** The covered field in both
  witnesses is a *signature* field, and `ViewState::set_field` fills in text and choice fields. It is
  asserted anyway, because `restriction::asserted` answers what the document says rather than what
  this program happens to be able to do; a host that grows a way to sign is owed the sentence without
  anybody remembering to add it.
