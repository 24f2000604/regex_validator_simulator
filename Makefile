CC     ?= cc
CFLAGS ?= -std=c11 -O2 -Wall -Wextra -pedantic

nfagrep: nfagrep.c
	$(CC) $(CFLAGS) -o $@ $<

clean:
	rm -f nfagrep

.PHONY: clean
