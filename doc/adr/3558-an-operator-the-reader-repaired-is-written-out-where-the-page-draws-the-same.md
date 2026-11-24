# 3558 — An operator this reader repaired is written out where the page draws the same without it, proved by drawing both

Session 2214. Status: **accepted and built**; extends ADR 2638 and ADR 2859 (`archive/as_drawn.rs`).
Context: ADR 0816's fence, ADR 3521, ADR 3541, ADR 3542 (`pdf_model::content::grammar`), the owner's
PDF/A stance (A694 item 3: where the input is bad, bake in the state the viewer shows).
Clauses: ISO 19005-2 and ISO 19005-4 section 5.1; ISO 32000-2 §7.5.6, §7.8.2, §8.4.2, §8.6.8,
§8.9.7, §9.4.1.

## Context

Five of `pdf_model`'s content classes are an operator's: short of its operands or given one of the
wrong type (§7.8.2), a `Q` with nothing saved (§8.4.2), a text operator outside `BT … ET` (§9.4.1),
colour operands that do not fit the current space (§8.6.8). The brief asked, class by class,
whether a narrow rewrite states exactly what the reader drew with no reading of the producer's
intent. Reading the interpreter for each answer would be a claim about this reader that decays;
the reader can be asked instead.

## Decision

1. **One rule for the five classes, not five readings.** Under a profile granting
   `content-as-drawn` (`as-shown` and `as-if-printed` both do, at the operator row and now at
   section 5.1's), each operator the grammar flags with one of these classes is written as one
   white-space character together with its operands — §7.8.2 makes them its own, and the reader
   clears them when it runs it — **only where the page draws the same**: the rewritten streams are
   appended to the source as §7.5.6's incremental update, and every page drawing one of them
   (through `/Contents`, resources or an annotation's appearance) is interpreted from both files;
   the write stands where each page's display list and text are equal, at most 256 pages.
   Otherwise every operator is left and the write is made again without them.
2. **Per class, what follows.** A `Q` with nothing saved: the reader ignores it, so it is written
   out wherever it stands alone (`bug1157493`, `issue1350`, `issue6298`). A short operator: written
   out where the reader drew nothing for it (`hesr13613`'s `> Tj`); left where it drew something
   (`T02-05-01_004`, whose page the proof found would change). Wrong type: the same test
   (`issue17679_2`). A text operator outside a text object that draws text: left, since writing it
   out takes a mark off the page and wrapping it in `BT … ET` would reset the text matrix the
   reader drew it with — a reading of the producer's intent (`T02-05-01_001`, `_003`,
   `issue10388_reduced`). Colour operands that do not fit: written out where the reader drew
   nothing different (`clippath`, `issue21570`), left where it did (`issue18894`, the
   `MICOM` poster). A left operator is photographed under `as-if-printed` (ADR 3557) and refused by
   its class's name under `as-shown`, in a sentence that now says why.
3. **The loss is `content-as-drawn`** — nothing a reader sees differs, but the producer's bytes are
   gone, as for ADR 2859's lone delimiter. It is in section 5.1's conditional losses, and the
   row's decision names it.
4. **A page's `/Contents` array is one walk**: the grammar's state (saved states, text object,
   operands) is carried from one stream to the next, and an operator whose operands began in an
   earlier stream is not written out.
5. **`inline-image-key-twice` is not one of them** (ADR 3560).

## Why this is on the near side of the authoring line

No mark is invented or moved: the proof is that the page draws what the window drew. It is the
photograph's test (A570) without the photograph's cost.

## Evidence

`quorra-transform render` of six sources and their `as-shown` outputs differs by at most 1 in a
channel, the output intent's colour conversion; veraPDF 1.30.3 passes every output.
