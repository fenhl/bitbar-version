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

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Data {
    pub(crate) hide_until_homebrew_gt: Option<Version>,
}

impl Data {
    pub(crate) async fn load() -> wheel::Result<Data> {
        Ok(if let Some(path) = BaseDirectories::new().find_data_file(PATH) {
            fs::read_json(path).await?
        } else {
            Self::default()
        })
    }

    pub(crate) async fn save(self) -> wheel::Result {
        let path = BaseDirectories::new().place_data_file(PATH).at_unknown()?;
        fs::write_json(path, self).await?;
        Ok(())
    }
}
