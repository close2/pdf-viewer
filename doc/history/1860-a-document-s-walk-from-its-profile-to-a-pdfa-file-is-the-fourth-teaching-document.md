# 1860 — A document's walk from its profile to a PDF/A file is the fourth teaching document

Slot 10 of batch 126, 2026-10-28, a "docs" round (Opus 5.5); ADR 2554 used, ADR 2555 and Q726 unused. I worked in
`scratchpad/r1860/` and in an export of HEAD built there, its target deleted; no ledger row moved, no code changed.

**Premise.** "Five stages, six diagrams": held. The converter was read at commit 0c34fee4 (`git show HEAD:<path>`),
because four sibling slots were editing `crates/pdf-transform/src/archive/`; the walk says so in its opening and names
RFC 0014 as what comes next.

**Built.** `doc/teaching/how-a-document-becomes-pdfa.md`: the profile (read by the caller, checked against one target,
folded into `ArchivePlan`), the verdict and one `Decision` per failure, the remedy in four shapes, the output held to
the target and settled (ADR 2534), and the report in text, JSON and `xmpMM:History` with the caller's tool loop — one
Mermaid diagram for the whole and one per stage, each with its maintenance note. The running example is a one-page
invoice (DeviceRGB, an unembedded Helvetica, an image with `/Interpolate true`, a FreeText with `/DA` and no `/AP`);
every report string was read off `quorra-transform archive --to 2b --profile as-if-printed`, built from HEAD: eight
requirements decided, the second pass settled `fonts/font-programs-embedded`, veraPDF 2b compliant (144 rules passed,
0 failed); with no profile it is refused on `image-smoothing`, exit 4. `WALKS` holds four walks; `doc/HANDOVER.md` and
`doc/crate-map.md` point at the new one.

**Plant.** `settle.rs` renamed `settled.rs` in one path of the walk, in the export: the gate failed naming
`crates/pdf-transform/src/archive/settled.rs`; mended, green.

**Found, not fixed.** `settle::fold` (`crates/pdf-transform/src/archive/settle.rs:80`) keeps the first pass's row for
a requirement both passes decided, so the invoice's `fonts/font-programs-embedded` reads "1 place(s) … (1 done)"
beside "2 font(s) … now embed a face". The two font `xmpMM:History` entries (composed near `prepare.rs:5383`) both say
"Helvetica was not embedded" and do not name the resource (`F1`, `Helv`), so a later reader cannot tell them apart.

**Gates.** `cargo test -p conformance --test teaching` exit 0, 3 passed (shared tree and HEAD export; planted: exit
101, then 0); `cargo test -p conformance --no-fail-fast` exit 101 — two reds, neither mine: `evidence` (record 1854's
scratch path) and `records` (records 1858 and 1859's Gates paragraphs); `--bin quotations` exit 0, `--bin pointers`
exit 0, `--bin cited` exit 0, none naming a file of this round.
