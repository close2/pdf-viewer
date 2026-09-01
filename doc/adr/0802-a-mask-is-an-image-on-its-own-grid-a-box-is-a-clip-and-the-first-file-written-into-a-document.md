# 0802 — A mask is an image on its own grid, a box asked for is a clip, and the first file written into a document

Session 870. Status: **accepted**. The third decision record of RFC 0002's implementation, on
the long-lived branch `round-867`.

## Context

`doc/todo/57`, as session 868 rewrote it, put two `images`/`render` flags first because they
need no writer, and held `attachments --attach` behind RFC 0002 §13's first question — whether
`CLAUDE.md`'s authoring exclusion is redrawn for a serializer. This round was asked to take all
three, on the reading RFC §6.6 already gives: `--attach` is "the one verb in the suite §7.5.6's
*incremental* writer could serve today", and §7.5.6's update is the one form of writing
`CLAUDE.md` already permits. So nothing here waits on §13, and no sentence of `CLAUDE.md` moves.

Three questions, each answered from the standard before any code:

1. What does `images` write for an image with a mask, and what is a mask *on its own*?
2. What does `render --page-box X` mean — the extent only, or the clip as well — and what does
   `--no-annotations` leave out?
3. What does an incremental update carrying one embedded file consist of, where does it go in
   the name tree, and which of Table 22's bits governs it?

## Decision

### 1. `images`: the mask beside the image, and always where the file cannot hold it

ISO 32000-2 §8.9.6.1 lists the ways an image is masked, and two of the four are a *second
image*: §8.9.6.3's explicit mask, "a separate image XObject which shall be used as an explicit
mask specifying which areas of the image to paint and which to mask out", and §11.6.5.2's soft
mask through `/SMask`. §8.9.6.3 states the relation between the two images — "[t]he base image
and the image mask need not have the same resolution (Width and Height values), but since all
images shall be defined on the unit square in user space, their boundaries on the page will
coincide" — so a mask is an image on its own grid and the page's picture is the base seen through
it. That gives three honest outputs and one rule for choosing:

- **the composite**, a PNG whose alpha is the mask resampled onto the base's grid — the image as
  the page draws it, and the default on the decoded route because PNG is the one file form that
  can carry it;
- **the base as it is with the mask beside it** as `<name>.mask.png`, an 8-bit grey image on
  the mask's own grid whose value is the opacity it gives the base — `--no-mask` on the decoded
  route, and **always on the native route**, because a JPEG, or a JP2 whose opacity is not in
  its codestream, has nowhere to put a mask, and dropping it (what session 868's `--native` did,
  and said in the usage text) was the verb's one silent loss;
- **nothing beside it** where the mask is not an image — §8.9.6.4's colour key is a range of
  sample values, and `/SMaskInData`'s opacity travelled inside the samples — and the report says
  so rather than a sidecar format being invented, which is the same refusal RFC §6.3 makes for
  JBIG2's globals.

The base without its mask is `pdf_model::image::decode` over a copy of the dictionary with
`/SMask` and `/Mask` removed: one decoder, not a second route through it, and `decode`'s own
ordering (§11.6.4.3's "shall override") is what decides which mask is the mask where both are
stated. A soft mask's grey is its decoded samples, because a soft-mask image's samples *are* the
opacity; an explicit mask's is the alpha of the stencil decoded as a stencil, because §8.9.6.3's
"[u]nmasked areas shall be painted" are the places the stencil paints. The test derives the
relation the standard makes between the three files and holds the verb to it: composite alpha
equals mask sample on a shared grid, composite colour equals base colour, base opaque
everywhere, native JPEG's mask identical to the decoded route's.

### 2. `render`: the box asked for is both the extent and the clip; no annotations is no `/Annots`

§7.7.3.3's Table 31 gives the five boxes and the defaults that chain them — `CropBox` "[d]efault
value: the value of MediaBox", the other three "[d]efault value: the value of CropBox" — and
§14.11.2.1 the one rule on a processor for all four: where a box extends outside the media box,
"a processor shall treat the box as its intersection with the media box". `pdf_model::Page` already applies both, so
`--page-box` chooses among rectangles that are defaulted and intersected, through
`Page::boundary`.

**Extent and clip together, and it is a choice.** §14.11.2.1 defines every box as a clipping
region for a purpose — the crop box "the region to which the contents of the page shall be
clipped (cropped) when displayed or printed", the bleed box the same "when output in a
production environment" — so asking for a box is asking for that purpose's view of the page, and
marks outside it are not shown. The other construction, a larger extent around a smaller clip
with a blank margin between, is exactly §12.2's `/ViewArea` against `/ViewClip`, which is the
document's to state and not a flag's to invent; a document that states it is honoured under the
default, which remains the viewer's own `display_box` and `clip_box`. Under a named box the two
are that box. poppler's `pdftoppm` draws the media box unclipped by the crop box by default and
the crop box under `-cropbox`, which is the same reading arrived at independently — evidence,
not the reason.

**`--no-annotations` interprets the page as a page that states no `/Annots`.** §6.3.2.2 obliges
a rendering processor to render the appearance streams of annotations whose flags designate one,
so the default draws them; the opt-out removes the entry from the `Page` value handed to
`interpret`, and §12.5.3's pass has nothing to draw. Neither knob touches `interpret`: a `Page`
is a value whose public fields are the interpreter's inputs, `render` states the page it wants
drawn, and `Pages::detached` already makes the same move for §12.7.7's templates. This is why the
round changed no first-row crate and the corpus gates were not owed — the alternative, a knob on
`interpret`, would have been a first-row change for a distinction the `Page` value already
carries.

### 3. `attachments --attach`: §7.5.6 alone, three objects and a holder

The update is the source's bytes, byte for byte — "changes shall be appended to the end of the
file, leaving its original contents intact" — and after them:

- **§7.11.4's embedded file stream**: Table 44's `/Type /EmbeddedFile`, Table 45's `/Params`
  with `/Size` and `/CheckSum` computed from the bytes ("the standard MD5 message-digest
  algorithm"), unfiltered — a compression this crate chose would be a second decision in a writer
  whose one job is to say what was attached — and `/CreationDate` and `/ModDate` **only where
  `--date` was given**, because this crate has no clock and the same attachment is the same bytes
  on every run (RFC §9; §14.4's second identifier is ADR 0121's digest of the bytes so far);
- **§7.11.3's file specification**, indirect because Table 43 requires it where `/EF` is
  present, with `/F` and `/UF` both the filing name (Table 43: "[t]he UF entry should be used in
  addition to the F entry"), `/EF` naming the one stream under both keys, `/Desc` where given;
- **a new root for §7.7.4's `/EmbeddedFiles` tree**, one `/Names` node holding every entry the
  old tree held — values as the old leaves stated them, references included — plus the new one,
  sorted as §7.9.6 requires. **The whole tree is rewritten as one node, and that is a choice with
  a cost**: the clause permits it ("[i]f the root node has a Names entry, it shall be the only
  node in the tree"), it makes the update the same three objects whatever shape the producer
  chose, and the cost is a document with thousands of embedded files paying for all of them in
  one array — which no document in the corpus has. A key the tree already holds is refused
  rather than doubled: §7.9.6's keys "shall not overlap", and replacing the file would be a
  deletion nobody asked for;
- **the holder, rewritten at the nearest indirect object**: the old root's number where the tree
  was indirect, the name dictionary's where that was, the catalog's otherwise.

The new object numbers are `ViewState::save`'s answer — the larger of the table's highest number
plus one and the trailer's `/Size` — for the reason that function gives. `pdf_syntax::Document`
stays immutable: the update is a `BTreeMap` beside it, as the viewer's saves are.

**Table 22 bit 4 governs it, and the round was told there was no bit.** The instruction was to
say in this record that Table 22 has no bit for attaching; read, the table has no bit that
*names* it and one that binds it: bit 4, "[m]odify the contents of the document by operations
other than those controlled by bits 6, 9, and 11", is the residual every modification not named
elsewhere falls under, and an embedded file is not an annotation (6), a form value (9) or a page,
outline or thumbnail (11). So `Operation::Modify` reads bit 4 and the policy is asked once in
`apply`, at the three levels the seam already has, exactly as `Print` and `Extract` are.

The test reopens the output with this tree's own reader, reads the file back with Table 45's
checksum agreeing with the bytes, holds the source's prefix byte-identical, attaches twice into a
corpus tree and reads the keys back in §7.9.6's order, and — where `qpdf` is installed — runs
`qpdf --check` on the result as evidence about the reading, never its definition.

## Consequences

- `doc/todo/57`'s first section is done and its second loses `attachments --attach`; what waits
  on RFC §13 is now exactly the serializer and the four verbs that need it.
- `images --native` changes its output set: a masked JPEG now has a `.mask.png` beside it. That
  is a loss made loud rather than a new feature, and the usage text says so.
- `pdf-transform` takes `md-5`, a crate `pdf-syntax` already ships; nothing new in
  `doc/stack.md`.
- The `/Names`-dictionary-is-indirect holder case has no fixture in the tree — the two that
  exist are a catalog with no `/Names` and a tree that is itself indirect — and is stated as a
  gap rather than covered.
- Left where it was: `Operation` still belongs in `pdf_model::restriction` (`doc/todo/57` §3),
  now with three variants to move rather than two.
