SHELL := /bin/bash

CARGO ?= cargo
TARGET ?= hulk

LLVM_CONFIG ?= $(shell command -v llvm-config-17 2>/dev/null || command -v llvm-config 2>/dev/null)

LLVM_SYS_VERSION := $(shell $(LLVM_CONFIG) --version | awk -F. '{print $$1 $$2}')

.PHONY: build clean run check-llvm

check-llvm:
	@if [ -z "$(LLVM_CONFIG)" ]; then \
		echo "[hulk] ERROR: llvm-config no encontrado"; \
		exit 1; \
	fi

build: check-llvm
	@echo "[hulk] Usando LLVM desde: $(LLVM_CONFIG)"
	LLVM_SYS_$(LLVM_SYS_VERSION)_PREFIX=$$($(LLVM_CONFIG) --prefix) $(CARGO) build --release
	cp target/release/$(TARGET) ./$(TARGET)

clean:
	$(CARGO) clean
	rm -f ./$(TARGET) *.o *.ll output

run: build
	./$(TARGET) $(FILE)
