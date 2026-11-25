# 3702 — A four-component page group costs the window two device frames: what was cut, and the band re-taken

Session 2250. Status: accepted (the owner's A1071). Context: ISO 32000-2 §11.4.7; ADR 3701;
`doc/checks/turn-path.toml`'s `personwithdog.pdf` row; ADRs 1531, 1969.

## What moved, and why it is not a defect of ADR 3701

`personwithdog.pdf` page 1 has a page group `<< /CS /DeviceCMYK /S /Transparency >>`, so `pdf_model`
hands the backends two lists: the complements of cyan, magenta and yellow, and the black list beside
them (`DisplayList::black`). Before ADR 3701 the window drew the first list and dropped the second
(review 2248, B02's class), so its turn band was taken on a picture with no black ink. Main and the
branch, measured back to back on a quiet machine by `cargo test --release -p render-raster --test
turn_path -- --ignored --nocapture`: turn 31.03 against 105.56 ms, seventh 28.66 against 105.51,
step 37.77 against 150.35. The page's own commands are 18 patch meshes.

## What was cut (applied)

- **The page's own rectangle, not the window** (`finish::finished_page`). Outside §14.11.2.1's
  page boundary a finished page is transparent, so drawing, reading back, converting and uploading
  the rest of a 1600 by 1000 window is work with no answer. The rectangle is the target under a
  whole-pixel translation (`tiles`' argument), so its pixels are the target's. At the turn's
  magnification the page is 612 by 792 pixels, 30 % of the window.
- **§11.4.7's conversion across rayon's pool** (`pdf_render::blending::resolve`), with the equal
  pixel's answer copied, on `resolve_cube`'s argument (ADR 1531): neither changes a byte. The CPU
  oracle runs the same function and gains the same.

Measured under the lock (`bounded.sh --lock`, the probe being `frame_cost::round` for this page
alone): turn 75 to 90 ms, seventh 69 to 102, step 134 to 195. The turn gate's own run, not judged at
a load average of 106 to 216 against a ceiling of 48: turn 79.62, seventh 80.37, step 112.30.

## What is left, and why it is the clause's

A turn of this page is now about 30 ms for the page's own device frame (what main's whole turn was),
about 20 ms for the black list's, about 15 ms for the passes over 0.48 million pixels, and the
upload of the page's rectangle. `rasterize` of the same list costs 38 to 46 ms on the same device.
§11.4.7 makes the composite four components, and the device composites three channels and alpha per
frame, so a correct picture needs a second frame until the device draws both planes in one. That is
a shader beside `pdf_render::blending` that would have to answer identically, and a fourth statement
of a pass is what ADR 3701 declined.

## The band (A1071)

The owner answered Q1071: the `personwithdog.pdf` row's `turn`, `seventh` and `step` bands are
re-taken from a quiet, judged run of the fixed tree, and the row's `why` says what the page now
costs. They are re-taken by `tools/bands.sh retake`'s procedure, run for the turn gate alone, with
ADR 1969's rule applied by `tools/bands.py` and only this row (and A1073's) copied into
`doc/checks/machines/epyc-7301-32-rx580-vulkan/turn-path.toml`; every other row is unchanged.

The cheaper construction, which A1071 leaves as a later optimisation: both planes in one device
frame, as two render targets of one encode, with `blending::resolve` as a compute pass reading them.
It saves the second frame and the two read-backs, and costs the equality with the CPU oracle that
one function gives. 