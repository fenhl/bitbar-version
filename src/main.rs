#![deny(rust_2018_idioms, unused, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]

use {
    std::{
        convert::{
            Infallible as Never,
            TryFrom,
            TryInto,
        },
        env::{
            self,
            current_exe,
        },
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
    Env(env::VarError),
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
            Error::Env(e) => menu.push(MenuItem::new(e)),
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

fn is_swiftbar() -> bool {
    env::var_os("SWIFTBAR").is_some()
}

fn running_version() -> Result<Version, Error> {
    if is_swiftbar() {
        Ok(env::var("SWIFTBAR_VERSION")?.parse()?)
    } else {
        // BitBar does not provide running version info, assume same as installed
        installed_version()
    }
}

fn installed_version() -> Result<Version, Error> {
    let plist = if is_swiftbar() {
        plist::from_file::<_, Plist>("/Applications/SwiftBar.app/Contents/Info.plist")?
    } else {
        plist::from_file::<_, Plist>("/Applications/BitBar.app/Contents/Info.plist")?
    };
    Ok(plist.bundle_version)
}

async fn homebrew_version(client: &reqwest::Client) -> Result<Version, Error> {
    if is_swiftbar() {
        // SwiftBar is not on Homebrew yet
        //TODO update once SwiftBar is on Homebrew
        Ok(Version::new(0, 0, 0))
    } else {
        Ok(client
            .get("https://formulae.brew.sh/api/cask/bitbar.json")
            .send().await?
            .error_for_status()?
            .json::<BrewCask>().await?
            .version)
    }
}

async fn latest_version(client: &reqwest::Client) -> Result<Version, Error> {
    client
        .get(if is_swiftbar() {
            "https://api.github.com/repos/swiftbar/SwiftBar/releases/latest"
        } else {
            "https://api.github.com/repos/matryer/bitbar/releases/latest"
        })
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
    let installed = installed_version()?;
    let running = running_version()?;
    Ok(if running < latest && Data::new()?.hide_until_homebrew_gt.map_or(true, |min_ver| homebrew > min_ver) {
        if installed < latest {
            let mut menu = vec![
                ContentItem::default().template_image(&include_bytes!("../assets/logo.png")[..]).never_unwrap().into(),
                MenuItem::Sep,
                MenuItem::new(format!("{} {} available", if is_swiftbar() { "SwiftBar" } else { "BitBar" }, latest)),
                MenuItem::new(format!("You have {}", running)),
            ];
            if !is_swiftbar() && homebrew < latest { //TODO also enable for SwiftBar once that is on Homebrew
                menu.push(MenuItem::new(format!("Homebrew has {}", homebrew)));
            }
            if homebrew > installed {
                menu.push(ContentItem::new("Install using `brew upgrade --cask bitbar`").command(bitbar::Command::terminal(("brew", "upgrade", "--cask", "bitbar"))).into());
            }
            if homebrew < latest {
                if !is_swiftbar() { //TODO also enable for SwiftBar once that is on Homebrew
                    menu.push(ContentItem::new("Send Pull Request to Homebrew").command(bitbar::Command::terminal(("brew", "bump-cask-pr", "--version", latest, "bitbar"))).into());
                }
                menu.push(ContentItem::new("Open GitHub Release").href("https://github.com/matryer/bitbar/releases/latest").expect("failed to parse GitHub latest release URL").into());
                if !is_swiftbar() { //TODO also enable for SwiftBar once that is on Homebrew
                    menu.push(ContentItem::new("Hide Until Homebrew Is Updated").command((current_exe.display(), "hide_until_homebrew_gt", homebrew)).into());
                }
            }
            Menu(menu)
        } else {
            Menu(vec![
                ContentItem::default().template_image(&include_bytes!("../assets/logo.png")[..]).never_unwrap().into(),
                MenuItem::Sep,
                MenuItem::new(format!("Restart to update to {} {}", if is_swiftbar() { "SwiftBar" } else { "BitBar" }, installed)),
                MenuItem::new(format!("Currently running: {}", running)),
            ])
        }
    } else {
        Menu::default()
    })
}
