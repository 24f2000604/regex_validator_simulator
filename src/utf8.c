#include "utf8.h"

#include <stdbool.h>

size_t utf8_decode(const unsigned char *s, size_t len, rune *out)
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
