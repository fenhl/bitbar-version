#![deny(rust_2018_idioms, unused, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]

use {
    std::convert::{
        Infallible as Never,
        TryFrom,
        TryInto,
    },
    bitbar::{
        ContentItem,
        Menu,
        MenuItem,
    },
    derive_more::From,
    semver::{
        SemVerError,
        Version,
    },
    serde::Deserialize,
};

#[derive(Debug, From)]
enum Error {
    InvalidPlist,
    NoLeadingV,
    Plist(plist::Error),
    Reqwest(reqwest::Error),
    SemVer(SemVerError),
}

impl From<Error> for Menu {
    fn from(e: Error) -> Menu {
        let mut menu = Vec::default();
        match e {
            Error::InvalidPlist => menu.push(MenuItem::new("failed to read current BitBar version")),
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
    let plist = plist::Value::from_file("/Applications/BitBar.app/Contents/Info.plist")?;
    Ok(
        plist.as_dictionary().ok_or(Error::InvalidPlist)?
            .get("CFBundleVersion").ok_or(Error::InvalidPlist)?
            .as_string().ok_or(Error::InvalidPlist)?
            .parse()?
    )
}

async fn latest_version() -> Result<Version, Error> {
    reqwest::Client::builder()
        .user_agent(concat!("fenhl/bitbar-version/", env!("CARGO_PKG_VERSION")))
        .build()?
        .get("https://api.github.com/repos/matryer/bitbar/releases/latest")
        .send().await?
        .error_for_status()?
        .json::<Release>().await?
        .try_into()
}

#[bitbar::main(error_template_image = "../assets/logo.png")]
async fn main() -> Result<Menu, Error> {
    let latest = latest_version().await?;
    let current = current_version()?;
    Ok(if current < latest {
        Menu(vec![
            ContentItem::default().template_image(&include_bytes!("../assets/logo.png")[..]).never_unwrap().into(),
            MenuItem::Sep,
            MenuItem::new(format!("BitBar {} available", latest)),
            MenuItem::new(format!("You have {}", current)),
            ContentItem::new("Install using `brew reinstall --cask bitbar`").command(bitbar::Command::terminal(["brew", "reinstall", "--cask", "bitbar"])).into()
        ])
    } else {
        Menu::default()
    })
}
