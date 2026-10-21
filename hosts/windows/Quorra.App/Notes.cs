// §12.5.6.14's popup windows: one for each open popup quorra_window hands over, drawn in Windows'
// own window furniture at the rectangle the document states, and closed by activating the popup
// (RFC 0010 step N3a, ADR 2169). The clause makes a popup "a window … for entry and editing" with
// "no appearance stream", so nothing of it is on the page and all of it is here.

using System;
using System.Collections.Generic;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace Quorra.App;

/// <summary>The popup windows over the page, rebuilt whenever QUORRA_OWED_POPUPS says the set or a place changed.</summary>
internal sealed class PageNotes
{
    private readonly Canvas layer;
    private readonly Action<Note> close;
    private readonly Action<string> log;
    private readonly HashSet<(uint, ushort)> saidRich = new();

    /// <summary>Popup windows in <paramref name="layer"/>.</summary>
    /// <param name="layer">The page area's top layer.</param>
    /// <param name="close">Closes one, which is quorra_window_activate on the popup (quorra.h).</param>
    /// <param name="log">Where what this window does not draw of a note is said.</param>
    internal PageNotes(Canvas layer, Action<Note> close, Action<string> log)
    {
        this.layer = layer;
        this.close = close;
        this.log = log;
    }

    /// <summary>The popup windows made again for <paramref name="notes"/>, in device pixels at <paramref name="scale"/> to a logical one.</summary>
    internal void Show(IReadOnlyList<Note> notes, double scale)
    {
        layer.Children.Clear();
        if (scale <= 0)
        {
            return;
        }
        foreach (Note note in notes)
        {
            // Table 172's /RC "shall be displayed in the popup window when the annotation is
            // opened"; this window draws /Contents, the plain text the same note states, and says
            // once that the formatting is not drawn rather than drop it (trap 5).
            if (note.Rich && saidRich.Add((note.Number, note.Generation)))
            {
                log($"the note in popup {note.Number} {note.Generation} states rich text (Table 172's /RC), and this window shows its plain /Contents");
            }
            FrameworkElement window = Window(note);
            float[] quad = note.Quad;
            double left = Math.Min(Math.Min(quad[0], quad[2]), Math.Min(quad[4], quad[6]));
            double right = Math.Max(Math.Max(quad[0], quad[2]), Math.Max(quad[4], quad[6]));
            double top = Math.Min(Math.Min(quad[1], quad[3]), Math.Min(quad[5], quad[7]));
            double bottom = Math.Max(Math.Max(quad[1], quad[3]), Math.Max(quad[5], quad[7]));
            Canvas.SetLeft(window, left / scale);
            Canvas.SetTop(window, top / scale);
            window.Width = (right - left) / scale;
            window.Height = (bottom - top) / scale;
            layer.Children.Add(window);
        }
    }

    // One popup: a title bar in Table 166's /C — "[t]he title bar of the annotation's popup window"
    // — or Windows' own where none is stated, then the note's text, which scrolls.
    private Border Window(Note note)
    {
        var title = new TextBlock
        {
            Text = note.Modified.Length == 0 ? note.Title : $"{note.Title}  {note.Modified}",
            TextTrimming = TextTrimming.CharacterEllipsis,
            VerticalAlignment = VerticalAlignment.Center,
            Margin = new Thickness(6, 2, 6, 2),
        };
        var closing = new Button { Content = "×", Padding = new Thickness(6, 0, 6, 0), MinWidth = 0, MinHeight = 0 };
        closing.Click += (_, _) => close(note);
        var bar = new Grid();
        bar.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        bar.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        bar.Children.Add(title);
        Grid.SetColumn(closing, 1);
        bar.Children.Add(closing);
        var head = new Border { Child = bar };
        if (note.Colour is float[] colour)
        {
            head.Background = new SolidColorBrush(Microsoft.UI.ColorHelper.FromArgb(
                255, Channel(colour[0]), Channel(colour[1]), Channel(colour[2])));
        }
        var body = new ScrollViewer
        {
            Content = new TextBlock
            {
                Text = note.Contents,
                TextWrapping = TextWrapping.Wrap,
                IsTextSelectionEnabled = true,
                Margin = new Thickness(6),
            },
        };
        var frame = new Grid();
        frame.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        frame.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        frame.Children.Add(head);
        Grid.SetRow(body, 1);
        frame.Children.Add(body);
        var window = new Border { Child = frame, BorderThickness = new Thickness(1) };
        if (Application.Current.Resources.TryGetValue("SolidBackgroundFillColorBaseBrush", out object? fill) && fill is Brush background)
        {
            window.Background = background;
        }
        if (Application.Current.Resources.TryGetValue("SurfaceStrokeColorDefaultBrush", out object? stroke) && stroke is Brush edge)
        {
            window.BorderBrush = edge;
        }
        return window;
    }

    // A DeviceRGB component in [0, 1] as a byte, a value outside the range taken to its nearest end
    // and one that is not a number to none of the colour.
    private static byte Channel(float component) =>
        float.IsNaN(component) ? (byte)0 : (byte)Math.Round(255 * Math.Clamp(component, 0f, 1f));
}
