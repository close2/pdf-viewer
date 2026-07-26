# ADR 0001 — Windowing toolkit: winit rather than Qt

Status: accepted, 2026-07-26. Supersedes the initial Qt/KDE choice.

## Context

The project initially planned Qt6 with KDE integration, driven by two requirements: a
file open/save dialog consistent with the rest of the desktop, and accessibility.

Qt is C++, so using it from Rust requires `cxx-qt`, which brings a CMake + moc build
alongside Cargo and an unsafe FFI boundary. That was assessed as the most fragile part
of the build.

Investigation of the actual environment changed the picture:

- `xdg-desktop-portal` 1.22.1 and `xdg-desktop-portal-kde` 6.7.3 are installed. The
  portal serves the real KDE file chooser to *any* client, regardless of toolkit, via
  the `ashpd` crate. The dialog requirement does not need Qt.
- `AccessKit` exposes an accessibility tree over AT-SPI on Linux for custom-drawn
  interfaces, covering the second requirement.

The project owner confirmed Qt/KDE was a preference rather than a hard requirement,
and noted the interface has little conventional GUI.

## Decision

Use `winit` for windowing, `ashpd` for file dialogs through the XDG desktop portal,
and `AccessKit` for accessibility. Do not depend on Qt.

## Consequences

Positive:

- No C++ FFI boundary, so no `unsafe` in the shell layer and no CMake, moc, or
  Corrosion in the build. Cargo alone builds the project.
- The whole stack is one language, which serves the goal of a codebase others can
  learn from (`CLAUDE.md` principle 4).
- Native desktop dialogs are still used, and under a non-Plasma desktop the portal
  serves that desktop's dialog instead — arguably better integration than hard-coding
  a single toolkit's chooser.

Negative:

- No mature widget set. If the interface later needs conventional widgets —
  preferences panels, or AcroForm editing UI — they must be built or a UI crate
  adopted.
- Qt's i18n, menu and shortcut infrastructure must be replaced with equivalents.
- `winit`'s current release is a pre-release series; the version is pinned and
  upgrades reviewed.

Reversible, but at real cost once the shell is written. Revisit only if conventional
widget requirements appear.
