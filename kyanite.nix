{
  lib,
  rustPlatform,
  wrapGAppsHook4,
  pkg-config,
  gtksourceview5,
  libadwaita,
  gtk4,
  libpanel,
  vte-gtk4,
  blueprint-compiler,
  glibc,
}:
rustPlatform.buildRustPackage {
  pname = "kyanite";
  version = "0.1.0";

  src = ./.;

  cargoHash = "sha256-qqv49DmNoLwoH/t5C1mm4A7+ATaOQiPU/0kSjVUo5l0=";

  nativeBuildInputs = [
    wrapGAppsHook4
    pkg-config
    blueprint-compiler
    gtk4
    glibc
  ];

  buildInputs = [
    gtksourceview5
    libadwaita
    libpanel
    vte-gtk4
  ];

  RUSTFLAGS = "-A dead_code -A unused_imports";

  postInstall = ''
    mkdir -p $out/share/icons
    cp -r resources/icons/hicolor $out/share/icons

    mkdir -p $out/share/applications
    cp resources/metadata/ca.potatoe.Kyanite.desktop $out/share/applications
  '';

  meta = {
    description = "Lightweight text editor written with GTK4 + Rust";
    homepage = "https://codeberg.org/pastthepixels/kyanite";
    license = lib.licenses.gpl3Only;
    mainProgram = "ky";

    maintainers = [
      {
        name = "breaady";
        email = "imadiamondtaco@gmail.com";
      }
    ];
  };
}
