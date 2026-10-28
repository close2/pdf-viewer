// How often the display refreshes, for the one place the window needs it: how long a zoom command's
// old picture is held before the scaled stand-in takes over (A657, ADR 2510).

using System.Runtime.InteropServices;

namespace Quorra.App;

/// <summary>The refresh rate of the screen the window is on, in the unit the library takes it in.</summary>
internal static partial class DisplayRefresh
{
    /// <summary>GetDeviceCaps' VREFRESH index.</summary>
    private const int VerticalRefresh = 116;

    [LibraryImport("user32.dll")]
    private static partial nint GetDC(nint window);

    [LibraryImport("user32.dll")]
    private static partial int ReleaseDC(nint window, nint context);

    [LibraryImport("gdi32.dll")]
    private static partial int GetDeviceCaps(nint context, int index);

    /// <summary>
    /// The primary screen's refresh in millihertz, or 0 where Windows states none (it answers 0 or 1
    /// for "the hardware's default", which is no rate), and the library then holds for a sixtieth of
    /// a second.
    /// </summary>
    internal static uint Millihertz()
    {
        nint screen = GetDC(0);
        if (screen == 0)
        {
            return 0;
        }
        try
        {
            int hertz = GetDeviceCaps(screen, VerticalRefresh);
            return hertz > 1 ? (uint)hertz * 1000u : 0u;
        }
        finally
        {
            _ = ReleaseDC(0, screen);
        }
    }
}
