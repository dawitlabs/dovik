#include <stdio.h>
#include <stdlib.h>
int main(void) {
    int n = 20000;
    char *s = malloc(n + 1);
    for (int i = 0; i < n; i++) s[i] = 'x';
    s[n] = 0;
    long long total = 0;
    for (int j = 0; j < 2000; j++) if (s[j] == 'x') total++;
    printf("%lld\n", total);
    free(s);
    return 0;
}
