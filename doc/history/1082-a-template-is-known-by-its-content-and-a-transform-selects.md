# 1082 — A template is known by its content, and a transform selects at last

Batch 1080–1085. Contract: §12.8.2's `partial` family — the residue round 1032 named, Table 257's
page-template operation and §12.8.2.1's parameters that do not select. ADR 1096 is the argument.

**1. How an instantiated template is told apart.** §12.7.7 puts a template out of the page tree's
reach — "[i]f the page is not intended to be displayed by the PDF processor, it shall be referenced
from the name dictionary's Templates tree instead" — and instantiating one adds "the named page to
the current document as a regular page". So `Kind::TemplateInstantiated` is a page **added** after
signing, stating Table 31's `/Type /Page` and `/Parent` (the two entries §12.7.7 says a template has
not got) and carrying the content of a page the signed revision's `/Templates` held — on content,
because the clause fixes the page's identity and not the mechanics of the copy. The receiving
`/Pages` node joins the operation only where Table 30's `/Kids` and `/Count` are all that moved,
nothing was lost, and every kid gained is one.

**2. The `FieldMDP` selection.** `Comparison::against_field_mdp` executes §12.8.2.1's `shall`:
Table 256's `/Data` says which object the analysis runs on — the catalog, the form dictionary, its
`/Fields`, or one field's subtree — and Table 259's `/Action` with `/Fields` says which fields are
covered, on §12.7.4.2's fully qualified name. `Covered`, `Outside`, `Excluded`, `Undecided`; a
`/Data` naming nothing is `NotScoped::NoData`, refused. **`UR` selects nothing and Table 256 says so,
not us**: "For transform methods other than FieldMDP , this object is implicitly defined."

**3. The census, 90 763 documents, 43 s — and the calibration it gave.** Documents naming each
Table 256 method: DocMDP 358, **UR3 327**, FieldMDP 170, UR 6, **FieldMDP2 1** — two names the value
list does not admit, and the walk read `UR` at zero until taught §12.8.6's route, where 327 of the
333 usage-rights documents keep theirs. Of 171 `FieldMDP` transforms 156 point `/Data` at the
catalog and 15 at nothing; over those documents' signatures the selection answers **15** times that
a covered field changed and **5** that only fields *outside* it did — calibration no fixture could
give, beside nine unit tests each naming its own plant. **97** carry a `/DocTimeStamp` (139
dictionaries; 1057 said 98, the command says 97 twice over), **1** a `FieldMDP` and **0** a `UR`.
`prefilled_f1040.pdf` states no `FieldMDP`, so no field of it is in one, and
`xfa_filled_imm1344e.pdf` still refuses comparison with its `/Data` on the catalog — the fixture's
shape.

**4. Rows.** §12.8.2.1 `partial` → **`implemented`**. §12.8.2, §12.8.2.2, §12.8.2.2.2, §12.8.2.3 and
§12.8.2.4 keep `partial` on one exact debt each: no host asks any of it (ADR 1043's), and for
§12.8.2.3 the second ranking, against Table 258's *rights*, that the clause states and nobody makes.

**5. Gates.** Tier 1 and all of tier 2, exit 0: `fmt --all --check`, `clippy -p pdf-signature`,
fuzz's two lines, `nextest --workspace` 4881, `--doc`, `conformance` 7, `pdf-model`'s `corpus`,
`dates`, `xmp` and `jpeg2000`, `pdf-transform --test gate`, `pdf-syntax --test on_disk`, and
`launch_path` at 26 banded 0 outside. Two are red on **siblings'** in-flight files: `clippy
--workspace` 101 on `render-cpu/src/lib.rs`, and `raster_golden` 101, 909 held 65 moved, all "raster
only (a rasteriser change under an unchanged list)" — 222 insertions stand in the two render
crates, and nothing here draws.
