{ lib, rustPlatform, pkg-config, openssl, makeWrapper, git }:

let
  manifest = builtins.fromTOML (builtins.readFile ../Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = "dtrack";
  version = manifest.package.version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../src
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ pkg-config makeWrapper ];
  buildInputs = [ openssl ];

  # Use Nix's OpenSSL rather than building the vendored copy from Cargo.
  OPENSSL_NO_VENDOR = 1;

  # Repository detection invokes Git, including when run outside a user shell.
  postInstall = ''
    wrapProgram "$out/bin/dtrack" \
      --prefix PATH : ${lib.makeBinPath [ git ]}
  '';

  meta = {
    description = "CLI for querying and managing Dependency-Track projects and findings";
    homepage = "https://github.com/bencetotht/dependency-track-mcp";
    mainProgram = "dtrack";
    platforms = lib.platforms.linux;
  };
}
