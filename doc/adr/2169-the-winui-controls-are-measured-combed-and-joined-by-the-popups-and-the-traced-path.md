# 2169 — The WinUI controls are measured and combed, and the popups and the traced path join them

Session 1666, RFC 0010 section 6's step N3a, third part. Status: **accepted**, in
`crates/viewer-ffi/src/window/shown.rs`, `quorra.h`'s window section and `hosts/windows/Quorra.App/`
(`Controls.cs`, `Notes.cs`, `PageArea.cs`, `QuorraWindow.cs`, `MainWindow.xaml.cs`). Context: ADRs
0245, 1264, 1604, 1726, 2137, 2153 section 4, 2168; `viewer_host::ControlFit`.

## 1. Decisions

1. **The measured fit crosses as numbers and is computed once.** `quorra_window_controls_measured`
   takes four `int32_t`s a control — the width and height it was asked to occupy, then the least it
   can be, in logical pixels — and feeds them to `viewer_host::ControlFit`, as `viewer-qt`'s
   `measured` does, so no two windows can disagree about the magnification. `QUORRA_KEY_W` is now
   carried out by the window from the last measure (`fitting §12.7's controls at …`, or "every
   control on this page already fits its /Rect") rather than handed back as `QUORRA_ACT_FIT_CONTROLS`.
   The WinUI window measures after every `CONTROLS` and every repaint that moved them. **What it
   calls a control's least**: a check box's or radio button's glyph both ways; for a text, choice or
   list control the height its line and chrome need at the width it was given, and the given width,
   because a XAML control states no least width once its style's `MinWidth` is set aside — which
   ADR 2153 does, the rectangle being the document's — and scrolls what does not fit across.
2. **Table 231 bit 25's combs are the control's.** "[T]he field shall be automatically divided into
   as many equally spaced positions, or combs, as the value of MaxLen, and the text is laid out into
   those combs." A delegated widget has no appearance under its control (ADR 2153 section 1), so the
   WinUI control draws them: one cell per position with a rule between each, a character centred in
   each — a text element, not a UTF-16 unit — while the text box does not hold the keyboard, and
   the box's own text while it does, which is what a person types into. A field that sets bit 25
   against the clause's own condition (with Multiline set) keeps its plain control.
3. **Table 231 bit 26's rich text is shown as its plain value**, as every toolkit window shows it:
   what `/RV`'s formatting changes is the widget's appearance, and no window of this tree draws it.
4. **§12.5.6.14's popups are windows in Windows' furniture**, on `QUORRA_OWED_POPUPS`: a title bar in
   Table 166's `/C` where one is stated (§12.5.6.2's `/T` and `/M` in it, a close button that is
   `quorra_window_activate` on the popup), then `/Contents`, selectable and scrolling, at the
   rectangle the document states, in a fourth layer above the controls. A note whose Table 172
   `/RC` states rich text is drawn from `/Contents` and said once in the log (trap 5). Retyping a
   note (ADR 1726) needs `quorra_window` entry points this step does not add.
5. **§12.9's traced path is a mark.** `quorra_window_traced_path` hands the measuring points over as
   flat pairs in the two-call idiom over floats, the repaint's `MARKS` bit covers them (ADR 2168),
   and the window draws them as a polyline in the accent colour (ADR 1264: the rubber band is
   chrome).

`POPUPS` leaves ADR 2137's list of records only logged; its five declined rows stand.

## 2. The compiler, found on this machine

ADRs 2136 and 2153 recorded that no gate could compile the C#. A .NET 10 SDK unpacked into a
round's scratch compiles both projects' C# on Linux against the Windows App SDK's reference
assemblies (`EnableWindowsTargeting`, the XAML replaced by a stub of the two fields and the method
it generates, PRI generation off). It found that `PageArea.Show`'s parameter `pictures` hid the
field of that name, two `CS1061` errors CI's first `window` run would have stopped at; fixed here.
What it cannot judge is the XAML compiler's, which runs on Windows only.
