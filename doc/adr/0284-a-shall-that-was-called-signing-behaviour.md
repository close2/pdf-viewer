# 0284 — A `shall` that was filed under "signing behaviour"

**Status.** Accepted.
**Context.** `doc/todo/01`'s blame-ordered reading list, run over the `partial` rows whose notes
nothing has touched in five hundred commits.

## The row that turned into work

§12.7.5.5's row said:

> Table 235's `/Lock` and `/SV` are signing behaviour, and validating a signature is §12.8's.

Half of that is right. Table 235's `/SV` — whose value is Table 237's seed value dictionary — constrains a processor that *signs* (this line gave the entry to 237, corrected by session 489's sweep), and this one does
not. `/Lock` is not the same kind of thing, and the clause says so in prose rather than in the
table — which is exactly why it was read as the table's:

> The signature field lock dictionary … contains the names of form fields whose values **shall** no
> longer be changed after this signature has been signed.

Table 236's own column says the fields "should be locked"; the sentence under it is a `shall`, and
it is addressed to whoever changes a value. **This program changes values.** It fills fields in,
it saves with §7.5.6's incremental update, and it did both on a field a signature had locked
without a word — which is the `silent` shape this project hunts, standing in a row that had
explained it away.

## What was built

`signature::field_locks` reads every `/Lock` a **signed** signature field states, and
`FieldLock::locks` answers Table 236's three actions over §12.7.4.2's fully qualified names:
`All`, `Include`, `Exclude`. `restriction::asserted` gains it as a fourth `Restriction`, beside
§7.6.4.2's Table 22 and §12.8.2.2's `/DocMDP`, and `viewer_core` words it for a person and refuses
the edit — under the reader's own policy, which `CLAUDE.md` says must always be switchable off
(ADR 0212's shape, unchanged).

Four decisions in it are the clause's rather than convenience:

- **The condition is the signature, not the entry.** "[A]fter this signature has been *signed*",
  so the lock is read off a `/V` that is a signature dictionary and an unsigned signature field
  carrying a `/Lock` locks nothing. §12.7.5.5's NOTE 1 says what such a field is — it "can also
  hold information needed later when the actual signing takes place" — so before there is a
  signature the entry is an instruction to the signer. Deriving the condition from the clause is
  trap 11's rule, and here it is the difference between a form that can be filled in and one that
  cannot.
- **The name is the fully qualified one.** Table 236 says only "[a]n array of text strings
  containing field names", and §12.7.4.2's is the only name in the clause that identifies a field
  uniquely. A partial name repeats across the tree, and locking every `Total` in a document
  because one was named would refuse edits the file never asked to refuse.
- **An action the table does not define locks nothing.** Falling back to `All` would close a
  document on a word the standard does not use — the same rule §12.8.2.2's row already applies to
  a `/P` outside 1..=3.
- **The walk is not gated on Table 225's `/SigFlags`**, which `signature::signatures` is. That
  flag exists so a processor can skip work; skipping it here would let a document that
  under-describes itself escape a restriction it wrote down. A missed signature costs a report, a
  missed lock costs a `shall`.

**And Table 236's `/P` is deliberately not read.** It reads like Table 257's, and its own sentence
puts it somewhere else: "absence of this key shall result in no effect on signature **validation
rules**". That is what invalidates the signature rather than what a reader may do — §12.8.2.2's
question, and that row's debt. The entry that makes §12.8.2.2's equivalent binding *on a
processor* is §12.8.6's permissions dictionary, and Table 236 names no such route.

## The witness is hand-built, and that is the finding under the finding

Not one of the 974 corpus documents states a `/Lock`: six carry a signature and none of the six
carries one. So no gate in this project could ever have found this, and no gate will notice if it
breaks — trap 8's exact shape. `forms_data.rs::a_signed_signature_field_locks_the_fields_its_lock_names`
is the whole defence: one fixture, five cases, one of them the unsigned condition.

## Four other rows the same reading corrected

Read off the same list, each a shape `doc/todo/01` already names:

- **§7.6** blamed a revision-4 password outside ASCII on "PDFDocEncoding bytes … Annex D data this
  crate does not hold". `pdf_syntax::text_string` has held the whole of Table D.3 in both
  directions for hundreds of commits, and `crypt.rs`'s own module comment says so and dates it.
  The true limit is narrower by every character in Table D.3 outside ASCII: a character the
  encoding has no code for has no bytes to hash. **Expired blocker.**
- **§7.7** said "what it does not read is everything the catalog holds for a *viewer* rather than a
  renderer" — while its own child §7.7.2 lists eighteen of twenty-five such entries as read.
  **A parent not maintained by the sessions that implemented its members.**
- **§14.6** said "[w]hat is *not* read is any tag's meaning", three sentences after saying that
  §8.11.3.3's optional content rides on `BDC`. Four tags are read by name and acted on — `/OC`,
  `/Artifact`, `/ReversedChars`, `/AF`. **A note that contradicts itself**, corrected once by
  appending and never re-read whole.
- **§14.6.1** carried the same sentence, and **§14.8.2.6.1** said the exception "except where an
  associated Alt or ActualText entry applies" was not read — both are read, from the property list
  of the `BDC` that opened the section and from the structure element where it states none. That
  row is `implemented` now: every requirement in §14.8.2.6.1 is addressed to a *document*, and
  what a reader owes it is §9.10.2's methods and the exception's two entries.

## What this says about the reading list

`doc/todo/01`'s blame order works — it put every one of these at the top — and it has one flaw
this round can name. **Seventeen rows the previous run read and *kept* are still at the top of the
list**, because keeping a row edits nothing and `git blame` cannot see a reading that changed
nothing. The list therefore re-offers them for ever, and the rows that have genuinely never been
read sit below them.

The remedy is not to stamp a row. It is that **a row read and kept records the evidence that kept
it** — the grep that was run, the entry that was checked, the clause sentence that still binds —
which is content rather than bookkeeping, and moves the blame pointer as a by-product. That is
what the previous run did for three of its seventeen and not for the other fourteen.
