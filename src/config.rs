use {
    serde::Deserialize,
    wheel::fs,
    xdg::BaseDirectories,
};

const PATH: &str = "bitbar/plugins/bitbar-version.json";

#[derive(Debug, thiserror::Error)]
pub(crate) enum LoadError {
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error(transparent)] Xdg(#[from] xdg::BaseDirectoriesError),
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Config {
    pub(crate) github_token: Option<String>,
}

impl Config {
    pub(crate) async fn load() -> Result<Self, LoadError> {
        Ok(if let Some(path) = BaseDirectories::new()?.find_config_file(PATH) {
            fs::read_json(path).await?
        } else {
            Self::default()
        })
    }
}
