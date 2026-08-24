# ADR 0579 — The group a sentence named and nobody read, and the default a value struck out

Status: accepted, 2026-08-24. Session the seven-hundred-and-tenth, a clause round under
`doc/todo/01`, reading one family's `partial` rows against each other as well as against the code —
ADR 0538's method, now in its fifth round (0551, 0560, 0567, this). Amends §12.5.2 and §12.5.5 in
the ledger; adds `Interpreter::note_appearance_group` and its caller; adds one test to
`crates/pdf-model/tests/annotations.rs`; adds `crates/pdf-model/examples/appearance_transparency_census.rs`;
adds one section to `doc/errata-read.md` and one correction to ADR 0030. **No status moves. One
report is added, and it fires on nothing in the corpus.** Extends ADRs 0013, 0030, 0193, 0237, 0253
and 0567.

## 1. The family, by the ranking rather than by eye

ADR 0567 §1's search was run on this base rather than read off any document: for every parent whose
subtree holds two or more `partial` rows, count the five-word sequences the notes share pairwise,
keeping only sequences that at most four rows in the whole ledger carry, and rank the families by
the total.

**§12.5 heads it**, on a subtree carrying more `partial` rows than any other, with §12.8 second,
§8.11 third, §14.8 fourth and §12.8.3 fifth. Two notes about reading that output, both of which
cost this round time and neither of which was written down before:

- **The clause-level parents have to come out.** §12, §11, §8, §14, §7 and §10 sort above every
  real family, because a subtree of ninety-six `partial` rows has 4560 pairs and scores on the tail
  of them. They are aggregates rather than families and no round would read one. ADR 0567's first
  run did not say so because §12.8 happened to beat them; on this base it does not.
- **The total ranks a family and the pairs choose the reading.** §12.5's score is spread over more
  `partial` rows than a round can read properly. Its three strongest pairs are what the family was
  actually opened on: §12.5.4 ~ §12.5.6.8 at 24 shared rare sequences, §12.5.2 ~ §12.5.5 at 16,
  §12.5.3 ~ §12.5.6.4 at 15.

Each of the three is a *quotation* one round wrote into two rows. §12.5.4 and §12.5.6.8 share
§12.5.4's sentence about the four subtypes whose `/BS` is not a border, plus a census figure;
§12.5.3 and §12.5.6.4 share Errata Collection 3 Issue #34's added sentence; §12.5.2 and §12.5.5
share Table 166's sentence about `/CA` and `/ca` beside a stored appearance stream. **The second
pair was taken**, because it is the one where the two rows do not merely quote the same sentence —
they *disagree about what it leaves standing*.

## 2. What the two rows disagreed about, and which of them was right

§12.5.5's row said of its own compositing sentence — the one that composites the appearance's
group "using the values of the BM , ca and CA entries in the annotation dictionary" — that it is
**not followed**, on the ground that Table 166 forbids two of the three beside a stored appearance
and that §12.5.2 "lists all three among the keys a reader shall ignore".

§12.5.2's row, one screen away, has said since the four-hundred-and-seventeenth session that
Errata Collection 3's Issues #23 and #34 **strike `BM` out of that list** and that `/BM` is applied
on both paths. `annotation::blend_mode` carries the reading in a doc comment of its own, and
`content::annotations::draw_appearance` sets the mode into the state the appearance runs under. So
the ignore-list claim was two entries rather than three, and one third of the sentence §12.5.5 calls
unfollowed has been followed for nearly three hundred sessions.

That is 697's rule met again — *a corrected row is not a safe row* — with the direction reversed:
here the correction landed in the row that stated the mechanism and never reached the row that
depended on it. **The retired claim is a string, and the round that retires one owes a grep of the
tree rather than of the row it is editing** (ADR 0101).

## 3. The note the row cited was the wrong note, and the right one states conditions

The same row's next paragraph justifies not building a transparency group at all:

> The requirement that an appearance with no /Group 'shall be treated as a non-isolated,
> non-knockout transparency group' is satisfied by construction rather than by code: … that group
> has alpha 1, the Normal blend mode and no soft mask, and §11.6.7's NOTE 1 makes it identical to
> painting the elements directly.

**§11.6.7 is *Patterns and transparency*.** Its NOTE 1 says something else — that a non-isolated
group all of whose elements paint Normal may be *treated as isolated*, which is the optimisation
`note_group_structure`'s own comment cites it for, correctly. The sentence this paragraph needs is
**§11.4.4's NOTE 5**:

> As a result of these corrections, the effect of compositing objects as a group is the same as
> that of compositing them separately (without grouping) if the following conditions hold:
>
> The group is non-isolated and has the same knockout attribute as its parent group …
>
> When compositing the group's results with the group backdrop, the Normal blend mode is used, and
> the shape and opacity inputs are always 1.0.

The conclusion survives and the reasoning is now checkable, which is the whole difference: NOTE 5
**states its conditions** where NOTE 1 states an unrelated optimisation, and stating them is what
makes §2's finding bite. `/BM` is no longer ignored, so the second condition is the annotation's to
break: where a file states a non-Normal `/BM` beside a stored appearance, this tree paints the
appearance's elements one at a time under that mode instead of compositing the group's result under
it once, and the two differ exactly where the appearance's own marks overlap.

**A wrong citation is not a typo when the citation is the argument.** Nothing in this tree could
have caught it: `--bin tables` reads table numbers, `--bin quotations` reads quoted spans, and a
*clause* number beside a paraphrase is neither.

## 4. The `shall` nobody read, and the report it now gets

Reading NOTE 5's conditions against the code turned up the sentence the row had never quoted whole.
§12.5.5:

> If the appearance's stream dictionary does not contain a Group entry, it shall be treated as a
> non-isolated, non-knockout transparency group. Otherwise, the isolated and knockout values
> specified in the group dictionary (see 11.6.6, "Transparency group XObjects") shall be used.

**The second sentence has no reader.** `draw_appearance` runs the stream through
`Interpreter::run` directly; `transparency_group` — the one function that reads Table 145's `/I`
and `/K` — is called from `draw_xobject` and from the soft-mask path and from nowhere else. No
comment in `crate::annotation` or `crate::content::annotations` mentions `/Group` at all, and
nothing reported it. That is trap 5's shape at the scale of a clause: a page drawn to a model the
file did not ask for, with nothing saying so.

**It is reported now rather than implemented, and the choice is deliberate.** Routing the
appearance through `run_transparency_group` would put every appearance stream that states a
`/Group` — all four in the corpus, every one of them non-isolated and non-knockout — through a
second code path with its own clipping, backdrop and colour-space handling, to obtain by
construction the result §11.4.4's NOTE 5 says the current path already produces. The requirement
gain on that population is zero and the pixel risk is not. What is owed is the population where the
model genuinely differs, and that is what `transparency::note_appearance_group` names.

**Each of the two values is reported only where it can change a pixel**, on the discriminator
`note_group_structure` already uses for a form XObject:

- **isolated**, where an element blends — §11.4.4's NOTE 2 makes an element's blend with the
  backdrop "what distinguishes non-isolated groups from isolated groups", so an isolated group of
  Normal-blending elements is the same page either way;
- **knockout**, where a later element composites over an earlier one — §11.4.6's own condition,
  through `knockout_can_show`.

**Table 145's `/CS` needs nothing beside them, and the clause is why rather than an omission.**
§11.6.6: "[f]or non-isolated groups, or if no group colour space is specified, the group colour
space shall be inherited from the parent group or page." On the group this path actually builds, a
`/CS` states nothing; where the file also says `/I true`, the first report above already names the
departure that carries it. Two of the corpus's four appearance groups state a `/CS` and both are
non-isolated, so a report conditioned on the entry would have fired twice for no difference at all —
trap 11 avoided by reading the clause rather than the key.

## 5. The population, and why it is a committed census

`crates/pdf-model/examples/appearance_transparency_census.rs` counts what an appearance stream says
about its group and what the annotation says about how that group meets the page, in
`border_precedence_census`'s three scopes. **The corpus and the world disagree, which is ADR 0490's
shape again**: over the 974, four appearance streams state a `/Group`, all four `/S /Transparency`,
none isolated, none knockout, and not one annotation anywhere states a `/BM`. Over
`CC-MAIN-2021-31`'s 65 944 there are 143 such groups with **95 isolated and one knockout**, and
**eight annotations stating `/BM /Multiply`** beside a stored appearance, on ink and polyline
annotations in two documents.

So the fixture pair in `annotations.rs` is the corpus's only witness (trap 8) and is *not* the
world's, and the report has producers behind it rather than a hand-built file. It is committed
because §12.5.5's row now states those figures and `doc/todo/01`'s rule is that a counted claim owes
a command.

## 6. The erratum, and it is a struck *value*

`spec-errata emit` over the two pages §12.5.2 spans files seventeen annotation objects, and **one of
them is named nowhere in this tree**: Issue #577, `Review`/`Accepted`, dated 2026-05-21 — the newest
erratum any round here has met. It is a `StrikeOut` over `1.0 ` with a `Caret` saying `the value of
CA`, and `pdftotext -bbox` puts it on Table 166's **`/ca`** row: 841.92 − 297.032 = 544.888 is the
strikeout's own top edge and `Default value: 1.0` sits at 544.888 from the top of physical page 484,
three lines under "…but not the popup window that appears when the annotation is opened".

So `/ca`'s default is *the value of `CA`*, which is what `annotation::construct` has computed since
it was written — `/ca`, then `/CA`, then 1.0 — on the strength of the **`/CA` row's** own sentence
rather than the `/ca` row's, which said 1.0 and seemed to deny it. The erratum settles the nearer
row with the further one's rule. **No arithmetic moves; the authority does**, which is the third
consecutive shape of this kind: an erratum that supplies the standard's warrant for something this
reader had been doing on an argument.

**And it is a third way `check` is blind**, beside a caret with no strikeout and a strikeout under
the four-word floor: **a strikeout whose text is a *value*.** `1.0` shares no sentence with anything,
so a tree quoting the whole `/ca` row verbatim would still not match it. `emit` is the only
instrument that can see one, which is `doc/todo/01`'s rule about running `emit` before writing
earning its keep for a fourth distinct reason.

## 7. Consequences

- §12.5.5's row says what is followed and what is not, cites §11.4.4's NOTE 5 with its conditions,
  names the clause's second case as reported, and carries the census's figures with the command that
  produces them.
- §12.5.2's row records Issue #577.
- ADR 0030, whose paragraph both defects came from, carries both corrections as a note rather than a
  rewrite: the decision it records — a stored appearance ignores the annotation's compositing
  parameters — stands, and only its citation and its list have moved.
- One report exists that did not, on a condition the corpus never meets and the crawl meets 96
  times. No ratchet moves and no page changes.
- **The method gains one rule**: strip the clause-level parents out of the ranking, and read the
  strongest *pairs* inside the family the total names. Prefer fewer rows read properly.
