#include "parse.h"

#include <stdio.h>
#include <stdlib.h>

#include "util.h"

/* Per-parenthesis-level counters, as in the article: nalt is the number of
 * '|' seen at this level, natom the atoms pending concatenation since the
 * last '|'. */
typedef struct {
    size_t nalt, natom, open_pos;
} Level;

bool re2post(const rune *re, size_t n, Tok *dst, size_t *dn, ParseError *e)
{
    Level *stack = xmalloc((n + 1) * sizeof *stack);
    size_t sp = 0, k = 0;
    Level cur = {0, 0, 0};

#define EMIT(o, ch) (dst[k++] = (Tok){(o), (ch)})
#define FAIL(p, ...) do { e->pos = (p); snprintf(e->msg, sizeof e->msg, __VA_ARGS__); \
                          free(stack); return false; } while (0)
#define ATOM(o, ch) do { if (cur.natom > 1) { cur.natom--; EMIT(T_CAT, 0); } \
                         EMIT((o), (ch)); cur.natom++; } while (0)

    for (size_t i = 0; i < n; i++) {
        rune c = re[i];
        switch (c) {
        case '(':
            if (cur.natom > 1) { cur.natom--; EMIT(T_CAT, 0); }
            stack[sp++] = cur;
            cur = (Level){0, 0, i};
            break;
        case '|':
            if (cur.natom == 0) FAIL(i, "empty alternative before '|'");
            while (cur.natom > 1) { EMIT(T_CAT, 0); cur.natom--; }
            cur.natom = 0;
            cur.nalt++;
            break;
        case ')':
            if (sp == 0) FAIL(i, "unmatched ')'");
            if (cur.natom == 0)
                FAIL(i, cur.nalt ? "empty alternative before ')'" : "empty group '()'");
            while (cur.natom > 1) { EMIT(T_CAT, 0); cur.natom--; }
            for (; cur.nalt > 0; cur.nalt--) EMIT(T_ALT, 0);
            cur = stack[--sp];
            cur.natom++;
            break;
        case '*': case '+': case '?':
            if (cur.natom == 0) FAIL(i, "'%c' has nothing to repeat", (char)c);
            EMIT(c == '*' ? T_STAR : c == '+' ? T_PLUS : T_QUEST, 0);
            break;
        case '.':
            ATOM(T_ANY, 0);
            break;
        case '\\':
            if (i + 1 >= n) FAIL(i, "trailing '\\' escapes nothing");
            i++;
            ATOM(T_LIT, re[i]);
            break;
        default:
            ATOM(T_LIT, c);
        }
    }

    if (sp > 0) FAIL(cur.open_pos, "unclosed '('");
    if (cur.natom == 0 && cur.nalt > 0) FAIL(n, "empty alternative at end");
    while (cur.natom > 1) { EMIT(T_CAT, 0); cur.natom--; }
    for (; cur.nalt > 0; cur.nalt--) EMIT(T_ALT, 0);

#undef EMIT
#undef FAIL
#undef ATOM
    free(stack);
    *dn = k;
    return true;
}
