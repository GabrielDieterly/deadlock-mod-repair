use deadlock_mod_repair::export::PreparedMod;
use source2_model::vpk_extract::VpkArchive;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use vpkmanager::source2::kv3::{self, Value};

const TEXT: &str = "resource/localization/citadel_heroes/citadel_heroes_english.txt";
const SKELETON: &str = "models/heroes/unrelated/custom.vnmskel_c";

fn pack(root: &Path, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
  let directory = root.join(format!("{name}-files"));
  fs::create_dir_all(&directory).unwrap();
  for (relative, bytes) in files {
    let path = directory.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
  }
  let output = root.join(format!("{name}_dir.vpk"));
  vpkmanager::pack_directory(&directory, &output).unwrap();
  output
}

fn localization(tokens: &str) -> Vec<u8> {
  format!("\"lang\" {{ \"Language\" \"English\" \"Tokens\" {{ {tokens} }} }}").into_bytes()
}

fn setup() -> (TempDir, PathBuf, PathBuf) {
  let root = TempDir::new().unwrap();
  let citadel = root.path().join("Deadlock/game/citadel");
  fs::create_dir_all(&citadel).unwrap();
  let game = pack(
    root.path(),
    "game",
    &[(TEXT, &localization("\"base\" \"Current game\""))],
  );
  fs::copy(game, citadel.join("pak01_dir.vpk")).unwrap();
  fs::write(citadel.join("steam.inf"), "ClientVersion=1234\n").unwrap();
  let input = pack(
    root.path(),
    "author",
    &[
      (
        TEXT,
        &localization("\"base\" \"Authored name\" \"custom\" \"Authored line\""),
      ),
      ("materials/custom.vtex_c", b"authored texture\0\xff"),
      ("models/custom.vmdl_c", b"unrecognized model preserved"),
    ],
  );
  (root, input, citadel)
}

#[test]
fn export_keeps_every_unmodified_payload_and_leaves_inputs_untouched() {
  let (root, input, citadel) = setup();
  let before = fs::read(&input).unwrap();
  let game_before = fs::read(citadel.join("pak01_dir.vpk")).unwrap();
  let prepared = PreparedMod::analyze(&input, &citadel).unwrap();
  let output = root.path().join("fixed_dir.vpk");
  let report = root.path().join("fixed.json");
  let result = prepared.export(&output, &report).unwrap();
  let original = VpkArchive::open(&input).unwrap();
  let fixed = VpkArchive::open(&output).unwrap();
  assert_eq!(fixed.list_entries().len(), original.list_entries().len());
  for path in ["materials/custom.vtex_c", "models/custom.vmdl_c"] {
    assert_eq!(
      original.extract_entry(path).unwrap(),
      fixed.extract_entry(path).unwrap()
    );
  }
  let text = String::from_utf8(fixed.extract_entry(TEXT).unwrap()).unwrap();
  assert!(text.contains("Authored name") && text.contains("Authored line"));
  assert_eq!(fs::read(input).unwrap(), before);
  assert_eq!(
    fs::read(citadel.join("pak01_dir.vpk")).unwrap(),
    game_before
  );
  assert_eq!(result.game.client_version.as_deref(), Some("1234"));
  assert_eq!(result.unchanged_files, 2);
  assert!(result.output_sha256.is_some());
  assert!(report.exists());
  // An unrecognized model is reported and preserved rather than replaced.
  assert!(!result.analysis.asset_warnings.is_empty());
}

#[test]
fn existing_vpk_or_report_is_never_overwritten() {
  let (root, input, citadel) = setup();
  let prepared = PreparedMod::analyze(&input, &citadel).unwrap();
  let output = root.path().join("fixed.vpk");
  let report = root.path().join("fixed.json");
  fs::write(&report, b"existing report").unwrap();
  assert!(prepared.export(&output, &report).is_err());
  assert!(!output.exists());
  assert_eq!(fs::read(&report).unwrap(), b"existing report");
  fs::remove_file(&report).unwrap();
  fs::write(&output, b"existing VPK").unwrap();
  assert!(prepared.export(&output, &report).is_err());
  assert!(!report.exists());
  assert_eq!(fs::read(&output).unwrap(), b"existing VPK");
}

#[test]
fn input_and_game_installation_cannot_be_export_destinations() {
  let (root, input, citadel) = setup();
  let prepared = PreparedMod::analyze(&input, &citadel).unwrap();
  let report = root.path().join("report.json");
  assert!(prepared.export(&input, &report).is_err());
  assert!(
    prepared
      .export(&citadel.join("repaired.vpk"), &report)
      .is_err()
  );
  let game_output = citadel.parent().unwrap().join("repaired.vpk");
  assert!(prepared.export(&game_output, &report).is_err());
  assert!(!report.exists());
}

#[test]
fn mod_or_game_changes_after_analysis_abort_export() {
  let (root, input, citadel) = setup();
  let prepared = PreparedMod::analyze(&input, &citadel).unwrap();
  let output = root.path().join("fixed.vpk");
  let report = root.path().join("report.json");
  fs::write(citadel.join("steam.inf"), "ClientVersion=1235\n").unwrap();
  assert!(prepared.export(&output, &report).is_err());
  assert!(!output.exists() && !report.exists());
  fs::write(citadel.join("steam.inf"), "ClientVersion=1234\n").unwrap();
  fs::write(&input, b"changed input").unwrap();
  assert!(prepared.export(&output, &report).is_err());
  assert!(!output.exists() && !report.exists());
}

#[test]
fn check_command_writes_a_report_without_exporting_or_editing_a_vpk() {
  let (root, input, citadel) = setup();
  let before = fs::read(&input).unwrap();
  let result = Command::new(env!("CARGO_BIN_EXE_deadlock-mod-repair"))
    .args(["check"])
    .arg(&input)
    .arg("--game")
    .arg(citadel.parent().unwrap().parent().unwrap())
    .arg("--report")
    .arg(root.path().join("check.json"))
    .output()
    .unwrap();
  assert!(
    result.status.success(),
    "{}",
    String::from_utf8_lossy(&result.stderr)
  );
  assert!(String::from_utf8_lossy(&result.stdout).contains("No repaired VPK exported"));
  assert_eq!(fs::read(input).unwrap(), before);
  let report: serde_json::Value =
    serde_json::from_slice(&fs::read(root.path().join("check.json")).unwrap()).unwrap();
  assert!(report["output"].is_null());
}

#[test]
fn repair_command_exports_a_vpk_and_default_report() {
  let (root, input, citadel) = setup();
  let output = root.path().join("fixed_dir.vpk");
  let result = Command::new(env!("CARGO_BIN_EXE_deadlock-mod-repair"))
    .arg("repair")
    .arg(input)
    .arg("--game")
    .arg(citadel.parent().unwrap())
    .arg("--output")
    .arg(&output)
    .output()
    .unwrap();
  assert!(
    result.status.success(),
    "{}",
    String::from_utf8_lossy(&result.stderr)
  );
  assert!(VpkArchive::open(&output).is_ok());
  assert!(output.with_extension("repair.json").exists());
}

fn compiled(value: Value) -> Vec<u8> {
  let data = kv3::encode(&value, &kv3::Format([0; 16]));
  let mut bytes = vec![0; 32];
  bytes[..4].copy_from_slice(&u32::try_from(32 + data.len()).unwrap().to_le_bytes());
  bytes[4..6].copy_from_slice(&12u16.to_le_bytes());
  bytes[8..12].copy_from_slice(&8u32.to_le_bytes());
  bytes[12..16].copy_from_slice(&1u32.to_le_bytes());
  bytes[16..20].copy_from_slice(b"DATA");
  bytes[20..24].copy_from_slice(&12u32.to_le_bytes());
  bytes[24..28].copy_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
  bytes.extend(data);
  bytes
}

fn skeleton(reordered: bool) -> Vec<u8> {
  let pose = Value::Array(
    [0., 0., 0., 1., 0., 0., 0., 1.]
      .into_iter()
      .map(Value::Double)
      .collect(),
  );
  let names = if reordered {
    ["child", "root"]
  } else {
    ["root", "child"]
  };
  let parents = if reordered { [1, -1] } else { [-1, 0] };
  compiled(Value::Object(vec![
    ("m_ID".into(), Value::String("custom.vnmskel".into())),
    (
      "m_boneIDs".into(),
      Value::Array(
        names
          .into_iter()
          .map(|name| Value::String(name.into()))
          .collect(),
      ),
    ),
    (
      "m_parentIndices".into(),
      Value::Array(parents.into_iter().map(Value::Int).collect()),
    ),
    (
      "m_parentSpaceReferencePose".into(),
      Value::Array(vec![pose.clone(), pose.clone()]),
    ),
    (
      "m_modelSpaceReferencePose".into(),
      Value::Array(vec![pose.clone(), pose]),
    ),
    (
      "m_numBonesToSampleAtLowLOD".into(),
      Value::Int(if reordered { 1 } else { 2 }),
    ),
    ("m_maskDefinitions".into(), Value::Array(vec![])),
    ("m_secondarySkeletons".into(), Value::Array(vec![])),
  ]))
}

#[test]
fn animation_repair_is_embedded_in_complete_exported_vpk() {
  let (root, _, citadel) = setup();
  let current = skeleton(true);
  let game = pack(root.path(), "animation-game", &[(SKELETON, &current)]);
  fs::copy(game, citadel.join("pak01_dir.vpk")).unwrap();
  let old = skeleton(false);
  let input = pack(
    root.path(),
    "animation-author",
    &[
      (SKELETON, &old),
      ("materials/skin.vtex_c", b"custom material"),
    ],
  );
  let prepared = PreparedMod::analyze(&input, &citadel).unwrap();
  assert_eq!(prepared.report().analysis.asset_repairs.len(), 1);
  let output = root.path().join("fixed.vpk");
  prepared
    .export(&output, &root.path().join("report.json"))
    .unwrap();
  let archive = VpkArchive::open(&output).unwrap();
  assert_eq!(archive.extract_entry(SKELETON).unwrap(), current);
  assert_eq!(
    archive.extract_entry("materials/skin.vtex_c").unwrap(),
    b"custom material"
  );
  assert_eq!(
    VpkArchive::open(&input)
      .unwrap()
      .extract_entry(SKELETON)
      .unwrap(),
    old
  );
}

#[test]
fn enum_migrations_and_authored_game_data_are_embedded_in_export() {
  let (root, _, citadel) = setup();
  let field =
    |value: &str| Value::Object(vec![("m_eLosCheck".into(), Value::String(value.into()))]);
  let current = compiled(Value::Object(vec![("ability_aura".into(), field("Head"))]));
  let game = pack(
    root.path(),
    "enum-game",
    &[("scripts/abilities.vdata_c", &current)],
  );
  fs::copy(game, citadel.join("pak01_dir.vpk")).unwrap();
  let old = compiled(Value::Object(vec![
    ("ability_aura".into(), field("ELOSCheck_Head")),
    (
      "ability_custom".into(),
      Value::Object(vec![
        ("m_eLosCheck".into(), Value::String("ELOSCheck_Head".into())),
        ("customDamage".into(), Value::Int(77)),
      ]),
    ),
  ]));
  let input = pack(
    root.path(),
    "enum-author",
    &[("scripts/abilities.vdata_c", &old)],
  );
  let prepared = PreparedMod::analyze(&input, &citadel).unwrap();
  let output = root.path().join("fixed.vpk");
  prepared
    .export(&output, &root.path().join("report.json"))
    .unwrap();
  let archive = VpkArchive::open(&output).unwrap();
  let bytes = archive.extract_entry("scripts/abilities.vdata_c").unwrap();
  let resource = vpkmanager::source2::resource::Resource::parse(&bytes).unwrap();
  let data = kv3::decode(resource.data_block().unwrap()).unwrap();
  let custom = data.get("ability_custom").unwrap();
  assert_eq!(
    custom.get("m_eLosCheck"),
    Some(&Value::String("Head".into()))
  );
  assert_eq!(custom.get("customDamage"), Some(&Value::Int(77)));
  assert_eq!(
    VpkArchive::open(&input)
      .unwrap()
      .extract_entry("scripts/abilities.vdata_c")
      .unwrap(),
    old
  );
}

#[test]
fn preloaded_entries_are_rejected_before_export_instead_of_losing_bytes() {
  let (root, _, citadel) = setup();
  let input = root.path().join("preload_dir.vpk");
  // One valid v2 directory entry containing two preload bytes and no payload.
  let mut tree = b"txt\0 \0readme\0".to_vec();
  tree.extend(0u32.to_le_bytes());
  tree.extend(2u16.to_le_bytes());
  tree.extend(0x7fffu16.to_le_bytes());
  tree.extend(0u32.to_le_bytes());
  tree.extend(0u32.to_le_bytes());
  tree.extend(0xffffu16.to_le_bytes());
  tree.extend(b"hi\0\0\0");
  let mut bytes = Vec::new();
  bytes.extend(0x55aa1234u32.to_le_bytes());
  bytes.extend(2u32.to_le_bytes());
  bytes.extend(u32::try_from(tree.len()).unwrap().to_le_bytes());
  bytes.extend([0; 16]);
  bytes.extend(tree);
  fs::write(&input, &bytes).unwrap();
  let error = PreparedMod::analyze(&input, &citadel).err().unwrap();
  assert!(error.to_string().contains("Preloaded"));
  assert_eq!(fs::read(input).unwrap(), bytes);
}

#[test]
fn equivalent_game_data_stays_byte_identical_when_no_repairs_apply() {
  let (root, _, citadel) = setup();
  let input = pack(
    root.path(),
    "textures-only",
    &[("materials/custom.vtex_c", b"unchanged")],
  );
  let prepared = PreparedMod::analyze(&input, &citadel).unwrap();
  assert!(prepared.report().changed_files.is_empty());
  let output = root.path().join("unchanged.vpk");
  prepared
    .export(&output, &root.path().join("report.json"))
    .unwrap();
  assert_eq!(fs::read(&input).unwrap(), fs::read(&output).unwrap());
}

#[cfg(unix)]
#[test]
fn symlinked_output_parent_cannot_bypass_game_directory_guard() {
  let (root, input, citadel) = setup();
  let prepared = PreparedMod::analyze(&input, &citadel).unwrap();
  let link = root.path().join("linked-game");
  std::os::unix::fs::symlink(&citadel, &link).unwrap();
  assert!(
    prepared
      .export(&link.join("fixed.vpk"), &root.path().join("report.json"))
      .is_err()
  );
}
