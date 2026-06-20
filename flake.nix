{
  description = "momentum scrolling for touchpads on wlroots-based wayland compositors";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      pkgsFor = system: nixpkgs.legacyPackages.${system};
    in
    {
      packages = forAllSystems (system:
        let pkgs = pkgsFor system; in rec {
          zoomies = pkgs.callPackage ./package.nix { };
          default = zoomies;
        });

      nixosModules = rec {
        zoomies = import ./module.nix self;
        default = zoomies;
      };

      devShells = forAllSystems (system:
        let pkgs = pkgsFor system; in {
          default = pkgs.mkShell {
            packages = [ pkgs.cargo pkgs.rustc pkgs.rust-analyzer pkgs.pkg-config pkgs.wayland ];
          };
        });

      formatter = forAllSystems (system: (pkgsFor system).nixfmt-rfc-style);
    };
}
