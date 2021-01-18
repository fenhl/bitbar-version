This is a [BitBar](https://getbitbar.com/)/[SwiftBar](https://swiftbar.app/) plugin that checks for available BitBar/SwiftBar updates.

# Installation

1. Install [BitBar](https://getbitbar.com/) or [SwiftBar](https://swiftbar.app/).
    * If you have [Homebrew](https://brew.sh/), you can also install BitBar with `brew install --cask bitbar`.
2. [Install Rust](https://www.rust-lang.org/tools/install).
    * If you have Homebrew, you can also install Rust with `brew install rust`.
3. Install the plugin:
    ```sh
    cargo install --git=https://github.com/fenhl/bitbar-version --branch=main
    ```
4. Create a symlink to `~/.cargo/bin/bitbar-version` into your BitBar/SwiftBar plugin folder. Call it something like `bitbar-version.2m.o`, where `2m` is the rate of update checks.
5. Refresh BitBar/SwiftBar by opening a menu and pressing <kbd>⌘</kbd><kbd>R</kbd>.

# Notes

* This plugin assumes that you have installed BitBar to `/Applications/BitBar.app`, or SwiftBar to `/Applications/SwiftBar.app`.
* The plugin only actually appears in the menu bar when an update is available or if there was an error during the most recent check. When your BitBar/SwiftBar is up to date, the plugin is completely hidden.
