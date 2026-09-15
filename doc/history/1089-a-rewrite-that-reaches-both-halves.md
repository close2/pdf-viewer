# 1089 — a rewrite that reaches both halves, and a space whose components need no transform

Date: 2026-09-15. ADR 1103. Two residues named by sentence: §8.6.6.5's `NChannel`, and §11.6.6 /
§11.7.2's isolated knockout group. Touched `colour.rs`, `content/transparency.rs`, two test files, a
new census, the golden and the ledger; three rows narrowed, none moved.

**§8.6.6.5, census first (trap 8).** `pdf-model --example nchannel_census` over 90 340 documents
(`doc/pdf.js`, `doc/corpora`, the crawled `safedocs`, `tika`, `openpreserve` caches): 7983 `DeviceN`
spaces in 2417 documents, 3764 `NChannel` in 1720; **3580 in 1650 have no spot colourant** — two name
every component of their process space, 3313 omit one under a `/DeviceCMYK` process space and 265
under one named otherwise; 205 carry `/Colorants`; **none states `/MixingHints`**.

**Built.** `ColourSpace::nchannel_process` answers with Table 71's `/ColorSpace` and which tint
supplies each of its components (`colour::Tints::Process`, beside `Tints::Transform`), asked *before*
the `alternateSpace` and `tintTransform` parameters are read — the clause's order, and why a space
whose tint transform this tree cannot read no longer loses the fill. Four conditions, each a sentence;
the last two figures decided the fourth, which is about the **space** and not the entry because
`/DeviceCMYK` and a four-channel profile can be one another (trap 6). Six tests, each
calibrated on a planted defect (trap 13). **A spot colourant keeps the tint transform, unreported**:
combining one with the process components is a blending the clause never states (NOTE 3),
and the tint transform is what its earlier paragraph requires of a display anyway (trap 11).

**§11.6.6 / §11.7.2: buildable in the pairing, no backend change.** §11.4.6's rewrite is a function
of the element list alone and a press pair's halves are one content stream interpreted twice, so
`knockout_construction` carries the pair and `commit_knockout` applies each rewrite to the black half
beside the chromatic one, committing neither unless the results still `paired`; `group_press` drops
its knockout condition. The initial backdrop needs no conversion — the group is isolated, and §11.4.6
gives it a transparent one — which kept this in `pdf-model`; `render-cpu` already encodes both lists
under the group's own `Compose`. No corpus witness (1039 measured), so the fixture is hand-built and
fails on three plants: the refusal restored, the black half left un-rewritten, and no rewrite
committing. `the_blending_space_is_the_one_in_force_rather_than_the_one_declared` took a better
lever: a `/DefaultCMYK` naming four components this tree cannot sample, what keeps the rows `partial`.

**Gates**, in a private worktree at the same HEAD with only this diff. fmt 0; clippy `-D warnings` 0;
`nextest` 4892/4892 (0); doc 0; fuzz fmt and clippy 0; conformance 7/7 (0); `pdf-model --test corpus`
0, every ratchet at its ceiling; oracle 3/3 (0). **`raster_golden` held 972,
moved 2, exit 101**: `stamps.pdf` p1 and `type4psfunc.pdf` p1, both "list only", each holding one
process-only `NChannel` space whose tint transform is the double complement `1 − (1 − t)`, not `t` in
`f32` — the list digest moves, the raster digest does not; regenerated, then 0. **Trap 1 by eye** on
both movers and on the knockout fixture at 4× beside the same group without `/K`.
