fn main() {
    let mut total: i64 = 0;
    let mut i: i64 = 0;
    while i < 200000 {
        let xs = vec![i, i + 1, i + 2];
        total += xs[0];
        i += 1;
    }
    println!("{total}");
}
