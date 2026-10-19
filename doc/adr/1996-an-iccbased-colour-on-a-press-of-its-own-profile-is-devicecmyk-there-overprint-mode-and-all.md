# 1996 — An ICCBased colour on a press of its own profile is DeviceCMYK there, overprint mode and all

Session 1580, slot 4 of batch ninety-two, a clause round. Status: **accepted** and **built** in
`crates/pdf-model/src/content/overprint.rs` (`ProcessTints`) and `content/colour.rs`
(`cmyk_tints`). Context: ADRs 0272, 1157, 1241, 1978; the §8.6.5.7, §8.6.7, §11.7.4.3 and §11.7.4.5
ledger rows.

## 1. The question, and what the clauses say

Round 1571 found that on a page compositing in a press this reader gives an `ICCBased` CMYK mark no
tints, so overprint mode 1 reaches none (ADR 1978 §2). The brief asked whether an `ICCBased` space
whose `/N` is 4 and whose `/Alternate` is `DeviceCMYK` is an implicit conversion to `DeviceCMYK`.

Read anew, §8.6.5.7 is not about `/Alternate` and not about `/DefaultCMYK` alone. Its subject is a
CIE-based source space that "accurately represents the particular output device being used", whose
values a processor "should" pass through rather than convert; NOTE 3 says the conditions "cannot be
specified in PDF", NOTE 4 that the conversion "need not depend on whether the CIE-based space is
specified directly or indirectly". It then binds the device space's semantics to the conversion
having been done: "When this type of implicit conversion is done, all of the semantics of the device
colour space shall also apply, even though they do not apply to CIE-based spaces in general", the
first being that "[t]he non-zero overprint mode (see 8.6.7, "Overprint control") shall determine the
interpretation of colour component values in the space". §8.6.7 names the same case from its side:
the mode applies when the current colour space is DeviceCMYK "(or is implicitly converted to
DeviceCMYK ; see (8.6.5.7, "Implicit conversion of CIE-Based colour spaces")". Table 146's fourth row,
"Any process colour space (including other cases of DeviceCMYK )", keeps `C_s` for every other case.

So the test is the processor's own: **does this reader pass the colour's values through?** It does,
in exactly one place — `ColourSpace::to_cmyk` hands a four-component `ICCBased` colour's own values to
a press sampled from the same profile (`Press::is_of_profile`, ADRs 0272 and 1054). There the
conversion is done and the `shall` binds; everywhere else it is not done and the row is the fourth.
`/Alternate` decides nothing: it names what a reader that cannot use the profile draws (§8.6.5.5).

## 2. Decision

`GraphicsState`'s tints become `ProcessTints`: the four values as stated, and the profile identity
where the space is `ICCBased` (`None` for `DeviceCMYK`). `cmyk_tints` answers for a four-component
`ICCBased` space and for a `Separation` or `DeviceN` reverting to one (§11.7.4.3 NOTE 2, as ADR 1241
reads it for `DeviceCMYK`). Whether they apply is asked when the mark is painted, of the press the
run composites in (`ProcessTints::on`), because a colour set outside a form is painted in the form's
group, whose press may be another. Both consumers ask it: §11.7.4.3's first bullet, and §10.8.3's
spot planes through `DeviceSpots::overprint`.

**This also restores the mode to `DeviceCMYK` on a page with an output intent.** `k` there is drawn
through the intent's profile (`ColourSpace::device_family`), so it carried no tints, and on a
`/DeviceCMYK` page group — whose press is that profile, Annex P's order — the mode reached no `k`
at all, though Table 146's first row is "DeviceCMYK , specified directly" in the file's own words.
The rule above gives it back without a second rule, because that colour is also passed through.

## 3. Proof, and what moved

Four tests in `tests/overprint.rs`, each an equality with a page painting the named colour: an
`ICCBased` colour on its own profile's press keeps the backdrop's zeros; one converted into another
profile's press, in both directions, is the source colour; `k` and `/Press scn` on an output-intent
page keep the mode; a `Separation` reverting to the press's profile takes it. Before the change the
first, third and fourth fail; with `ProcessTints::on` planted to ignore the press, the second fails.

**No corpus page moves.** HEAD's six arms against this change's, by page digest: 968 pages at 1×
and 965 at 4× on each of the processor, device and compute lanes, none different (the change is in
the corpus binary: its SHA-256 differs from HEAD's). The witness `102952.pdf` draws its 77 pages at
1× and pages 1–8 at 10× byte-identically — ADR 1978 §4 found it never states a four-component
blending space, so no run of it composites in a press. A page moves only where a press is in force,
the overprint parameter is true, the mode is 1 and a zero tint is stated in a space this rule now
admits; the corpus holds none (trap 8), so the fixtures are the evidence.

## 4. Found, not built

On a page with an output intent, a `k` inside a group whose `/CS` is an `ICCBased` CMYK profile
*other than* the intent's is drawn through the intent's profile and converted into the group's,
where §11.7.2 says "DeviceCMYK shall be redefined within the transparency group to be the same as the
blending colour space". §14.11.5's intent is informational and §11.7.2's sentence is a `shall`, so
the ranking is the standard's; the mode follows the colour and is lost there too. It is a colour
change in `pdf-colour`'s family resolution, owed its own ADR.
