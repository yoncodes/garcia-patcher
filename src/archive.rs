use std::{
    collections::{BTreeSet, HashMap},
    error::Error,
    ffi::OsStr,
    fs::{self, File},
    io::{self, Cursor, Read, Seek, Write},
    path::{Path, PathBuf},
};

use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub fn extract_xapk(input: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    let mut archive = ZipArchive::new(File::open(input)?)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| format!("unsafe archive path: {}", entry.name()))?;
        let output = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output)?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        io::copy(&mut entry, &mut File::create(output)?)?;
    }
    Ok(())
}

pub fn build_xapk(directory: &Path, output: &Path) -> Result<()> {
    let mut files = Vec::new();
    collect_files(directory, directory, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut writer = ZipWriter::new(File::create(output)?);
    for (name, path) in files {
        writer.start_file(
            name,
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )?;
        io::copy(&mut File::open(path)?, &mut writer)?;
    }
    writer.finish()?;
    Ok(())
}

fn collect_files(root: &Path, directory: &Path, files: &mut Vec<(String, PathBuf)>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else {
            let relative = path.strip_prefix(root)?;
            files.push((relative.to_string_lossy().replace('\\', "/"), path));
        }
    }
    Ok(())
}

pub fn apk_files(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut apks = fs::read_dir(directory)?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(OsStr::to_str) == Some("apk"))
        .collect::<Vec<_>>();
    apks.sort();
    Ok(apks)
}

pub fn find_settings(apks: &[PathBuf]) -> Result<(PathBuf, String)> {
    for apk in apks {
        if let Some(settings) = try_read_zip_entry(File::open(apk)?, "assets/settings.txt")? {
            return Ok((apk.clone(), String::from_utf8(settings)?));
        }
    }
    Err("assets/settings.txt was not found in any APK split".into())
}

pub fn read_apk_entry(apk: &Path, entry: &str) -> Result<Vec<u8>> {
    try_read_zip_entry(File::open(apk)?, entry)?
        .ok_or_else(|| format!("{entry} was not found in {}", apk.display()).into())
}

fn try_read_zip_entry(reader: impl Read + Seek, entry: &str) -> Result<Option<Vec<u8>>> {
    let mut archive = ZipArchive::new(reader)?;
    match archive.by_name(entry) {
        Ok(mut file) => {
            let mut bytes = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut bytes)?;
            Ok(Some(bytes))
        }
        Err(zip::result::ZipError::FileNotFound) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn patch_apk(apk: &Path, replacements: &[(&str, &[u8])]) -> Result<()> {
    let replacement_map = replacements.iter().copied().collect::<HashMap<_, _>>();
    let temporary = apk.with_extension("rewritten.apk");
    let mut source = ZipArchive::new(File::open(apk)?)?;
    let mut writer = ZipWriter::new(File::create(&temporary)?);
    let mut replaced = BTreeSet::new();

    for index in 0..source.len() {
        let mut entry = source.by_index(index)?;
        let name = entry.name().to_owned();
        if is_signature_entry(&name) {
            continue;
        }
        let options = entry_options(&entry);
        if entry.is_dir() {
            writer.add_directory(name, options)?;
            continue;
        }
        writer.start_file(&name, options)?;
        if let Some(bytes) = replacement_map.get(name.as_str()) {
            writer.write_all(bytes)?;
            replaced.insert(name);
        } else {
            io::copy(&mut entry, &mut writer)?;
        }
    }
    writer.finish()?;

    for name in replacement_map.keys() {
        if !replaced.contains(*name) {
            fs::remove_file(&temporary)?;
            return Err(format!("APK entry not found: {name}").into());
        }
    }
    fs::remove_file(apk)?;
    fs::rename(temporary, apk)?;
    Ok(())
}

fn entry_options(entry: &zip::read::ZipFile<'_, File>) -> SimpleFileOptions {
    let compression = match entry.compression() {
        CompressionMethod::Stored => CompressionMethod::Stored,
        _ => CompressionMethod::Deflated,
    };
    let mut options = SimpleFileOptions::default().compression_method(compression);
    if let Some(mode) = entry.unix_mode() {
        options = options.unix_permissions(mode);
    }
    options
}

fn is_signature_entry(name: &str) -> bool {
    let uppercase = name.replace('\\', "/").to_ascii_uppercase();
    uppercase == "STAMP-CERT-SHA256"
        || uppercase == "META-INF/MANIFEST.MF"
        || (uppercase.starts_with("META-INF/")
            && [".RSA", ".DSA", ".EC", ".SF"]
                .iter()
                .any(|extension| uppercase.ends_with(extension)))
}

pub fn verify_xapk(
    xapk: &Path,
    source_apks: &[PathBuf],
    source_base_apk: &Path,
    expected_settings: &str,
    expected_manifest: &[u8],
) -> Result<()> {
    let expected_names = source_apks
        .iter()
        .filter_map(|path| path.file_name().and_then(OsStr::to_str))
        .collect::<BTreeSet<_>>();
    let base_name = source_base_apk
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or("base APK name is not valid Unicode")?;
    let mut archive = ZipArchive::new(File::open(xapk)?)?;
    let actual_names = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|entry| entry.name().to_owned())
        })
        .filter(|name| name.ends_with(".apk"))
        .collect::<BTreeSet<_>>();
    if expected_names != actual_names.iter().map(String::as_str).collect() {
        return Err("output XAPK does not contain the same APK split set as the input".into());
    }

    let mut base = Vec::new();
    archive.by_name(base_name)?.read_to_end(&mut base)?;
    let mut base = ZipArchive::new(Cursor::new(base))?;
    let mut settings = Vec::new();
    base.by_name("assets/settings.txt")?
        .read_to_end(&mut settings)?;
    if settings != expected_settings.as_bytes() {
        return Err("output XAPK settings verification failed".into());
    }
    let mut manifest = Vec::new();
    base.by_name("AndroidManifest.xml")?
        .read_to_end(&mut manifest)?;
    if manifest != expected_manifest {
        return Err("output XAPK manifest verification failed".into());
    }
    Ok(())
}
