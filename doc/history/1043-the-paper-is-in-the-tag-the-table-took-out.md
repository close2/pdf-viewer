# 1043 — the paper is in the tag the table took out

Date: 2026-09-14. No ADR: the decision is the standard's, cited where it is made. §8.6's six
`partial` rows and §8.10's five, each read against the clause. Touched `icc.rs`, four lines of
`colour.rs`, `content/colour.rs`, `tests/rendering_intent.rs` and the ledger. One row moved.

**§8.6.5.8 `partial` → `implemented`.** Table 69's `AbsoluteColorimetric` row: "no correction
shall be made for the output medium's white point (such as the colour of unprinted paper)". A
profile's `A2B1` states media-relative colorimetry, so the paper it describes lands on the
connection space's white and every reader of this tree drew unprinted paper as the display's white
under either colorimetric intent — the row's own last paragraph said so. ISO 15076-1:2010 is on
disk only as a preview, but ICC.1:2022 is the ICC's freely published current text of the same
standard and its clause 6.3.2.2 writes the derivation out: Equations (4) to (6) multiply each
tristimulus value by the ratio of the `wtpt` tag to the connection space's white, and (1) to (3)
are the way back. `icc::A2b::Absolute` is the fourth variant — `A2B1` with that scaling —
`Profile::to_xyz_with` applies it, `Profile::to_device` undoes it (it now takes the whole
`Rendering`, which is why `colour.rs` moved at all), and `Rendering::new` refuses compensation under
it, which is §8.6.5.9's sentence held by the type. The screen's side of the ratio is one because
clause 9.2.36 fixes a display's media white point at the PCS illuminant; a display-class *source*
profile is read at that value rather than at its tag — the 1998 sRGB carries its monitor's D65 there,
and both ICC.1:2022 and ICC.1:2001-12 §6.4.27 have already answered for it. No tag, no scaling.

The fixture is derived, not read off the raster: a D50-white table with the paper in `wtpt` under the
absolute intent draws the pixel a paper-white table draws under the relative one, with
`/UseBlackPtComp /OFF` so that the comparison is white points and nothing else. Calibrated by taking
the scaling out: the integration test and two unit tests fail.

**The ten rows that did not move, each read against its clause.** §8.6.5.5: `/Range` is the identity
for the three data spaces `Profile::parse` admits and the Decode default Table 88 hands an image, so
reading it closes nothing without the `'Lab '` class it exists for; that class is one item and it
stays owed. §8.6.5.7: a `should` whose general case costs under a level on the shipped sRGB; kept.
§8.6.5.9: ISO 18619, not held. §8.6.6 and §8.6.6.5: one `shall` about `NChannel` on a display, which
needs a blending the clause leaves to the processor; kept. §8.10 and §8.10.2: the aggregate is
`partial` for §8.10.4 alone once §8.10.2's unread entries are seen to be other clauses' subjects
(`/Metadata` §14.3.2, `/AF` §14.13, `/StructParent` §14.7.5), and it stays so. §8.10.4, §8.10.4.1,
§8.10.4.3: importing another document's page is a feature not built, with a population of zero in
67 195 files and a sandbox that would have to open a file for it; the proxy is drawn as the clause's
own provision.
