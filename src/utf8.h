/* UTF-8 decoding, so that . matches one character rather than one byte. */
#ifndef NFAGREP_UTF8_H
#define NFAGREP_UTF8_H

#include <stddef.h>
#include <stdint.h>

typedef uint32_t rune;

/* An invalid byte b decodes to INVALID_BASE + b, outside the Unicode range,
 * so it never equals a real character but still equals the same bad byte. */
#define INVALID_BASE 0x110000u

/* Decode s[0..len) into out, which needs room for len runes.
 * Returns the number of runes written. */
size_t utf8_decode(const unsigned char *s, size_t len, rune *out);

/* ASCII-only case folding, used by -i. */
static inline rune rune_fold(rune c)
{
    return (c >= 'A' && c <= 'Z') ? c - 'A' + 'a' : c;
}

#endif
