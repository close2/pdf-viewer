// The controls over the page: one platform control for each widget quorra_window hands over,
// placed over the widget's rectangle, showing what its field holds and handing back what a person
// does to it, and measured against the rectangle it was given (RFC 0010 step N3a, ADRs 2104, 2153,
// 2169). The window opens with the widgets' appearances
// delegated, so a text, check box, radio button or choice field's widget is drawn by its control
// here and by nothing else; a push-button, a signature and a field stating no /FT keep their
// appearance on the page, which is why they get no control.

using System;
using System.Collections.Generic;
using System.Globalization;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Quorra.Interop;
using VirtualKey = Windows.System.VirtualKey;

namespace Quorra.App;

/// <summary>What a person did to a control, which MainWindow carries to quorra_window.</summary>
/// <param name="Typed">Characters typed into a text control, the whole text as it now stands.</param>
/// <param name="Finished">A text control finished with: Enter, or the keyboard taken elsewhere.</param>
/// <param name="Toggled">A check box or radio button clicked.</param>
/// <param name="Chose">A choice control's options chosen, by their index in /Opt.</param>
internal sealed record Commits(
    Action<Placed, string> Typed,
    Action<Placed> Finished,
    Action<Placed> Toggled,
    Action<Placed, IReadOnlyList<nuint>> Chose);

/// <summary>The controls placed over the page, rebuilt when the set changes and moved with the page.</summary>
internal sealed class PageControls
{
    private readonly Canvas layer;
    private readonly Commits commits;
    private readonly Action toPage;
    private readonly List<Placed> fields = new();
    private readonly List<FrameworkElement?> elements = new();
    // The text this window last wrote into each text control, null for the other kinds. WinUI
    // raises TextChanged after the call that set the text has returned, so a change is told apart
    // from this window's own write by its text rather than by a flag raised around the write.
    private readonly List<Written?> written = new();
    // Whether this window is writing a choice control's selection, whose change XAML raises inside
    // the write, one option at a time: a list of several chosen passes through selections nobody
    // made on its way to the field's.
    private bool writing;

    /// <summary>Controls in <paramref name="layer"/>, their commits <paramref name="commits"/>'.</summary>
    /// <param name="layer">The page area's layer over the marks.</param>
    /// <param name="commits">Where what a person does to a control goes.</param>
    /// <param name="toPage">Gives the keyboard back to the page, as Enter in a single-line field does.</param>
    internal PageControls(Canvas layer, Commits commits, Action toPage)
    {
        this.layer = layer;
        this.commits = commits;
        this.toPage = toPage;
    }

    /// <summary>How many widgets the controls stand for, a push-button's among them.</summary>
    internal int Count => fields.Count;

    /// <summary>The controls made again for <paramref name="wanted"/>, as QUORRA_OWED_CONTROLS asks.</summary>
    /// <remarks>
    /// Rebuilt rather than moved, because the set changed: a control being typed into keeps the
    /// keyboard only while it stays, and the window owes this record only where it does not (ADR 2104).
    /// </remarks>
    internal void Rebuild(IReadOnlyList<Placed> wanted, double scale)
    {
        layer.Children.Clear();
        fields.Clear();
        elements.Clear();
        written.Clear();
        for (int index = 0; index < wanted.Count; index++)
        {
            fields.Add(wanted[index]);
            written.Add(null);
            FrameworkElement? element = Make(index, wanted[index]);
            elements.Add(element);
            if (element is not null)
            {
                // §14.9.3: the name "shall be used in place of the actual field name when an
                // interactive PDF processor identifies the field in a user-interface".
                ToolTipService.SetToolTip(element, wanted[index].Shown);
                AutomationProperties.SetName(element is CombBox comb ? comb.Box : element, wanted[index].Shown);
                layer.Children.Add(element);
            }
        }
        Place(wanted, scale);
    }

    /// <summary>
    /// Each placed control against the rectangle it was given, four logical pixels a control: the
    /// width and height it was asked to occupy, then the least it can be (ADR 2169).
    /// </summary>
    /// <remarks>
    /// A XAML control states no least width of its own once its style's MinWidth is set aside, which
    /// <see cref="Fitted"/> does because the rectangle is the document's: a text or a choice control
    /// scrolls what does not fit across. What it cannot be less than is the height its font's line
    /// and its chrome take, and a check box or radio button its glyph, so those are what is measured.
    /// </remarks>
    internal List<int> Measure()
    {
        var sizes = new List<int>();
        foreach (FrameworkElement? element in elements)
        {
            if (element is null)
            {
                continue;
            }
            double width = element.Width;
            double height = element.Height;
            (double leastWidth, double leastHeight) = Least(element, width);
            sizes.Add((int)Math.Round(width));
            sizes.Add((int)Math.Round(height));
            sizes.Add((int)Math.Ceiling(leastWidth));
            sizes.Add((int)Math.Ceiling(leastHeight));
        }
        return sizes;
    }

    // What one control cannot be smaller than, measured with its rectangle's size set aside and put
    // back after: a glyph's both ways, and for the rest the height at the width it was given.
    private static (double Width, double Height) Least(FrameworkElement element, double width)
    {
        FrameworkElement measured = element switch
        {
            CombBox comb => comb.Box,
            ListBox { Items.Count: > 0 } list when list.ContainerFromIndex(0) is ListBoxItem row => row,
            _ => element,
        };
        double keptWidth = measured.Width;
        double keptHeight = measured.Height;
        try
        {
            measured.Width = double.NaN;
            measured.Height = double.NaN;
            bool glyph = element is CheckBox or RadioButton;
            measured.Measure(new Windows.Foundation.Size(glyph ? double.PositiveInfinity : width, double.PositiveInfinity));
            return (glyph ? measured.DesiredSize.Width : width, measured.DesiredSize.Height);
        }
        finally
        {
            measured.Width = keptWidth;
            measured.Height = keptHeight;
        }
    }

    /// <summary>
    /// Each control moved to where its widget now is and showing what its field now holds; false,
    /// and nothing moved, where <paramref name="wanted"/> is not the set built, which is a
    /// QUORRA_OWED_CONTROLS still to come.
    /// </summary>
    internal bool Place(IReadOnlyList<Placed> wanted, double scale)
    {
        if (wanted.Count != fields.Count || scale <= 0)
        {
            return false;
        }
        for (int index = 0; index < wanted.Count; index++)
        {
            fields[index] = wanted[index];
            if (elements[index] is not FrameworkElement element)
            {
                continue;
            }
            float[] quad = wanted[index].Quad;
            double left = Math.Min(Math.Min(quad[0], quad[2]), Math.Min(quad[4], quad[6]));
            double right = Math.Max(Math.Max(quad[0], quad[2]), Math.Max(quad[4], quad[6]));
            double top = Math.Min(Math.Min(quad[1], quad[3]), Math.Min(quad[5], quad[7]));
            double bottom = Math.Max(Math.Max(quad[1], quad[3]), Math.Max(quad[5], quad[7]));
            Canvas.SetLeft(element, left / scale);
            Canvas.SetTop(element, top / scale);
            element.Width = (right - left) / scale;
            element.Height = (bottom - top) / scale;
            Show(index, element, wanted[index]);
        }
        return true;
    }

    // What the field holds, written into its control where the control shows something else.
    private void Show(int index, FrameworkElement element, Placed field)
    {
        writing = true;
        try
        {
            Write(index, element, field);
        }
        finally
        {
            writing = false;
        }
    }

    private void Write(int index, FrameworkElement element, Placed field)
    {
        switch (element)
        {
            case TextBox box:
                WriteText(index, box, field);
                break;
            case CombBox comb:
                WriteText(index, comb.Box, field);
                comb.ShowCells();
                break;
            // Table 231 bit 14's field answers with its echo, which written back would be sent as
            // the next value, so a password control is never written to (ADR 0244 finding 3).
            case PasswordBox:
                break;
            case CheckBox check:
                check.IsChecked = field.On;
                break;
            case RadioButton radio:
                radio.IsChecked = field.On;
                break;
            case ComboBox combo:
                int first = FirstChosen(field.Selected);
                if (combo.SelectedIndex != first)
                {
                    combo.SelectedIndex = first;
                }
                // Table 233 bit 19 lets a person type "a value other than the predefined choices",
                // and such a value is the box's text rather than an option.
                if (combo.IsEditable && first < 0 && combo.FocusState == FocusState.Unfocused)
                {
                    combo.Text = field.Displayed ?? field.Value ?? string.Empty;
                }
                break;
            case ListBox list:
                for (int option = 0; option < list.Items.Count && option < field.Selected.Count; option++)
                {
                    if (list.Items[option] is ListBoxItem item && item.IsSelected != field.Selected[option])
                    {
                        item.IsSelected = field.Selected[option];
                    }
                }
                break;
        }
    }

    // A control holding the keyboard shows the field's characters, so typing starts from what the
    // format was made of; one that does not shows what Table 199's /F displays, which is what the
    // page shows under it everywhere else (ADR 1604).
    private void WriteText(int index, TextBox box, Placed field)
    {
        bool holds = box.FocusState != FocusState.Unfocused;
        string shows = (holds ? field.Value : field.Displayed) ?? string.Empty;
        if (box.Text != shows && written[index] is Written mine)
        {
            mine.Text = shows;
            box.Text = shows;
            box.SelectionStart = shows.Length;
        }
    }

    // The platform's control for one widget, or null where the page draws the widget itself.
    private FrameworkElement? Make(int index, Placed field)
    {
        bool readOnly = Has(field, Native.QUORRA_FIELD_READ_ONLY);
        switch (field.Kind)
        {
            case Native.QUORRA_CONTROL_ENTRY when Has(field, Native.QUORRA_FIELD_PASSWORD):
                return PasswordControl(field);
            // ISO 32000-2 §12.7.5.3, Table 231 bit 25, may be set "only if the MaxLen entry is
            // present ... and if the Multiline, Password, and FileSelect flags are clear"; a field
            // that breaks the rule keeps its plain control (a password field took its own above).
            case Native.QUORRA_CONTROL_ENTRY when Has(field, Native.QUORRA_FIELD_COMB)
                && field.Cells > 0
                && !Has(field, Native.QUORRA_FIELD_MULTILINE)
                && !Has(field, Native.QUORRA_FIELD_FILE_SELECT):
                return new CombBox(TextControl(index, field, readOnly), field.Cells);
            case Native.QUORRA_CONTROL_ENTRY:
                return TextControl(index, field, readOnly);
            case Native.QUORRA_CONTROL_CHECK:
                var check = new CheckBox { IsThreeState = false, IsEnabled = !readOnly };
                check.Click += (_, _) => commits.Toggled(Current(field));
                return Fitted(check);
            case Native.QUORRA_CONTROL_RADIO:
                // XAML puts every radio button of one parent into one group, which is not
                // §12.7.5.2.4's: a PDF's set is the widgets of one field, and Table 229 bit 15
                // decides what a second click does. So each button is a group of its own and the
                // clause's rule is the library's, as it is in the toolkit windows.
                var radio = new RadioButton
                {
                    GroupName = $"{field.Number} {field.Generation}",
                    IsEnabled = !readOnly,
                };
                radio.Click += (_, _) => commits.Toggled(Current(field));
                return Fitted(radio);
            case Native.QUORRA_CONTROL_COMBO:
                return ComboControl(field, readOnly);
            case Native.QUORRA_CONTROL_LIST:
                return ListControl(field, readOnly);
            default:
                return null;
        }
    }

    // Table 231 bit 26's rich text field is shown by its plain value, as every toolkit window shows
    // it: what /RV's formatting would change is the widget's appearance, which a delegated control
    // stands in for, and no window of this tree draws it yet.
    private TextBox TextControl(int index, Placed field, bool readOnly)
    {
        // Table 231 bit 13: "the field may contain multiple lines of text"; without it Enter is
        // the commit rather than a character.
        bool multiline = Has(field, Native.QUORRA_FIELD_MULTILINE);
        var box = new TextBox
        {
            AcceptsReturn = multiline,
            TextWrapping = multiline ? TextWrapping.Wrap : TextWrapping.NoWrap,
            IsReadOnly = readOnly,
            IsSpellCheckEnabled = !Has(field, Native.QUORRA_FIELD_DO_NOT_SPELL_CHECK),
            MaxLength = Length(field.MaxLength),
        };
        var mine = new Written();
        written[index] = mine;
        box.TextChanged += (_, _) =>
        {
            if (box.Text != mine.Text)
            {
                mine.Text = box.Text;
                commits.Typed(Current(field), box.Text);
            }
        };
        // The keyboard leaving a single-line field is Table 197's /Bl, and Enter gives the keyboard
        // back to the page, so the two end in one commit; a multi-line field's text is typed as it
        // stands, as it is in viewer-qt.
        if (!multiline)
        {
            box.LostFocus += (_, _) => commits.Finished(Current(field));
            box.KeyDown += (_, pressed) => ToPageOnEnter(pressed);
        }
        return Fitted(box);
    }

    // Table 231 bit 14: "a secure password that should not be echoed visibly to the screen", which
    // is the platform's own secure entry.
    private PasswordBox PasswordControl(Placed field)
    {
        var box = new PasswordBox
        {
            MaxLength = Length(field.MaxLength),
            IsEnabled = !Has(field, Native.QUORRA_FIELD_READ_ONLY),
        };
        box.PasswordChanged += (_, _) => commits.Typed(Current(field), box.Password);
        box.LostFocus += (_, _) => commits.Finished(Current(field));
        box.KeyDown += (_, pressed) => ToPageOnEnter(pressed);
        return Fitted(box);
    }

    private ComboBox ComboControl(Placed field, bool readOnly)
    {
        // Table 233 bit 19: if set, "the combo box shall include an editable text box as well as
        // a drop-down list", and only the list where it is clear.
        var combo = new ComboBox
        {
            IsEditable = Has(field, Native.QUORRA_FIELD_EDITABLE),
            IsEnabled = !readOnly,
        };
        foreach (string label in field.Options)
        {
            combo.Items.Add(new ComboBoxItem { Content = label });
        }
        combo.SelectionChanged += (_, _) =>
        {
            if (writing)
            {
                return;
            }
            int chosen = combo.SelectedIndex;
            // The field's own state written back is not a choice.
            Placed now = Current(field);
            if (chosen >= 0 && chosen != FirstChosen(now.Selected))
            {
                commits.Chose(now, new[] { (nuint)chosen });
            }
        };
        // What is typed into the box is characters held until it is submitted, and the field's
        // Table 199 /K in its commit form and /V wait for that moment.
        combo.TextSubmitted += (_, submitted) =>
        {
            submitted.Handled = true;
            commits.Typed(Current(field), submitted.Text);
            commits.Finished(Current(field));
        };
        return Fitted(combo);
    }

    private ListBox ListControl(Placed field, bool readOnly)
    {
        // Table 233 bit 22: "more than one of the field's option items may be selected
        // simultaneously".
        var list = new ListBox
        {
            SelectionMode = Has(field, Native.QUORRA_FIELD_MULTI_SELECT)
                ? SelectionMode.Multiple
                : SelectionMode.Single,
            IsEnabled = !readOnly,
        };
        foreach (string label in field.Options)
        {
            list.Items.Add(new ListBoxItem { Content = label });
        }
        list.SelectionChanged += (_, _) =>
        {
            if (writing)
            {
                return;
            }
            Placed now = Current(field);
            var chosen = new List<nuint>();
            bool same = true;
            for (int option = 0; option < list.Items.Count; option++)
            {
                bool selected = list.Items[option] is ListBoxItem { IsSelected: true };
                if (selected)
                {
                    chosen.Add((nuint)option);
                }
                same &= option < now.Selected.Count && now.Selected[option] == selected;
            }
            // The field's own state written back is not a choice.
            if (!same)
            {
                commits.Chose(now, chosen);
            }
        };
        return Fitted(list);
    }

    private void ToPageOnEnter(KeyRoutedEventArgs pressed)
    {
        if (pressed.Key == VirtualKey.Enter)
        {
            pressed.Handled = true;
            toPage();
        }
    }

    // A control sized by the widget's rectangle and not by its style's minimum, which can be
    // larger than a document's /Rect: the rectangle is the document's own (§12.5.2), and a control
    // that grew past it would cover the page around it.
    private static T Fitted<T>(T control)
        where T : Control
    {
        control.MinWidth = 0;
        control.MinHeight = 0;
        control.Padding = new Thickness(2, 0, 2, 0);
        control.VerticalContentAlignment = VerticalAlignment.Center;
        return control;
    }

    private static bool Has(Placed field, uint flag) => (field.Flags & flag) != 0;

    // The record the window now holds for the widget a control was made for. A control's event can
    // arrive after the set was read again — WinUI raises a lost focus after the call that moved
    // it — and a widget no longer on the page is committed as it was made, so the window's refusal
    // is said rather than the commit dropped.
    private Placed Current(Placed made)
    {
        foreach (Placed field in fields)
        {
            if (field.Number == made.Number && field.Generation == made.Generation)
            {
                return field;
            }
        }
        return made;
    }

    // §12.7.5.3, Table 231 bit 25: "the field shall be automatically divided into as many equally
    // spaced positions, or combs, as the value of MaxLen, and the text is laid out into those
    // combs". The page draws no widget under a delegated control, so the combs are this control's:
    // a cell for each position with a character centred in each, shown while the text box does not
    // hold the keyboard; while it does, the box's own text, which is what a person types into.
    private sealed class CombBox : Grid
    {
        private readonly TextBlock[] cells;
        private readonly Grid layout = new() { IsHitTestVisible = false };

        internal CombBox(TextBox box, uint count)
        {
            Box = box;
            cells = new TextBlock[checked((int)count)];
            Brush? rule = Application.Current.Resources.TryGetValue("TextControlForeground", out object? found)
                ? found as Brush
                : null;
            for (int at = 0; at < cells.Length; at++)
            {
                layout.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
                cells[at] = new TextBlock
                {
                    HorizontalAlignment = HorizontalAlignment.Center,
                    VerticalAlignment = VerticalAlignment.Center,
                };
                // Equally spaced positions, a rule between each and the next.
                var cell = new Border
                {
                    BorderBrush = rule,
                    BorderThickness = new Thickness(at == 0 ? 0 : 1, 0, 0, 0),
                    Child = cells[at],
                };
                Grid.SetColumn(cell, at);
                layout.Children.Add(cell);
            }
            var outline = new Border { BorderBrush = rule, BorderThickness = new Thickness(1), IsHitTestVisible = false };
            Children.Add(box);
            Children.Add(layout);
            Children.Add(outline);
            box.GotFocus += (_, _) => ShowCells();
            box.LostFocus += (_, _) => ShowCells();
            box.TextChanged += (_, _) => ShowCells();
            ShowCells();
        }

        /// <summary>The text box a person types into.</summary>
        internal TextBox Box { get; }

        /// <summary>The cells drawn from the box's text, or the box itself while it holds the keyboard.</summary>
        internal void ShowCells()
        {
            bool holds = Box.FocusState != FocusState.Unfocused;
            Box.Opacity = holds ? 1 : 0;
            layout.Visibility = holds ? Visibility.Collapsed : Visibility.Visible;
            // A position holds one character as a reader sees one, which is a text element rather
            // than a UTF-16 unit.
            TextElementEnumerator characters = StringInfo.GetTextElementEnumerator(Box.Text);
            for (int at = 0; at < cells.Length; at++)
            {
                cells[at].Text = characters.MoveNext() ? characters.GetTextElement() : string.Empty;
            }
        }
    }

    // What this window last wrote into one text control.
    private sealed class Written
    {
        internal string? Text { get; set; }
    }

    // The first option chosen, or -1 where none is.
    private static int FirstChosen(IReadOnlyList<bool> selected)
    {
        for (int option = 0; option < selected.Count; option++)
        {
            if (selected[option])
            {
                return option;
            }
        }
        return -1;
    }

    // Table 232's /MaxLen as a XAML control's MaxLength, where zero is no limit for both.
    private static int Length(uint maxLength) => maxLength > int.MaxValue ? int.MaxValue : (int)maxLength;
}
