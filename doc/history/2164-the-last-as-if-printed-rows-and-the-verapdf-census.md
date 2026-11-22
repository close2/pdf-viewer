# 2164 — The last as-if-printed rows at part 4, and the veraPDF census

Session 2164, an archive round in the `lane3` worktree: R05 and R15 at part 4, the veraPDF census
over the kept outputs, then the `as-shown` rows. ADRs 3357 to 3359; no question. Ledger rows
14.13.2, 12.5.3 and 11.7.4.3 (`implemented`, before and after; each gains a test).

**R15, `issue17056` and `issue18030` (ADR 3357).** Printed: "nothing in a file specification states
one" (a media type). §14.13.2 names `application/octet-stream` for a type the writer does not know;
an associated file stating no `/Subtype` is given it, `Mechanical`. One stating a non-media type
stays refused; a `supply` table's `unlisted = "stop"` keeps an unnamed attachment refused.

**R05, `363_Risk…` and `response-to-fiber-concerns[1]` (ADR 3358).** Printed: "the mark is blended
in four components". `as-if-printed` photographs those pages as drawn and writes the default for the
rest; `as-shown` keeps ADR 1978's refusal. Not beside a fenced row's photographs. The photographed
page at 300 dpi is identical to the source's.

**R16 under `as-shown` (ADR 3359).** `issue7446`: an annotation with no `/Subtype` and no `/AP`
draws nothing (Table 167), so the preserve keeps nothing and appends no page.
`multimedia_annotations` at 4: marks carried under one subtype row answer all four. Held:
`poppler-395-0-fuzzed` (`as-shown` takes no stand-in face, by its own profile).

**veraPDF census** over round 2159's kept outputs: 5512 outputs, 109 failed over 26 rules and 33
documents, 20 unfinished. Triaged in `doc/pdfa-blocking-issues.md` section 1: about half ours, from
the source carried through (unparseable font programs, PFB headers, an empty CharSet, a string
`/BaseFont`, null appearance states, undescribed XMP properties, a bfchar Encoding CMap, 34-digit
integers), queued; the rest veraPDF's reading (`re%`, format 12 cmaps, `lenIV -1`, CFF supplement
SIDs, indirect box elements, §7.3.4.2's end-of-line, deep JPEG 2000).

**Census.** With the pack, `as-if-printed` 4: pdf.js 957 (was 955), corpora 483 (482); `as-shown`
2b pdf.js 907 (906), 4 pdf.js 920 (916). `020747` is refused at 4 with the pack by HEAD's own
binary too (a page past a drawing bound), so 2159's census predated a later edit; it is R08's.
Without the pack, pdf.js `as-if-printed` is one worse at each part (`issue15053`), as before.

**Gates.** Over `-p pdf-transform`: fmt 0; clippy 0 (after two helpers and a flag moved into
`ForbiddenAnnotations`); `pdf-sandbox` build 0; nextest 1287 passed, 0 failed, matching the empty
manifest; rustdoc 0; `fuzz` check 0 (`Rewrite` gained a variant); conformance below.
