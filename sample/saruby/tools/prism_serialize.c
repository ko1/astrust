// Build time (build.rs): prism_serialize SRC OPTIONS OUT writes pm_serialize_parse's output
// for SRC parsed with the options blob OPTIONS (prism.rs build_options).
#include <stdio.h>
#include <stdlib.h>
#include "prism.h"

static char *slurp(const char *path, size_t *len) {
    FILE *f = fopen(path, "rb");
    if (!f) { perror(path); exit(1); }
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *b = malloc(n + 1);
    if (!b || fread(b, 1, n, f) != (size_t)n) { perror(path); exit(1); }
    fclose(f);
    *len = n;
    return b;
}

int main(int argc, char **argv) {
    if (argc != 4) { fprintf(stderr, "usage: %s SRC OPTIONS OUT\n", argv[0]); return 2; }
    size_t slen, olen;
    const char *src = slurp(argv[1], &slen);
    const char *opts = slurp(argv[2], &olen);
    (void)olen;
    pm_buffer_t *buf = pm_buffer_new();
    pm_serialize_parse(buf, (const uint8_t *)src, slen, opts);
    FILE *o = fopen(argv[3], "wb");
    if (!o || fwrite(pm_buffer_value(buf), 1, pm_buffer_length(buf), o) != pm_buffer_length(buf) || fclose(o)) { perror(argv[3]); return 1; }
    return 0;
}
