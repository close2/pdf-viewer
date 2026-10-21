# 1671 — Width-array counts, Symbol selections, and a proved program that survives writing

Archive round, batch one hundred and four, 2026-10-21. ADRs 2178, 2179; Q566.
**Built.** Table 109's simple-font array count is an independent row under both font clauses,
linked in the sentence-coverage table. Its `Stated` remedy narrows LastChar or truncates only
ignored tail widths; every retained advance stays unchanged. The prior row claimed unchecked
by the brief was already implemented; font-format conformance remains separately unchecked.
Two Symbol fonts lost their code-to-name table on TrueType→CFF restatement: Differences now
state the source names, with the full proof retained. Eight other documents lose horizontal
fitting, not hinting; their specific refusal remains and Q566 proposes a separate loss.
The `316267` second verdict caught a shared descriptor's later program overwriting an earlier
font's proved metrics. Each simple font now gets its own descriptor; incompatible composite
programs sharing one descriptor refuse before overwrite. Descriptor cleanup is shared by both
write routes. The state archive report now retains the real-census counts and refusal details.

**Real corpora** (converted/conforming/refused/unreadable, integrated tree):
| corpus | printing 2b | printing 4 | as-shown 2b | as-shown 4 |
|---|---|---|---|---|
| pdf.js (976) | 756/2/198/11 | 703/0/253/11 | 751/2/203/11 | 699/0/257/11 |
| corpora (503) | 340/6/129/16 | 309/0/166/16 | 318/6/151/16 | 294/0/181/16 |
Against immutable HEAD only corpora printing 2b moves: 339/6/130/16 → 340/6/129/16.
The categories omit nine and twelve source-structure refusals respectively, unchanged; no panic.
**Witnesses.** veraPDF 1.30.3 accepts converted `issue6410`, `wp315`, `316267` at 2b;
`wp315` emits its invalid-glyph-code warnings beside PASS. `issue6410`, page 1 at 100 dpi,
827×1170: both renders inspected, four pixels differ by one level of 255 in the footer.
Its conversion also embeds Helvetica and restates four encodings. The two Symbol documents
clear the embedding refusal but still meet independent content-stream fences.
**Gates.** Scoped fmt and `-D warnings` all-target clippy exit 0; nextest 909 passed, 15 skipped,
4.228 s tests. Shared-descriptor regression first red, then green; width, Symbol and descriptor
fixtures cover both parts. State archive validator: zero missed/over for every target, 39 s, 0.79 GiB.
`tools/state.sh archive`: exit 0, 1329 s including queues; converter corpus 228 s/2.19 GiB,
2b 525 converted/86 refused, 4 266/93; all conforming sources stay conforming. Cross-check:
2908 documents, 12951 pages, 13547 selections, zero disagreements (77 s/1.03 GiB).
Conformance: exit 0, 136 s. Foreign clock: exit 0, 162 s/6.40 GiB; no panics, 143.48 s test.
Foreign: 203/974 sources; 8 draw differences and 5 structure faults reported across verbs.
Sample hold: exit 0, 66 s including 54 s build, 2.13 GiB. The first sample command failed at
build parsing (duplicate --bins), before reading a PDF; corrected run passed all three.
**Limits.** Existing xref-only font traversal omits inline fonts; missing arrays, invalid byte
ranges and Type 3 stay gaps. Empty arrays stay refused. CIDFont descriptor isolation remains
unbuilt; its collision now refuses. No fitting loss is inferred from machine-face permission.
