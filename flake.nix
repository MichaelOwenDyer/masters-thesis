{
  description = "Typst environment for petrivet masters thesis";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "aarch64-darwin" "x86_64-darwin" "x86_64-linux" ];
    in
    {
      devShells = nixpkgs.lib.genAttrs supportedSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              typst
              texliveBasic
              tex-gyre.termes
            ];
            shellHook = ''
              export TYPST_FONT_PATHS="${pkgs.tex-gyre.termes}/share/fonts"
              mkdir -p target
              echo "Typst thesis shell ready."
              echo "  make        # compile to target/thesis.pdf"
              echo "  make watch  # live reload"
            '';
          };
        });
    };
}
