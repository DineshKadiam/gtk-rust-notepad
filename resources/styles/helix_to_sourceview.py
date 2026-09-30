import argparse
import os
import sys

import tomllib
from colour import Color

# TODO: we should lighten the selection bg a substantial amount
# TODO: copy styles over, not just colors


# Function -- Fill in palette
def fill_palette(hx):
    def _fill_palette(n, find, replace):
        if isinstance(n, dict):
            for key, val in n.items():
                n[key] = _fill_palette(val, find, replace)
        if isinstance(n, str) and n == find:
            return replace
        else:
            return n

    for key, val in hx["palette"].items():
        for i in hx:
            if i != "palette":
                if isinstance(hx[i], str) and hx[i] == key:
                    hx[i] = {"fg": val}
                else:
                    hx[i] = _fill_palette(hx[i], key, val)
    return hx


def compat(hx):
    if "ui.cursorline.primary" not in hx:
        hx["ui.cursorline.primary"] = hx["ui.cursorline"]
    if "markup.link.url" not in hx:
        hx["markup.link.url"] = hx["markup.link.text"]
    if "ui.gutter" not in hx:
        hx["ui.gutter"] = {"bg": hx["ui.background"]["bg"]}
    if "ui.selection.primary" not in hx:
        hx["ui.selection.primary"] = hx["ui.selection"]
    if "constant.builtin" not in hx:
        hx["constant.builtin"] = hx["constant"]
    if "constant.builtin.boolean" not in hx:
        hx["constant.builtin.boolean"] = hx["constant.builtin"]
    if "markup.raw.inline" not in hx:
        hx["markup.raw.inline"] = hx["markup.raw"]
    if "ui.virtual.whitespace" not in hx:
        hx["ui.virtual.whitespace"] = hx["ui.virtual"]

    if "bg" not in hx["ui.gutter"]:
        hx["ui.gutter"]["bg"] = hx["ui.background"]["bg"]
    if "bg" not in hx["ui.cursor.match"]:
        hx["ui.cursor.match"]["bg"] = hx["ui.background"]["bg"]
    if "bg" not in hx["ui.linenr"]:
        hx["ui.linenr"]["bg"] = hx["ui.gutter"]["bg"]
    if "bg" not in hx["ui.linenr.selected"]:
        hx["ui.linenr.selected"]["bg"] = hx["ui.cursorline.primary"]["bg"]

    if "fg" not in hx["ui.cursor.match"]:
        hx["ui.cursor.match"]["fg"] = hx["ui.text"]["fg"]
    if "fg" not in hx["markup.bold"]:
        hx["markup.bold"]["fg"] = hx["ui.text"]["fg"]
    if "fg" not in hx["markup.link.url"]:
        hx["markup.link.url"]["fg"] = hx["ui.text"]["fg"]

    # Brighten primary selection
    sel = Color(hx["ui.selection.primary"]["bg"])
    sel.luminance += 0.1
    sel.saturation += 0.05
    hx["ui.selection.primary"]["bg"] = sel.hex
    return hx


# Load Helix UI theme
parser = argparse.ArgumentParser()
parser.add_argument("input", help="Path to the Helix theme file", type=str)
parser.add_argument(
    "-v",
    "--variant",
    help="Theme variant; can be `dark` or `light`",
    type=str,
    default="dark",
)
args = parser.parse_args()

hx = tomllib.load(open(args.input, "rb"))

if "inherits" in hx:
    inherit = tomllib.load(open(hx["inherits"] + ".toml", "rb"))
    inherit.update(hx)
    hx = inherit

hx = compat(fill_palette(hx))
id = os.path.basename(sys.argv[1]).replace(".toml", "")
variant = args.variant

print(f"""<?xml version="1.0" encoding="UTF-8"?>
<style-scheme id="{id}" _name="{id} (Helix theme)" version="1.0">
  <_description></_description>

  <metadata>
    <property name="variant">{variant}</property>
  </metadata>

  <!-- Global Styles -->
  {f'''<style name="background-pattern"      background="{hx["ui.virtual"]["fg"]}"/>''' if "ui.virtual" in hx else ""}
  <style name="bracket-match"           background="{hx["ui.cursor.match"]["bg"]}" foreground="{hx["ui.cursor.match"]["fg"]}" italic="false" strikethrough="false"/>
  <style name="current-line"            background="{hx["ui.cursorline.primary"]["bg"]}"/>
  <style name="current-line-number"     background="{hx["ui.linenr.selected"]["bg"]}" foreground="{hx["ui.linenr.selected"]["fg"]}" bold="false"/>
  <style name="cursor"                  foreground="{hx["ui.cursor"]["fg"]}"/>
  <style name="draw-spaces"             foreground="{hx["ui.virtual.whitespace"]["fg"]}"/>
  <style name="line-numbers"            background="{hx["ui.linenr"]["bg"]}" foreground="{hx["ui.linenr"]["fg"]}"/>
  <style name="right-margin"            foreground="{hx["ui.gutter"]["bg"]}"/>
  <style name="search-match"            background="{hx["ui.selection"]["bg"]}"/>
  <style name="selection"               background="{hx["ui.selection.primary"]["bg"]}"/>
  <style name="text"                    background="{hx["ui.background"]["bg"]}" foreground="{hx["ui.text"]["fg"]}"/>

  <!-- Defaults -->
  <style name="def:base-n-integer"      foreground="{hx["constant.numeric"]["fg"]}"/>
  <style name="def:boolean"             foreground="{hx["constant.builtin.boolean"]["fg"]}"/>
  <style name="def:builtin"             foreground="{hx["constant.builtin"]["fg"]}"/>
  <style name="def:character"           foreground="{hx["constant.character.escape"]["fg"]}"/>
  <style name="def:comment"             foreground="{hx["comment"]["fg"]}" italic="true"/>
  <style name="def:complex"             foreground="{hx["constant.numeric"]["fg"]}"/>
  <style name="def:constant"            foreground="{hx["constant"]["fg"]}"/>
  <style name="def:decimal"             foreground="{hx["constant.numeric"]["fg"]}"/>
  <style name="def:deletion"            foreground="{hx["constant.numeric"]["fg"]}"/>
  <style name="def:doc-comment"         foreground="{hx["comment"]["fg"]}" bold="false" italic="true"/>
  <style name="def:doc-comment-element" bold="true"/>
  <style name="def:emphasis"            bold="false" italic="true"/>
  <style name="def:floating-point"      foreground="{hx["constant.numeric"]["fg"]}"/>
  <style name="def:function"            foreground="{hx["function"]["fg"]}"/>
  <style name="def:keyword"             foreground="{hx["keyword"]["fg"]}" bold="true"/>
  <style name="def:number"              foreground="{hx["constant.numeric"]["fg"]}"/>
  <style name="def:operator"            foreground="{hx["operator"]["fg"]}"/>
  <style name="def:preprocessor"        foreground="{hx["function.macro"]["fg"]}" bold="true"/>
  <style name="def:special-char"        foreground="{hx["constant.character.escape"]["fg"]}"/>
  <style name="def:special-constant"    foreground="{hx["constant"]["fg"]}"/>
  <style name="def:string"              foreground="{hx["string"]["fg"]}"/>
  <style name="def:type"                foreground="{hx["type"]["fg"]}" bold="true"/>
  <!-- <style name="def:identifier"          foreground="#DBAD49"/> --> <!-- TODO what is this????? -->
  <!-- <style name="def:insertion"           foreground="#A3BE8C"/> --> <!-- TODO what is this ???? -->

  <!-- Info/error kinds -->
  <style name="def:note"                foreground="{hx["info"]["fg"]}"/>
  <style name="def:warning"             foreground="{hx["warning"]["fg"]}"/>
  <style name="def:error"               foreground="{hx["error"]["fg"]}" underline="error"/>

  <!-- Markup -->
  <style name="def:heading"             foreground="{hx["markup.heading"]["fg"]}" bold="true"/>
  <style name="def:heading0"            scale="1.2"/>
  <style name="def:strong-emphasis"     foreground="{hx["markup.bold"]["fg"]}" bold="true"/>
  <style name="def:inline-code"         foreground="{hx["markup.raw.inline"]["fg"]}"/>
  <style name="def:link-destination"    foreground="{hx["markup.link.url"]["fg"]}"/>
  <style name="def:link-symbol"         foreground="{hx["markup.link.text"]["fg"]}"/>
  <style name="def:link-text"           foreground="{hx["markup.link.text"]["fg"]}"/>
  <style name="def:list-marker"         foreground="{hx["markup.list"]["fg"]}" bold="true"/>
  <style name="def:net-address"         foreground="{hx["markup.link.url"]["fg"]}"/>
</style-scheme>
""")
