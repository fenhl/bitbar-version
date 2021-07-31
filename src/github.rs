use {
    bitbar::{
        Menu,
        MenuItem,
    },
    derive_more::From,
    reqwest::{
        Client,
        StatusCode,
    },
    semver::Version,
    serde::Deserialize,
};

#[derive(Deserialize)]
struct RepoInfo {
    branches_url: String,
    default_branch: String,
}

#[derive(Deserialize)]
pub(crate) struct BranchInfo {
    commit: Commit,
}

#[derive(Deserialize)]
pub(crate) struct Commit {
    pub(crate) sha: String,
}

#[derive(Deserialize)]
pub(crate) struct Release {
    tag_name: String,
}

#[derive(Debug, From)]
pub(crate) enum ReleaseVersionError {
    NoLeadingV,
    SemVer(semver::Error),
}

impl From<ReleaseVersionError> for Menu {
    fn from(e: ReleaseVersionError) -> Menu {
        let mut menu = Vec::default();
        match e {
            ReleaseVersionError::NoLeadingV => menu.push(MenuItem::new("latest GitHub release does not include version number")),
            ReleaseVersionError::SemVer(e) => {
                menu.push(MenuItem::new(format!("error parsing version: {}", e)));
                menu.push(MenuItem::new(format!("{:?}", e)));
            }
        }
        Menu(menu)
    }
}

impl Release {
    pub(crate) fn version(&self) -> Result<Version, ReleaseVersionError> {
        if !self.tag_name.starts_with('v') { return Err(ReleaseVersionError::NoLeadingV); }
        Ok(self.tag_name[1..].parse()?)
    }
}

/// A GitHub repository. Provides API methods.
pub(crate) struct Repo {
    /// The GitHub user or organization who owns this repo.
    user: String,
    /// The name of the repo.
    name: String,
}

impl Repo {
    pub(crate) fn new(user: impl ToString, name: impl ToString) -> Repo {
        Repo {
            user: user.to_string(),
            name: name.to_string(),
        }
    }

    /// Returns the latest commit on the default branch.
    pub(crate) async fn head(&self, client: &Client) -> reqwest::Result<Commit> {
        let repo_info = client.get(&format!("https://api.github.com/repos/{}/{}", self.user, self.name))
            .send().await?
            .error_for_status()?
            .json::<RepoInfo>().await?;
        Ok(client.get(&repo_info.branches_url.replace("{/branch}", &format!("/{}", repo_info.default_branch)))
            .send().await?
            .error_for_status()?
            .json::<BranchInfo>().await?
            .commit)
    }

    pub(crate) async fn latest_release(&self, client: &Client) -> reqwest::Result<Option<Release>> {
        let response = client.get(&format!("https://api.github.com/repos/{}/{}/releases/latest", self.user, self.name))
            .send().await?;
        if response.status() == StatusCode::NOT_FOUND { return Ok(None) } // no releases yet
        Ok(Some(
            response.error_for_status()?
                .json::<Release>().await?
        ))
    }
}
