# 1211 — A writer's `shall not` is a signature, and a shape selects a half

Status: accepted. Session 1187.
Context: `crates/pdf-model/src/attachment/filing.rs` (`tree_root`), `crates/pdf-syntax/src/tree.rs`,
`crates/pdf-transform/src/{merge,split}.rs`, `crates/pdf-transform/src/attachments.rs`,
`crates/pdf-transform/src/archive/{config,decision}.rs`, `doc/errata-read.md` #307,
`doc/rfc/0007` section 5b.2, `doc/pdf-a-mitigations.md` section 14 finding 1. Amends ADR 0660's
open item and ADR 1012's last one.

Two decisions, unrelated in subject and identical in shape: **the place a rule is stated decides
whether a check is needed at all**, and a check that cannot fail is worse than no check, because it
tells a reader there is a failure mode when there is not.

## 1. Errata Collection 3's Issue #307, and why no refusal was built

The erratum adds one sentence to Table 36's `/Names` row and the same sentence to §7.9.7's Table 37
`/Nums` row: *Keys shall not be the null object.* `pdf-syntax`'s reader has met the reader's half
since it was read — a null key costs its own pair and the pairs after it keep their values. The
writer's half was owed, and ADR 0660 recorded it as owed.

**The brief asked for a typed refusal at each of the four writers, and that turned out to be a
refusal that could never be returned.** Every one of `merge`, `split`, `attachments` and
`attachment::filing` built its `/Names` array out of a `BTreeMap<Vec<u8>, Object>` or a
`Vec<(Vec<u8>, Object)>`; a key is bytes, it leaves as `Object::String`, and no caller has an
`Object` to put in a key position at all. A `Refusal` variant for it would be dead code with a
doc comment claiming otherwise — `CLAUDE.md` principle 1's "no placeholder" read from the other
end.

**So the decision is: the prohibition is one function's signature.**
`pdf_model::attachment::filing::tree_root` is now the only place this tree writes a §7.9.6 `/Names`
node; `merge` and `split` had their own copies of the loop and now call it. The erratum's sentence
sits above that function, the reader's half cites it across, and `tree.rs`'s module comment joins
the two. A later round adding a fifth writer inherits the guarantee by calling the same function,
which is the whole reason to have one.

**What a signature cannot assert is that a null cannot *cross*** — a source states one, the reader
drops it, the writer emits the survivors — and that is what the tests are for: a hand-built document
whose `/Dests` (or `/EmbeddedFiles`) tree states a null key between two real ones, put through
`merge`, through `split` and through an attach, with the pairs either side keeping their own keys
every time. Hand-built rather than found, because a file that states one is by this erratum's own
words a file that should not exist; trap 4's other half, and `tree.rs`'s own fixture set the
precedent.

**`pdf-transform`'s `structure.rs` writes §14.7.5's `/IDTree` the same way and was left alone**: its
keys are bytes too, so it is under the same guarantee, and it is a number-tree sibling whose round
this was not. A round that touches it should route it through `tree_root` as well.

## 2. `Qualifier::Shape` selects, and the half with no answer stays inert

ADR 1012 built the shape qualifier into the configuration grammar and left it discarded: a
shape-qualified row read, validated and applied to nothing. `doc/rfc/0007` section 5b.2 and
`doc/pdf-a-mitigations.md` section 14's first finding say why the grammar needs it — seven
requirements fail for two different reasons under one identifier, and *an operator who answers the
safe shape must not be taken to have answered the dangerous one*.

**What was missing was the enumeration, and it belongs beside the decision table.**
`decision::SHAPES` now states each split requirement's halves and which half `REMEDIES` answers,
one row per requirement with its ISO 19005 sections and the reason the two halves differ; the
seventh distinction the catalogue lists, `/CIDToGIDMap`, is deliberately absent because it differs
by *base edition*, which the target qualifier already says.

Two consequences, and both are the point:

- **A shape no requirement splits into is an error naming both**, as an unknown target already was —
  `ConfigError::UnknownShapeQualifier` prints the shapes the site does have. Before this, such a row
  was read, validated and silently inert, which is the failure mode ADR 0954 exists to remove.
- **A row naming the half the converter answers applies**; a row naming the other stays inert,
  exactly as a row for another target does. This is safe by construction rather than by care: no
  split requirement is an `Answer::Loses` row, so no shape-qualified row can authorise a loss, and
  `Configuration::unbuilt` names an applying row whose remedy has no code behind it.

**`--remedy-sites` does not yet enumerate the shapes**, which `doc/pdf-a-mitigations.md` section 14
already records as owed. The listing's `Site` struct was being edited by another round in the same
worktree, and a one-line column added underneath that is a conflict rather than a feature; the
error message is the discovery path meanwhile, and the column is the next converter round's.
