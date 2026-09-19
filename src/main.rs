mod android;
mod archive;
mod args;
mod config;
mod manifest;
mod settings;
mod signing;
mod temp;

use std::{error::Error, ffi::OsStr, fs};

use clap::Parser;

use android::{find_build_tools, tool_path};
use archive::{apk_files, build_xapk, extract_xapk, find_settings, patch_apk, verify_xapk};
use args::Cli;
use config::Config;
use manifest::restore_game_application;
use settings::patch_settings;
use signing::{sign_apk, signing_key};
use temp::TempDir;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load(&cli.config)?;
    let input = cli.input.canonicalize()?;
    if input.extension().and_then(OsStr::to_str) != Some("xapk") {
        return Err("input must be an XAPK so every split can be re-signed".into());
    }

    let output = cli.output_path(&input, &config.endpoints.host)?;
    if output.exists() {
        return Err(format!("output already exists: {}", output.display()).into());
    }

    let tools = find_build_tools(
        config.tools.zipalign.as_deref(),
        config.tools.apksigner.as_deref(),
    )?;
    let keytool = tool_path(config.tools.keytool.as_deref(), "keytool")?;
    let temp = TempDir::new()?;
    let unpacked = temp.path().join("xapk");
    extract_xapk(&input, &unpacked)?;

    let apks = apk_files(&unpacked)?;
    if apks.is_empty() {
        return Err("XAPK contains no APK files".into());
    }

    let (base_apk, original_settings) = find_settings(&apks)?;
    let patched_settings = patch_settings(&original_settings, &config.endpoints)?;
    let manifest = archive::read_apk_entry(&base_apk, "AndroidManifest.xml")?;
    let (manifest, manifest_status) = restore_game_application(&manifest)?;
    patch_apk(
        &base_apk,
        &[
            ("assets/settings.txt", patched_settings.as_bytes()),
            ("AndroidManifest.xml", &manifest),
        ],
    )?;

    let keystore = signing_key(config.tools.keystore.as_deref(), &keytool)?;
    for apk in &apks {
        if apk != &base_apk {
            patch_apk(apk, &[])?;
        }
        sign_apk(apk, &keystore, &tools)?;
    }

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    build_xapk(&unpacked, &output)?;
    verify_xapk(&output, &apks, &base_apk, &patched_settings, &manifest)?;

    println!("patched endpoints for {}", config.endpoints.host);
    println!("application class {manifest_status}");
    println!("signed and verified {} APK split(s)", apks.len());
    println!("output: {}", output.display());
    Ok(())
}
