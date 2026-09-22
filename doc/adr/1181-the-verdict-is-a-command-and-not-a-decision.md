# 1181 — The verdict is a command under the mode, not a decision to use it

Status: accepted. Session 1172.
Amends: ADR 1158 section 1, which set `DisplayList::overprints` at the single place that chooses
§11.7.4.3's special blend mode and read it as the question two backends ask.
Answers: ADR 1178's second finding, which measured what that cost and named where to fix it.
Context: `crates/pdf-render/src/display_list.rs`, `crates/pdf-model/src/content.rs`,
`crates/pdf-model/src/content/overprint.rs`, `crates/pdf-model/tests/overprint.rs`,
`crates/pdf-model/examples/overprint_ink_group_census.rs`.
Clauses: ISO 32000-2 §8.6.7, §11.4.7, §11.7.4.3.

## 1. Two events, one flag

`render-raster` and `render-gpu` refuse a whole display list by name when
`DisplayList::overprints()` is true, and fall back to `render-cpu`. So the flag is not a hint
about a page — it is what decides which backend draws it, and a page that carries it for a mark
that is not on it loses its backend for nothing.

The flag was set in `Interpreter::special_overprint`, where the mode is **chosen**. Choosing and
emitting are two different events, and three shapes separate them:

- **§8.6.7 gives stroking and non-stroking operations a parameter each**, so `overprint_blend` is
  asked twice per painting operator — before the operator knows which of the two parts marks the
  page. An `S` whose non-stroking colour has a zero tint chose the mode for a fill that never
  happened.
- **Table 104's text rendering modes 3 and 7 mark nothing**, and a hidden optional-content layer
  marks nothing either, while the modes are still asked for once per show-text operator.
- **A run of content can be thrown away.** `Interpreter::group_commands` interprets a group in its
  own colour space and, where that space cannot be carried out to the parent, runs the content
  again in what the parent composites in (`rerun_inheriting`); `mask_halves` does the same for a
  soft mask's group. The first run's commands are discarded. Only the first run can reach the
  special mode, because only it composites in four components.

ADR 1178's census printed the size of it: **177 of 10 040 pages** carried the verdict with no
command under it, 1.8% of what the two backends refused. Re-run over the same 65 944 documents
with the change in, that column is **0**, and the documents the mode reaches go from 1826 to
**1788** — the 38 whose only overprinting page was one of those.

## 2. The verdict is settled against the finished list

`DisplayList::settle_overprinting` walks what the list actually holds — its commands, the
commands nested in a group, a `Shaped` command's object and shape, `GroupBlending`'s black half
and every §11.6.5.1 soft-mask group's elements, including that mask's own black half — and sets
the flag to whether one of them carries `BlendMode::Overprint`. `content::finished` calls it once
per interpretation, beside `noninvertible_marks`, whose own comment states the same reason: *the
condition is a property of the command, and one walk cannot miss a route*.

**The walk is gated on the old flag, which stays as a hint.** `note_overprinting` still fires
where the mode is chosen, and `settle_overprinting` returns without walking when it never did — so
a page that states no overprint pays one boolean test, and the walk is taken on the fraction of
pages the census counts. That gating is also why this is not a behaviour a later round should move
back to the emission sites: setting the flag in `path.rs` and `text.rs` where a command is pushed
would fix the first two shapes above and not the third, because a discarded run pushes its
commands and then has them taken away.

**Nothing about what is drawn changes.** The flag decides which backend draws a list and is read
nowhere else; 177 pages of the crawl stop being refused by both backends for a mark they do not
carry.

**And nothing on the painting path changes either**, which is why this needs no benchmark beside
it: `note_overprinting` is where it was, so a page that states no overprint pays exactly what it
paid before plus one predictable branch per *interpretation* — `settle_overprinting`'s early
return. The walk itself is taken on the 9863 pages of 248 615 the census interprets, and the
census's own wall clock says nothing about it this time, the machine having been under a load
average above 200 from five neighbouring rounds building while it ran.

## 3. What checks it

- `tests/overprint.rs::a_mode_chosen_for_a_part_that_never_paints_is_not_the_verdict`: a page
  whose non-stroking colour has three zero tints, whose stroking colour has none, and whose
  operator is `S`. Calibrated by planting `settle_overprinting` away, which fails it. Its second
  half is the same two colours under `f`, where the fill *is* painted and the verdict stands.
- The census's own column — pages with the verdict and no mark — which is this claim measured over
  the crawl rather than on a fixture.
