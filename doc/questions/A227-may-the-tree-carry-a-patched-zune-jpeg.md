Status: complete
Given: 2026-10-05, in conversation — transcribed by the round
Owes: both patches carried — a fork of `zune-jpeg` under the owner's control, pinned by `rev` and wired the way the `hayro` fork is, carrying `doc/patches/zune-jpeg-dc-prediction-overflow.patch` and `doc/patches/zune-jpeg-scan-complete-without-eoi.patch`; the DC-overflow reproduction becoming a regression test in `crates/pdf-model/tests/banded_decodes.rs` whose whole decode returns rather than panics; the grey-row test updated the day the second patch lands; and both reported upstream with their reproducers, the fork retiring when a release carries the fixes (acting round)

> I agree with all your recommendations.

Reading: **Yes — a second fork, on the `hayro` precedent.** The tree already carries an
owner-controlled fork pinned by `rev` with its patches in `doc/patches/`, and this answer extends
that to `zune-jpeg` for the two defects the fuzzer found, both with reproducers. The first patch
makes the DC dequantisation's multiply wrap as the addition four lines above it already does —
the release build's behaviour in every profile, changing no sample any release build produces, and
ending the aborts that made hostile bytes kill a test or fuzz run instead of being reported. The
second changes samples a release build produces, on baseline scans complete by ITU-T T.81
section E.2.3's MCU count that end with no `EOI` and no other frame — decoding their last MCU row
whole where 0.5.15 fills it with grey — with the regression test holding the current grey row by
name so the change is forced and visible the day the fork takes it. Upstream comes first in the
obligations: the fork exists until a release carries the fixes, not as the permanent home of
either patch.