# AVaSt Installer

Sets up AVaSt for Antivirus Survivors 2003 Professional in a few clicks. It finds the game in your Steam libraries, installs the GDPatch mod loader and the latest AVaSt API and AVaSt Core, and tells you the Steam launch options to set when your system needs them.

## Download

| System | |
| --- | --- |
| Windows | [AVaSt-Installer-Windows.exe](https://github.com/avast-mod/installer/releases/latest/download/AVaSt-Installer-Windows.exe) |
| macOS (Apple Silicon) | [AVaSt-Installer-macOS.dmg](https://github.com/avast-mod/installer/releases/latest/download/AVaSt-Installer-macOS.dmg) |
| Linux | `curl -fsSL https://avast.malidev-xp-desktop.workers.dev/install.sh \| bash` |

The installers aren't code-signed yet:

- **Windows:** SmartScreen shows "Windows protected your PC". Click **More info**, then **Run anyway**.
- **macOS:** the first launch is blocked. Open **System Settings > Privacy & Security** and click **Open Anyway**.

## Building

```sh
cargo build --release
```

## Credits

The installer uses the fonts fs Tahoma 8px by ETHproductions ([CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/)), Roboto Condensed by The Roboto Project Authors and Michroma by The Michroma Project Authors (SIL Open Font License 1.1). Details and license texts are in [assets/fonts](assets/fonts/README.md).
