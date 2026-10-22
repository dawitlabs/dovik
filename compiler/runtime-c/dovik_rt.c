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
    Cell *items;
    struct List *next_free;
} List;

#define TAG_EMPTY 255

typedef struct {
    Cell key;
    Cell val;
} Entry;

typedef struct {
    Header h;
    size_t len;
    size_t cap;
    Entry *entries;
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
        default: return "dictionary key not found";
    }
}

static const char *trap_code(uint32_t code) {
    switch (code) {
        case 0: return "DVK5001";
        case 1: return "DVK5002";
        case 2: return "DVK5003";
        case 3: return "DVK5004";
        case 4: return "DVK5005";
        default: return "DVK5006";
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

static inline Header *header_of(uint8_t tag, uint64_t word) {
    if (word == 0) return NULL;
    switch (tag) {
        case TAG_STR:
        case TAG_LIST:
        case TAG_DICT: return (Header *)(uintptr_t)word;
        default: return NULL;
    }
}

void dovik_retain(uint8_t tag, uint64_t word) {
    Header *header = header_of(tag, word);
    if (header != NULL) header->count += 1;
}

void dovik_release(uint8_t tag, uint64_t word) {
    Header *header = header_of(tag, word);
    if (header == NULL) return;
    if (--header->count > 0) return;

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
        for (size_t i = 0; i < owned->len; i++)
            dovik_release(owned->items[i].tag, owned->items[i].word);

        if (list_pool_len < POOL_LIMIT && owned->cap * sizeof(Cell) <= POOL_MAX_BYTES) {
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
    for (size_t i = 0; i < owned->cap; i++) {
        if (owned->entries[i].key.tag == TAG_EMPTY) continue;
        dovik_release(owned->entries[i].key.tag, owned->entries[i].key.word);
        dovik_release(owned->entries[i].val.tag, owned->entries[i].val.word);
    }
    free(owned->entries);
    free(owned);
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

uint64_t dovik_list_new(void) {
    live_objects += 1;

    if (list_pool != NULL) {
        List *value = list_pool;
        list_pool = value->next_free;
        list_pool_len -= 1;
        value->h.count = 1;
        value->len = 0;
        return (uint64_t)(uintptr_t)value;
    }

    List *value = checked_alloc(sizeof(List));
    value->h.count = 1;
    value->len = 0;
    value->cap = 0;
    value->items = NULL;
    return (uint64_t)(uintptr_t)value;
}

static void reserve(List *value, size_t needed) {
    if (needed <= value->cap) return;
    size_t cap = value->cap < 4 ? 4 : value->cap;
    while (cap < needed) cap *= 2;
    Cell *items = checked_alloc(cap * sizeof(Cell));
    if (value->items != NULL) {
        memcpy(items, value->items, value->len * sizeof(Cell));
        free(value->items);
    }
    value->items = items;
    value->cap = cap;
}

void dovik_list_push(uint64_t list, uint8_t tag, uint64_t word) {
    List *value = (List *)(uintptr_t)list;
    dovik_retain(tag, word);
    reserve(value, value->len + 1);
    value->items[value->len].tag = tag;
    value->items[value->len].word = word;
    value->len += 1;
}

int64_t dovik_list_len(uint64_t list) { return (int64_t)((List *)(uintptr_t)list)->len; }

uint64_t dovik_list_get(uint64_t list, int64_t index, uint32_t line, uint32_t column) {
    List *value = (List *)(uintptr_t)list;
    if (index < 0 || (size_t)index >= value->len) dovik_trap(4, line, column);
    return value->items[index].word;
}

uint64_t dovik_list_get_fast(uint64_t list, int64_t index) {
    return ((List *)(uintptr_t)list)->items[index].word;
}

void dovik_list_set(uint64_t list, int64_t index, uint8_t tag, uint64_t word, uint32_t line,
                    uint32_t column) {
    List *value = (List *)(uintptr_t)list;
    if (index < 0 || (size_t)index >= value->len) dovik_trap(4, line, column);
    dovik_retain(tag, word);
    Cell previous = value->items[index];
    value->items[index].tag = tag;
    value->items[index].word = word;
    dovik_release(previous.tag, previous.word);
}

void dovik_list_insert(uint64_t list, int64_t index, uint8_t tag, uint64_t word, uint32_t line,
                       uint32_t column) {
    List *value = (List *)(uintptr_t)list;
    if (index < 0 || (size_t)index > value->len) dovik_trap(4, line, column);
    dovik_retain(tag, word);
    reserve(value, value->len + 1);
    memmove(value->items + index + 1, value->items + index,
            (value->len - (size_t)index) * sizeof(Cell));
    value->items[index].tag = tag;
    value->items[index].word = word;
    value->len += 1;
}

uint64_t dovik_list_remove_at(uint64_t list, int64_t index, uint32_t line, uint32_t column) {
    List *value = (List *)(uintptr_t)list;
    if (index < 0 || (size_t)index >= value->len) dovik_trap(4, line, column);
    Cell removed = value->items[index];
    memmove(value->items + index, value->items + index + 1,
            (value->len - (size_t)index - 1) * sizeof(Cell));
    value->len -= 1;
    return removed.word;
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

uint64_t dovik_dict_new(void) {
    Dict *value = checked_alloc(sizeof(Dict));
    value->h.count = 1;
    value->len = 0;
    value->cap = 0;
    value->entries = NULL;
    live_objects += 1;
    return (uint64_t)(uintptr_t)value;
}

static inline size_t probe(Dict *dict, Cell key) {
    size_t mask = dict->cap - 1;
    size_t slot = (size_t)hash_cell(key) & mask;
    Entry *entries = dict->entries;

    while (entries[slot].key.tag != TAG_EMPTY && !cells_equal(entries[slot].key, key))
        slot = (slot + 1) & mask;

    return slot;
}

static void grow(Dict *dict) {
    size_t cap = dict->cap < 8 ? 8 : dict->cap * 2;
    Entry *entries = checked_alloc(cap * sizeof(Entry));
    for (size_t i = 0; i < cap; i++) entries[i].key.tag = TAG_EMPTY;

    Dict moved = {dict->h, 0, cap, entries};

    for (size_t i = 0; i < dict->cap; i++) {
        if (dict->entries[i].key.tag == TAG_EMPTY) continue;
        size_t slot = probe(&moved, dict->entries[i].key);
        moved.entries[slot] = dict->entries[i];
        moved.len += 1;
    }

    free(dict->entries);
    dict->entries = entries;
    dict->cap = cap;
    dict->len = moved.len;
}

void dovik_dict_set(uint64_t dict, uint8_t ktag, uint64_t kword, uint8_t vtag, uint64_t vword) {
    Dict *value = (Dict *)(uintptr_t)dict;
    if (value->len * 4 >= value->cap * 3) grow(value);

    Cell key = {ktag, kword};
    Cell item = {vtag, vword};
    size_t slot = probe(value, key);

    dovik_retain(vtag, vword);

    if (value->entries[slot].key.tag != TAG_EMPTY) {
        Cell previous = value->entries[slot].val;
        value->entries[slot].val = item;
        dovik_release(previous.tag, previous.word);
        return;
    }

    dovik_retain(ktag, kword);
    value->entries[slot].key = key;
    value->entries[slot].val = item;
    value->len += 1;
}

uint64_t dovik_dict_get(uint64_t dict, uint8_t ktag, uint64_t kword, uint32_t line,
                        uint32_t column) {
    Dict *value = (Dict *)(uintptr_t)dict;
    if (value->cap == 0) dovik_trap(5, line, column);
    Cell key = {ktag, kword};
    size_t slot = probe(value, key);
    if (value->entries[slot].key.tag == TAG_EMPTY) dovik_trap(5, line, column);
    return value->entries[slot].val.word;
}

int64_t dovik_dict_len(uint64_t dict) { return (int64_t)((Dict *)(uintptr_t)dict)->len; }

uint8_t dovik_dict_has_key(uint64_t dict, uint8_t ktag, uint64_t kword) {
    Dict *value = (Dict *)(uintptr_t)dict;
    if (value->cap == 0) return 0;
    Cell key = {ktag, kword};
    return value->entries[probe(value, key)].key.tag != TAG_EMPTY ? 1 : 0;
}

uint8_t dovik_dict_remove_key(uint64_t dict, uint8_t ktag, uint64_t kword) {
    Dict *value = (Dict *)(uintptr_t)dict;
    if (value->cap == 0) return 0;

    Cell key = {ktag, kword};
    size_t slot = probe(value, key);
    if (value->entries[slot].key.tag == TAG_EMPTY) return 0;

    dovik_release(value->entries[slot].key.tag, value->entries[slot].key.word);
    dovik_release(value->entries[slot].val.tag, value->entries[slot].val.word);
    value->entries[slot].key.tag = TAG_EMPTY;
    value->len -= 1;

    size_t mask = value->cap - 1;
    size_t next = (slot + 1) & mask;
    while (value->entries[next].key.tag != TAG_EMPTY) {
        Entry moved = value->entries[next];
        value->entries[next].key.tag = TAG_EMPTY;
        size_t target = probe(value, moved.key);
        value->entries[target] = moved;
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
            used += render(list->items[i], buffer + used, size - used);
        }
        used += (size_t)snprintf(buffer + used, size - used, "]");
        return used;
    }

    Dict *dict = (Dict *)(uintptr_t)cell.word;
    used += (size_t)snprintf(buffer + used, size - used, "{");
    int first = 1;
    for (size_t i = 0; i < dict->cap; i++) {
        if (dict->entries[i].key.tag == TAG_EMPTY) continue;
        if (!first) used += (size_t)snprintf(buffer + used, size - used, ", ");
        first = 0;
        used += render(dict->entries[i].key, buffer + used, size - used);
        used += (size_t)snprintf(buffer + used, size - used, ": ");
        used += render(dict->entries[i].val, buffer + used, size - used);
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
    for (size_t i = 0; i < a->len; i++)
        if (!cell_equal(a->items[i], b->items[i])) return 0;
    return 1;
}

static int dict_equal(Dict *a, Dict *b) {
    if (a->len != b->len) return 0;
    for (size_t i = 0; i < a->cap; i++) {
        if (a->entries[i].key.tag == TAG_EMPTY) continue;
        if (b->cap == 0) return 0;
        size_t slot = probe(b, a->entries[i].key);
        if (b->entries[slot].key.tag == TAG_EMPTY) return 0;
        if (!cell_equal(a->entries[i].val, b->entries[slot].val)) return 0;
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
