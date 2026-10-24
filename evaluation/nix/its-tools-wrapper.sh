#!@bash@/bin/bash
# `its-tools`: the launcher of ITS-Tools (see nix/its-tools.nix).
#
# The tool makes its z3 executable with File.setExecutable before it starts it, which fails for a file of the
# read-only store (even if the file is executable already), and without z3 several examinations stall until the
# cap. So the tool runs from a small tree in the cache of the user: symbolic links to the store for every plugin,
# a writable copy for the plugin that holds z3. The tree is built once (in a temporary folder that is renamed, so
# that runs starting at the same time cannot see a half-built tree) and is named after this package build.
set -eu
export PATH="@path@:$PATH"
store=@store@
cache_root="${XDG_CACHE_HOME:-$HOME/.cache}/its-tools"
cache="$cache_root/@key@"
if [ ! -d "$cache/plugins" ]; then
	mkdir -p "$cache_root"
	tmp=$(mktemp -d "$cache_root/.new.XXXXXX")
	mkdir "$tmp/plugins"
	for entry in "$store"/plugins/*; do
		name=$(basename "$entry")
		case "$name" in
			fr.lip6.smt.z3.binaries_*) cp -r "$entry" "$tmp/plugins/$name"; chmod -R u+w "$tmp/plugins/$name" ;;
			*) ln -s "$entry" "$tmp/plugins/$name" ;;
		esac
	done
	ln -s "$store/plugins-lib" "$tmp/plugins-lib"
	mv -T "$tmp" "$cache" 2> /dev/null || rm -rf "$tmp"
fi
export ITSTOOLS="$cache"
export ITS_JAVA_OPTS="-Xmx@heap@ ${ITS_JAVA_OPTS:-}"
exec @bash@/bin/bash "$store/its-tools-flat.sh" "$@"
