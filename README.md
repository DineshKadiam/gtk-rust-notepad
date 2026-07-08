<h1> <img height="32px" src="resources/icons/src/ca.potatoe.Kyanite.svg"/> Kyanite </h1>

![trans rights](https://img.shields.io/badge/-rights_btw-wtf?style=flat&label=trans&labelColor=5BCEFA&color=F5A9B8)

A lightweight, graphical text editor.

![](resources/screenshots/code.png)

<small>(see resources/screenshots for more screenshots)</small>


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
- rust >= 1.92.0
- gtksourceview5 >= 5.18
- libadwaita >= 1.8
- gtk4 >= 4.20
- libpanel
- vte4 >= 0.78
- blueprint-compiler
- glibc
  - it breaks all the time on chimera linux and the answer is always "broken musl implementation"

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

## Roadmap/TODO

### quality-of-life
- recent file list

### lsp integration (the horrors)
- code actions
- lsp hover
- gracefully close lsp when window closes

### keybinds
- modal by design
  - default mode is always insert
- configurable modes (e.g. you can define your own modes)
- configurable keybinds
  - conditional keybind actions (e.g. only run in a certain mode)
  - multiple binds should be possible

### plugins
- ask charlotte about this, she wants to do it and she has interesting ideas

### git integration(?)
  - maybe make this a plugin....
  - highlight changed (git) lines(?)
  - sidebar window to:
    - make git commits
    - pull
    - push
    - switch commits


- mode line

- plugins

-----

<!-- ward off tech bros, if literally everything else wasn't enough -->

# Disclaimer

By using or contributing to this software, you agree that *trans rights are human rights*, *ACAB*, and that *hate has no place here*.

<img width="256px" src="resources/screenshots/kills_fascists.png"/>
