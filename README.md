This is a [BitBar](https://getbitbar.com/) plugin that checks for available BitBar updates.

# Installation

1. [Install BitBar](https://getbitbar.com/).
    * If you have [Homebrew](https://brew.sh/), you can also install with `brew install --cask bitbar`.
2. [Install Rust](https://www.rust-lang.org/tools/install).
    * If you have Homebrew, you can also install with `brew install rust`.
3. Install the plugin:
    ```sh
    cargo install --git=https://github.com/fenhl/bitbar-version
    ```
4. Create a symlink to `~/.cargo/bin/bitbar-version` into your BitBar plugin folder. Call it something like `bitbar-version.2m.o`, where `2m` is the rate of update checks.
5. Refresh BitBar by opening a menu and pressing <kbd>⌘</kbd><kbd>R</kbd>.

# Notes

* This plugin assumes that you have installed BitBar to `/Applications/BitBar.app`.
* The plugin only actually appears in the menu bar when an update is available or if there was an error during the most recent check. When your BitBar is up to date, the plugin is completely hidden.
