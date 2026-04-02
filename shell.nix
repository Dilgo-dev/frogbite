{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
    cargo
    rustc
    rustfmt
    clippy
  ];

  buildInputs = with pkgs; [
    gtk3
    webkitgtk_4_1
    libsoup_3
    openssl
    glib
    cairo
    pango
    gdk-pixbuf
    atk
    librsvg
    gsettings-desktop-schemas
  ];

  shellHook = ''
    export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"
    export XDG_DATA_DIRS="$GSETTINGS_SCHEMAS_PATH:$XDG_DATA_DIRS"
  '';
}
