# Q717 — May a page photographed under `as-if-printed`, in a document that states optional content, bake in the configuration this viewer shows?

Asked by round 1851, the archive slot of batch 126, after building your answer A707 (ADR 2516 item 7).

**What changed.** Q707's census counted the documents the optional-content condition holds back as
none; that count was taken behind the structure-tree condition, which is checked first. With A707
built, round 1851 converted the seven documents the structure tree held under `as-if-printed`: two
now convert at 2b and at 4 (veraPDF passes all four outputs), and **three of the other five are now
held by the optional-content condition** (`050734.pdf`, `064034.pdf`, `504076.pdf`, govdocs1's error
set, each stating one or two optional content groups); one is held because the reader does not
draw the page whole and one by a font row that is not the photograph's.

**The condition.** ADR 2394 refuses to photograph a page of a document that states optional
content (ISO 32000-2 section 8.11), because a photograph of the configuration this viewer shows
discards the others. A print discards them too: it carries the configuration the printing reader
showed, or the one §8.11.4.4's `/Print` usage asks for.

**The question.** May the converter, under `as-if-printed` and at 2b and every part 4 flavour only,
photograph such a page as this viewer shows it, under a loss of its own word, say
`optional-content-of-photographed-pages`, naming per page in the report and in `xmpMM:History` the
optional content groups whose other states the photograph discarded? The rest of the document
keeps its optional content.

*Recommendation*: **yes, the photographed pages only, drawn in the configuration a print would
carry**, which your A694 item 3 (bake in the state the viewer shows) already reads as favouring;
Level A and Level U are offered no photograph, so they keep the refusal. The alternative is keeping
the refusal for these three documents.

Nothing above is built; answer in `doc/questions/A717-...` (the same name with `A`).
