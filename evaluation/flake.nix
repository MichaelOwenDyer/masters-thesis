{
  description = "Environment for the petrivet evaluation: petrivet, LoLA, TINA (tina, selt, ndrio, ...) and ITS-Tools";

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

      # Python environment of analysis/ (SPEC section 12): polars, scipy, numpy, matplotlib (SVG), pytest.
      # Versions are pinned through flake.lock (nixpkgs).
      analysisPython = p: p.python3.withPackages (ps: [ ps.polars ps.scipy ps.numpy ps.matplotlib ps.pytest ]);
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
            its-tools = pkgs.callPackage ./nix/its-tools.nix { };
          };
        };

      # `nix build .#checks.x86_64-linux.its-tools`: runs ITS-Tools as a user of the package would (see nix/its-tools-check.nix)
      checks.${system}.its-tools = pkgs.callPackage ./nix/its-tools-check.nix { its-tools = self.packages.${system}.its-tools; };

      devShells = nixpkgs.lib.recursiveUpdate
        # `nix develop .#analysis`: only the Python environment, on every system (LoLA and TINA are not needed to
        # regenerate export/). From this directory: python -m analysis --results results --truth results/truth.jsonl --out export
        (forAllSystems (p: { analysis = p.mkShell { packages = [ (analysisPython p) ]; }; }))
        {
          # The shell the evaluation runs in: petrivet, LoLA, TINA and ITS-Tools on PATH (ndrio comes with TINA), plus the Python environment.
          ${system}.default = pkgs.mkShell {
            # building petrivet-eval compiles the petrivet library, which needs Z3 and libclang (via the package's inputs)
            inputsFrom = [ self.packages.${system}.petrivet ];
            packages = [
              self.packages.${system}.petrivet
              self.packages.${system}.lola
              self.packages.${system}.tina
              self.packages.${system}.its-tools
              pkgs.cargo
              pkgs.rustc
              pkgs.clippy
              (analysisPython pkgs)
            ];
          };
        };
    };
}
