# 0919 — Twelve rows whose clause states its requirement in the indicative, read by hand

Session 939. Status: **accepted**. The bucket `--bin permitted` is right about and cannot judge:
five rows moved, seven kept, and the one that reached code was a requirement written as a
statement of fact.

## Context

ADR 0900 built the twenty-fourth sweep and ADR 0907 found its second blindness: **the standard
sometimes states a requirement in the indicative**, so the sweep files the row under *no modal
verb* and is right about the words and wrong about the debt. Session 933 named the twelve rows in
that bucket and was explicit that they must be read by hand and never moved on the flag. This
session read all twelve.

The bucket turns out to hold three quite different things, and the flag cannot tell them apart:

1. **A framing subclause that states no requirement of anybody** — and whose `partial` is therefore
   a promise the clause never made. Four of the twelve.
2. **A row whose flagged quotation is an argument it *wins*** — a true sentence the row stands on,
   beside a real debt stated in prose with no quotation to flag. Six of the twelve.
3. **A requirement genuinely stated in the indicative**, which is ADR 0907's finding and which the
   flag makes look like the first two. Two of the twelve, and both reached code.

## Decision

### Five moved to `implemented`

| row | the sentence the sweep flagged | why it moved |
|---|---|---|
| **§12.10.1** | "PDF is a common delivery mechanism for map and satellite imagery data" | two sentences, no requirement, no entry of its own. The projection it owed is §12.10.2's and §12.10's, both `partial` and both naming it |
| **§12.8.3.4.1** | "[t]he PDF signatures using the SubFilter value ETSI.CAdES.detached are referred to as PAdES signatures" | three sentences, no modal verb, no entry. PAdES's rules are stated in the standard's own terms by §12.8.3.4.2–.8, whose eight rows carry them |
| **Annex O** | "selecting the first matching word in the document" | all eleven parameters carried out (`tools/state.sh annex-o`), and the debt its note deferred to — §O.2.1 and §O.2.2 — had been paid underneath it |
| **§14.9** | "a complete (or whole) word or phrase substitution for the current element" | the row had outlived every debt it named; the flagged sentence is *implemented* and was ADR 0214's own correction |
| **§14.13.2** | "[b]oth types are allowed for associated files but the embedded form is recommended" | ADR 0918: the debt was real, was fixed, and was not the one the row stated |

The first two are §12.8.4.1's move of six sessions earlier, twice. **The instrument's own second
column called both of them**: `--bin permitted` prints the clause's `shall` sentences outside its
tables beside every hit, and both showed **0**. ADR 0897's rule — "a hit over a clause with none is
a status with nothing under it" — held for both, and they are the only two of the twelve for
which it printed zero.

**Annex O and §14.9 were found by a different question**, and it is the cheapest one in this file:
*which rows are `partial` while every descendant is settled?* Over 875 rows there are exactly two,
and they were these. Both had a note whose every named debt was discharged, which is the fourteenth
sweep's population, and both were flagged here for a sentence that was never the debt. Two
instruments agreeing on two rows — this bucket and the arithmetic — is what made them safe to move
without new code.

### Seven kept, and their stated reasons corrected

Six remain flagged, honestly: §8.7.4.5.8, §12.7.8.3.1, §12.8, §12.8.3, §12.8.3.4 and §14.8. Each
owes something real and larger than a round — a tessellation derived from §10.7.3; `/EmbeddedFDFs`;
trust and revocation, three times over; §14.8.5's derived rectangles — and in each the flagged
sentence is a *true* sentence the row stands on. §12.8.4 was the seventh: kept, and it left the
bucket, because its corrected note now quotes the `shall` it actually owes ("the DSS dictionary
shall be used to collect the certificates, CRLs and OCSP responses that are relevant to validate
that signature", §12.8.4.2, addressed to a validator) instead of a bulleted list of what a DSS may
hold.

**Two stale sentences were found in the keeping**, both the same shape as the moves:

- **§14.8** named "§14.8.2.2's and §14.8.6's own remainders" among what the family still owes, and
  both rows are `implemented` — §14.8.6's since session 861, which turned its one file-addressed
  `shall` into a report. A parent owing a debt its children had discharged, in a note whose own
  third sentence is about that.
- **§8.7.4.5.8**'s debt was true and unfalsifiable as written. It is `mesh::PATCH_STEPS`, the
  constant 10: every patch of every type 6 and type 7 shading is evaluated on an 11×11 grid
  whatever its size on the page, so the departure is one number and finding it is a grep.

### The one requirement genuinely in the indicative, and it was cheap

§12.7.8.3.1's Table 245 says of an FDF file's `/Version`:

> If the header specifies a later version, or if this entry is absent, the document conforms to the
> version specified in the header.

Session 933 named this as an entry "read into `FormsData::version` as a name and never *ranked*,
which is the entry's whole meaning". Ranking it cost a line: `Document::header_version` already
matched `%FDF-`, `Version::parse` is now public so that both clause families read a version by one
rule, and `FormsData::conforms_to` is the later of the two. `FormsData::version` still returns the
entry as the file spells it, because the entry and the meaning are two answers rather than one.

Calibrated by planting the reading it replaces — the entry taken as the answer — under which
`the_version_an_fdf_conforms_to_is_the_later_of_its_header_and_its_entry` fails on the
*earlier*-entry case and the nine other `forms_data` tests pass.

**Neither field reaches a consumer**, and the row says so: nothing outside the module's own tests reads `version` or `conforms_to`, which was already true of the entry before it was ranked. That is the fifth sweep's shape and is written down rather than left to be found — what changed is that the rule is now implemented and asserted, so the day something displays an FDF's provenance it gets the right number instead of the stated one.

`/EmbeddedFDFs` is what §12.7.8.3.1's `partial` now stands for, alone.

## Consequences

`partial` 209 → 204, `implemented` 460 → 465; the *no modal verb* bucket 12 → 6, and every one of
the six has been read. **The sweep should learn nothing from this**, and that is the finding worth
recording: its rank was correct about the words in all twelve cases, its second column correctly
printed zero `shall` sentences for exactly the two clauses that state none, and the two rows whose
debt it could not see were both found by a different instrument in the same file. A discriminator
that is right about what it measures and silent about what it does not is working; the failure
would have been believing it.

**What is owed after this** is the bucket nothing has read: 30 rows quoting nothing `doc/md/`
holds, which is the largest of `--bin permitted`'s five and the only one whose hits say nothing
about the standard at all — they say the quotation could not be located, which is three different
defects wearing one flag (a misquotation, a quotation of a document other than ISO 32000-2, and a
conversion artefact).
