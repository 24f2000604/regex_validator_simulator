/* Infix -> postfix conversion (the article's re2post).
 *
 * Syntax: literals, |, *, +, ?, ( ), . (any character) and \x (escape).
 * Concatenation, implicit in the infix form, becomes the explicit T_CAT
 * operator in postfix. */
#ifndef NFAGREP_PARSE_H
#define NFAGREP_PARSE_H

#include <stdbool.h>
#include <stddef.h>

#include "utf8.h"

enum { T_LIT, T_ANY, T_CAT, T_ALT, T_STAR, T_PLUS, T_QUEST };

typedef struct {
    int op;
    rune c; /* for T_LIT */
} Tok;

typedef struct {
    size_t pos; /* character index in the pattern */
    char msg[64];
} ParseError;

/* Convert re[0..n) to postfix. dst needs room for 2*n tokens. An empty
 * pattern gives zero tokens (it matches everything, as in grep).
 * On failure, fills *e and returns false. */
bool re2post(const rune *re, size_t n, Tok *dst, size_t *dn, ParseError *e);

#endif
