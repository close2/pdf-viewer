# ADR 2236 — Qt frame borrows end inside a local image sink

Preparing each image must not reconstruct the entire placement list for its count, descriptor
and pixels. The host takes one immutable placement snapshot and sends its descriptors and
borrowed pixel slices to a synchronous C++ image sink, in the existing painter order.

The sink is a local value in `showFrame`. It owns copied `QImage` values, their rectangles and
accounting only. It holds no host or window pointer, dispatches no events, and retains neither
Rust slices nor the temporary borrowed images. Each accepted image is copied exactly once.
The header forward-declares the sink without exposing Qt types. Its append function joins the
existing CXX declaration block; it adds no hand-written unsafe operation.

All Rust borrows end when collection returns. Only then may `showFrame` update the widget and
call `painted`, which can process host work and open another document. A sink that called back
into the host or pumped the event loop would break this contract even if the bridge compiled.

Returning a vector of borrowed shared descriptors was considered first. With CXX 1.0.199,
both a receiver-elided vector return and a receiver-elided shared wrapper around that vector
compile their C++ but fail in the generated Rust return shim's lifetime relationship. Explicit
function lifetimes require an unsafe-to-call declaration. Neither a new unsafe boundary,
persistent descriptor cache nor copied raster buffers is justified to force that return shape.
The synchronous sink avoids returning borrows through the bridge.

Transition rasters take their own single origin-zero descriptor. Ordinary frames retain the
shared partial, retained and oversized placement rules. Invalid rasters are refused before
crossing and checked again before the C++ read; empty or entirely invalid collections retain
the existing no-update behavior. This change does not alter that separate UI policy.
