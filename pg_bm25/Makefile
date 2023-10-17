#
# This Makefile is used exclusively to install the extension via PGXN. For
# anything development related, please follow the instructions in the README
# and use `pgrx` instead.
#

# Variables
EXTENSION = pg_bm25
BUILD_DIR = build

# Detect the OS
UNAME_S := $(shell uname -s)

# Set pg_config path and PostgreSQL directories based on the OS
ifeq ($(UNAME_S),Linux)
	PG_CONFIG = pg_config
endif
ifeq ($(UNAME_S),Darwin) # This is macOS
	PG_CONFIG = /usr/local/bin/pg_config
endif

PG_LIB_DIR = $(shell $(PG_CONFIG) --pkglibdir)
PG_EXTENSION_DIR = $(shell $(PG_CONFIG) --sharedir)/extension

# Default target
all: install

# Install the extension
install: $(BUILD_DIR)
	cp $(BUILD_DIR)/pg_bm25--9.9.9/usr/lib/postgresql/15/lib/$(EXTENSION).so $(PG_LIB_DIR)/
	cp $(BUILD_DIR)/pg_bm25--9.9.9/usr/share/postgresql/15/extension/$(EXTENSION)--9.9.9.sql $(PG_EXTENSION_DIR)/
	cp $(BUILD_DIR)/pg_bm25--9.9.9/usr/share/postgresql/15/extension/$(EXTENSION).control $(PG_EXTENSION_DIR)/

# Clean up build artifacts
clean:
	rm -rf $(BUILD_DIR)

# Phony targets
.PHONY: all install clean
