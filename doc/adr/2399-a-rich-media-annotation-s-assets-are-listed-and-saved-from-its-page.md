# 2399 — A rich media annotation's assets are listed and saved from its page

Session 1782, slot 4 of batch 118. Status: implemented. Context: `CLAUDE.md` principles 2 and 5;
`doc/questions/A631`; ADRs 0295, 1186, 2382; ISO 32000-2 §13.7.2.3.2, Table 333, Table 341, §7.11.4.

## Decision

A631 makes §13.7.2.3.2's assets — "a name tree of embedded file specification dictionaries" — files a
person saves under the existing level for embedded files, never handed to a player.
`pdf_model::attachment::rich_media::assets` reads Table 333's `/RichMediaContent` and Table 341's
`/Assets` of one `RichMedia` annotation into `Attachment`s filed under the tree's keys.

- **From the annotation's page, not from `attachment::attachments`.** The brief and ADR 2382 said the
  document-wide walk; that premise did not hold. An annotation sits on a page, every host asks
  `Query::Attachments` when a document opens, and a document-wide list of annotation files is the
  page-tree walk ADR 0295 measured (78 to 123 ms cold over ISO 32000-2's pages) and principle 2 forbids
  at launch. So the assets join the list the way §14.13.4's page `/AF` does (ADR 1186): the page in view's
  rich media annotations, deduplicated by stream. A click on the annotation hands each asset out as
  `Event::Extracted`, as a click on §12.5.6.15's annotation does — §12.5.1's "exhibits its associated
  object" — and never as `Event::Media`.
- **`Command::Extract` looks a name up in the panel's list.** It read the log's list alone, so a file the
  panel listed from the page in view — a page `/AF` since ADR 1186, and now an asset — answered "this
  document embeds no file called …". One list now answers both; a name the document-wide list holds is
  still found first.
- **The naming rules are checked and said, not enforced.** The clause's other sentences are the
  producer's: the key "shall match the values stored for both the F and UF keys", is a relative URI (RFC
  3986's `relative-ref` characters), 1 to 255 characters, no U+0000, none of `: * " < > |`, and no FULL
  STOP last. A click reports each departure by the asset's name; the asset is listed and saved anyway,
  because withholding a file over its name tells a person less than a sentence beside it.

## Cost

One public module in `pdf-model`; `attachments_in_view` widened to `pub(super)`. §13.7.2.3.2 moves
`out-of-scope` → `implemented`; Table 341's `/Configurations` and `/Views` (§13.7.2.3.1) stay the
engine's and their row stays `out-of-scope`.
