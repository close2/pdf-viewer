# 1092 — "otherwise OFF" is the answer to a comparison, and a zoom reapplies the dictionaries

ADR 1106. §8.11's six `partial` rows all stood on two sentences; both are now met.

**Census first (trap 8).** `examples/oc_usage_census` read only the groups an `/AS` named, which is
the narrower of two populations. It now reads every group `/OCProperties /OCGs` declares and prints
the three categories' *values*. Over **963 of the pdf.js corpus that open** (31 with
`/OCProperties`, 6 with an application) and **65 720 of the crawl** (2 409, 475): `/Zoom`, `/User`
and `/Language` appear in **no group's `/Usage` dictionary at all**, under either population. New:
`PageElement` on 1 462 declared groups, and a malformed `/Creator` for Table 100's `/CreatorInfo` on
63. **Calibrated (trap 13)** by planting a document stating all three — the census names each and
prints `min=1.5 max=4.0`, `Type=/Ind Name=(Alice)`, `Lang=(es-MX) Preferred=/ON`.

**The reading that moved four rows.** §8.11.4.4's `User` bullet — "If there is an exact match, the
ON state shall be used; otherwise OFF shall be used" — has an antecedent, *an exact match with the
user's identification*, so both branches sit inside a comparison; a processor nobody has told who is
reading is outside the sentence rather than in its second branch, and the AND above would otherwise
have turned the **group** off on a question nobody asked. `Recommendation::Unanswerable` was never a
departure, and the clause is met the moment a host answers. **`Language` is stated over the list**, not the group: the match is sought "among the Lang entries of
the optional content groups in the usage application dict ionary's OCGs list", `Preferred` decides
only among partial matches, and "All other groups shall receive an OFF recommendation" is taken
literally — the one category whose OFF the clause writes outright, against its own `Zoom` example
(§14.9.2.2 makes it case-insensitive).

**The surfaces are ADR 1076's and ADR 1101's**: `optional_content::Audience`/`Reader`,
`ViewState::set_audience`, `Command::Audience`, `viewer_host::audience` with `--reader-name`,
`--reader-title`, `--reader-organisation`, `--interface-language`, confined kind 30,
`quorra_audience`. `QUORRA_ABI_VERSION` unmoved. **Nothing is read off this machine** — a locale or a
login would be this program telling a document who its reader is.

**The reapplication is inside `set_magnification`**, not in a call a host can forget.
`OptionalContent::read` resolves the `/View` dictionaries once, so a zoom step reads no document and
a file with no `/AS` costs one empty-vector test. Groups restart from step b)'s initial state; a group
a person switched is pinned, the clause's own next sentence. `Magnified` splits the two obligations:
§12.5.3 re-places ink, §8.11.4.5 supersedes it.

**Four plants, four catches**: reapplication returning early (3 tests), override not recorded (1),
`Preferred` ignored (1), `/User` `/Type` ignored (1). Rows: `8.11.4.4` partial → **implemented**;
`8.11`, `8.11.1`, `8.11.4`, `8.11.4.1` and `8.11.4.5` stay `partial` on one residue — the Print and
Export *events*, which need a print operation.
