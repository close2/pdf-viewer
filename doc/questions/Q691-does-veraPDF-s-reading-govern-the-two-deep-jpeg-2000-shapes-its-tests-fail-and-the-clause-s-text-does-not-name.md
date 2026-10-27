# Q691 — Does veraPDF's reading govern shipped output for the two deep JPEG 2000 shapes its tests fail and the clause's text does not name?

Asked by round 1825, the archive slot of batch 123 (ADR 2485), under ADR 0922 (in a conflict the
specification wins unless you say otherwise).

ADR 2484 writes a JPEG 2000 image with a component deeper than sixteen bits back as a JP2 file.
ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 state the channel count and the common
depth of the **colour channels**. veraPDF 1.30.3 applies both tests to **every** channel of the file,
and so fails two shapes: (1) an opacity channel of another depth than the colour channels (the file
needs a `bpcc` box; test 5), and (2) a grey image with an opacity channel (two channels; test 1). The
tree's own checker, `pdf-archive`, follows the clause's words and accepts both. Neither shape occurs
in any corpus document today.

Both shapes stay refused, by name, until you answer.

*Recommendation*: **veraPDF's reading governs shipped output for these two shapes, and the refusals
stay**, with `pdf-archive` taught the two tests. The words of the clause are one way and the
reference validator another; a depositor's archive runs the validator, the cost is a refusal no
document meets today, and your answer reverses it in two lines if you rule that the clause's text
governs. The alternative, **the clause's text governs**, takes the refusals off and writes both
shapes.
