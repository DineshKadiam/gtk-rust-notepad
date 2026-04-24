# spikedit

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
- gtksourceview5 goodies:
  - Syntax highlighting!
  - Themes!
    - Window recoloring to match themes (like GNOME Builder)!
- Adaptive UI (aka works on phones)!

## Contribute
This is a huge work-in-progress and I'm still working together core features.
That being said, I'll still try to review PRs to the best I can! Anything that helps refine the editor and make it stronger is welcome.
Only thing I ask is the same as all my projects: No "AI" -- that's the reason I made this, and fascists aren't allowed here.

## Why?
- emacs: slow, every package is slop
- vim: slow, terminal uis are slow as hell
  - also slop
    - there is evi and vim-classic
- zed: ai slop
  - gram: fork that removes ai slop! but it retains the sloppified *codebase* since it's a fork
  - zed is fucking slow and clunky and almost all extensions are slop
- helix: slop :(
  - also it's a great text editor but it doesn't have a nice little "run" button and that makes me sad
  - also terminal uis suck
- gnome builder: WONDERFUL but it's designed to be a *gnome* thing so much that i can't use it as a general purpose IDE :(
- kate: i'm trying it out. it could be nice? it breaks in fascinating ways on niri

## TODO

i marked optional things with a question mark

- libpanel omnibar(?)

- lsp integration:
  - code actions(?)
  - code completion(?)

- git integration:
  - highlight changed (git) lines(?)
  - sidebar window to:
    - make git commits
    - pull
    - push
    - switch commits

- filter through sidebar files

- helix keybinds (configurable)
- mode line

- plugins

-----

# Disclaimer

By using or contributing to this software, you agree that *trans rights are human rights*, *ACAB*, and that *hate has no place here*.

