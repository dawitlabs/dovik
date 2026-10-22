#include <stdio.h>
long long step(long long n) { return n + 1; }
int main(void) {
    long long total = 0;
    for (long long i = 0; i < 10000000; i++) total = step(total);
    printf("%lld\n", total);
    return 0;
}
