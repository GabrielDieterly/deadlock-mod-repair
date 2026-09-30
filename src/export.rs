use crate::errors::Error;
use crate::mod_manager::localization_overlay::asset_compatibility::RepairKind;
use crate::mod_manager::localization_overlay::{
  LocalizationModInput, LocalizationOverlayAnalysis, LocalizationOverlayPlan,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use source2_model::vpk_extract::VpkArchive;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tempfile::{NamedTempFile, TempDir};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedFile {
  pub path: String,
  pub reason: String,
  pub before_sha256: String,
  pub after_sha256: String,
}

#[derive(Clone, Serialize)]
pub struct SkippedFile {
  pub path: String,
  pub reason: String,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameReference {
  pub citadel: PathBuf,
  pub directory_sha256: String,
  pub version_sha256: Option<String>,
  pub client_version: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairReport {
  pub tool_version: String,
  pub engine_revision: String,
  pub input: PathBuf,
  pub input_sha256: String,
  pub game: GameReference,
  pub output: Option<PathBuf>,
  pub output_sha256: Option<String>,
  pub original_files: usize,
  pub unchanged_files: usize,
  pub changed_files: Vec<ChangedFile>,
  pub skipped_files: Vec<SkippedFile>,
  pub analysis: LocalizationOverlayAnalysis,
}

pub struct PreparedMod {
  workspace: TempDir,
  report: RepairReport,
  entries: BTreeMap<String, String>,
}

fn invalid(message: impl Into<String>) -> Error {
  Error::InvalidInput(message.into())
}

fn hash_bytes(bytes: &[u8]) -> String {
  format!("{:x}", Sha256::digest(bytes))
}

fn hash_file(path: &Path) -> Result<String, Error> {
  let mut file = fs::File::open(path)?;
  let mut hash = Sha256::new();
  let mut buffer = [0; 64 * 1024];
  loop {
    let count = file.read(&mut buffer)?;
    if count == 0 {
      break;
    }
    hash.update(&buffer[..count]);
  }
  Ok(format!("{:x}", hash.finalize()))
}

fn entry_path(path: &str) -> Result<String, Error> {
  let normalized = path.replace('\\', "/").to_ascii_lowercase();
  if normalized
    .split('/')
    .any(|part| part.is_empty() || matches!(part, "." | "..") || part.contains(':'))
  {
    return Err(invalid(format!("Unsafe archive entry: {path}")));
  }
  Ok(normalized)
}

fn game_reference(citadel: &Path) -> Result<GameReference, Error> {
  let version_path = citadel.join("steam.inf");
  let version = version_path
    .is_file()
    .then(|| fs::read(&version_path))
    .transpose()?;
  let client_version = version.as_ref().and_then(|bytes| {
    String::from_utf8_lossy(bytes).lines().find_map(|line| {
      let (key, value) = line.split_once('=')?;
      key
        .trim()
        .eq_ignore_ascii_case("ClientVersion")
        .then(|| value.trim().to_owned())
    })
  });
  Ok(GameReference {
    citadel: citadel.to_path_buf(),
    directory_sha256: hash_file(&citadel.join("pak01_dir.vpk"))?,
    version_sha256: version.as_deref().map(hash_bytes),
    client_version,
  })
}

fn resolve_game(path: &Path) -> Result<PathBuf, Error> {
  for candidate in [
    path.join("game/citadel"),
    path.join("citadel"),
    path.to_path_buf(),
  ] {
    if candidate.join("pak01_dir.vpk").is_file() {
      return Ok(candidate.canonicalize()?);
    }
  }
  Err(invalid(format!(
    "No game/citadel/pak01_dir.vpk found under {}",
    path.display()
  )))
}

fn game_root(citadel: &Path) -> &Path {
  citadel
    .parent()
    .filter(|parent| parent.file_name().is_some_and(|name| name == "game"))
    .and_then(Path::parent)
    .unwrap_or(citadel)
}

fn new_destination(path: &Path, citadel: &Path) -> Result<PathBuf, Error> {
  let name = path
    .file_name()
    .ok_or_else(|| invalid("Output must name a new file"))?;
  let parent = path
    .parent()
    .filter(|parent| !parent.as_os_str().is_empty())
    .unwrap_or(Path::new("."));
  let destination = parent.canonicalize()?.join(name);
  if destination.starts_with(game_root(citadel)) {
    return Err(invalid(
      "Export outside the Deadlock installation; the tool never edits installed game files",
    ));
  }
  if destination.try_exists()? || destination.symlink_metadata().is_ok() {
    return Err(invalid(format!(
      "Output already exists: {}",
      destination.display()
    )));
  }
  Ok(destination)
}

fn write_entry(root: &Path, path: &str, bytes: &[u8]) -> Result<(), Error> {
  let output = root.join(path);
  if let Some(parent) = output.parent() {
    fs::create_dir_all(parent)?;
  }
  fs::write(output, bytes)?;
  Ok(())
}

impl PreparedMod {
  pub fn analyze(input: &Path, game: &Path) -> Result<Self, Error> {
    let input = input.canonicalize()?;
    let citadel = resolve_game(game)?;
    if input.parent() == Some(citadel.as_path())
      || input.parent() == Some(citadel.with_file_name("core").as_path())
    {
      return Err(invalid("Select a mod VPK, not a base-game archive"));
    }
    let input_sha256 = hash_file(&input)?;
    let reference = game_reference(&citadel)?;
    let archive = VpkArchive::open(&input)?;
    let workspace = TempDir::new()?;
    let files = workspace.path().join("files");
    fs::create_dir(&files)?;
    let mut entries = BTreeMap::new();
    for original in archive.list_entries() {
      let path = entry_path(&original)?;
      if archive.has_preload_bytes(&original) {
        return Err(invalid(format!(
          "Preloaded archive entry is not supported yet: {original}. No files were exported."
        )));
      }
      let bytes = archive.extract_entry(&original)?;
      if entries.insert(path.clone(), hash_bytes(&bytes)).is_some() {
        return Err(invalid(format!("Duplicate archive entry: {path}")));
      }
      write_entry(&files, &path, &bytes)?;
    }
    if entries.is_empty() {
      return Err(invalid("The input VPK has no files"));
    }
    let plan = LocalizationOverlayPlan::build(
      &citadel,
      &[LocalizationModInput {
        mod_id: input
          .file_name()
          .unwrap_or_default()
          .to_string_lossy()
          .into_owned(),
        vpks: vec![input.clone()],
      }],
    )?;
    let overlay = workspace.path().join("repairs_dir.vpk");
    let written = plan.write(&overlay, &[])?;
    let mut skipped_files = Vec::new();
    let mut blocked = BTreeSet::new();
    for warning in &plan.analysis.snapshot_warnings {
      blocked.insert(warning.file_path.to_ascii_lowercase());
      skipped_files.push(SkippedFile {
        path: warning.file_path.clone(),
        reason: "Ambiguous historical localization snapshot".into(),
      });
    }
    for warning in &plan.analysis.parse_warnings {
      blocked.insert(warning.file_path.to_ascii_lowercase());
      skipped_files.push(SkippedFile {
        path: warning.file_path.clone(),
        reason: "Localization contains lines the parser could not preserve".into(),
      });
    }
    for path in plan
      .analysis
      .conflicts
      .iter()
      .map(|conflict| &conflict.file_path)
      .chain(
        plan
          .analysis
          .compiled_data_conflicts
          .iter()
          .map(|conflict| &conflict.file_path),
      )
    {
      if blocked.insert(path.to_ascii_lowercase()) {
        skipped_files.push(SkippedFile {
          path: path.clone(),
          reason: "Conflicting changes require manual review".into(),
        });
      }
    }
    let mut changed_files = Vec::new();
    if written.has_overlay {
      let repairs = VpkArchive::open(&overlay)?;
      for original in repairs.list_entries() {
        let path = entry_path(&original)?;
        if blocked.contains(&path) {
          continue;
        }
        let Some(before_sha256) = entries.get(&path) else {
          return Err(invalid(format!(
            "Repair proposed an unexpected new resource: {path}"
          )));
        };
        let bytes = repairs.extract_entry(&original)?;
        let after_sha256 = hash_bytes(&bytes);
        if after_sha256 == *before_sha256 {
          continue;
        }
        let reason = plan
          .analysis
          .asset_repairs
          .iter()
          .find(|repair| repair.file_path == path)
          .map(|repair| match repair.kind {
            RepairKind::AnimationSkeleton => "Animation skeleton updated",
            RepairKind::AnimationSkeletonRebased => {
              "Animation skeleton updated; custom settings preserved"
            }
            RepairKind::CameraInterface => "Missing camera interface restored",
          })
          .unwrap_or_else(|| {
            if path.ends_with(".vdata_c") {
              "Game data rebuilt against the installed game (including verified enum migrations)"
            } else {
              "Localization rebuilt against the installed game"
            }
          })
          .to_owned();
        write_entry(&files, &path, &bytes)?;
        changed_files.push(ChangedFile {
          path,
          reason,
          before_sha256: before_sha256.clone(),
          after_sha256,
        });
      }
    }
    let upstream: serde_json::Value = serde_json::from_str(include_str!("../UPSTREAM.json"))?;
    let report = RepairReport {
      tool_version: env!("CARGO_PKG_VERSION").into(),
      engine_revision: upstream["revision"].as_str().unwrap_or_default().into(),
      input,
      input_sha256,
      game: reference,
      output: None,
      output_sha256: None,
      original_files: entries.len(),
      unchanged_files: entries.len() - changed_files.len(),
      changed_files,
      skipped_files,
      analysis: plan.analysis,
    };
    let prepared = Self {
      workspace,
      report,
      entries,
    };
    prepared.verify_sources()?;
    Ok(prepared)
  }

  pub fn report(&self) -> &RepairReport {
    &self.report
  }

  fn verify_sources(&self) -> Result<(), Error> {
    if hash_file(&self.report.input)? != self.report.input_sha256
      || game_reference(&self.report.game.citadel)? != self.report.game
    {
      return Err(invalid(
        "Mod or game changed during analysis; run the command again",
      ));
    }
    let original = VpkArchive::open(&self.report.input)?;
    for path in original.list_entries() {
      let normalized = entry_path(&path)?;
      if self.entries.get(&normalized) != Some(&hash_bytes(&original.extract_entry(&path)?)) {
        return Err(invalid(format!(
          "Mod archive payload changed during analysis: {path}"
        )));
      }
    }
    Ok(())
  }

  pub fn write_report(&self, path: &Path) -> Result<(), Error> {
    let destination = new_destination(path, &self.report.game.citadel)?;
    let mut file = NamedTempFile::new_in(destination.parent().unwrap())?;
    serde_json::to_writer_pretty(&mut file, &self.report)?;
    file.write_all(b"\n")?;
    file.as_file().sync_all()?;
    file
      .persist_noclobber(destination)
      .map_err(|error| Error::Io(error.error))?;
    Ok(())
  }

  pub fn export(&self, output: &Path, report_path: &Path) -> Result<RepairReport, Error> {
    let output = new_destination(output, &self.report.game.citadel)?;
    let report_path = new_destination(report_path, &self.report.game.citadel)?;
    if output == report_path {
      return Err(invalid("VPK and report must use different output paths"));
    }
    if !output
      .extension()
      .is_some_and(|extension| extension.eq_ignore_ascii_case("vpk"))
    {
      return Err(invalid("Output VPK must have a .vpk extension"));
    }
    self.verify_sources()?;
    let file = NamedTempFile::new_in(output.parent().unwrap())?;
    let files = self.workspace.path().join("files");
    let packed = vpkmanager::pack_directory(&files, file.path())?;
    if packed != self.entries.len() {
      return Err(invalid(
        "Packed archive file count does not match the original",
      ));
    }
    let exported = VpkArchive::open(file.path())?;
    if exported.list_entries().len() != self.entries.len() {
      return Err(invalid("Exported archive directory verification failed"));
    }
    for path in exported.list_entries() {
      if !self.entries.contains_key(&path)
        || hash_bytes(&exported.extract_entry(&path)?) != hash_file(&files.join(&path))?
      {
        return Err(invalid(format!(
          "Exported resource verification failed: {path}"
        )));
      }
    }
    self.verify_sources()?;
    let mut report = self.report.clone();
    report.output = Some(output.clone());
    report.output_sha256 = Some(hash_file(file.path())?);
    let mut report_file = NamedTempFile::new_in(report_path.parent().unwrap())?;
    serde_json::to_writer_pretty(&mut report_file, &report)?;
    report_file.write_all(b"\n")?;
    report_file.as_file().sync_all()?;
    file.as_file().sync_all()?;
    report_file
      .persist_noclobber(&report_path)
      .map_err(|error| Error::Io(error.error))?;
    if let Err(error) = file.persist_noclobber(&output) {
      fs::remove_file(&report_path)?;
      return Err(Error::Io(error.error));
    }
    Ok(report)
  }
}
