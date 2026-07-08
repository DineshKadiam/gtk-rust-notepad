{
  fetchFromCodeberg,
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
    mainProgram = "ky";
  };
}
