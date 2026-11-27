/*
 * nfagrep: a small grep built on a Thompson NFA, after Russ Cox, "Regular
 * Expression Matching Can Be Simple And Fast".
 *
 * A line matches if the pattern matches any substring of it. A leading ^ or
 * trailing $ anchors the match to the start or end of the line.
 */
#define _POSIX_C_SOURCE 200809L /* getline */
#include <errno.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "nfa.h"
#include "parse.h"
#include "sim.h"
#include "utf8.h"
#include "util.h"

typedef struct {
    bool icase, invert, count, number, quiet, with_name;
    bool bol, eol; /* pattern anchors */
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
            PROG);
    exit(2);
}

/* Returns the number of selected lines, or -1 on a read error. */
static long grep_file(FILE *f, const char *name, Sim *sim, const Opts *o)
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

        if (sim_search(sim, runes, nr, o->bol, o->eol, o->icase) == o->invert) continue;
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

/* Compile pat into *nfa, setting the anchor flags in *o. Exits on error. */
static void compile(const char *pat, Nfa *nfa, Opts *o)
{
    size_t plen = strlen(pat);
    rune *pr = xmalloc((plen + 1) * sizeof *pr);
    size_t pn = utf8_decode((const unsigned char *)pat, plen, pr);

    size_t lo = 0, hi = pn;
    if (hi > lo && pr[lo] == '^') { o->bol = true; lo++; }
    if (hi > lo && pr[hi - 1] == '$') {
        size_t bs = 0; /* an odd run of backslashes means the $ is escaped */
        while (hi - 1 - bs > lo && pr[hi - 2 - bs] == '\\') bs++;
        if (bs % 2 == 0) { o->eol = true; hi--; }
    }

    Tok *post = xmalloc((2 * (hi - lo) + 1) * sizeof *post);
    size_t postn;
    ParseError pe;
    if (!re2post(pr + lo, hi - lo, post, &postn, &pe)) {
        fprintf(stderr, "%s: bad pattern at position %zu: %s\n", PROG, pe.pos + lo, pe.msg);
        fprintf(stderr, "  %s\n  %*s^\n", pat, (int)(pe.pos + lo), "");
        exit(2);
    }
    if (!post2nfa(post, postn, nfa)) {
        fprintf(stderr, "%s: malformed postfix\n", PROG);
        exit(2);
    }
    free(post);
    free(pr);
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
                fprintf(stderr, "%s: unknown option -%c\n", PROG, *p);
                usage();
            }
        }
    }
    if (i >= argc) usage();

    Nfa nfa;
    compile(argv[i++], &nfa, &o);
    Sim sim;
    sim_init(&sim, &nfa);

    int nfiles = argc - i;
    o.with_name = name_mode == 1 || (name_mode == 0 && nfiles > 1);
    bool any = false, error = false;

    if (nfiles == 0) {
        long r = grep_file(stdin, "(standard input)", &sim, &o);
        if (r < 0) error = true;
        else any = r > 0;
    }
    for (; i < argc && !(o.quiet && any); i++) {
        const char *name = argv[i];
        bool is_stdin = strcmp(name, "-") == 0;
        FILE *f = is_stdin ? stdin : fopen(name, "r");
        if (!f) {
            fprintf(stderr, "%s: %s: %s\n", PROG, name, strerror(errno));
            error = true;
            continue;
        }
        long r = grep_file(f, is_stdin ? "(standard input)" : name, &sim, &o);
        if (r < 0) {
            fprintf(stderr, "%s: %s: read error\n", PROG, name);
            error = true;
        } else if (r > 0) {
            any = true;
        }
        if (!is_stdin) fclose(f);
    }

    sim_free(&sim);
    nfa_free(&nfa);
    if (o.quiet && any) return 0;
    return error ? 2 : any ? 0 : 1;
}
