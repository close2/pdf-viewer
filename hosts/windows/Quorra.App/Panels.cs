// Read only the visible panel. Thumbnails are decoded only for realized page rows.
using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices.WindowsRuntime;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Markup;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Quorra.Interop;

namespace Quorra.App;

/// <summary>The native panels over the library's row vocabulary, with lazy page thumbnails.</summary>
internal sealed class WindowPanels
{
    private readonly NavigationView navigation;
    private readonly ComboBox choice;
    private readonly ListView list;
    private readonly Func<QuorraWindow?> window;
    private readonly Action<string, Action> guard;
    private readonly Action<string, Action<QuorraWindow>> commit;
    private readonly List<PanelRow> rows = new();
    private readonly HashSet<PanelRow> collapsed = new(ReferenceEqualityComparer.Instance);
    private bool labels;
    private bool changing;
    private bool dirty = true;

    internal WindowPanels(NavigationView navigation, ComboBox choice, ListView list,
        Func<QuorraWindow?> window, Action<string, Action> guard,
        Action<string, Action<QuorraWindow>> commit)
    {
        this.navigation = navigation;
        this.choice = choice;
        this.list = list;
        this.window = window;
        this.guard = guard;
        this.commit = commit;
        list.ItemTemplate = (DataTemplate)XamlReader.Load(
            "<DataTemplate xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\"><StackPanel /></DataTemplate>");
        navigation.PaneOpened += (_, _) => guard("a panel opened", Refresh);
        navigation.SizeChanged += (_, resized) => list.Height = Math.Max(0, resized.NewSize.Height - 96);
        choice.SelectionChanged += (_, _) =>
        {
            if (!changing) { dirty = true; guard("a panel selected", Refresh); }
        };
        list.ContainerContentChanging += (_, args) => guard("a panel row shown", () => Paint(args));
        list.ItemClick += (_, args) =>
        {
            if (args.ClickedItem is int page)
                commit("a page selected", open => open.GoToPage(page));
            else if (args.ClickedItem is PanelRow row)
            {
                if (row.Kind == Native.QUORRA_ROW_ACTIVATE)
                    commit("a panel item activated", open => open.Activate(row.Number, row.Generation));
                else if (row.Kind == Native.QUORRA_ROW_EXTRACT && row.Name is string name)
                    commit("an attachment extracted", open => open.Extract(name));
            }
        };
    }

    internal void Toggle() => navigation.IsPaneOpen = !navigation.IsPaneOpen;

    internal void Changed()
    {
        dirty = true;
        if (navigation.IsPaneOpen) guard("a panel updated", Refresh);
    }

    private void Refresh()
    {
        if (!navigation.IsPaneOpen || !dirty || window() is not QuorraWindow open) return;
        if (!labels)
        {
            changing = true;
            try
            {
                choice.ItemsSource = QuorraWindow.PanelLabels();
                choice.SelectedIndex = 0;
                labels = true;
            }
            finally { changing = false; }
        }
        uint tab = checked((uint)choice.SelectedIndex);
        rows.Clear();
        collapsed.Clear();
        if (tab == Native.QUORRA_TAB_PAGES)
        {
            // Enumerating indices does not read page dictionaries or decode thumbnails.
            list.ItemsSource = Enumerable.Range(0, open.PageCount);
        }
        else
        {
            rows.AddRange(open.Panel(tab));
            foreach (PanelRow row in rows) if (!row.Expanded) collapsed.Add(row);
            ShowRows();
        }
        dirty = false;
    }

    private void ShowRows()
    {
        var visible = new List<PanelRow>();
        uint? hiddenBelow = null;
        foreach (PanelRow row in rows)
        {
            if (hiddenBelow is uint depth && row.Depth > depth) continue;
            hiddenBelow = null;
            visible.Add(row);
            if (collapsed.Contains(row)) hiddenBelow = row.Depth;
        }
        list.ItemsSource = visible;
    }

    private void Paint(ContainerContentChangingEventArgs args)
    {
        if (args.ItemContainer.ContentTemplateRoot is not StackPanel content) return;
        content.Children.Clear();
        if (args.InRecycleQueue || window() is not QuorraWindow open) return;
        args.Handled = true;
        if (args.Item is int page)
        {
            content.Children.Add(Text(open.PageLabel(page)));
            if (open.Thumbnail(page) is Thumbnail thumbnail)
            {
                PageArea.ToPremultipliedBgra(thumbnail.Pixels);
                var bitmap = new WriteableBitmap(checked((int)thumbnail.Width), checked((int)thumbnail.Height));
                thumbnail.Pixels.CopyTo(bitmap.PixelBuffer);
                bitmap.Invalidate();
                content.Children.Add(new Image { Source = bitmap, MaxHeight = 180, MaxWidth = 220,
                    Stretch = Stretch.Uniform, HorizontalAlignment = HorizontalAlignment.Left });
                if (thumbnail.Departures != 0)
                    content.Children.Add(Text("The document's thumbnail does not meet its format constraints."));
            }
            return;
        }
        if (args.Item is not PanelRow row) return;
        var line = new StackPanel { Orientation = Orientation.Horizontal,
            Margin = new Thickness(Math.Min(row.Depth, 16) * 12, 0, 0, 0), Spacing = 4 };
        int index = rows.FindIndex(candidate => ReferenceEquals(candidate, row));
        if (index >= 0 && index + 1 < rows.Count && rows[index + 1].Depth > row.Depth)
        {
            var fold = new Button { Content = collapsed.Contains(row) ? "▸" : "▾",
                Padding = new Thickness(4) };
            fold.Click += (_, _) => guard("a panel branch folded", () =>
            {
                if (!collapsed.Remove(row)) collapsed.Add(row);
                ShowRows();
            });
            line.Children.Add(fold);
        }
        if (row.Kind == Native.QUORRA_ROW_TOGGLE)
        {
            var toggle = new CheckBox { Content = row.Title, IsChecked = row.On, IsEnabled = !row.Locked };
            toggle.Checked += (_, _) => commit("a layer enabled", current => current.SetGroup(row, true));
            toggle.Unchecked += (_, _) => commit("a layer disabled", current => current.SetGroup(row, false));
            line.Children.Add(toggle);
        }
        else line.Children.Add(Text(row.Title));
        content.Children.Add(line);
        if (row.Detail.Length > 0) content.Children.Add(Text(row.Detail));
    }

    private static TextBlock Text(string text) => new() { Text = text, TextWrapping = TextWrapping.Wrap };
}
