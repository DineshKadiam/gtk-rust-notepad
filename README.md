# Kyanite

![](resources/icons/ca.potatoe.Kyanite.svg) 

![](resources/screenshots/code.png)

(see resources/screenshots for more screenshots)

A lightweight, graphical text editor.

## Features
- File tree!
- VTE (virtual terminal library) support - aka built-in terminal windows!
- Searching text!
- LSP support:
  - Configurable language servers!
  - Inline diagnostics!
  - Code completion!
- gtksourceview5 goodies:
  - Syntax highlighting!
  - Themes!
    - Window recoloring to match themes (like GNOME Builder)!
- Adaptive UI (aka works on phones)!

## Build/run

### Dependencies (install these first)
- rust >= (I don't know to be honest, I have 1.95 installed and my Cargo.toml's `edition` tag says 2024.)
- gtksourceview5 >= 5.18
- libadwaita >= 1.8
- gtk4 >= 4.20
- libpanel
- vte4 >= 0.78
- blueprint-compiler
- Filesystem where files have forward instead of backward slashes
  - yeah I programmed file i/o like this and left it becuase I hate Windows and DOS..

On Arch Linux, that looks like: `pacman -S rust gtksourceview5 libadwaita gtk4 libpanel vte4 blueprint-compiler`

### Running
- Release build: `cargo run --release`
- Debug build: `cargo run`

### Installing
Running `cargo install --path .` will build Kyanite and install the editor command, `ky`, to your user directory (`~/.cargo/bin`).  

If you're on Linux, copy the icons at `resources/icons` to `~/.local/share/icons/`, and `resources/metadata/ca.potatoe.Kyanite.desktop` to `~/.local/share/applications/`.
The latter sets up a shortcut for Kyanite that's locally installed in your home directory (you can remove it any time).

## Contribute
This is a huge work-in-progress and I'm still working together core features.
That being said, I'll still try to review PRs to the best I can! Anything that helps refine the editor and make it stronger is welcome.
Only thing I ask is the same as all my projects: No "AI" -- that's the reason I made this, and fascists aren't allowed here.


### Where does the name come from?
[CHECK OUT THIS COOL ROCK](https://en.wikipedia.org/wiki/Kyanite)

## TODO

i marked optional things with a question mark

- Warn when closing about unsaved documents

- Warn when documents change while editing them

- Format on save

- Sidebar:
  - Rename files

- lsp integration:
  - code actions(?)

- git integration:
  - highlight changed (git) lines(?)
  - sidebar window to:
    - make git commits
    - pull
    - push
    - switch commits

- helix keybinds (configurable)
- mode line

- plugins

-----

# Disclaimer

By using or contributing to this software, you agree that *trans rights are human rights*, *ACAB*, and that *hate has no place here*.

![](resources/screenshots/kills_fascists.png)
