fn main() {
    let s: String = std::iter::repeat('x').take(20000).collect();
    let mut total: i64 = 0;
    let mut j = 0;
    while j < 2000 {
        if s.chars().nth(j) == Some('x') { total += 1; }
        j += 1;
    }
    println!("{total}");
}
