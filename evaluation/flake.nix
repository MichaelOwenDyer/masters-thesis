{
  description = "Environment for the petrivet evaluation: petrivet, LoLA and TINA (tina, selt, ndrio, ...)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # TINA ships only as an x86-64 Linux binary and has no stated redistribution licence,
      # so it is unfree. Allow exactly that one package here so users of this flake do not need
      # NIXPKGS_ALLOW_UNFREE or a global allowUnfree.
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        config.allowUnfreePredicate = p: nixpkgs.lib.hasPrefix "tina" (nixpkgs.lib.getName p);
      };
    in
    {
      packages = nixpkgs.lib.recursiveUpdate
        (forAllSystems (pkgs: {
          petrivet = pkgs.callPackage ./nix/petrivet-cli.nix { };
          lola = pkgs.callPackage ./nix/lola.nix { };
        }))
        {
          ${system} = {
            tina = pkgs.callPackage ./nix/tina.nix { };
            tina-large = pkgs.callPackage ./nix/tina.nix { large = true; };
          };
        };

      # The shell the evaluation runs in: petrivet, LoLA and TINA on PATH (ndrio comes with TINA).
      devShells.${system}.default = pkgs.mkShell {
        # building petrivet-eval compiles the petrivet library, which needs Z3 and libclang (via the package's inputs)
        inputsFrom = [ self.packages.${system}.petrivet ];
        packages = [
          self.packages.${system}.petrivet
          self.packages.${system}.lola
          self.packages.${system}.tina
          pkgs.cargo
          pkgs.rustc
          pkgs.clippy
        ];
      };
    };
}
