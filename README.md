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
`cargo run --release`! You need:
  - gtksourceview5 >= 5.18
  - libadwaita >= 1.8
  - gtk4 >= 4.20
  - libpanel
  - vte4 >= 0.78

## Contribute
This is a huge work-in-progress and I'm still working together core features.
That being said, I'll still try to review PRs to the best I can! Anything that helps refine the editor and make it stronger is welcome.
Only thing I ask is the same as all my projects: No "AI" -- that's the reason I made this, and fascists aren't allowed here.


### Where does the name come from?
[CHECK OUT THIS COOL ROCK](https://en.wikipedia.org/wiki/Kyanite)

## TODO

i marked optional things with a question mark

- Warn when closing about unsafed documents

- Warn when documents change while editing them

- Sidebar:
  - Create files
  - Delete files
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
