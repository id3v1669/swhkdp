# Destination dir, defaults to root. Should be overridden for packaging
# e.g. make DESTDIR="packaging_subdir" install
DESTDIR ?= "/"
DAEMON_BINARY := swhkdp
SERVER_BINARY := swhks
BUILDFLAGS := --release
POLKIT_DIR := /usr/share/polkit-1/actions
POLKIT_POLICY_FILE := com.github.swhkdp.pkexec.policy
TARGET_DIR := /usr/bin
VERSION = $(shell awk -F ' = ' '$$1 ~ /version/ { gsub(/["]/, "", $$2); printf("%s",$$2) }' Cargo.toml)

all: build

build:
	@cargo build $(BUILDFLAGS)
	@./scripts/build-polkit-policy.sh \
		--policy-path=$(POLKIT_POLICY_FILE) \
		--swhkdp-path=$(TARGET_DIR)/$(DAEMON_BINARY)

install:
	@install -Dm 755 ./target/release/$(DAEMON_BINARY) -t $(DESTDIR)/$(TARGET_DIR)
	@install -Dm 755 ./target/release/$(SERVER_BINARY) -t $(DESTDIR)/$(TARGET_DIR)
	@install -Dm 644 -o root ./$(POLKIT_POLICY_FILE) -t $(DESTDIR)/$(POLKIT_DIR)

uninstall:
	@$(RM) $(TARGET_DIR)/$(SERVER_BINARY)
	@$(RM) $(TARGET_DIR)/$(DAEMON_BINARY)
	@$(RM) $(POLKIT_DIR)/$(POLKIT_POLICY_FILE)

check:
	@cargo fmt
	@cargo check
	@cargo clippy

release:
	@$(MAKE) -s
	@zip -r "swhkdp-x86_64-$(VERSION).zip" ./target/release/swhkdp ./target/release/swhks

test:
	@cargo test

clean:
	@cargo clean
	@$(RM) -f $(DAEMON_BINARY)rc
	@$(RM) -f $(POLKIT_POLICY_FILE)

setup:
	@rustup install stable
	@rustup default stable

.PHONY: check clean setup all install build release
