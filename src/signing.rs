use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::android::{BuildTools, run_command};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const STORE_PASSWORD: &str = "garcia-local";
const KEY_ALIAS: &str = "garcia";

pub fn signing_key(explicit: Option<&Path>, keytool: &Path) -> Result<PathBuf> {
    let key = match explicit {
        Some(path) => path.to_owned(),
        None => data_directory()?.join("garcia-local.p12"),
    };
    if let Some(parent) = key.parent() {
        fs::create_dir_all(parent)?;
    }
    if !key.exists() {
        run_command(
            Command::new(keytool)
                .args(["-genkeypair", "-storetype", "PKCS12", "-keyalg", "RSA"])
                .args(["-keysize", "2048", "-validity", "10000"])
                .args(["-alias", KEY_ALIAS, "-dname", "CN=Garcia Local"])
                .arg("-keystore")
                .arg(&key)
                .args(["-storepass", STORE_PASSWORD, "-keypass", STORE_PASSWORD]),
            "create signing key",
        )?;
    }
    Ok(key)
}

fn data_directory() -> Result<PathBuf> {
    Ok(
        PathBuf::from(env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is not set")?)
            .join("GarciaPatcher"),
    )
}

pub fn sign_apk(apk: &Path, keystore: &Path, tools: &BuildTools) -> Result<()> {
    let aligned = apk.with_extension("aligned.apk");
    let signed = apk.with_extension("signed.apk");
    run_command(
        Command::new(&tools.zipalign)
            .args(["-f", "-p", "4"])
            .arg(apk)
            .arg(&aligned),
        "align APK",
    )?;
    run_command(
        Command::new(&tools.apksigner)
            .arg("sign")
            .arg("--ks")
            .arg(keystore)
            .args(["--ks-key-alias", KEY_ALIAS])
            .arg("--ks-pass")
            .arg(format!("pass:{STORE_PASSWORD}"))
            .arg("--key-pass")
            .arg(format!("pass:{STORE_PASSWORD}"))
            .args(["--v4-signing-enabled", "false"])
            .arg("--out")
            .arg(&signed)
            .arg(&aligned),
        "sign APK",
    )?;
    run_command(
        Command::new(&tools.apksigner)
            .args(["verify", "--verbose"])
            .arg(&signed),
        "verify APK signature",
    )?;
    fs::remove_file(apk)?;
    fs::rename(&signed, apk)?;
    fs::remove_file(aligned)?;
    Ok(())
}
