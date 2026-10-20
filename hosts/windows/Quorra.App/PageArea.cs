// The page area on the processor lane: each picture quorra_window hands over shown in an Image, at
// the place and the extent the window states (RFC 0010 step N3a, ADR 2136). The device's
// SwapChainPanel replaces the Images in step N3c; the area's input stays where it is.

using System.Collections.Generic;
using System.Runtime.InteropServices.WindowsRuntime;
using Microsoft.UI.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Windows.Foundation;

namespace Quorra.App;

/// <summary>The page area: the window's pictures, its keyboard focus and its pointer.</summary>
/// <remarks>
/// A control rather than a bare panel, because a panel takes no keyboard focus and the cursor over
/// a link is <c>ProtectedCursor</c>, which only a subclass can set.
/// </remarks>
public sealed partial class PageArea : ContentControl
{
    private readonly Canvas canvas = new();
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
                canvas.Children.Add(made);
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
            canvas.Children.Remove(images[^1]);
            images.RemoveAt(images.Count - 1);
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
}
