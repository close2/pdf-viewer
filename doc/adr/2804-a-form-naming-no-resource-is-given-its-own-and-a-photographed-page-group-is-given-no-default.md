# 2804 — A form naming no resource is given a dictionary of its own, and a photographed page's group is given no default

Session 1996, a single archive round.
Status: **accepted and built** in `pdf-transform` (`archive/rewrite.rs`: `DefaultSites::unresourced`,
`names_no_resource`; `archive/rewrite/values.rs`: `write_default`; `archive/decision.rs`: `PAGE_GROUP_ROWS`,
`only_on_photographed_page_groups`, `only_photographed_page_groups_want_a_supplied_default`; `archive/prepare.rs`).
It answers two lines of `doc/pdfa-blocking-issues.md` section 8 and revises ADR 2795 section 4's third bullet.
Context: ADRs 2130, 2394, 2518, 2747, 2795.
Clauses: ISO 19005-2 sections 6.2.4.3, 6.2.10; ISO 19005-4 section 6.2.10; TechNote 0010 A003, A028; ISO 32000-2
§7.8.3, §8.10 (Table 93), §11.6.5.2, §11.6.6.

## 1. `issue14200` at 2b: a soft mask's group states no `/Resources`

The luminosity soft mask's `/G` is a form whose content is `q Q` and whose group blends in `DeviceCMYK`; it states no
`/Resources`. A028 reads part 2's default only from the dictionary explicitly associated with the content stream, so
no default reached it. ISO 32000-2 §7.8.3 asks a writer to "include a Resources entry in the stream's dictionary
specifying the resource dictionary which contains all the resources used by that content stream". **Where the content
names no resource, an empty dictionary is that entry exactly**, and the default is then written into it as into any
direct one. "Names no resource" is read conservatively: the only name operands are `DeviceGray`, `DeviceRGB` and
`DeviceCMYK`, which §8.6.3 makes self-identifying, and there is no inline image. A form that names anything keeps no
dictionary, because §7.8.3's earlier-versions rule looks its names up on the page, and an empty dictionary would hide
them; ADR 2795's alternative, copying the page's dictionary into it, is not built.

## 2. `bug1365930`: a default for a group the photograph takes away

The page's `/Group` blends in `DeviceCMYK`, and the page is photographed for its `Impact` text, which writes it
without `/Group` (ADR 2394). Round 1992's part 2 stand-in (ADR 2795) was built for that group and written into the
photographed page's resources: the shipped press as `press-assumed` under `as-if-printed`, the inks' profile under
`state-every-loss`, about 1.3 to 2.2 MB, changing no mark. ISO 19005-2 section 6.2.10 and ISO 19005-4 section 6.2.10
judge a group's `CS`, and the output holds no such group. **No default is owed.** Where every place a group row fails is
a page's own `/Group` on a page photographed for another row, the row is decided by the photograph, at the losses it
already takes, and where those are the only rows that could want a stand-in `/DefaultCMYK` the preparation drops it, so
the history states none (the packet is composed from what was prepared). A group in a form, or one on a page not
photographed, still takes ADR 2795's answer.

## 3. Evidence

`a_soft_mask_group_stating_no_resources_is_given_its_own_for_the_default` (`pdf-transform/tests/archive.rs`) and
`photographed::a_page_group_the_photograph_takes_off_its_page_is_given_no_default` fail under their plants
(`doc/history/1996-*.md`). `issue14200` converts at 2b under every main profile; `bug1365930`'s outputs under
`as-if-printed` fall from about 2.2 MB to 57 KB.
