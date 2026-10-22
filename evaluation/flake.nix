{
  description = "Environment for the petrivet evaluation: petrivet, LoLA and TINA (tina, selt, ndrio, ...)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  # petrivet has a flake of its own; its CLI package is what the evaluation runs.
  inputs.petrivet.url = "github:MichaelOwenDyer/petrivet";
  inputs.petrivet.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, nixpkgs, petrivet }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # TINA ships only as an x86-64 Linux binary and has no stated redistribution licence,
      # so it is unfree. Allow exactly that one package here so users of this flake do not need
      # NIXPKGS_ALLOW_UNFREE or a global allowUnfree.
      evalSystem = "x86_64-linux";
      unfreePkgs = import nixpkgs {
        system = evalSystem;
        config.allowUnfreePredicate = p: nixpkgs.lib.hasPrefix "tina" (nixpkgs.lib.getName p);
      };
    in
    {
      packages = nixpkgs.lib.recursiveUpdate
        (forAllSystems (pkgs: {
          petrivet = petrivet.packages.${pkgs.system}.default;
          lola = pkgs.callPackage ./nix/lola.nix { };
        }))
        {
          ${evalSystem} = {
            tina = unfreePkgs.callPackage ./nix/tina.nix { };
            tina-large = unfreePkgs.callPackage ./nix/tina.nix { large = true; };
          };
        };

      # The shell the evaluation runs in: petrivet, LoLA and TINA on PATH (ndrio comes with TINA).
      devShells.${evalSystem}.default = unfreePkgs.mkShell {
        # not `with self.packages...`: the flake input named `petrivet` would shadow the package
        packages = [
          self.packages.${evalSystem}.petrivet
          self.packages.${evalSystem}.lola
          self.packages.${evalSystem}.tina
        ];
      };
    };
}
