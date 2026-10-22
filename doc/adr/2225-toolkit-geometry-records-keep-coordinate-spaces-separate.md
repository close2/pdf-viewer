# ADR 2225 — Toolkit geometry records keep coordinate spaces separate

Toolkit logical coordinates, core device pixels, input-tool coordinates and screenshot pixels
are different spaces. Fractional compositor scaling can give input movement and window resize
different mappings. Do not derive screenshot crops from an input calibration.

The opt-in geometry record describes the actual content widget: GTK's Overlay bounds in its
root widget, or Qt's PageArea origin in its client window. It names logical content and client
extents, content scale and surface scale separately. Device extents obtained by multiplication
are explicitly labelled derived and remain unrounded. An unavailable origin or surface value
is null, not zero. GTK's Fixed natural allocation is recorded separately; it can exceed the
clipped Overlay without changing the visible viewport.

An independent core-viewport receipt accompanies an accepted resize call. Widget geometry does
not prove that a busy host accepted that resize. A driver compares the latest accepted viewport
with actual content geometry before treating two runs as matched. These observations do not
introduce a cached viewport authority or change layout.

Backend identity comes from the active GTK display type or Qt platform, not the requested
environment variable. Widget and surface scale do not by themselves establish a compositor's
fractional scaling or final screenshot transform. Xwayland observation is not native Wayland
reproduction. A native backend must be observed independently before attributing blur to it.

Geometry is separate from paint metadata. A completed opaque foreground may hide a scaled
retained underlay, while a partial frame can contain transparent rows. Native-size dimensions
alone establish neither opacity nor current-target acceptance. Toolkit paint callbacks are
not scanout; unavailable presentation feedback stays unavailable. A programmatic native-window
zoom can test rendering but cannot establish physical wheel delivery or pointer anchoring.
