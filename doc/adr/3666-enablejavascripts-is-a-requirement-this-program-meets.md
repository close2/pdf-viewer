# 3666 — `EnableJavaScripts` is a requirement this program meets

Session 2241. Status: accepted. Context: review 2239 (D01), ISO 32000-2 §12.11.2 Table 275,
RFC 0008, ADRs 1616 and 1752, `doc/questions/A100`, A946, A1006.

## Decision

`requirements::Kind::unmet` answers `None` for `EnableJavaScripts`. Table 275 asks for "support for
execution of ECMAScripts appearing in ECMAScript actions and in the ECMAScript name tree for
document-level ECMAScripts". Both run: an action chain's `JavaScript` actions are handed to the
runner (`view::script_sites`, ADR 1752) and the `/Names` `/JavaScript` tree runs when the document
opens, in the confined `pdf-script-worker` at the level the reader sets.

That level is `off` until a host says otherwise (ADR 1616), and that does not make the requirement
unmet. It is the reader's restriction on a capability the program has, the same as
`SeparationSimulation`, which is met as "a requirement executed under a control a host supplies"
(`doc/questions/A100`). What a script may not reach, every call that leaves the process, is an API
Table 275 does not name. The exclusion in `CLAUDE.md` is bounded the same way.

The other two decayed reasons are restated and stay unmet, because each names a part Table 275
asks for that is not built:

- **`Multimedia`**: renditions are judged and played, audio by this program (A946) and H.264
  video (A1006). A media clip that is a form `XObject` (Table 285) is named and not presented.
- **`Geospatial2D`**: a position is answered for a system stated in WKT. A system named by its
  EPSG code alone, or by a projection or datum method that is not carried, is refused by name.

`Geospatial3D`, `RichMedia` and the 3D types each get their own reason. The 3D types are on the
exclusion list, and rich media assets are listed and saved, not played.

**The penalty total follows.** A document's `EnableJavaScripts` penalty no longer counts toward
§12.11.3's threshold.
