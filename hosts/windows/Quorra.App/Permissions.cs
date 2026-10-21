// Menu wording, hierarchy and checked state come from the same rows as the other hosts.
using System;
using System.Collections.Generic;
using Microsoft.UI.Xaml.Controls;
using Quorra.Interop;

namespace Quorra.App;

internal sealed record PermissionRow(nuint Index, uint Depth, bool Chosen, string Label, string Note);

internal sealed unsafe partial class QuorraWindow
{
    internal List<PermissionRow> Permissions()
    {
        var rows = new List<PermissionRow>();
        for (nuint index = 0; index < Native.quorra_window_restriction_count(handle); index++)
        {
            nuint at = index;
            uint depth = 0;
            byte chosen = 0;
            string Read(uint part) => Library.ReadText("quorra_window_restriction_read", (into, cap, needed) =>
            {
                uint nesting = 0;
                byte tick = 0;
                int status = Native.quorra_window_restriction_read(handle, at, part, &nesting, &tick, into, cap, needed);
                depth = nesting;
                chosen = tick;
                return status;
            });
            string label = Read(0);
            string note = Read(1);
            rows.Add(new PermissionRow(at, depth, chosen != 0, label, note));
        }
        return rows;
    }

    internal void ChoosePermission(nuint index) => Library.Check("quorra_window_restriction_choose",
        Native.quorra_window_restriction_choose(handle, index));
}

internal sealed class WindowPermissions
{
    private readonly Button button;
    private readonly Func<QuorraWindow?> window;
    private readonly Action<string, Action> guard;
    private readonly Action<string, Action<QuorraWindow>> commit;

    internal WindowPermissions(Button button, Func<QuorraWindow?> window,
        Action<string, Action> guard, Action<string, Action<QuorraWindow>> commit)
    {
        this.button = button;
        this.window = window;
        this.guard = guard;
        this.commit = commit;
        button.Click += (_, _) => Show();
    }

    internal void Show() => guard("permissions opened", () =>
    {
        if (window() is not QuorraWindow open) return;
        var menu = new MenuFlyout();
        MenuFlyoutSubItem? scope = null, operation = null;
        foreach (PermissionRow row in open.Permissions())
        {
            if (row.Depth < 2)
            {
                var heading = new MenuFlyoutSubItem { Text = row.Label };
                if (row.Note.Length != 0) ToolTipService.SetToolTip(heading, row.Note);
                if (row.Depth == 0) { menu.Items.Add(heading); scope = heading; operation = null; }
                else { scope?.Items.Add(heading); operation = heading; }
            }
            else
            {
                var item = new ToggleMenuFlyoutItem { Text = row.Label, IsChecked = row.Chosen };
                item.Click += (_, _) => commit("permission selected", current => current.ChoosePermission(row.Index));
                operation?.Items.Add(item);
            }
        }
        menu.ShowAt(button);
    });
}
