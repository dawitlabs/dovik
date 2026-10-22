#include <stdio.h>
#include <stdlib.h>
int main(void) {
    long long n = 300000;
    long long *xs = malloc(sizeof(long long) * n);
    for (long long i = 0; i < n; i++) xs[i] = i;
    long long total = 0;
    for (long long i = 0; i < n; i++) total += xs[i];
    printf("%lld\n", total);
    long long m = 100000;
    long long *d = calloc(m, sizeof(long long));
    for (long long k = 0; k < m; k++) d[k] = k * 2;
    printf("%lld\n", m);
    free(xs);
    free(d);
    return 0;
}
