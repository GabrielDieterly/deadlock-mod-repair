# Deadlock Mod Repair

Repair outdated Deadlock mods and export a new VPK. The tool compares your mod
with your installed game and fixes supported skeleton, camera, game-data, and
localization issues. It uses the repair code from
[Deadlock Mod Manager](https://github.com/GabrielDieterly/deadlock-mod-manager/tree/feature/localization-merge-poc).

[Download](https://github.com/GabrielDieterly/deadlock-mod-repair/releases/latest):
Windows ZIP or Linux tar.gz.

## Usage

Update Deadlock, extract the download, and open a terminal in that folder.
Replace the game path below with your installation.

Windows:

```powershell
.\deadlock-mod-repair.exe repair .\mod_dir.vpk --game "C:\Games\Deadlock" --output .\fixed_dir.vpk
```

Linux:

```sh
./deadlock-mod-repair repair ./mod_dir.vpk --game /path/to/Deadlock --output ./fixed_dir.vpk
```

The command creates `fixed_dir.vpk` and a `fixed_dir.repair.json` report listing
the changes and unresolved issues. Your original VPK stays unchanged. Choose a
new output filename outside the game folder; existing files won't be overwritten.

For a report without exporting, use `check` instead of `repair` and omit `--output`.
For multipart mods, pass the `_dir.vpk` and keep its numbered companions beside it.
VPK preload bytes aren't supported yet.

Unresolved resources stay unchanged. Test the exported mod in-game before uploading it.

## Build

Requires stable Rust and a C/C++ toolchain.

```sh
cargo build --release --locked
```

GPL-3.0. [License](LICENSE.md) · [Credits](THIRD-PARTY-NOTICES.md)
