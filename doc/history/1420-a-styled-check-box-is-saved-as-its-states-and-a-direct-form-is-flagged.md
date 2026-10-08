# 1420 — A styled check box is saved as its states, and a direct form gets the flag

Slot 1 of batch sixty-six, 2026-10-08, a script round. ADRs 1676 and 1677; ADR 1665 section 3
marked superseded. No ledger row moved (§12.7.5.2.3, §12.7.3 and §12.7.2 are `implemented`, §12.5.5
`departed`, none touched in status); no question written.

**Premise.** Held. `view::interactive_form` read `/AcroForm` only through `Object::as_reference`,
and the check-box fixture of ADR 1665's own test holds its form directly, so that test never saw
the flag. Found beside it: the flag was read from the file's form, not the update's, so a save that
also stated the free-text font in `/DR` lost that font.

**States written (ADR 1676).** A toggling widget whose script set `style` is saved with both of
§12.7.5.2.3's states constructed (posed by `/AS`), as new objects under `/N`, and under `/D` and `/R`
where those exist. The on state keeps the file's name for it; with no name, it is owed as before.
Rendered at 6 px per unit with no script and looked at: ✘ in the border, and the border alone with
`/AS /Off`. The glyph sits at the left, quadding 0, which is how the page draws it too.

**Direct form (ADR 1677).** `Update::owe_appearances` sets `/NeedAppearances` in a referenced form
or in the catalog that holds a direct one, reading both through `Update::current`;
`state_default_font` reads the catalog the same way. `interactive_form` is gone. Planted back: the
direct arm off fails `a_style_with_no_on_state_is_owed_and_flagged_in_a_direct_form`; the file's
form read fails `the_flag_and_the_free_text_font_are_written_into_one_form`.

**`setFocus` (unanswered).** The request crosses as a field name (`take_focus_request`), and
`viewer-core`'s `carry_out_focus_requests` focuses `widgets.first()`. Answering it changes the
wire's `ScriptEdit::Focus`, the view state's request type and that host site together, and the host
crate is slot 2's this batch. The refusal stays. Class 6 is still unbuilt.

**Gates.** `rustfmt --check` on the 3 changed `.rs` files: exit 0. `-D warnings` clippy on
`pdf-model --all-targets`: exit 0. It failed first in `pdf-font/src/pairs.rs`, which was slot 5's
edit in progress; on retry, exit 0. nextest `pdf-model`: 1 979 passed, 19 skipped. `cargo test -p
conformance --no-fail-fast`, run twice. On the first run only `--test bounded` failed, 2 tests,
while slot 6 was editing it. On the second, at my end, only `--test names` failed, naming slot 3's
`raster-gpu` `helpers.rs:11`; no failure named a file of mine. Behind the lock,
the Tier 1 column held 21 101 runs, 11 776 finished, 9 317 threw, 8 unparsed and 0 over a budget,
so `HELD_THREW` and `HELD_RUNS` are unchanged. That run waited 622 s and held for 147 s, peak 1.44
GiB. The worker column gave the same five figures with 0 lost and 0 `SIGSYS`, holding for 157 s.
`save_round_trip` and the `pdf-model` corpus: exit 0, every ratchet at slack 0, one hold of 38 s,
peak 1.50 GiB. Duration 3 700 s.
