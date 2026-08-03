# ADR 0159 — A usage rights signature a save outgrows

Status: accepted, 2026-08-03. Session 198. The `should` `doc/todo/01-ledger-partial-rows.md`
named as owed and did not close.

## The clause, and why it is this tree's

§12.8.2.3 describes the `UR` transform method, and opens by saying the whole feature "[is]
deprecated with PDF 2.0". Two sentences in it are about a *processor*, and they are addressed to
two different programs:

> The transform parameters dictionary (see "Table 258 -Entries in the UR transform parameters
> dictionary") specifies the additional rights that shall be enabled if the signature is valid.

That one needs a processor with features behind a gate. This one has none — every operation it
can perform, it performs on every document — and the ledger has said so, correctly, since the
row was written.

> A PDF processor that modifies a PDF, with a UR signature in excess of the rights that are
> granted by that signature, should remove that signature prior to writing the newly modified
> PDF.

That one needs a processor that *writes*, and this became one in the hundred-and-thirty-sixth
session. It is the same shape as §12.8.2.2.1's parenthesis (session 191) and §7.6.3.2's random
initialisation vector (ADR 0129): a clause that was somebody else's until the program acquired a
verb, with nothing announcing the moment it changed hands.

## What is read

`signature::UsageRights` is Table 258 — `/Document`, `/Annots`, `/Form`, `/Signature`, `/EF` as
name lists, plus `/V` and `/P` — found through the `/Reference` chain, which is `modification`'s
walk one transform method over. `UsageRights::grants` answers for the verbs this program has,
and the order of its three steps is the argument:

1. **`/P` first.** "If false , any possible restriction may be ignored", and the table makes
   `false` the default. A document that has not asked for its restrictions to be honoured has
   granted everything, and the arrays are not reached.
2. **`/V` next.** "If an unknown version is present, no rights shall be enabled" — a rule about
   the rights, so it comes after the rule that says the rights need not be consulted.
3. **Then the array**, per right.

`Right` has three arms — `FillInForm`, `ImportFormData`, `FullSave` — and deliberately not one
per name in Table 258. A right no operation of ours can exceed is a right there is nothing to
check against, and an enum arm for it would claim otherwise.

`FullSave` also reads Table 258's implicit rule, in the narrow form the table itself states for
this case: "If the PDF document contains a UR3 dictionary, only rights specified by the Annots
entry that permit the document to be modified shall implicitly enable the FullSave right."

## What is written, and what is not

`ViewState::save` rewrites the permissions dictionary without its `/UR3` when the save would
exceed the grant. **The signature object stays in the file**, because `CLAUDE.md` permits
§7.5.6's incremental update and nothing else — the producer's bytes always stay. What is removed
is the entry that makes the object a grant at all: §12.8.6 defines a usage rights signature as
the one "referred to from the UR3 entry in the permissions dictionary", so a `/Perms` without
that key refers to none. That is the same construction §7.5.6 already uses for a deletion, and
the same one ADR 0100 records for a free cross-reference entry.

`/Perms` is rewritten where the catalog states it indirectly and the catalog itself where it does
not, which is `interactive_form`'s distinction for `/AcroForm`. Unlike there it cannot fail: the
catalog always has an object number.

And `notes.rs` says so **when the document opens**, not when it is saved. A signature
disappearing from a file is not a thing to do without warning, and the warning is only useful
while a person can still decide not to.

## Measured: the condition has no members

Four of the 974 corpus documents carry a `/UR3`, and every one of them grants what this program
does:

```text
160F-2019.pdf          FillIn true  FullSave true  /P false  /V 2.2 true
issue6127.pdf          FillIn true  FullSave true  /P false  /V 2.2 true
prefilled_f1040.pdf    FillIn true  FullSave true  /P false  /V 2.2 true
xfa_filled_imm1344e.pdf FillIn true FullSave true  /P false  /V 2.2 true
```

All four come out `/P false` — two state it, two leave it to the default — so the arrays are not
even reached, and all four state `/Form [/FillIn …]` and `/Document [/FullSave]` besides. **No
file this corpus holds can trip the rule**, which is trap 11's discipline reaching the opposite
conclusion from usual: derive the condition from the clause, count what it matches, and write
the count down. `no_corpus_documents_usage_rights_are_exceeded_by_what_this_program_does` holds
that at zero, so a document arriving which *does* trip it announces itself.

The code is therefore exercised by a fixture rather than by the corpus:
`a_save_beyond_the_granted_usage_rights_withdraws_the_signature` builds two files differing in
one name — `/Form [/FillIn]` against `/Form [/Import]` — so the same edit is inside the grant in
one and outside it in the other. Both state `/P true`, because a fixture at the table's default
would pass whatever the code did. Confirmed to fail when the withdrawal is removed.

## What this does not close

§12.8.2.2.2's comparison — "examine the current version of the document to see whether there
have been modifications to any objects that are not permitted by the transform parameters" —
needs the digest and two revisions, and is not done. The row stays `partial` for that, having
been `reported`.

## Alternatives rejected

- **Free the signature object as well.** §7.5.6 permits it and ADR 0100 already reads a free
  entry as a deletion, so it would work. It is more than the clause asks: the clause says remove
  the signature, and what makes those bytes a signature is the reference. Deleting an object a
  form field may also point at, on a `should`, is a larger claim about somebody else's file than
  the sentence supports.
- **Refuse the save instead.** The clause says *remove that signature*, not *refuse*. A person
  who fills in a form is entitled to their file.
- **Leave it, since no corpus document trips it.** That is the corpus-as-specification failure
  `CLAUDE.md` principle 5 forbids by name, and the ledger already recorded the debt.
