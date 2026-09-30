# Deadlock Mod Repair

Deadlock updates can leave an otherwise good skin crushed, stuck, or crashing in
hero select. This tool takes the fixes we developed in
[Deadlock Mod Manager](https://github.com/GabrielDieterly/deadlock-mod-manager/tree/feature/localization-merge-poc)
and applies them to a copy of your mod, so you can test it and upload an update.

## Using it

[Download the Windows or Linux version](https://github.com/GabrielDieterly/deadlock-mod-repair/releases)
and extract it. You only need the executable, your mod VPK, and an up-to-date
Deadlock installation.

Open a terminal in the extracted folder and run:

**Windows (PowerShell)**

```powershell
.\deadlock-mod-repair.exe repair "C:\Mods\my_mod_dir.vpk" --game "C:\Program Files (x86)\Steam\steamapps\common\Deadlock" --output "C:\Mods\my_mod_fixed_dir.vpk"
```

**Linux**

```sh
./deadlock-mod-repair repair ./my_mod_dir.vpk \
  --game "$HOME/.local/share/Steam/steamapps/common/Deadlock" \
  --output ./my_mod_fixed_dir.vpk
```

Replace the paths with yours. Pick a new output filename outside the game folder.
The tool won't overwrite your original or an existing output file.

You'll get a complete repaired VPK and a `.repair.json` report showing what
changed and anything that still needs attention. The fixes are baked into that
copy; players can install it normally without the manager's compatibility overlay.

Test the repaired VPK on its own before uploading it. Check hero select, movement,
and abilities, and keep your original files for future game updates.

To see what would change without exporting a VPK, use `check` instead of `repair`
and leave off `--output`. Add `--report ./check.json` if you want to save the report.

## What can it fix?

- Outdated animation skeletons (`.vnmskel_c`), while keeping custom poses and mask
  weights where the game and mod data show they can be preserved.
- Missing camera metadata in compatible models, such as the camera freezing
  during Calico's Cat ability.
- Supported game-data changes, including old enum values that the game no longer accepts.
- Mod names and text that need to be carried onto the current game's localization files.

It compares your mod with your installed game. The rules aren't tied to specific
mods or authors. If a change can't be verified, the affected file stays as it was
and the report explains why. Models and textures outside the repairs keep their
original contents.

For a multipart mod, select the `_dir.vpk` and keep its numbered companion VPKs
beside it. The export will be a single VPK. Archives with preload bytes aren't
supported yet; the tool will stop and tell you rather than export incomplete files.

## Building it

With stable Rust and a C/C++ build toolchain installed:

```sh
cargo build --release --locked
cargo test --locked
```

The executable ends up in `target/release`. Windows builds need Visual Studio C++
Build Tools; Linux builds need GCC/G++.

The repair code is copied from a specific manager commit, recorded in
[UPSTREAM.json](UPSTREAM.json). To pull in newer rules from a local manager checkout:

```sh
python scripts/sync-upstream.py /path/to/deadlock-mod-manager
cargo test --locked
```

GPL-3.0. [License](LICENSE.md) · [Third-party credits](THIRD-PARTY-NOTICES.md)
