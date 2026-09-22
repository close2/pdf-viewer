# Habits: code, bounds and dependencies

Status: **standing** — method, not code.
Read by: a round that writes code, sets or lifts a bound, or takes a dependency. `doc/stack.md` and
`doc/third-party-data.md` are what a dependency has to be before it is trusted.

`doc/habits.md` is the index of the six, and it states what a habit is and what each keeps.

- **A parameter that changes what a value means travels *with* the value the routes already share,
  never beside it on one signature.** §14.11.5's output intent was read by `k`, `rg` and `g` and by
  nothing else that selects a device space, then carried to `cs … scn`, then found missing at four
  more sites. The fix that held was putting it into `Conversion`, which every route already passed —
  so omitting it became a type error rather than a habit (ADRs 1001, 1008). One grep for the field's
  name is the audit.

- **Measure a per-operator `Box` clone before calling it a refcount-shaped cost.** 4,000 `k` fills
  under a real 718 KB CMYK profile: 1,799 M instructions, 95% of them `memcpy`, against 49.8 M with
  an `Arc` — 85–100 ms against 9.3 ms (ADR 1008). The number goes on the variant, per principle 2;
  without it the `Arc` would have been an assumption dressed as an optimisation.

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

- **A scratch checkout that shares the build directory poisons it for everyone the moment it is
  deleted.** Round 1041 ran its gates in a private worktree under its scratchpad, sharing
  `/home/AI/cargo-target/pdf-viewer` with the five rounds beside it, then removed the worktree.
  Cargo had run `pdf-font`'s and `pdf-sandbox`'s build scripts from *that* checkout, and their
  outputs record the path they read (`cargo::rerun-if-changed=…/scratchpad/r1041/…/data/cmaps`);
  the next `cargo check` in the shared tree re-ran them against a directory that no longer
  existed and the whole workspace stopped building for a reason no source diff could explain.
  `cargo clean -p pdf-font -p pdf-sandbox` was the repair, 2.6 GiB of it — and then `-p conformance`
  as well, because a *test binary* bakes `CARGO_MANIFEST_DIR` at compile time the same way: four of
  its unit tests opened the deleted checkout's `Cargo.toml` and reported the workspace unreadable,
  while a `nextest` run made before the deletion had reported them passing. A stale binary passes
  until the path it remembers is gone. And `cargo clean -p` cleans **one profile**: the `gates` and
  `release` profiles each keep their own build outputs, so the merge's tier 2 and 3 — every line
  under `--profile gates` — failed 30 of 31 on the same panic after `debug` had been cleaned.
  `cargo clean --profile gates -p …` and `--profile release -p …` too, then
  `grep -rl <the dead path> <target>/*/build/*/output` until it prints nothing.

  The same directory has a second way to lie, seen at the merge of sessions 1062-1067: the FFI
  tests' nested `cargo test --no-run --package viewer-ffi` failed compiling `pdf-model` against a
  `pdf_render` with no `TilingType`, while the workspace build had just compiled the same crates
  green. `-v` showed the cause in one word — `Fresh pdf-render` — cargo reusing, for that one
  `--package` graph, an rlib built before the module existed. No feature, no path: a fingerprint
  miss. `cargo clean -p pdf-render` was the whole repair. So when a nested or `--package` build
  disagrees with the workspace build about what a crate exports, ask `-v` whether the crate is
  `Fresh` before reading any source.

  `tools/worktree.sh open` exists so that this cannot happen: every worktree it makes gets its
  own `target-dir` in `.cargo/config.toml`. A checkout made any other way must do the same or
  must not be deleted while anything else builds. `tools/round.sh` already checks for a build
  script baked against a checkout that no longer exists — this is what that check is for.
- **Never restore a whole file in a shared worktree, not even one you are editing.** Round 1020 made
  a `cp` backup of `crates/pdf-signature/src/signature.rs` before planting a calibration defect and
  copied it back afterwards — the ordinary, careful thing to do alone. A sibling round was editing
  the same file in that window, and the restore put the file back as it stood *before their edit*.
  The symptom was legible and nobody would have read it correctly: a variant
  (`Excluded::TheDigitsOfTheSignatureValue`) appeared in one round's report while its declaration
  was missing from the file, then resolved when its author wrote it again. Neither round could have
  proved what happened from its own side; it was found only because 1020 said out loud that it had
  done it.

  A whole-file restore is not an edit, it is a **claim that the file has no other author**, and in a
  shared worktree that claim is false. To plant a calibration defect (trap 13), make the smallest
  edit that plants it and reverse *that edit* — by hand, or with a patch of the region — never by
  putting a saved copy of the file back. The same holds for `git checkout -- <file>`, `git stash`
  and `git restore`: each of them restores a whole file from a state that predates whatever a
  neighbour has written since. The existing rule against `git stash` in this tree is the special
  case; this is the general one.

## A child nobody waits on is a task the cgroup still counts

`std::process::Child` does not wait on drop. A worker dropped alive — a handshake that timed out or
greeted wrongly, a connection replaced without a `wait` — runs until its pipe closes and then sits
as a zombie of a parent that never collects it; a zombie holds no memory and one pid, and the pid
is what the scope's `pids.max` counts. Three crawl censuses leaked ten thousand of them on
2026-09-15 and every fork in every Konsole tab returned `EAGAIN`, which killed the agent itself.
The rule: a type that owns a `Child` implements `Drop` as kill-then-wait (`pdf_sandbox::Connection`),
and a test plants the leak with a worker that stays alive after being refused
(`crates/pdf-sandbox/tests/reaping.rs`). Watch `ps -eo stat | grep -c ^Z` during any crawl walk.


## A writer of a construct the reader already reads owes a mirror, not a second reading

Twice while §7.6 was built on the way out the defect would have been a writer making a choice the
reader would not make: a `/Crypt` filter carried from a source, which this reader resolves to
`Identity` while the writer would have encrypted the stream under `/StmF` anyway, and §7.5.7's
amended shall-not list (Errata Issue #439), recorded as satisfied only because no encrypted output
existed. Write down what the reader does with the bytes about to be emitted, and make the writer
ask the same questions in the same order — `Protected::method` mirrors `Document::stream_method`
(ADR 1161).

## A second policy with four levels shares the words only if it shares the direction

`--restrictions=` runs `off` to `on` with `off` the permissive end, because its subject is a
restriction a document asserts; a link's policy has its permissive end at `open`. Reusing
`off|on|ask|warn` would have made one word permissive in one policy and restrictive in the other.
Ask what the most permissive value is called in each before reusing a vocabulary (ADR 1155).

## A check that moves onto the launch path pays for every source it asks

`restriction::asserted` was correct and cheap for five hundred sessions because every caller was a
gesture; the moment `Viewer::open` became a caller it was a §12.7.4 field-tree walk per document,
and nothing failed because the answer was identical either way. When a function's *caller set*
changes rather than its body, re-read what it reads — and run `launch_path`, which is the only gate
that can see it (ADR 1167).

## A batch of edits tags each edit with the rewrite that asked for it

The converter's preparation is computed from the validator's failures and never sees the
requirements the caller departed from; only the rewriter's own gate keeps a departed requirement's
edits from being applied. Anything that produces a batch of edits therefore tags each with the
rewrite that asked for it (`actions.rs`'s `Edit::by()`) rather than gating the batch as a whole —
a batch without it silently carries out a departed row (ADR 1175).

## A flag set where a decision is made outlives the decision

`DisplayList::overprints()` was set where §11.7.4.3's mode was chosen, and three shapes put the
verdict on pages with no such mark: a part that never paints, a text mode or hidden layer that
marks nothing, and a whole run of content interpreted and then discarded by a re-run. No
emission-site discipline sees the third. When a flag decides what a backend does with a list,
settle it against what the list holds, once, where the list is finished — `noninvertible_marks`'s
shape — and gate the walk on the cheap over-approximation so the default path pays one branch
(ADR 1181).

## A reference count is not an ownership proof

An object referenced exactly once can still belong to two pages, because the container that names
it may be shared: a form nested in a shared form is reached from every page that draws the outer
one. Redaction's `Walk::owns` asked the count alone and would have replaced a stranger's content
silently; it now asks what reaches the container as well (ADR 1196). Any code deciding "this object
is mine to overwrite" from a count owes that second question.

## An absent input and an empty output are different facts

§12.9.2's `format` answers an optional array that is absent with `""`, correctly at its own level;
composed into a measurement it showed a slope of nothing beside the word *slope*. When a reader
composes several optional clause-derived strings into one answer, only the absence of the input
belongs in the answer (ADR 1191).

## Before building a refusal, check whether the type already refuses

Errata #307's `shall not` on a null name-tree key asked for a typed refusal at four writers; every
writer builds its keys from byte strings, so no caller has an `Object` to put in a key position and
a `Refusal` variant would be dead code with a doc comment claiming a failure mode that does not
exist. The rule got one home instead (`filing::tree_root`) and tests that a source's null key does
not cross a merge, a split or an attach (ADR 1211). A `shall not` addressed to a writer is answered
by the type the writer's keys have.

## A predicate borrowed from a neighbouring function answers that neighbour's question

`settled_over` reused `group_alpha_is_shape`'s element test because the two looked like one
question; the borrowed arm's `false` for a `Command::Shaped` was a decision about §8.5.4's exactness
silently imported into §11.6.4.3's reading, while the doc comment beside it argued the opposite of
what the code did. When a doc comment and a borrowed predicate disagree, the predicate is answering
another clause (ADR 1205).

## A message that names a remedy is checked against the code that would carry it out

Five messages named a `--font` flag the program refused as a usage error (ADR 1200); once the flag
existed, two of them still named it for cases it structurally cannot reach — a bare-CFF `Type1`
dictionary and a composite font (ADR 1209). `tools/state.sh flags` catches the first shape; the
second is read by hand at the call site that prints the message.

## Before recording that a dependency cannot do something, read its public API in the pinned checkout

§8.9.6.4's residue was "the JPX decoder's eight-bit hand-off" for seventy sessions; the decoder had
exposed each component's samples at the codestream's precision the whole time, and this tree's own
sandbox already used that path for palette indices. The narrowing was ours, at two named places
(ADR 1232's finding). Trap 40 says a refused capability may be forty lines above the refusal; this
is the same failure pointed at a dependency, and the convenience path is not the API.

## A window that says something about an event owes the reply as well as the sentence

`quorra-confined` printed a good refusal for a file a document asked for and never sent
`Command::Supply`, so the worker kept the action pending for ever and the person's click reported
nothing — the loud sentence was on the wrong side of the wire (ADR 1227). When a host declines an
event the protocol gives a reply message, it declines by sending the reply; the greppable shape is a
match arm that mentions an `Event` and pushes no `Command`.

## A configuration reader that picks the first matching row makes every later override dead text

`Configuration::read` kept the first applicable row per site, so a target-qualified row written
below its unqualified default never applied and `only-metadata-loss.toml` silently got a page where
it asked for an attachment at PDF/A-4f (ADR 1245). Where a row can be qualified, winning is a
property of the row and never of its position, and the test states the default before the
exception — the order a person writes a profile in.

## A destroy step that replaces an object checks every stream its dictionary points to

Redaction carried an image's `/SMask` and `/Mask` by reference while clearing the image, which looks
like leaving them unchanged and in fact kept the outline of the removed content in the mask
(ADR 1277). Every stream the replaced dictionary reaches is its own possible trace; read each one
against the region before calling the object destroyed.

## A profile key no reader reads makes an answer that is built count as unbuilt

`keep-everything.toml` asked for the packet rows under a `prefer` key nothing parsed, so three
answers ADRs 1245 and 1270 had built were counted among the unbuilt (ADR 1285). Before building an
"unbuilt" answer, grep the key the profile states against the reader; and a remedy that walks a
bounded `places` list checks the outcome's `total` against `places.len()` before calling itself done.

## A texture with alpha is premultiplied before a hardware filter reads it

raster-gpu filtered straight-alpha image textures with the bilinear sampler and premultiplied
afterwards, so the black stored under a soft mask's zeros leaked along its edges at every non-integer
reduction — the icon fringe the owner saw on Windows, and the long-unexplained heaviness of one corpus
page (ADR 1287). The test is a fixture whose colour under the transparent area is black, swept over
non-integer reductions against the CPU oracle; an integer-only sweep hides the leak.
