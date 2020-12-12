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
    },
    bitbar::{
        ContentItem,
        Menu,
        MenuItem,
    },
    derive_more::From,
    itertools::Itertools as _,
    semver::{
        SemVerError,
        Version,
    },
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
    SemVer(SemVerError),
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
    #[serde(rename = "CFBundleVersion")]
    bundle_version: Version,
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

fn current_version() -> Result<Version, Error> {
    let plist = plist::from_file::<_, Plist>("/Applications/BitBar.app/Contents/Info.plist")?;
    Ok(plist.bundle_version)
}

async fn homebrew_version(client: &reqwest::Client) -> Result<Version, Error> {
    Ok(client
        .get("https://formulae.brew.sh/api/cask/bitbar.json")
        .send().await?
        .error_for_status()?
        .json::<BrewCask>().await?
        .version)
}

async fn latest_version(client: &reqwest::Client) -> Result<Version, Error> {
    client
        .get("https://api.github.com/repos/matryer/bitbar/releases/latest")
        .send().await?
        .error_for_status()?
        .json::<Release>().await?
        .try_into()
}

#[derive(From)]
enum HideUntilHomebrewGtError {
    DataSave(crate::data::SaveError),
    Json(serde_json::Error),
    NumArgs,
    SemVer(SemVerError),
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
        .user_agent(concat!("fenhl/bitbar-version/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let latest = latest_version(&client).await?;
    let homebrew = homebrew_version(&client).await?;
    let current = current_version()?;
    Ok(if current < latest && Data::new()?.hide_until_homebrew_gt.map_or(true, |min_ver| homebrew > min_ver) {
        let mut menu = vec![
            ContentItem::default().template_image(&include_bytes!("../assets/logo.png")[..]).never_unwrap().into(),
            MenuItem::Sep,
            MenuItem::new(format!("BitBar {} available", latest)),
            MenuItem::new(format!("You have {}", current)),
        ];
        if homebrew < latest {
            menu.push(MenuItem::new(format!("Homebrew has {}", homebrew)));
        }
        if homebrew > current {
            menu.push(ContentItem::new("Install using `brew reinstall --cask bitbar`").command(bitbar::Command::terminal(("brew", "reinstall", "--cask", "bitbar"))).into());
        }
        if homebrew < latest {
            menu.push(ContentItem::new("Send Pull Request to Homebrew").command(bitbar::Command::terminal(("brew", "bump-cask-pr", "--version", latest, "bitbar"))).into());
            menu.push(ContentItem::new("Open GitHub Release").href("https://github.com/matryer/bitbar/releases/latest").expect("failed to parse GitHub latest release URL").into());
            menu.push(ContentItem::new("Hide Until Homebrew Is Updated").command((current_exe.display(), "hide_until_homebrew_gt", homebrew)).into());
        }
        Menu(menu)
    } else {
        Menu::default()
    })
}
