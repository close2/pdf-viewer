# 3560 — What stays refused by name: a duplicated inline image key under `as-shown`, a page tree cycle, a `/Contents` entry that is no stream

Session 2214. Status: **accepted**. Context: ADRs 3557–3559, ADR 3543, the owner's PDF/A stance (no
refusal is absolute: each refusal below names what would lift it).

## Decision

- **`model/inline-image-key-twice` (§8.9.7)** — under `as-shown`, refused. Which of the two values
  this reader used is its lexer's choice, and dropping the other edits bytes inside the image's
  dictionary that §8.9.7 gives no order to; the proof of ADR 3558 would hold it, but a rewrite of
  an inline image's dictionary is not built (`InlineAbbreviations`, `issue14256`). Under
  `as-if-printed` the page is photographed (ADR 3557).
- **`model/page-tree-cycle` (§7.7.3.2)** — refused under both (`Pages-tree-refs`). The whole-file
  rewrite writes `/Kids` as stated, cycle included; restating the tree as this reader walked it is
  ADR 2456's recovered-structure construction, not yet reached for a cycle.
- **`model/contents-entry-not-a-stream` (§7.7.3.3)** — refused under both
  (`T02-05-01_013_stream-keyword-missing`). The reader drew nothing for the entry; a photograph
  would carry that, but the fence (ADR 3557) photographs repairs at the content site only, and this
  one is at the page object.
- **A `/Rotate` that is no multiple of 90** — no document of the census reaches it; left as ADR
  3543 left it.

Each is a stated question for the next archive round, ranked by the census.
