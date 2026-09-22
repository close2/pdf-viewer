# 1174 — Which operations are exports in §8.11.4.5's sense, and what a writer owes an alternate

Session 1168. Status: **accepted**. Companion to [ADR 1173](1173-what-the-output-is-for-is-an-input-and-the-adjustment-sets.md).

ADR 1173 built the input. This one is the two questions that follow from it: which of this
program's operations supply `Purpose::Export`, and what a writer that rewrites an image carrying
§8.9.5.4 `/Alternates` has to do now that step c) draws one of them.

## 1. The clause states two conditions, and both have to hold

§8.11.4.5:

> Similarly, when a document is exported to a format that does not support optional content,
> usage application dictionaries with an event type Export shall be applied over the current
> states of optional content groups.

and Table 100's `/Export` entry says the same thing with its own example:

> This value indicates the recommended state for content in this group when the document (or part
> of it) is saved by a PDF processor to a format that does not support optional content (for
> example, a raster image format).

So: **(i)** the document or part of it is saved, and **(ii)** the target format cannot carry
optional content. And one thing follows from what the event *does*: it sets group **states**, and
a state decides what is drawn (§8.11.3). An operation no group's state bears on is an operation
this event has nothing to say about, whatever its format.

## 2. The five candidates, decided from the clause

The revisit note named five operations that now exist. Only one of them is an export here.

- **`quorra-transform render` — yes, and it is the clause's own example.** It interprets a page
  and writes the raster to PNG, PPM or PGM. Both conditions hold and the states decide every
  pixel. `render::exporting` is where it is stated, and the duration is that state's lifetime:
  the states there were never a reader's, so there is nothing to revert to.
- **FDF and XFDF export of form data — no.** Condition (ii) holds, and (i) does not bear: what
  leaves is field names and values, and no optional content group governs a `/V`. §8.11.3.2
  makes optional content a property of marked content, `XObject`s and annotations; §12.7.4.3's
  value is none of those. A `SubmitPDF` submission (Table 240 bit 9) and an `EmbedForm` one
  (bit 14) carry a PDF, which supports optional content, so (ii) fails for them instead.
- **Embedded-file extraction — no.** What leaves is the attached file's own decoded bytes. An
  embedded file stream (§7.11.4) is not content any group governs, so there is nothing for a
  group state to change.
- **Clipboard text — no.** `Event::Copied` carries a *sub-range of the page the person is looking
  at*, identified by position in the display list the `View` event produced. Applying a different
  event would put text in the clipboard that they did not select and remove text they did, and
  §8.11.4.5's "for the duration of the export operation" has no duration to name here: there is
  no artefact being composed, only a range of one that already exists.
- **The derived PDFs — no, on condition (ii).** `split`, `merge`, `pages`, `optimize`, `redact`
  and the archival converter all write PDF, which supports optional content, and they carry the
  producer's `/OCProperties` forward. The event exists to decide what a format that *cannot* hold
  the layers should show; a format that holds them needs no decision.

`quorra-transform images` is worth naming with them because it looks like the raster case and is
not: it copies image objects out by identity rather than drawing a page, so no group's state
bears on what it writes. The line is the same one in every row above — **does the operation's
output depend on what is drawn?**

## 3. What a writer owes an image carrying `/Alternates`

The grep, whole, over `crates/**.rs` outside tests, `pdf-archive`'s tables and the interpreter:

- `pdf-transform/src/archive/rewrite.rs` — `Rewrite::ImageAlternatesAndOpi` **removes**
  `/Alternates` and `/OPI` from every image `XObject`, which is the archival target's own
  requirement and leaves the base image as the only image. Nothing is owed: removing the entry
  removes the question.
- `pdf-signature/src/signature.rs` — counts images that state the entry, for Table 264. A census,
  not a write.
- `pdf-model/src/thumbnail.rs` — lists the key among the entries §12.3.4's thumbnail image does not
  inherit from the page's. Not a write either.

**And one place did owe something: redaction.** §12.5.6.23 requires destruction rather than
concealment — "[i]f a portion of an image is contained in a redaction region, that portion of the
image data shall be destroyed; clipping or image masks shall not be used to hide that data" — and
§8.9.5.4 calls an alternate a variant representation of the *same* image: "[t]hese variant
representations of the image may differ, for example, in resolution or in colour space." So
clearing the base image's samples and leaving its alternates alone destroys one copy of the
picture and leaves another in the file. ADR 1173 §4 makes that second copy *reachable*: step c)
draws the `/DefaultForPrinting` alternate whenever the output is a printing, so what was a
latent object in the file is now ink on paper.

Destroying an alternate's samples too is a capability this writer does not have — each alternate
is its own grid and its own filter, may be behind a codec the region clearing refuses, and may be
shared with another page's placement, which the single-referrer guard would have to be extended
to reach. So the page is **refused by name**, in the same voice as the `JPXDecode` refusal beside
it, rather than cut wrong (trap 5, principle 1). Building the destruction is the owed capability
and it is stated as one rather than hidden.

Nothing in the corpus moves: no corpus document carries `/Alternates` at all.
