# 0691 — The filter set that became this program's, and the head a mention took off the list

Status: accepted.
Context: the successor selection rule's seventh use, its third run with the fourth step in place,
and the first time the *instrument's own record* moved a ranking.

## The rule, unchanged

ADR 0627's rule with ADR 0637's repair to step 2, ADR 0653's tie-break and ADR 0671's fourth step:

> Rank each ledger row by the errata annotations that fall on it whose issue number this tree names
> nowhere. Rank once over the live rows and once over **every** row, take the head of the two, and
> prefer the settled row where they tie. Reassemble the issue from every clause `emit` files it
> under, and read the issue whole.

Of the 307 issue numbers in `doc/ISO_32000-2_sponsored_EC3.pdf` that carry a strike or a caret, 118
were named nowhere in this tree at this round's base. **That figure is two short and the shortfall
is the round's second finding**, below.

## The head

Over live rows: **§7.6.4.1 and §7.6.6, six annotations apiece.** Over every row: **§7.4.1, eight
annotations under Issue #216 and Issue #527, `implemented`** — the row 750 measured from outside at
eight before the fourth step existed and 760 named as second, which is the calibration this rule now
takes before it trusts its own arithmetic. Both issues fall under one `emit` heading, so nothing had
to be reassembled; the check costs one grep and is not skipped because four of the six previous uses
needed it.

The full ranking has out-ranked the live one on every run of the fourth step, and this is the third.

## What the head said

`doc/errata-read.md` has both errata with the rectangle that places each. In outline:

- **Issue #216**, three annotations, `Review/Accepted`. Two are a producer's: *are* becomes *shall
  be* in "which decoding filter or filters to use are specified in the stream dictionary", and a new
  sentence is inserted after it — *All stream data shall follow the appropriate format(s) as
  described below.* The third is not. It strikes *files* from "PDF files support a standard set of
  filters that fall into two main categories" and writes *processors shall*, which turns an
  inventory of what documents contain into an obligation on whatever reads one.
- **Issue #527**, two annotations, `Review/Completed`, both on the clause's EXAMPLE 3. The base-85
  stream the standard prints is missing §7.4.3's `~>` end-of-data marker; one caret adds it and the
  other turns `/Length 447` into 449. Two bytes of marker, two of length: the halves check each
  other, and neither reaches this tree, which quotes the example's *arrangement* and never its
  bytes.

## The decision

**Table 6 is a closed set this program owes, and it is now asserted as one.**

`crates/pdf-syntax/tests/filters.rs::every_filter_table_6_names_is_supported_under_both_of_its_spellings`
walks Table 6's ten filter names and §8.9.7 Table 92's seven abbreviations — seventeen spellings,
because `filter::decode_reported` admits the inline form beside the full one — and asks of each
whether it decodes here or is an image codec `is_image_codec` hands to the image pipeline, and
whether two spellings of one filter answer alike on the same bytes.

**Why it did not exist before.** Every filter in Table 6 has a test of its own *output*; not one of
them asks whether the table is covered. A name dropped from `decode_reported`'s match arms or from
`is_image_codec` therefore becomes `FilterRefusal::Unsupported` — which is also what a name from no
table gets — with every other test in the crate green. Before Issue #216 the clause gave no reason
to close the set: it described files. Afterwards the set is a requirement, and a requirement with no
gate is what ADR 0671 predicted a settled row's erratum would find.

**Calibrated per trap 13, against two plants the rest of the crate cannot see**: `JPXDecode` removed
from `is_image_codec`, and `A85` removed from `ASCII85Decode`'s arm. Under each, `cargo test -p
pdf-syntax` fails on this test alone — 104 unit tests and the other seven integration tests stay
green — and passes again when the plant is restored.

**And §7.4's row could not add up.** It described Table 6's ten as "[f]our … stream filters
implemented here, one … a pass-through … and four … image codecs", which is nine, while the same
note has said since ADR 0587 that "all five of Table 6's byte-to-byte filters can be windowed".
`filter.rs` decodes five. Corrected. `--bin counts` is not at fault: a cardinal is a claim about a
family there only where it governs one of the ledger's own words for a row, and *stream filters* is
not one of them.

## The second decision: a mention is not a use, and step 2 cannot tell them apart

**The live ranking's head did not move because a round read it. It moved because a sentence
mentioned it.**

ADR 0660 recorded that an erratum read only far enough to break a tie must be written *bare*, so
that step 2's prefixed grep leaves it in the population. 760 obeyed that rule in its ADR and then
recorded the fix in its history file — in a sentence that writes both numbers with the `Issue #`
prefix, inside backticks, in order to say that they should not be written that way. The grep reads a
mention exactly as it reads a use. So two issue numbers left the population with no verdict, and
§14.8.5.3 — the live head for four consecutive uses — vanished from the live ranking.

Measured rather than argued: with the two restored, this round's script prints 760's own figures
back, 120 named nowhere and §14.8.5.3 at the live head with seven; without them the live head is
§7.6.4.1 and §7.6.6 with six. Nothing else in the tree names either number.

**The repair is a rule about writing, because no third grep can distinguish the two.** A bare-number
search over the tree is already ruled out — `doc/HAYRO_ISSUES.md` lists another project's issues
under the same numbers — and excluding `doc/history/` from the population grep would be worse, since
a history file's mention of an erratum it *read* is a true record. So:

> **A sentence about the form of an issue number must not contain one.** Write "with the `Issue #`
> prefix" and say how many; never say which.

The two numbers are left in the population, unrecorded, in the form both greps already read. They
will be at the head of the live ranking again the moment a round looks at it, which is where they
belong: no round has yet given either a verdict.

**This is the eighth blindness on the rule's list and the second that belongs to the instrument
rather than to an erratum**, the first being ADR 0681's — that an erratum's *added* text cannot be a
rustdoc blockquote. Both were found by a round obeying the rule that the previous round wrote.

## Consequences

- §7.4.1 keeps `implemented` and gains the requirement Issue #216 puts on it, with the test that
  asserts it. §7.4's row loses a wrong number.
- Issue #216 and Issue #527 leave the population; it stands at 116 after this round.
- The rule gains a ninth line under *how to write about it*, and the writing rule now has two halves
  that pull the same way: an erratum read to a verdict goes in `doc/errata-read.md` with its number,
  and an erratum named for any other reason at all — including to say that it should not have been
  named — is not named.
- No pixel moves and no behaviour moves. What the round adds is evidence for a requirement that was
  already met, and what it removes are two false sentences.
