# 3353 — A text object ends at its `ET` or at the `BT` that begins the next, and `ET` discards its matrices

Session 2163. Status: accepted. Context: audit 2161's findings B01 and B04; ADRs 0421, 2759.

## The clauses

Table 105, `BT`: "Begin a text object, initializing the text matrix, Tm, and the text line matrix, Tlm,
to the identity matrix. Text objects shall not be nested; a second BT shall not appear before an ET."
`ET`: "End a text object, discarding the text matrix." §9.4.1: Tm and Tlm "may be specified only within
a text object and shall not persist from one text object to the next". §9.3.6: "Glyphs shall accumulate
until the text object is ended by an ET operator". §9.4.2 and §9.4.3: each family "shall only appear
within text objects", and Figure 9 admits neither in the page-description state.

## What was wrong

A `BT` inside an open text object replaced the whole `TextObject`, so the outlines a clipping mode had
accumulated never became a clip and the §11.7.4.4 and §9.3.8 groups of the glyphs already shown were
dropped, in silence. `ET` kept Tm and Tlm, so `BT 50 50 Td ET (A) Tj` drew at 50, 50 by a matrix the
clause says was discarded; and a positioning or showing operator outside a text object said nothing.

## Decision

**A `BT` inside an open text object ends that object as `ET` would, and then begins its own**: the clip,
the knockout groups and the readings the open object owes are realised by `end_text_object`, and the page
reports `StreamSyntaxFault::TextObjectNested`. The clause states no reading of the fault, so the choice is
argued from the sentences that are there. Of the two readings — end the open object, or ignore the
second `BT` and keep it open — only the first lets `BT` do what Table 105 says it does: initialise both
matrices to the identity. Ignoring it would place every following `Td` relative to the old object's
matrix, a position the file never stated. And ending the object is what §9.3.6 needs of the glyphs
already shown, whose clip "shall" be made at the object's end. A stream that ends inside a text object
is still the one exception, reported as `TextObjectLeftOpen`: no later operator reads a clip it never
reaches.

**`ET` discards Tm and Tlm**: the `TextObject` is reset to its default, whose matrices are the identity
`BT` would set, so nothing of one text object reaches the next or the stream between them.

**A positioning or showing operator outside a text object is reported and acts on those identity
matrices** (`TextOperatorOutsideATextObject`, one sentence per operator). Whether it draws is a
robustness choice, written down here: it draws, because a stream that never wrote `BT` at all — the
commonest shape of this fault — then still places its text where its own `Td` says, which is all the
file states; what it can never be is placed by the matrix `ET` discarded. A stray `ET` is reported and
acts on nothing.

## Cost

Two new `StreamSyntaxFault` variants are public, so the wide archive gate (ADR 2618) treats them as
the stream's own faults. A valid page writes none of them, so no valid page moves.
