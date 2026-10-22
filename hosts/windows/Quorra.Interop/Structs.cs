// quorra.h's structs, field for field in the header's order. A C `bool` field is a `byte` here:
// C gives it one byte, and a `byte` keeps the struct blittable, so its managed layout is the C
// layout and a pointer to it can be handed over without a marshaller in between.

using System.Runtime.InteropServices;

namespace Quorra.Interop;

// The library writes these fields through a pointer and no C# assigns them, which is what CS0649
// would otherwise say about every one.
#pragma warning disable CS0649

[StructLayout(LayoutKind.Sequential)]
internal struct quorra_geometry
{
    internal float page_width;
    internal float page_height;
    internal float scale;
    internal uint width;
    internal uint height;
    internal float origin_x;
    internal float origin_y;
}

[StructLayout(LayoutKind.Sequential)]
internal struct quorra_frame
{
    internal nuint page;
    internal uint width;
    internal uint height;
    internal uint format;
    internal nuint bytes;
    internal float origin_x;
    internal float origin_y;
}

[StructLayout(LayoutKind.Sequential)]
internal struct quorra_viewing
{
    internal nuint page;
    internal uint zoom;
    internal float scale;
    internal float scroll_x;
    internal float scroll_y;
}

[StructLayout(LayoutKind.Sequential)]
internal struct quorra_rich_tabs
{
    internal nuint stops;
    internal byte has_interval;
    internal float interval_per_base;
    internal float interval_points;
}

[StructLayout(LayoutKind.Sequential)]
internal struct quorra_rich_tab
{
    internal uint align;
    internal float at_per_base;
    internal float at_points;
}

[StructLayout(LayoutKind.Sequential)]
internal struct quorra_rich_leader
{
    internal uint pattern;
    internal uint rule_style;
    internal byte has_thickness;
    internal float thickness_per_base;
    internal float thickness_points;
    internal byte has_width;
    internal float width_per_base;
    internal float width_points;
}

[StructLayout(LayoutKind.Sequential)]
internal struct quorra_rich_paragraph
{
    internal uint align;
    internal ushort level;
    internal byte has_tag;
    internal nuint pieces;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct quorra_rich_run
{
    internal byte tag;
    internal byte bold;
    internal byte italic;
    internal byte has_colour;
    internal fixed float colour[3];
    internal float size_per_base;
    internal float size_points;
    internal float rise_per_base;
    internal float rise_points;
    internal float spacing_per_base;
    internal float spacing_points;
    internal float spacing_of_space;
    internal float horizontal_scale;
    internal float vertical_scale;
    internal byte underlines;
    internal byte underline_by_word;
    internal byte line_through;
    internal nuint families;
}

[StructLayout(LayoutKind.Sequential)]
internal struct quorra_window_chrome_state
{
    internal uint menu_bar;
    internal uint tool_bar;
    internal uint window_ui;
    internal uint other_windows;
}
