# 3567 — The converter's answers to the font classes: what the routes replace, and the output's check for the rest

Session 2216. Status: **accepted**; extends ADRs 3521 and 3543. Context: ADR 3565 (the classes), ADR
3566 (the mapping); ADRs 2773, 2846 and 3493 (the routes that embed the face the window drew); the
owner's PDF/A stance — archive every PDF it can see, the elegant remedy first, else a stated loss,
no refusal absolute.

## Context

ADR 3521's converter answers a repair outside content with the whole-file rewrite and leaves to the
output's own check what the rewrite carries as written. Every font class is logged at an object —
a program stream, a map stream, the descriptor — never at `Site::Content`, so none of them meets the
fence. What decides each is whether the output still holds the departure.

## Decision

Per class, under the stance:

- **`font/program-unreadable`**: answered where a font route embeds the face the window drew in
  the refused program's place (ADRs 2773, 3493, the `unreadable-program` remedy); the substituted
  program is the converter's own writing and logs nothing when the output is read.
- **The program repairs** (`sfnt-loca-*`, `sfnt-table-past-the-program`,
  `collection-cut-to-one-face`, `cff-font-dicts-rewritten`): no route answers them. The program
  stream crosses byte for byte, the output's check reads the same repair, and the document is
  refused by the class's name (ADR 3521's `declined_output`). Mapping them to section 6.2.11.4.1
  made the embedding routes run and refuse "not built yet" for want of a font with no program
  (ADR 3566), which is why that mapping waits for the remedy below.
- **The widths classes** fail `fonts/widths-agree-with-the-program` for a rendered font, and the
  widths route that restates a simple font's `/Widths` from the program answers a malformed array
  where it runs; elsewhere the output's check names the class.
- **`font-file-subtype-disagrees`**: the filing rewrite that restates a `/FontFile3`'s `/Subtype`
  for section 6.2.11.4.1's filing row answers it; elsewhere the output's check names it.
- **The maps and the encoding** (`to-unicode-*`, `cmap-damaged`, `differences-malformed`,
  `encoding-name-unknown`): no route rewrites these from the reader's reading, so a document that
  holds one is refused by the class's name by the output's check.

No class is refused before writing: a refusal up front would pre-empt the routes that already
answer some of them, as ADR 3543 found for the page entries.

## Consequences

- What is not answered is named in the refusal and is a later round's elegant remedy, in the
  stance's order: a program written as this reader read it (the `loca` restated, the one face
  extracted) — the same bytes the window drew — and a `ToUnicode` or `/Differences` restated as read
  at a stated loss for the entries that did not read.
