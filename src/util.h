/* Allocation helpers that exit with status 2 when memory runs out. */
#ifndef NFAGREP_UTIL_H
#define NFAGREP_UTIL_H

#include <stddef.h>

#define PROG "nfagrep"

void *xmalloc(size_t n);
void *xrealloc(void *p, size_t n);

#endif
