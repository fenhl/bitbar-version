use {
    std::{
        fmt,
        fs::File,
        io,
    },
    derive_more::From,
    semver::Version,
    serde::{
        Deserialize,
        Serialize,
    },
};

const PATH: &str = "bitbar/plugin-cache/bitbar-version.json";

#[derive(From)]
pub(crate) enum SaveError {
    Basedir(xdg_basedir::Error),
    Io(io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Basedir(e) => e.fmt(f),
            SaveError::Io(e) => write!(f, "I/O error: {}", e),
            SaveError::Json(e) => write!(f, "JSON error: {}", e),
        }
    }
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
