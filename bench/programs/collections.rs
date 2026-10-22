use std::collections::HashMap;
fn main() {
    let mut xs: Vec<i64> = Vec::new();
    let mut i: i64 = 0;
    while i < 300000 { xs.push(i); i += 1; }
    let mut total: i64 = 0;
    for v in &xs { total += v; }
    println!("{total}");
    let mut d: HashMap<i64, i64> = HashMap::new();
    let mut k: i64 = 0;
    while k < 100000 { d.insert(k, k * 2); k += 1; }
    println!("{}", d.len());
}
