# Calliope's everyday commands, from a POSIX shell (Git Bash, WSL, macOS,
# Linux).
#
#   make update   nsh to its latest nightly: the CLI, the nsh crates' `rev`
#                 in Cargo.toml (which CI builds with too), and every other
#                 dependency within Cargo.toml's ranges
#   make run      the app on this machine's desktop

# Windows' own tools where the shell reaches them (WSL, Git Bash), else the
# shell's: `make CARGO=cargo` chooses.
CARGO ?= $(if $(shell command -v cargo.exe 2>/dev/null),cargo.exe,cargo)
NSH ?= $(if $(shell command -v nsh.exe 2>/dev/null),nsh.exe,nsh)

.PHONY: update run

# The CLI first, then the crates at the commit it was built from (its
# `nsh --version`, resolved to the full hash on GitHub): CI installs the
# same nightly, so its runners and the crates agree.
update:
	$(NSH) update nightly
	@short=$$($(NSH) --version | sed -E 's/.*\(([0-9a-f]+) nightly.*/\1/'); \
	sha=$$(curl -fsSL "https://api.github.com/repos/longbridge/nsh/commits/$$short" \
		| awk -F'"' '/"sha"/ && !found { print $$4; found = 1 }'); \
	[ $${#sha} -eq 40 ] || { echo "cannot find nsh's commit $$short on GitHub" >&2; exit 1; }; \
	sed -E "s|(github.com/longbridge/nsh\", rev = \")[0-9a-f]+|\1$$sha|" Cargo.toml > Cargo.toml.new \
		&& mv Cargo.toml.new Cargo.toml; \
	echo "nsh crates at $$sha"
	$(CARGO) update

run:
	$(CARGO) run -p calliope
