# 1418 — The forward projection stands beside each inverse, and the remedy listing names its halves

Slot 5 of batch sixty-five, 2026-10-07, a clause round. ADRs 1672 and 1673. §12.10.2 stays
`partial` with its note moved; §12.10 derives, unchanged. No question written.

**Premise.** It held for the projection: `projection.rs` had `inverse` and no `fn forward`, and the
row named Q271. It did not hold for the listing's place: `print_remedy_sites` is in
`crates/pdf-transform/src/bin/quorra-transform.rs`, not `crates/pdf-archive/src/`; the hunk went
there and into `archive/config.rs`, which no slot owned.

**The forward.** `Projection::forward` for the eight methods, each method's constants shared with
its inverse. Held to 1.5 cm on the ground, which is ADR 1587's 0.0005″ as a distance, and the round
trip to 0.0005″. Five of seven worked examples meet half their last printed digit. The British
National Grid example is off by itself: its printed η and ξ give 577274.984 m and 69740.492 m, but
it prints .99 and .50. Its constants and intermediates are asserted to their digits. Four domains
are refused rather than left to `f64`, among them the Pseudo-Mercator past 88° in both directions.

**What it serves.** A projected `/DCS` on the file's datum is now displayed (`Geospatial::display`)
where it used to be refused. `Viewport::page_position` turns a position back into its page point.
Through `/PCSM` it projects and solves the matrix; otherwise it inverts ADR 1593's fit, now
`geospatial::AffineRegistration`. The census's round trip at every registration point: 160 systems
over seven methods, all within 0.0005″, none refused.

**The listing.** A split site prints its two halves, saying which one is answered. Four of the
eleven split requirements are sites under no target, because no row changes what happens to them;
the test says so for each. Planting an empty shape list failed the test.

**Unfinished.** A window writes eastings for a projected `/DCS` only after a host change (the
next host brief carries it). Slot 2 (round 1415) took this round's 95-line patch in the same batch,
so `viewer-core/src/located.rs` now calls `AffineRegistration`; the merge gave §12.10.4's note its
sentence on the forward (ADR 1672).

**Gates.** rustfmt `--check` on the eight touched sources: exit 0. `RUSTFLAGS="-D warnings" cargo
clippy --all-targets` for `-p pdf-model` and for `-p pdf-transform`: exit 0 each. `cargo nextest run -p
pdf-model`: 1977 passed. `-p pdf-transform`: 487 passed. `cargo test -p conformance`: 53 binaries,
403 passed, 0 failed, `the_frontier_map` among them. Behind the lock: `--profile gates -p pdf-model
--test corpus`: exit 0, ratchet 59/59, 83 s. The geospatial census: exit 0, 174 s, 8.81 GiB.
