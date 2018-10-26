#! /bin/sh

run_cargo() {
	if [ -n "${FEATURES:-}" ]; then
		cargo "$@" --verbose --features="$FEATURES"
	else
		cargo "$@" --verbose
	fi
}

run_cargo test
