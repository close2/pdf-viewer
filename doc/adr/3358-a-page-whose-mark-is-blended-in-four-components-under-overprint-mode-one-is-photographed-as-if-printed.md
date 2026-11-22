# 3358 — A page whose mark is blended in four components under overprint mode 1 is photographed, as if printed

Session 2164. Status: accepted. Context: ISO 32000-2 §8.6.5.6, §8.6.7, §11.7.4.3; ISO 19005-4
sections 6.2.4.2 and 6.2.4.3; ADRs 1978, 2394, 2516, 2749, 2750, 2889; A681, A694 item 1; R05 of
`doc/pdfa-blocking-issues.md`.

## The question

R05's last two documents, `363_Risk…` and `response-to-fiber-concerns[1]`, stop at part 4 under
`as-if-printed`. Printed: "the mode is set by a graphics state this conversion does not set to 0 …
the mark is blended in four components, where this reader's window would move (doc/adr/2749)". Each
paints a `DeviceCMYK` mark with overprinting on under `/OPM 1` inside a transparency group whose
`/CS` is `DeviceCMYK`. ADR 1978 refused that case under every profile: §11.7.4.3 composites the mark
in the group's own components, this reader composites by the mode there, and writing the mode 0
would change what the window shows; a `/DefaultCMYK` that left it 1 makes an `ICCBased` mark
section 6.2.4.2 forbids it for. ADR 2750 left "a per-document proof by drawing both" undecided.

## Decision

No proof by drawing: what a company does with such a page is print it and scan the print (A681,
A694 item 1), which is the photograph ADR 2394 built and ADR 2516 opened to a page's fault.

- At part 4, where the `/DefaultCMYK` is prepared and the profile grants `page-as-image`, the pages
  on which `overprint::blended_pages` finds such a mark are photographed as this reader draws them,
  the composite by the mode included (`fenced::Fenced::blended`, under the `DeviceCMYK` row).
- The default and ADR 2750's zeroing answer the marks on every other page: `under_a_default` passes
  over a mark on a photographed page, because neither it nor the state reaching it through that
  page is drawn from the output's content.
- The row's decision is the default's with the photographs' losses beside it
  (`cmyk_answer_under_part_four`); it is not the fenced rows' "answered by the photographs alone",
  and a photograph that was not taken refuses the row. A decision whose losses hold `page-as-image`
  asks for the photographs' rewrite beside its own.
- `as-shown` grants no photograph, and keeps ADR 1978's refusal (A694 item 1). So does any profile
  at part 2, whose `DeviceN` default leaves the marks `DeviceCMYK`.
- Not where a fenced row (ADR 2516) already photographs pages: those photographs answer the
  `DeviceCMYK` row where they take its page groups (ADR 2804), and the census showed `020747`, which
  converts that way, refused once a blended page this reader does not draw whole joined them.
- A mark in an annotation's appearance on such a page is not taken by the photograph; the output,
  held to the target afterwards, is then refused rather than wrong.

## What it costs

The losses of a photograph (`page-as-image`, and `text-as-image` and the tagging of the page where
it held text or structure), named per page with the row that asked. Both documents convert at 4 and
pass veraPDF 1.30.3; the photographed page drawn at the photograph's 300 dpi is identical to the
source's, every other page differs by at most one level at 72 dpi (the shipped press).
