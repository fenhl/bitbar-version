This is a [SwiftBar](https://swiftbar.app/) plugin that checks for available SwiftBar updates.

# Installation

1. Install [SwiftBar](https://swiftbar.app/).
    * If you have [Homebrew](https://brew.sh/), you can also install with `brew install --cask swiftbar`.
2. [Install Rust](https://www.rust-lang.org/tools/install).
    * If you have Homebrew, you can also install with `brew install rust`.
3. Install the plugin:
    ```sh
    cargo install --git=https://github.com/fenhl/bitbar-version --branch=main
    ```
4. Create a symlink to `~/.cargo/bin/bitbar-version` into your SwiftBar plugin folder. Name it something like `bitbar-version.2m.o`, where `2m` is the rate of update checks.
5. This plugin will also notify about available self-updates. To make the “Update Via Cargo” menu item work, install the updater:
    ```sh
    cargo install cargo-update
    ```

# Notes

* This plugin assumes that you have installed SwiftBar to `/Applications/SwiftBar.app`.
* The plugin only actually appears in the menu bar when an update is available or if there was an error during the most recent check. When your SwiftBar is up to date, the plugin is completely hidden.
* If a newer version has been released but is not yet in Homebrew, you will get the option to send a pull request to update Homebrew, or to hide the plugin until Homebrew is updated.

# Configuration

You can optionally configure the behavior of the plugin by creating a [JSON](https://json.org/) file at `bitbar/plugins/bitbar-version.json` inside an [XDG](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html) config directory. All entries are optional:

* `githubToken`: A [GitHub personal access token](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens) that will be used to check GitHub for the latest versions of SwiftBar and this plugin. May be used to avoid “API rate limit exceeded” errors, since GitHub grants authenticated requests a higher rate limit. The token does not require any scopes.
