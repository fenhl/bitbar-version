use {
    std::time::Duration,
    bitbar::{
        Menu,
        MenuItem,
    },
    chrono::prelude::*,
    reqwest::{
        Client,
        StatusCode,
    },
    semver::Version,
    serde::Deserialize,
    tokio::time::sleep,
    wheel::traits::ReqwestResponseExt as _,
    crate::Error,
};

pub(crate) trait RequestBuilderExt {
    async fn send_github(self) -> Result<reqwest::Response, Error>;
}

impl RequestBuilderExt for reqwest::RequestBuilder {
    async fn send_github(self) -> Result<reqwest::Response, Error> {
        let mut exponential_backoff = Duration::from_secs(60);
        loop {
            match self.try_clone().ok_or(Error::UncloneableGitHubRequest)?.send().await?.detailed_error_for_status().await {
                Ok(response) => break Ok(response),
                Err(wheel::Error::ResponseStatus { inner, headers, text }) if inner.status().is_some_and(|status| matches!(status, StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS)) => {
                    if let Some(retry_after) = headers.get(reqwest::header::RETRY_AFTER) {
                        sleep(Duration::from_secs(retry_after.to_str()?.parse()?)).await;
                    } else if headers.get("x-ratelimit-remaining").is_some_and(|x_ratelimit_remaining| x_ratelimit_remaining == "0") {
                        let now = Utc::now();
                        let until = DateTime::from_timestamp(headers.get("x-ratelimit-reset").ok_or(Error::MissingRateLimitResetHeader)?.to_str()?.parse()?, 0).ok_or(Error::InvalidDateTime)?;
                        if let Ok(delta) = (until - now).to_std() {
                            sleep(delta).await;
                        }
                    } else if exponential_backoff >= Duration::from_secs(60 * 60) {
                        break Err(wheel::Error::ResponseStatus { inner, headers, text }.into())
                    } else {
                        sleep(exponential_backoff).await;
                        exponential_backoff *= 2;
                    }
                }
                Err(e) => break Err(e.into()),
            }
        }
    }
}

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

#[derive(Debug, thiserror::Error)]
pub(crate) enum ReleaseVersionError {
    #[error(transparent)] SemVer(#[from] semver::Error),
    #[error("latest GitHub release does not include version number")]
    NoLeadingV,
}

impl From<ReleaseVersionError> for Menu {
    fn from(e: ReleaseVersionError) -> Menu {
        let mut menu = Vec::default();
        match e {
            ReleaseVersionError::SemVer(e) => {
                menu.push(MenuItem::new(format!("error parsing version: {}", e)));
                menu.push(MenuItem::new(format!("{:?}", e)));
            }
            ReleaseVersionError::NoLeadingV => menu.push(MenuItem::new("latest GitHub release does not include version number")),
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
    pub(crate) async fn head(&self, client: &Client) -> Result<Commit, Error> {
        let repo_info = client.get(&format!("https://api.github.com/repos/{}/{}", self.user, self.name))
            .send_github().await?
            .json_with_text_in_error::<RepoInfo>().await?;
        Ok(client.get(&repo_info.branches_url.replace("{/branch}", &format!("/{}", repo_info.default_branch)))
            .send_github().await?
            .json_with_text_in_error::<BranchInfo>().await?
            .commit)
    }

    pub(crate) async fn latest_release(&self, client: &Client) -> Result<Option<Release>, Error> {
        Ok(match client.get(&format!("https://api.github.com/repos/{}/{}/releases/latest", self.user, self.name)).send_github().await {
            Ok(response) => Some(response.json_with_text_in_error::<Release>().await?),
            Err(Error::Reqwest(e)) if e.status().is_some_and(|status| status == StatusCode::NOT_FOUND) => None, // no releases yet
            Err(e) => return Err(e),
        })
    }
}
