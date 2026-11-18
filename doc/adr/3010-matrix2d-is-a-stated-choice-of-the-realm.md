# 3010 — `Matrix2D` is in every document's realm, each member a stated choice

Session 2080. Status: **implemented.** Supersedes ADR 1602 section 7's reading that `Matrix2D` is a producer's
folder-level helper this realm need not carry. Context: review 2038 (finding B52). Source: the JavaScript for
Acrobat API Reference (A193 makes it the working source), `Doc.addLink` Example 2.

The reference's example constructs `new Matrix2D`, calls `fromRotated(this, p)`, `invert()` and `transform(q)`, to
"Convert quads in default user space to rotated User space used by Links". It documents the object nowhere else. So
the realm defines it (`pdf-script/src/engine/matrix2d.js`, evaluated at install), and every member is a choice
written beside it:

- the six numbers are ISO 32000-2 §8.3.3's `[a b c d e f]` named `a b c d h v`, the constructor's default is the
  identity;
- `invert()` is the inverse and a singular matrix throws a `RangeError`;
- `transform(points)` maps coordinate pairs of a flat array and maps each nested array in turn (a quad list);
- `fromRotated(doc, page)` maps the page's rotated user space onto default user space, rotated user space being
  the documented choice `getPageBox` already makes (`engine/pages.rs`), so the example's inverse lands in the space
  `getPageBox` answers in.

A document that defines its own `Matrix2D` replaces this one, as any global assignment does.

Test: `pdf-script::matrix2d_reference_spec_audit`.
