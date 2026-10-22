#include <stdio.h>
#include <stdlib.h>
int main(void) {
    long long total = 0;
    for (long long i = 0; i < 200000; i++) {
        long long *xs = malloc(sizeof(long long) * 3);
        xs[0] = i; xs[1] = i + 1; xs[2] = i + 2;
        total += xs[0];
        free(xs);
    }
    printf("%lld\n", total);
    return 0;
}
