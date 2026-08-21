# 0472 — The witness a row named, and the report nothing asserted

Status: accepted.
Session: 641. Follows ADR 0469, which read rank 1 of this band and left the eleven rows below it
named in `doc/todo/01` together with a prediction about their shape. Follows ADR 0455 for the rule
that chose the reading — *rank by `git blame` over each `note =` line, then read the row whose
stated reason is a claim about this codebase rather than about the standard* — and ADR 0465 for
the reminder that a settled-looking half of the vocabulary is where nobody looks.

## The decision

**Three things, and none of them changes a status.**

1. **Two counted claims in the ledger get a command, and one of them was wrong.**
   `examples/signature_algorithm_census` counts §12.8.5's `/Type /DocTimeStamp` and §12.8.3.3.2's
   `adbe-revocationInfoArchival`, and prints the file that carries each. §12.8.3.3.2's "the
   corpus's one witness" is **three** documents; §12.8.5's "no corpus document carries a document
   timestamp" **holds**.
2. **Six signature rows point at evidence that reaches what they claim.** Five `reported` rows rest
   on a sentence `viewer_core::notes` writes and cited three `pdf-model` tests apiece; a sixth,
   §12.8.3.4.1, says "which is what a test asserts" and named no PAdES test. One new test is
   written — the revocation sentence had none at all — and five arrays are repointed.
3. **`requirements::Kind::unmet`'s signature arm becomes three arms**, because Table 275 words two
   of the three types as strict increments on the first and because the single sentence had
   expired: it said this program does not verify a signature, and it has since ADR 0229.

**And §12.11's parent row stops claiming a capability its own children deny.**

## How the rows were chosen

The blame band was re-derived on this base — 833 commits — and it is the eleven rows ADR 0469 left,
at ranks 513 to 534 with the same forty-two-commit gap above them. Nine of the eleven are §12.8's
signature rows, sharing one paragraph of boilerplate five times over; the other two are §12.7.6.2
and §12.11.

620's rule picked four to read, and the same rule explains why they were worth reading: each states
a reason that is a claim about *this tree*. §12.11's is "Read in full — …"; §12.8.3.3.2's is "named
to a person where it is present … `issue17069.pdf` is the corpus's one witness"; §12.8.5's is "no
corpus document carries a document timestamp"; the four revocation rows' is "which it says out loud
on every signed document it opens".

## Why a counted claim in a note is a claim without an instrument

`CLAUDE.md` states the rule for the instruction files: a fact that can be counted is not written
down, and what is written down is the command that counts it. The ledger has always been outside
that rule, and reasonably — a row's job is to record a claim, and `tools/state.sh` cannot print
"which documents carry an OCSP response".

What this round found is the cost of the exemption, and it is legible only because both claims were
re-derived at once. Two sentences of the same age, in neighbouring rows, in the same vocabulary:
one was right and one was wrong by a factor of three, and **nothing in the tree could tell them
apart**. A count with no instrument is not a weaker fact than a count with one; it is a different
kind of thing, because it cannot be checked and therefore cannot decay visibly.

The fix is not to delete the numbers. It is to make the row's own question answerable:

> A note stating a count over the corpus names the command that produces it, or the round that
> writes the count adds one.

The cost here was about twenty lines in a census that was already opening every document and
reading every `SignedData`. That is the general case rather than a lucky one: a row that states a
count over the corpus states it about a walk somebody has already written, because that is what
made the row's *other* half checkable.

## §12.8.3.3.2, and the three witnesses

§12.8.3.3.2 is what a program with no network can say about revocation, and the clause hands it the
whole answer by printing its own object identifier:

> adbe-revocationInfoArchival OBJECT IDENTIFIER::= {adbe(1.2.840.113583) acrobat(1) security(1) 8}

`cms::ADBE_REVOCATION_INFO_ARCHIVAL` is that constant and `notes::about_one` says the attribute is
there. The row named `issue17069.pdf` as the corpus's one witness, and the census finds
`issue6127.pdf` and `xfa_filled_imm1344e.pdf` beside it. The same sentence stood in `notes.rs`
beside the code, which is `doc/habits.md`'s "run the sweeps over the source, not only over the
ledger" — the ledger has a gate and the comment does not.

## The evidence gap, for the sixth round running

The five `reported` rows of §12.8.3.3.2, §12.8.3.4.4, .6, .7 and .8 all rest on the same thing: that
the third question is *asked out loud*. That sentence is `viewer-core`'s. Every test the five rows
cited is `pdf-model`'s. So a status whose entire content is "we report this" had, for its whole
life, no test in the tree that could fail if the report stopped.

Half of it turned out to be already covered, and that is worth separating from the half that was
not. `notes.rs::a_document_whose_signed_bytes_moved_says_so_and_claims_nothing_more` does assert
the three-questions paragraph, word for word, including "no certificate store and makes no network
request" — the four revocation rows were *right* and simply never pointed at it. §12.8.3.3.2 was
not: nothing anywhere asserted the revocation-material sentence, so
`a_signature_carrying_revocation_information_says_so_and_claims_no_more` is new, on `issue6127.pdf`
— the plainest of the three witnesses — with `bug854315.pdf`, signed and stating no signed
attributes at all, as the negative half. Mutation-checked: pointing the lookup at another object
identifier fails it.

§12.8.3.4.1 is the sixth and was found by reading the arrays rather than by the rule: its note ends
"which is what a test asserts alongside the one that finds them", and its array named the three
general tests. The test it means is one rather than two —
`a_pades_signature_is_held_to_the_rules_that_need_no_certificate` breaks three §12.8.3.4.2 rules
under `ETSI.CAdES.detached`, then breaks the same three under `adbe.pkcs7.detached` and asserts
silence — which is the scope sentence the whole subclause depends on.

## §12.11, and a parent that overstated

The fifth failure shape in `doc/todo/01` is a family's parent row going stale about its children,
and all four recorded instances have the parent *understating* — listing as unread things that were
read. §12.11 is the other direction: it listed "Table 276's handlers" among what it reads, and both
of its children say the opposite in as many words. §12.11.1: "[i]t is unread, and the requirement it
carries is met by construction". §12.11.5: "the `/RH` entry is read by nobody".

The direction matters because it decides which sweeps can see it. An understating parent names a
thing the tree has, so the fourteenth sweep prints it; an overstating parent names a thing the tree
*lacks*, which is the seventh sweep's discriminator — and the seventh sweep only reads
`inapplicable` rows. Nothing was looking.

What the children say is also the right answer, and it is worth not losing: §12.11.5's `/RH` names
an ECMAScript handler that shall be disabled, `CLAUDE.md` excludes ECMAScript, so every handler a
file could name is disabled here whatever the file says. The requirement is met by construction. The
parent's mistake was to call that *reading the table*.

## §12.11.2, and the fourth decay of a method that predicts its own decay

`Kind::unmet` answers, per Table 275 type, whether this program meets the requirement — and its own
doc comment says the answer "decays exactly as a ledger row does: a session that builds a layer
panel has to come back and change `OCInteract`". It has now done so four times. The fourth:

```
"no signature validation or signing: §12.8 is read and reported, and verifying a
 signature needs a certificate store"
```

one sentence answering `DigSigValidation`, `DigSig` and `DigSigMDP`, and wrong twice over.

**It had expired.** A signature's value *is* verified here, under the key in the certificate the
file itself carries, since ADR 0229 — `Authenticity::Verified` is that answer and every one of the
corpus's ten signatures reaches it. What is missing is not verification but *trust*.

**And it named the whole where the table words increments.** Table 275: "[i]n addition to the
validation requirements of DigSigValidation"; "[i]n addition to the requirements of DigSig and
DigSigValidation". One sentence cannot name three increments, and naming the increment is what
`unmet`'s own doc comment demands of a reason. Three arms now — the trust decision, signing, and
§12.8.2.2.2's object-by-object comparison — with
`the_three_signature_requirements_name_three_different_increments` as the gate on the shape, beside
the pair test `a_collection_is_met_and_editing_one_is_not` already is.

Both of the last two decays were found by reading the *method* against the tree rather than the
ledger row against the tree, and in both cases §12.11.2's row had already been corrected without
the code beside it. A ledger row is gated and a `&'static str` is not.

## What was confirmed rather than changed

- **§12.8.5 holds.** 974 documents, 964 opened, 9 carrying a signature dictionary, 10 dictionaries
  between them, **0** of them a `/Type /DocTimeStamp`. Table 255 is what makes keying on `/Type`
  safe rather than lucky: the entry is optional for a signature with a default of `Sig` and
  required for a timestamp, so a dictionary saying nothing is a signature and never a timestamp
  read by mistake. The fixture witness stands (trap 8).
- **`spec-errata emit` over all fourteen documents before writing.** §12.11's errata are Issue #187,
  which §12.11.1's row already records and which vindicates the code — Table 273's `/S` row sends a
  reader to Table 276 for its valid values and the amendment sends them to Table 275, which is what
  `Kind::read` has always matched — and Issue #656, an editorial column heading. Nothing touches
  §12.8.3.3.2, §12.8.3.4.x or §12.8.5.
- **A third false claim, in the source only.** `Authenticity::UnknownDigest`'s doc comment said
  three corpus signatures reach it, each stating a *signature* algorithm where a digest algorithm
  belongs. None does, and none could have when the sentence was written: reading `digestAlgorithm`
  by shape found the issuer's `SEQUENCE`, and that was fixed — with a test — two hundred sessions
  before the comment was added. An observation of a defect, recorded after the defect was gone.

## What this does not do

No status moves and no pixel moves. The four revocation rows stay `reported` because question 3
needs a trust store and a network, which is ADR 0215's and ADR 0229's standing position; §12.8.5
and §12.11 stay `partial` for the halves their notes already name. `/RH` stays unread, because the
requirement it carries is met by not running ECMAScript and reading the entry would buy nothing.
