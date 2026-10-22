# 1690 — Toolkit zoom boundaries

Session 1690, 2026-10-22. Host investigation; ADRs 2214/2215. Provisional: live reproduction continues.

**Premise.** The owner reports Qt zoom lag, GTK blurry text and deep-zoom position jumps,
with ISO_32000-2_sponsored_EC3.pdf as the primary reproduction. These symptoms are not yet
all attributed. The old GTK allocation contract independently fails at device scale two:
a 200×160 texture occupies 400×320 physical pixels despite a smaller size request.

**Change.** GTK now allocates texture-native dimensions and applies device-to-logical scaling
in every complete placement transform. Finished origins retain device-pixel rounding; previews
retain fractional movement. An automatic owned-Xvfb witness checks fifteen reused placements
at scales 1, 2 and 1.5. A wrong-filter control proves zero child tests cannot pass silently.
Opt-in toolkit traces separate input, retrieval/copy, host follow-up and paint callbacks.
The Qt callback comment now accurately describes queued painting, not completed presentation.

**Gates.** Scoped warnings-denied all-target clippy for viewer-host/GTK/Qt and toolkit unit suites
passed: GTK 10, Qt 23; exit 0, 95 s hold, 1.29 GiB. Immutable release copy passed, exit 0,
249 s, 3.79 GiB; full-source comparison stayed equal. CPU software smoke7 passed, exit 0,
43 s, 0.29 GiB: each toolkit delivered 32 Zoom and six Scroll commands, with native-size final
paint receipts. Earlier smoke failures exposed synthetic no-op input and overly narrow trace
parsing; all evidence is retained. Exact parser controls reject missing input and scaled previews.

**Display evidence.** The first owner-loop attempt failed permissions before starting viewers.
Fractional Xwayland discovery reports Qt DPR 1.59375 and compositor vertical maximization.
Resize and pointer coordinates have different mappings; measured affine input calibration
reaches the requested anchor. Probe4 delivered only three of four paced commands and was
rejected. Those three events reached first Qt paint in 6.2–8.1 ms and native-size paint in
19–30 ms; the incomplete sequence is exploratory, not a performance verdict.

**Limits.** The reported user symptoms remain under investigation. No Qt performance fix is
asserted. Toolkit paint is not scanout; native-size placement is not pixel-coordinate proof.
Fixed's oversized allocation is clipped by its Overlay and is not itself viewport drift.
Xwayland evidence cannot establish native Wayland sharpness. Matched settled-pixel/anchor and
complete real-display comparisons, final toolkit gates and batch integration remain owed.
