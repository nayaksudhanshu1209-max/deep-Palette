# Palette

A Windows 10/11 keyboard-first command launcher. Single native .exe.
No background service, no auto-start, no network calls, no elevation.

## Requirements

- Windows 10 or 11 (x64)
- WebView2 Evergreen runtime for the built-in browser only (usually already
  present on Win11 and updated Win10)

## Install

Download `Palette-x64-portable.zip` from the GitHub Actions artifacts,
extract it anywhere, and run `palette.exe`. There is no installer, no
service, and no auto-start shortcut — launch it when you want it.

## Usage

- **Ctrl+Space** opens the palette.
- Type to search apps, commands, quick folders, indexed files, and Windows
  settings.
- Type a bang trigger followed by a space to convert it to a chip:
  `g `, `yt `, `gh `, `so `, `wiki ` open the matching site in your default
  browser. `w ` opens the built-in browser. `note ` opens the note editor.
  `settings ` opens this settings panel.
- `Enter` runs the selected result. `Esc` closes.
- Right-click or **Ctrl+H** on any result to hide it (adds to the blocklist).

## Hotkey conflict

`Ctrl+Space` is the default IME toggle on many CJK machines. If registration
fails, Palette automatically uses `Ctrl+Shift+Space` and shows a notice in
Settings → General. You can also rebind in Settings.

## Settings

Open the settings panel by typing `settings ` and pressing Enter, or by
searching for "Settings" in the palette. All changes save immediately.

## Notes

Notes are stored as `.md` files under `%APPDATA%\Palette\notes` by default;
change the folder in Settings → Notes. Esc or Ctrl+W saves and closes.

## Browser

The built-in browser uses WebView2 with memory-tuned arguments and no
chrome. Esc or Ctrl+W closes it. Alt+Left/Right navigate. WebView2 must be
installed for this feature; everything else works without it.

## Submitting to Microsoft for AV review

If Windows Defender flags the build, upload the binary to
https://www.microsoft.com/en-us/wdsi/filesubmission and mark it as a
false positive. Include this README and note that the binary is
open-source, does not auto-start, uses no hooks or injection, and makes no
network calls.

## Building

GitHub Actions builds on push. To build locally: `cargo build --release`.
