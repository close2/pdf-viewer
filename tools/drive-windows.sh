#!/usr/bin/env bash
# Drives the three windows — `quorra`, `quorra-gtk`, `quorra-qt` — through what a reader does with an
# open document, headless under Xvfb, and photographs every step; then `quorra-confined` through the
# one thing only it can show, a device refusal of a page its sandboxed worker sent as marks.
#
#   tools/drive-windows.sh [--out DIR] [--window NAME]... [--bin DIR] [--display :N]
#
# The list is `doc/verify.md`'s "Driving the three windows": open, §12.2's Table 147 entries, the
# outline, page turns by key and wheel, zoom, find (a Latin word, an Arabic word, a word typed
# without the marks printed on it, and one placed under a mirroring text matrix), a popup, a link,
# a markup, §7.5.6's save read back, the pages panel, the restrictions levels, print, §7.6.4's password, and a form's §12.5.1 tab order, check
# box, choice and push button saved and re-read. Each step's observable is a title, a line the
# window printed, the saved file's bytes, what AT-SPI reads off the window, or a count of pixels of
# a colour the step draws, and the verdict is printed as
# `step<TAB>window<TAB>works|wrong|not offered|manual<TAB>what was seen`, one line each, into
# `$OUT/results.tsv`; the photographs are `$OUT/shots/<window>/<step>.png`, and every other
# top-level window the program has up (a popup, a dialog) is photographed beside it, because with no
# window manager GTK's and Qt's popups are not on the root's picture. A title is a weaker witness
# than the picture (ADR 1453), so no step rests on one where the window says more.
#
# The reopened form is read off the bus in all three windows: the toolkits' widgets, and `quorra`'s
# own form nodes, which carry §12.7.4.3's value as a text run and a choice's options as selectable
# items (ADR 1489).
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

ROOT=$(cd "$(dirname "$0")/.." && pwd)
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
PY
ARABIC="$ROOT/doc/pdf.js/test/pdfs/ArabicCIDTrueType.pdf"
[ -f "$ARABIC" ] && cp "$ARABIC" "$FIXTURES/arabic.pdf"

# --- the instrument --------------------------------------------------------------------------------
# A display somebody else has would put this run's windows beside theirs, and its photographs too.
if [ -e "/tmp/.X${DISPLAY_NUMBER#:}-lock" ]; then
    echo "drive-windows: display $DISPLAY_NUMBER is in use; pass --display :N" >&2
    exit 2
fi
Xvfb "$DISPLAY_NUMBER" -screen 0 1400x1100x24 -nolisten tcp >/dev/null 2>&1 &
XVFB=$!
sleep 1

# A private session bus with AT-SPI enabled on it, so that every window driven below publishes its
# widgets' extents (doc/verify.md's AT-SPI recipe, held for the whole drive).
COORDINATES="$OUT/coordinates.tsv"
: > "$COORDINATES"
if command -v dbus-daemon >/dev/null && [ -x /usr/lib/at-spi-bus-launcher ] \
        && python3 -c 'import gi; gi.require_version("Atspi", "2.0")' 2>/dev/null; then
    read -r address BUS <<< "$(dbus-daemon --session --fork --print-address=1 --print-pid=1 | tr '\n' ' ')"
    export DBUS_SESSION_BUS_ADDRESS=$address
    /usr/lib/at-spi-bus-launcher --launch-immediately >/dev/null 2>&1 < /dev/null &
    sleep 2
    busctl --user set-property org.a11y.Bus /org/a11y/bus org.a11y.Status IsEnabled b true
    A11Y=$(busctl --user call org.a11y.Bus /org/a11y/bus org.a11y.Bus GetAddress | cut -d'"' -f2)
    /usr/lib/at-spi2-registryd >/dev/null 2>&1 < /dev/null &
    sleep 1
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

WINDOW=""     # which of the three is being driven
LOG=""        # its standard output
STEP=0

launch() { # file [arguments]
    local file=$1
    shift
    [ -n "$APP" ] && { kill "$APP" 2>/dev/null; wait "$APP" 2>/dev/null; }
    LOG="$OUT/$WINDOW.$(basename "$file" .pdf).log"
    if [ "$WINDOW" = quorra-qt ]; then
        QT_XCB_NO_XI2=1 "$BIN/$WINDOW" --trace=events "$@" "$file" > "$LOG" 2>&1 &
    else
        "$BIN/$WINDOW" --trace=events "$@" "$file" > "$LOG" 2>&1 &
    fi
    APP=$!
    sleep 5
}
main_window() { xdotool search --onlyvisible --pid "$APP" 2>/dev/null | head -1; }
title() { xdotool getwindowname "$(main_window)" 2>/dev/null; }
origin() { xdotool getwindowgeometry --shell "$(main_window)" 2>/dev/null | awk -F= '/^[XY]=/{printf "%s ", $2}'; }
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
key() { xdotool windowfocus --sync "$(main_window)" 2>/dev/null; xdotool key --delay 300 "$@"; sleep 1.2; }
type_in() { xdotool windowfocus --sync "$(main_window)" 2>/dev/null; xdotool type --delay 300 "$1"; sleep 1.2; }
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
    printf '%s\t%s\t%s\t%s\n' "$1" "$WINDOW" "$2" "$3" | tee -a "$RESULTS"
}
expect_title() { # step needle
    local seen
    seen=$(title)
    case "$seen" in
        *"$2"*) verdict "$1" works "$seen" ;;
        *) verdict "$1" wrong "title: $seen (wanted $2)" ;;
    esac
}
said() { grep -c -- "$1" "$LOG" 2>/dev/null; }
lines() { wc -l < "$LOG" 2>/dev/null || echo 0; }
# found_since N: whether the window said, after line N of its log, that a search found something —
# `quorra`'s trace of the core's answer, or the two native windows' "found" note.
found_since() { tail -n "+$(($1 + 1))" "$LOG" 2>/dev/null | grep -q 'searched: page\|^note: found "'; }
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
# asked VARIABLE ROLE NAME: replaces the measured coordinates in VARIABLE with the widget's own
# centre where the window publishes it, and says which the click will be.
asked() {
    local seen=""
    [ -n "$BUS" ] && seen=$(timeout 20 python3 "$OUT/asked.py" "$APP" "$2" "$3" 2>/dev/null)
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
    local copy="$OUT/$WINDOW-drive.pdf" form="$OUT/$WINDOW-form.pdf" seen
    cp "$FIXTURES/drive.pdf" "$copy"
    rm -f "${copy%.pdf}.edited.pdf"

    # Table 147's /DisplayDocTitle, /FitWindow, /CenterWindow (ADR 1429).
    launch "$copy"
    shot 01-open
    expect_title 01-display-doc-title "Drive 三窗口 عنوان — page 1 of 3"
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
    key plus; key plus; sleep 1; shot 05-zoom-keys
    [ "$(said 'Zoom\|zoom In')" -gt 0 ] && verdict 05-zoom-keys works "zoom in reached the core" \
        || verdict 05-zoom-keys wrong "no zoom in the trace"
    wheel $PAGE 5 4; shot 05-wheel-scroll
    key 0
    local before
    before=$(said 'Zoom\|zoom In')
    xdotool windowfocus --sync "$(main_window)"; xdotool keydown ctrl
    wheel $PAGE 4; xdotool keyup ctrl; sleep 1; shot 06-zoom-wheel
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
    click $PAGE; key Home; key a; key h; key Escape; shot 13-markup; key ctrl+s; sleep 1
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
    click $PAGE; key r; sleep 1; shot 15-restrictions
    if [ "$WINDOW" = quorra ]; then
        # The card is drawn rather than published, so it is driven instead: the second row sets
        # copy to "on", which the core's trace states, and the first sets it back.
        key Down; key Return; sleep 1
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
    for _ in $(seq 1 10); do
        [ "$WINDOW" = quorra-gtk ] && printing=$(xdotool search --onlyvisible --pid "$APP" --name '^Print$' 2>/dev/null | head -1)
        [ "$WINDOW" != quorra-gtk ] && [ "$(said 'over 3 page(s)')" -gt 0 ] && printing=said
        [ -n "$printing" ] && break
        sleep 1
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
    launch "$FIXTURES/drive-password.pdf"
    shot 17-password-prompt
    local prompt
    prompt=$(xdotool search --onlyvisible --pid "$APP" | tail -1)
    xdotool windowraise "$prompt" 2>/dev/null
    [ "$WINDOW" = quorra-qt ] && { read -r px py <<< "$(xdotool getwindowgeometry --shell "$prompt" | awk -F= '/^[XY]=/{printf "%s ", $2}')"; xdotool mousemove $((px + 240)) $((py + 60)) click 1; }
    xdotool windowfocus --sync "$prompt" 2>/dev/null; xdotool type --delay 300 drive; xdotool key Return; sleep 2
    shot 18-password-open; expect_title 18-password "page 1 of 1"

    # The form: §12.5.1's /Tabs /C, §12.7.4.3's typed value, §12.7.5.2.3's check box, §12.7.5.4's
    # choice, §12.7.5.2.2's push button; saved, and the saved values read back.
    cp "$FIXTURES/drive-form.pdf" "$form"; rm -f "${form%.pdf}.edited.pdf"
    launch "$form"
    click 690 850
    key Tab; type_in 1; key Tab; type_in 2; key Tab; key Tab; type_in 3
    shot 20-tab-order
    asked CHECK "check box" '*'; asked CHOICE "combo box" '*'
    click $CHECK; click $CHOICE; shot 21-choice-open
    if [ "$WINDOW" = quorra ]; then click $BLUE; else read -r bx by <<< "$BLUE"; xdotool mousemove "$bx" "$by" click 1; sleep 1.3; fi
    shot 22-choice-chosen
    click 690 850; key ctrl+s; sleep 1
    click $BUTTON; shot 23-push-button; expect_title 23-push-button "page 2 of 2"
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
    launch "${form%.pdf}.edited.pdf"; shot 24-reopened
    seen=""
    [ -n "$BUS" ] && seen=$(timeout 30 python3 "$OUT/reopened.py" "$APP" "$WINDOW" 2>/dev/null)
    if [ "$seen" = "1 2 3 True Blue" ]; then
        verdict 24-reopened works "A C B D E on the bus: $seen"
    else
        verdict 24-reopened wrong "A C B D E on the bus: ${seen:-nothing}"
    fi

    # A right-to-left word on a page whose text runs in visual order through presentation forms.
    if [ -f "$FIXTURES/arabic.pdf" ]; then
        launch "$FIXTURES/arabic.pdf"
        click 400 600; key f; local mark; mark=$(lines); type_in "العربية"; key Return
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
    click 400 600; key f; mark=$(lines); type_in "كتب"; key Return; shot 25-find-vowelled
    if found_since "$mark"; then
        verdict 25-find-vowelled works "the bare word found the vowelled one"
    else
        verdict 25-find-vowelled wrong "the bare word did not find the vowelled one"
    fi
    launch "$FIXTURES/drive-vowelled.pdf"
    click 400 600; key f; mark=$(lines); type_in "كُتُب"; key Return; sleep 1
    if found_since "$mark"; then
        verdict 25-find-other-mark wrong "a word with dammas found one with fathas"
    else
        verdict 25-find-other-mark works "a word with dammas did not find one with fathas"
    fi
    # ADR 1490: a TJ in reading order under a mirroring Tm reads back as one word, and is found.
    launch "$FIXTURES/drive-mirrored.pdf"
    click 400 600; key f; mark=$(lines); type_in "عرب"; key Return; shot 25-find-mirrored
    if found_since "$mark"; then
        verdict 25-find-mirrored works "the word under a mirroring Tm is found"
    else
        verdict 25-find-mirrored wrong "the word under a mirroring Tm is not found (doc/todo/27)"
    fi

    # Principle 2: a frame the graphics device refuses is drawn on the processor, "reported out
    # loud" — on the terminal and in the window's title, the channel `quorra` reports what a page
    # could not draw through (ADR 1466). The two native windows draw no page through a graphics
    # device, so there is nothing for one to refuse (doc/ui-boundary.md).
    CYCLE="$ROOT/doc/pdf.js/test/pdfs/ContentStreamCycleType3insideType3.pdf"
    if [ -f "$CYCLE" ]; then
        if [ "$WINDOW" = quorra ]; then
            launch "$CYCLE"
            for _ in $(seq 1 24); do
                [ "$(said 'drawn on the processor instead')" -gt 0 ] && break
                sleep 5
            done
            sleep 3
            shot 27-processor-fallback
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
        timeout 30 python3 - "$APP" > "$answer" 2>/dev/null <<'PY'
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
confined() {
    [ -x "$BIN/quorra-confined" ] || { verdict 28-confined-refusal "not offered" "no $BIN/quorra-confined"; return; }
    launch "$FIXTURES/drive-coverage.pdf"
    for _ in $(seq 1 24); do
        [[ "$(title)" == *"drawn on the processor"* ]] && break
        sleep 5
    done
    sleep 3
    shot 28-confined-refusal
    local seen colours
    seen=$(title)
    colours=$(magick "$OUT/shots/$WINDOW/28-confined-refusal.png" -unique-colors -format %w info: 2>/dev/null)
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

for WINDOW in "${WINDOWS[@]}"; do
    if [ "$WINDOW" = quorra-confined ]; then
        confined
        continue
    fi
    drive
    accessibility
done
echo "drive-windows: $(grep -c "	works	" "$RESULTS") works, $(grep -c "	wrong	" "$RESULTS") wrong," \
     "$(grep -c "	manual	" "$RESULTS") to look at; $RESULTS and $OUT/shots"
