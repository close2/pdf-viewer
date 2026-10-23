#!/usr/bin/env python3
"""Activate one process's window on a KWin session and confirm it is the active one (ADR 2266).

Run as `python3 -I -B tools/zoom-probe-activate.py <pid> <report-file>` in the session whose KWin
holds the window. A KWin script's print() is not readable outside the compositor's own log, so the
script reports over D-Bus to a name this process owns for the length of one question. Exits 0 only
when the active window's pid is the one asked for; a capture of the active window taken after that
is a capture of this window, never of whatever else is in front (trap 234).
"""
import subprocess, sys, tempfile, os
import dbus, dbus.service, dbus.mainloop.glib
from gi.repository import GLib

pid, out = int(sys.argv[1]), sys.argv[2]
NAME = "org.quorra.ZoomProbe"
dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
bus = dbus.SessionBus()
heard = []
loop = GLib.MainLoop()

class Probe(dbus.service.Object):
    @dbus.service.method(NAME, in_signature="s", out_signature="")
    def Report(self, text):
        heard.append(str(text)); loop.quit()

name = dbus.service.BusName(NAME, bus)
Probe(bus, "/probe")
script = f"""
const want = {pid};
let found = null;
for (const w of workspace.windowList()) {{ if (w.pid === want && w.normalWindow) {{ found = w; }} }}
if (found) {{ workspace.activeWindow = found; }}
const a = workspace.activeWindow;
const g = a ? a.frameGeometry : null;
callDBus("{NAME}", "/probe", "{NAME}", "Report",
  "found=" + (found ? 1 : 0) + " active_pid=" + (a ? a.pid : -1) + " caption=" + (a ? a.caption : "") +
  " frame=" + (g ? [g.x, g.y, g.width, g.height].join(",") : "none"));
"""
with tempfile.NamedTemporaryFile("w", suffix=".js", delete=False) as f:
    f.write(script); path = f.name
kwin = bus.get_object("org.kde.KWin", "/Scripting")
plugin = f"zoom-probe-activate-{os.getpid()}"
number = bus.call_blocking("org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting", "loadScript", "ss", (path, plugin))
GLib.timeout_add(500, lambda: (dbus.Interface(bus.get_object("org.kde.KWin", f"/Scripting/Script{int(number)}"), "org.kde.kwin.Script").run(), False)[1])
GLib.timeout_add(5000, loop.quit)
loop.run()
bus.call_blocking("org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting", "unloadScript", "s", (plugin,))
os.unlink(path)
text = heard[0] if heard else "no report"
open(out, "w").write(text + "\n")
print(text)
sys.exit(0 if heard and f"active_pid={pid} " in text else 1)
