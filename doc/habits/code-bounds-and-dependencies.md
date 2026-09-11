# Habits: code, bounds and dependencies

Status: **standing** — method, not code.
Read by: a round that writes code, sets or lifts a bound, or takes a dependency. `doc/stack.md` and
`doc/third-party-data.md` are what a dependency has to be before it is trusted.

`doc/habits.md` is the index of the six, and it states what a habit is and what each keeps.

- **Adding a `use` lints lines this round never opened.** Importing `Object` into a test file turned
  seven pre-existing fully-qualified paths into `unused_qualifications`, which under
  `RUSTFLAGS="-D warnings"` are failures in code the round did not write (ADR 0992). That makes
  three ways a round edits files it did not open — a `use`, an unscoped `cargo fmt --all`, and a
  shared `mod.rs` — and the first is the one with no command behind it to blame.

- **A gap inside a feature you have implemented does not announce itself.** Every missing
  *subsystem* reports, because whoever decided not to build it wrote the report. **A fast path
  inherits none of the rules of the path it skips.**
- **A "nothing here" is data, and dropping it is not the same as recording it.** §7.5's free
  entries and §7.5.8.3's unknown entry types both say an object number names nothing; both were
  *skipped*, so the question fell through to an older section and the reader resurrected objects
  its own file had deleted. **Ask what a `continue`, a dropped branch or an unmatched arm hands the
  question *to*.** ADR 0100.
- **A refusal is not a repair, and the difference is invisible from inside the function that
  refuses.** `Document::load` would not hand back an object whose header named a different number,
  which is right — returning object 2's bytes under number 3 corrupts the graph silently. What it
  handed the question to was the page-tree walk, which found object 3 was not a `/Type /Page`,
  skipped it, and returned the *next* kid: `issue7229.pdf`'s page one was its page two for the
  project's whole life, with `Pages::len()` answering 2 from `/Count` and `get(1)` answering
  `None`. **Every correct local refusal is a question passed upwards, and the caller may answer it
  by drawing something.** ADR 0148, and it is ADR 0100 one level along.
- **A partial repair can be worse than none.** The first version of that fix recovered *in-use*
  entries one at a time and left the misfiled free entry standing, so the page's image became a
  deletion and the page drew **nothing** where it had drawn the wrong page. A displacement is a
  property of the subsection; repairing half of one is a new file nobody wrote. Ask what class the
  defect belongs to before choosing the granularity of the fix.
- **The archetype is the `d` operator.** Every layer of dashing existed and one line read only the
  *empty* array, so not one dashed line in 974 documents. When a feature looks finished, check the
  operand path from the content stream to the state. **A feature switched off in one place is
  switched off everywhere it is not switched on**, and **a clause whose operators are implemented
  can still be unread** (`J`/`j`/`M` from the first commit; Table 57's `/LC`/`/LJ`/`/ML` for
  twenty-three sessions).
- **A lookup table with a deliberate many-to-one entry has no inverse, and reading one backwards
  fails in the direction nothing checks.** `Pages::indices` answers *object → index* and holds an
  entry for an intermediate `/Pages` node as well as for each page, because a destination may name
  a node — its own doc comment says so. Three call sites wanted *index → object* and got it by
  scanning the map for the first matching value, so on a document whose node has the lower object
  number the answer was a node that is not a page: every Table 355 `/Pg` comparison failed and page
  one of ten tagged documents, ISO 14289-1 among them, told a screen reader the page has no
  structure. **The map answered, the answer was well formed, and it named the wrong kind of
  object** — which is why no test and no report saw it for the whole life of the code. Ask what a
  map's entries *mean* before reading one the other way, and take the identity from whatever states
  it (`Page::id`, here). ADR 0342.
- **A cache that reports a perfect hit rate can still be missing.** `render-cpu`'s mask cache
  answered every one of the 303 lookups page 6 made and built 303 identical page-wide masks,
  because the key was the leaf's `ClipId` — a *name* — and the page states one region. **Instrument
  the count of distinct keys, not the hit rate**: a hit rate is a statement about the lookups you
  made, never about the ones you should have made. ADR 0132, and it is ADR 0115 with the sign
  reversed — that key was too weak, this one too strong, and both ask whether the key is what the
  claim is about.
- **A count of what is *shared* is not a count of what can be *reused*.** 5933 fills of 107
  outlines said a coverage cache would hit 55 times over; the outlines are shared through an
  `Arc` and the coverage is not shared at all, because the sub-pixel phase the count left out is
  what a coverage bitmap depends on. **Ask what the cache's key would have to be before believing
  the count.** ADR 0131.
- **A cache is a claim that two things are the same, and the currency of the claim is the key.**
  The font cache said it in the weakest one available — a resource name, which §7.8.3 scopes to the
  dictionary that defines it — and handed a form `XObject`'s `/F1` the page's glyphs for
  thirty-one sessions. Every other cache keys on object identity. ADR 0115.
- **A display list holding the right commands can still draw nothing, and no report will say so.**
  A type 5 mesh was complete, correct and 180 points from where it belonged. Between "we could not
  build it" and "we drew it" there is a third state only the oracle catches.
- **A representation can forbid a correct answer.** No evenly spaced array of colours can express a
  discontinuity. Ask what a data structure *cannot say*.
- **A file's extension is a claim, and the bytes decide.** PDFium ships the standard 14's Foxit
  faces as `.pfb` and every one of them begins `01 00 04 02`, which is a CFF header and not
  PostScript. Four lines of `xxd` settled what a module comment would have got wrong. ADR 0133.
- **A dependency's refusal can be silent *and* size-dependent.** `tiny-skia` insets the clip by a
  pixel before hairline stroking and returns early when the inset is empty, so a hairline stroke
  into a target under three rows tall draws nothing and reports nothing. Found by a test that had
  passed for a hundred and fifty sessions failing at one of its three scales — trap 12b's question
  ("what *size* is every case in this suite?") arriving from the other direction. ADR 0139.
- **A probe is a suite, and a suite of one shape proves one shape.** ADR 0138 split a page with a
  cubic in it and concluded "a clipped line is the same line"; a quadrilateral took ten minutes and
  said otherwise, which moved the rule and doubled what it permits. ADR 0139.
- **A parser that recognises a delimiter without parsing it will be read as parsing it.**
- **An operator that is matched and ignored may still be a rule.** `BX`/`EX` sat with `MP`/`DP` for
  thirty-one sessions; §7.8.2 makes them the one place an unrecognised operator is not an error.
- **Where a clause states arithmetic exactly, two independent implementations are worth more than
  one shared one** — trap 2 sends a device *decision* to the shared crate; §11.3.5.3's formulas are
  the other kind. **Two rasterisers disagreeing is information; two agreeing is not proof.**
- **An assumption a test cannot exercise is not tested, however many tests run over it.** The GPU
  backend demultiplied Vello's output for fifteen sessions; every scene rendered onto an opaque
  background.
- **Ask which arm of your own enum no test has ever taken.** `Rendered` has a variant for a
  tier-1 host and one for a tier-2 host; twelve tests played tier 1 and the tier-2 path asked
  for the same frame for ever. The variant existed, the doc comment explained it, and nothing had
  ever sent it. ADR 0117.
- **A number computed to fit must be checked against the rounding of whatever consumes it.** A
  page fitted to a window by `viewport / extent` is one pixel too tall about half the time,
  because `TargetSpec::for_page` rounds a raster *up* to contain the page and the nearest `f32`
  to the exact ratio is above it as often as below — a fitted page with a scrollbar. The fix is
  not an epsilon: step to the next representable scale until the consumer's rounding lands. ADR
  0116.
- **Two copies of a constant is one defect waiting.**
- **A constant that is a property of the state must reach every paint, including the ones that
  replace the colour.** A shading replaces the current colour, and the line that returned it
  dropped `ca`.
- **A clamp is a decision.** `width.max(0.0)` reads as hygiene and was this program's whole answer
  to a value §8.4.3.2 forbids. Ask what a `max`, `clamp` or `unwrap_or` *decides*.
- **A fallback that fills the page is worse than one that leaves it blank.** "If nothing else
  matched, the code is the glyph index" drew `v 0' ' W` for `What's an interval?`. **What makes a
  fallback legitimate is where the answer comes from, and it is measurable**: §9.10.2's permission
  is taken by asking the *program* what it drew, and the readback rose 96.5% → 97.8% with **no
  document moving the other way**. A fallback that invents text lowers a score somewhere.
- **An optional entry must not erase what the clause states**, which is now four ADRs: a line
  ending (0106), an `/Encoding` name Table 112 does not permit (0111), a `/DA` font `/DR` lacks
  (0112), a missing `/BBox` or `/Rect` (0113, 0114). **And a stand-in may not fall short** — the
  first version of ADR 0112 drew six dots of Arabic punctuation on an otherwise empty page.
- **A shortcut right on the common case is worse than one wrong on all of them.** The Cal-space
  pass-through was nearly correct for `/Gamma 2.2` and badly wrong otherwise, and nothing
  distinguishes the two at runtime.
- **Silent caps are defects, not safety**, and **a bound written for the pathological case can
  refuse a reasonable one** — the bound belongs on the *growth*.
- **A panic in a dependency is a symptom, not a diagnosis**, especially where its arithmetic is
  modular. **Being right for the wrong reason is worse than being wrong.**
- **A dependency can be doing the thing your architecture forbids.** Trap 6 has said since the
  sixth session that `ColourSpace::to_rgb` is the only place a colour becomes RGB, and
  `zune-jpeg` was converting every four-component codestream to RGB with a formula of its own —
  reachable by any `DeviceCMYK` JPEG, invisible to `colour_paths.rs` because every fixture there
  states its samples as hex rather than as a codestream. **Ask of each dependency which of your
  own invariants it is in a position to break**, and write the fixture in the form the dependency
  actually sees. ADR 0149.
- **A dependency is a decision, and this project's own precedent decides it.** `zune-jpeg` owns
  `DCTDecode`, `skrifa` font parsing, `flate2` Flate, `tiny-skia` rasterisation. ADR 0014. **A
  dependency can implement more of a specification than the clause cites** — `read_fonts::ps::agl`
  gives the Adobe Glyph List *and* its specification's algorithm. **Look in `read-fonts` before
  writing font-format code**: an earlier handover specified ~80 lines of CFF charset parsing that
  already existed. ADR 0006.
- **The interesting half of a "viewer feature" is usually a clause.** Of the click that follows a
  link, the mouse is four lines and the rest is Table 176's three conditions, §12.5.2's coordinate
  space and §7.7.3.3's rotation.
