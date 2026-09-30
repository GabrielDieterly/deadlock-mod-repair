use clap::{Parser, Subcommand};
use deadlock_mod_repair::export::{PreparedMod, RepairReport};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
  name = "deadlock-mod-repair",
  bin_name = "deadlock-mod-repair",
  version,
  about = "Check and export repaired Deadlock mod VPKs"
)]
struct Cli {
  #[command(subcommand)]
  command: Command,
}

#[derive(Subcommand)]
enum Command {
  /// Check a mod and print the repairs that can be verified.
  Check {
    /// Mod VPK (for multipart mods, select the _dir.vpk).
    input: PathBuf,
    /// Deadlock installation, game folder, or game/citadel folder.
    #[arg(long)]
    game: PathBuf,
    /// Save the full JSON report without exporting a VPK.
    #[arg(long)]
    report: Option<PathBuf>,
  },
  /// Export a complete VPK with verified repairs applied.
  Repair {
    /// Mod VPK (for multipart mods, select the _dir.vpk).
    input: PathBuf,
    /// Deadlock installation, game folder, or game/citadel folder.
    #[arg(long)]
    game: PathBuf,
    /// New output VPK. Existing files and game folders are never overwritten.
    #[arg(long)]
    output: PathBuf,
    /// JSON report path (default: output filename with .repair.json).
    #[arg(long)]
    report: Option<PathBuf>,
  },
}

fn print_report(report: &RepairReport) {
  println!("{} verified resource changes", report.changed_files.len());
  for file in &report.changed_files {
    println!("  {}: {}", file.reason, file.path);
  }
  for warning in &report.analysis.asset_warnings {
    println!("  Warning: {}: {}", warning.file_path, warning.detail);
  }
  for skipped in &report.skipped_files {
    println!("  Left unchanged: {}: {}", skipped.path, skipped.reason);
  }
}

fn run(cli: Cli) -> Result<(), deadlock_mod_repair::errors::Error> {
  match cli.command {
    Command::Check {
      input,
      game,
      report,
    } => {
      let prepared = PreparedMod::analyze(&input, &game)?;
      print_report(prepared.report());
      if let Some(path) = report {
        prepared.write_report(&path)?;
        println!("Report: {}", path.display());
      }
      println!("Input VPK unchanged. No repaired VPK exported.");
    }
    Command::Repair {
      input,
      game,
      output,
      report,
    } => {
      let prepared = PreparedMod::analyze(&input, &game)?;
      let report_path = report.unwrap_or_else(|| output.with_extension("repair.json"));
      let result = prepared.export(&output, &report_path)?;
      print_report(&result);
      println!("Exported: {}", output.display());
      println!("Report: {}", report_path.display());
      println!("Original VPK unchanged. Test the exported mod in-game before publishing.");
    }
  }
  Ok(())
}

fn main() -> ExitCode {
  match run(Cli::parse()) {
    Ok(()) => ExitCode::SUCCESS,
    Err(error) => {
      eprintln!("Error: {error}");
      ExitCode::FAILURE
    }
  }
}
