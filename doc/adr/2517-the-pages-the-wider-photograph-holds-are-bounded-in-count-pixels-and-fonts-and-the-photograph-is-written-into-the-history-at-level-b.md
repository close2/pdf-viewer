# 2517 — The pages the wider photograph holds are bounded in count, pixels and fonts, and the photograph is written into the history at Level B

Status: implemented. Session 1841, the second archive slot of batch one hundred and twenty-five.
Completes ADR 2516 with the bounds principle 3 asks of every loop an input reaches, and with one
defect the wider gate exposed. Context: ADR 2394 item 5 (the photograph is a loss the file states
in its own history), `doc/questions/A694`.

## The bounds

A document of many faulty pages must cost what a bounded number of pages cost. Four numbers, all
of the wider gate (ADR 2516) alone:

| bound | value | what it holds | past it |
|---|---|---|---|
| pages photographed | 32 | rasters at 300 dpi are held, as encoded streams, until the file is written | the 33rd page's refusal names the bound |
| pixels photographed | `1 << 28` together | what `Budget::max_pixels` lets one page be drawn to, so a document of hostile pages costs what one hostile page did | the pages the fenced rows name are refused before the first is drawn; others, before the page that would pass it is drawn, by `TargetSpec` |
| fonts photographed around | 16 | each is a restart of the font pass over the whole document | the 17th keeps its own refusal |
| text named | 1000 characters a page | the report and the history carry the page's text, white space collapsed | the count left out is stated |

The page and pixel bounds apply to a page only when the wider gate is what admits it; a page the
narrow gate admits (ADR 2394) is photographed as before, with no count. The pixel total is the
tighter of the two for ordinary pages: letter-size pages at 300 dpi are 8.4 million pixels each, so
thirty-one fit; the page bound is the tighter for small ones.

Measured on the final binary, with the load average below twelve, from a generated source of letter
pages each holding eight thousand words and an operator no standard defines: 31 pages written, 634
MiB peak resident, 47 s, a 122 MB file; 32, 33 and 40 pages refused in 0.3 s and 17 MiB, as
`as-shown` refuses them (the fenced pages are counted and sized before any is drawn). The source
with eight thousand words in one string is refused earlier, by ISO 19005-2 section 6.1.13's string
length. A single page drawn to the per-page pixel budget costs 1.7 GiB peak resident; that is the
budget ADR 2394 already allowed one page, not a new one.

The pages a fenced row names are known from the survey before any page is drawn, so a document
whose fenced pages are more than 32, or whose display boxes at 300 dpi already pass the pixel total,
is refused by name before the first is drawn. A page the wide gate photographs because of a font
or its text is known only as each is interpreted, so a document of many such pages costs the
drawing of the pages up to the bound before its refusal; predicting those from the page boxes is
not built.

## The history at Level B

ADR 2394 item 5 states the photograph in the file's `xmpMM:History`. At Level B with a packet that
needed no other edit, the rewriter's test for "write the packet" did not see the photograph, so
the History was not written. The test now also holds when a font substitution is wanted and any page was
photographed. The same gap exists for a plain font substitution at 2b with a conforming packet
(the report states the loss and no History is written): found, not fixed.
