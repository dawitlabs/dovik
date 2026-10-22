#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define TAG_INT 0
#define TAG_FLOAT 1
#define TAG_BOOL 2
#define TAG_STR 3
#define TAG_LIST 4
#define TAG_DICT 5

typedef struct {
    uint8_t tag;
    uint64_t word;
} Cell;

typedef struct {
    int64_t count;
} Header;

typedef struct Str {
    Header h;
    size_t len;
    size_t cap;
    uint8_t frozen;
    uint8_t hashed;
    uint64_t hash;
    char *bytes;
    struct Str *next_free;
} Str;

typedef struct List {
    Header h;
    size_t len;
    size_t cap;
    uint8_t elem;
    uint64_t *items;
    struct List *next_free;
} List;

typedef struct {
    uint64_t key;
    uint64_t val;
} Entry;

typedef struct {
    Header h;
    size_t len;
    size_t cap;
    uint8_t key_tag;
    uint8_t val_tag;
    Entry *entries;
    uint8_t *state;
} Dict;

static int64_t live_objects = 0;

#define POOL_LIMIT 2048
#define POOL_MAX_BYTES 4096

static Str *string_pool = NULL;
static size_t string_pool_len = 0;
static List *list_pool = NULL;
static size_t list_pool_len = 0;

static char out_buffer[1 << 16];
static size_t out_len = 0;

static void flush_output(void) {
    if (out_len == 0) return;
    fwrite(out_buffer, 1, out_len, stdout);
    out_len = 0;
}

static void emit(const char *bytes, size_t len) {
    if (len >= sizeof(out_buffer)) {
        flush_output();
        fwrite(bytes, 1, len, stdout);
        return;
    }
    if (out_len + len > sizeof(out_buffer)) flush_output();
    memcpy(out_buffer + out_len, bytes, len);
    out_len += len;
}

static const char *trap_message(uint32_t code) {
    switch (code) {
        case 0: return "integer overflow";
        case 1: return "division by zero";
        case 2: return "remainder by zero";
        case 3: return "invalid conversion";
        case 4: return "index out of bounds";
        case 5: return "dictionary key not found";
        case 6: return "could not read or write that file";
        default: return "assertion failed";
    }
}

static const char *trap_code(uint32_t code) {
    switch (code) {
        case 0: return "DVK5001";
        case 1: return "DVK5002";
        case 2: return "DVK5003";
        case 3: return "DVK5004";
        case 4: return "DVK5005";
        case 5: return "DVK5006";
        case 6: return "DVK5007";
        default: return "DVK5008";
    }
}

__attribute__((noreturn, cold)) void dovik_trap(uint32_t code, uint32_t line, uint32_t column) {
    flush_output();
    fflush(stdout);
    fprintf(stderr, "runtime error[%s] at %u:%u: %s\n", trap_code(code), line, column,
            trap_message(code));
    exit(1);
}

static void *checked_alloc(size_t bytes) {
    void *block = malloc(bytes);
    if (block == NULL) {
        flush_output();
        fprintf(stderr, "runtime error: out of memory\n");
        exit(1);
    }
    return block;
}

void dovik_retain(uint8_t tag, uint64_t word);
void dovik_release(uint8_t tag, uint64_t word);
static void reclaim(uint8_t tag, uint64_t word);

static inline Header *header_of(uint8_t tag, uint64_t word) {
    if (word == 0) return NULL;
    switch (tag) {
        case TAG_STR:
        case TAG_LIST:
        case TAG_DICT: return (Header *)(uintptr_t)word;
        default: return NULL;
    }
}

__attribute__((always_inline)) void dovik_retain(uint8_t tag, uint64_t word) {
    Header *header = header_of(tag, word);
    if (header != NULL) header->count += 1;
}

__attribute__((noinline, cold)) static void reclaim(uint8_t tag, uint64_t word) {
    Header *header = (Header *)(uintptr_t)word;
    live_objects -= 1;

    if (tag == TAG_STR) {
        Str *owned = (Str *)header;

        if (string_pool_len < POOL_LIMIT && owned->cap <= POOL_MAX_BYTES) {
            owned->len = 0;
            owned->frozen = 0;
            owned->next_free = string_pool;
            string_pool = owned;
            string_pool_len += 1;
            return;
        }

        free(owned->bytes);
        free(owned);
        return;
    }

    if (tag == TAG_LIST) {
        List *owned = (List *)header;

        if (owned->elem >= TAG_STR) {
            for (size_t i = 0; i < owned->len; i++)
                dovik_release(owned->elem, owned->items[i]);
        }

        if (list_pool_len < POOL_LIMIT && owned->cap * sizeof(uint64_t) <= POOL_MAX_BYTES) {
            owned->len = 0;
            owned->next_free = list_pool;
            list_pool = owned;
            list_pool_len += 1;
            return;
        }

        free(owned->items);
        free(owned);
        return;
    }

    Dict *owned = (Dict *)header;
    int keys_owned = owned->key_tag >= TAG_STR;
    int vals_owned = owned->val_tag >= TAG_STR;

    if (keys_owned || vals_owned) {
        for (size_t i = 0; i < owned->cap; i++) {
            if (!owned->state[i]) continue;
            if (keys_owned) dovik_release(owned->key_tag, owned->entries[i].key);
            if (vals_owned) dovik_release(owned->val_tag, owned->entries[i].val);
        }
    }

    free(owned->entries);
    free(owned);
}

__attribute__((always_inline)) void dovik_release(uint8_t tag, uint64_t word) {
    Header *header = header_of(tag, word);
    if (header == NULL) return;
    if (--header->count > 0) return;
    reclaim(tag, word);
}

int64_t dovik_live_objects(void) { return live_objects; }

void dovik_report_leaks(void) {
    flush_output();
    fflush(stdout);
    if (getenv("DOVIK_CHECK_LEAKS") == NULL) return;
    fprintf(stderr, "dovik: %lld live object(s) at exit\n", (long long)live_objects);
}

static Str *new_string_with_capacity(size_t len, size_t cap) {
    live_objects += 1;

    if (string_pool != NULL && string_pool->cap >= cap) {
        Str *value = string_pool;
        string_pool = value->next_free;
        string_pool_len -= 1;
        value->h.count = 1;
        value->len = len;
        value->frozen = 0;
        value->hashed = 0;
        value->bytes[len] = 0;
        return value;
    }

    Str *value = checked_alloc(sizeof(Str));
    value->h.count = 1;
    value->len = len;
    value->cap = cap;
    value->frozen = 0;
    value->hashed = 0;
    value->bytes = checked_alloc(cap + 1);
    value->bytes[len] = 0;
    return value;
}

static Str *new_string(size_t len) { return new_string_with_capacity(len, len); }

uint64_t dovik_str_new(const char *bytes, int64_t len) {
    Str *value = new_string((size_t)len);
    memcpy(value->bytes, bytes, (size_t)len);
    value->frozen = 1;
    return (uint64_t)(uintptr_t)value;
}

static inline int can_extend_in_place(Str *a, Str *b) {
    return a->h.count == 1 && !a->frozen && a != b;
}

uint64_t dovik_str_concat(uint64_t left, uint64_t right) {
    Str *a = (Str *)(uintptr_t)left;
    Str *b = (Str *)(uintptr_t)right;

    if (can_extend_in_place(a, b)) {
        size_t needed = a->len + b->len;

        if (needed > a->cap) {
            size_t cap = a->cap < 16 ? 16 : a->cap;
            while (cap < needed) cap *= 2;
            char *bytes = checked_alloc(cap + 1);
            memcpy(bytes, a->bytes, a->len);
            free(a->bytes);
            a->bytes = bytes;
            a->cap = cap;
        }

        memcpy(a->bytes + a->len, b->bytes, b->len);
        a->len = needed;
        a->bytes[needed] = 0;
        a->hashed = 0;
        a->h.count += 1;
        return (uint64_t)(uintptr_t)a;
    }

    size_t total = a->len + b->len;
    size_t cap = total < 16 ? 16 : total;
    Str *value = new_string_with_capacity(total, cap);
    memcpy(value->bytes, a->bytes, a->len);
    memcpy(value->bytes + a->len, b->bytes, b->len);
    return (uint64_t)(uintptr_t)value;
}

int64_t dovik_str_byte_len(uint64_t word) { return (int64_t)((Str *)(uintptr_t)word)->len; }

static inline int is_continuation(unsigned char byte) { return (byte & 0xC0) == 0x80; }

int64_t dovik_str_scalar_len(uint64_t word) {
    Str *value = (Str *)(uintptr_t)word;
    int64_t count = 0;
    for (size_t i = 0; i < value->len; i++)
        if (!is_continuation((unsigned char)value->bytes[i])) count++;
    return count;
}

static inline size_t scalar_width(unsigned char lead) {
    if (lead < 0x80) return 1;
    if ((lead & 0xE0) == 0xC0) return 2;
    if ((lead & 0xF0) == 0xE0) return 3;
    if ((lead & 0xF8) == 0xF0) return 4;
    return 1;
}

uint64_t dovik_str_scalar_at(uint64_t word, int64_t index, uint32_t line, uint32_t column) {
    Str *value = (Str *)(uintptr_t)word;
    if (index < 0) dovik_trap(4, line, column);

    size_t offset = 0;
    int64_t seen = 0;
    while (offset < value->len) {
        size_t width = scalar_width((unsigned char)value->bytes[offset]);
        if (seen == index) {
            Str *scalar = new_string(width);
            memcpy(scalar->bytes, value->bytes + offset, width);
            return (uint64_t)(uintptr_t)scalar;
        }
        offset += width;
        seen++;
    }

    dovik_trap(4, line, column);
}

uint64_t dovik_str_next(uint64_t word, int64_t offset, int64_t *next) {
    Str *value = (Str *)(uintptr_t)word;
    size_t start = offset < 0 ? 0 : (size_t)offset;

    if (start >= value->len) {
        *next = (int64_t)value->len;
        return (uint64_t)(uintptr_t)new_string(0);
    }

    size_t width = scalar_width((unsigned char)value->bytes[start]);
    if (start + width > value->len) width = value->len - start;
    Str *scalar = new_string(width);
    memcpy(scalar->bytes, value->bytes + start, width);
    *next = (int64_t)(start + width);
    return (uint64_t)(uintptr_t)scalar;
}

int32_t dovik_str_cmp(uint64_t left, uint64_t right) {
    if (left == right) return 0;
    Str *a = (Str *)(uintptr_t)left;
    Str *b = (Str *)(uintptr_t)right;
    size_t shorter = a->len < b->len ? a->len : b->len;
    int ordering = memcmp(a->bytes, b->bytes, shorter);
    if (ordering != 0) return ordering < 0 ? -1 : 1;
    if (a->len == b->len) return 0;
    return a->len < b->len ? -1 : 1;
}

uint64_t dovik_list_new(uint8_t elem) {
    live_objects += 1;

    if (list_pool != NULL) {
        List *value = list_pool;
        list_pool = value->next_free;
        list_pool_len -= 1;
        value->h.count = 1;
        value->len = 0;
        value->elem = elem;
        return (uint64_t)(uintptr_t)value;
    }

    List *value = checked_alloc(sizeof(List));
    value->h.count = 1;
    value->len = 0;
    value->cap = 0;
    value->elem = elem;
    value->items = NULL;
    return (uint64_t)(uintptr_t)value;
}

static void reserve(List *value, size_t needed) {
    if (needed <= value->cap) return;
    size_t cap = value->cap < 4 ? 4 : value->cap;
    while (cap < needed) cap *= 2;
    uint64_t *items = checked_alloc(cap * sizeof(uint64_t));
    if (value->items != NULL) {
        memcpy(items, value->items, value->len * sizeof(uint64_t));
        free(value->items);
    }
    value->items = items;
    value->cap = cap;
}

uint64_t dovik_list_new_sized(uint8_t elem, int64_t wanted) {
    uint64_t handle = dovik_list_new(elem);
    List *value = (List *)(uintptr_t)handle;

    if (wanted > 0 && (size_t)wanted > value->cap) {
        free(value->items);
        value->items = checked_alloc((size_t)wanted * sizeof(uint64_t));
        value->cap = (size_t)wanted;
    }

    return handle;
}

void dovik_list_push_fast(uint64_t list, uint64_t word) {
    List *value = (List *)(uintptr_t)list;
    if (value->elem >= TAG_STR) dovik_retain(value->elem, word);
    value->items[value->len] = word;
    value->len += 1;
}

void dovik_list_push(uint64_t list, uint64_t word) {
    List *value = (List *)(uintptr_t)list;
    if (value->elem >= TAG_STR) dovik_retain(value->elem, word);
    reserve(value, value->len + 1);
    value->items[value->len] = word;
    value->len += 1;
}

int64_t dovik_list_len(uint64_t list) { return (int64_t)((List *)(uintptr_t)list)->len; }

uint64_t dovik_list_get(uint64_t list, int64_t index, uint32_t line, uint32_t column) {
    List *value = (List *)(uintptr_t)list;
    if (index < 0 || (size_t)index >= value->len) dovik_trap(4, line, column);
    return value->items[index];
}

uint64_t dovik_list_get_fast(uint64_t list, int64_t index) {
    return ((List *)(uintptr_t)list)->items[index];
}

void dovik_list_set(uint64_t list, int64_t index, uint64_t word, uint32_t line,
                    uint32_t column) {
    List *value = (List *)(uintptr_t)list;
    if (index < 0 || (size_t)index >= value->len) dovik_trap(4, line, column);

    if (value->elem < TAG_STR) {
        value->items[index] = word;
        return;
    }

    dovik_retain(value->elem, word);
    uint64_t previous = value->items[index];
    value->items[index] = word;
    dovik_release(value->elem, previous);
}

void dovik_list_insert(uint64_t list, int64_t index, uint64_t word, uint32_t line,
                       uint32_t column) {
    List *value = (List *)(uintptr_t)list;
    if (index < 0 || (size_t)index > value->len) dovik_trap(4, line, column);
    if (value->elem >= TAG_STR) dovik_retain(value->elem, word);
    reserve(value, value->len + 1);
    memmove(value->items + index + 1, value->items + index,
            (value->len - (size_t)index) * sizeof(uint64_t));
    value->items[index] = word;
    value->len += 1;
}

uint64_t dovik_list_remove_at(uint64_t list, int64_t index, uint32_t line, uint32_t column) {
    List *value = (List *)(uintptr_t)list;
    if (index < 0 || (size_t)index >= value->len) dovik_trap(4, line, column);
    uint64_t removed = value->items[index];
    memmove(value->items + index, value->items + index + 1,
            (value->len - (size_t)index - 1) * sizeof(uint64_t));
    value->len -= 1;
    return removed;
}

static uint64_t hash_cell(Cell key) {
    if (key.tag != TAG_STR) {
        uint64_t mixed = key.word * 0x9E3779B97F4A7C15ull;
        return mixed ^ (mixed >> 29);
    }

    Str *text = (Str *)(uintptr_t)key.word;
    if (text->hashed) return text->hash;

    uint64_t hashed = 1469598103934665603ull;
    for (size_t i = 0; i < text->len; i++) {
        hashed ^= (unsigned char)text->bytes[i];
        hashed *= 1099511628211ull;
    }

    text->hash = hashed;
    text->hashed = 1;
    return hashed;
}

static int cells_equal(Cell a, Cell b) {
    if (a.tag != b.tag) return 0;
    if (a.word == b.word) return 1;
    if (a.tag != TAG_STR) return 0;

    Str *x = (Str *)(uintptr_t)a.word;
    Str *y = (Str *)(uintptr_t)b.word;
    if (x->len != y->len) return 0;
    if (x->hashed && y->hashed && x->hash != y->hash) return 0;
    return memcmp(x->bytes, y->bytes, x->len) == 0;
}

uint64_t dovik_dict_new(uint8_t key_tag, uint8_t val_tag) {
    Dict *value = checked_alloc(sizeof(Dict));
    value->h.count = 1;
    value->len = 0;
    value->cap = 0;
    value->key_tag = key_tag;
    value->val_tag = val_tag;
    value->entries = NULL;
    value->state = NULL;
    live_objects += 1;
    return (uint64_t)(uintptr_t)value;
}

static inline size_t probe(Dict *dict, uint64_t key) {
    size_t mask = dict->cap - 1;
    Cell probe_key = {dict->key_tag, key};
    size_t slot = (size_t)hash_cell(probe_key) & mask;

    while (dict->state[slot]) {
        Cell stored = {dict->key_tag, dict->entries[slot].key};
        if (cells_equal(stored, probe_key)) break;
        slot = (slot + 1) & mask;
    }

    return slot;
}

static void grow(Dict *dict) {
    size_t cap = dict->cap < 8 ? 8 : dict->cap * 2;
    uint8_t *block = checked_alloc(cap * sizeof(Entry) + cap);
    Entry *entries = (Entry *)block;
    uint8_t *state = block + cap * sizeof(Entry);
    memset(state, 0, cap);

    Dict moved = {dict->h, 0, cap, dict->key_tag, dict->val_tag, entries, state};

    for (size_t i = 0; i < dict->cap; i++) {
        if (!dict->state[i]) continue;
        size_t slot = probe(&moved, dict->entries[i].key);
        moved.entries[slot] = dict->entries[i];
        moved.state[slot] = 1;
        moved.len += 1;
    }

    free(dict->entries);
    dict->entries = entries;
    dict->state = state;
    dict->cap = cap;
    dict->len = moved.len;
}

void dovik_dict_set(uint64_t dict, uint8_t ktag, uint64_t kword, uint8_t vtag, uint64_t vword) {
    Dict *value = (Dict *)(uintptr_t)dict;
    if (value->len * 4 >= value->cap * 3) grow(value);

    size_t slot = probe(value, kword);
    dovik_retain(vtag, vword);

    if (value->state[slot]) {
        uint64_t previous = value->entries[slot].val;
        value->entries[slot].val = vword;
        dovik_release(vtag, previous);
        return;
    }

    dovik_retain(ktag, kword);
    value->entries[slot].key = kword;
    value->entries[slot].val = vword;
    value->state[slot] = 1;
    value->len += 1;
}

uint64_t dovik_dict_get(uint64_t dict, uint8_t ktag, uint64_t kword, uint32_t line,
                        uint32_t column) {
    Dict *value = (Dict *)(uintptr_t)dict;
    if (value->cap == 0) dovik_trap(5, line, column);
    (void)ktag;
    size_t slot = probe(value, kword);
    if (!value->state[slot]) dovik_trap(5, line, column);
    return value->entries[slot].val;
}

int64_t dovik_dict_len(uint64_t dict) { return (int64_t)((Dict *)(uintptr_t)dict)->len; }

uint8_t dovik_dict_has_key(uint64_t dict, uint8_t ktag, uint64_t kword) {
    Dict *value = (Dict *)(uintptr_t)dict;
    if (value->cap == 0) return 0;
    (void)ktag;
    return value->state[probe(value, kword)] ? 1 : 0;
}

uint8_t dovik_dict_remove_key(uint64_t dict, uint8_t ktag, uint64_t kword) {
    Dict *value = (Dict *)(uintptr_t)dict;
    if (value->cap == 0) return 0;

    (void)ktag;
    size_t slot = probe(value, kword);
    if (!value->state[slot]) return 0;

    dovik_release(value->key_tag, value->entries[slot].key);
    dovik_release(value->val_tag, value->entries[slot].val);
    value->state[slot] = 0;
    value->len -= 1;

    size_t mask = value->cap - 1;
    size_t next = (slot + 1) & mask;
    while (value->state[next]) {
        Entry moved = value->entries[next];
        value->state[next] = 0;
        size_t target = probe(value, moved.key);
        value->entries[target] = moved;
        value->state[target] = 1;
        next = (next + 1) & mask;
    }

    return 1;
}

static size_t render_float(double value, char *buffer, size_t size) {
    if (value != value) return (size_t)snprintf(buffer, size, "nan");
    if (value > 1.7976931348623157e308) return (size_t)snprintf(buffer, size, "inf");
    if (value < -1.7976931348623157e308) return (size_t)snprintf(buffer, size, "-inf");

    for (int digits = 1; digits <= 17; digits++) {
        int written = snprintf(buffer, size, "%.*g", digits, value);
        if (written > 0 && strtod(buffer, NULL) == value) return (size_t)written;
    }

    return (size_t)snprintf(buffer, size, "%.17g", value);
}

static size_t render(Cell cell, char *buffer, size_t size);

static size_t render_sequence(Cell cell, char *buffer, size_t size) {
    size_t used = 0;
    if (cell.tag == TAG_LIST) {
        List *list = (List *)(uintptr_t)cell.word;
        used += (size_t)snprintf(buffer + used, size - used, "[");
        for (size_t i = 0; i < list->len; i++) {
            if (i > 0) used += (size_t)snprintf(buffer + used, size - used, ", ");
            Cell item = {list->elem, list->items[i]};
            used += render(item, buffer + used, size - used);
        }
        used += (size_t)snprintf(buffer + used, size - used, "]");
        return used;
    }

    Dict *dict = (Dict *)(uintptr_t)cell.word;
    used += (size_t)snprintf(buffer + used, size - used, "{");
    int first = 1;
    for (size_t i = 0; i < dict->cap; i++) {
        if (!dict->state[i]) continue;
        if (!first) used += (size_t)snprintf(buffer + used, size - used, ", ");
        first = 0;
        Cell k = {dict->key_tag, dict->entries[i].key};
        Cell v = {dict->val_tag, dict->entries[i].val};
        used += render(k, buffer + used, size - used);
        used += (size_t)snprintf(buffer + used, size - used, ": ");
        used += render(v, buffer + used, size - used);
    }
    used += (size_t)snprintf(buffer + used, size - used, "}");
    return used;
}

static size_t render(Cell cell, char *buffer, size_t size) {
    switch (cell.tag) {
        case TAG_INT: return (size_t)snprintf(buffer, size, "%lld", (long long)cell.word);
        case TAG_FLOAT: {
            double value;
            memcpy(&value, &cell.word, sizeof(double));
            return render_float(value, buffer, size);
        }
        case TAG_BOOL:
            return (size_t)snprintf(buffer, size, "%s", cell.word != 0 ? "true" : "false");
        case TAG_STR: {
            Str *text = (Str *)(uintptr_t)cell.word;
            size_t len = text->len < size - 1 ? text->len : size - 1;
            memcpy(buffer, text->bytes, len);
            return len;
        }
        default: return render_sequence(cell, buffer, size);
    }
}

void dovik_print(uint8_t tag, uint64_t word) {
    if (tag == TAG_STR) {
        Str *text = (Str *)(uintptr_t)word;
        emit(text->bytes, text->len);
        emit("\n", 1);
        return;
    }

    char buffer[512];
    size_t len = render((Cell){tag, word}, buffer, sizeof(buffer));
    emit(buffer, len);
    emit("\n", 1);
}

uint64_t dovik_to_string(uint8_t tag, uint64_t word) {
    if (tag == TAG_STR) {
        dovik_retain(tag, word);
        return word;
    }

    char buffer[512];
    size_t len = render((Cell){tag, word}, buffer, sizeof(buffer));
    Str *value = new_string(len);
    memcpy(value->bytes, buffer, len);
    return (uint64_t)(uintptr_t)value;
}

static int parse_int(const char *text, size_t len, long long *out) {
    char buffer[64];
    if (len >= sizeof(buffer)) return 0;
    memcpy(buffer, text, len);
    buffer[len] = 0;

    char *end = NULL;
    const char *start = buffer;
    while (*start == ' ' || *start == '\t' || *start == '\n' || *start == '\r') start++;
    if (*start == 0) return 0;

    long long parsed = strtoll(start, &end, 10);
    if (end == start) return 0;
    while (*end == ' ' || *end == '\t' || *end == '\n' || *end == '\r') end++;
    if (*end != 0) return 0;

    *out = parsed;
    return 1;
}

static int parse_float_text(const char *text, size_t len, double *out) {
    char buffer[512];
    if (len >= sizeof(buffer)) return 0;
    memcpy(buffer, text, len);
    buffer[len] = 0;

    char *end = NULL;
    const char *start = buffer;
    while (*start == ' ' || *start == '\t' || *start == '\n' || *start == '\r') start++;
    if (*start == 0) return 0;

    double parsed = strtod(start, &end);
    if (end == start) return 0;
    while (*end == ' ' || *end == '\t' || *end == '\n' || *end == '\r') end++;
    if (*end != 0) return 0;
    if (parsed != parsed) return 0;
    if (parsed > 1.7976931348623157e308 || parsed < -1.7976931348623157e308) return 0;

    *out = parsed;
    return 1;
}

int64_t dovik_parse_int(uint64_t word, uint32_t line, uint32_t column) {
    Str *text = (Str *)(uintptr_t)word;
    long long parsed = 0;
    if (!parse_int(text->bytes, text->len, &parsed)) dovik_trap(3, line, column);
    return parsed;
}

double dovik_parse_float(uint64_t word, uint32_t line, uint32_t column) {
    Str *text = (Str *)(uintptr_t)word;
    double parsed = 0;
    if (!parse_float_text(text->bytes, text->len, &parsed)) dovik_trap(3, line, column);
    return parsed;
}

uint8_t dovik_can_parse_int(uint64_t word) {
    Str *text = (Str *)(uintptr_t)word;
    long long parsed = 0;
    return parse_int(text->bytes, text->len, &parsed) ? 1 : 0;
}

uint8_t dovik_can_parse_float(uint64_t word) {
    Str *text = (Str *)(uintptr_t)word;
    double parsed = 0;
    return parse_float_text(text->bytes, text->len, &parsed) ? 1 : 0;
}

static int cell_equal(Cell left, Cell right);

static int list_equal(List *a, List *b) {
    if (a->len != b->len) return 0;
    if (a->elem == TAG_INT || a->elem == TAG_BOOL)
        return memcmp(a->items, b->items, a->len * sizeof(uint64_t)) == 0;

    for (size_t i = 0; i < a->len; i++) {
        Cell x = {a->elem, a->items[i]};
        Cell y = {b->elem, b->items[i]};
        if (!cell_equal(x, y)) return 0;
    }
    return 1;
}

static int dict_equal(Dict *a, Dict *b) {
    if (a->len != b->len) return 0;
    for (size_t i = 0; i < a->cap; i++) {
        if (!a->state[i]) continue;
        if (b->cap == 0) return 0;
        size_t slot = probe(b, a->entries[i].key);
        if (!b->state[slot]) return 0;
        Cell x = {a->val_tag, a->entries[i].val};
        Cell y = {b->val_tag, b->entries[slot].val};
        if (!cell_equal(x, y)) return 0;
    }
    return 1;
}

static int cell_equal(Cell left, Cell right) {
    if (left.tag != right.tag) return 0;
    switch (left.tag) {
        case TAG_FLOAT: {
            double a, b;
            memcpy(&a, &left.word, sizeof(double));
            memcpy(&b, &right.word, sizeof(double));
            return a == b;
        }
        case TAG_STR: return dovik_str_cmp(left.word, right.word) == 0;
        case TAG_LIST:
            return list_equal((List *)(uintptr_t)left.word, (List *)(uintptr_t)right.word);
        case TAG_DICT:
            return dict_equal((Dict *)(uintptr_t)left.word, (Dict *)(uintptr_t)right.word);
        default: return left.word == right.word;
    }
}

uint8_t dovik_eq(uint8_t tag, uint64_t left, uint64_t right) {
    return cell_equal((Cell){tag, left}, (Cell){tag, right}) ? 1 : 0;
}

static int saved_argc = 0;
static char **saved_argv = NULL;

void dovik_set_arguments(int argc, char **argv) {
    saved_argc = argc > 0 ? argc - 1 : 0;
    saved_argv = argv;
}

int64_t dovik_argument_count(void) { return saved_argc; }

uint64_t dovik_argument(int64_t index, uint32_t line, uint32_t column) {
    if (index < 0 || index >= saved_argc) dovik_trap(4, line, column);
    const char *text = saved_argv[index + 1];
    return dovik_str_new(text, (int64_t)strlen(text));
}

uint8_t dovik_file_exists(uint64_t path) {
    FILE *handle = fopen(((Str *)(uintptr_t)path)->bytes, "rb");
    if (handle == NULL) return 0;
    fclose(handle);
    return 1;
}

uint64_t dovik_read_file(uint64_t path, uint32_t line, uint32_t column) {
    FILE *handle = fopen(((Str *)(uintptr_t)path)->bytes, "rb");
    if (handle == NULL) dovik_trap(6, line, column);

    size_t cap = 4096;
    size_t len = 0;
    char *bytes = checked_alloc(cap + 1);

    for (;;) {
        if (len == cap) {
            cap *= 2;
            char *grown = checked_alloc(cap + 1);
            memcpy(grown, bytes, len);
            free(bytes);
            bytes = grown;
        }

        size_t read = fread(bytes + len, 1, cap - len, handle);
        len += read;

        if (read == 0) break;
    }

    int failed = ferror(handle);
    fclose(handle);

    if (failed) {
        free(bytes);
        dovik_trap(6, line, column);
    }

    Str *value = new_string(len);
    memcpy(value->bytes, bytes, len);
    free(bytes);
    return (uint64_t)(uintptr_t)value;
}

static void write_bytes(uint64_t path, uint64_t contents, const char *mode, uint32_t line,
                        uint32_t column) {
    FILE *handle = fopen(((Str *)(uintptr_t)path)->bytes, mode);
    if (handle == NULL) dovik_trap(6, line, column);

    Str *text = (Str *)(uintptr_t)contents;
    size_t written = text->len == 0 ? 0 : fwrite(text->bytes, 1, text->len, handle);
    int failed = written != text->len || ferror(handle);

    if (fclose(handle) != 0 || failed) dovik_trap(6, line, column);
}

void dovik_write_file(uint64_t path, uint64_t contents, uint32_t line, uint32_t column) {
    write_bytes(path, contents, "wb", line, column);
}

void dovik_append_file(uint64_t path, uint64_t contents, uint32_t line, uint32_t column) {
    write_bytes(path, contents, "ab", line, column);
}

static size_t scalar_to_byte(const Str *text, int64_t index) {
    size_t offset = 0;
    int64_t seen = 0;

    while (offset < text->len && seen < index) {
        offset += scalar_width((unsigned char)text->bytes[offset]);
        seen++;
    }

    return seen < index ? text->len + 1 : offset;
}

static uint64_t make_string(const char *bytes, size_t len) {
    Str *value = new_string(len);
    if (len > 0) memcpy(value->bytes, bytes, len);
    return (uint64_t)(uintptr_t)value;
}

static const char *find_bytes(const Str *haystack, const Str *needle, size_t from) {
    if (needle->len == 0) return haystack->bytes + from;
    if (needle->len > haystack->len) return NULL;

    for (size_t i = from; i + needle->len <= haystack->len; i++)
        if (memcmp(haystack->bytes + i, needle->bytes, needle->len) == 0)
            return haystack->bytes + i;

    return NULL;
}

uint64_t dovik_str_split(uint64_t text, uint64_t separator) {
    Str *source = (Str *)(uintptr_t)text;
    Str *sep = (Str *)(uintptr_t)separator;
    uint64_t list = dovik_list_new(TAG_STR);

    if (sep->len == 0) {
        size_t offset = 0;
        while (offset < source->len) {
            size_t width = scalar_width((unsigned char)source->bytes[offset]);
            if (offset + width > source->len) width = source->len - offset;
            uint64_t piece = make_string(source->bytes + offset, width);
            dovik_list_push(list, piece);
            dovik_release(TAG_STR, piece);
            offset += width;
        }
        return list;
    }

    size_t start = 0;
    for (;;) {
        const char *hit = find_bytes(source, sep, start);

        if (hit == NULL) {
            uint64_t piece = make_string(source->bytes + start, source->len - start);
            dovik_list_push(list, piece);
            dovik_release(TAG_STR, piece);
            return list;
        }

        size_t at = (size_t)(hit - source->bytes);
        uint64_t piece = make_string(source->bytes + start, at - start);
        dovik_list_push(list, piece);
        dovik_release(TAG_STR, piece);
        start = at + sep->len;
    }
}

uint64_t dovik_str_join(uint64_t list, uint64_t separator) {
    List *parts = (List *)(uintptr_t)list;
    Str *sep = (Str *)(uintptr_t)separator;

    size_t total = 0;
    for (size_t i = 0; i < parts->len; i++)
        total += ((Str *)(uintptr_t)parts->items[i])->len;
    if (parts->len > 1) total += sep->len * (parts->len - 1);

    Str *value = new_string(total);
    size_t at = 0;

    for (size_t i = 0; i < parts->len; i++) {
        if (i > 0 && sep->len > 0) {
            memcpy(value->bytes + at, sep->bytes, sep->len);
            at += sep->len;
        }
        Str *piece = (Str *)(uintptr_t)parts->items[i];
        memcpy(value->bytes + at, piece->bytes, piece->len);
        at += piece->len;
    }

    return (uint64_t)(uintptr_t)value;
}

uint64_t dovik_str_substring(uint64_t text, int64_t start, int64_t count, uint32_t line,
                             uint32_t column) {
    Str *source = (Str *)(uintptr_t)text;

    if (start < 0 || count < 0) dovik_trap(4, line, column);

    size_t from = scalar_to_byte(source, start);
    if (from > source->len) dovik_trap(4, line, column);

    size_t to = scalar_to_byte(source, start + count);
    if (to > source->len) dovik_trap(4, line, column);

    return make_string(source->bytes + from, to - from);
}

uint8_t dovik_str_contains(uint64_t text, uint64_t needle) {
    Str *source = (Str *)(uintptr_t)text;
    Str *target = (Str *)(uintptr_t)needle;
    return find_bytes(source, target, 0) != NULL ? 1 : 0;
}

int64_t dovik_str_index_of(uint64_t text, uint64_t needle, uint32_t line, uint32_t column) {
    Str *source = (Str *)(uintptr_t)text;
    Str *target = (Str *)(uintptr_t)needle;
    const char *hit = find_bytes(source, target, 0);

    if (hit == NULL) dovik_trap(4, line, column);

    size_t at = (size_t)(hit - source->bytes);
    int64_t scalars = 0;
    for (size_t i = 0; i < at; i++)
        if (!is_continuation((unsigned char)source->bytes[i])) scalars++;

    return scalars;
}

static int is_space(unsigned char byte) {
    return byte == ' ' || byte == '\t' || byte == '\n' || byte == '\r' || byte == '\f' ||
           byte == '\v';
}

uint64_t dovik_str_trim(uint64_t text) {
    Str *source = (Str *)(uintptr_t)text;
    size_t start = 0;
    size_t end = source->len;

    while (start < end && is_space((unsigned char)source->bytes[start])) start++;
    while (end > start && is_space((unsigned char)source->bytes[end - 1])) end--;

    return make_string(source->bytes + start, end - start);
}

uint8_t dovik_str_starts_with(uint64_t text, uint64_t prefix) {
    Str *source = (Str *)(uintptr_t)text;
    Str *target = (Str *)(uintptr_t)prefix;
    if (target->len > source->len) return 0;
    return memcmp(source->bytes, target->bytes, target->len) == 0 ? 1 : 0;
}

uint8_t dovik_str_ends_with(uint64_t text, uint64_t suffix) {
    Str *source = (Str *)(uintptr_t)text;
    Str *target = (Str *)(uintptr_t)suffix;
    if (target->len > source->len) return 0;
    return memcmp(source->bytes + source->len - target->len, target->bytes, target->len) == 0
               ? 1
               : 0;
}

static uint64_t recase(uint64_t text, int to_upper) {
    Str *source = (Str *)(uintptr_t)text;
    Str *value = new_string(source->len);

    for (size_t i = 0; i < source->len; i++) {
        unsigned char byte = (unsigned char)source->bytes[i];
        if (to_upper && byte >= 'a' && byte <= 'z') byte -= 32;
        if (!to_upper && byte >= 'A' && byte <= 'Z') byte += 32;
        value->bytes[i] = (char)byte;
    }

    return (uint64_t)(uintptr_t)value;
}

uint64_t dovik_str_upper(uint64_t text) { return recase(text, 1); }
uint64_t dovik_str_lower(uint64_t text) { return recase(text, 0); }

uint64_t dovik_str_replace(uint64_t text, uint64_t from, uint64_t to) {
    Str *source = (Str *)(uintptr_t)text;
    Str *needle = (Str *)(uintptr_t)from;
    Str *patch = (Str *)(uintptr_t)to;

    if (needle->len == 0) {
        dovik_retain(TAG_STR, text);
        return text;
    }

    size_t cap = source->len + 16;
    size_t len = 0;
    char *bytes = checked_alloc(cap + 1);
    size_t at = 0;

    while (at < source->len) {
        const char *hit = find_bytes(source, needle, at);
        size_t stop = hit == NULL ? source->len : (size_t)(hit - source->bytes);
        size_t chunk = stop - at;
        size_t extra = chunk + (hit == NULL ? 0 : patch->len);

        if (len + extra > cap) {
            while (len + extra > cap) cap *= 2;
            char *grown = checked_alloc(cap + 1);
            memcpy(grown, bytes, len);
            free(bytes);
            bytes = grown;
        }

        memcpy(bytes + len, source->bytes + at, chunk);
        len += chunk;

        if (hit == NULL) break;

        memcpy(bytes + len, patch->bytes, patch->len);
        len += patch->len;
        at = stop + needle->len;
    }

    uint64_t result = make_string(bytes, len);
    free(bytes);
    return result;
}

uint64_t dovik_read_input(uint32_t line, uint32_t column) {
    size_t cap = 4096;
    size_t len = 0;
    char *bytes = checked_alloc(cap + 1);

    for (;;) {
        if (len == cap) {
            cap *= 2;
            char *grown = checked_alloc(cap + 1);
            memcpy(grown, bytes, len);
            free(bytes);
            bytes = grown;
        }

        size_t read = fread(bytes + len, 1, cap - len, stdin);
        len += read;

        if (read == 0) break;
    }

    if (ferror(stdin)) {
        free(bytes);
        dovik_trap(6, line, column);
    }

    uint64_t result = make_string(bytes, len);
    free(bytes);
    return result;
}

void dovik_assert(uint8_t condition, uint32_t line, uint32_t column) {
    if (!condition) dovik_trap(7, line, column);
}

int64_t dovik_test_index(void) {
    if (saved_argc < 1) return -1;
    char *end = NULL;
    long long parsed = strtoll(saved_argv[1], &end, 10);
    if (end == saved_argv[1] || *end != 0) return -1;
    return parsed;
}
