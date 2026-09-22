# 1170 — The two groups §11.7.4 builds around the special mode

Status: accepted. Session 1166.
Builds on: ADR 1157, which built the mode, and ADR 1158, which named these two constructions as
reported rather than built and derived when each collapses.
Depends on: ADR 1169, without which either group parts §11.4.7's page pair.
Context: `crates/pdf-model/src/content/overprint.rs`, `crates/pdf-model/src/content/path.rs`,
`crates/pdf-model/src/content/text.rs`, `crates/pdf-model/src/content.rs`,
`crates/pdf-model/tests/overprint.rs`.
Clauses: ISO 32000-2 §11.3.3, §11.4.4, §11.4.5, §11.4.6, §11.6.4.4, §11.7.4.3, §11.7.4.4.

## 1. §11.7.4.3's last paragraph

> If the current blend mode is any mode other than Normal when invoking this special
> overprinting blend mode, the object being painted shall be implicitly treated as if it were
> defined in a non-isolated, non-knockout transparency group, and painted using the this special
> blend mode. The group's results shall then be painted using the current blend mode in the
> graphics state.

The display list already has the command: `Command::Group` with `isolated: false` and
`knockout: false`, which `render-cpu` draws by §11.4.4's own model — the elements onto a copy of
the backdrop, then a second run onto transparency for Table 140's group alpha and NOTE 3's
removal (`blend::remove_backdrop`, ADR 1107). So the object is painted with
`BlendMode::Overprint` inside a group whose own blend mode is the document's.

**The object keeps §11.6.4.4's constants and its soft mask; only the blend mode moves.** The
clause moves the mode and says nothing of either, and its NOTE 3 is what checks the reading:

> It is not necessary to create such an implicit transparency group if the current blend mode is
> Normal ; simply substituting the special blend mode while painting the object produces
> equivalent results.

With the constants left on the object, a non-isolated group composited onto its own backdrop at
an alpha of 1.0 under Normal returns its elements unchanged (§11.4.4 NOTE 3), so NOTE 3's
equivalence is exact. With them moved to the group it is not — the two differ wherever the
backdrop is not opaque — so the reading that makes the clause's own note true is the one built.

## 2. §11.7.4.4's first bullet

ADR 1158 §2 derived two cases where this bullet needs nothing built, and both hold unchanged: the
special mode equal to `C_s` in every component makes the two bullets one picture, and a pair whose
two constants are 1.0 under Normal *is* the bullet's group by §11.4.4's cancellation. What is
left is a pair whose constants or mode are not the identity, and that group is now emitted.

**The constants are lifted before the parts are painted**, because a paint carries the constant
inside its colour — `GraphicsState::solid_fill` multiplies it into the alpha, and
`Shading::with_alpha` reaches every colour of a shading pattern.
`GraphicsState::with_opaque_constants` is the lift and it changes those two numbers and nothing
else; §11.7.5.2's opacity conditions are
still asked of the state the object *composites* under, because the clause's question is about
what covers a point on the page.

**The soft mask stays on the parts.** The bullet's nouns are the alpha *constants* — "the current
stroking and nonstroking alpha constants are equal", "with an alpha value of 1.0", "using the
originally specified alpha" — and §11.7.4.4's second bullet spends them the same way, its own
group carrying neither constant nor mask while the parts keep theirs. The collapse condition is
unaffected either way: a group at an alpha of 1.0 under Normal is the identity whatever the parts
carry.

## 3. §11.7.4.3 is not owed on top of §11.7.4.4

A pair that goes to §11.7.4.4's second bullet has parts painted "with their respective prevailing
alpha constants and the prevailing blend mode", and a part may carry the special mode there — the
bullet is reached when `/OP` and `/op` disagree, or when the two constants do. Whether
§11.7.4.3's last paragraph then wants a further group around it looked like an open question and
is not one: §11.7.4.4 opens by saying §11.7.4.3's considerations "also affect those path-painting
operations that combine filling and stroking a path in a single operation", makes the combined
fill and stroke "a single graphics object", and then states the construction exhaustively —
"This implicit group is established and used as follows", two bullets and an "[i]n all other
cases". The prevailing blend mode goes where the second bullet puts it. There is no third group.

## 4. The one position that still reports, and why it is §11.4.6's

`Command::Group`'s `isolated` is `false` only where no enclosing group is a knockout group, and
§11.4.6's NOTE 6 is about this exact nesting: "When a non-isolated group is nested within a
knockout group, the initial backdrop of the inner group is the same as that of the outer group;
it is not the immediate backdrop of the inner group." No command of this list can hand that
backdrop over. Two sub-cases, and only one of them is a gap:

- **The enclosing knockout group's initial backdrop is transparent.** NOTE 6 hands the inner
  group that same backdrop, so §11.4.5's isolated group *is* the clause's non-isolated one,
  exactly — the backdrop composited in and removed again is nothing either way.
  `Interpreter::transparent_initial_backdrop` already answers that question for §11.6.6's
  caller, and the group is emitted with `isolated: true`.
- **A direct element of a non-isolated knockout group.** That is reported by name
  (`Unsupported::Overprint`), and the object is painted under the document's own mode.

The second is a restriction on **what an element of a knockout group may be**, which §11.7.4.4's
ledger row already carries among its own unstatable cases ("a non-isolated group used as a part
under the shape reading") and which `implicit_knockout_group` now enforces rather than assumes
(ADR 1169 §3). It is not a requirement of §11.7.4.3 left undone, which is why that row moves to
`implemented` with the position named in its note. A round that lifts §11.4.6's restriction
removes the report; a round that disagrees with this division should say so here rather than
move the row quietly.

## 5. What the constructions cost, and what they moved

- **The default path, and it is not free.** Page 101 of ISO 32000-2 — the tree's densest text
  page — interpreted fifty times under callgrind, against the same build with §11.7.4's
  decisions and constructions planted away (Table 57's three lookups per `gs` and the tints
  beside every colour stand in both arms, ADR 1158 §3 having priced those at 38 531
  instructions): **+2 788 934 instructions, 0.223%** of one interpretation. Of that, 513 947
  (0.041%) is everything but the wrap point — the two `overprint_blend` calls per painting
  operator, `combined_overprint`, `implicit_group_owed` taken by value behind the caller's own
  blend-mode test, and the element check in `implicit_knockout_group` — and the rest is a guarded
  call standing in `show_text`'s glyph loop, which costs the registers spilled around it in a
  function that large. Three shapes were measured rather than one: `implicit_group_owed` taking
  its pair by *slice* cost 4 879 021 (0.391%) because the array was built before anything was
  tested, and the glyph wrap inlined as a three-armed match cost 3 163 973 (0.253%). The numbers
  sit in `implicit_group_owed`'s and `Interpreter::wrap_in_the_implicit_group`'s own comments,
  which is where `CLAUDE.md` asks for a benchmark to sit. **What is left is the price of
  building the clause at all**: the only shape that costs 0.041% is the one that does not wrap
  a glyph.
- **The corpus.** `issue12798_page1_reduced.pdf` stops reporting and `MAX_INCOMPLETE` goes back
  to 61. It also stops drawing the way it drew: its overprinting mark is `0 0 0 1` under
  `/BM /Multiply`, so the special mode leaves all three channels of the chromatic raster to the
  backdrop, and the group's result — the backdrop — is then multiplied *with* that backdrop where
  the object used to be painted under Multiply directly. That is the clause's own construction and
  it is why NOTE 3 exempts only a Normal current mode.
- **Four fixtures**, three in `tests/overprint.rs` derived from §11.3.3, §11.4.4 and Table 146
  and one in the confined protocol for the eight subsets a command can now carry, each
  calibrated by planting its own construction away (trap 13). Two of them name a *page* rather
  than a number —
  the colour §11.7.4.3's bullet computes, painted directly — so they hold whatever the arithmetic
  of a blend mode over subtractive components turns out to be.
