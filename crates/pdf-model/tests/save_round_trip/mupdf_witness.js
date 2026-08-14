// What mupdf sees in a file this program saved — the save round-trip's third assertion
// (ADR 0334, instrument 2 of ADR 0323), run under `mutool run`.
//
// Asked exactly one question: "open this file and say what is in it". The answer comes from
// mupdf's raw object layer — the trailer, the page tree, the interactive form tree — so what
// this exercises is mupdf's reading of §7.5.6's update chain: the appended cross-reference
// section found, its entries winning over the sections beneath, the new objects parsed and
// (where the document is encrypted) decrypted with the document's own key. Coordinates are the
// objects' own numbers in default user space, so no page-space convention of mupdf's is in the
// answer at all — trap 3's lesson, applied by leaving the frame out rather than by auditing it.
//
// Output, one record per line, tab-separated:
//   freetext\t<contents>\t<x0>\t<y0>\t<x1>\t<y1>     (page one's /Annots, raw /Rect)
//   field\t<qualified name>\t<value>                 (every terminal field; /V inherited per Table 226)
// An uncaught throw exits nonzero, which is how a file mupdf cannot open reports.

var path = scriptArgs[0];
var password = scriptArgs.length > 1 ? scriptArgs[1] : "";

var doc = new PDFDocument(path);
if (doc.needsPassword() && !doc.authenticatePassword(password)) {
	// mutool's own tools warn and continue here ("warning: cannot authenticate password"),
	// because a document whose string filter is Identity is readable without the key — so
	// this does the same, and says so, rather than inventing a refusal mupdf does not make.
	print("auth\tfailed");
}

function cell(text) {
	return text.replace(/[\t\n\r]/g, " ");
}

// Page one's free text annotations, from the page object's own /Annots array.
var page = doc.findPage(0);
var annots = page.Annots;
if (annots && annots.isArray()) {
	for (var i = 0; i < annots.length; i++) {
		var annotation = annots.get(i);
		if (!annotation || !annotation.isDictionary()) continue;
		var subtype = annotation.Subtype;
		if (!subtype || !subtype.isName() || subtype.asName() !== "FreeText") continue;
		var contents = annotation.Contents;
		var rect = annotation.Rect;
		if (!rect || !rect.isArray() || rect.length !== 4) continue;
		print(
			"freetext\t" +
				cell(contents && contents.isString() ? contents.asString() : "") +
				"\t" + rect.get(0).asNumber() +
				"\t" + rect.get(1).asNumber() +
				"\t" + rect.get(2).asNumber() +
				"\t" + rect.get(3).asNumber()
		);
	}
}

// Every terminal field of §12.7.3's tree, named by §12.7.4.2's rule: a kid contributes its /T
// where it has one, and a widget kid with no /T is the same field. Table 226 marks /V
// inheritable, so the value in force is carried down the walk.
function walk(node, prefix, inherited, depth) {
	if (depth > 32 || !node || !node.isDictionary()) return;
	var name = prefix;
	var t = node.T;
	if (t && t.isString()) {
		name = prefix === "" ? t.asString() : prefix + "." + t.asString();
	}
	var value = node.V !== undefined && node.V !== null ? node.V : inherited;
	var kids = node.Kids;
	var terminal = true;
	if (kids && kids.isArray()) {
		for (var i = 0; i < kids.length; i++) {
			var kid = kids.get(i);
			if (!kid || !kid.isDictionary()) continue;
			var widget =
				kid.Subtype && kid.Subtype.isName() && kid.Subtype.asName() === "Widget";
			if (widget && !kid.T) continue; // the same field's widget, not a child field
			terminal = false;
			walk(kid, name, value, depth + 1);
		}
	}
	if (terminal) {
		print(
			"field\t" + cell(name) + "\t" +
				cell(value && value.isString() ? value.asString() : "")
		);
	}
}

var acroform = doc.getTrailer().Root.AcroForm;
if (acroform && acroform.isDictionary()) {
	var fields = acroform.Fields;
	if (fields && fields.isArray()) {
		for (var i = 0; i < fields.length; i++) {
			walk(fields.get(i), "", null, 0);
		}
	}
}
