# 0950 — The edition a profile names decides which text judges it

Session 950. Status: **accepted**. Four ICC specifications arrived complete — ICC.1:1998-09,
ICC.1:2001-12, ICC.1:2022 and ICC.2:2023 — and four things in `crates/pdf-archive` had been blocked
on exactly them. `doc/third-party-data.md` records what each text is and how it is treated; this
records what was decided with them. The last two arrived first and section 4 was written against
them alone; the first two arrived mid-round and changed its answer, which is why section 6 exists.

## 1. Where the MD5 lives, and why it is not on `Profile`

ISO 19005-4 section 6.2.4.2 makes two `ICCBased` colour spaces the same profile when they reach one
embedded stream by indirect reference, **or** when their MD5 values are equal — each read from that
profile's `Profile ID` field where it is present and not zero, and otherwise calculated by
ISO 15076-1:2010 section 7.2.18's method. The tree already had MD5 (`md-5`, in this graph for
Table 45's `/CheckSum` and §7.6's algorithms, reached through `cms::Digest`), so no dependency was
added and `doc/stack.md` was not opened for one.

What it did not have was anywhere to put the question. `pdf-archive`'s manifest says it adds no
reader of its own, and `pdf_model::icc` is the ICC reader — but the natural-looking home,
`icc::Profile`, is the wrong one. `Profile::parse` answers *what colour is this* and refuses every
profile it cannot convert with: a data space that is not one of three, a transform in an encoding it
does not read. The questions a conformance check asks are the other kind — which specification does
this profile claim, is the identifier it states the one its own bytes give, which tags does it carry
— and **a profile that fails to parse still has answers to all three**. So they are free functions
over the bytes in `icc`, beside `Identification`, which was already that second entry point:
`stated_id`, `computed_id`, `profile_id` and `has_tag`. `crate::jpeg2000` splits the same subject
the same way (ADR 0925).

**The cost on the viewer's open path is nil, and by construction rather than by luck.** Nothing on
the drawing path calls any of the four; `Profile::parse` is unchanged but for a literal `1024`
becoming the named `MAX_TAGS` it shares with the new walk. Measured all the same, since the rule is
to measure rather than to argue: `crates/pdf-model/examples/open_one` on an `ICCBased`-CMYK document
(`doc/pdf.js/test/pdfs/bug886717.pdf`) runs 8.1–11.4 ms with the change against 8.7–12.1 ms without
it, one spread inside the other. The digest is paid only where a verdict asks for it, and
`table::graphics::Profiles` memoises it per profile object for the reason it already memoised the
decode — a half-megabyte press profile compared against three selections would otherwise be hashed
three times.

## 2. The method is read from two texts, neither of which is the one the clause names

ISO 19005-4 cites ISO 15076-1:2010 section 7.2.18, and this project holds that document only as a
front-matter preview. Taking the method from ICC.1:2022's section 7.2.18 instead is an
identification and not a substitution, and the argument is worth keeping because it is the shape
principle 5 asks for:

- ICC.1:2022's foreword states that it is an update to ICC.1:2010, that ICC.1:2010 and
  ISO 15076-1:2010 are technically identical, and lists the technical changes it makes — none of
  which touches the profile header or this calculation.
- The ISO 15076-1:2010 preview's own foreword agrees from the other side: technically identical to
  ICC.1:2010, profile version 4.3.0.0.
- ICC.2:2023 section 7.2.20 states the same method, sentence for sentence, for iccMAX.
- ISO 19005-4 itself supplies the clause number, and the preview's contents list puts clause 7 where
  that number needs it to be.

Two held texts agreeing on a clause a third names is evidence about the reading, which is the only
direction of inference `CLAUDE.md` principle 5 allows.

**And the clause conditions neither route on the profile's own version**, which is what the corpus's
`6-2-4-2-t03-fail-e` turns on: two copies of one **v2.1** press profile, identical but for the
sixteen bytes of the `Profile ID` field, one stating the identifier and the other stating zero.
Reading the field alone answers *different*; computing alone answers *same*; the clause's order —
stated, else computed — answers *same* for both, and the pair is one profile. Miss closed.

## 3. ICC.2 decides `6-2-3-t01-fail-d`, and against the file

The witness is an Adobe RGB (1998) profile whose version field has been set to 5.0.0.0. ICC.2
section 7.2.6 states that 5.0.0.0 is the version consistent with iccMAX, and ISO 32000-2 §8.6.5.5
closes by requiring a profile to conform to the specification version its own header names — so
ICC.2 is the text that judges it. It fails twice over:

- **Section 8.4.** A display profile shall carry one or more of `AToB0Tag` to `AToB3Tag` or
  `DToB0Tag` to `DToB3Tag`, and one or more of `BToA0Tag` to `BToA3Tag` or `BToD0Tag` to `BToD3Tag`.
  This profile carries none of the sixteen — only the matrix-column and TRC tags of the v2 form,
  which iccMAX does not define at all.
- **Section 7.2.20.** Its `Profile ID` field is not zero and is not the value the method gives.

Both are now reported, so the miss is closed under both parts. **The interesting part is that this
was decidable at all**: the previous reason called the file undecidable because nothing held forbade
a version-5 profile, and it was right — what was missing was not a prohibition but the text the
profile itself points at.

## 4. Three splits, because a version number identifies a claim and not a conformance

`doc/questions/A20` forbids weakening an `Unchecked` reason to make a row look better. The way to
report what is now readable without doing that is the split this crate has used twice before
(session 941's identification-amendment row, session 946's five-way serialisation split), and it was
used three times here, and section 6 adds a fourth `Implemented` row to the second of them:

| kept `Unchecked` | added `Implemented` |
|---|---|
| `…/destination-profile-conforms-to-an-icc-edition` | `…-states-a-correct-profile-id` |
| — | `…/destination-profile-carries-the-tags-its-class-requires` |
| `…/icc-profiles-conform-to-a-permitted-edition` (part 2) | `…/icc-profiles-claim-a-permitted-edition` |
| `…/icc-profiles-conform-to-the-version-they-name` (part 4, new) | `…/icc-profiles-carry-the-tags-their-version-requires` |

The last pair is new: part 4's section 6.2.4.2 defers to §8.6.5.5 whole, that clause's version
sentence was deliberately excluded from `graphics/icc-profiles-conform-to-the-base-standard`'s doc
comment, and it had no row of its own — so a sentence nobody checked was also a sentence nobody
reported. It now has both halves. **The part 2 and part 4 rows disagree on purpose about a profile
stating 2.1.0**: part 2 asks whether it conforms to one of four documents, which the version does
not decide, and part 4 asks what its own edition requires, which for 2.1.0 is a text not held.

**`IccEdition` matches exactly, not "or later".** Each of the four held texts says which number is
consistent with *it* — 2.2.0, 4.0.0, 4.4.0.0, 5.0.0.0 — and nothing held says what a 2.1.0, a 4.1.0
or a 5.1 profile would owe, so those fall to the unchecked remainder rather than being judged by a
neighbouring text.

**And 4.4.0.0 is *not* reported by the part 2 claim row while 5.x is** — asymmetric on
purpose.
ISO 19005-2's reference to ISO 15076-1 is *undated*, and its clause 2 says an undated reference
takes the latest edition; whether an edition of ISO 15076-1 based on ICC.1:2022 exists is a fact
about ISO's catalogue rather than about any document here, so a 4.4.0.0 profile is left alone.
Version 5
needs no such fact: ICC.2 section 1 places iccMAX *beside* ISO 15076-1 — a document based on it that
expands the profile specification, removing some of its types and adding others — so a profile
conforming to ICC.2 conforms to none of part 2's four texts whatever edition the undated reference
reaches.

## 5. What the version numbering settled, and it was worth establishing

The row above rests on a premise a round could have assumed: that a profile's version number
identifies which *edition* it claims. Both held texts state it directly — the major and minor
versions are set by the ICC, and each edition states the number consistent with itself — and
ICC.1:2022's foreword walks the chain (revision 4.2 → ISO 15076-1:2005; 4.3.0.0 → ISO 15076-1:2010 =
ICC.1:2010; 4.4 → this edition). ISO 19005-2's own NOTE 1 makes the same identification from the
other side, calling ISO 15076-1 technically identical to the earlier texts in every respect relevant
here *other than the value of the profile version number*.

What that does **not** establish is the mapping from part 2's other three designations —
ICC.1:1998-09, ICC.1:2001-12, ICC.1:2003-09 — to version numbers. Those texts are not here, and no
held document states their versions. So the permitted-edition row keeps its `Unchecked` status and
its reason names what is missing rather than what was found.

## 6. What two more editions changed, and the shipped profile they settled

ICC.1:1998-09 and ICC.1:2001-12 arrived after sections 1 to 5 were written, and they are *two of the
four texts ISO 19005-2 section 6.2.4.2 actually names*. That is a different kind of holding from the
two above, and it moved the row that section 4 had just left `Unchecked`.

**A version check alone was the wrong answer for part 2, and the reason is in the sentence.** Part
2's clause is a disjunction over four *documents*: a profile fails it only by conforming to none of
them. It does not ask a profile to state any particular version number, and neither held text asks
that either — ICC.1:1998-09 section 6.1.3 and ICC.1:2001-12 section 6.1.3 each state the number
consistent with themselves and require nothing of a profile's field. So the new row,
`graphics/icc-profiles-carry-the-tags-a-permitted-edition-requires`, judges **every** `ICCBased`
profile against both texts' clause 6.3 required-tag lists whatever version it states, and reports
only a profile that satisfies neither. The version-claim row stays for the one case a version alone
decides: iccMAX is a different document, not a later edition of ISO 15076-1.

**And the question worth the round: `data/icc/sRGB2014.icc` is admissible.** The profile this tree
ships as the converter's default destination profile states version 2.0.0, and ICC.1:1998-09 — the
earliest text part 2 names — states its own number as 2.2.0. The answer is that it conforms, on
four things the held text says and one it does not:

- **It does not require a profile to carry its number.** Section 6.1.3 describes the field and says
  "the current version number is 2.2.0"; that is a statement about the specification.
- **Nothing required was added inside major version 2.** Section 6.1.3's own note makes a major
  version change the one that carries incompatible changes, new required tags being its example, and
  a minor change the one that carries compatible ones. Annex F then lists what the 2.1.0 and 2.2.0
  revisions actually changed: new optional tags, clarifications, and one corrected signature.
- **It carries what Table 27 requires** of an RGB display profile — `desc`, the three colorant tags,
  the three TRC tags, `wtpt`, `cprt` — and a test in `table::graphics` now asserts exactly that
  against the shipped bytes, under ICC.1:2001-12's Table 25 as well.
- **Its `Profile ID` offends nothing.** ICC.1:1998-09's Table 9 gives bytes 84 to 127 to "44 bytes
  reserved for future expansion" and, unlike every other reserved field in that document, states no
  requirement that they be zero.
- **Its `chad` tag is not a defect either.** ICC.1:1998-09 does not define `chromaticAdaptationTag`
  — it arrives with ICC.1:2001-12 — but that text's clause 6.2 admits tags beyond the ones it
  defines, requiring only that each signature be registered with the ICC, and `chad` is an ICC
  signature.

So `data/icc/PROVENANCE.md` stands and the converter's default is right for all six targets. The
one thing that is *not* settled is the same thing as everywhere else in this ADR: conformance in
every respect, which would take reading each text whole.

**One correction this brought, and it matters beyond the row.** ICC.1:2001-12 section 6.1.13 states
the `Profile ID` at the same bytes as the later texts and a **different** calculation — it zeroes
the rendering intent, the *device attributes* and the field itself, where ICC.1:2022 and ICC.2:2023
zero the profile *flags* in the attributes' place — and it does not state the method at all, but
points at a technical note on the ICC's web site. So the method is not edition-invariant.
`pdf_model::icc::computed_id` is the later one, which is the one ISO 19005-4 names, and
`IccEdition::profile_id_clause` returns `None` for ICC.1:2001-12 so that no profile claiming 4.0.0
is judged by a calculation its own edition does not describe. Had the two texts arrived in the other
order, the round would have shipped that mistake.

## What is still not readable

- **What a profile stating 2.0.0, 2.1.0, 2.3.0 or 2.4.0 has to satisfy as its *own* edition** —
  2.1.0 above all, which is most of the profiles that exist. Those revisions of the v2 text are not
  held. Under part 2 this costs nothing, because that clause asks about documents rather than
  labels; under part 4 and §8.6.5.5 it is the whole question.
- **What a profile stating 4.1.0 to 4.3.0 has to satisfy.** ICC.1:2003-09 is supplied by the ICC on
  request only, and ISO 15076-1:2010 is a front-matter preview that stops before clause 7;
  ICC.1:2022's list of technical changes is evidence about section 7.2.18 specifically, not a
  warrant for judging a 4.3 profile against 4.4's whole text.
- **Everything of the four held editions beyond their required-tag lists and the `Profile ID`** —
  the tag types, the encodings, the rest of the header. The unchecked rows say so.
- **ICC.2 section 8.2's `spectralWhitePointTag`**, required where the header's spectral PCS field is
  not zero. That one is readable and deliberately left: it needs bytes 100 to 103 of the header,
  which `pdf_model::icc` does not yet report.
