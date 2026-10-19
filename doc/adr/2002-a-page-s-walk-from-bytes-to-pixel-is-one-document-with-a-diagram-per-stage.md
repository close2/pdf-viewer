# 2002 — A page's walk from bytes to pixel is one document, with a diagram per stage

Session 1583, slot 7 of batch ninety-two, a docs round. Status: **accepted** and **built** in
`doc/teaching/how-a-page-lands-on-the-screen.md`. Context: the project owner, 2026-10-19: "improve
code quality. Do not fix any bugs. … The focus in this round should be the 'project which we can
show students' part. This includes documentation files, which explain (probably with diagrams) how
pdf instructions land on the screen", and `CLAUDE.md` principle 4. Clauses: ISO 32000-2 §7.5.4,
§7.7.3, §7.8.2, §8.5.3, §10.7, §11.4.

## 1. The gap

`doc/crate-map.md` is one row per crate; `doc/state-of-play.md` is dense and says what is built;
`doc/PLAN.md` is a plan. Each answers *what is here*. None answers the question a student asks first:
*what happens, in order, to one instruction?* Following `re f` from a file's bytes to a pixel crosses
nine crates and three rasterisers, and the order is in no document.

## 2. The decision

**One walk, seven stages, one diagram each and one for the whole**, in the order a page takes:
`pdf-syntax` (bytes to objects), `pdf-model`'s page tree, `pdf-model`'s content interpreter with
`pdf-colour` and `pdf-font`, `pdf-render`'s list and trait, the three rasterisers (`render-cpu` the
oracle, `render-raster` over `raster/` the window's, `render-gpu` the comparison), compositing, and the
window. The unit is a single path fill, because a rectangle exercises every stage and a glyph is the
same stage with a different source of outline; the document says where text, an image and a group
leave the path. **Every box names the crate, the module and, where one argued it, the ADR, and every
stage names the clause it answers to.** Sentences quoted from the standard are verbatim (`--bin
quotations`).

Diagrams are **Mermaid**, in fenced blocks: they render in the forge's page view, they are text a
diff shows, and they need no tool in the build. The convention is set here because no document under
`doc/` had one; a later diagram follows it — a label of `crate/path.rs`, ADR numbers as `ADR NNNN`,
and a **Maintenance note** after it.

## 3. What was left out, deliberately

Annotations and form appearances, optional content, shadings, signatures, the writer and scripts are
named at the end as what the walk does not follow, each pointing at its `doc/crate-map.md` row. A walk
that tried to hold them all would be the crate map again.

## 4. Cost

A document describing code goes stale. ADR 2003 is the answer: what is held by a test, what is held
by a note, and what is left to the round that moves the code.
