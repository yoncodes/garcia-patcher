use std::error::Error;

use crate::config::Endpoints;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub fn patch_settings(settings: &str, endpoints: &Endpoints) -> Result<String> {
    let replacements = [
        ("cdn", endpoints.cdn_url()),
        ("ipaddress", endpoints.game_address()),
        ("url", endpoints.sdk_url()),
        ("noticeURL", endpoints.sdk_url()),
    ];
    let mut found = [false; 4];
    let lines = settings
        .lines()
        .map(|line| {
            for (index, (key, value)) in replacements.iter().enumerate() {
                if line
                    .strip_prefix(key)
                    .is_some_and(|remainder| remainder.starts_with('='))
                {
                    found[index] = true;
                    return format!("{key}={value}");
                }
            }
            line.to_owned()
        })
        .collect::<Vec<_>>();

    if let Some(((key, _), _)) = replacements.iter().zip(found).find(|(_, found)| !found) {
        return Err(format!("settings.txt is missing {key}").into());
    }
    Ok(lines.join("\r\n") + "\r\n")
}
