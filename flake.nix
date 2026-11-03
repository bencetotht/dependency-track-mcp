{
  description = "Dependency-Track CLI";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          dtrack = pkgs.callPackage ./nix/package.nix { };
        in
        {
          inherit dtrack;
          default = dtrack;
        });

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.dtrack}/bin/dtrack";
          meta.description = "Dependency-Track CLI";
        };
      });

      checks = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          dtrack = self.packages.${system}.dtrack;
        in
        {
          inherit dtrack;
          cli = pkgs.runCommand "dtrack-cli-check" { } ''
            export HOME="$TMPDIR/home"
            mkdir -p "$HOME"
            ${dtrack}/bin/dtrack --version
            ${dtrack}/bin/dtrack --help > /dev/null
            ${dtrack}/bin/dtrack --output json config > "$out"
          '';
        });
    };
}
