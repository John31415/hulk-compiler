SHELL := /bin/bash

CARGO ?= cargo
TARGET ?= hulk

LLVM_CONFIG_PATH := $(shell command -v llvm-config-17 2>/dev/null || command -v llvm-config 2>/dev/null)

ifneq ($(LLVM_CONFIG_PATH),)
	# Si existe llvm-config (Local), le pedimos la ruta exacta
	LLVM_PREFIX_DETECTED := $(shell $(LLVM_CONFIG_PATH) --prefix)
else
	# Si no existe (CI del profesor), forzamos la ruta estática de Ubuntu
	LLVM_PREFIX_DETECTED := /usr/lib/llvm-17
endif

LLVM_PREFIX ?= $(LLVM_PREFIX_DETECTED)

.PHONY: build clean run

build:
	@echo "[hulk] Usando LLVM_PREFIX=$(LLVM_PREFIX)"
	LLVM_SYS_170_PREFIX=$(LLVM_PREFIX) $(CARGO) build --release
	cp target/release/$(TARGET) ./$(TARGET)

clean:
	$(CARGO) clean
	rm -f ./$(TARGET) *.o *.ll output

run: build
	./$(TARGET) $(FILE)