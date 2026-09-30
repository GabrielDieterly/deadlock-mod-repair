# Deadlock Mod Repair

A small, offline command-line tool for mod authors. Check a mod against your
installed Deadlock build and export a **complete repaired VPK**, ready to test
and distribute.

The tool uses the compatibility rules developed and tested in
[Deadlock Mod Manager](https://github.com/GabrielDieterly/deadlock-mod-manager/tree/feature/localization-merge-poc).
It requires no manager installation, AI agent, Steam login, or network connection
to repair a mod.

## Download

Download the Windows or Linux archive from
[Releases](https://github.com/GabrielDieterly/deadlock-mod-repair/releases).
Extract it and open a terminal in that folder. The ZIP/TAR contains the executable
and license notices. You do not need Rust to use a release executable.

## Repair a mod

Update your local Deadlock installation first. Select your original mod VPK and
choose a new output filename **outside the game installation**.

Windows PowerShell:

```powershell
.\deadlock-mod-repair.exe repair "C:\Mods\my_mod_dir.vpk" --game "C:\Program Files (x86)\Steam\steamapps\common\Deadlock" --output "C:\Mods\my_mod_fixed_dir.vpk"
```

Linux:

```sh
./deadlock-mod-repair repair ./my_mod_dir.vpk \
  --game "$HOME/.local/share/Steam/steamapps/common/Deadlock" \
  --output ./my_mod_fixed_dir.vpk
```

This exports:

- `my_mod_fixed_dir.vpk`: the entire mod with verified repairs embedded.
- `my_mod_fixed_dir.repair.json`: changed resources, warnings, input/output hashes,
  installed game version, and the repair engine revision.

The original VPK stays unchanged. Existing output files are never overwritten.
Install the exported VPK by itself and test the portrait preview, movement, and
abilities in-game before publishing it. Do not enable the original and repaired
copies together during testing.

These repairs are permanent **in the exported copy**. They do not need a runtime
overlay or the mod manager. A future game update can require another repair, so
keep your original authoring files.

## Check without exporting

```sh
./deadlock-mod-repair check ./my_mod_dir.vpk --game /path/to/Deadlock
```

Add `--report ./check.json` to save the findings. `check` does not export a VPK.
The game argument accepts the Deadlock installation, its `game` folder, or its
`game/citadel` folder.

For multipart VPKs, select the `_dir.vpk` and keep its `_000.vpk`, `_001.vpk`, etc.
beside it. The export is one self-contained VPK.

## What it repairs

- **Animation skeletons (`.vnmskel_c`):** update outdated sampled bone mappings
  against the installed game. Where verified, preserve custom reference poses
  and mask weights. "Sampling layout" means bone order, parent indices, reference
  poses, and masks; it does not mean changing the animation clips.
- **Camera interfaces (`.vmdl_c`):** restore missing camera metadata and points
  when compatible rigs provide sufficient evidence, retaining custom geometry.
- **Shared game data (`.vdata_c`):** rebuild supported tables against the current
  game, preserving detected mod edits and applying verified enum migrations.
- **Localization:** carry authored names and text onto the current game's table.

Rules use resource structure and the installed game's corresponding resource,
rather than author names, mod IDs, or a list of specific mods. Unknown skeleton
differences remain unchanged and appear as warnings. Ambiguous localization
snapshots, parse warnings, or conflicting changes leave the affected file
unchanged. A successful export verifies the archive; it does not replace an
in-game test or guarantee every possible mod issue is fixed.

The output contains every original resource. The exporter reopens the packed
VPK and verifies every payload against the staged copy. Resources outside the
selected repairs keep their original payload bytes. Inputs with preloaded VPK
entries are currently rejected rather than risking incomplete extraction.

## Build from source

Install stable Rust and a native C/C++ build toolchain. On Windows use the MSVC
Rust target with Visual Studio C++ Build Tools. On Linux install GCC/G++.

```sh
cargo build --release --locked
cargo test --locked
```

The executable is in `target/release`. There are no Tauri, React, GTK, or WebView
dependencies. GitHub Actions tests and builds Windows and Linux executables.

## Maintaining the repair rules

The engine, history fingerprints, VPK reader, and VPK writer are vendored from a
fixed manager commit. [UPSTREAM.json](UPSTREAM.json) records each upstream source
path and SHA-256 hash. [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) identifies
the upstream projects and preserves their licenses.

To refresh that snapshot from a checked-out manager repository:

```sh
python scripts/sync-upstream.py /path/to/deadlock-mod-manager
cargo test --locked
```

Run the exporter tests and compare real repaired resources with the manager's
verified output before releasing refreshed rules. Changes to the upstream
engine's input/output API may require updates to `src/export.rs`.

GPL-3.0. See [LICENSE.md](LICENSE.md).
