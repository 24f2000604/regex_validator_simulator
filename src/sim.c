#include "sim.h"

#include <stdlib.h>
#include <string.h>

#include "util.h"

void sim_init(Sim *s, const Nfa *nfa)
{
    size_t n = nfa->nstates;
    s->nfa = nfa;
    s->lastlist = xmalloc(n * sizeof *s->lastlist);
    memset(s->lastlist, 0, n * sizeof *s->lastlist);
    s->listid = 0;
    s->clist = xmalloc(n * sizeof(int));
    s->nlist = xmalloc(n * sizeof(int));
    s->work = xmalloc(n * sizeof(int));
    s->cn = s->nn = 0;
    s->matched = false;
}

void sim_free(Sim *s)
{
    free(s->lastlist);
    free(s->clist);
    free(s->nlist);
    free(s->work);
}

/* Add s and everything reachable from it over Split arrows (addstate).
 * Uses an explicit stack, so deeply nested patterns can't overflow. */
static void addstate(Sim *sim, int *l, size_t *ln, int s0)
{
    size_t top = 0;
    sim->work[top++] = s0;
    while (top > 0) {
        int s = sim->work[--top];
        if (s < 0 || sim->lastlist[s] == sim->listid) continue;
        sim->lastlist[s] = sim->listid;
        const State *st = &sim->nfa->states[s];
        if (st->kind == S_SPLIT) {
            /* out1 pushed first so out is explored first, as in the article */
            sim->work[top++] = st->out1;
            sim->work[top++] = st->out;
            continue;
        }
        if (st->kind == S_MATCH) sim->matched = true;
        l[(*ln)++] = s;
    }
}

bool sim_search(Sim *sim, const rune *line, size_t n, bool bol, bool eol, bool icase)
{
    const Nfa *m = sim->nfa;

    sim->listid++;
    sim->matched = false;
    sim->cn = 0;
    addstate(sim, sim->clist, &sim->cn, m->start);
    if (sim->matched && !eol) return true;

    for (size_t i = 0; i < n; i++) {
        rune c = icase ? rune_fold(line[i]) : line[i];
        sim->listid++;
        sim->matched = false;
        sim->nn = 0;
        for (size_t j = 0; j < sim->cn; j++) {
            const State *st = &m->states[sim->clist[j]];
            bool ok = st->kind == S_ANY ||
                      (st->kind == S_CHAR && (icase ? rune_fold(st->c) : st->c) == c);
            if (ok) addstate(sim, sim->nlist, &sim->nn, st->out);
        }
        /* Unanchored: a new match attempt may begin at every position. */
        if (!bol) addstate(sim, sim->nlist, &sim->nn, m->start);

        int *t = sim->clist; sim->clist = sim->nlist; sim->nlist = t;
        sim->cn = sim->nn;

        if (sim->matched && !eol) return true;
        if (sim->cn == 0) return false;
    }
    return sim->matched;
}
