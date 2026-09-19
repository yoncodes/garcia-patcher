use std::{
    error::Error,
    fs,
    net::Ipv4Addr,
    path::{Path, PathBuf},
};

use serde::Deserialize;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub endpoints: Endpoints,
    #[serde(default)]
    pub tools: Tools,
}

#[derive(Debug, Deserialize)]
pub struct Endpoints {
    pub host: String,
    pub game_port: u16,
    pub sdk_port: u16,
    pub hotpatch_port: u16,
}

#[derive(Debug, Default, Deserialize)]
pub struct Tools {
    pub zipalign: Option<PathBuf>,
    pub apksigner: Option<PathBuf>,
    pub keytool: Option<PathBuf>,
    pub keystore: Option<PathBuf>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let mut config: Self = toml::from_str(&contents)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        config.endpoints.validate()?;

        let root = path.parent().unwrap_or_else(|| Path::new("."));
        config.tools.zipalign = resolve(root, config.tools.zipalign);
        config.tools.apksigner = resolve(root, config.tools.apksigner);
        config.tools.keytool = resolve(root, config.tools.keytool);
        config.tools.keystore = resolve(root, config.tools.keystore);
        Ok(config)
    }
}

impl Endpoints {
    pub fn cdn_url(&self) -> String {
        format!(
            "http://{}:{}/prod/en/Android",
            self.host, self.hotpatch_port
        )
    }

    pub fn game_address(&self) -> String {
        format!("{}:{}", self.host, self.game_port)
    }

    pub fn sdk_url(&self) -> String {
        format!("http://{}:{}", self.host, self.sdk_port)
    }

    fn validate(&self) -> Result<()> {
        validate_host(&self.host)?;
        for (name, port) in [
            ("game_port", self.game_port),
            ("sdk_port", self.sdk_port),
            ("hotpatch_port", self.hotpatch_port),
        ] {
            if port == 0 {
                return Err(format!("endpoints.{name} must be from 1 to 65535").into());
            }
        }
        Ok(())
    }
}

fn resolve(root: &Path, path: Option<PathBuf>) -> Option<PathBuf> {
    path.map(|path| {
        if path.is_absolute() {
            path
        } else {
            root.join(path)
        }
    })
}

fn validate_host(host: &str) -> Result<()> {
    host.parse::<Ipv4Addr>()
        .map(|_| ())
        .map_err(|_| "endpoints.host must be an IPv4 address".into())
}
