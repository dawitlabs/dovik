use std::collections::HashMap;
use std::io::Write;

pub const TAG_INT: u8 = 0;
pub const TAG_FLOAT: u8 = 1;
pub const TAG_BOOL: u8 = 2;
pub const TAG_STR: u8 = 3;
pub const TAG_LIST: u8 = 4;
pub const TAG_DICT: u8 = 5;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Cell {
    tag: u8,
    word: u64,
}

struct Str {
    bytes: String,
}

struct List {
    items: Vec<Cell>,
}

struct Dict {
    entries: HashMap<Cell, Cell>,
    order: Vec<Cell>,
}

fn leak<T>(value: T) -> u64 {
    Box::into_raw(Box::new(value)) as u64
}

unsafe fn borrow<T>(word: u64) -> &'static mut T {
    unsafe { &mut *(word as *mut T) }
}

fn text(word: u64) -> &'static str {
    unsafe { borrow::<Str>(word).bytes.as_str() }
}

fn code_name(code: u32) -> &'static str {
    match code {
        0 => "DVK5001",
        1 => "DVK5002",
        2 => "DVK5003",
        3 => "DVK5004",
        4 => "DVK5005",
        _ => "DVK5006",
    }
}

fn message(code: u32) -> &'static str {
    match code {
        0 => "integer overflow",
        1 => "division by zero",
        2 => "remainder by zero",
        3 => "`to_int` cannot convert this value: NaN, infinity, or out of range",
        4 => "index out of bounds",
        _ => "dictionary key not found",
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_trap(code: u32, line: u32, column: u32) -> ! {
    let mut err = std::io::stderr();
    let _ = writeln!(
        err,
        "runtime error[{}] at {line}:{column}: {}",
        code_name(code),
        message(code)
    );
    let _ = err.flush();
    std::process::exit(1);
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_print(tag: u8, word: u64) {
    let mut out = std::io::stdout();
    let _ = writeln!(out, "{}", render(Cell { tag, word }));
}

fn render(cell: Cell) -> String {
    match cell.tag {
        TAG_INT => (cell.word as i64).to_string(),
        TAG_FLOAT => float(f64::from_bits(cell.word)),
        TAG_BOOL => match cell.word != 0 {
            true => "true".to_string(),
            false => "false".to_string(),
        },
        TAG_STR => text(cell.word).to_string(),
        TAG_LIST => {
            let items = unsafe { &borrow::<List>(cell.word).items };
            let rendered: Vec<String> = items.iter().map(|item| render(*item)).collect();
            format!("[{}]", rendered.join(", "))
        }
        _ => {
            let dict = unsafe { borrow::<Dict>(cell.word) };
            let rendered: Vec<String> = dict
                .order
                .iter()
                .filter_map(|key| dict.entries.get(key).map(|value| (key, value)))
                .map(|(key, value)| format!("{}: {}", render(*key), render(*value)))
                .collect();
            format!("{{{}}}", rendered.join(", "))
        }
    }
}

fn float(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }

    if value.is_infinite() {
        return match value.is_sign_negative() {
            true => "-inf".to_string(),
            false => "inf".to_string(),
        };
    }

    format!("{value}")
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_str_new(bytes: *const u8, len: u64) -> u64 {
    let slice = unsafe { std::slice::from_raw_parts(bytes, len as usize) };
    let owned = String::from_utf8_lossy(slice).into_owned();
    leak(Str { bytes: owned })
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_str_concat(left: u64, right: u64) -> u64 {
    let mut joined = String::with_capacity(text(left).len() + text(right).len());
    joined.push_str(text(left));
    joined.push_str(text(right));
    leak(Str { bytes: joined })
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_str_scalar_len(word: u64) -> i64 {
    text(word).chars().count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_str_byte_len(word: u64) -> i64 {
    text(word).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_str_scalar_at(word: u64, index: i64, line: u32, column: u32) -> u64 {
    if index < 0 {
        dovik_trap(4, line, column);
    }

    match text(word).chars().nth(index as usize) {
        Some(scalar) => leak(Str {
            bytes: scalar.to_string(),
        }),
        None => dovik_trap(4, line, column),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_str_next(word: u64, offset: i64, next: *mut i64) -> u64 {
    let source = text(word);
    let start = offset.max(0) as usize;

    if start >= source.len() {
        unsafe { *next = source.len() as i64 };
        return leak(Str {
            bytes: String::new(),
        });
    }

    match source[start..].chars().next() {
        Some(scalar) => {
            unsafe { *next = (start + scalar.len_utf8()) as i64 };
            leak(Str {
                bytes: scalar.to_string(),
            })
        }
        None => {
            unsafe { *next = source.len() as i64 };
            leak(Str {
                bytes: String::new(),
            })
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_str_cmp(left: u64, right: u64) -> i32 {
    match text(left).chars().cmp(text(right).chars()) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_list_new() -> u64 {
    leak(List { items: Vec::new() })
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_list_push(list: u64, tag: u8, word: u64) {
    unsafe { borrow::<List>(list).items.push(Cell { tag, word }) };
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_list_len(list: u64) -> i64 {
    unsafe { borrow::<List>(list).items.len() as i64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_list_get(list: u64, index: i64, line: u32, column: u32) -> u64 {
    let items = unsafe { &borrow::<List>(list).items };

    if index < 0 || index as usize >= items.len() {
        dovik_trap(4, line, column);
    }

    items[index as usize].word
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_list_set(
    list: u64,
    index: i64,
    tag: u8,
    word: u64,
    line: u32,
    column: u32,
) {
    let items = unsafe { &mut borrow::<List>(list).items };

    if index < 0 || index as usize >= items.len() {
        dovik_trap(4, line, column);
    }

    items[index as usize] = Cell { tag, word };
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_dict_new() -> u64 {
    leak(Dict {
        entries: HashMap::new(),
        order: Vec::new(),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_dict_set(dict: u64, ktag: u8, kword: u64, vtag: u8, vword: u64) {
    let target = unsafe { borrow::<Dict>(dict) };
    let key = normalized(Cell {
        tag: ktag,
        word: kword,
    });

    if target.entries.insert(key, Cell { tag: vtag, word: vword }).is_none() {
        target.order.push(key);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_dict_get(
    dict: u64,
    ktag: u8,
    kword: u64,
    line: u32,
    column: u32,
) -> u64 {
    let target = unsafe { borrow::<Dict>(dict) };
    let key = normalized(Cell {
        tag: ktag,
        word: kword,
    });

    match target.entries.get(&key) {
        Some(value) => value.word,
        None => dovik_trap(5, line, column),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_dict_len(dict: u64) -> i64 {
    unsafe { borrow::<Dict>(dict).entries.len() as i64 }
}

fn normalized(cell: Cell) -> Cell {
    if cell.tag != TAG_STR {
        return cell;
    }

    let mut hashed = 1469598103934665603u64;

    for byte in text(cell.word).as_bytes() {
        hashed ^= u64::from(*byte);
        hashed = hashed.wrapping_mul(1099511628211);
    }

    Cell {
        tag: TAG_STR,
        word: hashed,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dovik_eq(tag: u8, left: u64, right: u64) -> u8 {
    u8::from(equal(Cell { tag, word: left }, Cell { tag, word: right }))
}

fn equal(left: Cell, right: Cell) -> bool {
    match left.tag {
        TAG_STR => text(left.word) == text(right.word),
        TAG_LIST => {
            let a = unsafe { &borrow::<List>(left.word).items };
            let b = unsafe { &borrow::<List>(right.word).items };
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| equal(*x, *y))
        }
        TAG_DICT => {
            let a = unsafe { borrow::<Dict>(left.word) };
            let b = unsafe { borrow::<Dict>(right.word) };
            a.entries.len() == b.entries.len()
                && a.entries.iter().all(|(key, value)| {
                    b.entries.get(key).is_some_and(|other| equal(*value, *other))
                })
        }
        TAG_FLOAT => f64::from_bits(left.word) == f64::from_bits(right.word),
        _ => left.word == right.word,
    }
}
