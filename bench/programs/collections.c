#include <stdio.h>
#include <stdlib.h>

typedef struct { long long key, val; } Entry;

static Entry *entries;
static unsigned char *state;
static size_t cap, count;

static unsigned long long hash_key(long long key) {
    unsigned long long mixed = (unsigned long long)key * 0x9E3779B97F4A7C15ull;
    return mixed ^ (mixed >> 29);
}

static size_t probe(long long key) {
    size_t mask = cap - 1;
    size_t slot = hash_key(key) & mask;
    while (state[slot] && entries[slot].key != key) slot = (slot + 1) & mask;
    return slot;
}

static void grow(void) {
    size_t next = cap < 8 ? 8 : cap * 2;
    Entry *moved = malloc(next * sizeof(Entry));
    unsigned char *moved_state = calloc(next, 1);
    Entry *old = entries;
    unsigned char *old_state = state;
    size_t old_cap = cap;

    entries = moved;
    state = moved_state;
    cap = next;
    count = 0;

    for (size_t i = 0; i < old_cap; i++) {
        if (!old_state[i]) continue;
        size_t slot = probe(old[i].key);
        entries[slot] = old[i];
        state[slot] = 1;
        count++;
    }

    free(old);
    free(old_state);
}

int main(void) {
    long long n = 300000;
    long long *xs = malloc(sizeof(long long) * n);
    for (long long i = 0; i < n; i++) xs[i] = i;

    long long total = 0;
    for (long long i = 0; i < n; i++) total += xs[i];
    printf("%lld\n", total);

    long long m = 100000;
    for (long long k = 0; k < m; k++) {
        if (count * 4 >= cap * 3) grow();
        size_t slot = probe(k);
        if (!state[slot]) {
            entries[slot].key = k;
            state[slot] = 1;
            count++;
        }
        entries[slot].val = k * 2;
    }
    printf("%zu\n", count);

    free(xs);
    free(entries);
    free(state);
    return 0;
}
