This is a [BitBar](https://getbitbar.com/) plugin that checks for available BitBar updates.

# Installation

1. [Install Rust](https://www.rust-lang.org/tools/install)
2. Run `cargo install --git=https://github.com/fenhl/bitbar-version`
3. Symlink the file `~/.cargo/bin/bitbar-version` into your BitBar plugin directory. Call it something like `bitbar-version.2m.o`, where `2m` is the rate of update checks.
4. Refresh BitBar by opening a menu and pressing <kbd>⌘</kbd><kbd>R</kbd>

# Notes

* This plugin assumes that you have installed BitBar to `/Applications/BitBar.app`.
* The plugin only actually appears in the menu bar when an update is available or if there was an error during the most recent check. When your BitBar is up to date, the plugin is completely hidden.
