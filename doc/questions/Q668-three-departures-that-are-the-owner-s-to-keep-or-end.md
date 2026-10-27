# Q668 — Three departures that are the owner's to keep or end

Asked by round 1802, which read the ledger's eighteen `departed` rows for a reason that no longer
holds (ADR 2439). Fifteen were settled in the tree: one ended (§7.4.2, ADR 2438), fourteen hold on a
clause or a figure. Three turn on a judgement the principles leave to the owner. Nothing below is
built; meanwhile each row stays `departed` exactly as its note states.

## 1. §7.4.8 — does Table 13's default of 1 apply to a frame whose components are named `R`, `G`, `B`?

The clause: "the default value of ColorTransform shall be 1 if the image has three components and 0
otherwise". The tree takes a three-component frame with no Adobe marker and component identifiers
`R`, `G`, `B` as RGB instead, which is `libjpeg`'s convention, stated by no clause of ISO 32000-2 or
ISO/IEC 10918-1 (ADR 1183). The cost is one page in the corpus, `issue11931.pdf`, which `poppler`,
`mupdf` and `hayro` draw as this tree does and `ghostscript`, obeying the default, draws with six
and a half times the ink. Principle 5 says agreement with the others is evidence and never the
target; "renders as its producer specified" says the producer's file is what decides.

*Recommendation*: **end it** — apply the default; the departure is a convention, the `shall` is
addressed to the processor, and principle 5 is the tie-breaker the project chose. The change is
small and its one page is known.

## 2. §12.6.4.6 — may a reader set a level at which a document launches a program?

Table 207's `/F` names an "application that shall be launched". The tree hands a launched
*document* to this machine's handler at four levels, `refuse` by default, and never launches a
*program* at any level (ADR 1358, ADR 2171). The reader's levels (*A document's restrictions are the
reader's to set*) are about what a document forbids its reader; a launch is a document asking the
reader's machine for power, which is principle 3's direction.

*Recommendation*: **keep it** — a program is never launched on a document's say-so, at any level.

## 3. §12.5.6.23 — is a lossless JPEG 2000 writer taken so a redaction can clear a component deeper than 16 bits?

A redaction rewrites a `JPXDecode` image's cleared grid under `FlateDecode`, whose samples Table 87
bounds at 16 bits; a deeper component cannot be written there without requantising pixels outside
the region, so the removal is refused by name (ADR 1371). Ending it takes a lossless JPEG 2000
encoder in the confined decoder worker — a dependency choice under `deny.toml` and principle 3.
No document on this disk carries such a component under a redaction.

*Recommendation*: **keep the refusal** until a document asks; it is named on `Report::refused`.
