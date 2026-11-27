#include "nfa.h"

#include <stdlib.h>
#include <string.h>

#include "util.h"

/* A partly built NFA: a start state plus its dangling arrows, each encoded
 * as state*2 for that state's out, or state*2+1 for its out1. */
typedef struct {
    int start;
    int *out;
    size_t n;
} Frag;

static int new_state(Nfa *m, int kind, rune c, int out, int out1)
{
    m->states[m->nstates] = (State){kind, c, out, out1};
    return (int)m->nstates++;
}

/* Point every dangling arrow of f at state s. */
static void patch(Nfa *m, const Frag *f, int s)
{
    for (size_t i = 0; i < f->n; i++) {
        State *st = &m->states[f->out[i] >> 1];
        if (f->out[i] & 1) st->out1 = s;
        else st->out = s;
    }
}

/* Replace f's dangling list with the single arrow a. */
static void set_out(Frag *f, int a)
{
    f->out = xrealloc(f->out, sizeof *f->out);
    f->out[0] = a;
    f->n = 1;
}

bool post2nfa(const Tok *post, size_t n, Nfa *m)
{
    m->states = xmalloc((n + 1) * sizeof *m->states);
    m->nstates = 0;
    Frag *stack = xmalloc((n + 1) * sizeof *stack);
    size_t sp = 0;

    for (size_t i = 0; i < n; i++) {
        Frag e1, e2;
        int s;
        switch (post[i].op) {
        case T_LIT:
        case T_ANY:
            s = new_state(m, post[i].op == T_LIT ? S_CHAR : S_ANY, post[i].c, -1, -1);
            stack[sp].start = s;
            stack[sp].out = xmalloc(sizeof(int));
            stack[sp].out[0] = s * 2;
            stack[sp].n = 1;
            sp++;
            break;
        case T_CAT:
            if (sp < 2) goto bad;
            e2 = stack[--sp];
            e1 = stack[--sp];
            patch(m, &e1, e2.start);
            free(e1.out);
            stack[sp++] = (Frag){e1.start, e2.out, e2.n};
            break;
        case T_ALT:
            if (sp < 2) goto bad;
            e2 = stack[--sp];
            e1 = stack[--sp];
            s = new_state(m, S_SPLIT, 0, e1.start, e2.start);
            e1.out = xrealloc(e1.out, (e1.n + e2.n) * sizeof *e1.out);
            memcpy(e1.out + e1.n, e2.out, e2.n * sizeof *e2.out);
            free(e2.out);
            stack[sp++] = (Frag){s, e1.out, e1.n + e2.n};
            break;
        case T_QUEST:
            if (sp < 1) goto bad;
            e1 = stack[--sp];
            s = new_state(m, S_SPLIT, 0, e1.start, -1);
            e1.out = xrealloc(e1.out, (e1.n + 1) * sizeof *e1.out);
            e1.out[e1.n] = s * 2 + 1;
            stack[sp++] = (Frag){s, e1.out, e1.n + 1};
            break;
        case T_STAR:
            if (sp < 1) goto bad;
            e1 = stack[--sp];
            s = new_state(m, S_SPLIT, 0, e1.start, -1);
            patch(m, &e1, s);
            set_out(&e1, s * 2 + 1);
            e1.start = s;
            stack[sp++] = e1;
            break;
        case T_PLUS:
            if (sp < 1) goto bad;
            e1 = stack[--sp];
            s = new_state(m, S_SPLIT, 0, e1.start, -1);
            patch(m, &e1, s);
            set_out(&e1, s * 2 + 1);
            stack[sp++] = e1;
            break;
        }
    }

    int match = new_state(m, S_MATCH, 0, -1, -1);
    if (sp == 0 && n == 0) {
        m->start = match; /* empty pattern */
    } else if (sp == 1) {
        patch(m, &stack[0], match);
        m->start = stack[0].start;
        free(stack[0].out);
    } else {
        goto bad;
    }
    free(stack);
    return true;

bad:
    while (sp > 0) free(stack[--sp].out);
    free(stack);
    free(m->states);
    m->states = NULL;
    return false;
}

void nfa_free(Nfa *m)
{
    free(m->states);
    m->states = NULL;
    m->nstates = 0;
}
