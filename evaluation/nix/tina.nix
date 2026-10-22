# TINA (TIme petri Net Analyzer) toolbox 4.0.0, LAAS-CNRS. Installs the upstream x86-64
# Linux binary distribution (tina, sift, selt, ndrio, struct, stan, ...) and patches it.
#
# Licence: no licence file ships in the tarball, and the download page
# (https://projects.laas.fr/tina/download.php) states none. It is free to download, but
# redistribution terms are unstated, so this is marked unfree (needs allowUnfree or a
# predicate, see flake.nix) and is not built by any binary cache.
#
# `large = true` selects the "large" build which encodes markings over 64-bit instead of
# 32-bit integers (upstream recommends it when token counts may exceed 32 bits).
{ lib, stdenv, fetchurl, autoPatchelfHook, zlib, large ? false }:

let
  variant = if large then "large-amd64-linux" else "amd64-linux";
  hashes = {
    amd64-linux = "sha256-IvC0he+grA0+ueqUhA7OG2Ru69dr1vdZMdkZI0lbC+g=";
    large-amd64-linux = "sha256-Ij+s9JN7e9BbMT6AA8QDwaodd1ZVU3ITsnDJ8WKPXA8=";
  };
in
stdenv.mkDerivation rec {
  pname = "tina" + lib.optionalString large "-large";
  version = "4.0.0";

  src = fetchurl {
    url = "https://projects.laas.fr/tina/binaries/tina-${version}-${variant}.tgz";
    hash = hashes.${variant};
  };

  nativeBuildInputs = [ autoPatchelfHook ];
  buildInputs = [ stdenv.cc.cc.lib zlib ];

  dontConfigure = true;
  dontBuild = true;

  # The bundled libgmp.so.3 / libpolka*.so live in lib/; let autoPatchelf find them.
  preFixup = ''
    addAutoPatchelfSearchPath $out/share/tina/lib
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out/share/tina $out/bin
    cp -r . $out/share/tina/
    # `dot`, `neato`, `circo` are one multi-call binary (symlinks), keep them relative.
    for f in $out/share/tina/bin/*; do
      [ -f "$f" ] && [ -x "$f" ] && ln -s "$f" "$out/bin/$(basename "$f")"
    done
    mkdir -p $out/share/man
    ln -s $out/share/tina/doc/man/mann $out/share/man/mann
    runHook postInstall
  '';

  meta = {
    description = "Time Petri net analysis toolbox (tina, sift, selt, ndrio, ...), prebuilt binaries";
    homepage = "https://projects.laas.fr/tina/";
    license = lib.licenses.unfree;
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
    mainProgram = "tina";
    platforms = [ "x86_64-linux" ];
  };
}
