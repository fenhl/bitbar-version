#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use {
    std::{
        convert::Infallible as Never,
        env::current_exe,
        io,
        str::FromStr,
        time::Duration,
    },
    bitbar::{
        ContentItem,
        Flavor,
        Menu,
        MenuItem,
    },
    semver::Version,
    serde::Deserialize,
    serde_with::{
        DisplayFromStr,
        serde_as,
    },
    wheel::traits::ReqwestResponseExt as _,
    crate::{
        config::Config,
        data::Data,
        github::Repo,
    },
};

mod config;
mod data;
mod github;
mod version;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)] HeaderToStr(#[from] reqwest::header::ToStrError),
    #[error(transparent)] InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
    #[error(transparent)] Io(#[from] io::Error),
    #[error(transparent)] Json(#[from] serde_json::Error),
    #[error(transparent)] ParseInt(#[from] std::num::ParseIntError),
    #[error(transparent)] Plist(#[from] plist::Error),
    #[error(transparent)] ReleaseVersion(#[from] github::ReleaseVersionError),
    #[error(transparent)] Reqwest(#[from] reqwest::Error),
    #[error(transparent)] SemVer(#[from] semver::Error),
    #[error(transparent)] VersionCheck(#[from] bitbar::flavor::swiftbar::VersionCheckError),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error("x-ratelimit-reset header is out of range for chrono::DateTime")]
    InvalidDateTime,
    #[error("missing x-ratelimit-reset header in GitHub error response")]
    MissingRateLimitResetHeader,
    #[error("no GitHub releases for {0}")]
    NoReleases(&'static str),
    #[error("attempted to send GitHub API request with streamed body")]
    UncloneableGitHubRequest,
}

impl From<Error> for Menu {
    fn from(e: Error) -> Menu {
        let mut menu = Vec::default();
        match e {
            Error::InvalidHeaderValue(e) => {
                menu.push(MenuItem::new(format!("reqwest error: {e}")));
                menu.push(MenuItem::new(format!("{e:?}")));
            }
            Error::Io(e) => {
                menu.push(MenuItem::new(format!("I/O error: {e}")));
                menu.push(MenuItem::new(format!("{e:?}")));
            }
            Error::Json(e) => {
                menu.push(MenuItem::new(format!("JSON error: {e}")));
                menu.push(MenuItem::new(format!("{e:?}")));
            }
            Error::NoReleases(repo) => menu.push(MenuItem::new(format!("no GitHub releases for {repo}"))),
            Error::Plist(e) => {
                menu.push(MenuItem::new(format!("error reading plist: {e}")));
                menu.push(MenuItem::new(format!("{e:?}")));
            }
            Error::ReleaseVersion(e) => menu.extend(Menu::from(e).0),
            Error::Reqwest(e) => {
                menu.push(MenuItem::new(format!("reqwest error: {e}")));
                if let Some(url) = e.url() {
                    menu.push(ContentItem::new(format!("URL: {url}"))
                        .href(url.clone()).expect("failed to parse the request error URL")
                        .color("blue").expect("failed to parse the color blue")
                        .into());
                }
                menu.push(MenuItem::new(format!("{e:?}")));
            }
            Error::SemVer(e) => {
                menu.push(MenuItem::new(format!("error parsing version: {e}")));
                menu.push(MenuItem::new(format!("{e:?}")));
            }
            Error::VersionCheck(e) => menu.extend(Menu::from(e).0),
            _ => {
                menu.push(MenuItem::new(&e));
                menu.push(MenuItem::new(format!("{e:?}")));
            }
        }
        menu.push(ContentItem::new("Refresh").refresh().into());
        Menu(menu)
    }
}

#[serde_as]
#[derive(Deserialize)]
struct BrewCask {
    #[serde_as(as = "DisplayFromStr")]
    version: VersionWithBuild,
}

struct VersionWithBuild(Version);

#[derive(Debug, thiserror::Error)]
enum VersionWithBuildParseError {
    #[error(transparent)] SemVer(#[from] semver::Error),
    #[error("no comma in version with build")]
    Split,
}

impl FromStr for VersionWithBuild {
    type Err = VersionWithBuildParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (version, _) = s.split_once(',').ok_or(VersionWithBuildParseError::Split)?;
        Ok(Self(version.parse()?))
    }
}

#[derive(Deserialize)]
struct Plist {
    #[serde(rename = "CFBundleShortVersionString")]
    bundle_short_version_string: Version,
}

trait ResultNeverExt {
    type Ok;

    fn never_unwrap(self) -> Self::Ok;
}

impl<T> ResultNeverExt for Result<T, Never> {
    type Ok = T;

    fn never_unwrap(self) -> T {
        match self {
            Ok(x) => x,
            Err(never) => match never {}
        }
    }
}

fn running_version() -> Result<Version, Error> {
    Ok(match Flavor::check() {
        Flavor::SwiftBar(swiftbar) => swiftbar.running_version()?,
        // BitBar does not provide running version info, assume same as installed
        Flavor::BitBar => installed_version()?,
    })
}

fn installed_version() -> Result<Version, Error> {
    let plist = match Flavor::check() {
        Flavor::SwiftBar(_) => plist::from_file::<_, Plist>("/Applications/SwiftBar.app/Contents/Info.plist"),
        Flavor::BitBar => plist::from_file::<_, Plist>("/Applications/BitBar.app/Contents/Info.plist"),
    }?;
    Ok(plist.bundle_short_version_string)
}

async fn homebrew_version(client: &reqwest::Client) -> Result<(&'static str, Version), Error> {
    let flavor_cask = match Flavor::check() {
        Flavor::SwiftBar(_) => "swiftbar",
        Flavor::BitBar => "bitbar",
    };
    //TODO use `brew info` subprocess instead? It seems to update faster than the API
    let VersionWithBuild(version) = client
        .get(format!("https://formulae.brew.sh/api/cask/{flavor_cask}.json"))
        .send().await?
        .detailed_error_for_status().await?
        .json_with_text_in_error::<BrewCask>().await?
        .version;
    Ok((flavor_cask, version))
}

async fn latest_version(client: &reqwest::Client) -> Result<Version, Error> {
    Ok(match Flavor::check() {
        Flavor::SwiftBar(_) => Repo::new("swiftbar", "SwiftBar")
            .latest_release(client).await?
            .ok_or(Error::NoReleases("swiftbar/SwiftBar"))?
            .version()?,
        Flavor::BitBar => Version::new(1, 10, 1), //TODO suggest moving to either SwiftBar or xbar
    })
}

#[bitbar::command]
async fn hide_until_homebrew_gt(version: Version) -> wheel::Result {
    let mut data = Data::load().await?;
    data.hide_until_homebrew_gt = Some(version);
    data.save().await?;
    Ok(())
}

#[bitbar::main(
    commands(hide_until_homebrew_gt),
    error_template_image = "../assets/logo.png",
)]
async fn main() -> Result<Menu, Error> {
    let current_exe = current_exe()?;
    let config = Config::load().await?;
    let http_client = reqwest::Client::builder()
        .user_agent(concat!("fenhl-bitbar-version/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .http2_prior_knowledge()
        .use_rustls_tls()
        .https_only(true)
        .build()?;
    let github_http_client = if let Some(github_token) = config.github_token { //TODO read from a config file
        reqwest::Client::builder()
            .user_agent(concat!("fenhl-bitbar-version/", env!("CARGO_PKG_VERSION")))
            .default_headers([(reqwest::header::AUTHORIZATION, format!("Bearer {github_token}").parse()?)].into_iter().collect())
            .timeout(Duration::from_secs(30))
            .http2_prior_knowledge()
            .use_rustls_tls()
            .https_only(true)
            .build()?
    } else {
        http_client.clone()
    };
    let latest = latest_version(&github_http_client).await?;
    let (cask_name, homebrew) = homebrew_version(&http_client).await?;
    let installed = installed_version()?;
    let running = running_version()?;
    let remote_plugin_commit_hash = Repo::new("fenhl", "bitbar-version").head(&github_http_client).await?.sha;
    Ok(if version::GIT_COMMIT_HASH != remote_plugin_commit_hash || running < latest && Data::load().await?.hide_until_homebrew_gt.map_or(true, |min_ver| homebrew > min_ver) {
        let mut menu = vec![
            ContentItem::default().template_image(&include_bytes!("../assets/logo.png")[..]).never_unwrap().into(),
            MenuItem::Sep,
        ];
        if version::GIT_COMMIT_HASH != remote_plugin_commit_hash {
            menu.push(MenuItem::new("New version of this plugin available"));
            menu.push(ContentItem::new("Update Via Cargo").command(bitbar::attr::Command::terminal(("cargo", "install-update", "--git", "bitbar-version"))).never_unwrap().into());
        }
        if installed < latest {
            menu.push(MenuItem::new(format!("{} {latest} available", Flavor::check())));
            menu.push(MenuItem::new(format!("You have {running}")));
            if homebrew < latest {
                menu.push(MenuItem::new(format!("Homebrew has {homebrew}")));
            }
            if homebrew > installed {
                menu.push(ContentItem::new(format!("Install using `brew upgrade --cask {cask_name}`")).command(bitbar::attr::Command::terminal(("brew", "upgrade", "--cask", cask_name))).never_unwrap().into());
            }
            if homebrew < latest {
                menu.push(ContentItem::new("Send Pull Request to Homebrew").command(bitbar::attr::Command::terminal(("brew", "bump-cask-pr", "--version", latest, cask_name))).never_unwrap().into());
                menu.push(ContentItem::new("Open GitHub Release").href(match Flavor::check() {
                    Flavor::SwiftBar(_) => "https://github.com/swiftbar/SwiftBar/releases/latest",
                    Flavor::BitBar => "https://github.com/matryer/BitBar/releases/latest",
                }).expect("failed to parse GitHub latest release URL").into());
                menu.push(ContentItem::new("Hide Until Homebrew Is Updated").command((current_exe.display(), "hide_until_homebrew_gt", homebrew)).never_unwrap().into());
            }
        } else if running < latest {
            menu.push(MenuItem::new(format!("Restart to update to {} {}", Flavor::check(), installed)));
            menu.push(MenuItem::new(format!("Currently running: {}", running)));
        }
        Menu(menu)
    } else {
        Menu::default()
    })
}
