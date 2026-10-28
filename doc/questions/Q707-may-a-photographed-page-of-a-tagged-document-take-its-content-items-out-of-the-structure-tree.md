# Q707 — May a page photographed under `as-if-printed` of a tagged document take its content items out of the structure tree?

Asked by round 1841, the second archive slot of batch 125 (ADR 2516), under your answer A694
item 1 ("q694 1) yes allowed"). A694 is silent on two conditions ADR 2394 put on the photograph,
and ADR 2516 keeps both until you say.

**The condition.** A page that is photographed has its content stream replaced by one image, so the
marked content a structure tree (ISO 32000-2 section 14.7) points at is no longer on it. ADR 2394
refuses a document that states a structure tree, and a document that states optional content
(section 8.11) because a photograph of the configuration this viewer shows discards the others. At the
last census under `as-if-printed`, with the wide gate built, the structure-tree condition holds
back seven documents at 2b and the same seven at part 4 (one of pdf.js's, six of the other
corpora); the optional-content condition holds back none.

**The question.** May the converter, under `as-if-printed` and at 2b and every part 4 flavour only,
write the document without the structure tree's leaves that name a photographed page (the tree
otherwise kept), under a loss of its own word, say `tagging-of-photographed-pages`, recorded per page
in the report and in `xmpMM:History`? The alternative is dropping the whole tree (a larger loss
for the pages that keep their text), or keeping the refusal.

*Recommendation*: **yes, the leaves of the photographed pages only**, since the loss is then exactly
the pages the photograph already turned into pixels; Level A keeps the refusal, because that level
exists to keep the tagging. Optional content stays refused until a document asks: none does, and your A694 item 3 (bake in the
state the viewer shows) may be read as favouring yes the day one does, under a loss that names the discarded
configurations.

Nothing above is built; answer in `doc/questions/A707-...` (the same name with `A`).
