#![deny(rust_2018_idioms, unused, unused_import_braces, unused_qualifications, warnings)]

use {
    std::convert::{
        Infallible,
        TryFrom,
        TryInto
    },
    bitbar::{
        ContentItem,
        Menu,
        MenuItem
    },
    semver::{
        SemVerError,
        Version
    },
    serde::Deserialize
};

#[derive(Debug)]
enum Error {
    InvalidPlist,
    NoLeadingV,
    Plist(plist::Error),
    Reqwest(reqwest::Error),
    SemVer(SemVerError)
}

impl From<plist::Error> for Error {
    fn from(e: plist::Error) -> Error {
        Error::Plist(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Error {
        Error::Reqwest(e)
    }
}

impl From<SemVerError> for Error {
    fn from(e: SemVerError) -> Error {
        Error::SemVer(e)
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String
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

impl<T> ResultNeverExt for Result<T, Infallible> {
    type Ok = T;

    fn never_unwrap(self) -> T {
        match self {
            Ok(x) => x,
            Err(never) => match never {}
        }
    }
}

fn bitbar() -> Result<Menu, Error> {
    let latest = latest_version()?;
    let current = current_version()?;
    Ok(if current < latest {
        Menu(vec![
            ContentItem::default().template_image(&include_bytes!("../assets/logo.png")[..]).never_unwrap().into(),
            MenuItem::Sep,
            MenuItem::new(format!("BitBar {} available", latest)),
            MenuItem::new(format!("You have {}", current)),
            ContentItem::new("Install using `brew cask reinstall bitbar`").command(bitbar::Command::terminal(["brew", "cask", "reinstall", "bitbar"])).into()
        ])
    } else {
        Menu::default()
    })
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

fn latest_version() -> Result<Version, Error> {
    reqwest::get("https://api.github.com/repos/matryer/bitbar/releases/latest")?
        .error_for_status()?
        .json::<Release>()?
        .try_into()
}

fn main() {
    match bitbar() {
        Ok(menu) => { print!("{}", menu); }
        Err(e) => {
            print!("{}", Menu(vec![
                ContentItem::new("?").template_image(&include_bytes!("../assets/logo.png")[..]).never_unwrap().into(),
                MenuItem::Sep,
                MenuItem::new(format!("{:?}", e))
            ]));
        }
    }
}
