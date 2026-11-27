#include "util.h"

#include <stdio.h>
#include <stdlib.h>

static void oom(void)
{
    fprintf(stderr, "%s: out of memory\n", PROG);
    exit(2);
}

void *xmalloc(size_t n)
{
    void *p = malloc(n ? n : 1);
    if (!p) oom();
    return p;
}

void *xrealloc(void *p, size_t n)
{
    p = realloc(p, n ? n : 1);
    if (!p) oom();
    return p;
}
