use {
    serde::Deserialize,
    wheel::fs,
    xdg::BaseDirectories,
};

const PATH: &str = "bitbar/plugins/bitbar-version.json";

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Config {
    pub(crate) github_token: Option<String>,
}

impl Config {
    pub(crate) async fn load() -> wheel::Result<Self> {
        Ok(if let Some(path) = BaseDirectories::new().find_config_file(PATH) {
            fs::read_json(path).await?
        } else {
            Self::default()
        })
    }
}
