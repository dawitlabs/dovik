fn step(total: i64, value: i64) -> i64 {
    total + value * 3
}

fn main() {
    let mut xs: Vec<i64> = Vec::new();
    let mut i: i64 = 0;
    while i < 1024 {
        xs.push(i % 7);
        i += 1;
    }

    let mut total: i64 = 0;
    let mut round = 0;
    while round < 10000 {
        for v in &xs {
            total = step(total, *v);
        }
        round += 1;
    }
    println!("{total}");
}
