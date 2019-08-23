#![deny(unused, unused_qualifications)]
#![deny(rust_2018_idioms)] // this badly-named lint actually produces errors when Rust 2015 idioms are used
#![forbid(unused_import_braces)]

use std::convert::{
    TryFrom,
    TryInto
};
use bitbar::{
    ContentItem,
    Menu,
    MenuItem
};
use semver::{
    SemVerError,
    Version
};
use serde_derive::Deserialize;

const BITBAR_LOGO: &str = "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";

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

fn bitbar() -> Result<Menu, Error> {
    let latest = latest_version()?;
    let current = current_version()?;
    Ok(if current < latest {
        Menu(vec![
            ContentItem::default().template_image(BITBAR_LOGO).into(),
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
                ContentItem::new("?").template_image(BITBAR_LOGO).into(),
                MenuItem::Sep,
                MenuItem::new(format!("{:?}", e))
            ]));
        }
    }
}
