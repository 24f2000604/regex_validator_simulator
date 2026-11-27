/* Postfix -> NFA (the article's post2nfa), using Thompson's construction.
 *
 * Every state is a character test (one out arrow), a Split (two unlabeled
 * arrows out/out1), or Match. */
#ifndef NFAGREP_NFA_H
#define NFAGREP_NFA_H

#include <stdbool.h>
#include <stddef.h>

#include "parse.h"
#include "utf8.h"

enum { S_CHAR, S_ANY, S_SPLIT, S_MATCH };

typedef struct {
    int kind;
    rune c;        /* for S_CHAR */
    int out, out1; /* -1 = none */
} State;

typedef struct {
    State *states;
    size_t nstates;
    int start;
} Nfa;

/* Build the NFA for post[0..n). Returns false if the postfix is malformed. */
bool post2nfa(const Tok *post, size_t n, Nfa *m);
void nfa_free(Nfa *m);

#endif
