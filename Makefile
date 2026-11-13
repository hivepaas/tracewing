# Building tracewing in one word: `make debug` runs a debug build, `make release` a
# release one; `make` alone lists the rest. Needs Rust (rustup) and, on Linux, the
# packages listed in .github/workflows/ci.yml.
#
# On Linux the window builds for Wayland by default:
#   make release FEATURES=x11

# rustup's tools, for shells that have not sourced ~/.cargo/env. Make 3.81 (macOS)
# looks commands up on the PATH it started with, so cargo is named by path too.
CARGO_BIN := $(HOME)/.cargo/bin
export PATH := $(CARGO_BIN):$(PATH)
CARGO ?= $(if $(wildcard $(CARGO_BIN)/cargo),$(CARGO_BIN)/cargo,cargo)

# Extra cargo features for the window, like x11.
FEATURES ?=
APP := -p tracewing $(if $(FEATURES),--features $(FEATURES))

.DEFAULT_GOAL := help
.PHONY: help debug release build build-debug package test test-window lint fmt playground playground-trust clean

help: ## List the targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  make %-18s %s\n", $$1, $$2}'

debug: ## Build and run the debug version: quicker to build, slower to run
	$(CARGO) run $(APP)

release: ## Build and run the release version
	$(CARGO) run --release $(APP)

build-debug: ## Build the debug version: target/debug/tracewing
	$(CARGO) build $(APP)

build: ## Build the release version: target/release/tracewing
	$(CARGO) build --release $(APP)

package: build ## Package the release build for this system: dist/ (Linux; needs cargo-about)
	scripts/package_linux.sh

test: ## Run every test, without the ones that draw the window
	$(CARGO) test --workspace --release

test-window: ## Run every test, with the window drawn with no display (Linux)
	$(CARGO) test --workspace --release --features tracewing/headless

lint: ## Check the format, the translations and clippy, as CI does
	$(CARGO) fmt --all -- --check
	# The messages follow the code, in what they say and in what order they
	# are said: moving a `tr!` is enough to put the files out of step, and
	# adding one that already exists elsewhere is too. CI checks this, and
	# twice in one afternoon it was CI that found it -- once on main.
	python3 scripts/i18n.py check
	$(CARGO) clippy --workspace --all-targets --release -- -D warnings
	# The window tests are behind `headless`, so without it they compile to
	# nothing and clippy never sees them. CI runs this; so must `make lint`,
	# or a lint error in a window test is found at the pull request.
	$(CARGO) clippy --workspace --all-targets --release --features tracewing/headless -- -D warnings

fmt: ## Format the code
	$(CARGO) fmt --all

playground: ## Run a server to try tracewing on: http://localhost:10000, https://localhost:10443, mutual TLS on 10444
	$(CARGO) run --release -p tracewing-dev --bin tracewing-dev -- playground

playground-trust: ## The same, its authority trusted by tracewing first (once; restart tracewing)
	$(CARGO) run --release -p tracewing-dev --bin tracewing-dev -- playground --trust

clean: ## Remove what was built
	$(CARGO) clean
