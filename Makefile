SHELL := /bin/bash

CARGO ?= cargo
TARGET ?= hulk

LLVM_CONFIG ?= $(shell which llvm-config 2>/dev/null)

.PHONY: build clean run check-llvm

check-llvm:
	@if [ -z "$(LLVM_CONFIG)" ]; then \
		echo "[hulk] ERROR: llvm-config no encontrado en el entorno"; \
		exit 1; \
	fi

build: check-llvm
	@echo "[hulk] Usando LLVM desde: $(LLVM_CONFIG)"
	LLVM_SYS_$(shell $(LLVM_CONFIG) --version | cut -d. -f1)_PREFIX=$$($(LLVM_CONFIG) --prefix) \
	$(CARGO) build --release

	cp target/release/$(TARGET) ./$(TARGET)

clean:
	$(CARGO) clean
	rm -f ./$(TARGET) *.o *.ll output

run: build
	./$(TARGET) $(FILE)
