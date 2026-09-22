# 1187 — Three premises expired, and a subtype was checked against its clause

The writer round of batch twenty-eight. Four contract items, three of them premises
`doc/todo/65` recorded as expired; all four closed, one by finding the thing asked for could not
be built honestly.

## What was built

- **§7.9.6, Errata Collection 3 Issue #307's writer half.** The brief asked for a typed refusal at
  each of the four name-tree writers. There is no such refusal to build: every writer's keys are
  `Vec<u8>` and leave as `Object::String`, so `Object::Null` cannot reach a key position and the
  variant would be dead code claiming a failure mode. The prohibition is one function's signature
  instead — `filing::tree_root` is now the only place this tree writes a `/Names` node, and `merge`
  and `split` had copies of the loop — with three end-to-end tests that a *source's* null key does
  not cross a merge, a split or an attach. ADR 1211.
- **§14.3.3 and §14.3.4, the merged `/Info`.** `MergePlan::information` and `--info Key=value`,
  validated and built by the same two `update` functions so Table 349's types are read once. The
  empty default is unchanged and now has `A55`'s reason rather than ADR 0821's retired one: a
  derivation is never a default. Where a stated entry is a date, §14.3.2's packet is written beside
  the dictionary naming the same instant, because §14.3.4's first rule binds a processor creating a
  new document and a merge is one. ADR 1212.
- **`Qualifier::Shape` selects.** `decision::SHAPES` enumerates the seven split requirements'
  halves and which half `REMEDIES` answers; a shape outside a site's is an error naming both, the
  answered half's row applies, the other stays inert. ADR 1211 section 2.
- **§12.5.6.2's `/ExData`, checked rather than restated.** Table 173 gives `MarkupGeo` a `/Type`
  and a `/Subtype` and states *This Subtype does not define any additional entries.*; §12.10 names
  the subtype nowhere at all. Nothing is owed, no reader was built (it would have no consumer), and
  the row's note now lists what actually keeps it `partial`: `/Subj`, `/CreationDate`, `/DS`.

## Rows and what is left

§7.9.6, §14.3.3 and §14.3.4 stay `implemented`, code and tests named; §12.5.6.2 stays `partial`
with its debt itemised and the `/ExData` claim checked against §12.10.

`--remedy-sites` still does not enumerate the shapes — the `Site` struct was being edited by
another round in this worktree and a column added underneath that is a conflict, so the error
message is the discovery path meanwhile. `pdf-transform`'s `structure.rs` writes §14.7.5's
`/IDTree` with the same key type and is under the same guarantee without calling `tree_root`; a
round that touches it should route it through.
