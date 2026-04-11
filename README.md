oh my god can everything stop being slop please for the love of god i make *games*

# spikedit

my editor. test. i don't know if this will go anywhere. slapping together gtk things
a lot of editors that i use are either slow as fuck or on thin ice so i'm making this as an escape hatch in case something happens. also i want a lightweight editor

### why?
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

### why gtk?
- it's cross platform
- it's what i know
- it works

### why NOT gtk
- it looks clunky

## todo

- highlight when tabs are unsaved
- search with rust regex (instead of whatever gtk has. if possible idk how much is ingrained to gtksourceview that i can't do myself)
- tab right click actions
- terminal that opens from the bottom

- lsp integration
  - code actions
- snippets
- inlay hints
- underline errors

- helix keybinds (configurable)
- plugins

- settings page
