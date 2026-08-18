# 592 — The memo decides which stream becomes an allocation

ADR 0365 windowed a page's `/Contents` and wrote down what it had not done: "a bomb hidden in a
form XObject still costs its gibibyte". `doc/todo/14` then asked for "a route that streams the
*first* read and remembers the bytes for the second, or a measurement saying the re-inflation is
cheaper than the memo". **It is neither** — the decoded-stream memo already declines exactly the
decodes that are dangerous, and a decode the memo declines is one this tree re-runs on every read
anyway. So windowing that half costs nothing that was not already being paid, and no new number
was invented to decide it.

Date: 2026-08-18.
ADR: [0427](../adr/0427-the-memo-decides-which-stream-becomes-an-allocation.md).

## What now streams, and what does not

A page's `/Contents` (ADR 0365) and now a form `XObject`, a Type 3 glyph description and an
annotation's appearance — including the soft mask's `/G`, which §11.6.5.1 makes a form. What does
not is a chain no `Pump` can produce, which is everything but a single `FlateDecode` with no
predictor, and **§8.7.3.1's tiling cell**, which is the round's second finding.

## The premise had an exception and the fuzzer found it

"A decode the memo declines is re-run on every read anyway" is a claim about *who reads*. It is
true of a form, a glyph description and an appearance, each of which asks for its bytes afresh
every time it is drawn — and false of a tiling pattern, whose `Tiling` holds the decode for the
whole tiling. Windowing that one inflates the cell again per cell, with `MAX_TILES` allowing four
thousand. `cargo fuzz run page` over this round's own seeds produced a mutated pattern that ran 76
seconds under the sanitiser; in a release binary it is **0.242 s before this round, 8.99 s after,
and 0.238 s with §8.7.3.1 taken back off the routing constructor**. Cutting its cell count from 25
to 4 takes the 8.99 to 3.95, which is what says the cost is per cell. The exception is kept by the
type `Tiling` holds — `HeldContent` — because a route decision is invisible in its output.

## Numbers this round printed

`doc/todo/10` §2's Bomb B, rebuilt from that file's description at 1 847 511 bytes and 1028.7:1,
put once in `/Contents` and once inside a form. `VmHWM` from `/proc`, `pdf-retrieve page … 0`,
`--profile gates`:

| | before | after |
|---|---|---|
| the bomb in a form `XObject` | 1032.3–1032.7 MB, 1.23–1.32 s, `undecodable form /Fx` | **10.5–10.7 MB**, 0.11 s, `MAX_OPERATIONS` |
| the same bomb in `/Contents` — the control | 8.4–8.6 MB, 0.10 s | 8.3 MB, 0.10 s |

Callgrind, `RAYON_NUM_THREADS=1`, A/B in one sitting with this round's own patch: ISO 32000-2 page
101 × 50 **+0.089%**, `prefilled_f1040.pdf` page 1 **+0.030%**, `alphatrans.pdf` page 1
**+0.015%**. `display_list_digest` over the 974: 37.2–37.9 s on both arms.

The three that stay on the routing constructor keep what they gained on the same 40 MiB seeds:
0.135 → 0.051 s for a form, 0.250 → 0.102 s for a Type 3 glyph, 0.119 → 0.037 s for an appearance.

## Correctness

`examples/display_list_digest` over every pdf.js corpus document's page one — 975 lines,
`sha256 04a07587…` — **byte-identical** across the change, with the same `pdf-sandbox-worker` on
disk for both arms.

## The spec-driven half

§7.3.8.2's three constraints on a stream's extent, read from the side road D asks them from: not
one of them bounds a *decoded* stream. `/Length` is the encoded bytes and Errata Collection 3's
Issue #319 sharpens that further; the self-limiting EOD bounds the input; the inferable length
holds for §7.10.2's sample arrays and Table 17's rows and not for a content stream. So the
standard offers no way to know a content stream's decoded extent before decoding it, which is not
a gap but the reason road D exists.

Also read in clause 7's errata: Issue #10 raises Table 5's `/F` row from *should* to *shall* for an
external stream's `/Length`, which this tree refuses anyway (§7.3.8.1); Issue #302 adds a `shall`
about the proper nesting of `BMC`/`EMC`, `BT`/`ET`, `BX`/`EX` and `q`/`Q` in §7.8.2, which nothing
here enforces and which is not this round's.

## Gates

The whole of `doc/todo/02` §2, green. Two fuzz runs: `page` over the 40 089-entry corpus, which
spent the round in libFuzzer's merge exactly as that file warns, and `page` over this round's own
26 seeds, which reached 7900 covered edges in twenty minutes and produced the tiling finding.

## Files

- `crates/pdf-syntax/src/document.rs` — `nested_content_source`, `is_pumpable`, `decoded_under`,
  `DecodedStreams::allowance`.
- `crates/pdf-model/src/content/reader.rs` — `NestedContent`, `HeldContent`, `Window::single`, `Shape`.
- `crates/pdf-model/src/content/run.rs` — `content_stream` returns a source, `run` takes one,
  `note_nested`.
- `crates/pdf-model/src/content/{pattern,transparency,annotations}.rs`,
  `crates/pdf-model/src/annotation.rs` — the call sites.
- `crates/pdf-model/tests/nested_content_window.rs` — new, and confirmed to fail in both
  directions.
- `fuzz/seed_nested_content.py` — new: 26 documents whose nested stream straddles the memo's
  allowance, because no document on this disk states one that does (trap 8). It is what found the
  tiling exception.
- `doc/conformance/ledger.toml` — §7.3.8, §7.3.8.2, §7.8.2, §8.7.3.1, §8.10.1, §9.6.4, §11.6.5.1,
  §12.5.5.
- `doc/todo/14`, `doc/todo/10`, `doc/todo/README.md`, `doc/verify.md`.
