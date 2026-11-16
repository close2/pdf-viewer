# 2908 — An enforced print scaling is locked in GTK's dialogue and handed to a C caller's

Session 2046. Status: **accepted and built** in `viewer-host` (`printing::Defaults::held_scaling`),
`viewer-gtk` (`host/printing.rs`: `hold_the_scaling`, `lock_the_scale`, `keep_the_scale`) and `viewer-ffi`
(`quorra_event_printing_scaling`, `quorra_owed_print_scaling`; `Owed::Print::held`). It finishes ADR 2840 section
2's "what is left". Clause: ISO 32000-2 §12.2, Tables 147 and 148.

## Context

Table 147's `/Enforce` names preferences "that shall be enforced by PDF processors and that shall not be
overridden by subsequent selections in the application user interface"; Table 148 names one, `/PrintScaling`,
admitted only where that entry states a valid value other than `AppDefault` — so `None`. ADR 2840 made obeying it
the reader's level for `OverridePreference` (default `warn`), answered by `viewer_core` as `Event::Printing`'s
`scaling_held`. `quorra-qt` locked its scaling combo; GTK's dialogue and a C caller's were only told a sentence.

## Decision

**GTK: the dialogue GTK builds is locked through GTK's public print API, and the job is held independently of
the lock.** GTK 4.18's `GtkPrintOperation` builds a `GtkPrintUnixDialog` and gives it, as the capabilities the
operation applies itself, `PAGE_SET | COPIES | COLLATE | REVERSE | SCALE | PREVIEW | NUMBER_UP |
NUMBER_UP_LAYOUT` (`get_print_dialog`, `gtkprintoperation-unix.c`); the dialogue makes its scale control
sensitive only where `manual_capabilities | printer_capabilities` contains `SCALE`
(`update_dialog_from_capabilities`), and neither the CUPS nor the file backend states `SCALE` among a printer's.
Where the level holds the scaling, the operation answers `create-custom-widget` with a tab that says why, and from
that tab — placed in the dialogue as the handler returns — reaches its root and withdraws `SCALE` at
`Priority::HIGH`, ahead of the first frame. Then `custom-widget-apply`, which `handle_print_response` emits after
the dialogue's settings are the operation's and before `finish_print` builds the `GtkPrintJob` whose scale becomes
the operation's `manual_scale` (the factor `cairo_scale` applies to every page), sets the settings' scale back to
the document's, and says so where it had moved. So a backend that did state `SCALE` would leave the control live
and still print the document's scaling.

**Why not a dialogue of this program's own**: it would rebuild GTK's printer list, its page setup and its
backends' options for one control, and lose everything a person expects of the platform's dialogue; GTK's public
API reaches the one control, which is the narrower construction. **Why not `GtkPrintUnixDialog` with a
`GtkPrintJob` directly**: the capability would be ours to omit from the start, but the operation's pagination,
preview and page-setup handling would be rebuilt here, for no difference in what a reader can change.

**C ABI: two entry points carry the hold and the value to keep.** `quorra_event_printing_scaling` reads
`Event::Printing`'s `scaling_held`, and `quorra_owed_print_scaling` a window's `PRINT` record, which carries
`Defaults::held_scaling` — the document's `/PrintScaling` where held. An event carries no preferences, so its value
is Table 148's only admissible one, `QUORRA_PRINT_SCALING_NONE`; the window's is read from the document, and the C
driver pins the two to agree on a document enforcing `None`. Where nothing is held both answer `APP_DEFAULT`, the
caller's own default and the reader's choice; the dialogue's opening value stays `quorra_preference`'s. A new entry
point leaves `QUORRA_ABI_VERSION` where it is (ADR 0737).

## Consequences

Every window this program builds that has a scaling control now holds it: `quorra-qt` and `quorra-gtk` lock theirs,
and a C caller is handed what to lock. The WinUI host offers no print dialogue — its `PRINT` card is a preview,
"Printer selection and spooling are not available in this window yet" — so it has no scaling a reader could
select; it is not built here, there being no .NET SDK on this machine, and its card reading
`quorra_owed_print_scaling` is owed with its dialogue. Number-up and the other controls stay the reader's:
`/Enforce` names `/PrintScaling` alone.
