#!/usr/bin/env bash
# Drives the three windows — `quorra`, `quorra-gtk`, `quorra-qt` — through what a reader does with an
# open document, headless under Xvfb, and photographs every step; then `quorra-confined` through its
# opening caption's section and the one thing only it can show, a device refusal of a page its
# sandboxed worker sent as marks.
#
#   tools/drive-windows.sh [--out DIR] [--window NAME]... [--bin DIR] [--display :N]
#
# The list is `doc/verify.md`'s "Driving the three windows": open, §12.2's Table 147 entries, the
# outline, page turns by key and wheel, zoom, find (a Latin word, an Arabic word, a word typed
# without the marks printed on it, and one placed under a mirroring text matrix), a popup, a link,
# a markup, §7.5.6's save read back, the pages panel, the restrictions levels, print, §7.6.4's password, and a form's §12.5.1 tab order, check
# box, choice and push button saved and re-read, and Annex O's fragment — a page, a search, and form
# data fetched from the drive's own loopback server at three `--submissions=` levels, `ask` answered both ways. Each step's
# observable is a title, a line the window printed, the saved file's bytes, what AT-SPI reads off the window, or a count of pixels of
# a colour the step draws, and the verdict is printed as
# `step<TAB>window<TAB>works|wrong|not offered|manual<TAB>what was seen<TAB>seconds`, one line
# each, into `$OUT/results.tsv`, where `seconds` is the wall-clock since the verdict before it — the
# first step of a window carries that window's launch — so the column sums to the drive and
# `tools/state.sh drive` can say where its time goes; the photographs are `$OUT/shots/<window>/<step>.png`, and every other
# top-level window the program has up (a popup, a dialog) is photographed beside it, because with no
# window manager GTK's and Qt's popups are not on the root's picture. A title is a weaker witness
# than the picture (ADR 1453), so no step rests on one where the window says more.
#
# The reopened form is read off the bus in all three windows: the toolkits' widgets, and `quorra`'s
# own form nodes, which carry §12.7.4.3's value as a text run and a choice's options as selectable
# items (ADR 1489).
#
# The reader's three policy words are driven in all four windows, each launched with the word and
# without it: a signature this drive makes is valid only under `--trust-anchors`, a reference XObject
# draws the page `--reference-files` supplies, and a layer is drawn for the reader `--reader-name`
# names and not for another (ADR 1580).
#
# And in all four windows, §12.7.4.3's commit — a tab out of a field and Enter in one, a character a
# field's keystroke script refuses and a value its commit refuses, each said (ADR 1592) — and §12.10's
# position on a geographic map, with a projected map's refusal beside it (ADR 1593). In the two
# toolkit windows the committed values are then read off each control's own text as the fields
# display them, and the field's characters once its control holds the keyboard (ADR 1604).
#
# A step waits for the window's own word wherever the window gives one — its first frame's line
# under `--trace=launch`, a command's line under `events`, a title, a node on the bus, a colour on a
# photograph — each with a ceiling, rather than for a fixed time; an input the window says nothing
# about keeps its settle, with the reason beside the number (ADR 1605).
#
# And two things a screen reader does, through AT-SPI alone: it asks which window is the active one,
# whose frame says so with the keyboard in it and not without (ADR 1565), and it clicks a text field
# in the document's tree and types into it, which the saved file then holds (ADR 1566).
#
# Coordinates are ASKED of the window where it can answer: the drive runs on a private session bus
# with AT-SPI on it, and the outline rows, the pages tab and row, the check box and the choice are
# found by role and name and clicked at the centre of `Component.GetExtents` — so a change of layout
# moves the click with the widget. `quorra-gtk` and `quorra-qt` answer for all of them; `quorra`
# draws its own panels and publishes bounds (accesskit) only for the document's nodes, so its form
# controls are asked and its panel rows are not. What is not answered falls back to the coordinates
# measured on the fixtures this script writes, at Xvfb's 1400x1100, and `$OUT/coordinates.tsv` says
# which each click was. A step that clicks nothing says `wrong` with the title it saw. The release
# binaries are
# taken from `--bin` (default: this worktree's release directory); build them first:
#   cargo build --release -p viewer-ui --bin quorra --bin quorra-confined -p viewer-gtk \
#                         --bin quorra-gtk -p viewer-qt --bin quorra-qt
# Needs Xvfb, xdotool, xwd, ImageMagick's `magick` and python3 with pikepdf; the accessibility step
# and the asked coordinates need at-spi2-core and python3's `gi` Atspi. Nothing here is a gate: a test that skipped silently
# would be worse than none (doc/environment.md).
set -u
# No `__pycache__` where the workspace's `tools/*` glob would read it as a member (trap 114).
export PYTHONDONTWRITEBYTECODE=1

ROOT=$(cd "$(dirname "$0")/.." && pwd)
# Under the agent's task budget, the figure `tools/bounded.sh` writes once (trap 116, ADR 1612); a
# limit already at or under it is kept.
task_budget=$("$ROOT/tools/bounded.sh" --task-budget) || exit 1
[ "$(ulimit -u)" != unlimited ] && [ "$(ulimit -u)" -le "$task_budget" ] || ulimit -u "$task_budget" || exit 1
OUT="$ROOT/scratchpad/drive-windows"
DISPLAY_NUMBER=":93"
TARGET=$(cargo metadata --format-version 1 --no-deps --manifest-path "$ROOT/Cargo.toml" 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])' 2>/dev/null)
BIN="${TARGET:-$ROOT/target}/release"
WINDOWS=()
while [ $# -gt 0 ]; do
    case "$1" in
        --out) OUT=$2; shift 2 ;;
        --window) WINDOWS+=("$2"); shift 2 ;;
        --bin) BIN=$2; shift 2 ;;
        --display) DISPLAY_NUMBER=$2; shift 2 ;;
        *) echo "drive-windows: unknown argument $1" >&2; exit 2 ;;
    esac
done
[ ${#WINDOWS[@]} -eq 0 ] && WINDOWS=(quorra quorra-gtk quorra-qt quorra-confined)
for tool in Xvfb xdotool xwd magick python3; do
    command -v "$tool" >/dev/null || { echo "drive-windows: $tool is not installed" >&2; exit 2; }
done
for window in "${WINDOWS[@]}"; do
    [ -x "$BIN/$window" ] || { echo "drive-windows: no $BIN/$window — build it first" >&2; exit 2; }
done

export DISPLAY=$DISPLAY_NUMBER
FIXTURES="$OUT/fixtures"
mkdir -p "$FIXTURES"
RESULTS="$OUT/results.tsv"
: > "$RESULTS"
XVFB=""
APP=""
BUS=""        # the private session bus's daemon, when AT-SPI is up
A11Y=""       # the accessibility bus's address on it
SERVER=""     # the loopback server Annex O's fetched `fdf` is served from
PORT=""       # its port

# Every process either bus has, by the pid the bus names — a service D-Bus activated for a window
# (the registry, a portal) is nobody's child here — then the session bus itself.
atspi_down() {
    [ -z "$BUS" ] && return
    local pid
    for pid in $( { busctl --user list --no-legend; [ -n "$A11Y" ] && busctl --address="$A11Y" list --no-legend; } \
                  2>/dev/null | awk '$2 ~ /^[0-9]+$/ {print $2}' | sort -u); do
        [ "$pid" != $$ ] && [ "$pid" != "$BUS" ] && kill "$pid" 2>/dev/null
    done
    kill "$BUS" 2>/dev/null
    BUS=""
}

finish() {
    [ -n "$APP" ] && kill "$APP" 2>/dev/null
    [ -n "$SERVER" ] && kill "$SERVER" 2>/dev/null
    atspi_down
    [ -n "$XVFB" ] && kill "$XVFB" 2>/dev/null
    wait 2>/dev/null
}
trap finish EXIT

# --- the fixtures ----------------------------------------------------------------------------------
python3 - "$FIXTURES" <<'PY' || { echo "drive-windows: the fixtures need python3's pikepdf" >&2; exit 2; }
import sys, pikepdf
from pikepdf import Name, Dictionary, Array, String
out = sys.argv[1]

def helv(pdf):
    return pdf.make_indirect(Dictionary(Type=Name.Font, Subtype=Name.Type1, BaseFont=Name.Helvetica,
                                        Encoding=Name.WinAnsiEncoding))

def page(pdf, font, text, extra=b"", box=(612, 792)):
    stream = (b"BT /F1 36 Tf 72 %d Td (%s) Tj ET\nBT /F1 14 Tf 72 %d Td "
              b"(The quorra drive fixture, a word to find.) Tj ET\n"
              % (box[1] - 100, text.encode(), box[1] - 130)) + extra
    pdf.pages.append(pikepdf.Page(Dictionary(
        Type=Name.Page, MediaBox=[0, 0, box[0], box[1]],
        Resources=Dictionary(Font=Dictionary(F1=font, Helv=font)),
        Contents=pdf.make_stream(stream))))
    return pdf.pages[-1]

# drive.pdf: an outline in three scripts, Table 147's /FitWindow /CenterWindow /DisplayDocTitle,
# §12.5.6.5's link to page three, §12.5.6.4's text annotation with §12.5.6.14's popup.
pdf = pikepdf.new()
font = helv(pdf)
p1 = page(pdf, font, "Page one",
          b"0 0 1 RG 2 w 70 498 m 300 498 l S\nBT /F1 18 Tf 72 505 Td (Go to page three) Tj ET\n")
p2 = page(pdf, font, "Page two")
p3 = page(pdf, font, "Page three", box=(792, 612))
link = pdf.make_indirect(Dictionary(Type=Name.Annot, Subtype=Name.Link, Rect=[70, 495, 300, 530],
                                    Border=[0, 0, 1], C=[0, 0, 1], Dest=[p3.obj, Name.Fit]))
note = pdf.make_indirect(Dictionary(Type=Name.Annot, Subtype=Name.Text, Rect=[400, 600, 424, 624],
                                    Contents=String("A popup note: the reader opened it."),
                                    T=String("Drive"), Name=Name.Comment, C=[1, 0.9, 0.2], F=4))
popup = pdf.make_indirect(Dictionary(Type=Name.Annot, Subtype=Name.Popup, Rect=[430, 480, 600, 600],
                                     Parent=note, Open=False, F=4))
note.Popup = popup
p1.obj.Annots = Array([link, note, popup])
items = [pdf.make_indirect(Dictionary(Title=String(title), Dest=[pg.obj, Name.Fit]))
         for title, pg in [("Chapter one", p1), ("第二章 页面", p2), ("الفصل 3: السلام", p3)]]
outlines = pdf.make_indirect(Dictionary(Type=Name.Outlines, First=items[0], Last=items[-1],
                                        Count=len(items)))
for i, item in enumerate(items):
    item.Parent = outlines
    if i:
        item.Prev = items[i - 1]
    if i + 1 < len(items):
        item.Next = items[i + 1]
pdf.Root.Outlines = outlines
pdf.Root.PageMode = Name.UseOutlines
pdf.Root.ViewerPreferences = Dictionary(FitWindow=True, CenterWindow=True, DisplayDocTitle=True)
pdf.docinfo[Name.Title] = String("Drive 三窗口 عنوان")
pdf.save(f"{out}/drive.pdf")

# drive-form.pdf: six widgets in two columns, page /Tabs /C and /Annots in reverse, so that §12.5.1's
# column order (A C E B D F), row order (A B C D E F) and array order (F E D C B A) all differ.
pdf = pikepdf.new()
font = helv(pdf)
f1 = page(pdf, font, "Form")
page(pdf, font, "Form page two")
def appearance(body, w, h):
    return pdf.make_stream(body, Type=Name.XObject, Subtype=Name.Form, BBox=[0, 0, w, h],
                           Resources=Dictionary(Font=Dictionary(Helv=font)))
fields = {}
def widget(name, rect, **entries):
    fields[name] = pdf.make_indirect(Dictionary(
        Type=Name.Annot, Subtype=Name.Widget, T=String(name), Rect=rect, F=4, P=f1.obj,
        MK=Dictionary(BC=[0, 0, 0], BG=[0.9, 0.95, 1]), **entries))
text = String("/Helv 14 Tf 0 g")
widget("A", [72, 560, 272, 590], FT=Name.Tx, DA=text, V=String(""))
widget("B", [340, 560, 540, 590], FT=Name.Tx, DA=text, V=String(""))
widget("C", [72, 460, 272, 490], FT=Name.Tx, DA=text, V=String(""))
widget("D", [340, 460, 370, 490], FT=Name.Btn, V=Name.Off, AS=Name.Off,
       AP=Dictionary(N=Dictionary(Yes=appearance(b"0 g 4 4 22 22 re f", 30, 30),
                                  Off=appearance(b"0 G 1 w 1 1 28 28 re S", 30, 30))))
widget("E", [72, 360, 272, 390], FT=Name.Ch, Ff=131072, DA=text, V=String("Red"),
       Opt=[String("Red"), String("Green"), String("Blue")])
widget("F", [340, 360, 540, 390], FT=Name.Btn, Ff=65536,
       AP=Dictionary(N=appearance(b"0.8 g 0 0 200 30 re f 0 g BT /Helv 14 Tf 50 9 Td (Next page) Tj ET",
                                  200, 30)),
       A=Dictionary(S=Name.Named, N=Name.NextPage))
f1.obj.Annots = Array([fields[n] for n in "FEDCBA"])
f1.obj.Tabs = Name.C
pdf.Root.AcroForm = Dictionary(Fields=Array([fields[n] for n in "ABCDEF"]), DA=text,
                               DR=Dictionary(Font=Dictionary(Helv=font)))
pdf.save(f"{out}/drive-form.pdf")

# served/values.fdf: §12.7.8's form data for drive-form.pdf's field A, which Table Annex O.4's `fdf`
# fetches from the drive's own loopback server (ADR 1527).
import os
os.makedirs(f"{out}/served", exist_ok=True)
with open(f"{out}/served/values.fdf", "wb") as fdf:
    fdf.write(b"%FDF-1.2\n1 0 obj\n<< /FDF << /Fields [<< /T (A) /V (Fetched) >>] >> >>\nendobj\n"
              b"trailer\n<< /Root 1 0 R >>\n%%EOF\n")

# drive-password.pdf: §7.6.4.1's user password, "drive".
pdf = pikepdf.new()
page(pdf, helv(pdf), "Behind a password")
pdf.save(f"{out}/drive-password.pdf", encryption=pikepdf.Encryption(user="drive", owner="owner", R=6))

# drive-vowelled.pdf: "كَتَبَ" with its three fathas, in reading order (each glyph placed leftwards of
# the one before by TJ), through a Type 3 font whose /ToUnicode names the letters and the marks; a
# letter is a box and a fatha a bar above it, of no width (ADR 1477).
pdf = pikepdf.new()
# The TJ's 1000 moves each fatha's origin back a whole em, so its bar is drawn 550 units on, over
# the letter shown before it.
glyphs = {"l": b"500 0 d0 0 0 450 700 re f", "m": b"0 0 d0 550 780 350 60 re f"}
procs = Dictionary({f"/{name}": pdf.make_stream(body) for name, body in glyphs.items()})
cmap = (b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /Vowelled def "
        b"/CMapType 2 def 1 begincodespacerange <00> <FF> endcodespacerange 4 beginbfchar "
        b"<01> <0643> <02> <062A> <03> <0628> <04> <064E> endbfchar endcmap "
        b"CMapName currentdict /CMap defineresource pop end end")
type3 = pdf.make_indirect(Dictionary(
    Type=Name.Font, Subtype=Name.Type3, FontBBox=[0, 0, 950, 840],
    FontMatrix=[0.001, 0, 0, 0.001, 0, 0], CharProcs=procs,
    Encoding=Dictionary(Type=Name.Encoding, Differences=[1, Name.l, Name.l, Name.l, Name.m]),
    FirstChar=1, LastChar=4, Widths=[500, 500, 500, 0], Resources=Dictionary(),
    ToUnicode=pdf.make_stream(cmap)))
vowelled = page(pdf, helv(pdf), "Vowelled",
                b"BT /F2 60 Tf 300 500 Td [<01> 1000 <04> <02> 1000 <04> <03> 1000 <04>] TJ ET\n")
vowelled.obj.Resources.Font.F2 = type3
pdf.save(f"{out}/drive-vowelled.pdf")

# drive-mirrored.pdf: "عرب" placed glyph by glyph by TJ in reading order, each adjustment moving the
# pen back against the advance, under a mirroring text matrix — §9.4.4's text space, which the
# readback's word gaps are measured in (ADR 1490). The same Type 3 boxes, mapped to the three letters.
pdf = pikepdf.new()
cmap = (b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /Mirrored def "
        b"/CMapType 2 def 1 begincodespacerange <00> <FF> endcodespacerange 3 beginbfchar "
        b"<01> <0628> <02> <0631> <03> <0639> endbfchar endcmap "
        b"CMapName currentdict /CMap defineresource pop end end")
boxes = pdf.make_indirect(Dictionary(
    Type=Name.Font, Subtype=Name.Type3, FontBBox=[0, 0, 950, 840],
    FontMatrix=[0.001, 0, 0, 0.001, 0, 0],
    CharProcs=Dictionary({"/l": pdf.make_stream(glyphs["l"])}),
    Encoding=Dictionary(Type=Name.Encoding, Differences=[1, Name.l, Name.l, Name.l]),
    FirstChar=1, LastChar=3, Widths=[500, 500, 500], Resources=Dictionary(),
    ToUnicode=pdf.make_stream(cmap)))
mirrored = page(pdf, helv(pdf), "Mirrored",
                b"BT /F2 60 Tf -1 0 0 1 612 0 Tm 220 500 Td [<03> 1000 <02> 1000 <01>] TJ ET\n")
mirrored.obj.Resources.Font.F2 = boxes
pdf.save(f"{out}/drive-mirrored.pdf")

# drive-field.pdf: one text field holding "123", whose characters §12.7.4.3's layout places (ADR 1501).
pdf = pikepdf.new()
font = helv(pdf)
f1 = page(pdf, font, "Field")
held = pdf.make_indirect(Dictionary(
    Type=Name.Annot, Subtype=Name.Widget, T=String("N"), Rect=[72, 500, 372, 540], F=4, P=f1.obj,
    FT=Name.Tx, DA=String("/Helv 18 Tf 0 g"), V=String("123"),
    MK=Dictionary(BC=[0, 0, 0], BG=[0.9, 0.95, 1])))
f1.obj.Annots = Array([held])
pdf.Root.AcroForm = Dictionary(Fields=Array([held]), DR=Dictionary(Font=Dictionary(Helv=font)))
pdf.save(f"{out}/drive-field.pdf")

# drive-coverage.pdf: a thousand fifteen-point stars over the whole page. A few kilobytes of marks,
# so a confined worker sends them as a list rather than as pixels (ADR 0607), and more coverage than
# a device's scratch sheet holds, so the device refuses the frame (ADR 1478).
import math
pdf = pikepdf.new()
stars = []
for i in range(1000):
    cx, cy = 306 + (i % 7) * 3, 396 + (i % 5) * 3
    points = [(cx + 700 * math.cos(2 * math.pi * k * 7 / 15 + i * 0.01),
               cy + 700 * math.sin(2 * math.pi * k * 7 / 15 + i * 0.01)) for k in range(15)]
    stars.append("%.3f %.3f %.3f rg " % ((i % 3) / 3, (i % 5) / 5, (i % 7) / 7)
                 + "%.1f %.1f m " % points[0]
                 + " ".join("%.1f %.1f l" % point for point in points[1:]) + " h f")
pdf.pages.append(pikepdf.Page(Dictionary(Type=Name.Page, MediaBox=[0, 0, 612, 792],
                                         Contents=pdf.make_stream("\n".join(stars).encode()))))
pdf.save(f"{out}/drive-coverage.pdf")

# The reader's three policy words (ADR 1580): a page for each that draws one thing with the word and
# another without it.
import os, subprocess
os.makedirs(f"{out}/targets", exist_ok=True)
# drive-target.pdf: one page filled blue, with §14.4's identifier a reference names.
ident = [String(bytes.fromhex("d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1")), String(bytes.fromhex("e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2"))]
pdf = pikepdf.new()
pdf.pages.append(pikepdf.Page(Dictionary(Type=Name.Page, MediaBox=[0, 0, 612, 792],
    Contents=pdf.make_stream(b"0 0 1 rg 72 72 468 648 re f"))))
pdf.trailer.ID = Array(ident)
pdf.save(f"{out}/targets/drive-target.pdf")
# The identifier as the file now states it: a writer replaces the second string at every save.
ident = [String(bytes(s)) for s in pikepdf.open(f"{out}/targets/drive-target.pdf").trailer.ID]
# drive-reference.pdf: a reference XObject (§8.10.4) naming that page, whose proxy is grey.
pdf = pikepdf.new()
form = pdf.make_stream(b"0.5 g 72 72 468 648 re f", Type=Name.XObject, Subtype=Name.Form,
    BBox=[0, 0, 612, 792], Ref=Dictionary(F=String("drive-target.pdf"), Page=0, ID=Array(ident)))
pdf.pages.append(pikepdf.Page(Dictionary(Type=Name.Page, MediaBox=[0, 0, 612, 792],
    Resources=Dictionary(XObject=Dictionary(R=form)), Contents=pdf.make_stream(b"/R Do"))))
pdf.save(f"{out}/drive-reference.pdf")
# drive-audience.pdf: a green square in a group §8.11.4.4's User category names Ada for.
pdf = pikepdf.new()
group = pdf.make_indirect(Dictionary(Type=Name.OCG, Name=String("For Ada"),
    Usage=Dictionary(User=Dictionary(Type=Name.Ind, Name=String("Ada")))))
pdf.pages.append(pikepdf.Page(Dictionary(Type=Name.Page, MediaBox=[0, 0, 612, 792],
    Resources=Dictionary(Properties=Dictionary(oc=group)),
    Contents=pdf.make_stream(b"/OC /oc BDC 0 1 0 rg 72 72 468 648 re f EMC"))))
pdf.Root.OCProperties = Dictionary(OCGs=Array([group]), D=Dictionary(
    AS=Array([Dictionary(Event=Name.View, Category=Array([Name.User]), OCGs=Array([group]))])))
pdf.save(f"{out}/drive-audience.pdf")

# drive-signed.pdf: §12.8.3.3.1's adbe.pkcs7.detached signature by a certificate a root this drive
# issues certified; the root alone is in anchors/, which `--trust-anchors` names (§12.8.1's third
# question), and signing/ holds the keys and is named by nothing.
keys = os.path.join(out, "signing")
anchors = os.path.join(out, "anchors")
os.makedirs(keys, exist_ok=True); os.makedirs(anchors, exist_ok=True)
def run(*args):
    subprocess.run(args, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
k = lambda name: os.path.join(keys, name)
run("openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout", k("root.key"),
    "-out", os.path.join(anchors, "root.pem"), "-days", "3650", "-subj", "/CN=Drive root",
    "-addext", "basicConstraints=critical,CA:TRUE", "-addext", "keyUsage=critical,keyCertSign,cRLSign")
run("openssl", "req", "-newkey", "rsa:2048", "-nodes", "-keyout", k("signer.key"), "-out", k("signer.csr"),
    "-subj", "/CN=Drive signer")
with open(k("signer.ext"), "w") as ext:
    ext.write("basicConstraints=CA:FALSE\nkeyUsage=critical,digitalSignature,nonRepudiation\n")
run("openssl", "x509", "-req", "-in", k("signer.csr"), "-CA", os.path.join(anchors, "root.pem"),
    "-CAkey", k("root.key"), "-CAcreateserial", "-CAserial", k("root.srl"), "-days", "3650", "-extfile", k("signer.ext"), "-out", k("signer.pem"))

SIZE = 8192
content = b"BT /F1 24 Tf 72 700 Td (Signed by the drive) Tj ET"
objs = [
    b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /SigFlags 3 >> >>",
    b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [4 0 R] /Contents 6 0 R "
    b"/Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >>",
    b"<< /Type /Annot /Subtype /Widget /FT /Sig /T (Drive) /Rect [0 0 0 0] /F 132 /P 3 0 R /V 5 0 R >>",
    None,
    b"<< /Length %d >>\nstream\n" % len(content) + content + b"\nendstream",
]
sig_head = b"<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /Name (Drive signer) /ByteRange "
placeholder_range = b"[0 0000000000 0000000000 0000000000]"
sig_tail = b" /Contents <" + b"0" * (2 * SIZE) + b"> >>"
objs[4] = sig_head + placeholder_range + sig_tail
body = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
offsets = []
for number, obj in enumerate(objs, 1):
    offsets.append(len(body))
    body += b"%d 0 obj\n" % number + obj + b"\nendobj\n"
xref = len(body)
body += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
for offset in offsets:
    body += b"%010d 00000 n \n" % offset
body += b"trailer\n<< /Size %d /Root 1 0 R /ID [<0123456789abcdef0123456789abcdef> <0123456789abcdef0123456789abcdef>] >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
at = body.index(b"/Contents <" + b"0" * 16) + len(b"/Contents ")
end = at + 2 + 2 * SIZE
ranges = b"[0 %010d %010d %010d]" % (at, end, len(body) - end)
start = body.index(placeholder_range)
body[start:start + len(placeholder_range)] = ranges
with open(k("signed-part.bin"), "wb") as part:
    part.write(bytes(body[:at]) + bytes(body[end:]))
run("openssl", "cms", "-sign", "-binary", "-in", k("signed-part.bin"), "-signer", k("signer.pem"),
    "-inkey", k("signer.key"), "-certfile", os.path.join(anchors, "root.pem"), "-outform", "DER", "-md", "sha256",
    "-nosmimecap", "-out", k("signature.der"))
der = open(k("signature.der"), "rb").read()
assert len(der) <= SIZE
body[at + 1:at + 1 + 2 * len(der)] = der.hex().encode()
open(os.path.join(out, "drive-signed.pdf"), "wb").write(body)

# drive-commit.pdf: three number fields under AFNumber_Format and AFNumber_Keystroke and a read-only
# total under AFSimple_Calculate that /CO names, in row order (ADR 1592) — the shape of
# crates/pdf-model/tests/aform.rs and crates/viewer-core/tests/field_commit.rs.
pdf = pikepdf.new()
font = helv(pdf)
f1 = page(pdf, font, "Commit")
currency = Dictionary(
    F=Dictionary(S=Name.JavaScript, JS=String('AFNumber_Format(2, 0, 0, 0, "$", true);')),
    K=Dictionary(S=Name.JavaScript, JS=String('AFNumber_Keystroke(2, 0, 0, 0, "$", true);')))
priced = {}
for name, top in [("Price1", 600), ("Price2", 540), ("Price3", 480)]:
    priced[name] = pdf.make_indirect(Dictionary(
        Type=Name.Annot, Subtype=Name.Widget, T=String(name), Rect=[72, top - 30, 272, top], F=4,
        P=f1.obj, FT=Name.Tx, DA=text, AA=currency, MK=Dictionary(BC=[0, 0, 0], BG=[0.9, 0.95, 1])))
priced["Total"] = pdf.make_indirect(Dictionary(
    Type=Name.Annot, Subtype=Name.Widget, T=String("Total"), Rect=[72, 390, 272, 420], F=4, Ff=1,
    P=f1.obj, FT=Name.Tx, DA=text, MK=Dictionary(BC=[0, 0, 0], BG=[0.95, 0.95, 0.95]),
    AA=Dictionary(
        C=Dictionary(S=Name.JavaScript,
                     JS=String('AFSimple_Calculate("SUM", ["Price1", "Price2", "Price3"]);')),
        F=Dictionary(S=Name.JavaScript, JS=String('AFNumber_Format(2, 0, 0, 0, "$", true);')))))
f1.obj.Annots = Array([priced[n] for n in ["Price1", "Price2", "Price3", "Total"]])
f1.obj.Tabs = Name.R
pdf.Root.AcroForm = Dictionary(Fields=Array(list(priced.values())), CO=Array([priced["Total"]]),
                               DA=text, DR=Dictionary(Font=Dictionary(Helv=font)))
pdf.save(f"{out}/drive-commit.pdf")

# drive-projected.pdf: a projected map whose /GPTS are degrees, the shape every projected map of the
# crawl takes; refused by name until doc/questions/Q271 is answered (ADRs 1586, 1593).
wkt = ('PROJCS["ETRS89_UTM_zone_32N",GEOGCS["GCS_ETRS_1989",DATUM["D_ETRS_1989",'
       'SPHEROID["GRS_1980",6378137,298.257222101]],PRIMEM["Greenwich",0],'
       'UNIT["Degree",0.017453292519943295]],PROJECTION["Transverse_Mercator"],'
       'PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",9],'
       'PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",500000],'
       'PARAMETER["false_northing",0],UNIT["Meter",1]]')
pdf = pikepdf.new()
pdf.pages.append(pikepdf.Page(Dictionary(Type=Name.Page, MediaBox=[0, 0, 612, 792],
    Contents=pdf.make_stream(b"0.8 0.9 0.8 rg 0 0 612 792 re f"),
    VP=Array([Dictionary(Type=Name.Viewport, BBox=[0, 0, 612, 792], Measure=Dictionary(
        Type=Name.Measure, Subtype=Name.GEO, GCS=Dictionary(Type=Name.PROJCS, WKT=String(wkt)),
        GPTS=Array([47, 8, 48, 8, 48, 10, 47, 10]), LPTS=Array([0, 0, 0, 1, 1, 1, 1, 0])))]))))
pdf.save(f"{out}/drive-projected.pdf")
PY
ARABIC="$ROOT/doc/pdf.js/test/pdfs/ArabicCIDTrueType.pdf"
[ -f "$ARABIC" ] && cp "$ARABIC" "$FIXTURES/arabic.pdf"
# §12.10's one geographic map among the curated documents (ADR 1586's census): its four corners
# are a north-up rectangle of degrees from 9.43386 S to 17.71438 S and 165.52069 E to 176.86596 E.
GEOGRAPHIC="$ROOT/doc/pdf.js/test/pdfs/bug1146106.pdf"
[ -f "$GEOGRAPHIC" ] && cp "$GEOGRAPHIC" "$FIXTURES/drive-geographic.pdf"

# --- the instrument --------------------------------------------------------------------------------
# A loopback HTTP server for the fetched `fdf` (step 31), on a port the kernel chose; every request
# it answers is a line of `$SERVED`, which is how a `refuse` is checked to have sent nothing.
SERVED="$OUT/served.log"
: > "$SERVED"
python3 -u - "$FIXTURES/served" > "$OUT/server.port" 2>> "$SERVED" <<'PY' &
import functools, http.server, sys
handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=sys.argv[1])
server = http.server.HTTPServer(("127.0.0.1", 0), handler)
print(server.server_address[1])
server.serve_forever()
PY
SERVER=$!
for _ in $(seq 1 20); do PORT=$(head -1 "$OUT/server.port" 2>/dev/null); [ -n "$PORT" ] && break; sleep 0.2; done
# A display somebody else has would put this run's windows beside theirs, and its photographs too.
if [ -e "/tmp/.X${DISPLAY_NUMBER#:}-lock" ]; then
    echo "drive-windows: display $DISPLAY_NUMBER is in use; pass --display :N" >&2
    exit 2
fi
Xvfb "$DISPLAY_NUMBER" -screen 0 1400x1100x24 -nolisten tcp >/dev/null 2>&1 &
XVFB=$!
# The server answering a client is the condition, rather than a second (ADR 1605); `wait_for` is
# defined further down, so this is its loop written out.
for _ in $(seq 1 100); do xdotool getdisplaygeometry >/dev/null 2>&1 && break; sleep 0.1; done

# A private session bus with AT-SPI enabled on it, so that every window driven below publishes its
# widgets' extents (doc/verify.md's AT-SPI recipe, held for the whole drive).
COORDINATES="$OUT/coordinates.tsv"
: > "$COORDINATES"
if command -v dbus-daemon >/dev/null && [ -x /usr/lib/at-spi-bus-launcher ] \
        && python3 -c 'import gi; gi.require_version("Atspi", "2.0")' 2>/dev/null; then
    read -r address BUS <<< "$(dbus-daemon --session --fork --print-address=1 --print-pid=1 | tr '\n' ' ')"
    export DBUS_SESSION_BUS_ADDRESS=$address
    /usr/lib/at-spi-bus-launcher --launch-immediately >/dev/null 2>&1 < /dev/null &
    # Each service is waited for by its name on its bus, rather than for two seconds and one.
    for _ in $(seq 1 100); do busctl --user status org.a11y.Bus >/dev/null 2>&1 && break; sleep 0.1; done
    busctl --user set-property org.a11y.Bus /org/a11y/bus org.a11y.Status IsEnabled b true
    A11Y=$(busctl --user call org.a11y.Bus /org/a11y/bus org.a11y.Bus GetAddress | cut -d'"' -f2)
    /usr/lib/at-spi2-registryd >/dev/null 2>&1 < /dev/null &
    for _ in $(seq 1 100); do
        busctl --address="$A11Y" status org.a11y.atspi.Registry >/dev/null 2>&1 && break
        sleep 0.1
    done
fi
cat > "$OUT/asked.py" <<'PY'
# asked.py PID ROLE NAME: the centre of the smallest showing widget of that role and name, in the
# coordinates of the window it is in ("x y"), or nothing. "*" matches any role or any name. A
# toolkit's own widget is preferred to the node the document's tree (a DocumentFrame) publishes for
# the same field: the second is placed by this program's bridge and the first is what takes a click.
import sys
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi
pid, role, name = int(sys.argv[1]), sys.argv[2], sys.argv[3]
WINDOW = Atspi.CoordType.WINDOW
def extents(node):
    component = node.get_component_iface()
    return component.get_extents(WINDOW) if component else None
best = None
def walk(node, depth, origin, document=False):
    global best
    if node is None or depth > 40:
        return
    try:
        here = extents(node)
        document = document or node.get_role() == Atspi.Role.DOCUMENT_FRAME
        if (role in ("*", node.get_role_name()) and name in ("*", node.get_name())
                and node.get_state_set().contains(Atspi.StateType.SHOWING)
                and here and 0 < here.width < 5000 and 0 < here.height < 5000):
            x, y = here.x + here.width // 2 - origin[0], here.y + here.height // 2 - origin[1]
            # A hidden widget can still say it is showing; one whose centre is off its window is not.
            if 0 <= x < origin[2] and 0 <= y < origin[3]:
                rank = (document, here.width * here.height)
                if best is None or rank < best[0]:
                    best = (rank, x, y)
        for index in range(node.get_child_count()):
            walk(node.get_child_at_index(index), depth + 1, origin, document)
    except gi.repository.GLib.Error:
        return
desktop = Atspi.get_desktop(0)
for index in range(desktop.get_child_count()):
    application = desktop.get_child_at_index(index)
    if application is None or application.get_process_id() != pid:
        continue
    for at in range(application.get_child_count()):
        frame = application.get_child_at_index(at)
        whole = extents(frame) if frame else None
        # A frame with no extents is a tree published beside the window rather than the window.
        if whole and whole.width > 0 and whole.height > 0:
            walk(frame, 0, (whole.x, whole.y, whole.width, whole.height))
if best:
    print(best[1], best[2])
PY

cat > "$OUT/press.py" <<'PY'
# press.py PID ROLE NAME: performs the first action of the showing widget of that role and name in
# any window of the process, and fails where there is none — a dialogue's button is pressed the way
# an assistive technology presses it, without coordinates and without a window manager (ADR 1540).
import sys
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi
pid, role, name = int(sys.argv[1]), sys.argv[2], sys.argv[3]
def find(node, depth):
    if node is None or depth > 40:
        return None
    try:
        # Older AT-SPI says "push button" where this one says "button"; either names the one role.
        said = node.get_role_name()
        if (role in (said, "button" if said == "push button" else said) and node.get_name() == name
                and node.get_state_set().contains(Atspi.StateType.SHOWING)):
            return node
        for index in range(node.get_child_count()):
            found = find(node.get_child_at_index(index), depth + 1)
            if found is not None:
                return found
    except gi.repository.GLib.Error:
        return None
    return None
desktop = Atspi.get_desktop(0)
for index in range(desktop.get_child_count()):
    application = desktop.get_child_at_index(index)
    if application is None or application.get_process_id() != pid:
        continue
    button = find(application, 0)
    if button is not None and button.get_action_iface() is not None:
        Atspi.Action.do_action(button.get_action_iface(), 0)
        sys.exit(0)
sys.exit(1)
PY

WINDOW=""     # which of the three is being driven
LOG=""        # its standard output
STEP=0

# wait_for SECONDS COMMAND…: runs COMMAND every tenth of a second until it succeeds, and fails once
# SECONDS have passed without — the ceiling a condition gets in place of a fixed sleep (ADR 1605).
# A ceiling reached is said on standard error and judged by the step's own verdict, never here.
wait_for() {
    local until=$(( ${EPOCHREALTIME/[.,]/} + $1 * 1000000 ))
    shift
    until "$@"; do
        if [ "${EPOCHREALTIME/[.,]/}" -ge "$until" ]; then
            echo "drive-windows: $WINDOW: waited the ceiling for: $*" >&2
            return 1
        fi
        sleep 0.1
    done
}
# said_since N PATTERN: whether the window's log says PATTERN after its line N.
said_since() { tail -n "+$(($1 + 1))" "$LOG" 2>/dev/null | grep -q -- "$2"; }
# first_frame: whether the window has said, under `--trace=launch`, that its first frame is on the
# screen — each window's own sentence for it — and is mapped where xdotool can find it.
first_frame() {
    local line
    case "$WINDOW" in
        quorra) line='launch path, process start to first present' ;;
        quorra-confined) line='first frame presented' ;;
        *) line='first frame on the screen at' ;;
    esac
    grep -q -- "$line" "$LOG" 2>/dev/null && [ -n "$(main_window)" ]
}
# a_window: whether the process has any window mapped — a launch whose first frame waits behind a
# question (§7.6.4's password) is ready once the question is up.
a_window() { [ -n "$(main_window)" ]; }
# LAUNCHED is the condition `launch` waits for; LAUNCH_TRACE the topics the window prints.
LAUNCHED=first_frame
LAUNCH_TRACE=events,launch
launch() { # file [arguments]
    local file=$1
    shift
    [ -n "$APP" ] && { kill "$APP" 2>/dev/null; wait "$APP" 2>/dev/null; }
    LOG="$OUT/$WINDOW.$(basename "$file" .pdf).log"
    if [ "$WINDOW" = quorra-qt ]; then
        QT_XCB_NO_XI2=1 "$BIN/$WINDOW" --trace="$LAUNCH_TRACE" "$@" "$file" > "$LOG" 2>&1 &
    else
        "$BIN/$WINDOW" --trace="$LAUNCH_TRACE" "$@" "$file" > "$LOG" 2>&1 &
    fi
    APP=$!
    # The window's own word that its first frame is up, in place of five seconds (ADR 1605). The
    # ceiling is the slowest launch measured under a neighbour's build, with room.
    wait_for 30 "$LAUNCHED"
}
main_window() { xdotool search --onlyvisible --pid "$APP" 2>/dev/null | head -1; }
title() { xdotool getwindowname "$(main_window)" 2>/dev/null; }
origin() { xdotool getwindowgeometry --shell "$(main_window)" 2>/dev/null | awk -F= '/^[XY]=/{printf "%s ", $2}'; }
# `click` and `wheel` settle for the reason `key` gives below: nothing says when a press is done.
click() { # x y, in the main window's own coordinates
    local x y
    read -r x y <<< "$(origin)"
    xdotool mousemove $((x + $1)) $((y + $2)) click 1
    sleep 1.3
}
wheel() { # x y button [count]
    local x y
    read -r x y <<< "$(origin)"
    xdotool mousemove $((x + $1)) $((y + $2))
    for _ in $(seq "${4:-1}"); do xdotool click "$3"; done
    sleep 1.3
}
# The settle after an input with no line of its own to wait for: GTK and Qt move the keyboard and
# open popups from their event loops after the input's handler has returned, and nothing says when.
key() { xdotool windowfocus --sync "$(main_window)" 2>/dev/null; xdotool key --delay 300 "$@"; sleep 1.2; }
type_in() { xdotool windowfocus --sync "$(main_window)" 2>/dev/null; xdotool type --delay 300 "$1"; sleep 1.2; }
# key_then PATTERN KEY…, type_then PATTERN TEXT, click_then PATTERN X Y: the input, and then the
# window's own line for what it did with it, in place of the settle (ADR 1605). The lines are the
# core's commands as each window traces them under `events`: `Focused(Next)` and `focus Next
# annotation`, `SetField { field: "A", value: Text("1")` in all four, `saved N bytes to` from every
# save.
key_then() {
    local mark pattern=$1
    shift
    mark=$(lines)
    xdotool windowfocus --sync "$(main_window)" 2>/dev/null; xdotool key --delay 300 "$@"
    wait_for 10 said_since "$mark" "$pattern"
}
type_then() {
    local mark
    mark=$(lines)
    xdotool windowfocus --sync "$(main_window)" 2>/dev/null; xdotool type --delay 300 "$2"
    wait_for 10 said_since "$mark" "$1"
}
click_then() {
    local mark x y
    mark=$(lines)
    read -r x y <<< "$(origin)"
    xdotool mousemove $((x + $2)) $((y + $3)) click 1
    wait_for 10 said_since "$mark" "$1"
}
FOCUS_NEXT='Focused(Next)\|focus Next'
SAVED='saved [0-9]* bytes to'
shot() { # name
    local dir="$OUT/shots/$WINDOW" w n=0
    mkdir -p "$dir"
    xwd -root -silent -out "$OUT/root.xwd" && magick "xwd:$OUT/root.xwd" "$dir/$1.png"
    for w in $(xdotool search --onlyvisible --pid "$APP" 2>/dev/null | tail -n +2); do
        n=$((n + 1))
        xwd -id "$w" -silent -out "$OUT/window.xwd" 2>/dev/null \
            && magick "xwd:$OUT/window.xwd" "$dir/$1.window$n.png"
    done
    rm -f "$OUT/root.xwd" "$OUT/window.xwd"
}
verdict() { # step verdict what
    local now=${EPOCHREALTIME/[.,]/}   # microseconds; the radix is the locale's
    printf '%s\t%s\t%s\t%s\t%s\n' "$1" "$WINDOW" "$2" "$3" \
        "$(awk -v a="${CLOCK:-$now}" -v b="$now" 'BEGIN { printf "%.1f", (b - a) / 1e6 }')" | tee -a "$RESULTS"
    CLOCK=$now
}
title_has() { [[ "$(title)" == *"$1"* ]]; }
expect_title() { # step needle
    local seen
    # The title is the step's witness and the window writes it when its answer lands, so it is
    # waited for rather than read once (ADR 1605); a title that never comes is judged below.
    wait_for 10 title_has "$2"
    seen=$(title)
    case "$seen" in
        *"$2"*) verdict "$1" works "$seen" ;;
        *) verdict "$1" wrong "title: $seen (wanted $2)" ;;
    esac
}
said() { grep -c -- "$1" "$LOG" 2>/dev/null; }
# asked_fetch_up yes|no: whether the `ask` level's question about a fetched `fdf` is up and holding
# — no request at the server and no import-data line but the question's — and then answers it.
# `quorra` draws its card and is answered by its two keys, Enter and Escape; the two toolkits' card
# is a dialogue whose buttons AT-SPI finds by name and presses through their `Action` (ADR 1540).
# A card that is not up, or a fetch that ran before the answer, is a `wrong` said here.
asked_fetch_up() {
    local step="31-fragment-fdf-ask-$1" word
    if [ "$(grep -c 'GET /values.fdf' "$SERVED")" -ne "$served" ] \
            || grep -q 'import-data: fetching\|import-data: 1 field' "$LOG"; then
        verdict "$step" wrong "fetched before the question was answered"
        return 1
    fi
    local mark
    mark=$(lines)
    if [ "$WINDOW" = quorra ]; then
        # The card is drawn rather than published, so its picture is the witness that it is up.
        shot "$step-card"
        # The answer is done when the window says what became of the fetch (ADR 1605).
        if [ "$1" = yes ]; then key_then 'import-data:' Return; else key_then 'import-data:' Escape; fi
        return 0
    fi
    [ "$1" = yes ] && word="Go ahead" || word="Do not"
    if [ -z "$A11Y" ]; then
        verdict "$step" "not offered" "no AT-SPI bus to find the dialogue's buttons on"
        return 1
    fi
    # Pressed as soon as the dialogue's button is on the bus, and done once the window has said
    # what became of the fetch (ADR 1605).
    if ! wait_for 20 pressed "$word"; then
        shot "$step"
        verdict "$step" wrong "no showing \"$word\" button: the question is not up"
        return 1
    fi
    wait_for 10 said_since "$mark" 'import-data:'
}
# pressed WORD: press.py's press of the showing button WORD, which fails until it is up.
pressed() { timeout 20 python3 "$OUT/press.py" "$APP" button "$1" > /dev/null 2>&1; }
lines() { wc -l < "$LOG" 2>/dev/null || echo 0; }
# coloured PNG HEX: how many pixels are within 6% of that colour. Counted through the alpha channel
# rather than by painting them, because a photograph with no colour in it is stored as grey and a
# fill of red then reads as grey too.
coloured() {
    magick "$1" -alpha off -fuzz 6% -transparent "$2" -alpha extract -negate \
        -format "%[fx:round(mean*w*h)]" info: 2>/dev/null
}
# outside: a corner of the screen the main window does not cover, as "x y".
outside() {
    local x y w h X Y WIDTH HEIGHT
    eval "$(xdotool getwindowgeometry --shell "$(main_window)" 2>/dev/null | grep -E '^(X|Y|WIDTH|HEIGHT)=')"
    x=${X:-0}; y=${Y:-0}; w=${WIDTH:-0}; h=${HEIGHT:-0}
    if [ "$x" -gt 0 ] || [ "$y" -gt 0 ]; then echo "0 0"; else echo "$((x + w + 5)) $((y + h + 5))"; fi
}
# found_since N: whether the window said, after line N of its log, that a search found something —
# `quorra`'s trace of the core's answer, or the two native windows' "found" note.
found_since() { tail -n "+$(($1 + 1))" "$LOG" 2>/dev/null | grep -q 'searched: page\|^note: found "'; }
# What every window says once a search has ended, found or not: what `key_then` waits for.
SEARCHED='searched: page\|^note: found "\|not in this document'
# inside_the_field SEEN FX FY FW FH: whether the document node's box the extents probe appended
# ("… document x y w h") lies inside the field; true where it appended none.
inside_the_field() {
    [[ "$1" != *" document "* ]] && return 0
    local dx dy dw dh
    read -r dx dy dw dh <<< "${1##* document }"
    [ -n "$dh" ] && [ "$dw" -gt 0 ] && [ "$dx" -ge "$2" ] && [ $((dx + dw)) -le $(($2 + $4)) ] \
        && [ "$dy" -ge "$3" ] && [ $((dy + dh)) -le $(($3 + $5)) ]
}
# holds_colour NAME HEX: takes the photograph NAME and says whether it holds more than ten thousand
# pixels of that colour — a page of it has landed.
holds_colour() {
    shot "$1"
    [ "$(coloured "$OUT/shots/$WINDOW/$1.png" "$2")" -gt 10000 ] 2>/dev/null
}
# drawn_on_the_shot NAME: takes the photograph NAME and says whether it holds more than the three
# colours of an empty window — the page has landed on it.
drawn_on_the_shot() {
    local colours
    shot "$1"
    colours=$(magick "$OUT/shots/$WINDOW/$1.png" -unique-colors -format %w info: 2>/dev/null)
    [ "${colours:-0}" -gt 3 ]
}
# yellow PNG: how many pixels are the drive popup's /C colour, [1 0.9 0.2] — its title bar and icon.
yellow() {
    magick "$1" -fuzz 6% -fill "#ff0000" -opaque "#ffe633" -fuzz 0 -fill black +opaque "#ff0000" \
        -fill white -opaque "#ff0000" -colorspace gray -format "%[fx:round(mean*w*h)]" info: 2>/dev/null
}
# reopened PID WINDOW: the form's fields as AT-SPI reads them — each text field's text through
# `Text`, the check box's `checked` state and the combo box's choice — on one line, A C B D E. The
# two toolkits' are their own widgets, outside the document's frame, and a combo box says its choice
# as its name; `quorra`'s are the document's own form nodes, inside the frame, and its combo box says
# its choice as the item `Selection` holds (ADR 1489).
cat > "$OUT/reopened.py" <<'PY'
import sys
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi
pid, seen = int(sys.argv[1]), {}
own = sys.argv[2] == "quorra"
def walk(node, depth, document=False):
    if node is None or depth > 40:
        return
    try:
        role, name = node.get_role_name(), node.get_name()
        document = document or node.get_role() == Atspi.Role.DOCUMENT_FRAME
        ours = document if own else not document
        if role in ("text", "entry") and name in "ABC" and len(name) == 1 and ours:
            text = node.get_text_iface()
            seen[name] = Atspi.Text.get_text(text, 0, Atspi.Text.get_character_count(text))
        elif role == "check box" and name == "D" and ours:
            seen.setdefault("D", node.get_state_set().contains(Atspi.StateType.CHECKED))
        elif role == "combo box" and ours and not own:
            seen["E"] = name
        elif role == "combo box" and ours and name == "E":
            chosen = Atspi.Selection.get_selected_child(node.get_selection_iface(), 0)
            seen["E"] = chosen.get_name() if chosen is not None else "-"
        for index in range(node.get_child_count()):
            walk(node.get_child_at_index(index), depth + 1, document)
    except gi.repository.GLib.Error:
        return
desktop = Atspi.get_desktop(0)
for index in range(desktop.get_child_count()):
    application = desktop.get_child_at_index(index)
    if application is not None and application.get_process_id() == pid:
        walk(application, 0)
print(" ".join(str(seen.get(key, "-")) for key in "ACBDE"))
PY
# shown.py PID NAME…: what each named text control of a toolkit says through `Text`, in the order
# asked, one word each ("-" where there is none) — the toolkit's own widgets, outside the document's
# frame, which is where a committed value is shown as the field displays it (ADR 1604).
cat > "$OUT/shown.py" <<'PY'
import sys
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi
pid, names, seen = int(sys.argv[1]), sys.argv[2:], {}
def walk(node, depth, document=False):
    if node is None or depth > 40:
        return
    try:
        document = document or node.get_role() == Atspi.Role.DOCUMENT_FRAME
        name = node.get_name()
        if not document and node.get_role_name() in ("text", "entry") and name in names:
            text = node.get_text_iface()
            seen.setdefault(name, Atspi.Text.get_text(text, 0, Atspi.Text.get_character_count(text)))
        for index in range(node.get_child_count()):
            walk(node.get_child_at_index(index), depth + 1, document)
    except gi.repository.GLib.Error:
        return
desktop = Atspi.get_desktop(0)
for index in range(desktop.get_child_count()):
    application = desktop.get_child_at_index(index)
    if application is not None and application.get_process_id() == pid:
        walk(application, 0)
print(" ".join(seen.get(name) or "-" for name in names))
PY
# active.py PID: whether the window that holds the document's tree is the active one, as AT-SPI's
# state set says — "active", "inactive", or "none" where no window of the process holds a
# DocumentFrame. In the two toolkits' windows the toolkit publishes a frame of its own beside the
# document's, so the frame asked is the one above the DocumentFrame (ADR 1565).
cat > "$OUT/active.py" <<'PY'
import sys
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi
pid = int(sys.argv[1])
def holds_a_document(node, depth):
    if node is None or depth > 6:
        return False
    try:
        if node.get_role() == Atspi.Role.DOCUMENT_FRAME:
            return True
        return any(holds_a_document(node.get_child_at_index(index), depth + 1)
                   for index in range(node.get_child_count()))
    except gi.repository.GLib.Error:
        return False
desktop = Atspi.get_desktop(0)
for index in range(desktop.get_child_count()):
    application = desktop.get_child_at_index(index)
    if application is None or application.get_process_id() != pid:
        continue
    for nth in range(application.get_child_count()):
        window = application.get_child_at_index(nth)
        if window is not None and holds_a_document(window, 0):
            active = window.get_state_set().contains(Atspi.StateType.ACTIVE)
            print("active" if active else "inactive")
            sys.exit(0)
print("none")
PY
# aim.py PID NAME: performs the "click" action of the node of that name inside the document's tree
# — §14.7's tree, or the widgets an untagged page publishes (ADR 1369) — the way a screen reader
# activates a form field, without coordinates; fails where no such node declares the action.
cat > "$OUT/aim.py" <<'PY'
import sys
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi
pid, name = int(sys.argv[1]), sys.argv[2]
def find(node, depth, document):
    if node is None or depth > 40:
        return None
    try:
        document = document or node.get_role() == Atspi.Role.DOCUMENT_FRAME
        if document and node.get_name() == name and node.get_action_iface() is not None:
            return node
        for index in range(node.get_child_count()):
            found = find(node.get_child_at_index(index), depth + 1, document)
            if found is not None:
                return found
    except gi.repository.GLib.Error:
        return None
    return None
desktop = Atspi.get_desktop(0)
for index in range(desktop.get_child_count()):
    application = desktop.get_child_at_index(index)
    if application is None or application.get_process_id() != pid:
        continue
    node = find(application, 0, False)
    if node is None:
        continue
    action = node.get_action_iface()
    for nth in range(Atspi.Action.get_n_actions(action)):
        if Atspi.Action.get_action_name(action, nth) == "click":
            Atspi.Action.do_action(action, nth)
            print("clicked %s, a %s" % (name, node.get_role_name()))
            sys.exit(0)
sys.exit(1)
PY
# extents PID WINDOW: where AT-SPI's `GetCharacterExtents` puts the second character of the text
# field N, and the field's own extents, both on the screen ("x y w h in X Y W H"), or "refused: …"
# where the widget answers the call with an error — GTK 4's own text widget does, in every
# coordinate space, so that window does not offer the step. `quorra`'s field is
# the document's own node, inside the frame, whose characters are §12.7.4.3's placed glyphs (ADR
# 1501); the two toolkits' are their own entries, outside it.
cat > "$OUT/extents.py" <<'PY'
import sys
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi
pid, own = int(sys.argv[1]), sys.argv[2] == "quorra"
# The screen's coordinates first, which is what a client asks for; then the window's, because GTK 4
# answers text extents in no other space and gives a component's screen origin as 0, 0 (ADR 1516).
SPACES = ((Atspi.CoordType.SCREEN, ""), (Atspi.CoordType.WINDOW, " window"))
# The field "N" as the toolkit publishes it and as the document's tree does (ADR 1501), each the
# first answer in the first space that gave one, or the refusals.
fields = {}
def ask(node):
    text = node.get_text_iface()
    if text is None or Atspi.Text.get_character_count(text) < 2:
        return None
    refusals = []
    for space, said in SPACES:
        whole = node.get_component_iface().get_extents(space)
        try:
            box = Atspi.Text.get_character_extents(text, 1, space)
        except gi.repository.GLib.Error as error:
            refusals.append(error.message or "an error with no message")
            continue
        return ("%d %d %d %d" % (box.x, box.y, box.width, box.height), whole, said)
    whole = node.get_component_iface().get_extents(Atspi.CoordType.WINDOW)
    return ("refused: %s" % "; ".join(refusals), whole, " window")
def walk(node, depth, document=False):
    if node is None or depth > 40:
        return
    try:
        document = document or node.get_role() == Atspi.Role.DOCUMENT_FRAME
        key = "document" if document else "toolkit"
        if node.get_role_name() in ("text", "entry") and node.get_name() == "N" and key not in fields:
            asked = ask(node)
            if asked is not None:
                fields[key] = asked
        for index in range(node.get_child_count()):
            walk(node.get_child_at_index(index), depth + 1, document)
    except gi.repository.GLib.Error:
        return
desktop = Atspi.get_desktop(0)
for index in range(desktop.get_child_count()):
    application = desktop.get_child_at_index(index)
    if application is not None and application.get_process_id() == pid:
        walk(application, 0)
def place(whole):
    return "%d %d %d %d" % (whole.x, whole.y, whole.width, whole.height)
toolkit, document = fields.get("toolkit"), fields.get("document")
if own and document:
    print("%s in %s%s" % (document[0], place(document[1]), document[2]))
elif toolkit and not toolkit[0].startswith("refused"):
    # Qt's own field answers; the document's node for the same field is read beside it, in the same
    # space, because the bridge places that node and nothing else checks where (ADR 1528).
    beside = ""
    if document and not document[0].startswith("refused") and document[2] == toolkit[2]:
        beside = " document %s" % document[0]
    print("%s in %s%s%s" % (toolkit[0], place(toolkit[1]), toolkit[2], beside))
elif toolkit and document and not document[0].startswith("refused"):
    # The toolkit's own field answers nothing, and the document's node for the same field does:
    # its box is checked against the toolkit field's place in the window, which is where GTK puts
    # the widget over the field's /Rect and the one space both answer in (ADR 1516).
    print("%s in %s window, the document's node; the toolkit's %s" % (
        document[0], place(toolkit[1]), toolkit[0]))
elif toolkit:
    print(toolkit[0])
PY
# answered ROLE NAME: whether asked.py finds that widget, its centre left in ASKED.
answered() { ASKED=$(timeout 20 python3 "$OUT/asked.py" "$APP" "$1" "$2" 2>/dev/null); [ -n "$ASKED" ]; }
# controls_show SEEN NAME…: whether shown.py reads SEEN off the named controls.
controls_show() {
    local want=$1
    shift
    [ "$(timeout 20 python3 "$OUT/shown.py" "$APP" "$@" 2>/dev/null)" = "$want" ]
}
# bus_reads SEEN: whether reopened.py reads SEEN off the bus.
bus_reads() { [ "$(timeout 30 python3 "$OUT/reopened.py" "$APP" "$WINDOW" 2>/dev/null)" = "$1" ]; }
# extents_answer: whether extents.py answers anything at all for the field N.
extents_answer() { [ -n "$(timeout 30 python3 "$OUT/extents.py" "$APP" "$WINDOW" 2>/dev/null)" ]; }
# activity_is STATE: whether active.py says STATE of the window.
activity_is() { [ "$(timeout 20 python3 "$OUT/active.py" "$APP" 2>/dev/null)" = "$1" ]; }
# aimed_at NAME: aim.py's click on the document's node NAME, which fails until the node is published.
aimed_at() { timeout 20 python3 "$OUT/aim.py" "$APP" "$1" > /dev/null 2>&1; }
# asked VARIABLE ROLE NAME: replaces the measured coordinates in VARIABLE with the widget's own
# centre where the window publishes it, and says which the click will be.
asked() {
    local seen=""
    # The two toolkits publish every widget asked for here, so they are asked until it is up — a
    # panel is built when the outline beside page one arrives; `quorra` publishes no panel row,
    # so it is asked once (ADR 1605).
    ASKED=""
    if [ -n "$BUS" ] && [ "$WINDOW" = quorra ]; then
        answered "$2" "$3"
    elif [ -n "$BUS" ]; then
        wait_for 10 answered "$2" "$3"
    fi
    seen=$ASKED
    if [ -n "$seen" ]; then
        printf '%s\t%s\tasked\t%s %s at %s (measured %s)\n' "$WINDOW" "$1" "$2" "$3" "$seen" "${!1}" >> "$COORDINATES"
        printf -v "$1" '%s' "$seen"
    else
        printf '%s\t%s\tmeasured\t%s\n' "$WINDOW" "$1" "${!1}" >> "$COORDINATES"
    fi
}

# --- per-window coordinates, measured on the fixtures above ----------------------------------------
# outline row 2, row 3; a point on the page; the popup icon; the link; the pages tab; pages row 3;
# check box D; choice E; option Blue (screen-absolute for the two toolkits' popups); push button F.
coordinates() {
    case "$WINDOW" in
        quorra)     ROW2="45 57";  ROW3="50 77";  PAGE="550 374"; ICON="635 147"; LINK="450 229"
                    PAGES_TAB="75 12"; PAGE_ROW3="150 400"
                    CHECK="461 400"; CHOICE="230 526"; BLUE="140 600"; BUTTON="568 526" ;;
        quorra-gtk) ROW2="145 85"; ROW3="160 108"; PAGE="690 600"; ICON="793 228"; LINK="570 330"
                    PAGES_TAB="48 121"; PAGE_ROW3="234 126"
                    CHECK="732 482"; CHOICE="556 583"; BLUE="480 699"; BUTTON="823 581" ;;
        quorra-qt)  ROW2="92 141"; ROW3="100 165"; PAGE="690 698"; ICON="796 251"; LINK="568 356"
                    PAGES_TAB="15 193"; PAGE_ROW3="56 146"
                    CHECK="735 494"; CHOICE="555 595"; BLUE="485 684"; BUTTON="825 596" ;;
    esac
}

drive() {
    coordinates
    local copy="$OUT/$WINDOW-drive.pdf" form="$OUT/$WINDOW-form.pdf" seen mark
    cp "$FIXTURES/drive.pdf" "$copy"
    rm -f "${copy%.pdf}.edited.pdf"

    # Table 147's /DisplayDocTitle, /FitWindow, /CenterWindow (ADR 1429).
    launch "$copy"
    shot 01-open
    expect_title 01-display-doc-title "Drive 三窗口 عنوان — page 1 of 3"
    # §12.3.3's section in the opening caption: the outline is read beside page one rather than by
    # the open, and the window says the page again once it has been (ADRs 1543, 1553).
    expect_title 01-section "page 1 of 3 — Chapter one"
    seen=$(xdotool getwindowgeometry "$(main_window)" | tr '\n' ' ')
    case "$WINDOW" in
        quorra-gtk) verdict 01-center-window "not offered" "GTK 4 places no window (ADR 1429): $seen" ;;
        *) case "$seen" in
               *"Position: 0,0 "*) verdict 01-center-window wrong "$seen" ;;
               *) verdict 01-center-window works "$seen" ;;
           esac ;;
    esac

    # §12.3.3: an outline item is activated by a click; a Chinese and an Arabic title.
    asked ROW2 '*' "第二章 页面"; asked ROW3 '*' "الفصل 3: السلام"
    click $ROW2; shot 02-outline-chinese; expect_title 02-outline-chinese "page 2 of 3 — 第二章 页面"
    click $ROW3; shot 03-outline-arabic; expect_title 03-outline-arabic "page 3 of 3"

    # Page turns by key, after a click on the page (trap 57's shape: a toolkit's focus can swallow
    # a key a click elsewhere gave it).
    click $PAGE
    key Home; expect_title 04-key-home "page 1 of 3"
    key Right; expect_title 04-key-right "page 2 of 3"
    key End; expect_title 04-key-end "page 3 of 3"
    key Left; expect_title 04-key-left "page 2 of 3"
    # The press after an arrow key: GTK moves its focus on an arrow nobody claimed (ADR 1453).
    key Home; expect_title 04-key-home-after-left "page 1 of 3"

    # Zoom: keys, then Control and the wheel.
    # The second after the keys is the zoomed frame landing on the photograph, which says nothing.
    key plus; key plus; sleep 1; shot 05-zoom-keys
    [ "$(said 'Zoom\|zoom In')" -gt 0 ] && verdict 05-zoom-keys works "zoom in reached the core" \
        || verdict 05-zoom-keys wrong "no zoom in the trace"
    wheel $PAGE 5 4; shot 05-wheel-scroll
    key 0
    local before
    before=$(said 'Zoom\|zoom In')
    xdotool windowfocus --sync "$(main_window)"; xdotool keydown ctrl
    wheel $PAGE 4; xdotool keyup ctrl; sleep 1; shot 06-zoom-wheel  # the same second, for the same frame
    [ "$(said 'Zoom\|zoom In')" -gt "$before" ] && verdict 06-zoom-wheel works "Control and the wheel zoomed" \
        || verdict 06-zoom-wheel wrong "Control and the wheel scrolled"
    key 0

    # Find: a word, the next and the previous occurrence.
    key Home; key f
    local bar
    case "$WINDOW" in
        quorra-gtk) bar=entry ;;
        quorra-qt) bar=text ;;
        *) bar="" ;;
    esac
    # The find bar is the one text entry on this page in the two native windows; `quorra` draws its
    # own and publishes nothing of it.
    if [ -n "$bar" ] && [ -n "$BUS" ]; then
        FIND_BAR=""; asked FIND_BAR "$bar" '*'
        [ -n "$FIND_BAR" ] && verdict 07-find-bar works "the bar is showing, at $FIND_BAR" \
            || verdict 07-find-bar wrong "f put no find bar on the accessibility bus"
    fi
    type_in drive; key Return
    shot 07-find; expect_title 07-find "page 1 of 3"
    key Return; expect_title 07-find-next "page 2 of 3"
    key shift+Return; shot 08-find-previous; expect_title 08-find-previous "page 1 of 3"
    key Escape; click $PAGE; key Home; key 0

    # §12.5.6.14: the popup opens and closes on its annotation; §12.5.6.5's link.
    # The popup's title bar is drawn in the annotation's /C colour in all three windows (ADR 1466),
    # so the open picture has thousands of pixels of it and the closed one only the icon's.
    click $ICON; shot 10-popup-open
    click $ICON; shot 11-popup-closed
    local open closed
    open=$(yellow "$OUT/shots/$WINDOW/10-popup-open.png"); closed=$(yellow "$OUT/shots/$WINDOW/11-popup-closed.png")
    if [ "${open:-0}" -ge $((${closed:-0} + 1000)) ] && [ "${closed:-0}" -lt 1000 ]; then
        verdict 10-popup works "opened ($open pixels of /C) and closed ($closed, the icon's)"
    else
        verdict 10-popup wrong "pixels of /C: open ${open:-?}, closed ${closed:-?}"
    fi
    click $LINK; shot 12-link; expect_title 12-link "page 3 of 3"

    # §12.5.6.10's highlight over everything selected, then §7.5.6's save read back.
    click $PAGE; key Home; key a; key h; key Escape; shot 13-markup; key_then "$SAVED" ctrl+s
    seen=$(python3 -c "import pikepdf,sys; p=pikepdf.open(sys.argv[1]); print(' '.join(str(a.Subtype) for a in p.pages[0].Annots))" \
        "${copy%.pdf}.edited.pdf" 2>&1)
    case "$seen" in
        *Highlight*) verdict 13-markup-saved works "$seen" ;;
        *) verdict 13-markup-saved wrong "$seen" ;;
    esac

    # §12.3.4: "navigate to a page by clicking its thumbnail image".
    asked PAGES_TAB "page tab" Pages
    click $PAGES_TAB; asked PAGE_ROW3 '*' "Page 3"; click $PAGE_ROW3; shot 14-pages-panel; expect_title 14-pages-panel "page 3 of 3"

    # The four levels (CLAUDE.md principle 3), and print.
    # A toolkit's menu maps from its event loop and `quorra`'s card is drawn, and neither says when.
    click $PAGE; key r; sleep 1; shot 15-restrictions
    if [ "$WINDOW" = quorra ]; then
        # The card is drawn rather than published, so it is driven instead: the second row sets
        # copy to "on", which the core's trace states, and the first sets it back.
        key Down; key Return; wait_for 10 said_since 0 'restrictions in this window copy:On'
        if [ "$(said 'restrictions in this window copy:On')" -gt 0 ]; then
            verdict 15-restrictions works "the card's second row set copy to on"
        else
            verdict 15-restrictions wrong "r put up no card that sets a level"
        fi
        key Up; key Return
    elif [ "$(xdotool search --onlyvisible --pid "$APP" | wc -l)" -gt 1 ]; then
        verdict 15-restrictions works "r put the menu up: look at 15-restrictions.window1.png"
    else
        verdict 15-restrictions wrong "r put no menu up"
    fi
    key Escape; key Escape
    # GTK's is the platform's print dialogue; `quorra` and Qt have no printer and say so (ADR 1180).
    click $PAGE; key shift+p
    local printing=""
    for _ in $(seq 1 100); do
        [ "$WINDOW" = quorra-gtk ] && printing=$(xdotool search --onlyvisible --pid "$APP" --name '^Print$' 2>/dev/null | head -1)
        [ "$WINDOW" != quorra-gtk ] && [ "$(said 'over 3 page(s)')" -gt 0 ] && printing=said
        [ -n "$printing" ] && break
        sleep 0.1
    done
    shot 16-print
    case "$WINDOW:$printing" in
        quorra-gtk:) verdict 16-print wrong "no print dialogue came up" ;;
        quorra-gtk:*) verdict 16-print works "the print dialogue is up: 16-print.window*.png" ;;
        *:said) verdict 16-print works "$(grep -m1 'over 3 page(s)' "$LOG")" ;;
        *) verdict 16-print wrong "print said nothing about three pages" ;;
    esac

    # A host that found itself borrowed dropped what a person asked for, and says so.
    if grep -q "was busy and an action was dropped" "$LOG"; then
        verdict 17-nothing-dropped wrong "$(grep -c 'was busy' "$LOG") action(s) dropped: $LOG"
    fi

    # §7.6.4.1's password.
    LAUNCHED=a_window launch "$FIXTURES/drive-password.pdf"
    # The prompt's entry takes the keyboard once it is mapped, from the toolkit's event loop.
    sleep 1
    shot 17-password-prompt
    local prompt
    prompt=$(xdotool search --onlyvisible --pid "$APP" | tail -1)
    xdotool windowraise "$prompt" 2>/dev/null
    [ "$WINDOW" = quorra-qt ] && { read -r px py <<< "$(xdotool getwindowgeometry --shell "$prompt" | awk -F= '/^[XY]=/{printf "%s ", $2}')"; xdotool mousemove $((px + 240)) $((py + 60)) click 1; }
    xdotool windowfocus --sync "$prompt" 2>/dev/null; xdotool type --delay 300 drive; xdotool key Return
    wait_for 10 title_has "page 1 of 1"
    shot 18-password-open; expect_title 18-password "page 1 of 1"

    # The form: §12.5.1's /Tabs /C, §12.7.4.3's typed value, §12.7.5.2.3's check box, §12.7.5.4's
    # choice, §12.7.5.2.2's push button; saved, and the saved values read back.
    cp "$FIXTURES/drive-form.pdf" "$form"; rm -f "${form%.pdf}.edited.pdf"
    launch "$form"
    click 690 850
    # Each input waits for the core's own line for it (ADR 1605). A Tab is said before GTK gives
    # the control the keyboard from its idle, hence the tenth of a second after it.
    key_then "$FOCUS_NEXT" Tab; sleep 0.1; type_then 'SetField { field: "A", value: Text("1")' 1
    key_then "$FOCUS_NEXT" Tab; sleep 0.1; type_then 'SetField { field: "C", value: Text("2")' 2
    key_then "$FOCUS_NEXT" Tab; key_then "$FOCUS_NEXT" Tab; sleep 0.1
    type_then 'SetField { field: "B", value: Text("3")' 3
    shot 20-tab-order
    asked CHECK "check box" '*'; asked CHOICE "combo box" '*'
    click_then 'SetField { field: "D"' $CHECK; click $CHOICE; shot 21-choice-open
    if [ "$WINDOW" = quorra ]; then
        click_then 'SetField { field: "E"' $BLUE
    else
        mark=$(lines); read -r bx by <<< "$BLUE"; xdotool mousemove "$bx" "$by" click 1
        wait_for 10 said_since "$mark" 'SetField { field: "E"'
    fi
    shot 22-choice-chosen
    click 690 850; key_then "$SAVED" ctrl+s
    click $BUTTON; wait_for 10 title_has "page 2 of 2"; shot 23-push-button
    expect_title 23-push-button "page 2 of 2"
    seen=$(python3 - "${form%.pdf}.edited.pdf" <<'PY'
import sys, pikepdf
p = pikepdf.open(sys.argv[1])
fields = {str(f.T): f for f in p.Root.AcroForm.Fields}
value = lambda n: str(fields[n].get("/V"))
print(value("A"), value("C"), value("B"), repr(fields["D"].get("/V")), repr(fields["D"].get("/AS")), value("E"))
PY
)
    case "$seen" in
        "1 2 3 pikepdf.Name(\"/Yes\") pikepdf.Name(\"/Yes\") Blue") verdict 20-tab-order-and-save works "$seen" ;;
        *) verdict 20-tab-order-and-save wrong "A C B D/V D/AS E: $seen" ;;
    esac
    # The saved values as the reopened window shows them, read off the bus in every window (ADR 1489).
    launch "${form%.pdf}.edited.pdf"
    # Read until the bus says what the file holds, which is the step's claim, or the ceiling
    # passes: the toolkit publishes its controls after page one is up (ADR 1605).
    [ -n "$BUS" ] && wait_for 20 bus_reads "1 2 3 True Blue"
    shot 24-reopened
    seen=""
    [ -n "$BUS" ] && seen=$(timeout 30 python3 "$OUT/reopened.py" "$APP" "$WINDOW" 2>/dev/null)
    if [ "$seen" = "1 2 3 True Blue" ]; then
        verdict 24-reopened works "A C B D E on the bus: $seen"
    else
        verdict 24-reopened wrong "A C B D E on the bus: ${seen:-nothing}"
    fi

    # ADR 1501: a field's second character has a box, inside the field, where AT-SPI asks for it —
    # and where the toolkit's field answers and the document's node is read beside it (Qt), the
    # node's box is inside the toolkit's field too (ADR 1528).
    launch "$FIXTURES/drive-field.pdf"
    [ -n "$BUS" ] && wait_for 20 extents_answer
    shot 29-field-extents
    seen=""
    [ -n "$BUS" ] && seen=$(timeout 30 python3 "$OUT/extents.py" "$APP" "$WINDOW" 2>/dev/null)
    if [ -z "$BUS" ]; then
        verdict 29-field-extents "not offered" "no accessibility bus on this machine"
    elif [[ "$seen" == refused:* ]]; then
        verdict 29-field-extents "not offered" "the toolkit's own field answers GetCharacterExtents with an error: $seen"
    elif read -r cx cy cw ch _ fx fy fw fh _ <<< "$seen" && [ -n "$fh" ] && [ "$cw" -gt 0 ] \
            && [ "$cx" -ge "$fx" ] && [ $((cx + cw)) -le $((fx + fw)) ] \
            && [ "$cy" -ge "$fy" ] && [ $((cy + ch)) -le $((fy + fh)) ] \
            && inside_the_field "$seen" "$fx" "$fy" "$fw" "$fh"; then
        verdict 29-field-extents works "GetCharacterExtents(1): $seen"
    else
        verdict 29-field-extents wrong "GetCharacterExtents(1): ${seen:-nothing}"
    fi

    # AT-SPI's frame says whether the window is the active one, which is what a screen reader
    # follows between applications: active with the keyboard in it, inactive once the keyboard is
    # given to the root window — on two documents, an outline's and a form's (ADR 1565).
    if [ -z "$BUS" ]; then
        verdict 32-window-active "not offered" "no accessibility bus on this machine"
    else
        local active inactive file said=""
        for file in drive drive-form; do
            launch "$FIXTURES/$file.pdf"
            # The frame's state is the witness, so it is waited for in both directions rather than
            # read a second and a half after the keyboard moved (ADR 1605).
            xdotool windowfocus --sync "$(main_window)" 2>/dev/null; wait_for 10 activity_is active
            active=$(timeout 20 python3 "$OUT/active.py" "$APP" 2>/dev/null)
            # The pointer leaves the window first: with the keyboard given to the root, X sends
            # the keys to whatever window is under the pointer, and GTK counts that as having them.
            # shellcheck disable=SC2046
            xdotool mousemove $(outside) 2>/dev/null
            xdotool windowfocus --sync "$(xwininfo -root | awk '/Window id:/ {print $4}')" 2>/dev/null
            wait_for 10 activity_is inactive
            inactive=$(timeout 20 python3 "$OUT/active.py" "$APP" 2>/dev/null)
            [ "$file" = drive ] && shot 32-window-active
            said="$said $file.pdf: ${active:-nothing}, then ${inactive:-nothing};"
        done
        if [[ "$said" == " drive.pdf: active, then inactive; drive-form.pdf: active, then inactive;" ]]; then
            verdict 32-window-active works "active with the keyboard and inactive without it:$said"
        else
            verdict 32-window-active wrong "with the keyboard, then without it:$said"
        fi
    fi

    # §12.7.5.3's text field activated the way a screen reader activates it — the document's node's
    # "click", no coordinates — takes what is typed next, in every window: a caret in the field
    # `quorra` draws, the keyboard in the control a toolkit placed over it (ADR 1566). Two
    # documents, an empty field A and a field N holding "123"; the witness is each saved file's /V.
    if [ -z "$BUS" ]; then
        verdict 33-aimed-field "not offered" "no accessibility bus on this machine"
    else
        local aimed field said="" values=""
        for field in drive-form:A drive-field:N; do
            aimed="$OUT/$WINDOW-aimed-${field%%:*}.pdf"
            cp "$FIXTURES/${field%%:*}.pdf" "$aimed"; rm -f "${aimed%.pdf}.edited.pdf"
            # `access` too, for the line each window says as it gives the field the keyboard.
            LAUNCH_TRACE=events,launch,access launch "$aimed"
            mark=$(lines)
            # Asked until the document's tree is on the bus, which it is shortly after page one.
            if ! wait_for 20 aimed_at "${field##*:}"; then
                said="$said no node ${field##*:} declares a click;"
                continue
            fi
            wait_for 10 said_since "$mark" "keyboard goes to the field ${field##*:}\|typing into the field ${field##*:}"
            # GTK and Qt say it before their event loop gives the control the keyboard.
            sleep 0.2
            type_then "SetField { field: \"${field##*:}\"" z
            [ "${field##*:}" = A ] && shot 33-aimed-field
            click 690 850; key_then "$SAVED" ctrl+s
            seen=$(python3 -c "import pikepdf,sys; p=pikepdf.open(sys.argv[1]); print(str({str(f.T): f for f in p.Root.AcroForm.Fields}[sys.argv[2]].get('/V')))" \
                "${aimed%.pdf}.edited.pdf" "${field##*:}" 2>&1 | tail -1)
            said="$said ${field##*:} is $seen;"
            values="$values ${field##*:}=$seen"
        done
        # N's z replaces "123" where the control selects its text on taking the keyboard (GTK's
        # entry does) and goes in beside it where it does not.
        local typed="${values##* N=}"
        if [ "$values" = " A=z N=$typed" ] && [ "${typed//[^z]/}" = z ] \
                && { [ "${typed//z/}" = "" ] || [ "${typed//z/}" = 123 ]; }; then
            verdict 33-aimed-field works "clicked through AT-SPI, typed z, saved:$said"
        else
            verdict 33-aimed-field wrong "clicked through AT-SPI, typed z, saved:$said"
        fi
    fi

    # Annex O: the text after `#` in a command-line word is the URI's fragment (ADR 0209), carried
    # out left to right as §O.2 requires — `page` opens the page Table Annex O.3 names, and `search`
    # (quoted, as Table Annex O.4 asks of a writer) selects "the first matching word in the
    # document", which the window walks for page by page (ADR 0250).
    launch "$FIXTURES/drive.pdf#page=3"; shot 30-fragment-page
    expect_title 30-fragment-page "page 3 of 3"
    launch "$FIXTURES/drive.pdf#page=3&search=%22drive%22"; wait_for 10 found_since 0
    shot 30-fragment-search
    if found_since 0; then
        verdict 30-fragment-search works "the fragment's word was searched for and found"
    else
        verdict 30-fragment-search wrong "the fragment's search found nothing in the trace"
    fi

    # Table Annex O.4's `fdf` naming an absolute URI: "Open the document and then import the data
    # from the specified FDF or XFDF file", fetched from the drive's loopback server at
    # `--submissions=send` and imported into the form; at `refuse`, said and nothing sent (ADR 1527).
    if [ -n "$PORT" ]; then
        local fetched="http://127.0.0.1:$PORT/values.fdf" served
        launch "$FIXTURES/drive-form.pdf#fdf=$fetched" --submissions=send
        wait_for 10 said_since 0 "import-data: 1 field(s) from $fetched"; shot 31-fragment-fdf
        if [ "$(said "import-data: 1 field(s) from $fetched, into 1 widget(s)")" -gt 0 ]; then
            verdict 31-fragment-fdf works "$(grep -m1 "import-data: 1 field(s) from" "$LOG")"
        else
            verdict 31-fragment-fdf wrong "$(grep -m1 'import-data' "$LOG" || echo 'no import-data line')"
        fi
        served=$(grep -c 'GET /values.fdf' "$SERVED")
        launch "$FIXTURES/drive-form.pdf#fdf=$fetched" --submissions=refuse
        wait_for 10 said_since 0 "import-data: declined"; shot 31-fragment-fdf-refused
        if [ "$(said "import-data: declined — $fetched was not fetched")" -gt 0 ] \
                && [ "$(grep -c 'GET /values.fdf' "$SERVED")" -eq "$served" ]; then
            verdict 31-fragment-fdf-refused works "said, and the server was not asked"
        else
            verdict 31-fragment-fdf-refused wrong "$(grep -m1 'import-data' "$LOG" || echo 'no import-data line')"
        fi
        # And at `ask` (ADR 1540): the question is up and nothing has been sent while it stands;
        # answered yes, the server is asked once and the data lands in the form that asked;
        # answered no, nothing is sent and the window says so.
        local answer
        for answer in yes no; do
            served=$(grep -c 'GET /values.fdf' "$SERVED")
            launch "$FIXTURES/drive-form.pdf#fdf=$fetched" --submissions=ask
            asked_fetch_up "$answer" || continue
            shot "31-fragment-fdf-ask-$answer"
            if [ "$answer" = yes ] && [ "$(said "import-data: 1 field(s) from $fetched, into 1 widget(s)")" -gt 0 ] \
                    && [ "$(grep -c 'GET /values.fdf' "$SERVED")" -eq $((served + 1)) ]; then
                verdict 31-fragment-fdf-ask-yes works "asked, held, answered yes: $(grep -m1 "import-data: 1 field(s) from" "$LOG")"
            elif [ "$answer" = no ] && [ "$(said "import-data: declined — $fetched was not fetched: you answered")" -gt 0 ] \
                    && [ "$(grep -c 'GET /values.fdf' "$SERVED")" -eq "$served" ]; then
                verdict 31-fragment-fdf-ask-no works "asked, answered no, and the server was not asked"
            else
                verdict "31-fragment-fdf-ask-$answer" wrong "$(grep 'import-data' "$LOG" | tr '\n' ' ')"
            fi
        done
    else
        verdict 31-fragment-fdf "not offered" "the loopback server did not start"
    fi

    # A right-to-left word on a page whose text runs in visual order through presentation forms.
    if [ -f "$FIXTURES/arabic.pdf" ]; then
        launch "$FIXTURES/arabic.pdf"
        click 400 600; key f; mark=$(lines); type_in "العربية"; key_then "$SEARCHED" Return
        shot 25-find-arabic
        if found_since "$mark"; then
            verdict 25-find-arabic works "found"
        else
            verdict 25-find-arabic wrong "the word on the page is not found (doc/todo/27)"
        fi
    fi

    # ADR 1477: a word typed without its marks finds one printed with them, and a mark typed is
    # asked for — "كتب" finds "كَتَبَ", "كُتُب" (dammas) does not.
    launch "$FIXTURES/drive-vowelled.pdf"
    click 400 600; key f; mark=$(lines); type_in "كتب"; key_then "$SEARCHED" Return; shot 25-find-vowelled
    if found_since "$mark"; then
        verdict 25-find-vowelled works "the bare word found the vowelled one"
    else
        verdict 25-find-vowelled wrong "the bare word did not find the vowelled one"
    fi
    launch "$FIXTURES/drive-vowelled.pdf"
    click 400 600; key f; mark=$(lines); type_in "كُتُب"; key_then "$SEARCHED" Return
    if found_since "$mark"; then
        verdict 25-find-other-mark wrong "a word with dammas found one with fathas"
    else
        verdict 25-find-other-mark works "a word with dammas did not find one with fathas"
    fi
    # ADR 1490: a TJ in reading order under a mirroring Tm reads back as one word, and is found.
    launch "$FIXTURES/drive-mirrored.pdf"
    click 400 600; key f; mark=$(lines); type_in "عرب"; key_then "$SEARCHED" Return; shot 25-find-mirrored
    if found_since "$mark"; then
        verdict 25-find-mirrored works "the word under a mirroring Tm is found"
    else
        verdict 25-find-mirrored wrong "the word under a mirroring Tm is not found (doc/todo/27)"
    fi

    # Principle 2: a frame the graphics device refuses is drawn on the processor, "reported out
    # loud" — on the terminal and in the window's title, the channel `quorra` reports what a page
    # could not draw through (ADR 1466). The two native windows draw no page through a graphics
    # device, so there is nothing for one to refuse (doc/ui-boundary.md). The page is
    # `drive-coverage.pdf`, whose coverage outgrows the device's scratch sheet (ADR 1478) — the
    # corpus's Type 3 cycle was the page once, and since `MAX_LIST_BYTES` bounds its list the device
    # draws it (ADR 1507).
    REFUSED="$FIXTURES/drive-coverage.pdf"
    if [ -f "$REFUSED" ]; then
        if [ "$WINDOW" = quorra ]; then
            launch "$REFUSED"
            wait_for 120 said_since 0 'drawn on the processor instead'
            # The refusal is said before the processor's page lands off its thread, so the
            # photograph is retaken until the page is on it (ADR 1605).
            wait_for 20 drawn_on_the_shot 27-processor-fallback
            seen=$(title)
            [ -n "$seen" ] || seen=$(xdotool getwindowname \
                "$(xdotool search --name 'drawn on the processor' 2>/dev/null | head -1)" 2>/dev/null)
            colours=$(magick "$OUT/shots/$WINDOW/27-processor-fallback.png" -unique-colors \
                -format %w info: 2>/dev/null)
            if [ "$(said 'drawn on the processor instead')" -eq 0 ]; then
                verdict 27-processor-fallback wrong "no refusal on standard output: the device drew it?"
            elif [[ "$seen" != *"drawn on the processor"* ]]; then
                verdict 27-processor-fallback wrong "said on the terminal only; title: $seen"
            elif [ "${colours:-0}" -le 3 ]; then
                verdict 27-processor-fallback wrong "the page is blank ($colours colours)"
            else
                verdict 27-processor-fallback works "$seen"
            fi
        else
            verdict 27-processor-fallback "not offered" "no page is drawn through a graphics device here"
        fi
    fi
    kill "$APP" 2>/dev/null; wait "$APP" 2>/dev/null; APP=""
}

# frames_published FILE: how many DocumentFrame nodes the window has on the bus, into FILE, and
# whether there is one.
frames_published() {
    timeout 30 python3 - "$APP" > "$1" 2>/dev/null <<'PY'
import sys
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi
pid, frames = int(sys.argv[1]), 0
def walk(node, depth):
    global frames
    if node is None or depth > 6:
        return
    if node.get_role() == Atspi.Role.DOCUMENT_FRAME:
        frames += 1
    for index in range(node.get_child_count()):
        walk(node.get_child_at_index(index), depth + 1)
desktop = Atspi.get_desktop(0)
for index in range(desktop.get_child_count()):
    application = desktop.get_child_at_index(index)
    if application is not None and application.get_process_id() == pid:
        walk(application, 0)
print(frames)
PY
    [ "$(tail -1 "$1" 2>/dev/null)" -gt 0 ] 2>/dev/null
}
# §14.7's tree on a real bus (doc/verify.md's AT-SPI recipe): a DocumentFrame per page shown.
# The answer goes through a file rather than the standard output, and every process either private
# bus started is stopped by the pid the bus names: a service D-Bus activated for the window (the
# registry, a portal) outlives `dbus-run-session` and holds any pipe it inherited open for ever.
accessibility() {
    command -v dbus-run-session >/dev/null && [ -x /usr/lib/at-spi-bus-launcher ] || {
        verdict 26-accessibility "not offered" "at-spi2-core is not installed"; return; }
    local answer="$OUT/$WINDOW.accessibility" found
    rm -f "$answer"
    # On the drive's own bus where it is up: a second at-spi-bus-launcher on this display would
    # take over the accessibility bus's socket, which is keyed by the display, and the windows
    # driven after this one would publish nothing.
    if [ -n "$BUS" ]; then
        launch "$FIXTURES/drive.pdf"
        # Asked until a DocumentFrame is on the bus or the ceiling passes: the bridge publishes the
        # tree once page one is up (ADR 1605).
        wait_for 20 frames_published "$answer"
        kill "$APP" 2>/dev/null; wait "$APP" 2>/dev/null; APP=""
    else
    QT_XCB_NO_XI2=1 timeout 60 dbus-run-session -- bash -c '
        /usr/lib/at-spi-bus-launcher --launch-immediately & sleep 2
        busctl --user set-property org.a11y.Bus /org/a11y/bus org.a11y.Status IsEnabled b true
        a11y=$(busctl --user call org.a11y.Bus /org/a11y/bus org.a11y.Bus GetAddress | cut -d"\"" -f2)
        /usr/lib/at-spi2-registryd & sleep 1
        "$1" "$2" & app=$!; sleep 7
        python3 - > "$3" <<PY
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi
frames = 0
def walk(node, depth):
    global frames
    if node is None or depth > 6:
        return
    if node.get_role() == Atspi.Role.DOCUMENT_FRAME:
        frames += 1
    for index in range(node.get_child_count()):
        walk(node.get_child_at_index(index), depth + 1)
desktop = Atspi.get_desktop(0)
for index in range(desktop.get_child_count()):
    walk(desktop.get_child_at_index(index), 0)
print(frames)
PY
        kill $app
        for pid in $( { busctl --user list --no-legend; busctl --address="$a11y" list --no-legend; } \
                      2>/dev/null | awk "\$2 ~ /^[0-9]+\$/ {print \$2}" | sort -u); do
            [ "$pid" != $$ ] && [ "$(cat /proc/$pid/comm 2>/dev/null)" != dbus-daemon ] && kill "$pid"
        done
        wait' _ "$BIN/$WINDOW" "$FIXTURES/drive.pdf" "$answer" >/dev/null 2>&1 < /dev/null
    fi
    found=$(tail -1 "$answer" 2>/dev/null)
    if [ "${found:-0}" -gt 0 ] 2>/dev/null; then
        verdict 26-accessibility works "$found DocumentFrame node(s) on the bus"
    else
        verdict 26-accessibility wrong "no DocumentFrame reached the bus (${found:-nothing})"
    fi
}

# `quorra-confined`, whose pages come from the sandboxed worker (ADR 0713): a frame its graphics
# device refuses is drawn on the processor and said in the title as well as on the terminal (ADR
# 1466). The page must cross as marks for its device to have anything to refuse — a page whose
# pixels are smaller crosses as pixels (ADR 0607) — so it is `drive-coverage.pdf`, a few kilobytes of
# stars whose coverage outgrows the device's scratch sheet (ADR 1478).
# The reader's three policy words, each launched with the word and without it, so that what the
# word changed is what is judged (ADR 1580). §12.8.1's third question: a signature by a certificate
# the drive's own root issued is called valid only where `--trust-anchors` names that root — and with
# `--accept-unknown-revocation`, because the file carries no §12.8.4 material to settle revocation.
# §8.10.4: a reference XObject draws the blue page `--reference-files` supplies, and its grey proxy
# where nothing is supplied. §8.11.4.4: a group whose /User names Ada is drawn for `--reader-name
# Ada` and not for `--reader-name Bob`.
reader_words() {
    local seen blue grey green
    # Every sentence the window ends §12.8.1's report with, valid or not (ADR 1580).
    local VERIFIED='that signature is\|It does not answer the third\|that signature is not called valid'
    launch "$FIXTURES/drive-signed.pdf" --trust-anchors "$FIXTURES/anchors" --accept-unknown-revocation
    # The window's report on the signature is the witness, and it is waited for (ADR 1605).
    wait_for 10 said_since 0 "$VERIFIED"; shot 34-trust-anchors
    seen=$(grep -o 'that signature is valid: .* reaches an authority you supplied' "$LOG" | head -1)
    if [ -n "$seen" ] && [ "$(said '1 of 1 read as RFC 5280 certificates')" -gt 0 ]; then
        launch "$FIXTURES/drive-signed.pdf"; wait_for 10 said_since 0 "$VERIFIED"
        if [ "$(said 'It does not answer the third')" -gt 0 ] && [ "$(said 'that signature is valid')" -eq 0 ]; then
            verdict 34-trust-anchors works "$seen; and without the word, the third question unasked"
        else
            verdict 34-trust-anchors wrong "valid without the word too, or no report: $LOG"
        fi
    else
        verdict 34-trust-anchors wrong "$(grep -m1 'that signature is not called valid[^.]*' "$LOG" || echo "no verdict: $LOG")"
    fi
    # Each page is photographed until its colour is on it or the ceiling passes (ADR 1605).
    launch "$FIXTURES/drive-reference.pdf" --reference-files "$FIXTURES/targets"
    wait_for 10 holds_colour 35-reference-files "#0000ff"
    blue=$(coloured "$OUT/shots/$WINDOW/35-reference-files.png" "#0000ff")
    launch "$FIXTURES/drive-reference.pdf"
    wait_for 10 holds_colour 35-reference-proxy "#808080"
    grey=$(coloured "$OUT/shots/$WINDOW/35-reference-proxy.png" "#808080")
    if [ "${blue:-0}" -gt 10000 ] && [ "${grey:-0}" -gt 10000 ]; then
        verdict 35-reference-files works "the target's page drawn ($blue blue pixels), the proxy without the word ($grey grey)"
    else
        verdict 35-reference-files wrong "blue with the word ${blue:-?}, grey without it ${grey:-?}"
    fi
    launch "$FIXTURES/drive-audience.pdf" --reader-name Ada
    wait_for 10 holds_colour 36-reader-name "#00ff00"
    green=$(coloured "$OUT/shots/$WINDOW/36-reader-name.png" "#00ff00")
    launch "$FIXTURES/drive-audience.pdf" --reader-name Bob
    # What is judged here is an absence, which no condition can wait for: the page's own frame is
    # what is waited for, and the two seconds are the processor's page landing behind it.
    sleep 2; shot 36-reader-name-other
    seen=$(coloured "$OUT/shots/$WINDOW/36-reader-name-other.png" "#00ff00")
    if [ "${green:-0}" -gt 10000 ] && [ "${seen:-1}" -eq 0 ]; then
        verdict 36-reader-name works "Ada's group drawn for Ada ($green green pixels) and not for Bob"
    else
        verdict 36-reader-name wrong "green for Ada ${green:-?}, for Bob ${seen:-?}"
    fi
    kill "$APP" 2>/dev/null; wait "$APP" 2>/dev/null; APP=""
}

# §12.7.4.3's commit, in all four windows (ADR 1592): a tab out of a field, and Enter in one, commit
# what was typed. 12.5 is accepted and saved under its format; the x of 7x is refused by Table 199's
# /K as it is typed, and said; a lone minus sign is refused at the commit, said, and put back — the
# step's proof that the commit ran, since a save draws every value through its format whether or not
# it was committed. The witnesses are the saved file and the window's own lines.
field_commit() {
    local form="$OUT/$WINDOW-commit.pdf" seen keyed committed
    cp "$FIXTURES/drive-commit.pdf" "$form"; rm -f "${form%.pdf}.edited.pdf"
    launch "$form"
    click 690 850
    # Each input waits for the window's own line for it: the core's command, or the refusal the
    # step judges, and a tenth of a second after a Tab for the form step's reason (ADR 1605).
    key_then "$FOCUS_NEXT" Tab; sleep 0.1; type_then 'SetField { field: "Price1", value: Text("12.5")' 12.5
    key_then "$FOCUS_NEXT" Tab; sleep 0.1; type_then 'AFNumber_Keystroke refused "7x"' 7x
    key_then "$FOCUS_NEXT" Tab; sleep 0.1; type_then 'SetField { field: "Price3", value: Text("-")' -
    key_then 'Price3: the value entered does not match the format of the field' Return
    shot 37-field-commit
    click 690 850; key_then "$SAVED" ctrl+s
    seen=$(python3 - "${form%.pdf}.edited.pdf" <<'PY' 2>&1 | tail -1
import re, sys, pikepdf
p = pikepdf.open(sys.argv[1])
fields = {str(f.T): f for f in p.Root.AcroForm.Fields}
def shown(name):
    ap = fields[name].get("/AP")
    drawn = re.findall(r"\((.*?)\) Tj", ap.N.read_bytes().decode("latin-1")) if ap is not None else []
    return drawn[0] if drawn else "nothing"
print(" ".join(str(fields[n].get("/V")) for n in ["Price1", "Price2", "Price3", "Total"]),
      shown("Price1"), shown("Price2"), shown("Total"))
PY
)
    keyed=$(said 'Price2: the field.s keystroke script AFNumber_Keystroke refused "7x"')
    committed=$(said 'Price3: the value entered does not match the format of the field')
    if [ "$seen" = '12.5 7 None 19.5 $12.50 $7.00 $19.50' ] && [ "${keyed:-0}" -gt 0 ] \
            && [ "${committed:-0}" -gt 0 ]; then
        verdict 37-field-commit works "saved: $seen; the x refused as typed and the minus refused at the commit, both said"
    else
        verdict 37-field-commit wrong "saved: ${seen:-nothing}; keystroke refusal said ${keyed:-0}, commit refusal said ${committed:-0}: $LOG"
    fi
    # ADR 1604: a toolkit's own control shows a committed value as the field displays it — Table
    # 199's /F applied, $12.50 where the field holds 12.5 — and the field's characters again once
    # it holds the keyboard, so that typing starts from them. `quorra` and the confined window
    # place no control and draw the appearance, which the saved file above already witnesses.
    if [ "$WINDOW" = quorra-gtk ] || [ "$WINDOW" = quorra-qt ]; then
        local shown held
        if [ -z "$BUS" ]; then
            verdict 39-field-shown "not offered" "no accessibility bus on this machine"
        else
            wait_for 10 controls_show '$12.50 $7.00 $19.50' Price1 Price2 Total
            shown=$(timeout 20 python3 "$OUT/shown.py" "$APP" Price1 Price2 Total 2>/dev/null)
            wait_for 20 aimed_at Price1 && wait_for 10 controls_show 12.5 Price1
            held=$(timeout 20 python3 "$OUT/shown.py" "$APP" Price1 2>/dev/null)
            if [ "$shown" = '$12.50 $7.00 $19.50' ] && [ "$held" = 12.5 ]; then
                verdict 39-field-shown works "Price1 Price2 Total committed: $shown; Price1 holding the keyboard: $held"
            else
                verdict 39-field-shown wrong "Price1 Price2 Total committed: ${shown:-nothing}; Price1 holding the keyboard: ${held:-nothing}"
            fi
        fi
    fi
    kill "$APP" 2>/dev/null; wait "$APP" 2>/dev/null; APP=""
}

# §12.10's position, in all four windows (ADR 1593): measuring on, one press on the geographic map,
# and the window's sentence carries a latitude and a longitude inside the map's own rectangle of
# degrees, six places each; a press on a projected map whose /GPTS are degrees says why it gives no
# position, which is Q271's refusal by its name.
located() {
    [ -f "$FIXTURES/drive-geographic.pdf" ] || {
        verdict 38-located "not offered" "doc/pdf.js is not checked out"; return; }
    local mark seen refused
    launch "$FIXTURES/drive-geographic.pdf"; mark=$(lines)
    key m; click 700 550; shot 38-located
    seen=$(tail -n "+$((mark + 1))" "$LOG" | grep -o 'geospatial.*' | head -1)
    launch "$FIXTURES/drive-projected.pdf"; mark=$(lines)
    key m; click 700 550; shot 38-located-projected
    refused=$(tail -n "+$((mark + 1))" "$LOG" | grep -o 'no position: the projected system.s registration points are shaped as degrees' | head -1)
    if [[ "$seen" =~ —\ (9|1[0-7])\.[0-9]{6}°\ S,\ 1(6[5-9]|7[0-6])\.[0-9]{6}°\ E ]] && [ -n "$refused" ]; then
        verdict 38-located works "$seen; and the projected map: $refused"
    else
        verdict 38-located wrong "geographic: ${seen:-nothing}; projected: ${refused:-nothing}"
    fi
    kill "$APP" 2>/dev/null; wait "$APP" 2>/dev/null; APP=""
}

confined() {
    [ -x "$BIN/quorra-confined" ] || { verdict 28-confined-refusal "not offered" "no $BIN/quorra-confined"; return; }
    # The worker reads the outline on a thread of its own after the open, and its next answer
    # carries page one's section to the caption (ADR 1553).
    launch "$FIXTURES/drive.pdf"
    expect_title 01-section "page 1 of 3 — Chapter one"
    kill "$APP" 2>/dev/null; wait "$APP" 2>/dev/null; APP=""
    launch "$FIXTURES/drive-coverage.pdf"
    wait_for 120 title_has "drawn on the processor"
    # The title is set when the device refuses, before the processor's page lands off its thread,
    # so the photograph is retaken until the page is on it or a minute has passed: a shot taken in
    # between is blank for a reason that is the clock's, not the window's.
    local seen colours
    wait_for 60 drawn_on_the_shot 28-confined-refusal
    colours=$(magick "$OUT/shots/$WINDOW/28-confined-refusal.png" -unique-colors -format %w info: 2>/dev/null)
    seen=$(title)
    if [ "$(said 'the graphics device refused the frame')" -eq 0 ]; then
        verdict 28-confined-refusal wrong "no refusal on standard error: the device drew it? $seen"
    elif [[ "$seen" != *"drawn on the processor"*"confined" ]]; then
        verdict 28-confined-refusal wrong "said on the terminal only; title: $seen"
    elif [ "${colours:-0}" -le 3 ]; then
        verdict 28-confined-refusal wrong "the page is blank ($colours colours)"
    else
        verdict 28-confined-refusal works "$seen"
    fi
    kill "$APP" 2>/dev/null; wait "$APP" 2>/dev/null; APP=""
}

CLOCK=${EPOCHREALTIME/[.,]/}   # the drive's clock starts with the first window, after the fixtures
for WINDOW in "${WINDOWS[@]}"; do
    if [ "$WINDOW" = quorra-confined ]; then
        confined
        reader_words
        field_commit
        located
        continue
    fi
    drive
    accessibility
    reader_words
    field_commit
    located
done
echo "drive-windows: $(grep -c "	works	" "$RESULTS") works, $(grep -c "	wrong	" "$RESULTS") wrong," \
     "$(grep -c "	manual	" "$RESULTS") to look at; $RESULTS and $OUT/shots"
