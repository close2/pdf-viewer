# 2291 — Rich media is presented embedded or windowed, and its instance matched to its asset

Lane `gwg`: `doc/todo/80` section 1 steps 2 and 3 (RFC 0015 section 10.1). ADRs 3865 to 3867; no
question.

- **Presentation** (ADR 3865): `pdf_model::media::rich_media::presentation` reads Table 338's
  `/Style`, Table 339's `/Width` and `/Height` and Table 340's position. `Embedded`, the default
  every processor "shall support", is the annotation's rectangle. `Windowed` is a Table 295 floating
  window relative to the document window — this program's presentation area — at the corner Table
  340's alignment names (`HAlign` reversed for a right-to-left catalog `/Lang`), moved by the offsets
  and held within the document window (`Floating::held_within`, `viewer_host::video::screens::place`).
  One user space unit is one pixel (a choice). Hosts that open no media window (Qt, `viewer-ffi`) or
  place none (GTK 4) present it `Embedded`, said; the winit window places one.
- **Flags** (ADR 3866): `/Transparent` sets the rectangle's background opacity (H.264 pictures carry
  no alpha); `/Toolbar` is the controller bar; `/PassContextClick` true and `/NavigationPane` true
  are said: this program's player provides no context-click interface and a video has no hierarchy.
- **Instances** (ADR 3867): every instance of the configuration's kind is tried in order and the
  first whose asset the players take plays; the asset's type is its Table 44 `/Subtype`, else its
  file name's extension (a choice); a type of the other kind is Table 343's mismatch, said and
  passed over; other kinds' instances are said as not loaded. `activated` now answers `Activated`,
  the clip and its sentences.
- `viewer-ui`'s `Asked::Media` boxes its `Handed`, which the new field took past `large_enum_variant`.
- **Tests**: three `rich_media.rs` units, one `screens.rs` unit, one in
  `viewer-core/tests/rich_media_activation.rs`; the protocol's window round trip carries the field.
- **Ledger**: no status moves (the todo keeps the rich media rows `out-of-scope` until section 1's
  last step); 13.7.2.1, 13.7.2.2, 13.7.2.2.1 to .3, .5, .6, 13.7.2.3, .3.3 and .3.4 restated.
- Left, in order: `doc/todo/80` section 1 steps 4 (views and snapshots) and 5 (animation); then the
  rich media rows' status.

**Gates.** Over `-p pdf-model -p pdf-media-worker -p viewer-core -p viewer-host -p viewer-confined
-p viewer-qt -p viewer-gtk -p viewer-ui -p viewer-ffi -p conformance`: fmt 0, clippy `-D warnings
--all-targets` 0, `pdf-sandbox`/`pdf-media-worker --bins` 0, nextest `--lib --bins --tests`
(`xvfb-run -a`) 5339 of 5340, the one `repair_census` on eight unmarked defaults this round wrote,
marked and passing on the conformance run (manifest: none expected); rustdoc (five crates, private
directory) 0, fuzz 0, conformance 0. Launch 101 twice: `PDF20_AN001-BPC`'s `peak_anon_mib` 6.656
and 6.520 of ..6.500, as round 2290 saw on main. HEAD 995e2eaee in this worktree read 0 outside: page
one's frame hash and first-frame instructions (10873.3 against 10873.4 thousand) are the same, the
allocation at first page is 19.5 MiB at HEAD against 19.0 and 19.2 here, and the subtracted median
of the device's own allocation was 12.99 MiB at HEAD against 12.58 and 12.69 here — the device's
noise, not this change, whose code nothing on the launch path reaches before `Command::Presented`.
