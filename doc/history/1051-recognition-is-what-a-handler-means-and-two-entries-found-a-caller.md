# 1051 — Recognition is what a handler means, and two entries found a caller

Ledger slot of batch 1050–1055. Contract: §12.5's four parent-level debts, named by 1046 and never
read. **One closed, three named exactly, four calibrated tests, a census counter, no pixel moves.**

| row | was | now | the clause's answer |
|---|---|---|---|
| §12.5.1 General | `partial` | **`implemented`** | the `shall` is about *recognition*, and all 28 of Table 171's are recognised |
| §12.5.2 Annotation dictionaries | `partial` | `partial` | `/Lang` and `/AF` both read; what holds it is the `/AP` departure — Q63 |
| §12.5.3 Annotation flags | `partial` | `partial` | `Print` and `Locked` restrict operations no verb here offers |
| §12.5.5 Appearance streams | `partial` | `partial` | the `/ca` `/CA` contradiction is real and decided — Q63 |

**1. A handler is recognition, and the next sentence says so.** "A PDF processor shall provide
annotation handlers for all of the conforming annotation types" is followed by "An interactive PDF
processor shall provide certain expected behaviour for all annotation types that it does not
recognise" — a pair dividing the world in two, so the first is met when a type is answered from its
own clause rather than from the fallback. All 28 are: an arm each in `appearance::construct`, plus
§12.5.5's `/AP` for any subtype at all. The five media types keep their clause-13 exclusion and are
refused *by name*, which keeps them out of the unrecognised half; the row's old list named `Movie`,
`Screen` and `RichMedia` — wrong about `Screen`, silent about `3D` and `Sound`. ADR 1065.

**2. §14.13.9 was `implemented` on a function nothing ever called with an annotation** — exactly
§14.13.3's state before session 1023, so its row moves nothing and gains a caller.
`viewer::attached_files` is it: §12.5.1 makes activation the moment an annotation "exhibits its
associated object", its *such as* being examples. And Table 166's `/Lang` is the language of the
element whose content *is* the annotation: §14.9.2.3 orders three carriers, an annotation is none of
them, so the table's own sentence decides what the hierarchy cannot reach.

**3. §12.5.3's two bits have no subject here**, checked verb by verb rather than asserted: no print
path; `Detach` declines the producer's annotation, `set_free_text` is bit 10's, `add_markup` creates,
`pages --rotate` writes nothing, and `archive`'s `/F` touches only an annotation stating none.

**Calibration (trap 13) and gates.** `b"Watermark"` misspelt: the Table 171 test names `/Watermark`
reaching the catch-all. `referenced_language` dropped: the language test answers `en-GB` for
`es-MX`. The `/AF` loop emptied: the click test extracts nothing. The census counter asked for
`/Rect` rather than `/AF`: **34 819** over pdf.js's 974, against **0** `/AF` over 1479 curated — the
zero is the corpus's. Tier 1: `fmt --all --check` 0, `nextest --workspace` **4723 passed** 0,
`--workspace --doc` 0, both `fuzz/` lines' `fmt` 0, `-p conformance` 0. `accessibility_census` 0
(216 289 elements, 10 905 of them an annotation a client may click). Two lines carry siblings'
failures and nothing of mine: workspace clippy, all ten errors in `pdf-signature/src/revocation.rs`,
and `fuzz/`'s clippy, on `bin "x509"` against that crate's changed API.