# 2460 — The archive census states whether the CJK pack was present, in its first line

Status: implemented. Session 1813, the archive slot of batch one hundred and twenty-one. Context:
ADR 2303 (the pack is found beside the program and held to digests it carries), ADR 2386 (CI
builds the pack), ADR 2442 (the ranking that counted 13 documents refused for the pack's absence).

## The problem

The census walks (`crates/pdf-transform/tests/archive_corpus.rs`) give every plan the pack named
by `QUORRA_CJK_PACK`, and nothing in their output said whether the variable was set. Round 1804
ranked thirteen pdf.js documents refused with `PACK_ABSENT`'s sentence; that is a condition of the
machine the census ran on, not of the converter, and a count taken with the pack and one taken
without it would read as a converter change if compared.

## The decision

Every walk of that file that converts prints one line first: `CJK pack: absent (…)` where the
variable is unset, `CJK pack: present (QUORRA_CJK_PACK), all 5 programs held to their digests`
where it is set. **A named pack that does not answer stops the walk** — a file missing or not
this tree's program fails the assertion, naming each — because a walk under a line saying the
pack was there whose fonts were refused as its absence is the silent fallback trap 5 is about.
The pack is one `OnceLock` for the walk, so each program is read and hashed once rather than
once per document.

## The census with the pack (against HEAD's tree, both profiles, 2b and 4)

Of the thirteen, nine convert under `as-if-printed` at both targets (issue11555, issue15053,
issue19182, issue6286, issue8372, mixedfonts, noembed-eucjp, noembed-jis7, noembed-sjis); under
`as-shown` all nine stop on `machine-face`, the profile's own `stop` (ADR 2162), which is the
design. The other four: issue2128r and issue3521 clear the font and stop on
`bound-names-are-valid-utf8`, a fence row no pack reaches; 90ms_rksj_h_sample (a descendant
stating no descriptor) and issue13343 (three descendants sharing one descriptor at three
`/DW`s) are ADR 2461's. veraPDF 1.30.3 passes all nine at part 4 and six at 2b; the other three at
2b fail its test 6.2.11.3.1-1, whose `CIDFontSupplement <= CMapSupplement` is the reverse of the
clause's own direction, which ADR 0922 records — each CIDFont states supplement 2 over a CMap
of supplement 1, which section 6.2.11.3.1 admits.

## Consequences

- A count and its column are read together; `doc/todo/72` names the variable beside the census.
- `issue5244`, which the handover named, is not one of the thirteen: it is an `Adobe-Identity`
  font, re-keying's population (ADR 2378), and its refusal sentence is that route's.
