# 655 — Three clauses that were said to meet at one operator

The round was told that a shading pattern's colours are resolved at the `scn` rather than at the
mark, that §8.6.5.9's black point and §11.4.7's compositing target are resolved there too, and that
moving all three was one round's work. **Two of those three sentences are wrong.** The clauses name
three different moments, and the one the previous round called "the black point" is *earlier* than
the `scn`, not later — so the repair everybody had priced would have made it worse.

Date: 2026-08-22.
ADR: [0483](../adr/0483-three-clauses-that-were-said-to-meet-at-one-operator.md).

Touched: `crates/pdf-model/src/content.rs`, `src/content/pattern.rs`, `src/content/colour.rs`,
`src/content/run.rs`, `src/content/transparency.rs`,
`crates/pdf-model/tests/rendering_intent.rs`, `tests/transparency_groups.rs`,
`crates/pdf-model/examples/pattern_state_census.rs` (new),
`doc/conformance/ledger.toml` (§8.6.5.9, §8.7.4.1, §8.7.4.2, §10.5, §11.4.7, §11.6.7),
`doc/todo/13`, `doc/todo/README.md`, the ADR and this file.

## What the three clauses say about *when*

Read whole from `doc/md/`, and the sentence that decides it was in neither of the two rows that had
been arguing about the question.

**§11.6.7** makes a shading pattern's definition an implicitly enclosed group and states outright
which graphics state it is evaluated under:

> The definition shall not inherit the current values of the graphics state parameters at the time
> it is evaluated; those parameters shall take effect only when the resulting pattern is later used
> to paint an object.

> Any parameters that are not so specified shall be inherited from the graphics state that was in
> effect at the beginning of the content stream in which the shading pattern is set to be the
> current colour in the graphics state or in which the sh operator is used.

> In the case of a shading pattern, the parameter values may be augmented by the contents of the
> ExtGState entry in the pattern dictionary (see 8.7.4, "Shading patterns"). Only those parameters
> that affect the sh operator, such as the current transformation matrix, black point compensation
> and rendering intent, shall be used.

Table 75 says the same in its own words. So **§8.6.5.9's black point** — and §8.6.5.8's intent, and
§10.7.3's smoothness by the same test — come from the *beginning of the content stream*, augmented
by the pattern's own `/ExtGState`. Neither the `scn` nor the mark.

**§11.4.7's compositing target** comes from the mark's group, by two sentences rather than one:
§11.6.7 makes the pattern's implicit group *non-isolated*, and §11.7.2 says "[n]on-isolated groups
shall inherit their colour space from the nearest ancestor isolated parent group" — which for a
pattern painted inside a group is that group.

**§10.5's transfer function** is the only one of the three whose answer is the mark. §11.7.5.2 puts
it at "the last (topmost) elementary graphics object enclosing that point", and §11.7.5.3's NOTE
takes it out of the group evaluation altogether — "whose values are used only when all colour
compositing has been completed and rasterization is being performed" — so a `/TR` in a pattern's
`/ExtGState` says nothing about the pattern's own colours.

**And §11.6.7's sentence names the `sh` operator too, which this round read and did not apply.**
Taken literally it would make a `ri` earlier in the same stream unreachable by the `sh` after it;
§11.7.5.3 gives an elementary object "the current rendering intent in effect in the graphics state
at the time of the painting operation", and Table 76 puts an `sh`'s coordinates in the current user
space. `a_shadings_ramp_honours_the_rendering_intent` already held that half and still does.

`spec-errata emit` over all fourteen documents, clauses 8 and 11 first: **no annotation falls in
any of §8.6.5.9, §8.7.4.1, §11.4.7, §11.6.7, §10.5, §11.7.5.2 or §11.7.5.3.** §8.6.5.8's one
strikeout was the only near miss and its text is in §8.6.5.8's own NOTE, checked against `doc/md/`
rather than assumed from the page a heading opens.

## The site this tree had already got right, for one parameter of three

§8.7.2 maps a pattern's matrix to "the default coordinate system of the pattern's parent content
stream" — `Interpreter::base`, swapped at each of the four ways of becoming a parent since the
fifty-second session. That is §11.6.7's *first named parameter*. The other two had never been
connected to it.

So the general shape is trap 5's, stated over a clause instead of an entry: **where one rule
governs a set of parameters, implementing it for one of them is the failure mode that reports
nothing.** `PatternInitial` is `base`'s companion and is scoped in `run` the same way.

## The population, measured before the code

`examples/pattern_state_census` is new; each condition is the clause's own.

- **`doc/pdf.js`**: 964 open, 38 hold a `/PatternType 2` object (601 patterns), **0** state Table
  75's `/ExtGState`, **0** can see the black point move, 2 hold a four-component group `/CS`.
- **The crawl**: 65 703 open, 1504 hold one (36 527 patterns), **42** state an `/ExtGState`, **0**
  can see the black point move, 211 hold a four-component group `/CS`.

**The ledger's §8.7.4.1 row had claimed since the six-hundred-and-sixteenth that no corpus document
writes an `/ExtGState`, and both of its measurements still hold** — over `doc/pdf.js`'s 974 and
`doc/corpora/`'s 275. The crawl was in neither, and it holds 42, whose keys are `/SM`, `/RI`,
`/HT`, `/OP`, `/op`, `/OPM`, `/SA`, `/BG2`, `/UCR2` and one dictionary's `/BM /CA /ca /SMask /AIS
/TK`. A negative claim about a population decays when the population grows.

**The black point's population is zero in both**, and that is said plainly rather than hedged:
compensation moves only an ICC conversion, so a pattern shading in a device space cannot see the
parameter however the state moves. The fixtures are hand-built (trap 8).

## What was built

- **`content::PatternInitial`** — black point, intent, smoothness — taken from the state each
  content stream begins with (`run`), augmented by Table 75's `/ExtGState` (`/UseBlackPtComp`,
  `/RI`, `/SM`), and read by `pattern()` in place of the state at the `scn`. `content/pattern.rs`
  carries which bucket each of Table 57's other entries falls into and the sentence that puts it
  there, so that nobody has to ask why three are read and twenty are not.
- **`group_press` gains one condition**: a group a shading pattern is carried into over the `Do`
  does not composite in a press of its own. It is the same argument the function already makes for
  §8.6.8's uncoloured cell — a colour resolved outside cannot be reinterpreted as ink inside — and
  the group keeps §11.6.6's standing report instead of drawing an ink nobody stated. It comes off
  when §10.5's rebuild lands, and the ADR says so.
- **`Interpreter::note_black_generation`**: a pattern dictionary is the second of the two routes
  §8.4.5's parameters have to a page, and `/BG2` or `/UCR2` stated there now reaches the same
  page-wide flag `gs` already sets.
- **Three tests**, each run against the defect it guards (trap 13). With the resolution back at the
  `scn`, both `rendering_intent.rs` tests fail — and in the first fixture the graphics state does
  not move between the `scn` and the `f`, so that arm is *both* of the obvious designs at once,
  which is the point of the fixture. With `group_press`'s condition removed,
  `a_shading_pattern_carried_into_a_press_keeps_the_group_on_the_device` fails.

## What moved, and it is one page

`examples/raster_digest` on both arms, which is trap 1's instrument rather than a gate summary:

- **`doc/pdf.js`'s 974 first pages are byte-identical.**
- **Of the 251 crawled documents the census named, one differs**: `6696799.pdf`, whose shading
  pattern's `/ExtGState` states `/SM 0.002` over a 256-sample type 0 function. §10.7.3's tolerance
  now asks for the finer sampling the file asked for. **6688 pixels of 3 601 584, by at most one
  level of 255**; ink 16.4368 either way. The page is a newspaper spread and was opened rather than
  assumed. Nothing else on this disk moves, so `doc/todo/00`'s step 7 is not owed.

## The ledger

Six rows. §8.7.4.1 loses the claim that `/ExtGState` is unread and gains the crawl's 42. §11.6.7
gains the whole reading and the two clauses' worth of it that is implemented. §8.6.5.9 gains a sixth
correction to a row that is entirely about which object a setting belongs to. §11.4.7 gains the
refusal beside the two it already listed for the same reason. §8.7.4.2 records why the `sh` operator
does *not* move. §10.5 loses the sentence that said closing its half "means resolving a pattern's
colours at the mark … so it is three clauses' question" — it is one clause's question now, and the
price is the same hundred lines with a warning attached: a rebuild at the mark must keep reading
`PatternInitial` for the other three parameters or it trades one departure for another.
