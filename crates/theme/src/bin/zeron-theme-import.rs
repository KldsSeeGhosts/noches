use std::fs;
use std::path::PathBuf;

use anyhow::{Context as _, Result, anyhow};
use clap::{Parser, ValueEnum};
use zeron_theme::Appearance;
use zeron_theme::vscode::{CompileOptions, ImportOptions, import_file};

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum Format {
    #[default]
    Vscode,
    T3,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AppearanceArg {
    Dark,
    Light,
}

impl From<AppearanceArg> for Appearance {
    fn from(value: AppearanceArg) -> Self {
        match value {
            AppearanceArg::Dark => Self::Dark,
            AppearanceArg::Light => Self::Light,
        }
    }
}

/// Convert VS Code JSON/JSONC or a T3 role file into a Zeron theme draft.
#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    report: PathBuf,
    #[arg(long, value_enum, default_value = "vscode")]
    format: Format,
    #[arg(long)]
    id: Option<String>,
    #[arg(long)]
    family_id: Option<String>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long, value_enum)]
    appearance: Option<AppearanceArg>,
    #[arg(long)]
    source_url: Option<String>,
    #[arg(long)]
    revision: Option<String>,
    #[arg(long)]
    license: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if matches!(args.format, Format::T3) {
        let family_id = args.family_id.unwrap_or_else(|| {
            let name = args
                .input
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("theme");
            format!(
                "t3-{}",
                name.chars()
                    .map(|c| if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    })
                    .collect::<String>()
            )
        });
        let imported = zeron_theme::t3::import_file(
            &args.input,
            CompileOptions {
                family_name: args.name.unwrap_or_else(|| family_id.clone()),
                family_id,
                source_url: args
                    .source_url
                    .unwrap_or_else(|| args.input.display().to_string()),
                revision: args.revision.unwrap_or_else(|| "local".into()),
                license: args.license.unwrap_or_else(|| "User supplied".into()),
            },
        )?;
        fs::write(&args.output, serde_json::to_vec_pretty(&imported.family)?)
            .with_context(|| format!("could not write {}", args.output.display()))?;
        fs::write(&args.report, serde_json::to_vec_pretty(&imported.reports)?)
            .with_context(|| format!("could not write {}", args.report.display()))?;
        return Ok(());
    }
    let required = |value: Option<String>, flag: &str| {
        value.ok_or_else(|| anyhow!("{flag} is required for --format vscode"))
    };
    let imported = import_file(
        &args.input,
        ImportOptions {
            id: required(args.id, "--id")?,
            family_id: required(args.family_id, "--family-id")?,
            name: required(args.name, "--name")?,
            appearance: args
                .appearance
                .ok_or_else(|| anyhow!("--appearance is required for --format vscode"))?
                .into(),
            source_url: required(args.source_url, "--source-url")?,
            revision: required(args.revision, "--revision")?,
            license: required(args.license, "--license")?,
        },
    )?;
    fs::write(&args.output, serde_json::to_vec_pretty(&imported.theme)?)
        .with_context(|| format!("could not write {}", args.output.display()))?;
    fs::write(&args.report, serde_json::to_vec_pretty(&imported.report)?)
        .with_context(|| format!("could not write {}", args.report.display()))?;
    Ok(())
}
