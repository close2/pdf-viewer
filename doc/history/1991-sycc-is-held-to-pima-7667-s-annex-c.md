# 1991 — sYCC is held to PIMA 7667's Annex C

Session 1991, a single ledger round (§7.4.9, §7.6.6), the ledger lane. ADRs 2793 and 2794; Q825.

**The slice, and why.** §7.4.9's remainder, the only one of the two rows with anything a round
could test: sYCC (18), which was drawn by the codec and pinned only for neutrals.

**The premise.** Did not hold as briefed. §7.4.9: CIEJab is the owner's decision (A348), and so are
e-sRGB at a depth PIMA 7667 does not define and a Lab `IL` no text codes, both of which the clause's
own fallback answers. No opacity, depth or `/Decode` sentence is unbuilt. But held PIMA 7667's
*normative* Annex C converts the sRGB-based BT.601 YCbCr (sYCC) to e-sRGB, and ROMM-RGB's text
(PIMA 7666) was never asked for. §7.6.6: Table 27 only, which is §7.6.5's build. **A66's trigger is
lit**: A800 (2026-11-06) orders §7.6.5 built now. The brief withheld it, so it was not built.

**Built.** No behaviour. The codec agrees with Annex C (a probe, then the test), so the codec's
matrix is held rather than replaced. Comments in `image/jpx.rs` and the test module now say which
text fixes 18.

**Tests.** `sycc_is_drawn_as_pima_7667_annex_c_converts_it`: C7, C8 and B9 worked by hand on four
pixels inside sRGB's gamut and two outside it, clipped on this screen. C6/C8's half-level centre
difference is inside the tolerance and stated. **Plant**: 18 routed through e-sYCC's equations draws
red 212 for 182, so the test fails; reversed, it passes.

**Gates.** fmt `--check` (`rustfmt` on both files, `cargo fmt -p pdf-model`): exit 0; clippy `-D
warnings --all-targets -p pdf-model`: exit 0; nextest `-p pdf-model`: 2396 of 2396, 22 skipped;
conformance after `git add -N`: exit 0; `state.sh --round 1991 docs`: exit 0 (4477 s under load);
tier 2 `raster_golden`: 101, 16 first pages moved, HEAD's count, all font documents and no JPX;
`corpus`: 101, 974 documents, 62 incomplete, HEAD's count. Fuzz not checked: no public type changed. `launch_path` not run.

**Left of the rows.** §7.4.9 stays `partial` (ADR 2793). It owes sYCC drawn by Annex C, effort
only, because the codec clips a colour outside sRGB before any destination that is not the screen.
That build costs ADR 2293's refusal to a writer, and it was not taken here. Once it is built, the
row departs on 19 under A348. ROMM-RGB waits on Q825. §7.6.6 stays `partial` with §7.6.5's build,
which A800 has unblocked. §7.4 and §7.6 follow their leaves.
