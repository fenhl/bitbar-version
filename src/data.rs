use {
    semver::Version,
    serde::{
        Deserialize,
        Serialize,
    },
    wheel::{
        fs,
        traits::IoResultExt as _,
    },
    xdg::BaseDirectories,
};

const PATH: &str = "bitbar/plugin-cache/bitbar-version.json";

#[derive(Debug, thiserror::Error)]
pub(crate) enum LoadError {
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error(transparent)] Xdg(#[from] xdg::BaseDirectoriesError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SaveError {
    #[error(transparent)] Json(#[from] serde_json::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error(transparent)] Xdg(#[from] xdg::BaseDirectoriesError),
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Data {
    pub(crate) hide_until_homebrew_gt: Option<Version>,
}

impl Data {
    pub(crate) async fn load() -> Result<Data, LoadError> {
        Ok(if let Some(path) = BaseDirectories::new()?.find_data_file(PATH) {
            fs::read_json(path).await?
        } else {
            Self::default()
        })
    }

    pub(crate) async fn save(self) -> Result<(), SaveError> {
        let path = BaseDirectories::new()?.place_data_file(PATH).at_unknown()?;
        fs::write(path, serde_json::to_vec_pretty(&self)?).await?;
        Ok(())
    }
}
