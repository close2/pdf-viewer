# 2159 — A sampled transform's row, a producer's container, and a blank check box

Session 2159, an archive round in the `lane3` worktree, on the census's next rows after R06b: R01,
R12, R14 and R16. ADRs 3337 to 3340; no question. Ledger rows 7.10.2 and 12.7.5.2.3
(`implemented`, before and after; each gains a test). R06b's 42 at 2b are what a one-part request
refuses, and `rank.py` counts them `built` (ADR 2498's two-target request). Each document converted
below passes veraPDF 1.30.3, its page 1 at 72 dpi pixel-identical to the source's.

**R12, `devicen.pdf` (ADR 3337).** Printed: a sampled tint transform, "refused until that loss is
named". It is no loss: at zero each absent input of its `/Size [13 13 13]` table lands on a sample,
so the restriction is that row, copied bit for bit. Converts at 2b and 4 under both profiles.
Left, by name: an absent input between two samples, which no corpus document reaches.

**R14, `JPXTestGrey` (ADR 3339).** Printed: a producer's extension schema container not describing
`illustrator`, refused rather than given a second. The schema is now one more item of the
producer's own bag (`xmp::describe_into_container`). Converts at 2b and 4 under both.

**R16, `checkbox_no_appearance` (ADR 3340).** Printed: a check box's states "inventing the control's
vocabulary", and section 6.4.1's fence. The window draws nothing for either box; under
`field-appearances` (blank in the archive, as on paper) each is given the states its `/AS` and
§12.7.5.2.3 name, each drawing nothing, and the flag goes. Converts at 2b and 4 under both.

**R01.** Its `as-if-printed` document, `T02-03_006`, holds none of its one page (A775's
refusal); the render verb says "page 1 is past the end", which `rank.py` now reads as no page: X2.

**The history sentence (ADR 3338).** "XIMJKA+TTE201B838t00 (resource R19) was not embedded" is now
"carried a program this conversion removed", and its clause says the program "decodes to its end and
the check value the stream carries over the decoded program disagrees with it", not that the file
held its front; the operator's supplied-fact subject the same.

**Census.** With the pack, `as-if-printed` 2b pdf.js 943 (was 941) and corpora 449 (448), part 4
pdf.js 955 (953); `as-shown` 2b 906 and 414, part 4 916; the rest unchanged. Without the pack,
pdf.js `as-if-printed` is one worse at each part (`issue15053`, R08b), as before. Under
`as-if-printed` the four rows free nothing now: R06b and the exceptions at 2b, R05 and R15 at 4.

**Gates.** Over `-p pdf-model -p pdf-transform`: fmt 0; clippy `-D warnings --all-targets` 0 (after
a test helper's arithmetic); `pdf-sandbox` build 0; nextest `--lib --bins --tests` 100, 4191 passed
and two failed, both expected by `expected-failures-current.json`; the archive binary again after
the helper, 1546 passed; rustdoc 0 (after a private link); `fuzz` check 0 (`WriteError` and
`WrittenAppearance` gained a variant and a field); conformance 0; both censuses 0.
