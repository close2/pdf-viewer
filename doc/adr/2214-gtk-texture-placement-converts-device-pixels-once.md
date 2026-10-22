# ADR 2214 — GTK texture placement converts device pixels once

A raster's dimensions and placement extent are device pixels. `GtkFixed` positions and
allocations are logical pixels. A `GtkPicture` that cannot shrink still requests its texture's
natural dimensions; setting a smaller size request does not reduce that natural allocation.
Consequently a 200×160 texture requested at 100×80 on a scale-two display still occupies
200×160 logical pixels unless the child transform performs the device-to-logical conversion.

Keep the picture's allocation in its texture's native dimensions. Apply one transform that
translates the desired device origin divided by device scale, then scales by desired device
extent divided by texture dimensions and device scale. A finished raster therefore occupies
one device pixel per texel. A stand-in additionally scales to the requested zoom extent until
the matching raster arrives. Finished origins retain the existing whole-device-pixel rounding;
preview origins retain their fractional movement.

Do not rely on a transform left by a previous placement. Every placement supplies the complete
translation and scale. The existing `gtk_fixed_move` implementation replaces its transform;
retention of an old scale was investigated and is not the demonstrated defect.

An automatic Linux GTK allocation witness reuses the same picture across finished and preview
placements, negative origins, growing and shrinking textures, and scales one, two and one-and-a-half.
The fractional value exercises transform arithmetic; GTK's reported display scale remains an
integer. Its owned Xvfb child uses Cairo and verifies actual widget bounds, rather than a copy
of the transform formula. The parent requires an explicit successful-placement receipt, so a
stale exact test filter returning zero tests cannot pass. Display startup and child execution
are bounded, and cleanup owns only the processes the witness started. CI explicitly installs
Xvfb, x11-utils and coreutils. This X11 harness does not establish native macOS/Windows behavior.

The page's Fixed is an unmeasured child of a clipped Overlay. Its natural allocation may exceed
the visible viewport without enlarging that viewport. A diagnostic of Fixed's allocation must
not be interpreted as the core viewport or as evidence of document drift.

This allocation defect is independently demonstrated at scale two. It does not establish the
cause of every reported blur or deep-zoom position error. Xwayland compositor scaling and native
Wayland presentation need separate observations before claiming the owner's symptoms resolved.
