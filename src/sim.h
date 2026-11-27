/* Simulating the NFA (the article's match, step and addstate).
 *
 * The simulation tracks the set of states the NFA could be in and advances
 * them all one character at a time. Each state is added at most once per
 * step (the lastlist trick), which bounds the run time to O(m*n). */
#ifndef NFAGREP_SIM_H
#define NFAGREP_SIM_H

#include <stdbool.h>
#include <stddef.h>

#include "nfa.h"
#include "utf8.h"

typedef struct {
    const Nfa *nfa;
    unsigned long long *lastlist; /* listid each state was last added in */
    unsigned long long listid;
    int *clist, *nlist, *work;
    size_t cn, nn;
    bool matched; /* Match is in the list being built */
} Sim;

void sim_init(Sim *s, const Nfa *nfa);
void sim_free(Sim *s);

/* Does the pattern match somewhere in line[0..n)? bol/eol anchor the match
 * to the start/end of the line; icase folds ASCII case. */
bool sim_search(Sim *s, const rune *line, size_t n, bool bol, bool eol, bool icase);

#endif
