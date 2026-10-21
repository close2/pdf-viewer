# RFC 0010 — A Windows-native host in .NET, on the C ABI

Status: **proposed**. Session 1606, from the owner's `doc/questions/A287` of 2026-10-19 and ADR 2048.
One choice in it is the owner's, the UI framework (§3.3), and `doc/questions/Q501` asks it. The rest
can start whichever way that is answered. **§6's N0, N1 and N2 are built (ADRs 2054, 2072, 2088,
2104, 2105, 2120, 2121); the owner chose WinUI 3 (`doc/questions/A501`), and N3a's first two steps
are written (ADRs 2136, 2137, 2152, 2153).**

## 1. Motivation

> The reference stays winit, but we will add a windows native (dotnet) version in addition to the
> other applications. If possible gtk, qt, and dotnet should provide a way to still render on the
> GPU. We will also introduce a macos native version, so the modifications should keep this in
> mind. However: we do not implement the macos version right now.

The tree has three in-process windows and a C ABI (`crates/viewer-ffi`, ADR 0247) that nothing
outside its own tests consumes. On Windows the tree ships `quorra.exe`, `quorra-confined.exe` and
the workers (`.github/workflows/ci.yml`'s `snapshot` row). It ships no window whose controls are
the platform's. A .NET host is that window. It is also the first real consumer of the C ABI, which
is what ADR 0247 froze the ABI for.

## 2. Prior art, as evidence about convention only

Windows' native PDF readers draw in the platform compositor and put the platform's own controls
over the page. Embedding a native renderer in .NET through P/Invoke is a common pattern. Both are
convention and not correctness (principle 5): nothing here decides what a page looks like.

## 3. The design

### 3.1 Which boundary

| | the C ABI through P/Invoke | the confined window's wire |
|---|---|---|
| what the host holds | a `Viewer` in process, so it is a level host (ADR 2048 §2.3) | no `Viewer`, so it is a second `quorra-confined` and owes only what it performs (ADR 1190) |
| what is frozen | `include/quorra.h`, checked against `src/abi.rs` by `tests/header_and_library_agree.rs`, with `QUORRA_ABI_VERSION` and `quorra_abi_check` | `viewer_confined::protocol`, private to one window and not frozen |
| what C# writes | declarations, using `[LibraryImport]` with UTF-8 string marshalling, since every string in the header is UTF-8 | the framing, the codec and a rasteriser for the marks that cross |
| confinement | none, as for `quorra.exe` on Windows, which has none (`doc/todo/35`) | the worker's, which Windows does not enforce either |

**The C ABI.** The wire would buy a confinement Windows does not provide, and it would cost a
second implementation of a protocol nobody froze.

**What the ABI lacked.** `tools/state.sh hosts` printed 37 of 45 `Command`s reaching it. The eight
are **built** (ADRs 2060, 2061), each in the shape the table prices except `Oversize`, which the
processor serves now — the request is the list, and a window of it is drawn and read back — and the
three script commands, whose runner is `viewer_host`'s, made beside the library:

| missing | what it is | the .NET host needs it for | price |
|---|---|---|---|
| `CommitField` | a toolkit control lost the keyboard | every placed `TextBox` | one entry point naming the document; no new shape |
| `Clock` | the reader's date for Table 166's `/M` | an annotation's date | one entry point taking `pdf_syntax::Date`'s fields |
| `Respond` | a server's answer to a submission or an import | §12.7.6.2's submit and §12.7.6.4's import by URL | document, URL and bytes, like `quorra_supply` |
| `Oversize` | the host draws a window of a page (`Rendered::Listed`) | pages past `MAX_PIXELS` | comes with §4's device. It needs the list to cross as a handle the device draws, which the processor ABI has no use for |
| `Keys` | Adobe's `event.shift`, `event.modifier`, `event.keyDown` | scripts only | a struct of three fields |
| `Scripts`, `AnswerScripts`, `AnswerScript` | the scripting level, its ask, a script's question | scripts | the ABI's session supplies no runner (ADR 1702). The library needs a runner made from `pdf-script-worker` spawned beside it. Until then the host is pinned `Scripting::Off` and says so, as a window that cannot supply a runner must (`CLAUDE.md`'s JavaScript exclusion) |

### 3.2 Which layer of the ABI

The ABI is today `viewer-core`'s vocabulary plus four of `viewer-host`'s mappings (`panel`,
`popup`, printing, archiving). The toolkit windows share much more: `viewer_host::native_window`'s
`NativeWindow` trait (ADRs 1920, 1972) holds the event match once, and its required methods are
exactly what a toolkit answers its own way. A C# host that re-wrote the match would be the third
copy `doc/todo/30` warns of.

**Proposed: a second layer, `quorra_window`, that drives `WindowState` and `NativeWindow` from Rust.**
Its required methods are a table of C function pointers the host fills: `say`, `repaint`,
`retitle`, `password_required`, `follow_the_link` and the rest. In C# they are
`[UnmanagedCallersOnly]` methods. A callback must catch every exception, because one that crosses
the boundary ends the process. The vocabulary layer stays for any caller that wants only it.
**Price**: about 30 callbacks and one entry point per provided method a host calls. Callbacks need
no new form of `unsafe`: they are called inside `abi.rs`'s `unsafe extern "C"` functions, which
`tests/unsafe_position.rs` already counts. **§3.2a amends the shape: records, not callbacks.**

### 3.2a What the layer owes arrives as records, not callbacks (ADR 2088)

The callback table disagrees with the ABI it would join. ADR 0247 section 2 refused callbacks for
this ABI because a caller answering one can call back into the viewer while it is borrowed, and the
layer would meet that twice: `say` is called from inside `NativeWindow::drain`, while the window is
held mutably, and in C# an exception that leaves an `[UnmanagedCallersOnly]` method ends the
process. So each required method a toolkit answers its own way leaves a record in an owned batch —
`quorra_window_take_owed`, read with indexed accessors and freed like `quorra_events` — and the
caller acts on the batch after the call that produced it returns. That is `viewer-qt`'s shape, whose
C++ side reads update flags because Rust never calls a Qt object.

**Built** (session 1626): `crates/viewer-ffi/src/window.rs` implements `NativeWindow`; ten
`QUORRA_OWED_*` kinds with their count; thirty-two entry points — open by path, free, resize,
tick and its interval, a key, the wheel, the pointer, the find bar's search, the strip of open
documents (count, label, front, choose, open beside, close), the password and the one question
answered, the frame presented, the title, the status, the page area's pictures, and the batch's
seven. A key is decided
by `viewer_host::decide` and the acts the trait carries out are carried out; the widget acts come
back as `QUORRA_ACT_*`. `c/drive_a_window.c` is a toolkit's loop with one call per thing that
happened to the window.

**Built** (session 1634, ADR 2104): **what changed is a record and what it changed to is read**, as
`viewer-qt`'s flags sit beside its getters. Three more kinds say "again" — `CONTROLS` (the set of
controls over the page moved: rebuild, do not move), `POPUPS` and `STRIP` — and every `REPAINT`
draws the frames and then the marks over them. Twenty-four entry points read and commit through the
window, the panels' tab enumeration and its count among them: the five panels' rows, the page count, label and thumbnail, the form, the popups, the
selection's, the find bar's and the highlight's quadrilaterals and the focus, each answered by the
handle the vocabulary layer already hands over through one `Reads` both handles lend; and the
commits a toolkit window makes — a row or a push-button activated, a field's text, options, clear
and commit by qualified name, a check box or radio button toggled by its widget through
`viewer_host::toggling`, a layer's switch (Table 99's lock said) and an attachment extracted. The
carrying-out of what a document asks — the link, the submission, the two fetches, the import, the
remote file, the embedded document and the one question's answer — is `viewer_host::native_window`'s
now, and GTK, Qt and this window each keep only the dialogue (ADR 2105). The archive chooser, a
script's dialogue and a print grant are four more record kinds — `ARCHIVE`, `SCRIPT`,
`SCRIPT_WITHDRAWN`, `PRINT` — each answered once by an entry point of its own, and the window
carries out the print key as `viewer-qt` does (ADR 2121); it opens under the levels a host states
on a `quorra_settings` handle (§5, ADR 2120).

### 3.3 Which .NET UI framework (the owner's choice, Q501)

| | WinUI 3 (Windows App SDK) | WPF | WinForms |
|---|---|---|---|
| a device-drawn page | `SwapChainPanel`, which wgpu 30 targets directly (`SurfaceTargetUnsafe::SwapChainPanel`, DX12) | `HwndHost`, under which no WPF element can be drawn ("airspace"), or `D3DImage`, which needs a D3D9Ex shared surface wgpu does not make | a child `HWND`, siblings by z-order |
| controls over the page (`Command::Delegate`) | XAML children compose over the swapchain | not over an `HwndHost` | each control is its own `HWND`; works, with flicker |
| builds on this Linux machine | no: the XAML compiler is Windows-only, so builds are CI's `windows-latest` only | compiles with `EnableWindowsTargeting`, runs on Windows only | the same as WPF |
| accent and highlight colours | `UISettings` | `SystemParameters` | `SystemColors` |

**Recommendation: WinUI 3.** It is the only one of the three where a device-drawn page and the
controls placed over it compose without a workaround. That is the same property ADR 2049 asks of
`QRhiWidget`, and the one a macOS host gets from a layer-backed `NSView`. What it costs: no build
or run on this machine, and the Windows App SDK as a runtime dependency.

## 4. Presenting a frame the device drew

**N3a — pixels across first.** The ABI gains `quorra_device`, ADR 2049's S0 library behind the C
ABI. It is made off the UI thread, it returns before it is warm, and through it a render request is
drawn by the chooser's lane and read back. The host copies the frame into a `WriteableBitmap`, as
it copies a processor frame. No `unsafe` is added, and no message changes.

**N3b — the swapchain.** `quorra_surface_attach(device, kind, handle, width, height)` makes the
surface from a tagged `void *`:

| `QUORRA_SURFACE_*` | wgpu 30's target | for |
|---|---|---|
| `SWAPCHAIN_PANEL` | `SurfaceTargetUnsafe::SwapChainPanel` | WinUI 3 |
| `HWND` | `SurfaceTargetUnsafe::RawHandle` with a Win32 handle | WinForms, a bare window |
| `COMPOSITION_VISUAL` | `SurfaceTargetUnsafe::CompositionVisual` | DirectComposition |
| `CA_METAL_LAYER` | `SurfaceTargetUnsafe::CoreAnimationLayer` | macOS, **reserved and refused by name** until a macOS host is built |

`create_surface_unsafe` is called inside `abi.rs`, where the existing three forms of `unsafe`
already cover it. `raster-gpu` needs one safe constructor that takes a `wgpu::Surface` the caller
made, beside `Device::for_surface`; that is the `raster/` sub-project's decision to record. The
surface is made on the thread that owns the view, which Metal requires and WinUI expects. Frames are
presented from the drawing thread through a detached `Presenter` (`raster/doc/adr/0056`). **Price**:
`wgpu` and `raster-gpu` enter `viewer-ffi`'s graph, which is 145 packages on Windows today against
`quorra`'s 226 (`cargo tree --target x86_64-pc-windows-msvc`). Whether that is one library or a
`device` feature is open (§8).

## 5. What it must match

The level-hosts rule (`doc/todo/30`) is checked by compiling. A C# host compiles in none of the
tree's gates on the machine they run on — CI's `interop` job compiles the declarations (ADR 2073)
and no gate compiles a window — so its levelness is held three ways. The constants it mirrors are compared with the
header by a Rust test reading the C# source, so the check needs no `dotnet`. `quorra_abi_check` fails
a startup, once, where a count moved. And whatever §3.2's layer exports is not re-written in C#.

| `viewer-host`'s decision | in the ABI today | owed |
|---|---|---|
| the key table (`keys`, ADR 0526) | **yes** (ADR 2072): `QUORRA_KEY_*` with `QUORRA_KEY_COUNT`, and `quorra_key_press`, which acts on a press as every window does and hands back a `QUORRA_ACT_*` for what only a window can do. The modifier is the platform's *command* modifier (Ctrl here, ⌘ on macOS), not "ctrl" | — |
| the six panels (`Tab`, `panel`) | **yes** (ADR 2104): the rows through `quorra_window`, and `QUORRA_TAB_*` with `QUORRA_TAB_COUNT`, `quorra_tab_count` and `quorra_tab_label` | — |
| the restriction levels' sentences and the word that turns them off (ADR 0604) | the operations, yes; the sentences, no | the sentences, or `quorra_window`'s `say` |
| password, cannot-open, no-pages, `NOTICE` | password, cannot-open and no-pages through `quorra_window` (ADR 2088) | `NOTICE` |
| the drawing queue, loading, page one drawn while read, stand-ins, the oversize window | **yes**, through `quorra_window` (ADR 2088): the window runs them and hands over the pictures | — |
| the window's settings words (`Settings::take`, `Settings::set`) | **yes** (ADR 2120): the four binaries read them through one function, and `quorra_settings_*` with `QUORRA_POLICY_*` and `QUORRA_LEVEL_*` state them for `quorra_window_open_under` | — |
| the reader's other words (`ReaderWords`) | **yes** (ADR 2156): `quorra_audience`, `quorra_trust_anchors` on the vocabulary layer; for a window, `quorra_settings_reader` with `QUORRA_READER_*`, `_accept_unknown_revocation`, `_archive` and `_take_next`, read as the four binaries read them through `viewer_host::policy::Words` | — |
| the cards: archive chooser, ask dialogues, links | archiving, yes; through `quorra_window` the chooser, a script's dialogue and a print grant are records the caller answers (ADR 2121) | the sentences each card says |

## 6. The steps, priced

| step | what | where it builds | price |
|---|---|---|---|
| **N0** | `hosts/windows/Quorra.Interop/`, hand-written `[LibraryImport]` declarations for the header's 240 entry points and its constants, outside the Cargo workspace. A Rust test in `crates/viewer-ffi/tests/` reads `quorra.h` and the C# file and asserts each entry point is declared once with the same parameter count and each constant has the same value, as `header_and_library_agree.rs` does for `abi.rs` | the test on this machine; `dotnet build` in a CI job | one round. **Built** in session 1609 (ADR 2054): one file per section of the header, and the test holds each parameter's and field's type under one rule as well as the names and values. `dotnet build` is still owed |
| N1 | `CommitField`, `Clock` and `Respond` reach the ABI, with the key table and its count. **Built**: the eight commands (ADRs 2060, 2061), then the key table, its two counts and a submission's owed sentences (ADR 2072). `dotnet build` of N0's layer is CI's `interop` job (ADR 2073) | this machine; CI for the C# | one round |
| N2 | `quorra_window` (§3.2, §3.2a) | this machine; driven by `a_c_program_drives_the_abi.rs`'s C caller | two rounds. **Both are built**: the loop, the keys, the pointer, the search, the strip, the prompt and the question, the pictures (ADR 2088); the panels' rows, the controls and their commits, the marks over the page, and the request handling one function for three hosts (ADRs 2104, 2105); the policy flags and the three dialogues (ADRs 2120, 2121) |
| N3a | the WinUI 3 window on the processor lane: the page, the controls over it, the panels, the cards | `windows-latest` only | three to four rounds. **The four processor-lane steps are written** (ADRs 2136, 2137, 2152, 2153, 2168, 2169, 2185): `hosts/windows/Quorra.App/`, unpackaged and self-contained, draws each of `quorra_window`'s pictures into an `Image`, copying only those a repaint calls fresh, the marks over them and §12.9's traced path in Windows' own colours, a platform control over each widget whose appearance it draws itself — Table 231's combs among them, each measured against its rectangle for `w`'s fit — committed as the toolkit windows commit them, and §12.5.6.14's popups in its own furniture; shows the title and the status line, takes the keys, the pointer, the wheel and Control with the wheel (`quorra_window_zoom_wheel`), reads the selected panel only while open, decodes thumbnails only for realized rows, and answers password, permission, archive, script and print-preview cards through native dialogues, and is built by CI's `window` job with `viewer_ffi.dll` beside it. `tools/dotnet-check.sh` compiles its C# against the Windows App SDK references on Linux; XAML and interaction still require Windows validation. Printer selection and spooling remain N4 |
| N3b | ADR 2049's S0, then `quorra_device`, read back | this machine, then CI | one round after S0 |
| N3c | `quorra_surface_attach` on a `SwapChainPanel` | `windows-latest`; to be seen on a Windows machine | two rounds and an ADR |
| N4 | UI Automation peers from `Query::Accessibility`, printing, file pickers, packaging with the workers beside the executable | `windows-latest` | three rounds |

## 7. macOS, kept in mind and not built

A Swift host would read `quorra.h` through a module map, so §3.2's layer serves it unchanged. Its
device is a `CAMetalLayer`, reserved in §4's table. Its command modifier is ⌘, which is why §5 names
the modifier by role. Its surface must be made on the main thread, which §4 already requires. Nothing
in the steps above names a Windows type in the header, and `QUORRA_SURFACE_*` is the one place a
platform appears.

## 8. Open questions

1. The framework (§3.3), and Q501.
2. One `viewer_ffi.dll`, or a `device` feature that keeps the processor-only library small.
3. Whether the .NET host's console is its log, as every window's is (ADR 1771), or a pane.
4. Whether the round that builds N0 may install a user-local .NET SDK under `/home/AI` so that the
   interop library's tests run here, or whether those tests are CI's alone.
