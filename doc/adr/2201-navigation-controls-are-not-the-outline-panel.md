# ADR 2201 — Navigation controls are not the outline panel

Status: **accepted** at the merge that took it, `b9d05ee8` (line added by ADR 2260).

A NavigationView packages a pane, its toggle and the document area in one toolkit widget.
Hiding that whole widget for HideWindowUI would also hide the document; closing its pane would
remove an outline even though ordinary-window preferences leave `other_windows` enabled.

Map the resolved flags to their actual subjects. Hide the toggle and status with `window_ui`,
and panel availability with `other_windows`. The Permissions menu follows ADR 1145: it remains
available despite HideMenubar, but presentation hides it with other windows. Keep the
page area visible. Retain the pane's user preference independently of forced hiding and block
panel toggles only while panels are unavailable. A pure state transition test pins both the
open-pane and closed-pane restoration cases.

Adding a toolbar solely to make HideToolbar visibly do something would expand this correction
into new UI design. Reading raw document flags in C# would duplicate the shared presentation
policy and mishandle a FullScreen document. Neither is needed to make the existing controls
follow the state the library already resolves.
