# 0460 — The threshold a clause stated and three documents denied

Status: accepted.
Session: 626. Follows ADR 0455, whose rule for choosing a row off `doc/todo/01`'s blame ordering
this round applied: *rank by blame, then read the row whose stated reason is a claim about this
codebase rather than a claim about the standard.*

## The decision

**`pdf-model` computes §12.11.6's penalty value, and `viewer-core` says it out loud.** The
computation is new; the refusal §12.11.6 asks for is still declined, and the reason it is declined
has changed from a false one to a true one.

Three places in this tree said the standard states no threshold on a document's requirement
penalties — §12.11.3's ledger row, §12.11.6's ledger row, and `requirements.rs`'s module header.
All three were wrong, and the sentence they denied is the fourth paragraph of §12.11.3:

> In the situation where the penalty values are being used to evaluate the presentation of the
> base PDF document, and there exist no other alternates, if the penalty value exceeds 100 then
> the PDF processor should not attempt to display or process the document.

§12.11.6 does not state the arithmetic; it points at that clause for it:

> If requirements cannot be met, as determined by the computation of the penalty value as
> described in 12.11.3, "Requirement penalty values", then the processing of the document shall
> not continue.

So the computation the standard names is stated, once, in the clause §12.11.6 sends a reader to —
and this tree recorded, in the two rows either side of it and in the code, that it was not.

## What the computation is, and why it is a sum over the unmet ones

Neither half is an inference this round invented; each is a sentence.

**A sum rather than any single entry.** Table 273 bounds one entry: `/Penalty` is "[a]n integer
value that shall be between 0 and 100 (inclusive)". A threshold on a single entry could therefore
never fire, and the sentence above would be dead text. The paragraph immediately before it is what
the sum is:

> Values between 0 and 100 are available to weight the value of this feature among other features
> in the same document requirements array as well as when contributing to the total penalty points
> to weigh against other documents in the choosing process if alternatives are available.

That paragraph is about choosing *between* documents. The threshold sentence is the case it leaves
over — "there exist no other alternates" — where the total has nothing to be weighed against
except the number the clause names.

**Over the requirements that cannot be met**, because Table 273 says what a penalty is the penalty
*for*: "the penalty value to be applied when this requirement cannot be met by a PDF processor". A
requirement this program meets costs nothing, whatever the file priced it at. `requirements::unmet`
was already exactly that population; nothing but the arithmetic was missing.

## What the program does with it, and why that is not obedience

`requirements::penalty_total` returns the sum. `viewer_core::notes::about` says it, with the
clause number, when it exceeds `PENALTY_LIMIT`, and the document is drawn.

Three reasons, in order of how much they bind.

1. **The clause says `should`, not `shall`.** "[S]hould not attempt to display or process the
   document" is the only sentence in §12.11 that turns the number into an instruction, and
   declining a `should` costs conformance nothing. The `shall` in §12.11.6 — "the processing of
   the document shall not continue" — is conditioned on the computation, and what the computation
   *means* is stated only in the `should` sentence; a reader that treated the `shall` as
   unconditional would be inventing the missing half rather than reading it.
2. **`CLAUDE.md` principle 3's restriction shape.** This is a document asserting a restriction over
   the person reading it — the same family as Table 22's `/P` flags — and the principle is that the
   *policy* is asked once, in a place a host can supply, never hard-coded as a refusal at the point
   of the operation and never decided inside `pdf-model`. The number is computed in `pdf-model`;
   the decision is taken nowhere. Adding the four levels later — off, on, ask, warn — is a change
   in the host and nothing below it.
3. **The old reason, which survives on its own merits**: refusing to open a file a person asked for
   is a worse failure for a viewer than showing it with its limits named, and the clause's own next
   sentences make that safe — "[i]f the reader encounters an unsupported feature (whether or not
   that feature was declared as a requirement), it shall take the normal fallback actions", with
   NOTE 1's "there is no formal connection between the requirement type and the operation of the
   associated feature(s)".

**The departure is now from a stated `should` rather than from a silence.** That is the whole
change in standing, and it is the difference between a documented choice and an unexamined one.

## The clamp, which was the near miss

`Requirement::penalty` is a `u8` clamped to [0, 100]. Had the threshold been on a single entry,
that clamp would have erased the one datum the clause's rule needs, and the ledger row that
described the clamp would have been describing the defect. It is not, because the threshold is on a
total and Table 273 had already bounded the entry — so the clamp bounds a value the standard bounds
first. The doc comment now says so rather than leaving a reader to work out that the two facts do
not collide.

## Why nothing on the disk fails without this

**0 of the 974 corpus documents state a `/Requirements` array at all.** No corpus run could have
found this and none ever will; it is `CLAUDE.md`'s two-denominators paragraph exactly — a corpus
cannot rank a requirement no document exercises. The witnesses are built: a unit test in
`requirements.rs` for the arithmetic and a `viewer-core` headless test for the sentence, both
mutation-checked (sum → max fails the first; `>` → `>=` fails the second).

## What else the round found, which belongs here because it is the same shape

**§12.6.4.9's row read the clause's first sentence backwards.** It said the clause was "[a]djacent
to clause 13's exclusion and not covered by it, because §12.6.4.9 is in clause 12". §12.6.4.9 opens
with the sentence §12.6.4.10 opens with, word for word:

> The features described in this subclause are deprecated with PDF 2.0. They are superseded by the
> general multimedia framework described in 13.2, "Multimedia".

The two neighbouring rows had held opposite readings of one identical sentence since the ledger was
written, and the one that quoted it was right. Refusing a `Sound` action is a reading of §12.6.4.9,
not an exclusion stretched across a clause boundary — which matters, because principle 5's
exclusions are closed and stretching one is how a closed list stops being closed.

**And their three cited tests reached none of what the rows claimed** — ADR 0455's newest shape, the
row that is right while its evidence is not. `a_name_the_table_does_not_hold_is_not_an_action`
asserts that a name outside Table 201 produces *no* action, which is the one path that never calls
`action::refused`. A single end-to-end test now clicks a link on each of the three action types and
asserts the sentence reaches `Event::Reported`, which is the channel `viewer-ui`'s `dispatch.rs`
prints from — and the rows' `code` array pointed at `pdf-viewer.rs`, which contains no such code.
