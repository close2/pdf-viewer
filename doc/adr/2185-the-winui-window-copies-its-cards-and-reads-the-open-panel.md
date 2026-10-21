# 2185 — The WinUI window copies its cards and reads the panel the reader opens

Session 1674, RFC 0010 step N3a. Status: accepted; validation recorded in session 1674. Context: ADRs 2104, 2121,
2137, 2168. This supersedes ADR 2137's five automatic card refusals.

## Cards

The owed batch is native-owned. Before freeing it, the host copies every string and choice a
dialogue needs into managed records. One `ContentDialog` is shown at a time with an asynchronous
wait, allowing the library's timer to continue. Closing the window stops the queue and prevents
an outstanding continuation from answering a disposed window. A withdrawn script question is
marked in the queue or hidden if visible, and receives no answer. A newer card of the same kind
invalidates its predecessor because the native window holds one answer slot per card kind.

| record | the reader's answer |
|---|---|
| password | an obscured password, or the header's null cancellation |
| question | allow or decline after the library's reason |
| archive | the library's target and policy lists, including the selected policy's explanation |
| script | the requested alert buttons or response, including obscured responses and defaults |
| print | a preview explanation and a finish action; printer selection and spooling remain N4 |

A failure to show a card is said and explicitly declined through the existing ABI. It must not
leave a question standing with no visible means of answering it.

## Panels

The pane starts closed. Labels and rows are read when it opens, and only the selected panel is
read on a change. The page list contains indices; labels and producer-supplied thumbnails are
read for realized rows. Native row and thumbnail handles are freed after their contents are
copied. Outline expansion follows depth and the producer's initial expansion; layer controls
honor the locked bit. Outline objects, attachments and layers use their existing ABI actions.

A page row needs a direct page selection. `quorra_window_go_to_page` adds that operation using
the core's existing `GoTo(Index)` command, refusing an index outside the focused document before
dispatch. It adds a symbol without changing a crossing structure. The C window driver checks
last and first page selection and that an invalid index leaves the page unchanged.

## Live permissions

The launched-document policy crosses `quorra_settings_set` as policy 6. A562 also requires a
menu in every capable window. `Restrictions` now owns this machine policy beside its other
machine acts; native and winit launch handlers read that same value. GTK and Qt enumerate the
shared acts, so their existing menus acquire the operation without another policy copy.
The confined viewer offers the one machine act through the existing `RestrictionsCard`,
opened with R and changed by arrows/Enter or a pointer. The choice stays outside the worker.

WinUI reads the shared rows when its Permissions button or R opens a native menu. Three ABI
functions count, read and choose these rows; they add no crossing structure. Headings cannot
be chosen and an invalid index changes nothing. The C window driver selects and reads back
all four launch levels, then restores the initial one. Shared and confined tests check that
the menu's chosen level is precisely the value the launch handler reads. All four Linux windows
passed a live change from refuse to ask followed by a launch question answered no. WinUI's menu and
question dialog compile against the real SDK; Windows runtime verification remains unavailable
here. With every capable window offering the menu, A562's default is now ask. ADR 2171's initial
refuse default and missing-menu section are superseded; its classification is unchanged.
