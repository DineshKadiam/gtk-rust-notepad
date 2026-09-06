use std::cell::RefCell;
use std::fs::DirEntry;
use std::path::Path;
use std::sync::Arc;
use std::{fs, io};

#[cfg(not(debug_assertions))]
const GRESOURCES_TEMPLATE: &'static str = r#"<?xml version="1.0" encoding="UTF-8"?>
    <gresources>
      <gresource prefix="/ca/potatoe/Kyanite/">
        {}
      </gresource>
    </gresources>
"#;
#[cfg(debug_assertions)]
const GRESOURCES_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
    <gresources>
      <gresource prefix="/ca/potatoe/Kyanite/Devel/">
        {}
      </gresource>
    </gresources>
"#;

const GRESOURCES_FILE_TEMPLATE: &str = r#"
    <file compressed="true" preprocess="xml-stripblanks">{}</file>
"#;

fn main() {
    let out = std::env::var("OUT_DIR").unwrap();
    // 1. Build ca.potatoe.Kyanite.gresources.xml. Include resources
    let strings = Arc::new(RefCell::new(vec![]));
    // 1.a. Walk directory for icons
    visit_dirs(Path::new("./resources/icons"), &|entry: &DirEntry| {
        let path = entry
            .path()
            .to_string_lossy()
            .to_string()
            .replace("./resources/", "");
        if !path.contains("README.md") {
            strings
                .borrow_mut()
                .push(GRESOURCES_FILE_TEMPLATE.replace("{}", &path));
        }
    })
    .expect("cargo::error=cannot find icons");
    // 1.b. Add strings to the template
    let _ = std::fs::create_dir_all(out.clone() + "/gresource");
    std::fs::write(
        out.clone() + "/gresource/ca.potatoe.Kyanite.gresource.xml",
        GRESOURCES_TEMPLATE.replace("{}", &strings.borrow().join("\n")),
    )
    .expect("cargo::error=cannot write to gresource xml");
    // 2. Compile gresource
    let output = std::process::Command::new("glib-compile-resources").args(
        format!("--sourcedir ./resources --target {out}/gresource/ca.potatoe.Kyanite.gresource {out}/gresource/ca.potatoe.Kyanite.gresource.xml").split(" ")
    ).output().expect("cargo::error=build failed: could not compile gresource");
    // If there's output...
    if !output.stdout.is_empty() {
        println!(
            "cargo::warning=Finished compiling gresource with output: {:?}",
            output.stdout
        );
    }
    // If there's output to stderr... (in which case something has gone horribly wrong)
    if !output.stdout.is_empty() {
        println!(
            "cargo::error=Finished compiling gresource with error(s): {:?}",
            output.stderr
        );
    }
}

// docs.rs boilerplate my beloved <333333333
fn visit_dirs(dir: &Path, cb: &dyn Fn(&DirEntry)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry);
            }
        }
    }
    Ok(())
}
