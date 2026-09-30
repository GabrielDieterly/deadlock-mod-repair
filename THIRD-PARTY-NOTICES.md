# Third-party notices

This project is distributed under GPL-3.0; see [LICENSE.md](LICENSE.md).

The compatibility engine and history fingerprint indexes in `src/mod_manager`
and `assets`, and the crates in `vendor`, originate from
[Deadlock Mod Manager](https://github.com/deadlock-mod-manager/deadlock-mod-manager).
The exact source commit and per-file hashes are recorded in
[UPSTREAM.json](UPSTREAM.json). Only fingerprint indexes are bundled; no mod VPKs
or game resource payloads are distributed with the tool.

## vpkmerge (morphic)

- Upstream: https://github.com/Slush97/vpkmerge
- License: MIT, copyright 2026 esoc.
- Original vendored revision: `142373c2cfcac522624519dcb64e616c8d58998e`.
- Scope: `vendor/vpkmanager/src/source2`; related audio/pattern adaptations in
  `vendor/vpkmanager/src/audio.rs` and `pattern.rs`.
- License text: [LICENSE-vpkmerge](vendor/vpkmanager/LICENSE-vpkmerge).

## ValveResourceFormat

- Upstream: https://github.com/ValveResourceFormat/ValveResourceFormat
- License: MIT, copyright 2015 ValveResourceFormat Contributors.
- Scope: Source 2 codecs in `vendor/vpkmanager/src/source2` (via vpkmerge), and
  model decoding behavior in `vendor/source2-model`.
- License texts: [VPK writer notice](vendor/vpkmanager/LICENSE-ValveResourceFormat)
  and [model reader notice](vendor/source2-model/LICENSE-ValveResourceFormat).

Crates.io dependencies retain their respective licenses. The vendored engine
and crates are copied unchanged; the CLI, exporter, and distribution workflows
are new. The compatibility snapshot can be updated with `scripts/sync-upstream.py`.
