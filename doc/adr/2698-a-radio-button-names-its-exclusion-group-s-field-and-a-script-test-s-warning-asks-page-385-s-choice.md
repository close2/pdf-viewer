# 2698 — A radio button names its exclusion group's field, and a script test's `warning` asks page 385's choice

Session 1942, a ledger round. Status: implemented. Extends ADRs 2566, 2584, 2627 and 2647, which reported a
pointer on an exclusion group's widget instead of running anything, and ADRs 2696 and 2697, which put
a script test's message with one button.

## Context

XFA 3.3 pages 390–392 give `click`, `mouseDown`, `mouseUp`, `mouseEnter` and `mouseExit` to a field
and none to an exclusion group. A hybrid form's exclusion group is one PDF radio field with one widget
per button. So the field list names the group, not the field the pointer was within.

Page 385: where a script test whose `scriptTest` is `warning` fails, the processor presents a message
with two choices. "dismiss" means the person "wishes to return to the form, so that s/he may satisfy
this constraint". "override" means the person "has chosen to contravene this constraint".

## Decision

1. **The widget names the field by its on value.** `pdf_model::form::button_on_value` gives the value
   a widget sets on its field's `/V` when it is on. That is Table 230's `/Opt` entry at the widget's
   place, or else the name of its on appearance state (§12.7.5.2.3–4), the same value the save writes.
   Annex K requires the XFA values to be consistent with the `/V` entries. An exclusion group's value
   is the on value of the button that is on, the first `items` value of that field (pages 195–197, the
   rule `datasets.rs` already writes by). So the group's field whose first `items` value equals the
   widget's on value is the field the pointer was within. Each pointer stimulus carries that value as
   `button`, and `Session::pointed` fires that field's events. The group hears them only where its
   `listen` says (page 387).
2. **No button, or a value no field holds, runs nothing and says so by name.** The template is not
   guessed at.
3. **Page 385's choice is one question, as page 642's is (ADR 2697).** A group of failures that are
   all warnings is a Yes/No alert that ends "Keep the value you entered anyway?". "Yes" is override.
   It keeps the value, which already stands, so it changes nothing. "No", or a dialogue closed without
   a press (ADR 1628), is dismiss: `ViewState::answer_form_validation` asks for the keyboard focus on
   the first widget of the first failing container the document names as a field. It goes through the
   `setFocus` path (ADR 1615), so `/Bl`, `/Fo` and the XFA `exit`/`enter` events run as a tab runs
   them. A group holding an `error` keeps one button, because page 385 gives an error no choice.

## Consequences

- **The keyboard focus follows the same rule.** `Stimulus::Exit` and `Stimulus::Enter` carry the on
  value of the widget on each side. A move between two buttons of one group runs the two buttons'
  `exit` and `enter` and not the group's, because page 390 fires those only for focus that crosses
  the group's edge (`Session::crossed`).
- An override is not remembered. No script test runs at a save or a submit yet, which is where page
  1340 would ask again.
- **`Activation::validations` is removed rather than wired to the window.** It held the null and
  datatype tests a save runs, computed over the settled form at every activation and read by nothing.
  Page 1340 runs those tests at the occasions a form's `config` names, a save and a submit, and both
  already run them over the values they write (`Written::xfa_invalid`, `Submission::owed`; ADRs 2450,
  2474, 2475). At an entry they would name every empty required field the person never reached,
  where page 382 tests only what the entry changed, which is what the script tests do.
- Tests: `pdf-formcalc --test cascade` (`a_click_on_a_button_of_an_exclusion_group_…`,
  `the_pointer_on_a_button_…`, `a_click_on_a_button_whose_on_value_…`, `the_focus_on_a_button_…`), `viewer-core`'s
  `scripting` unit test and `--test script_form` (`a_script_test_s_warning_…`).
