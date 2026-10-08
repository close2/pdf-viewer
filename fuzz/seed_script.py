#!/usr/bin/env python3
"""Seed the `script` fuzz target with scripts: every member the bridge carries and refuses, the
engine's library, the budgets at and past their bounds, and the malformed.

    python3 fuzz/seed_script.py fuzz/corpus/script

**Why a generator.** The target's input is a selector byte — the site in its low four bits, the
unsaved mark, a commit by Enter, an asker that answers, and `willCommit` in the four above — then a
script, the event's value and a keystroke's change,
NUL-separated (`fuzz/fuzz_targets/script.rs`). A mutator handed nothing will not spell
`event.value` or `this.getField` before it finds a parser error, so each is written here: the
`event` members a field event carries, a field's properties read and set, the `AF*` library, the
refused calls of RFC 0008 section 4.3 by name, a document-level function defined and called, and
the language's own library. Then each budget of ADR 1590 at its number and one past it — loop
iterations, recursion, a string, an array, a buffer — and what no per-call budget sees. Then the
malformed: an unterminated string, a regular expression that never closes, a script of one token
repeated to the campaign's `-max_len` (trap 117), and brackets nested to the target's own limit.
Every script is written at every site, the commit bit both ways. Each seed is named by its
content, so a re-run adds nothing new.
"""

import hashlib
import os
import sys

# The campaign's `-max_len` (`doc/verify.md`'s `script` line): a seed at it is the bound's own.
MAX_LEN = 8192

SCRIPTS = [
    # The event, read and written.
    "event.value = '<' + event.value + '>';",
    "if (event.change == 'x') event.rc = false;",
    "event.change = event.change.toUpperCase();",
    "if (event.willCommit) event.value = event.value.trim(); else event.rc = event.change != ' ';",
    "event.value = event.selStart + ':' + event.selEnd + ':' + event.change.length;",
    "event.value = String(event.target.name) + event.target.value;",
    "event.value = event.source ? event.source.name : 'none';",
    "event.value = event.type + '/' + event.name + '/' + event.targetName;",
    # Fields, read and set.
    "var f = this.getField('Total'); event.value = f.value + f.type + f.display;",
    "this.getField('Amount').value = Number(this.getField('Total').value) * 2;",
    "var f = this.getField('Agree'); f.display = display.hidden; f.readonly = true;",
    "var f = this.getField('Total'); f.fillColor = color.red; f.textColor = ['RGB', 0, 0.5, 1];",
    "var f = this.getField('Choice'); f.borderStyle = border.d; f.alignment = 'center';",
    "var f = this.getField('Total'); f.charLimit = 4; f.required = true; event.value = f.rect;",
    "var t = 0; ['Total', 'Amount'].forEach(function (n) { t += +this.getField(n).value; }, this);"
    " event.value = String(t);",
    "event.value = this.numPages + ':' + this.pageNum;",
    "this.getField('Missing').value = 1;",
    # The library.
    "AFNumber_Format(2, 0, 0, 0, '$', true);",
    "AFNumber_Keystroke(2, 0, 0, 0, '', true);",
    "AFPercent_Format(1, 0);",
    "AFDate_FormatEx('mm/dd/yyyy');",
    "AFSimple_Calculate('SUM', ['Total', 'Amount']);",
    "AFSpecial_Keystroke(2);",
    "event.value = util.printf('%.2f', 3.14159) + util.printd('yyyy-mm-dd', new Date(0));",
    "event.value = util.printf('%,0+08.3f|%x|%s|%%|%05d', -1234.5, 255, Math.PI, 7);",
    "event.value = util.printf('%,9d', 1);",
    "event.value = util.printf('%1025d', 1);",
    # The members ADR 1626 carries.
    "global.radius = 8; event.value = util.printf('%.5f', (4/3) * Math.PI * Math.pow(global.radius, 3));",
    "global.setPersistent('radius', true);",
    "event.value = [event.commitKey, event.fieldFull, event.changeEx].join();",
    "var b = this.dirty; this.dirty = false; event.value = String(this.dirty); this.dirty = b;",
    "for (var k in this.info) event.value += k + '=' + this.info[k] + ';'; this.info.Title = 1;",
    "var g = this.getOCGs(); g[0].state = !g[0].state; g[1].state = false; g[0].initState = 1;",
    "this.getOCGs(0);",
    "var b = this.getField('Send'); b.buttonSetCaption(b.buttonGetCaption() + '!', 2);",
    "this.getField('Total').buttonSetCaption('x');",
    # The questions ADR 1627 puts.
    "event.value = app.alert({cMsg: 'Sure?', nIcon: 2, nType: 3, cTitle: 'Form'}) + app.alert('again');",
    "var c = {cMsg: 'box', bInitialValue: true}; app.alert('x', 0, 1, 't', this, c); event.value = c.bAfterValue;",
    "event.value = String(app.response({cQuestion: 'Name?', cDefault: 'x', bPassword: true}));",
    # The members ADRs 1688 to 1737 carry: a focus, a check box, timers and a sound, the
    # annotations, the pages and their labels, a named destination, the window's view, and a
    # choice's options read and rewritten.
    "this.getField('Total').setFocus(); this.getField('Agree').checkThisBox(0, true);",
    "event.value = String(this.getField('Agree').isBoxChecked(0));",
    "var t = app.setTimeOut('event.value = 1;', 10); app.clearTimeOut(t);",
    "var i = app.setInterval('this.dirty = true;', 1); app.clearInterval(i); app.beep(0);",
    "var a = this.getAnnots(); event.value = a ? a.length : 0; var n = this.getAnnot(0, 'x');",
    "event.value = this.getPageLabel(0) + this.getPageBox('Crop', 0) + this.getPageRotation(0);",
    "this.pageNum = this.numPages - 1; this.gotoNamedDest('Chapter1'); this.calculate = false;",
    "event.value = app.activeDocs.length + this.title;",
    "this.zoom = 150; this.zoomType = zoomtype.fitW; event.value = this.zoom + this.zoomType;",
    "this.layout = 'TwoColumnLeft'; this.scroll(10, 20); event.value = this.layout;",
    "this.zoom = 1e9; this.zoom = -1; this.zoomType = 'Nonsense'; this.layout = 7; this.scroll();",
    "app.goBack(); app.goForward();",
    "var f = this.getField('Choice'); event.value = f.numItems + f.getItemAt(0, false)"
    " + f.currentValueIndices;",
    "var f = this.getField('Choice'); f.setItems(['a', ['B', 'b'], 'c']); f.insertItemAt('d', 'D', 0);"
    " f.deleteItemAt(1); event.value = f.numItems;",
    "var f = this.getField('Choice'); f.clearItems(); f.insertItemAt('x'); f.deleteItemAt(-1);",
    "this.getField('Choice').setItems(new Array(70000).fill('x'));",
    # Deep without brackets, and deep at run time.
    "event.value = eval('1' + '+1'.repeat(30000));",
    "event.value = Function('return ' + '!'.repeat(5000) + '1')();",
    # A document-level function, defined and called.
    "function total() { return 42; } event.value = String(total());",
    "var global = global || {}; global.count = (global.count || 0) + 1; event.value = global.count;",
    "console.println('logged ' + event.value);",
    # Refused by name.
    "app.launchURL('https://example.com');",
    "this.submitForm('https://example.com');",
    "this.exportDataObject({cName: 'x'});",
    "event.keyDown;",
    "try { app.launchURL('x'); } catch (e) { event.value = e.name + ': ' + e.message; }",
    # The language's library.
    "event.value = JSON.stringify(JSON.parse('{\"a\": [1, 2, {\"b\": null}]}'));",
    "event.value = '1,234.50'.replace(/[^0-9.]/g, '').match(/(\\d+)\\.(\\d+)/)[2];",
    "event.value = new Date(2024, 1, 29).toISOString() + Date.now();",
    "event.value = String(Math.round(Math.random() * 0) + Math.max(1, 2));",
    "var m = new Map([[1, 2]]); var s = new Set([1, 1, 2]); event.value = m.size + ':' + s.size;",
    "var a = [3, 1, 2]; a.sort(function (x, y) { return x - y; }); event.value = a.join('|');",
    "event.value = escape('ä b') + unescape('%41') + 'abc'.substr(1);",
    "event.value = (12345.678).toFixed(2) + (0.1 + 0.2).toPrecision(3) + parseInt('0x1F');",
    "var t = new Uint8Array(4); t.set([1, 2, 3]); event.value = Array.from(t).join();",
    "class A { get v() { return 1; } } event.value = String(new A().v);",
    "event.value = [..._x()].join(); function* _x() { yield 1; yield 2; }",
    "var o = {}; Object.defineProperty(o, 'v', {get: function () { return 7; }}); event.value = o.v;",
    "event.value = `${event.value}-${1 + 1}`;",
    "label: for (var i = 0; i < 3; i++) { for (;;) { continue label; } }",
    "event.value = typeof null + typeof undefined + typeof function () {};",
    # Budgets at their numbers and past them.
    "while (true) {}",
    "for (var i = 0; i < 100000; i++) {}",
    "for (var i = 0; i < 100001; i++) {}",
    "function f(n) { return n ? f(n - 1) : 0; } f(511);",
    "function f(n) { return n ? f(n - 1) : 0; } f(100000);",
    "var s = 'x'.repeat(100000);",
    "var s = 'x'.repeat(16777216);",
    "var s = 'x'.padStart(16777217, 'y');",
    "var a = new Array(1048576).fill(0);",
    "var a = new Array(1048577).fill(0);",
    "var b = new ArrayBuffer(16777216);",
    "var b = new ArrayBuffer(16777217);",
    "var a = []; for (var i = 0; i < 1000; i++) a.push(i); event.value = a.concat(a, a).length;",
    "function f() { for (var i = 0; i < 90000; i++) {} } for (;;) f();",
    "var s = 'x'; for (var i = 0; i < 20; i++) s += s; event.value = String(s.length);",
    "var a = 1; for (var i = 0; i < 100; i++) a = [a]; event.value = String(a);",
    # The malformed.
    "event.value = 'never ends",
    "event.value = /never closes",
    "event.value = ;",
    "}{",
    "événement = 1;",
    "event.value = '\\u{1F600}\\uD800';",
    "",
    "(" * 200 + "1" + ")" * 200,
    "[" * 256 + "]" * 256,
    "{" * 100 + "}" * 100,
    "!" * 1000 + "1",
    "1 ? 1 : " * 500 + "1",
    "if (1) " * 500 + "event.value = 'x';",
    "x=>" * 1000 + "1",
    "a = " * 2000 + "1",
    "if (x) {} " + "else if (x) {} " * 400,
]

VALUES = [("12", ""), ("", "x"), ("-1234.5678", "5"), ("é€ ", "€")]


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: seed_script.py <corpus directory>")
    directory = sys.argv[1]
    os.makedirs(directory, exist_ok=True)
    scripts = list(SCRIPTS)
    # A script of one statement repeated to the campaign's input bound, and one of one token.
    statement = "event.value += 'x';"
    scripts.append(statement * ((MAX_LEN - 16) // len(statement)))
    scripts.append("1" + " +1" * ((MAX_LEN - 16) // 3))
    written = 0
    for script in scripts:
        for selector in range(16):
            for commit in (0x00, 0x70, 0x80, 0xF0):
                value, change = VALUES[(selector + len(script)) % len(VALUES)]
                data = (bytes([selector | commit]) + script.encode("utf-8") + b"\0"
                        + value.encode("utf-8") + b"\0" + change.encode("utf-8"))
                if len(data) > MAX_LEN:
                    data = data[:MAX_LEN]
                name = hashlib.sha1(data).hexdigest()
                path = os.path.join(directory, name)
                if not os.path.exists(path):
                    with open(path, "wb") as out:
                        out.write(data)
                    written += 1
    print(f"seed_script.py: {written} seeds in {directory}")


if __name__ == "__main__":
    main()
