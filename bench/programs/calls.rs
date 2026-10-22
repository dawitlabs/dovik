#[inline(never)]
fn step(n: i64) -> i64 { n + 1 }
fn main() {
    let mut total: i64 = 0;
    let mut i: i64 = 0;
    while i < 10000000 { total = step(total); i += 1; }
    println!("{total}");
}
