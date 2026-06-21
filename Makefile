SHELL := /bin/bash

CARGO ?= cargo
TARGET ?= hulk

LLVM_CONFIG ?= $(shell command -v llvm-config-17 2>/dev/null || command -v llvm-config-16 2>/dev/null || command -v llvm-config-15 2>/dev/null || command -v llvm-config-14 2>/dev/null || command -v llvm-config 2>/dev/null)

LLVM_SYS_VERSION := $(shell $(LLVM_CONFIG) --version 2>/dev/null | awk -F. '{print $$1 $$2}')

.PHONY: build clean run check-llvm debug-env

debug-env:
	@echo "=== DEBUGGING CI ENVIRONMENT ==="
	@echo "LLVM_CONFIG detectado: '$(LLVM_CONFIG)'"
	@if [ -n "$(LLVM_CONFIG)" ]; then $(LLVM_CONFIG) --version; else echo "LLVM NO ENCONTRADO EN EL PATH"; fi
	@echo "LLVM_SYS_VERSION parseado: '$(LLVM_SYS_VERSION)'"
	@rustc --version
	@cargo --version
	@echo "================================"

check-llvm: debug-env
	@if [ -z "$(LLVM_CONFIG)" ]; then \
		echo "[hulk] ERROR CRITICO: llvm-config no encontrado. El CI no tiene LLVM."; \
		exit 1; \
	fi

build: check-llvm
	@echo "[hulk] Usando LLVM desde: $(LLVM_CONFIG)"
	# Usamos un fallback si la evaluación del prefix falla
	LLVM_SYS_$(LLVM_SYS_VERSION)_PREFIX=$$($(LLVM_CONFIG) --prefix) $(CARGO) build --release --verbose
	cp target/release/$(TARGET) ./$(TARGET)

clean:
	$(CARGO) clean
	rm -f ./$(TARGET) *.o *.ll output

run: build
	./$(TARGET) $(FILE)
