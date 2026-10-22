fn main() {
    let mut s = String::new();
    let mut i = 0;
    while i < 20000 {
        s = format!("{s}x");
        i += 1;
    }
    println!("{}", s.chars().count());
    let mut count: i64 = 0;
    for _ in s.chars() { count += 1; }
    println!("{count}");
}
