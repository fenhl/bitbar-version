#![deny(rust_2018_idioms, unused, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]

use {
    std::{
        convert::{
            Infallible as Never,
            TryFrom,
            TryInto,
        },
        env::current_exe,
        ffi::OsString,
        fmt,
        io,
        time::Duration,
    },
    bitbar::{
        ContentItem,
        Flavor,
        Menu,
        MenuItem,
    },
    derive_more::From,
    itertools::Itertools as _,
    semver::Version,
    serde::Deserialize,
    crate::data::Data,
};

mod data;

#[derive(Debug, From)]
enum Error {
    Io(io::Error),
    Json(serde_json::Error),
    NoLeadingV,
    Plist(plist::Error),
    Reqwest(reqwest::Error),
    SemVer(semver::Error),
    VersionCheck(bitbar::flavor::VersionCheckError),
}

impl From<Error> for Menu {
    fn from(e: Error) -> Menu {
        let mut menu = Vec::default();
        match e {
            Error::Io(e) => {
                menu.push(MenuItem::new(format!("I/O error: {}", e)));
                menu.push(MenuItem::new(format!("{:?}", e)));
            }
            Error::Json(e) => {
                menu.push(MenuItem::new(format!("JSON error: {}", e)));
                menu.push(MenuItem::new(format!("{:?}", e)));
            }
            Error::NoLeadingV => menu.push(MenuItem::new("latest GitHub release does not include version number")),
            Error::Plist(e) => {
                menu.push(MenuItem::new(format!("error reading plist: {}", e)));
                menu.push(MenuItem::new(format!("{:?}", e)));
            }
            Error::Reqwest(e) => {
                menu.push(MenuItem::new(format!("reqwest error: {}", e)));
                if let Some(url) = e.url() {
                    menu.push(ContentItem::new(format!("URL: {}", url))
                        .href(url.clone()).expect("failed to parse the request error URL")
                        .color("blue").expect("failed to parse the color blue")
                        .into());
                }
            }
            Error::SemVer(e) => {
                menu.push(MenuItem::new(format!("error parsing version: {}", e)));
                menu.push(MenuItem::new(format!("{:?}", e)));
            }
            Error::VersionCheck(e) => menu.extend(Menu::from(e).0),
        }
        Menu(menu)
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

impl TryFrom<Release> for Version {
    type Error = Error;

    fn try_from(release: Release) -> Result<Version, Error> {
        if !release.tag_name.starts_with('v') { return Err(Error::NoLeadingV); }
        Ok(release.tag_name[1..].parse()?)
    }
}

#[derive(Deserialize)]
struct BrewCask {
    version: Version,
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
    let version = client
        .get(format!("https://formulae.brew.sh/api/cask/{}.json", flavor_cask))
        .send().await?
        .error_for_status()?
        .json::<BrewCask>().await?
        .version;
    Ok((flavor_cask, version))
}

async fn latest_version(client: &reqwest::Client) -> Result<Version, Error> {
    match Flavor::check() {
        Flavor::SwiftBar(_) => client
            .get("https://api.github.com/repos/swiftbar/SwiftBar/releases/latest")
            .send().await?
            .error_for_status()?
            .json::<Release>().await?
            .try_into(),
        Flavor::BitBar => Ok(Version::new(1, 10, 1)), //TODO suggest moving to either SwiftBar or xbar
    }
}

#[derive(From)]
enum HideUntilHomebrewGtError {
    DataSave(crate::data::SaveError),
    Json(serde_json::Error),
    NumArgs,
    SemVer(semver::Error),
}

impl fmt::Display for HideUntilHomebrewGtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HideUntilHomebrewGtError::DataSave(e) => e.fmt(f),
            HideUntilHomebrewGtError::Json(e) => write!(f, "JSON error: {}", e),
            HideUntilHomebrewGtError::NumArgs => write!(f, "wrong number of arguments for hide_until_homebrew_gt subcommand"),
            HideUntilHomebrewGtError::SemVer(e) => e.fmt(f),
        }
    }
}

#[bitbar::command]
fn hide_until_homebrew_gt(args: impl Iterator<Item = OsString>) -> Result<(), HideUntilHomebrewGtError> {
    let (version,) = args.collect_tuple().ok_or(HideUntilHomebrewGtError::NumArgs)?;
    let mut data = Data::new()?;
    data.hide_until_homebrew_gt = Some(version.to_string_lossy().parse()?);
    data.save()?;
    Ok(())
}

#[bitbar::main(error_template_image = "../assets/logo.png")]
async fn main() -> Result<Menu, Error> {
    let current_exe = current_exe()?;
    let client = reqwest::Client::builder()
        .user_agent(concat!("fenhl-bitbar-version/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .http2_prior_knowledge()
        .use_rustls_tls()
        .https_only(true)
        .build()?;
    let latest = latest_version(&client).await?;
    let (cask_name, homebrew) = homebrew_version(&client).await?;
    let installed = installed_version()?;
    let running = running_version()?;
    Ok(if running < latest && Data::new()?.hide_until_homebrew_gt.map_or(true, |min_ver| homebrew > min_ver) {
        if installed < latest {
            let mut menu = vec![
                ContentItem::default().template_image(&include_bytes!("../assets/logo.png")[..]).never_unwrap().into(),
                MenuItem::Sep,
                MenuItem::new(format!("{} {} available", Flavor::check(), latest)),
                MenuItem::new(format!("You have {}", running)),
            ];
            if homebrew < latest {
                menu.push(MenuItem::new(format!("Homebrew has {}", homebrew)));
            }
            if homebrew > installed {
                menu.push(ContentItem::new(format!("Install using `brew upgrade --cask {}`", cask_name)).command(bitbar::Command::terminal(("brew", "upgrade", "--cask", cask_name))).into());
            }
            if homebrew < latest {
                menu.push(ContentItem::new("Send Pull Request to Homebrew").command(bitbar::Command::terminal(("brew", "bump-cask-pr", "--version", latest, cask_name))).into());
                menu.push(ContentItem::new("Open GitHub Release").href(match Flavor::check() {
                    Flavor::SwiftBar(_) => "https://github.com/swiftbar/SwiftBar/releases/latest",
                    Flavor::BitBar => "https://github.com/matryer/BitBar/releases/latest",
                }).expect("failed to parse GitHub latest release URL").into());
                menu.push(ContentItem::new("Hide Until Homebrew Is Updated").command((current_exe.display(), "hide_until_homebrew_gt", homebrew)).into());
            }
            Menu(menu)
        } else {
            Menu(vec![
                ContentItem::default().template_image(&include_bytes!("../assets/logo.png")[..]).never_unwrap().into(),
                MenuItem::Sep,
                MenuItem::new(format!("Restart to update to {} {}", Flavor::check(), installed)),
                MenuItem::new(format!("Currently running: {}", running)),
            ])
        }
    } else {
        Menu::default()
    })
}
