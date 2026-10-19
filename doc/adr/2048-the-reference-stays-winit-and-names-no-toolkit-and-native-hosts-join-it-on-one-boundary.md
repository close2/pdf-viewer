# 2048 — The reference stays winit and names no toolkit, and native hosts join it on one boundary

Session 1606, slot 8 of batch ninety-five, a "host" planning round. Status: **accepted**, by the
owner's answer `doc/questions/A287` of 2026-10-19. **Supersedes ADR 0001** whole. ADR 2049 prices
the toolkit windows' device paths and RFC 0010 proposes the Windows host; neither is built.

> The reference stays winit, but we will add a windows native (dotnet) version in addition to the
> other applications. If possible gtk, qt, and dotnet should provide a way to still render on the
> GPU. We will also introduce a macos native version, so the modifications should keep this in
> mind. However: we do not implement the macos version right now.

## 1. What ADR 0001 rested on, and where each premise stands

| ADR 0001 said | the tree says | checked by |
|---|---|---|
| "Revisit only if conventional widget requirements appear" | **they appeared, and the condition fired without a record.** A form's controls are toolkit widgets placed over the page, there are six panels, a print dialogue, file choosers and a password dialogue. The tree's answer was to *add* hosts, not to put a toolkit in the reference: GTK 4 (ADR 0244), Qt 6 Widgets (ADR 0246), the C ABI (ADR 0247). No record said that this answered ADR 0001's condition; this one does | `ls crates/`; `doc/todo/30` |
| "the interface has little conventional GUI" | **retired.** `quorra` draws its own cards, a choice list, a typed opener, the six panels and the note editor. That is the cost ADR 0001's own *Negative* list predicted ("they must be built"), and it was paid | `crates/viewer-ui/src/bin/quorra/` |
| "`ashpd` for file dialogs through the XDG desktop portal" | **retired.** No crate depends on `ashpd`; Ctrl + O in `quorra` opens a card a path is typed into (`arrivals.rs`), as ADR 1023 §2 found | `cargo tree -p viewer-ui -i ashpd` matches no package |
| "`AccessKit` for accessibility" | **held**, and wider than the reference: all three in-process windows publish through it (ADR 0623) | `cargo tree -p viewer-ui -i accesskit` |
| "Do not depend on Qt" | **broken by ADR 0246, and correctly**: the tree depends on Qt 6 through `cxx` in `crates/viewer-qt` alone, with one hand-written `unsafe` token; `gtk4` reaches `crates/viewer-gtk` alone | `cargo tree -p viewer-qt -e normal,build -i cxx` names it; `-p viewer-ui` matches no package |

One stale sentence survives in the reference's own source: `crates/viewer-ui/src/lib.rs`'s module
comment still says its dialogs go through the portal via `ashpd`. This round owns nothing under
`crates/` and reports it rather than mending it.

## 2. The decision

1. **The reference is `quorra`, on winit, and it stays the reference.** It is the window where
   principle 2's page-one rule is stated and measured, where the device's lane chooser lives (ADR
   1831), and the one whose dependencies the project chooses entirely.
2. **"Do not depend on Qt" becomes a rule about the reference's own graph: `viewer-ui` and
   `viewer-ffi` depend on no widget toolkit.** No `gtk4`, no `cxx` or Qt, no .NET, no AppKit
   binding. A toolkit belongs to the one host that binds it, and a host outside Cargo (.NET, Swift)
   is outside the workspace. The command is `cargo tree -p <crate> -e normal,build -i <toolkit
   crate>`, which must match no package for both crates; a conformance test that runs it is owed to
   the first round that adds a toolkit binding.
3. **The application set is the reference beside native hosts on `viewer-core`'s boundary**: the
   confined window, `quorra-gtk`, `quorra-qt`, the C ABI, and now a Windows-native .NET host (RFC
   0010). The in-process ones stay level (`doc/todo/30`'s rule). The .NET host holds a `Viewer`
   in process through the C ABI, so it joins that rule as the fourth level host, where
   `quorra-confined` does not (ADR 1190).
4. **A native host keeps a device path wherever its platform admits one**, and the processor
   remains the lane where none does. ADR 2049 lists the paths and prices them.
5. **macOS is planned, and nothing is built for it.** Until the owner says otherwise, a host-side
   change passes a test of shape: no boundary message, no ABI parameter and no `viewer-host` type
   may name something only Linux or Windows has. A platform handle crosses as a tagged `void *`,
   a modifier as the platform's command modifier and not as the Control key, and a surface is made
   on the thread that owns the view, because Metal requires it (`raster/crates/raster-gpu/src/present.rs`).

## 3. Why not the alternative the condition invited

Putting a toolkit in the reference would have answered ADR 0001's condition in the obvious way, and
it would have cost three things the tree depends on. Principle 2's launch rule assumes a window
whose initialisation the project owns. The cross-backend oracle compares lanes this window chooses
between. And the level-hosts rule would lose its point: every message has been tested by three
consumers because no one of them is the flagship. The owner's "the reference stays winit" keeps all
three.

## 4. What it costs

- `quorra` keeps drawing its own widgets. Each new dialogue is drawn there and again in each toolkit.
- One more level host makes a feature four hosts' work, and the fourth is in a language the gates
  do not compile. RFC 0010 §5 says how its levelness is checked without a `dotnet` on this machine.
- ADR 0001's status line now names this record. Nothing else in it changes.
