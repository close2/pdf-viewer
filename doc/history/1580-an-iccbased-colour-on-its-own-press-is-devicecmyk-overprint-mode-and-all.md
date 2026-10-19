# 1580 — An ICCBased colour on its own press is DeviceCMYK, overprint mode and all

Slot 4 of batch ninety-two, 2026-10-19, a clause round. ADR 1996; ADR 1997 and Q475 unused.

**Premise.** It held: on a page compositing in a press, `cmyk_tints` gave an `ICCBased` CMYK mark
no tints, so `/OPM 1` reached none — probed at the pixel before anything was built. **The
hypothesis did not hold.** §8.6.5.7's implicit conversion is not `/DefaultCMYK` alone (NOTE 4: it
"need not depend on whether the CIE-based space is specified directly or indirectly"), and
`/Alternate` decides nothing; the test is the processor's own passthrough, which this reader makes
for a four-component `ICCBased` colour on a press of the same profile (`ColourSpace::to_cmyk`). There
"all of the semantics of the device colour space shall also apply", the overprint mode first.

**Found on the way.** `k` on a page with an output intent is drawn through the intent's profile, so
it carried no tints either: on a `/DeviceCMYK` page group with an intent the mode reached no
`DeviceCMYK` mark at all. The same rule restores it, since that colour is passed through too.

**Built.** `content::overprint::ProcessTints` — four tints and the profile they are in; whether they
apply is asked of the run's press when the mark is painted (`ProcessTints::on`), in §11.7.4.3's
bullet and in §10.8.3's spot planes. `cmyk_tints` answers for a four-component `ICCBased` space and
a `Separation` reverting to one. Four tests in `tests/overprint.rs`, each an equality with a named
colour; with the old arms planted back three fail, with the press ignored the fourth.

**Rows.** §8.6.5.7, §8.6.7, §11.7.4.3, §11.7.4.5 stay `implemented`, notes corrected: §8.6.5.7's
said the paragraph was answered for "this device's three additive colourants", stale since the press passthrough.

**Pixels.** None in the corpus: HEAD's six arms against this export's, 968 pages at 1× and 965 at
4× per lane, 0 different (corpus binary SHA-256 8049e76e… against HEAD's 2615c892…). `102952.pdf`:
77 pages at 1×, pages 1–8 at 10×, 0 different — it states no four-component blending space.

**Unfinished.** A `k` inside an `ICCBased` CMYK group whose profile is not the output intent's is
drawn through the intent and converted, where §11.7.2 says `DeviceCMYK` "shall be redefined within
the transparency group to be the same as the blending colour space"; a colour change in
`pdf-colour`, owed its own ADR (ADR 1996 §4). `pdf-transform/src/archive/overprint.rs`'s module
comment says `cmyk_tints` "gives one no tints" — slot 1's file, now one parenthesis stale.

**Gates.** rustfmt clean on the four Rust files. `RUSTFLAGS=-D warnings clippy -p pdf-model
--all-targets`: exit 0. `nextest -p pdf-model`: 2105 passed, 0 failed. `cargo test -p conformance`:
440 passed, 0 failed. `--bin quotations`: exit 0, no divergence in the four rows. `tools/state.sh
docs`: exit 0. Six arms in one `--tree 12` hold, 1812 s: each exit 0, 0 pages different.
`t3-oracle` in the export under `--clock`: exit 0, 3 passed. Duration: 4300 s.
