#![deny(rust_2018_idioms, unused, unused_crate_dependencies, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]
#![forbid(unsafe_code)]

use {
    std::{
        env,
        fs::File,
        io::{
            self,
            prelude::*,
        },
        path::Path,
    },
    derive_more::From,
    git2::Repository,
};

#[derive(Debug, From)]
enum Error {
    Env(env::VarError),
    Git(git2::Error),
    Io(io::Error),
}

fn main() -> Result<(), Error> {
    println!("cargo:rerun-if-changed=nonexistent.foo"); // check a nonexistent file to make sure build script is always run (see https://github.com/rust-lang/cargo/issues/4213 and https://github.com/rust-lang/cargo/issues/3404)
    let mut f = File::create(Path::new(&env::var("OUT_DIR")?).join("version.rs"))?;
    writeln!(f, "/// The hash of the current commit of the bitbar-version repo at compile time.")?;
    writeln!(f, "pub(crate) const GIT_COMMIT_HASH: &str = \"{}\";", Repository::open_from_env()?.head()?.peel_to_commit()?.id())?;
    Ok(())
}
