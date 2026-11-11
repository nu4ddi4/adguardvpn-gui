VERSION := $(shell sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
PREFIX ?= /usr
DESTDIR ?=
CARGO ?= cargo
CARGO_TARGET_DIR ?= target
BUILDDIR := $(CURDIR)/build
APP_ID := io.github.nu4ddi4.AdguardVpnGui

.PHONY: all build install uninstall vendor source rpm clean help
all: build
build:
	$(CARGO) build --release --locked
install:
	install -Dm755 "$(CARGO_TARGET_DIR)/release/adguard-gui" "$(DESTDIR)$(PREFIX)/bin/adguard-gui"
	install -Dm644 packaging/adguard-gui.desktop "$(DESTDIR)$(PREFIX)/share/applications/$(APP_ID).desktop"
	install -Dm644 assets/icons/adguard-gui.svg "$(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/adguard-gui.svg"
	install -Dm644 packaging/$(APP_ID).metainfo.xml "$(DESTDIR)$(PREFIX)/share/metainfo/$(APP_ID).metainfo.xml"
uninstall:
	rm -f "$(DESTDIR)$(PREFIX)/bin/adguard-gui"
	rm -f "$(DESTDIR)$(PREFIX)/share/applications/$(APP_ID).desktop"
	rm -f "$(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/adguard-gui.svg"
	rm -f "$(DESTDIR)$(PREFIX)/share/metainfo/$(APP_ID).metainfo.xml"
vendor:
	rm -rf build/source
	mkdir -p build/source/.cargo
	$(CARGO) vendor --locked build/source/vendor > build/source/.cargo/config.toml
	sed -i 's|directory = ".*"|directory = "vendor"|' build/source/.cargo/config.toml
source: vendor
	python3 scripts/collect-rust-licenses.py
	cp Cargo.toml Cargo.lock LICENSE README.md CHANGELOG.md Makefile build/source/
	cp -r src packaging scripts docs .github build/source/
	mkdir -p build/source/assets/icons build/rpm/SOURCES
	cp assets/icons/adguard-gui.svg build/source/assets/icons/
	tar -C build/source --transform 's,^\.,adguard-gui-$(VERSION),' -czf "build/rpm/SOURCES/adguard-gui-$(VERSION).tar.gz" .
rpm: source
	mkdir -p build/rpm/BUILD build/rpm/RPMS build/rpm/SPECS build/rpm/SRPMS
	rpmbuild --define "_topdir $(BUILDDIR)/rpm" --define "app_version $(VERSION)" --define 'dist %{nil}' -bb packaging/rpm.spec
clean:
	$(CARGO) clean
	rm -rf build
help:
	@echo 'make build | install | uninstall | source | rpm | clean'
	@echo 'RPM은 잠긴 의존성을 소스에 포함하고 오프라인으로 컴파일합니다.'
