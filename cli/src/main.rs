use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use flart::codegen::{Kind, Options};
use flart::generate::{FileOutcome, Request, Status};
use flart::{Error, PartKind, Report, snapshot};
use serde_json::json;

#[derive(Parser)]
#[command(
    name = "flart",
    version,
    about = "Generate Dart design tokens from read-only Figma snapshots."
)]
struct Cli {
    /// Emit a versioned JSON envelope instead of text.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the read-only collector script for one snapshot part, ready for `use_figma`.
    Collector {
        /// styles, collections, remote, or variables:<collection id>.
        #[arg(long)]
        part: PartKind,
    },
    /// Merge collector parts into one snapshot file.
    Snapshot {
        #[command(subcommand)]
        command: SnapshotCommand,
    },
    /// Validate an assembled snapshot and print its summary.
    Check {
        /// Snapshot file written by `flart snapshot assemble`.
        snapshot: PathBuf,
    },
    /// Generate Dart files from a snapshot into <out>/<kind>.dart.
    Generate {
        /// Snapshot file written by `flart snapshot assemble`.
        snapshot: PathBuf,
        /// Directory to write the Dart files into.
        #[arg(long)]
        out: PathBuf,
        /// Kinds to generate: colors, text_styles, effects, variables. Defaults to all.
        #[arg(long, value_delimiter = ',')]
        only: Vec<Kind>,
        /// Emit ThemeExtension classes for text styles and multi-mode collections.
        #[arg(long)]
        theme_extensions: bool,
        /// Include fontFamily in text styles.
        #[arg(long)]
        font_name: bool,
        /// Mode names to generate, as the plugin's mode dialog selected them.
        #[arg(long, value_delimiter = ',')]
        modes: Vec<String>,
        /// Report what would change without writing.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum SnapshotCommand {
    /// Verify each part's checksum, merge the parts, check aliases, and write the snapshot.
    Assemble {
        /// Part files saved from the collector output.
        #[arg(required = true)]
        parts: Vec<PathBuf>,
        /// Snapshot file to write.
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) if error.use_stderr() && std::env::args_os().any(|arg| arg == "--json") => {
            let envelope = json!({
                "apiVersion": 1,
                "type": "error",
                "data": { "kind": "usage", "code": 2, "message": error.to_string().trim() }
            });
            let _ = emit(&envelope.to_string());
            return ExitCode::from(2);
        }
        Err(error) => error.exit(),
    };
    let json = cli.json;
    match run(cli) {
        Ok(output) => match emit(&output) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: cannot write output: {error}");
                ExitCode::from(6)
            }
        },
        Err(error) => report_error(&error, json),
    }
}

fn emit(text: &str) -> std::io::Result<()> {
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{text}")?;
    stdout.flush()
}

fn run(cli: Cli) -> anyhow::Result<String> {
    match cli.command {
        Command::Collector { part } => {
            let script = flart::collector_script(&part);
            if cli.json {
                let data = json!({ "part": part.to_string(), "code": script });
                return Ok(
                    json!({ "apiVersion": 1, "type": "collector", "data": data }).to_string(),
                );
            }
            Ok(script.trim_end().to_string())
        }
        Command::Snapshot {
            command: SnapshotCommand::Assemble { parts, output },
        } => {
            let assembled = snapshot::assemble(&parts)?;
            let report = snapshot::check(&assembled.snapshot)?;
            snapshot::write(&output, &assembled.raw)?;
            Ok(format_report(&report, Some(&output), cli.json))
        }
        Command::Check { snapshot: path } => {
            let report = snapshot::check(&snapshot::load(&path)?)?;
            Ok(format_report(&report, None, cli.json))
        }
        Command::Generate {
            snapshot: path,
            out,
            only,
            theme_extensions,
            font_name,
            modes,
            dry_run,
        } => {
            let loaded = snapshot::load(&path)?;
            snapshot::check(&loaded)?;
            let request = Request {
                kinds: if only.is_empty() {
                    Kind::ALL.to_vec()
                } else {
                    only
                },
                options: Options {
                    theme_extensions,
                    include_font_name: font_name,
                },
                modes,
            };
            let files = flart::generate::generate(&loaded, &request, &out, dry_run)?;
            Ok(format_files(&files, dry_run, cli.json))
        }
    }
}

fn format_files(files: &[FileOutcome], dry_run: bool, json: bool) -> String {
    if json {
        let data = json!({ "dryRun": dry_run, "files": files });
        return json!({ "apiVersion": 1, "type": "generate", "data": data }).to_string();
    }
    files
        .iter()
        .map(|file| {
            let verb = match (file.status, dry_run) {
                (Status::Created, false) => "created",
                (Status::Created, true) => "would create",
                (Status::Changed, false) => "changed",
                (Status::Changed, true) => "would change",
                (Status::Unchanged, _) => "unchanged",
                (Status::Skipped, _) => "skipped (nothing of this kind in the snapshot)",
            };
            let modes = if file.modes.is_empty() {
                String::new()
            } else {
                format!(" [{}]", file.modes.join(", "))
            };
            format!("{verb} {}{modes}", file.path.display())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_report(report: &Report, written: Option<&PathBuf>, json: bool) -> String {
    if json {
        let mut data = json!({ "report": report });
        if let Some(path) = written {
            data["path"] = json!(path.display().to_string());
        }
        return json!({ "apiVersion": 1, "type": "report", "data": data }).to_string();
    }
    let mut lines = Vec::new();
    if let Some(path) = written {
        lines.push(format!("wrote {}", path.display()));
    }
    lines.push(format!(
        "{} collections, {} variables ({} aliases), {} remote variables",
        report.collections, report.variables, report.aliases, report.remote_variables
    ));
    lines.push(format!(
        "{} text styles, {} paint styles, {} effect styles",
        report.text_styles, report.paint_styles, report.effect_styles
    ));
    lines.join("\n")
}

fn report_error(error: &anyhow::Error, json: bool) -> ExitCode {
    let typed = error.downcast_ref::<Error>();
    let code = typed.map_or(1, Error::exit_code);
    if json {
        let kind = typed.map_or("error", Error::kind);
        let envelope = json!({
            "apiVersion": 1,
            "type": "error",
            "data": { "kind": kind, "code": code, "message": format!("{error:#}") }
        });
        let _ = emit(&envelope.to_string());
    } else {
        eprintln!("error: {error:#}");
    }
    ExitCode::from(code)
}
