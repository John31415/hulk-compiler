CARGO = cargo
TARGET = hulk

build:
	$(CARGO) build --release
	cp target/release/$(TARGET) $(TARGET)

clean:
	$(CARGO) clean
	rm -f $(TARGET) *.o *.ll output

run: build
	./$(TARGET) $(FILE)
	./output