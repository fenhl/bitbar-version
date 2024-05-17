use {
    std::{
        fs::File,
        io,
    },
    semver::Version,
    serde::{
        Deserialize,
        Serialize,
    },
};

const PATH: &str = "bitbar/plugin-cache/bitbar-version.json";

#[derive(Debug, thiserror::Error)]
pub(crate) enum SaveError {
    #[error(transparent)] Basedir(#[from] xdg_basedir::Error),
    #[error(transparent)] Io(#[from] io::Error),
    #[error(transparent)] Json(#[from] serde_json::Error),
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Data {
    pub(crate) hide_until_homebrew_gt: Option<Version>,
}

impl Data {
    pub(crate) fn new() -> Result<Data, serde_json::Error> {
        let dirs = xdg_basedir::get_data_home().into_iter().chain(xdg_basedir::get_data_dirs());
        Ok(dirs.filter_map(|data_dir| File::open(data_dir.join(PATH)).ok())
            .next().map_or(Ok(Data::default()), serde_json::from_reader)?)
    }

    pub(crate) fn save(self) -> Result<(), SaveError> {
        let dirs = xdg_basedir::get_data_home().into_iter().chain(xdg_basedir::get_data_dirs());
        for data_dir in dirs {
            let data_path = data_dir.join(PATH);
            if data_path.exists() {
                if let Some(()) = File::create(data_path).ok()
                    .and_then(|data_file| serde_json::to_writer_pretty(data_file, &self).ok())
                { return Ok(()) }
            }
        }
        let data_path = xdg_basedir::get_data_home()?.join(PATH);
        let data_file = File::create(data_path)?;
        serde_json::to_writer_pretty(data_file, &self)?;
        Ok(())
    }
}
