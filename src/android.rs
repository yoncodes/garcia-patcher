use std::{
    env,
    error::Error,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct BuildTools {
    pub apksigner: PathBuf,
    pub zipalign: PathBuf,
}

pub fn tool_path(explicit: Option<&Path>, default: &str) -> Result<PathBuf> {
    match explicit {
        Some(path) if path.is_file() => Ok(path.to_owned()),
        Some(path) => Err(format!("tool was not found at {}", path.display()).into()),
        None => Ok(PathBuf::from(default)),
    }
}

pub fn find_build_tools(zipalign: Option<&Path>, apksigner: Option<&Path>) -> Result<BuildTools> {
    if zipalign.is_some() && apksigner.is_some() {
        return Ok(BuildTools {
            zipalign: tool_path(zipalign, "zipalign.exe")?,
            apksigner: tool_path(apksigner, "apksigner.bat")?,
        });
    }

    let sdk = PathBuf::from(env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is not set")?)
        .join("Android/Sdk/build-tools");
    let directory = fs::read_dir(&sdk)?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.join("zipalign.exe").is_file() && path.join("apksigner.bat").is_file())
        .max_by(|left, right| version_key(left).cmp(&version_key(right)))
        .ok_or_else(|| format!("Android Build Tools were not found under {}", sdk.display()))?;

    Ok(BuildTools {
        zipalign: match zipalign {
            Some(path) => tool_path(Some(path), "zipalign.exe")?,
            None => directory.join("zipalign.exe"),
        },
        apksigner: match apksigner {
            Some(path) => tool_path(Some(path), "apksigner.bat")?,
            None => directory.join("apksigner.bat"),
        },
    })
}

fn version_key(path: &Path) -> Vec<u32> {
    path.file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .split(['.', '-'])
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

pub fn run_command(command: &mut Command, action: &str) -> Result<()> {
    let status = command.status()?;
    if !status.success() {
        return Err(format!("failed to {action} (exit code {status})").into());
    }
    Ok(())
}
