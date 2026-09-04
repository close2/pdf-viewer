# 0896 — A `partial` whose debt is a permission, and three rows that carried one

Session 928. Status: **accepted**. A coverage round, reading the ledger's `partial` rows against
the code by `doc/todo/01`'s practice: `git blame` over `ledger.toml`, ordered by the commit that
last wrote each `note = ` line, oldest first.

## Context

The band read was the top of that list — five rows sharing one commit, rank 646 of a 1389-commit
base, written between sessions 489 and 525 and untouched since. Three of the five turned out to
share a defect that neither `doc/todo/01`'s sweeps nor the conformance gate can see, because it is
not a missing thing and not a wrong citation: **the row's status promises a debt and the row's own
note names a permission.**

`ledger.toml`'s header defines `partial` as "some [normative requirements] are [executed]; the note
says which are not". A clause entry the standard offers with *may* or *can* is not a normative
requirement on a reader, so a row `partial` for one says a requirement is unexecuted where the
standard states none. `doc/todo/01` already names the neighbouring shape — a `partial` whose note
argues it `implemented` — and this is that shape with the argument left implicit: the note is
correct about the tree and about the clause, and only the *status* is a claim nobody made.

`CLAUDE.md` decides which way it goes, in the sentence it uses about flatness and smoothness: "a
clause that permits is a clause that has been read, and it is a stronger answer than one that does
not apply". A permission read and declined is a result. A permission wearing a `partial` is a debt
the project will keep trying to pay.

## Decision

Three rows moved, each on its own reading of the clause, and each records the permission it
declines so the decision can be revisited by argument rather than rediscovered.

### §12.11.5 requirement handlers — `partial` → `out-of-scope`

The note argued outright that nothing is owed: "This program runs no ECMAScript (principle 5), so
there is nothing to disable and the `/RH` entry is read by nobody." Every requirement in the clause
binds a processor that *invokes* a handler. Table 276 admits exactly two handler types, `JS` and
`NoOp`, and `NoOp` is not a program — "A value of NoOp allows older PDF processors to ignore
unrecognised requirements." `/Script` names a document-level ECMAScript action and its `shall` is
on stopping it. The closing rule, "If an alternative requirement handler dictionary has an S entry
with an unrecognised type, it shall be ignored", is addressed to a reader that has read `/RH` in
order to invoke something.

So the clause is `CLAUDE.md` principle 5's script exclusion whole, and the row names it.
`out-of-scope` rather than `implemented`: the requirements are *vacated* rather than executed, and
that is the line §12.11.1's row keeps on the other side — Table 273's `/RH` sits among entries this
tree does read, so there the same reading leaves one `shall` met by construction. Both rows now
name each other, and §12.11's parent, which quoted the wording that moved.

**The row's cited test asserted a different clause.** `a_requirement_states_a_type_a_version_and_a_penalty`
builds a `/Requirements` array and asserts Table 273's `/S`, `/V` and `/Penalty` defaults — §12.11.1's
subject, and not one word of §12.11.5. It went with the status.

### §14.9.2.2 language identifiers, and §14.9.2 above it — `partial` → `implemented`

Both were `partial` for one entry: Table 122's `/Lang` on a CIDFont's font descriptor. Table 122
states it as "A name specifying the language of the font, which may be used for encodings where the
language is not implied by the encoding itself", and §14.9.2.2's own sentence about it is a
statement of fact — "Font Descriptors for CIDFonts can have a Lang key". There is no `shall`.

The permission is declined as a choice: the entry would be evidence for `substitute::Request`,
which is derived from the document alone, and what it would change is which of *this machine's*
faces stands in for a non-embedded CIDFont — an answer no gate in this tree can hold, since it
depends on what is installed. Table 122's next sentence is why declining costs nothing that could
otherwise be had: "If this entry is absent, such absence provides no information as to the language
of the document", so the entry is evidence about a *font* and never about the document's language,
which is what §14.9.2.3's hierarchy answers and what this tree hands a screen reader.

Every `shall` the clause does state is executed, and each was checked rather than assumed:

- the identifier is "either be the empty text string, to indicate that the language is unknown, or
  a Language-Tag as defined in BCP 47" — RFC 5646 section 2.1's own production, which
  `structure::well_formed_language_tag` judges. The *registry* judgement the note called the other
  half is not part of the production the standard names;
- an empty identifier records nothing and leaves the enclosing statement in force;
- "all language tags shall be treated as case-insensitive" holds of every comparison in the
  function, the grandfathered list and the `x-` singleton included.

§14.9.2 is an aggregate and follows its four children: three `implemented`, and §14.9.2.4
`out-of-scope` on clause 13's exclusion.

## Consequences

- Two rows left `partial` and one left the owing statuses altogether. The direction is towards
  fewer open debts, which is the direction a status inflation would also take, so each row carries
  the clause's own modal verb in its note — the thing a later round can check in one grep.
- **A sweep cannot find the rest of this population**, and that is the reason for the ADR rather
  than the three corrections. Every sweep in `doc/todo/01` reads a row that owes something and asks
  whether the owed thing exists; this defect is a row that owes nothing and says so. The
  discriminator is the clause's modal verb, and the fourteenth sweep (`--bin owed`) is the closest
  instrument — it asks whether the tree has the *thing* a note names, not whether the standard
  *asks* for it.
- The general form, and it is where the next one will be: **a note whose debt sentence contains
  "may", "can" or "is permitted" is a status claim resting on a permission**. That is a grep, and a
  round with minutes to spare can run it over the remaining `partial` rows.
