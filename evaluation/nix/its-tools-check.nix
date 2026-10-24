# Check of the its-tools package: runs five examinations on a three-place net through the installed wrapper, as an
# ordinary user of the package (its store path is read-only here, as it is at runtime) and fails on the errors that
# a binary that cannot be made executable gives. Run: nix build .#checks.x86_64-linux.its-tools
{ runCommand, its-tools }:

runCommand "its-tools-check" { nativeBuildInputs = [ its-tools ]; } ''
  export HOME=$TMPDIR
  mkdir smoke && cp ${./its-tools-smoke.pnml} smoke/model.pnml
  for e in StateSpace ReachabilityDeadlock QuasiLiveness OneSafe StableMarking; do
    its-tools -pnfolder smoke -examination $e -its -timeout 60 > $e.log 2>&1 || true
    echo "== $e"; tail -3 $e.log
    if grep -q -E "unable to make|SEVERE" $e.log; then echo "$e: a tool binary could not be started"; grep -E "unable to make|SEVERE" $e.log | head -5; exit 1; fi
  done
  grep -q "STATE_SPACE STATES  *3 " StateSpace.log
  grep -q "FORMULA ReachabilityDeadlock FALSE" ReachabilityDeadlock.log
  grep -q "FORMULA QuasiLiveness TRUE" QuasiLiveness.log
  grep -q "FORMULA OneSafe TRUE" OneSafe.log
  grep -q "FORMULA StableMarking FALSE" StableMarking.log
  touch $out
''
