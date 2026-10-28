# 2525 — A hybrid form's calculations activate when it is shown and at a commit, and an override reaches a window as its notice

Session 1845, a script round in batch 125. Status: accepted. Follows ADR 2524; ADR 2506 is the order they run in.

## Context

`Resource::activate` takes a stimulus (XFA 3.3 page 379: the completion of a merge, a person's
entry), a runner and limits, and says what it changed. Nothing called it outside tests. ADR 2506
item 5 left the message page 642 asks for an `override` of `error` or `warning` "named in the report"
because the activation "offers no flag a host could act on".

## Decision

1. **The view state raises the two stimuli** (`ViewState::activate_form_on_merge` and
   `activate_form_on_entry`, `crates/pdf-model/src/view/scripts/form.rs`) through whatever runner a
   host supplied, and only through it: no runner, no activation; the level `off` supplies none. The
   resource is read the first time a stimulus asks and kept; a document with no XFA resource, a
   `NeedsRendering` one, a template with a `subformSet` or a container that occurs a varying number
   of times, or a resource that does not read, runs nothing and says why once (the dynamic half
   waits on `doc/questions/Q639`). The fields the activation reads are those the calculation order's
   own table reaches (`Omitted::Calculation`), with the text each holds now, a person's entry
   included. What the scripts computed is applied as the edits a person's typing makes, so it
   reaches `/V` and the `datasets` packet through the save (ADR 2396).
2. **The merge's completion is the first frame**, not the open: `viewer-core` raises it in the open
   sequence (`Command::Presented`), where Table 32's name-tree scripts already run, so that opening a
   document pays nothing for a form it has not shown (CLAUDE.md, *Nothing eager*). A person's entry
   is the commit, after the field's validation has passed and before Table 224's `/CO`. Handing a
   runner over after the open runs the merge again, as it runs the open sequence again, and **what
   a person entered before it stands under it**: the merge's edits leave those fields as they are
   and each is then raised as an entry, so a `warning` or a disabled calculation keeps it and an
   `error` replaces it with a notice, as page 642 has it had the merge come first. The cost is a
   second activation for each field entered, the notices arriving with the merge rather than with
   the keystroke, and a set of at most 10 000 field names the view state keeps for it
   (`script_form.rs` and `form_script.rs` plant it at `ask`).
3. **The level decides, as for every other script** (ADR 1616). `ask` hands the activation the
   withheld runner, which records the first form script as the question's subject — "a FormCalc
   calculation of the document's form" and its first line — and runs nothing; a `yes` hands over
   the real runner and the merge runs. `warn` says once per activation in which a script ran that it
   did. `off` runs nothing and starts no process.
4. **`override` reaches the window as a notice.** `Activation::overridden` names the fields of a
   person's entry whose calculation has `override` `error` or `warning` (page 642; `ignore` and
   `disabled` ask for no message); the view state holds them (`take_form_overrides`) and
   `viewer-core` puts each as `Event::ScriptAsking` with an alert of one button — the event every
   window already draws for a script's `app.alert` (ADR 1628) — so no window and no event changes.
   The words are this program's: the page describes the dialogue and states none. A notice waits for
   the one before it and for a question the runner's script has put, and its answer is consumed by
   `viewer-core` and not handed to the runner, because the runner could mistake it for its own
   (`script_form.rs` plants both); at most 64 wait.
5. **The `warning`'s choice is not offered.** Page 642 has the person dismiss the warning, which
   keeps the calculated value, or override it. Here the entry stands (ADR 2506 item 5's choice) and
   the notice says so; it does not ask a question whose answer it would not carry out. Owed, with the
   state a dismissal needs: the value the calculation produced beside the entry.

6. **The activation is held to ADR 1603's sequence budget.** Each script is held to its runner's
   deadline and to the activation's step and text limits, and the run count (10 000) does not see
   time: thirty calculations each just under a deadline of 250 ms held a merge for 3.8 s. The
   activation's runs are therefore held together to `MAX_SEQUENCE_TIME` (1 s, as a commit's walk
   of `/CO` is); past it no further script is run, the activation ends as a spent budget does,
   every value it calculated is dropped, and the report says it was the clock. One run may end
   after the budget, so the longest an activation holds a window, a late merge's re-raised entries included, is the budget plus one deadline.
   A late merge and the entries it re-raises (item 2) are one sequence and share the one budget;
   the entries not yet raised when it is spent are left as the person made them, and the report says
   so.

## Consequences

- No Event variant, no wire change in `viewer-confined` (its level is pinned `off`, ADR 1616), no
  change under `hosts/windows`: the C# windows present `ScriptAsking` already. The Rust crates they
  link changed (`viewer-core`, `viewer-host`, `pdf-model`), which CI's `window` job exercises.
- A hybrid form shown with a runner pays `Resource::read` once, at the first frame; a document with
  a runner and no XFA resource pays one catalog lookup, once; a view state with no runner pays nothing.
- **Owed, and not this round's**: `quorra-gtk` and `quorra-qt` ask for `Command::Report` and
  `Command::Presented` once, after the first document's first frame, because `viewer-host`'s
  `report::Due` is one per window. A document opened beside is never given its open sequence, so
  neither its merge nor its Table 32 scripts run, and its signature report is not asked for;
  `quorra`'s window keeps one per document and does. The fix is in those hosts' call sites.
