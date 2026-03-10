# LinGet Makefile
PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin
DATADIR = $(PREFIX)/share
APPDIR = $(DATADIR)/applications
ICONDIR = $(DATADIR)/icons/hicolor

.PHONY: all build release package install uninstall clean dist-clean run check fmt test

all: build

build:
	cargo build

release:
	cargo build --release

package:
	bash scripts/package_release.sh

check:
	cargo check
	cargo clippy -- -D warnings

fmt:
	cargo fmt

run:
	cargo run

install:
	@test -x target/release/linget || (echo "Missing target/release/linget. Run 'make release' first." && exit 1)
	install -Dm755 target/release/linget $(DESTDIR)$(BINDIR)/linget
	@desktop_tmp=$$(mktemp); \
		sed 's|^Exec=.*$$|Exec=$(BINDIR)/linget|' data/io.github.linget.desktop > "$$desktop_tmp"; \
		install -Dm644 "$$desktop_tmp" $(DESTDIR)$(APPDIR)/io.github.linget.desktop; \
		rm -f "$$desktop_tmp"
	install -Dm644 data/icons/hicolor/scalable/apps/io.github.linget.svg $(DESTDIR)$(ICONDIR)/scalable/apps/io.github.linget.svg
	install -Dm644 data/icons/hicolor/symbolic/apps/io.github.linget-symbolic.svg $(DESTDIR)$(ICONDIR)/symbolic/apps/io.github.linget-symbolic.svg
	gtk-update-icon-cache -f -t $(DESTDIR)$(ICONDIR) || true
	update-desktop-database $(DESTDIR)$(APPDIR) || true

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/linget
	rm -f $(DESTDIR)$(APPDIR)/io.github.linget.desktop
	rm -f $(DESTDIR)$(ICONDIR)/scalable/apps/io.github.linget.svg
	rm -f $(DESTDIR)$(ICONDIR)/symbolic/apps/io.github.linget-symbolic.svg

clean:
	cargo clean
	rm -rf dist/linget-*
	rm -f dist/linget-v*.tar.gz linget-v*.tar.gz

dist-clean: clean

# Development targets
dev: build
	RUST_LOG=linget=debug cargo run

test:
	cargo test
