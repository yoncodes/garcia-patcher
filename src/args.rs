use std::{
    env,
    error::Error,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use clap::Parser;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Parser)]
#[command(version, about = "Patch an ALLfiring XAPK for a Garcia server")]
pub struct Cli {
    /// Source XAPK containing every APK split.
    pub input: PathBuf,

    /// TOML file containing endpoints and optional Android tool paths.
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,

    /// Destination XAPK. Defaults to a host-qualified name beside the input.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

impl Cli {
    pub fn output_path(&self, input: &Path, host: &str) -> Result<PathBuf> {
        let path = self.output.clone().unwrap_or_else(|| {
            let stem = input
                .file_stem()
                .and_then(OsStr::to_str)
                .unwrap_or("allfiring");
            input.with_file_name(format!("{stem}-garcia-{host}.xapk"))
        });
        Ok(if path.is_absolute() {
            path
        } else {
            env::current_dir()?.join(path)
        })
    }
}
