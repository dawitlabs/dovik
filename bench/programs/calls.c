#include <stdio.h>
#include <stdlib.h>

static long long step(long long total, long long value) {
    return total + value * 3;
}

int main(void) {
    int n = 1024;
    long long *xs = malloc(sizeof(long long) * n);
    for (int i = 0; i < n; i++) xs[i] = i % 7;

    long long total = 0;
    for (int round = 0; round < 10000; round++)
        for (int i = 0; i < n; i++) total = step(total, xs[i]);

    printf("%lld\n", total);
    free(xs);
    return 0;
}
