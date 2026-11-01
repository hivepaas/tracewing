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
.PHONY: help debug release build build-debug test test-window lint fmt clean

help: ## List the targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  make %-14s %s\n", $$1, $$2}'

debug: ## Build and run the debug version: quicker to build, slower to run
	$(CARGO) run $(APP)

release: ## Build and run the release version
	$(CARGO) run --release $(APP)

build-debug: ## Build the debug version: target/debug/tracewing
	$(CARGO) build $(APP)

build: ## Build the release version: target/release/tracewing
	$(CARGO) build --release $(APP)

test: ## Run every test, without the ones that draw the window
	$(CARGO) test --workspace --release

test-window: ## Run every test, with the window drawn with no display (Linux)
	$(CARGO) test --workspace --release --features tracewing/headless

lint: ## Check the format and run clippy, as CI does
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --workspace --all-targets --release -- -D warnings

fmt: ## Format the code
	$(CARGO) fmt --all

clean: ## Remove what was built
	$(CARGO) clean
