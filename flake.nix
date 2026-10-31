{
  description = "Typst environment for petrivet masters thesis";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "aarch64-darwin" "x86_64-darwin" "x86_64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f (import nixpkgs {
        inherit system;
        # corefonts (Times New Roman, Arial, Courier New) is unfree but redistributable
        config.allowUnfreePredicate = pkg: nixpkgs.lib.getName pkg == "corefonts";
      }));

      # typst with the @preview packages the thesis imports, so the build needs no network
      typstFor = pkgs: pkgs.typst.withPackages (p: [ p.fletcher_0_5_8 p.lovelace_0_3_0 p.oxifmt_0_2_1 ]);
      fontsFor = pkgs: [ pkgs.corefonts pkgs.tex-gyre.termes pkgs.tex-gyre-math.termes ];
      fontPathsFor = pkgs: pkgs.lib.concatMapStringsSep ":" (f: "${f}/share/fonts") (fontsFor pkgs);
    in
    {
      # `nix build`: compiles main.typ to result/thesis.pdf
      packages = forAllSystems (pkgs: {
        default = pkgs.stdenvNoCC.mkDerivation {
          name = "thesis.pdf";
          src = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions [
              ./main.typ
              ./thesis.typ
              ./content
              ./lib
              ./figures
              ./evaluation/export
            ];
          };
          nativeBuildInputs = [ (typstFor pkgs) ];
          buildPhase = ''
            runHook preBuild
            export TYPST_FONT_PATHS="${fontPathsFor pkgs}"
            # only the fonts above, so that the PDF is the same on every machine
            typst compile --ignore-system-fonts main.typ thesis.pdf
            runHook postBuild
          '';
          installPhase = ''
            runHook preInstall
            mkdir -p $out
            cp thesis.pdf $out/
            runHook postInstall
          '';
        };
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [ (typstFor pkgs) ] ++ fontsFor pkgs;
          shellHook = ''
            export TYPST_FONT_PATHS="${fontPathsFor pkgs}"
            mkdir -p target
            echo "Typst thesis shell ready."
            echo "  nix build   # compile to result/thesis.pdf"
            echo "  make watch  # live reload to target/thesis.pdf"
          '';
        };
      });
    };
}
