PG_CONFIG   ?= $(shell which pg_config)
DISTNAME     = $(shell grep -m 1 '^name' pg_search/Cargo.toml | sed -e 's/[^"]*"\([^"]*\)",\{0,1\}/\1/')
DISTDESC     = $(shell grep -m 1 '^description' pg_search/Cargo.toml | sed -e 's/[^"]*"\([^"]*\)",\{0,1\}/\1/')
DISTVERSION  = $(shell grep -m 1 '^version' Cargo.toml | sed -e 's/[^"]*"\([^"]*\)",\{0,1\}/\1/')
PGRXV        = $(shell perl -nE '/^pgrx\s+=\s"=?([^"]+)/ && do { say $$1; exit }' Cargo.toml)
PGV          = $(shell perl -E 'shift =~ /(\d+)/ && say $$1' "$(shell $(PG_CONFIG) --version)")
EXTRA_CLEAN  = META.json $(DISTNAME)-$(DISTVERSION).zip target

PGXS := $(shell $(PG_CONFIG) --pgxs)
include $(PGXS)

all: package

# Print the current Postgres version reported by pg_config.
.PHONY: pg-version
pg-version:
	@echo $(PGV)

# Print the current PGRX version from Cargo.toml
.PHONY: pg-version
pgrx-version:
	@echo $(PGRXV)

# Install the version of PGRX specified in Cargo.toml.
.PHONY: install-pgrx
install-pgrx: pg_search/Cargo.toml
	@cargo install --locked cargo-pgrx --version "$(PGRXV)"

# Initialize pgrx for the PostgreSQL version identified by pg_config.
.PHONY: pgrx-init
pgrx-init: pg_search/Cargo.toml
	@cargo pgrx init "--pg$(PGV)"="$(PG_CONFIG)"

# Install pg_search into the PostgreSQL cluster identified by pg_config.
.PHONY: install
install:
	@cargo pgrx install --package pg_search --release --pg-config "$(PG_CONFIG)"

# Build pg_search for the PostgreSQL cluster identified by pg_config.
.DEFAULT_GOAL: package
package:
	@cargo pgrx package --package pg_search --pg-config "$(PG_CONFIG)"

META.json: META.json.in pg_search/Cargo.toml
	@sed "s/@CRATE_DESC@/$(DISTDESC)/g" $< > $@
	@sed "s/@CRATE_VERSION@/$(DISTVERSION)/g" $< > $@

$(DISTNAME)-$(DISTVERSION).zip: META.json
	git archive --format zip --prefix $(DISTNAME)-$(DISTVERSION)/ --add-file $< -o $(DISTNAME)-$(DISTVERSION).zip HEAD

# Create a PGXN-compatible zip file.
dist: $(DISTNAME)-$(DISTVERSION).zip
