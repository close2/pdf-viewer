// The page area on the processor lane: each picture quorra_window hands over shown in an Image, at
// the place and the extent the window states, the marks over them drawn in the platform's colours,
// and the controls over the page above both (RFC 0010 step N3a, ADRs 2136, 2153). The device's
// SwapChainPanel replaces the Images in step N3c; the area's input and the layers over it stay.

using System;
using System.Collections.Generic;
using System.Runtime.InteropServices.WindowsRuntime;
using Microsoft.UI.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;
using Windows.UI.ViewManagement;

namespace Quorra.App;

/// <summary>The page area: the window's pictures, the marks and controls over them, its keyboard focus and its pointer.</summary>
/// <remarks>
/// A control rather than a bare panel, because a panel takes no keyboard focus and the cursor over
/// a link is <c>ProtectedCursor</c>, which only a subclass can set. Three layers, in the order they
/// are drawn: the pictures, the marks, which the pointer passes through to the page, and the
/// controls, which take the pointer and the keyboard themselves.
/// </remarks>
public sealed partial class PageArea : ContentControl
{
    // The weights of the platform's one highlight colour, which are viewer-qt's choice, so the four
    // windows draw the three kinds of mark alike: Annex O's highlight is how the document was
    // opened, the find bar's other occurrences are where else the needle is, and the selection is
    // what a person is doing, so the three are one hue and never one weight (ADR 2153).
    private const double HighlightWeight = 0.18;
    private const double MatchWeight = 0.12;
    private const double SelectionWeight = 0.35;

    // §12.5.1's focus ring, in logical pixels.
    private const double RingThickness = 2.0;

    private readonly Canvas canvas = new();
    private readonly Canvas pictures = new();
    private readonly Canvas marks = new() { IsHitTestVisible = false };
    private readonly Canvas controls = new();
    private readonly UISettings colours = new();
    private readonly RectangleGeometry clip = new();
    private readonly List<Image> images = new();
    private InputCursor? hand;
    private InputCursor? arrow;
    private bool overLink;

    /// <summary>An empty page area that takes the keyboard.</summary>
    public PageArea()
    {
        IsTabStop = true;
        HorizontalContentAlignment = HorizontalAlignment.Stretch;
        VerticalContentAlignment = VerticalAlignment.Stretch;
        // A Canvas draws its children wherever they are placed, so a page scrolled past the area's
        // edge would draw over the status line without the clip.
        canvas.Clip = clip;
        canvas.Children.Add(pictures);
        canvas.Children.Add(marks);
        canvas.Children.Add(controls);
        Content = canvas;
        // The Canvas carries the area's background, because an element with none is transparent to
        // the pointer and a press between two pages would reach nothing.
        canvas.Background = Background;
        RegisterPropertyChangedCallback(BackgroundProperty, (_, _) => canvas.Background = Background);
        SizeChanged += (_, resized) => clip.Rect = new Rect(0, 0, resized.NewSize.Width, resized.NewSize.Height);
    }

    /// <summary>Shows <paramref name="pictures"/>, in device pixels at <paramref name="scale"/> to a logical one.</summary>
    /// <remarks>
    /// Each picture's pixels are converted in place, so a picture is shown once. The Images and
    /// their bitmaps are kept from one repaint to the next and a bitmap is made again only where a
    /// picture's size changed.
    /// </remarks>
    internal void Show(IReadOnlyList<Picture> pictures, double scale)
    {
        for (int index = 0; index < pictures.Count; index++)
        {
            Picture picture = pictures[index];
            if (index == images.Count)
            {
                var made = new Image { Stretch = Stretch.Fill };
                images.Add(made);
                pictures.Children.Add(made);
            }
            Image image = images[index];
            image.Source = Bitmap(image.Source as WriteableBitmap, picture);
            // Logical pixels, which layout rounding puts on the display's own pixel grid, so a
            // picture shown at its raster's extent is drawn one pixel for one.
            Canvas.SetLeft(image, picture.OriginX / scale);
            Canvas.SetTop(image, picture.OriginY / scale);
            image.Width = picture.ShownWidth / scale;
            image.Height = picture.ShownHeight / scale;
        }
        while (images.Count > pictures.Count)
        {
            pictures.Children.Remove(images[^1]);
            images.RemoveAt(images.Count - 1);
        }
    }

    /// <summary>The layer the controls over the page are placed in, above the pictures and the marks.</summary>
    internal Canvas ControlLayer => controls;

    /// <summary>Whether <paramref name="source"/>, an event's original source, is the page rather than a control over it.</summary>
    /// <remarks>
    /// A key typed into a control placed over the page, or a press on one, bubbles up to this area
    /// as well, and it is the control's: the page is told of what reached the page alone.
    /// </remarks>
    internal bool IsPage(object source)
    {
        for (DependencyObject? at = source as DependencyObject; at is not null; at = VisualTreeHelper.GetParent(at))
        {
            if (ReferenceEquals(at, controls))
            {
                return false;
            }
            if (ReferenceEquals(at, this))
            {
                return true;
            }
        }
        return false;
    }

    /// <summary>
    /// Draws <paramref name="shown"/> over the pictures, in device pixels at <paramref name="scale"/>
    /// to a logical one: the marks cross as shapes so that they are drawn in Windows' own highlight
    /// and accent colours (doc/ui-boundary.md, ADR 2153).
    /// </summary>
    internal void ShowMarks(Marks shown, double scale)
    {
        marks.Children.Clear();
        Windows.UI.Color highlight = colours.UIElementColor(UIElementType.Highlight);
        Fill(shown.Highlights, Weighted(highlight, HighlightWeight), scale);
        Fill(shown.Matches, Weighted(highlight, MatchWeight), scale);
        Fill(shown.Selection, Weighted(highlight, SelectionWeight), scale);
        // What a focus ring looks like is the platform's, which is why it crosses as one
        // quadrilateral and not as pixels; the accent is the colour Windows draws its own in.
        if (shown.Focus is float[] focus)
        {
            Polygon ring = PolygonOf(focus, scale);
            ring.Stroke = new SolidColorBrush(colours.GetColorValue(UIColorType.Accent));
            ring.StrokeThickness = RingThickness;
            marks.Children.Add(ring);
        }
    }

    /// <summary>The cursor every window changes over §12.5.6.5's activation region.</summary>
    internal void ShowLinkCursor(bool over)
    {
        if (over == overLink)
        {
            return;
        }
        overLink = over;
        ProtectedCursor = over
            ? (hand ??= InputSystemCursor.Create(InputSystemCursorShape.Hand))
            : (arrow ??= InputSystemCursor.Create(InputSystemCursorShape.Arrow));
    }

    // The picture's pixels in a bitmap of its size, the one passed in where its size already is.
    private static WriteableBitmap Bitmap(WriteableBitmap? kept, Picture picture)
    {
        if ((ulong)picture.Pixels.Length != (ulong)picture.Width * picture.Height * 4)
        {
            throw new QuorraException(
                $"a picture of {picture.Width} by {picture.Height} pixels came with {picture.Pixels.Length} bytes");
        }
        WriteableBitmap bitmap = kept is not null
            && kept.PixelWidth == picture.Width
            && kept.PixelHeight == picture.Height
                ? kept
                : new WriteableBitmap(checked((int)picture.Width), checked((int)picture.Height));
        ToPremultipliedBgra(picture.Pixels);
        picture.Pixels.CopyTo(bitmap.PixelBuffer);
        bitmap.Invalidate();
        return bitmap;
    }

    /// <summary>
    /// QUORRA_FORMAT_RGBA8, straight alpha, as a WriteableBitmap holds it: blue, green, red,
    /// alpha, each colour multiplied by its alpha.
    /// </summary>
    /// <remarks>
    /// A layout change only, made once on the copy the window already owns. The product is rounded
    /// to the nearest whole value; an opaque pixel, which is nearly every pixel of a page, is its
    /// own colour exactly.
    /// </remarks>
    internal static void ToPremultipliedBgra(byte[] pixels)
    {
        for (int at = 0; at + 3 < pixels.Length; at += 4)
        {
            int red = pixels[at];
            int green = pixels[at + 1];
            int blue = pixels[at + 2];
            int alpha = pixels[at + 3];
            pixels[at] = Premultiplied(blue, alpha);
            pixels[at + 1] = Premultiplied(green, alpha);
            pixels[at + 2] = Premultiplied(red, alpha);
        }
    }

    private static byte Premultiplied(int channel, int alpha) => (byte)(((channel * alpha) + 127) / 255);

    private void Fill(IReadOnlyList<float[]> quads, Windows.UI.Color colour, double scale)
    {
        foreach (float[] quad in quads)
        {
            Polygon shape = PolygonOf(quad, scale);
            shape.Fill = new SolidColorBrush(colour);
            marks.Children.Add(shape);
        }
    }

    // A quadrilateral in device pixels as a polygon in this area's logical pixels.
    private static Polygon PolygonOf(float[] quad, double scale)
    {
        var points = new PointCollection();
        for (int corner = 0; corner + 1 < quad.Length; corner += 2)
        {
            points.Add(new Point(quad[corner] / scale, quad[corner + 1] / scale));
        }
        return new Polygon { Points = points };
    }

    // The colour at a fraction of full opacity.
    private static Windows.UI.Color Weighted(Windows.UI.Color colour, double weight) =>
        Microsoft.UI.ColorHelper.FromArgb((byte)Math.Round(255 * weight), colour.R, colour.G, colour.B);
}
