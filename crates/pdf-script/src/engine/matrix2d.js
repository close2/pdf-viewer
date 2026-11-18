// `Matrix2D`, the affine matrix the JavaScript for Acrobat API Reference's `Doc.addLink` Example 2
// constructs, inverts and applies (ADR 3010). The reference names the object in that example and
// documents it nowhere, so each member below is a stated choice rather than a reading:
//
// - the six numbers are ISO 32000-2 §8.3.3's `[a b c d e f]`, named `a b c d h v` as the
//   example's own era named them, with `x' = a·x + c·y + h` and `y' = b·x + d·y + v`, and
//   `new Matrix2D()` is the identity;
// - `invert()` is the inverse, and a matrix with none is refused by a `RangeError` rather than
//   answered with numbers no point maps to;
// - `transform(points)` maps the coordinate pairs of an array of numbers, and maps each member of
//   an array of such arrays — a quad list as `getPageNthWordQuads` answers one — in turn;
// - `fromRotated(doc, page)` maps the page's rotated user space onto default user space, the
//   space `getPageBox` answers in (`engine/pages.rs`), so that its inverse is the conversion the
//   example comments "Convert quads in default user space to rotated User space used by Links".
function Matrix2D(a, b, c, d, h, v) {
  this.a = a === undefined ? 1 : Number(a);
  this.b = b === undefined ? 0 : Number(b);
  this.c = c === undefined ? 0 : Number(c);
  this.d = d === undefined ? 1 : Number(d);
  this.h = h === undefined ? 0 : Number(h);
  this.v = v === undefined ? 0 : Number(v);
}

Matrix2D.prototype.invert = function () {
  var det = this.a * this.d - this.b * this.c;
  if (det === 0 || !isFinite(det)) {
    throw new RangeError("Matrix2D.invert: the matrix has no inverse");
  }
  return new Matrix2D(
    this.d / det,
    -this.b / det,
    -this.c / det,
    this.a / det,
    (this.c * this.v - this.d * this.h) / det,
    (this.b * this.h - this.a * this.v) / det
  );
};

Matrix2D.prototype.transform = function (points) {
  var m = this;
  function mapped(list) {
    var out = [];
    for (var i = 0; i < list.length; i++) {
      if (typeof list[i] === "object" && list[i] !== null) {
        out.push(mapped(list[i]));
      } else if (i + 1 < list.length) {
        var x = Number(list[i]);
        var y = Number(list[i + 1]);
        out.push(m.a * x + m.c * y + m.h, m.b * x + m.d * y + m.v);
        i++;
      } else {
        out.push(list[i]);
      }
    }
    return out;
  }
  return mapped(points);
};

Matrix2D.prototype.fromRotated = function (doc, page) {
  var rotate = doc.getPageRotation(page);
  // [left, top, right, bottom] in rotated user space, whose lower-left corner is the media box's
  // own lower-left corner in default user space.
  var box = doc.getPageBox("Media", page);
  var x0 = Math.min(box[0], box[2]);
  var y0 = Math.min(box[1], box[3]);
  var across = Math.abs(box[2] - box[0]);
  var up = Math.abs(box[1] - box[3]);
  var turned = rotate === 90 || rotate === 270;
  var w = turned ? up : across;
  var h = turned ? across : up;
  var toRotated;
  if (rotate === 90) {
    toRotated = new Matrix2D(0, -1, 1, 0, x0 - y0, y0 + x0 + w);
  } else if (rotate === 180) {
    toRotated = new Matrix2D(-1, 0, 0, -1, 2 * x0 + w, 2 * y0 + h);
  } else if (rotate === 270) {
    toRotated = new Matrix2D(0, 1, -1, 0, x0 + y0 + h, y0 - x0);
  } else {
    toRotated = new Matrix2D();
  }
  return toRotated.invert();
};
