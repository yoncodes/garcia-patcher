use std::{error::Error, fmt};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const PAIRIP_APPLICATION: &str = "com.pairip.application.Application";
const GAME_APPLICATION: &str = "com.DcSdk.DcApplication";

pub enum RestoreStatus {
    Restored,
    AlreadyRestored,
}

impl fmt::Display for RestoreStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Restored => write!(formatter, "restored to {GAME_APPLICATION}"),
            Self::AlreadyRestored => write!(formatter, "already set to {GAME_APPLICATION}"),
        }
    }
}

pub fn restore_game_application(manifest: &[u8]) -> Result<(Vec<u8>, RestoreStatus)> {
    let old = utf16le(PAIRIP_APPLICATION);
    let new = utf16le(GAME_APPLICATION);
    let old_matches = match_offsets(manifest, &old);
    let new_matches = match_offsets(manifest, &new);

    if old_matches.is_empty() && new_matches.len() == 1 {
        return Ok((manifest.to_vec(), RestoreStatus::AlreadyRestored));
    }
    if old_matches.len() != 1 {
        return Err(format!(
            "expected one Pairip application class in AndroidManifest.xml, found {}",
            old_matches.len()
        )
        .into());
    }
    let index = old_matches[0];
    if index < 2
        || u16::from_le_bytes([manifest[index - 2], manifest[index - 1]])
            != PAIRIP_APPLICATION.encode_utf16().count() as u16
        || manifest.get(index + old.len()..index + old.len() + 2) != Some(&[0, 0])
    {
        return Err("unsupported AndroidManifest.xml string-pool layout".into());
    }

    let mut patched = manifest.to_vec();
    patched[index - 2..index]
        .copy_from_slice(&(GAME_APPLICATION.encode_utf16().count() as u16).to_le_bytes());
    patched[index..index + new.len()].copy_from_slice(&new);
    patched[index + new.len()..index + old.len() + 2].fill(0);
    Ok((patched, RestoreStatus::Restored))
}

fn match_offsets(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    haystack
        .windows(needle.len())
        .enumerate()
        .filter_map(|(index, bytes)| (bytes == needle).then_some(index))
        .collect()
}

fn utf16le(value: &str) -> Vec<u8> {
    value.encode_utf16().flat_map(u16::to_le_bytes).collect()
}
