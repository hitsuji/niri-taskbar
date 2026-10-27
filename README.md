# Niri Taskbar (for Waybar)

This provides a [Waybar][waybar] taskbar for [Niri][niri].

The main shift from the builtin `wlr/taskbar` module is that windows are always
ordered to match the Niri layout: by workspace index, then column, then tile
within the column. Floating windows are placed after the tiled windows on their
workspace.

Buttons can be dragged and dropped to reorder windows: dropping a tiled window
onto another button moves its whole column to that position (moving it to
another workspace if required), and dropping a floating window onto a button on
another workspace moves it to that workspace. Right clicking a button shows a
menu to [resize](#resize-presets) or close the window.

![Example screenshot](images/screenshot.png)

## Installation

If you package niri-taskbar for another distro/OS, please let me know and I'll
update the README.

### Arch Linux

@coreymwamba has kindly packaged this as [the `waybar-niri-taskbar` package in
the AUR][aur].

### From source

Users of other distributions and OSes will need to build from source.

At the moment, this needs to built from source.
distro, let me know and I'll update the README.)

### Requirements

- Rust 1.87.0 or later
- Niri (with a version corresponding to the version in the `niri-taskbar` crate
  version; eg `0.4.0+niri.26.4` is specifically for Niri 26.04)
- Gtk+ 3 (including the development package on distros that separate those out)
- Waybar 0.12.0 (or any version that's API compatible with 0.12, which will
  _probably_ include later versions, but I have no actual knowledge there)

### Building

The standard Rust build process should work fine:

```bash
$ cargo build --release
```

This will give you a shared library module at
`target/release/libniri_taskbar.so`. Feel free to move that wherever makes
sense.

## Configuration

This uses the normal configuration for a [CFFI Waybar module][cffi], which in
practice will look something like this:

```jsonc
{
  "modules_left": ["cffi/niri-taskbar"],
  // ...
  "cffi/niri-taskbar": {
    "module_path": "/your/path/to/libniri_taskbar.so",
  },
}
```

### Application highlighting

In addition to [notification support](#notifications), you can highlight
applications based on their app ID and title by configuring application rules in
the Waybar configuration.

For example, to highlight a Signal window that starts with `(1)` (or any other
number), indicating pending notifications, you could configure the taskbar like
so:

```jsonc
{
  "cffi/niri-taskbar": {
    // module_path
    "apps": {
      "signal": [
        {
          "match": "\\([0-9]+\\)$",
          "class": "unread",
        },
      ],
    },
  },
}
```

Each key within the `apps` object is a Wayland app ID, which can have one or
more rules set within it. Each rule must have a `match`, which is a regex that
will be matched against the window title, and a `class`, which is a CSS class
that will be added to the button element if the regex matches.

If more than one rule matches for a single app ID, all matching classes will be
added.

The easiest way to get the app ID for a window is to ask Niri with `niri msg
windows`. Note that app IDs are case sensitive.

### Multiple outputs

By default, the taskbar will only show applications running on the same output
as the taskbar itself. You can enable the `show_all_outputs` option to show all
applications on all outputs:

```jsonc
{
  "cffi/niri-taskbar": {
    // other settings
    "show_all_outputs": true,
  },
}
```

Note that multiple output support is currently experimental, and may have some
quirks. Please open an issue with your use case if it's not working as you
expect!

### Resize presets

Right clicking a button shows a menu of column widths for that window, followed
by an option to close it. The `resize_presets` option maps each menu label to a
proportion of the working area, between 0 (exclusive) and 1. Presets are shown
in order of proportion.

```jsonc
{
  "cffi/niri-taskbar": {
    // other settings
    "resize_presets": {
      "25%": 0.25,
      "50%": 0.5,
      "75%": 0.75,
      "Full": 1.0,
    },
  },
}
```

If `resize_presets` isn't set, the defaults match the `preset-column-widths` in
Niri's default configuration: `33%`, `50%`, and `66%`. Setting it to `{}`
leaves only the close option in the menu.

### Notifications

You can enable the `notifications` configuration option to have the taskbar
listen to notifications and attempt to highlight the app that sent the
notification.

Configuration wise:

```jsonc
{
  "cffi/niri-taskbar": {
    // other settings
    "notifications": true,
  },
}
```

Highlighted buttons will gain the `.urgent` CSS class. Default styling is
included, but can be overridden [as described below](#styling).

## Icons

Icons are looked up by the window's app ID, first as an icon name in the
current icon theme, then via a matching desktop entry. Desktop entries are
matched by file name, then by their `StartupWMClass` (case insensitively), which
picks up apps whose desktop files are named differently from their app ID, such
as snaps (`<snap>_<app>.desktop`). Desktop entries that give an absolute path
for their `Icon` are also supported.

If an app still shows the fallback icon, check that its app ID (from `niri msg
windows`) matches either its desktop file name or the `StartupWMClass` in it.

## Styling

The taskbar uses [the same Gtk styling mechanism as Waybar][style]. The top
level taskbar element is given the class `.niri-taskbar`, and contains `button`
elements within it. The only CSS class that is applied by default is the
`focused` class, which is added to the button for the currently focused window.

While dragging a button, the button under the pointer gains the `.drop-before`
or `.drop-after` class to show where the window will land.

The default styling assumes a dark background. It provides a basic hover
effect, highlights the focused window, and draws a white line on the edge of
the button where a dragged window will be dropped.

For a light background, something like this is likely good:

```css
.niri-taskbar button:hover {
  background: rgba(0, 0, 0, 0.5);
}

.niri-taskbar button.focused {
  background: rgba(0, 0, 0, 0.3);
}
```

If you apply custom CSS classes using application rules as described above,
then those can be styled in the same way. For instance, with the `unread` class
demonstrated above, you could add a border highlight like so:

```css
.niri-taskbar button {
  /* This is useful to prevent the icon being resized when there's no unread. */
  border-bottom: solid 3px transparent;
}

.niri-taskbar button.unread {
  border-bottom: solid 3px white;
}
```

[aur]: https://aur.archlinux.org/packages/waybar-niri-taskbar
[cffi]: https://github.com/Alexays/Waybar/wiki/Module:-CFFI
[niri]: https://github.com/YaLTeR/niri
[style]: https://github.com/Alexays/Waybar/wiki/Styling
[waybar]: https://github.com/Alexays/Waybar
