# Builds the `petrivet` command-line tool from this workspace. z3 is the SMT backend and is linked dynamically.
{ lib, rustPlatform, pkg-config, z3 }:

rustPlatform.buildRustPackage {
  pname = "petrivet-cli";
  version = "0.1.0";

  src = lib.cleanSourceWith {
    src = ../.;
    filter = path: type:
      let
        name = baseNameOf path;
        rel = lib.removePrefix (toString ../. + "/") (toString path);
      in
      !(builtins.elem name [ "target" ".git" ".cargo" ".idea" ".DS_Store" "result" ])
      && !(builtins.elem rel [ "corpus/mcc/models" "corpus/mcc/oracle" ]);
  };

  cargoLock = {
    lockFile = ../Cargo.lock;
    # the lock file contains the petrivet library as a git dependency
    allowBuiltinFetchGit = true;
  };

  cargoBuildFlags = [ "-p" "petrivet-cli" ];
  cargoTestFlags = [ "-p" "petrivet-cli" ];

  nativeBuildInputs = [ pkg-config rustPlatform.bindgenHook ];
  buildInputs = [ z3 ];

  meta = {
    description = "Place/transition Petri net analyzer";
    mainProgram = "petrivet";
    license = lib.licenses.lgpl3Plus;
  };
}
