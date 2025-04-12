CFLAGS=$(shell pkg-config --cflags criu) -O2
LDFLAGS=-Wl,--no-as-needed $(shell pkg-config --libs criu)

.PHONY: all
all: main
	sudo rm -rf /tmp/criu-test-*

.PHONY: clean
clean:
	rm -rf main
	sudo rm -rf /tmp/criu-test-*
