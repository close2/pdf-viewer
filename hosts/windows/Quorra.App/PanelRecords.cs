// Native panel handles are borrowed only while their rows are copied into managed values.
using System;
using System.Collections.Generic;
using Quorra.Interop;

namespace Quorra.App;

/// <summary>A panel row with its hierarchy and the action the library assigns to it.</summary>
internal sealed record PanelRow(string Title, string Detail, uint Depth, bool Expanded,
    uint Kind, uint Number, ushort Generation, bool On, bool Locked, string? Name);

/// <summary>A producer-supplied thumbnail, in straight RGBA before the window converts its copy.</summary>
internal sealed record Thumbnail(uint Width, uint Height, byte[] Pixels, uint Departures);

internal sealed unsafe partial class QuorraWindow
{
    internal static List<string> PanelLabels()
    {
        var labels = new List<string>();
        for (uint tab = 0; tab < Native.QUORRA_TAB_COUNT; tab++)
        {
            uint at = tab;
            labels.Add(Library.ReadText("quorra_tab_label",
                (into, cap, needed) => Native.quorra_tab_label(at, into, cap, needed)));
        }
        return labels;
    }

    internal int PageCount
    {
        get
        {
            nuint count = 0;
            Library.Check("quorra_window_page_count", Native.quorra_window_page_count(handle, &count));
            return checked((int)count);
        }
    }

    internal string PageLabel(int page) => Library.ReadText("quorra_window_page_label",
        (into, cap, needed) => Native.quorra_window_page_label(handle, checked((nuint)page), into, cap, needed));

    internal void GoToPage(int page) => Library.Check("quorra_window_go_to_page",
        Native.quorra_window_go_to_page(handle, checked((nuint)page)));

    internal void SetGroup(PanelRow row, bool on) => Library.Check("quorra_window_set_group",
        Native.quorra_window_set_group(handle, row.Number, row.Generation, on));

    internal void Extract(string name) => Library.Check("quorra_window_extract", Native.quorra_window_extract(handle, name));

    internal List<PanelRow> Panel(uint tab)
    {
        if (tab == Native.QUORRA_TAB_CONTENTS) return Outline();
        nint taken = 0;
        int status = tab switch
        {
            Native.QUORRA_TAB_LAYERS => Native.quorra_window_layers_read(handle, &taken),
            Native.QUORRA_TAB_FILES => Native.quorra_window_attachments_read(handle, &taken),
            Native.QUORRA_TAB_ARTICLES => Native.quorra_window_articles_read(handle, &taken),
            Native.QUORRA_TAB_DOCUMENT => Native.quorra_window_properties_read(handle, &taken),
            _ => throw new QuorraException($"unknown panel {tab}")
        };
        // No answer means this document has no rows for the selected optional panel.
        if (status == Native.QUORRA_NO_ANSWER) return new List<PanelRow>();
        Library.Check("quorra_window_panel_read", status);
        nint panel = taken;
        try
        {
            var rows = new List<PanelRow>();
            for (nuint index = 0; index < Native.quorra_panel_len(panel); index++)
            {
                nuint at = index;
                uint depth = 0, kind = 0, number = 0;
                ushort generation = 0;
                byte expanded = 0, on = 0, locked = 0;
                Library.Check("quorra_panel_depth", Native.quorra_panel_depth(panel, at, &depth, &expanded));
                Library.Check("quorra_panel_action", Native.quorra_panel_action(panel, at, &kind, &number,
                    &generation, &on, &locked));
                string title = Library.ReadText("quorra_panel_text",
                    (into, cap, needed) => Native.quorra_panel_text(panel, at, 0, into, cap, needed));
                string detail = Library.ReadTextOrNone("quorra_panel_text",
                    (into, cap, needed) => Native.quorra_panel_text(panel, at, 1, into, cap, needed)) ?? string.Empty;
                string? name = kind == Native.QUORRA_ROW_EXTRACT ? Library.ReadText("quorra_panel_name",
                    (into, cap, needed) => Native.quorra_panel_name(panel, at, into, cap, needed)) : null;
                rows.Add(new PanelRow(title, detail, depth, expanded != 0, kind, number, generation,
                    on != 0, locked != 0, name));
            }
            return rows;
        }
        finally { Native.quorra_panel_free(panel); }
    }

    private List<PanelRow> Outline()
    {
        nint taken = 0;
        int status = Native.quorra_window_outline_read(handle, &taken);
        if (status == Native.QUORRA_NO_ANSWER) return new List<PanelRow>();
        Library.Check("quorra_window_outline_read", status);
        nint outline = taken;
        try
        {
            var rows = new List<PanelRow>();
            for (nuint index = 0; index < Native.quorra_outline_len(outline); index++)
            {
                nuint at = index;
                uint depth = 0, number = 0;
                ushort generation = 0;
                byte expanded = 0;
                Library.Check("quorra_outline_depth", Native.quorra_outline_depth(outline, at, &depth, &expanded));
                Library.Check("quorra_outline_object", Native.quorra_outline_object(outline, at, &number, &generation));
                string title = Library.ReadText("quorra_outline_title",
                    (into, cap, needed) => Native.quorra_outline_title(outline, at, into, cap, needed));
                rows.Add(new PanelRow(title, string.Empty, depth, expanded != 0, Native.QUORRA_ROW_ACTIVATE,
                    number, generation, false, false, null));
            }
            return rows;
        }
        finally { Native.quorra_outline_free(outline); }
    }

    internal Thumbnail? Thumbnail(int page)
    {
        nint taken = 0;
        int status = Native.quorra_window_thumbnail_read(handle, checked((nuint)page), &taken);
        if (status == Native.QUORRA_NO_ANSWER) return null;
        Library.Check("quorra_window_thumbnail_read", status);
        try
        {
            uint width = 0, height = 0, format = 0, departures = 0;
            nuint length = 0;
            Library.Check("quorra_thumbnail_info", Native.quorra_thumbnail_info(taken, &width, &height,
                &format, &length, &departures));
            if (format != Native.QUORRA_FORMAT_RGBA8 || (ulong)length != (ulong)width * height * 4)
                throw new QuorraException("the thumbnail has an unexpected format or byte count");
            byte[] pixels = new byte[checked((int)length)];
            nuint written = 0;
            fixed (byte* into = pixels)
                Library.Check("quorra_thumbnail_copy", Native.quorra_thumbnail_copy(taken, into, length, &written));
            if (written != length) throw new QuorraException("the thumbnail copy changed size");
            return new Thumbnail(width, height, pixels, departures);
        }
        finally { Native.quorra_thumbnail_free(taken); }
    }
}
