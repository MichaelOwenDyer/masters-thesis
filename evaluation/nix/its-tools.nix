# ITS-Tools (LIP6, Yann Thierry-Mieg), a symbolic Petri net model checker (hierarchical set decision diagrams),
# a contest participant. Installs the upstream x86-64 Linux product (an Eclipse application, GPL-3.0) and runs it
# through the product's own "flat" launcher: Java 21 on a plain class path, no Equinox.
#
# The download has no version number and its URL is not versioned, so the content is pinned by the hash only
# (product of 2026-09-11); a changed file makes the build fail instead of silently measuring another tool.
#
# What this package changes against the upstream product, and why:
#  * the bundled z3 is linked against the FHS loader, which NixOS does not have: it is replaced by z3 of nixpkgs
#    (the tool starts z3 for the SMT-based reductions of its default pipeline);
#  * the launcher script starts with `#! /bin/bash` (NixOS has no /bin/bash): patchShebangs;
#  * the launcher unpacks the jars nested in plugins into plugins-lib/ on its first run, which a read-only store
#    cannot do: it is run once at build time;
#  * the tool calls `killall` by name after a run, to stop its engine processes; `killall` is not installed and,
#    with several runs side by side, would also stop the engines of the other runs. A no-op stands in for it: the
#    runner of the evaluation stops the whole process group of every run itself;
#  * the heap: upstream starts the JVM with -Xmx16384m, above the 8 GiB limit of the measured runs; the JVM and the
#    engine processes it starts both count against that limit, so the JVM gets `heap` (default 3g).
# The other binaries of the product (its-reach, its-ctl, its-ltl, PetriSpot, spot) are statically linked. The LTSmin
# binaries are linked against the FHS loader and are not used (the evaluation runs the `-its` engine only).
{ lib, stdenv, fetchurl, unzip, makeWrapper, bash, coreutils, jdk21_headless, z3, heap ? "3g" }:

stdenv.mkDerivation {
  pname = "its-tools";
  version = "2026-09-11";

  src = fetchurl {
    url = "https://lip6.github.io/ITSTools/fr.lip6.move.gal.itscl.product-linux.gtk.x86_64.zip";
    hash = "sha256-+u80SmQcoZDjTu3ucaXMRKxwUQA4QaQx2k7f3wu3rl8=";
  };

  nativeBuildInputs = [ unzip makeWrapper jdk21_headless ];

  dontConfigure = true;
  dontBuild = true;
  # leave the vendor binaries as they are
  dontStrip = true;
  dontPatchELF = true;

  unpackPhase = ''
    runHook preUnpack
    mkdir product
    unzip -q $src -d product
    runHook postUnpack
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out/share/its-tools $out/bin $out/libexec/its-tools/bin
    cp -r product/. $out/share/its-tools/
    chmod -R u+w $out/share/its-tools

    for z in $out/share/its-tools/plugins/fr.lip6.smt.z3.binaries_*/bin/z3-linux64; do
      rm "$z"
      ln -s ${z3}/bin/z3 "$z"
    done

    patchShebangs $out/share/its-tools/its-tools-flat.sh

    printf '#!${bash}/bin/bash\nexit 0\n' > $out/libexec/its-tools/bin/killall
    chmod +x $out/libexec/its-tools/bin/killall

    # first run of the launcher (no arguments: it only complains about the missing input) creates plugins-lib/
    PATH=${lib.makeBinPath [ jdk21_headless unzip coreutils ]}:$PATH ITSTOOLS=$out/share/its-tools \
      $out/share/its-tools/its-tools-flat.sh || true
    test -d $out/share/its-tools/plugins-lib

    makeWrapper ${bash}/bin/bash $out/bin/its-tools \
      --add-flags "$out/share/its-tools/its-tools-flat.sh" \
      --prefix PATH : ${lib.makeBinPath [ jdk21_headless unzip coreutils ]}:$out/libexec/its-tools/bin \
      --set ITSTOOLS $out/share/its-tools \
      --set ITS_JAVA_OPTS "-Xmx${heap}"
    runHook postInstall
  '';

  meta = {
    description = "Symbolic Petri net model checker (ITS-Tools, LIP6), upstream binary product run on a flat class path";
    homepage = "https://lip6.github.io/ITSTools-web/";
    license = lib.licenses.gpl3Only;
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode lib.sourceTypes.binaryBytecode ];
    mainProgram = "its-tools";
    platforms = [ "x86_64-linux" ];
  };
}
