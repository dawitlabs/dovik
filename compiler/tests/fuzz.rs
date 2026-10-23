use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use compiler::{check, lexer, modules, parser, resolve};

const SEEDS: &[&str] = &[
    "x = 1\n",
    "function f(a: Int) -> Int:\n    return a + 1\n\nprint(f(2))\n",
    "if x == none:\n    print(1)\nelse:\n    print(2)\n",
    "xs: List<String> = [\"a\", \"b\"]\nfor s in xs:\n    print(s)\n",
    "d: Dict<String, Int> = {\"k\": 1}\nprint(d[\"k\"])\n",
    "while i < 10:\n    i = i + 1\n    if i == 3:\n        break\n    continue\n",
    "import \"lib.dovk\" (name, other as alias)\n\nprint(name)\n",
    "grid: List<List<Int>> = []\nfound: Int? = some(1)\n",
    "s = \"héllo\\u{1F600}\"\nprint(s[0])\nprint(len(s))\n",
    "function a() -> Int:\n    return b()\n\nfunction b() -> Int:\n    return a()\n",
];

const FRAGMENTS: &[&str] = &[
    ":", "\n", "    ", "\t", "?", "->", "==", "none", "some", "else", "function", "return",
    "import", "if", "while", "for", "in", "break", "continue", "not", "and", "or", "(", ")",
    "[", "]", "{", "}", ",", "=", "<", ">", "\"", "\\", "#", "0", "9223372036854775808", "-",
    "List<", "Dict<", "Int", "é", "\u{1F600}",
];

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

fn mutate(rng: &mut Rng, source: &str) -> String {
    let mut bytes: Vec<char> = source.chars().collect();

    for _ in 0..1 + rng.below(4) {
        if bytes.is_empty() {
            break;
        }

        let at = rng.below(bytes.len());

        match rng.below(4) {
            0 => {
                bytes.remove(at);
            }
            1 => {
                let fragment = FRAGMENTS[rng.below(FRAGMENTS.len())];
                for (offset, scalar) in fragment.chars().enumerate() {
                    bytes.insert((at + offset).min(bytes.len()), scalar);
                }
            }
            2 => {
                let taken = bytes[at];
                bytes.insert(at, taken);
            }
            _ => bytes.truncate(at),
        }
    }

    bytes.into_iter().collect()
}

fn frontend(source: &str) {
    let (tokens, _) = lexer::lex(source);
    let (ast, _) = parser::parse(source, &tokens);
    let graph = modules::unit_graph("fuzz.dovk", source, ast);
    let (resolution, _) = resolve::resolve(&graph);
    let _ = check::check(&graph, &resolution);
}

fn run_with_timeout(source: String) -> Result<(), String> {
    let (sender, receiver) = mpsc::channel();
    let subject = source.clone();

    thread::spawn(move || {
        let outcome = std::panic::catch_unwind(|| frontend(&subject));
        let _ = sender.send(outcome.is_ok());
    });

    match receiver.recv_timeout(Duration::from_secs(5)) {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!("the frontend panicked on {source:?}")),
        Err(_) => Err(format!("the frontend did not finish in 5s on {source:?}")),
    }
}

#[test]
fn the_frontend_never_panics_or_hangs() {
    let mut rng = Rng(0x5EED_1234_ABCD_0001);
    let mut cases = Vec::new();

    for seed in SEEDS {
        cases.push(seed.to_string());
    }

    for _ in 0..2000 {
        let seed = SEEDS[rng.below(SEEDS.len())];
        cases.push(mutate(&mut rng, seed));
    }

    for source in cases {
        if let Err(message) = run_with_timeout(source) {
            panic!("{message}");
        }
    }
}

#[test]
fn a_known_hang_stays_fixed() {
    let hang = "x = 1\nif x == :\n    print(\"a\")\nelse:\n    print(\"c\")\n";
    run_with_timeout(hang.to_string()).expect("the parser must not loop on a failed condition");
}
