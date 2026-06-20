use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "linux-voice-typer")]
#[command(version, about = "Linux-first local voice typing for Wayland")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(long, global = true)]
    pub terminal_hotkey: bool,
}

impl Cli {
    pub fn run_args(&self) -> RunArgs {
        RunArgs {
            terminal_hotkey: self.terminal_hotkey,
            terminal_paste: false,
            editor_paste: false,
        }
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    Setup(SetupArgs),
    Doctor,
    PasteTest,
    Run(RunArgs),
}

#[derive(Debug, Clone, clap::Args)]
pub struct RunArgs {
    #[arg(long)]
    pub terminal_hotkey: bool,

    #[arg(long, conflicts_with = "editor_paste")]
    pub terminal_paste: bool,

    #[arg(long, conflicts_with = "terminal_paste")]
    pub editor_paste: bool,
}

#[derive(Debug, Clone, clap::Args)]
pub struct SetupArgs {
    #[arg(long)]
    pub yes: bool,

    #[arg(long, value_enum, default_value_t = WhisperModel::Small)]
    pub model: WhisperModel,

    #[arg(long)]
    pub skip_system_deps: bool,

    #[arg(long)]
    pub skip_whisper_build: bool,

    #[arg(long)]
    pub skip_model_download: bool,

    #[arg(long)]
    pub rebuild_whisper: bool,

    #[arg(long)]
    pub force: bool,
}

impl Default for SetupArgs {
    fn default() -> Self {
        Self {
            yes: false,
            model: WhisperModel::Small,
            skip_system_deps: false,
            skip_whisper_build: false,
            skip_model_download: false,
            rebuild_whisper: false,
            force: false,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum WhisperModel {
    Tiny,
    Base,
    Small,
    Medium,
}

impl WhisperModel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tiny => "tiny",
            Self::Base => "base",
            Self::Small => "small",
            Self::Medium => "medium",
        }
    }
}
