# 2485 — The two shapes veraPDF fails and the clause's text does not name stay refused until the owner rules which governs

Session 1825, an "archive" round of batch one hundred and twenty-three, 2026-10-27.
Status: proposed, with `doc/questions/Q691` for the one decision the owner makes: whether
veraPDF's reading governs shipped output where it conflicts with the clause's text. Both shapes
stay refused meanwhile.
Context: ADR 2484 (the construction these two are cut out of), ADR 0922 (the rule for an
ambiguous requirement, and that in a conflict the specification wins unless the owner says
otherwise), ADR 2470 (the writer whose output they concern).

## 1. What was planted, and what failed

Two planted outputs of ADR 2484's route were rejected by veraPDF 1.30.3 at ISO 19005-2 section
6.2.8.3, a subclause that, like ISO 19005-4 section 6.2.7.3, states its channel-count and
depth requirements of the **colour channels**:

1. **Opacity at another depth than the colour channels** (colour 20 bits, opacity 18). A JP2 file
   whose channels differ in depth states each in a `bpcc` box (ISO/IEC 15444-1 I.5.3.2). veraPDF
   fails the file at test 5 of 6.2.8.3, the same-depth test, which it applies to every channel of
   the file.
2. **A grey image with an opacity channel** (colour 22, opacity 22, so two channels). veraPDF fails
   test 1, the one, three or four channels test, which it applies to every channel in the file.

Three colour channels with an opacity channel of the colour depth (four channels, no `bpcc` box)
passes, as does every colour-only file; 12 of 12 planted outputs of the other shapes passed at both
targets.

## 2. The two readings, and why this is a conflict and not an ambiguity

The clause's text speaks of colour channels. veraPDF counts every channel of the file. ADR 0922
says a disagreement with veraPDF is never by itself evidence that a clause is ambiguous, and that
where the two conflict the specification wins unless the owner says otherwise. These two shapes are
therefore a conflict between the clause's text and the reference validator, and the exception to the
specification winning is the owner's to make. This ADR does not infer room in the clause from the
validator's behaviour, and does not adopt veraPDF's reading on its own authority.

## 3. The decision taken meanwhile

`DEEP_OPACITY_AT_ANOTHER_DEPTH` and `DEEP_GREY_WITH_OPACITY` keep the image refused, named, with a
sentence that says what the reference validator does. This is the safe side, not a verdict on the
clause: a file this tree wrote that veraPDF fails would be useless to a depositor whose archive runs
veraPDF, and a refused image is reported rather than silently altered.

**Cost.** A deep image of either shape stays refused instead of converted. No corpus document is
affected (ADR 2484 section 4 counts none deeper than sixteen bits).

## 4. The question, and what each answer builds

Q691 asks whether veraPDF's reading governs shipped output for these shapes.
- If the clause's text governs, the two refusals come off, the images are written, and
  `pdf-archive` (which already accepts both files) is right.
- If veraPDF governs, the refusals stay with this ADR as their reason, and `pdf-archive` should learn
  the two tests so the tree's own checker and the reference validator agree.

## 5. Found, not fixed

`pdf-archive`, the tree's own conformance checker, accepts both planted files that veraPDF rejects.
It follows the clause's colour channels, so the two tools disagree on this pair of files, and
`cross_check` cannot see it because no corpus document has either shape.
