#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(void) {
    int n = 20000;
    char *s = malloc(1);
    s[0] = 0;
    size_t len = 0;
    for (int i = 0; i < n; i++) {
        char *next = malloc(len + 2);
        memcpy(next, s, len);
        next[len] = 'x';
        next[len + 1] = 0;
        free(s);
        s = next;
        len++;
    }
    printf("%zu\n", len);
    long long count = 0;
    for (size_t i = 0; i < len; i++) count++;
    printf("%lld\n", count);
    free(s);
    return 0;
}
