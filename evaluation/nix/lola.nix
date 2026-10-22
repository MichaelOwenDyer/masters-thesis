# LoLA 2.0 (University of Rostock), an explicit-state Petri net model checker.
# Upstream ships a pre-generated autotools build (kimwitu++/flex/bison output is
# included in the tarball), so only a C++ toolchain is needed.
{ lib, stdenv, fetchurl }:

stdenv.mkDerivation rec {
  pname = "lola";
  version = "2.0";

  # service-technology.org (the old home) is a parked domain now; the University of
  # Rostock still hosts the identical 2.0 tarball.
  src = fetchurl {
    url = "https://theo.informatik.uni-rostock.de/storages/uni-rostock/Alle_IEF/Inf_THEO/images/tools_daten/lola-${version}.tar.gz";
    hash = "sha256-ma+oO44SFkb4JGchR3cUzTBnv9vWPaGvKkdMJoL563E=";
  };

  # The 2016 sources use std::binary_function and the `register` keyword, which are
  # errors under the default C++17+ mode of current compilers; it also writes
  # "..."PRIu64 without a space (an error in clang under C++11, a warning in GCC).
  # CFLAGS -fcommon: Formula/LTL/{buchi,generalized}.c (ltl2ba) define the same globals (rank, scc_stack),
  # a duplicate-symbol link error under the -fno-common default of GCC 10+ and clang 11+.
  env.CXXFLAGS = "-std=gnu++11 -Wno-deprecated-declarations -Wno-reserved-user-defined-literal -Wno-literal-suffix";
  env.CFLAGS = "-O2 -fcommon";

  # Only the solver is needed; skip docs (texinfo) and the autotest suite.
  buildPhase = ''
    runHook preBuild
    make -C src -j$NIX_BUILD_CORES
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    make -C src install
    runHook postInstall
  '';

  enableParallelBuilding = true;

  meta = {
    description = "Explicit-state Petri net model checker (LoLA 2.0)";
    homepage = "https://theo.informatik.uni-rostock.de/theo-forschung/tools/lola/";
    license = lib.licenses.agpl3Plus;
    mainProgram = "lola";
    platforms = lib.platforms.unix;
  };
}
