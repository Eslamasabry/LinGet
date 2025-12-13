# LinGet Makefile
PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin
DATADIR = $(PREFIX)/share
APPDIR = $(DATADIR)/applications
ICONDIR = $(DATADIR)/icons/hicolor

.PHONY: all build release install uninstall clean run check fmt

all: build

build:
	cargo build

release:
	cargo build --release

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
	install -Dm644 data/io.github.linget.desktop $(DESTDIR)$(APPDIR)/io.github.linget.desktop
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

# Development targets
dev: build
	RUST_LOG=linget=debug cargo run

test:
	cargo test
