#!/usr/bin/env python3
"""Count, in each XFA resource `xfa_dynamic_census --dump` wrote, the template grammar that laying
out page one of a dynamic form would have to run.

The population is the dump directory the example wrote, one `<line>.xdp` per document that states
an XFA resource; `--census <tsv>` joins the example's lines so each row carries its path and its
verdict. The counts are of XFA 3.3's own element and attribute names, each with the chapter that
gives it its layout meaning (doc/todo/78 reads them):

  pageSet, pageArea, contentArea   chapter 8, "Flowing Between ContentArea Objects" and the
                                   pagination strategies (pages 288-300)
  subform layout                   chapter 8, flowed layout: position, tb, lr-tb, rl-tb, row,
                                   table, paginate (page 274 onwards)
  occur min/max/initial            chapter 9, page 333: a subform is dynamic where they differ or
                                   the maximum is -1 (unbounded)
  subformSet                       chapter 9, page 333: a form holding one is dynamic
  breakBefore, breakAfter, break   chapter 8, "Break Conditions" (pages 296-300)
  overflow, leader, trailer        chapter 8, overflow leaders and trailers (pages 300-305)
  keep                             chapter 8, adhesion (pages 305-307)
  draw                             boilerplate set at run time: text from font metrics
  exData                           rich text, chapter 27
  bind match                       chapter 4 and chapter 9's data binding
  script                           by contentType, JavaScript or FormCalc
  instanceManager / _name          a script that changes the number of instances (page 273)
  pageCount / absPageCount         a script that reads the whole layout's page count (chapter 8,
                                   the layout DOM's methods), which page one cannot know alone

`--self-test` plants one resource holding each of these and one holding none, and asserts the
counts; a conformance test runs it (tools/conformance/tests/xfa_census.rs), so the clean answer
of a corpus run is read only from an instrument shown to find what was planted (trap 13).
"""

import argparse
import collections
import os
import re
import sys
import tempfile
import xml.etree.ElementTree as ElementTree

TEMPLATE = re.compile(r"^\{http://www\.xfa\.org/schema/xfa-template/([0-9.]+)/\}template$")
DATA = "{http://www.xfa.org/schema/xfa-data/1.0/}data"
FLOWED = ("tb", "lr-tb", "rl-tb", "row", "table", "paginate")


def local(tag):
    return tag.rsplit("}", 1)[-1] if isinstance(tag, str) else ""


def find_template(root):
    """The template element and its version: the root's child, else the first at any depth."""
    for element in root.iter():
        match = TEMPLATE.match(element.tag) if isinstance(element.tag, str) else None
        if match:
            return element, match.group(1)
    return None, None


def count(text):
    """One row of counts for one resource's bytes, or a row naming why it could not be read."""
    row = collections.OrderedDict()
    try:
        root = ElementTree.fromstring(text)
    except ElementTree.ParseError as error:
        # A producer's bare ampersand in its data is the one malformation the corpora show; it is
        # escaped and the parse retried, and the row says so, so a count is never silently of a
        # repaired text.
        repaired = re.sub(rb"&(?!#?\w+;)", b"&amp;", text)
        try:
            root = ElementTree.fromstring(repaired)
            row["repaired"] = "bare-ampersand"
        except ElementTree.ParseError:
            row["error"] = f"xml: {error}"
            return row
    template, version = find_template(root)
    if template is None:
        row["error"] = "no template packet"
        return row
    row["version"] = version
    tally = collections.Counter()
    layouts = collections.Counter()
    binds = collections.Counter()
    languages = collections.Counter()
    variable = 0
    unbounded = 0
    instance_scripts = 0
    page_count_scripts = 0
    typefaces = set()
    for element in template.iter():
        name = local(element.tag)
        tally[name] += 1
        if name == "subform":
            layouts[element.get("layout", "position")] += 1
        elif name == "occur":
            low = element.get("min", "1")
            high = element.get("max", "1")
            initial = element.get("initial", low)
            if len({low, high, initial}) > 1:
                variable += 1
            if high == "-1":
                unbounded += 1
        elif name == "bind":
            binds[element.get("match", "once")] += 1
        elif name == "script":
            languages[element.get("contentType", "application/x-formcalc")] += 1
            body = element.text or ""
            if "instanceManager" in body or re.search(r"(^|[^\w])_\w+\.(addInstance|setInstances|removeInstance)", body):
                instance_scripts += 1
            if re.search(r"\b(absPageCount|pageCount|absPageCountInBatch)\s*\(", body):
                page_count_scripts += 1
        elif name == "font" and element.get("typeface"):
            typefaces.add(element.get("typeface"))
    for name in ("pageSet", "pageArea", "contentArea", "subform", "subformSet", "field",
                 "exclGroup", "draw", "exData", "breakBefore", "breakAfter", "break",
                 "overflow", "leader", "trailer", "keep", "image", "occur"):
        row[name] = tally[name]
    row["flowed"] = sum(layouts[value] for value in FLOWED)
    row["layouts"] = ",".join(f"{k}:{v}" for k, v in sorted(layouts.items()))
    row["occur-variable"] = variable
    row["occur-unbounded"] = unbounded
    row["binds"] = ",".join(f"{k}:{v}" for k, v in sorted(binds.items()))
    row["javascript"] = languages["application/x-javascript"]
    row["formcalc"] = languages["application/x-formcalc"]
    row["instance-scripts"] = instance_scripts
    row["page-count-scripts"] = page_count_scripts
    row["typefaces"] = len(typefaces)
    # The datasets packet's `xfa:data`, by its namespace: a `config` packet holds a `data`
    # element of its own, the data-loading options (XFA 3.3 chapter 30), which is not the form's.
    data = next((e for e in root.iter() if e.tag == DATA), None)
    row["data-nodes"] = sum(1 for _ in data.iter()) - 1 if data is not None else -1
    return row


def census(dump, tsv):
    lines = {}
    if tsv:
        with open(tsv, encoding="utf-8") as handle:
            for raw in handle:
                parts = raw.rstrip("\n").split("\t")
                if len(parts) >= 5:
                    lines[parts[0]] = parts[1:5]
    rows = []
    for entry in sorted(os.listdir(dump), key=lambda n: (len(n), n)):
        if not entry.endswith(".xdp"):
            continue
        with open(os.path.join(dump, entry), "rb") as handle:
            text = handle.read()
        key = entry[:-4]
        row = collections.OrderedDict()
        row["line"] = key
        path, needs, resource, verdict = lines.get(key, ("?", "?", "?", "?"))
        row["path"] = path
        row["needs-rendering"] = needs
        row["verdict"] = verdict
        row["bytes"] = len(text)
        row.update(count(text))
        rows.append(row)
    return rows


def print_rows(rows):
    columns = []
    for row in rows:
        for key in row:
            if key not in columns:
                columns.append(key)
    print("\t".join(columns))
    for row in rows:
        print("\t".join(str(row.get(c, "")) for c in columns))


PLANTED = """<?xml version="1.0"?>
<xdp:xdp xmlns:xdp="http://ns.adobe.com/xdp/">
<template xmlns="http://www.xfa.org/schema/xfa-template/3.3/">
 <subform name="form1" layout="tb">
  <pageSet><pageArea name="P1"><contentArea x="0" y="0" w="8in" h="10in"/>
   <draw><value><text>Header</text></value><font typeface="Myriad Pro"/></draw></pageArea></pageSet>
  <subform name="rows" layout="table"><occur min="0" max="-1" initial="1"/>
   <bind match="dataRef" ref="$.row[*]"/>
   <breakBefore targetType="pageArea" target="#P1"/>
   <overflow leader="head" trailer="foot"/>
   <keep next="contentArea"/>
   <field name="a"><bind match="global"/></field>
   <draw><value><exData contentType="text/html"/></value><font typeface="Arial"/></draw>
  </subform>
  <subformSet relation="choice"><subform name="x"/></subformSet>
  <subform name="fixed"><occur min="2" max="2"/>
   <event activity="click"><script contentType="application/x-javascript">rows.instanceManager.addInstance(1);</script></event>
   <event activity="click"><script>_rows.addInstance(1)</script></event>
   <event activity="ready:layout"><script contentType="application/x-javascript">this.rawValue = xfa.layout.pageCount();</script></event>
   <break after="pageArea"/>
  </subform>
 </subform>
</template>
<xfa:datasets xmlns:xfa="http://www.xfa.org/schema/xfa-data/1.0/"><xfa:data><form1><row/><row/></form1></xfa:data></xfa:datasets>
</xdp:xdp>
"""

PLAIN = """<xdp:xdp xmlns:xdp="http://ns.adobe.com/xdp/">
<config xmlns="http://www.xfa.org/schema/xci/2.8/"><data><adjustData>0</adjustData></data></config>
<template xmlns="http://www.xfa.org/schema/xfa-template/2.8/"><subform name="f"><field name="a"/></subform></template>
</xdp:xdp>
"""


def self_test():
    expected_planted = {
        "version": "3.3", "pageSet": 1, "pageArea": 1, "contentArea": 1, "subform": 4,
        "subformSet": 1, "field": 1, "draw": 2, "exData": 1, "breakBefore": 1, "break": 1,
        "overflow": 1, "keep": 1, "occur": 2, "flowed": 2, "occur-variable": 1,
        "occur-unbounded": 1, "binds": "dataRef:1,global:1", "javascript": 2, "formcalc": 1,
        "instance-scripts": 2, "page-count-scripts": 1, "typefaces": 2, "data-nodes": 3,
        "layouts": "position:2,table:1,tb:1",
    }
    expected_plain = {
        "version": "2.8", "subform": 1, "field": 1, "occur": 0, "occur-variable": 0,
        "subformSet": 0, "pageArea": 0, "flowed": 0, "data-nodes": -1,
    }
    failures = []
    with tempfile.TemporaryDirectory() as dump:
        for name, text in (("0", PLANTED), ("1", PLAIN), ("2", "<not xml"),
                           ("3", PLAIN.replace('name="a"', 'name="a"><value><text>A & B</text></value></field><field name="b"'))):
            with open(os.path.join(dump, name + ".xdp"), "w", encoding="utf-8") as handle:
                handle.write(text)
        tsv = os.path.join(dump, "census.tsv")
        with open(tsv, "w", encoding="utf-8") as handle:
            handle.write("0\ta.pdf\ttrue\ttrue\tits template holds a subformSet\t\n")
            handle.write("1\tb.pdf\tfalse\ttrue\tstatic\t\n")
        rows = census(dump, tsv)
    by_line = {row["line"]: row for row in rows}
    for line, expected in (("0", expected_planted), ("1", expected_plain)):
        for key, value in expected.items():
            if by_line[line].get(key) != value:
                failures.append(f"{line}.{key}: {by_line[line].get(key)!r}, expected {value!r}")
    if by_line["0"]["path"] != "a.pdf" or by_line["0"]["needs-rendering"] != "true":
        failures.append("the census line was not joined")
    if by_line["3"].get("repaired") != "bare-ampersand" or by_line["3"].get("field") != 2:
        failures.append("a bare ampersand was not repaired and counted")
    if not by_line["2"].get("error", "").startswith("xml:"):
        failures.append("an unparseable resource was not named")
    for failure in failures:
        print("FAIL", failure)
    if failures:
        return 1
    print("every case holds")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    parser.add_argument("dump", nargs="?", help="the directory xfa_dynamic_census --dump wrote")
    parser.add_argument("--census", help="the example's standard output, to join paths")
    parser.add_argument("--self-test", action="store_true")
    arguments = parser.parse_args()
    if arguments.self_test:
        return self_test()
    if not arguments.dump:
        parser.error("a dump directory is needed")
    print_rows(census(arguments.dump, arguments.census))
    return 0


if __name__ == "__main__":
    sys.exit(main())
