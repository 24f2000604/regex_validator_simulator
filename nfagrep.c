/*
 * nfagrep: a small grep built on the Thompson NFA engine from src/engine
 * (re2post -> post2nfa -> lock-step simulation), after Russ Cox, "Regular
 * Expression Matching Can Be Simple And Fast".
 *
 * Syntax, as in the Rust engine: literals, |, *, +, ?, ( ), . (any
 * character) and \x (escape). Like grep, a line matches if the pattern
 * matches any substring of it. A leading ^ or trailing $ anchors the match
 * to the start or end of the line.
 *
 * Text is decoded as UTF-8, so . matches one character, not one byte.
 * Invalid bytes are kept as distinct values and still compare correctly.
 */
#define _POSIX_C_SOURCE 200809L
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>

typedef uint32_t rune;

static const char *prog = "nfagrep";

static void *xmalloc(size_t n)
{
    void *p = malloc(n ? n : 1);
    if (!p) {
        fprintf(stderr, "%s: out of memory\n", prog);
        exit(2);
    }
    return p;
}

static void *xrealloc(void *p, size_t n)
{
    p = realloc(p, n ? n : 1);
    if (!p) {
        fprintf(stderr, "%s: out of memory\n", prog);
        exit(2);
    }
    return p;
}

/* ---------- UTF-8 ---------- */

/* An invalid byte b decodes to INVALID_BASE + b, outside the Unicode range. */
#define INVALID_BASE 0x110000u

/* Decode s[0..len) into out (room for len runes). Returns the rune count. */
static size_t utf8_decode(const unsigned char *s, size_t len, rune *out)
{
    size_t i = 0, n = 0;
    while (i < len) {
        unsigned char b = s[i];
        size_t need;
        rune c, min;
        if (b < 0x80) { out[n++] = b; i++; continue; }
        else if ((b & 0xE0) == 0xC0) { need = 1; c = b & 0x1F; min = 0x80; }
        else if ((b & 0xF0) == 0xE0) { need = 2; c = b & 0x0F; min = 0x800; }
        else if ((b & 0xF8) == 0xF0) { need = 3; c = b & 0x07; min = 0x10000; }
        else { out[n++] = INVALID_BASE + b; i++; continue; }

        bool ok = i + need < len;
        for (size_t k = 1; ok && k <= need; k++) {
            if ((s[i + k] & 0xC0) != 0x80) ok = false;
            else c = (c << 6) | (s[i + k] & 0x3F);
        }
        if (!ok || c < min || c > 0x10FFFF || (c >= 0xD800 && c <= 0xDFFF)) {
            out[n++] = INVALID_BASE + b;
            i++;
            continue;
        }
        out[n++] = c;
        i += need + 1;
    }
    return n;
}

static rune fold(rune c)
{
    return (c >= 'A' && c <= 'Z') ? c - 'A' + 'a' : c;
}

/* ---------- re2post: infix -> postfix ---------- */

enum { T_LIT, T_ANY, T_CAT, T_ALT, T_STAR, T_PLUS, T_QUEST };

typedef struct {
    int op;
    rune c; /* for T_LIT */
} Tok;

typedef struct {
    size_t pos; /* character index in the pattern */
    char msg[64];
} ParseError;

/* Per-parenthesis-level counters, as in the article: nalt is the number of
 * '|' seen at this level, natom the atoms pending concatenation since the
 * last '|'. */
typedef struct {
    size_t nalt, natom, open_pos;
} Level;

/* dst must have room for 2*n tokens. An empty pattern gives zero tokens
 * (it matches everything, as in grep). */
static bool re2post(const rune *re, size_t n, Tok *dst, size_t *dn, ParseError *e)
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

/* ---------- post2nfa: Thompson's construction ---------- */

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

/* A dangling arrow: state*2 for its out, state*2+1 for its out1. */
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

static bool post2nfa(const Tok *post, size_t n, Nfa *m)
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
    return false;
}

/* ---------- Simulation ---------- */

typedef struct {
    const Nfa *nfa;
    unsigned long long *lastlist; /* listid each state was last added in */
    unsigned long long listid;
    int *clist, *nlist, *work;
    size_t cn, nn;
    bool matched; /* Match is in the list being built */
} Sim;

static void sim_init(Sim *s, const Nfa *nfa)
{
    size_t n = nfa->nstates;
    s->nfa = nfa;
    s->lastlist = xmalloc(n * sizeof *s->lastlist);
    memset(s->lastlist, 0, n * sizeof *s->lastlist);
    s->listid = 0;
    s->clist = xmalloc(n * sizeof(int));
    s->nlist = xmalloc(n * sizeof(int));
    s->work = xmalloc(n * sizeof(int));
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

/* Does the pattern match somewhere in line[0..n)? */
static bool search(Sim *sim, const rune *line, size_t n, bool bol, bool eol, bool icase)
{
    const Nfa *m = sim->nfa;

    sim->listid++;
    sim->matched = false;
    sim->cn = 0;
    addstate(sim, sim->clist, &sim->cn, m->start);
    if (sim->matched && !eol) return true;

    for (size_t i = 0; i < n; i++) {
        rune c = icase ? fold(line[i]) : line[i];
        sim->listid++;
        sim->matched = false;
        sim->nn = 0;
        for (size_t j = 0; j < sim->cn; j++) {
            const State *st = &m->states[sim->clist[j]];
            bool ok = st->kind == S_ANY ||
                      (st->kind == S_CHAR && (icase ? fold(st->c) : st->c) == c);
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

/* ---------- CLI ---------- */

typedef struct {
    bool icase, invert, count, number, quiet, with_name;
} Opts;

static void usage(void)
{
    fprintf(stderr,
            "usage: %s [-icnvqHh] PATTERN [FILE...]\n"
            "  -i  ignore ASCII case      -v  select non-matching lines\n"
            "  -c  print only a count     -n  prefix line numbers\n"
            "  -q  quiet, exit status only\n"
            "  -H  always print file names  -h  never print file names\n"
            "Syntax: literals | * + ? ( ) . \\x, with optional leading ^ and trailing $\n",
            prog);
    exit(2);
}

/* Returns the number of selected lines, or -1 on a read error. */
static long grep_file(FILE *f, const char *name, Sim *sim, bool bol, bool eol, const Opts *o)
{
    char *buf = NULL;
    size_t cap = 0, rcap = 0;
    rune *runes = NULL;
    ssize_t len;
    long lineno = 0, selected = 0;

    while ((len = getline(&buf, &cap, f)) != -1) {
        lineno++;
        size_t n = (size_t)len;
        if (n > 0 && buf[n - 1] == '\n') n--;
        if (n > rcap) {
            rcap = n * 2;
            runes = xrealloc(runes, rcap * sizeof *runes);
        }
        size_t nr = utf8_decode((unsigned char *)buf, n, runes);

        if (search(sim, runes, nr, bol, eol, o->icase) == o->invert) continue;
        selected++;
        if (o->quiet) break;
        if (o->count) continue;
        if (o->with_name) printf("%s:", name);
        if (o->number) printf("%ld:", lineno);
        fwrite(buf, 1, n, stdout);
        putchar('\n');
    }
    bool err = ferror(f);
    free(buf);
    free(runes);
    if (err) return -1;
    if (o->count && !o->quiet) {
        if (o->with_name) printf("%s:", name);
        printf("%ld\n", selected);
    }
    return selected;
}

int main(int argc, char **argv)
{
    Opts o = {0};
    int name_mode = 0; /* 0 = auto, 1 = -H, -1 = -h */
    int i = 1;

    for (; i < argc && argv[i][0] == '-' && argv[i][1]; i++) {
        if (strcmp(argv[i], "--") == 0) { i++; break; }
        for (const char *p = argv[i] + 1; *p; p++) {
            switch (*p) {
            case 'i': o.icase = true; break;
            case 'v': o.invert = true; break;
            case 'c': o.count = true; break;
            case 'n': o.number = true; break;
            case 'q': o.quiet = true; break;
            case 'H': name_mode = 1; break;
            case 'h': name_mode = -1; break;
            default:
                fprintf(stderr, "%s: unknown option -%c\n", prog, *p);
                usage();
            }
        }
    }
    if (i >= argc) usage();

    /* Decode the pattern and peel off the ^ / $ anchors. */
    const char *pat = argv[i++];
    size_t plen = strlen(pat);
    rune *pr = xmalloc((plen + 1) * sizeof *pr);
    size_t pn = utf8_decode((const unsigned char *)pat, plen, pr);
    size_t lo = 0, hi = pn;
    bool bol = false, eol = false;
    if (hi > lo && pr[lo] == '^') { bol = true; lo++; }
    if (hi > lo && pr[hi - 1] == '$') {
        size_t bs = 0; /* an odd run of backslashes means the $ is escaped */
        while (hi - 1 - bs > lo && pr[hi - 2 - bs] == '\\') bs++;
        if (bs % 2 == 0) { eol = true; hi--; }
    }

    Tok *post = xmalloc((2 * (hi - lo) + 1) * sizeof *post);
    size_t postn;
    ParseError pe;
    if (!re2post(pr + lo, hi - lo, post, &postn, &pe)) {
        fprintf(stderr, "%s: bad pattern at position %zu: %s\n", prog, pe.pos + lo, pe.msg);
        fprintf(stderr, "  %s\n  %*s^\n", pat, (int)(pe.pos + lo), "");
        return 2;
    }
    Nfa nfa;
    if (!post2nfa(post, postn, &nfa)) {
        fprintf(stderr, "%s: malformed postfix\n", prog);
        return 2;
    }
    free(post);
    free(pr);

    Sim sim;
    sim_init(&sim, &nfa);

    int nfiles = argc - i;
    o.with_name = name_mode == 1 || (name_mode == 0 && nfiles > 1);
    bool any = false, error = false;

    if (nfiles == 0) {
        long r = grep_file(stdin, "(standard input)", &sim, bol, eol, &o);
        if (r < 0) error = true;
        else any = r > 0;
    }
    for (; i < argc && !(o.quiet && any); i++) {
        const char *name = argv[i];
        FILE *f = strcmp(name, "-") == 0 ? stdin : fopen(name, "r");
        if (!f) {
            fprintf(stderr, "%s: %s: %s\n", prog, name, strerror(errno));
            error = true;
            continue;
        }
        long r = grep_file(f, strcmp(name, "-") == 0 ? "(standard input)" : name, &sim, bol, eol, &o);
        if (r < 0) {
            fprintf(stderr, "%s: %s: read error\n", prog, name);
            error = true;
        } else if (r > 0) {
            any = true;
        }
        if (f != stdin) fclose(f);
    }

    if (o.quiet && any) return 0;
    return error ? 2 : any ? 0 : 1;
}
