use astrogre::*;
use std::io::Read;

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
}

fn fmt(r: &Option<Vec<Option<(usize, usize)>>>) -> String {
    match r {
        None => "N".into(),
        Some(v) => v.iter().map(|g| match g { Some((a, b)) => format!("{},{}", a, b), None => "-".into() }).collect::<Vec<_>>().join(";"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // The stress run uses a 2 MB thread stack: CPS recursion must stay bounded by the pattern.
    if args.len() > 1 && args[1] == "stress" { std::thread::Builder::new().stack_size(2 << 20).spawn(stress).unwrap().join().unwrap(); return; }
    if args.len() > 1 && args[1] == "bench" { bench(); return; }
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let (mut pass, mut fail) = (0, 0);
    for line in input.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 6 { continue; }
        let (pat, opts, subj, start, mode, exp) = (unhex(f[0]), f[1].parse::<u32>().unwrap(), unhex(f[2]), f[3].parse::<usize>().unwrap(), f[4], f[5]);
        let names_exp = f.get(6).copied().unwrap_or("");
        let got = match Regex::new(&pat, opts) {
            Err(e) => format!("E:{}", e),
            Ok(re) => {
                let r = if mode == "s" { re.search(&subj, start) } else { re.match_at(&subj, start) };
                if mode == "s" {
                    let rv = re.search_vm(&subj, start);
                    if fmt(&rv) != fmt(&r) { println!("CPS/VM DIFFER /{}/ {:?}: cps={} vm={}", String::from_utf8_lossy(&pat), String::from_utf8_lossy(&subj), fmt(&r), fmt(&rv)); fail += 1; }
                }
                let mut g = fmt(&r);
                assert!(re.group_count() + 1 == r.as_ref().map_or(re.group_count() + 1, |v| v.len()));
                for (n, v) in re.names() { assert_eq!(re.group_index(&n), v); }
                if !names_exp.is_empty() || !re.names().is_empty() {
                    let ns: Vec<String> = re.names().iter().map(|(n, v)| format!("{}={}", n, v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))).collect();
                    g = format!("{}\t{}", g, ns.join(" "));
                }
                g
            }
        };
        let expect = if names_exp.is_empty() { exp.to_string() } else { format!("{}\t{}", exp, names_exp) };
        if got == expect { pass += 1; } else {
            fail += 1;
            println!("FAIL /{}/{} subj={:?} start={} mode={}\n   expected: {}\n   got:      {}", String::from_utf8_lossy(&pat), opts, String::from_utf8_lossy(&subj), start, mode, expect, got);
        }
    }
    println!("pass={} fail={}", pass, fail);
    if fail > 0 { std::process::exit(1); }
}

fn scan_count(re: &Regex, s: &[u8]) -> usize {
    let mut n = 0;
    let mut p = 0;
    while let Some(m) = re.search(s, p) {
        let (a, b) = m[0].unwrap();
        n += 1;
        p = if b > a { b } else { b + 1 };
        if p > s.len() { break; }
    }
    n
}

fn bench() {
    use std::time::Instant;
    let words = ["alpha", "beta", "gamma", "foo", "bar", "baz", "12-34", "hello@example.com", "x", "QUUX", "a1b2"];
    let mut text = Vec::new();
    let mut i = 0usize;
    while text.len() < 1_000_000 {
        text.extend_from_slice(words[(i * 7 + i / 3) % words.len()].as_bytes());
        text.push(if i % 13 == 0 { b'\n' } else { b' ' });
        i += 1;
    }
    for p in ["\\w+", "[aeiou]{2}", "(\\d+)-(\\d+)", "foo|bar|baz", "(?i)quux", "\\b\\w+@\\w+\\.com\\b", "^\\w+", "\\s+$", "z{2}", "[[:alpha:]]+\\d"] {
        let re = Regex::new(p.as_bytes(), 0).unwrap();
        let t = Instant::now();
        let n = scan_count(&re, &text);
        println!("{:<28} {:>8} {:?}", p, n, t.elapsed());
    }
}

fn stress() {
    use std::time::Instant;
    let n = 1_000_000;
    let a = vec![b'a'; n];
    let ab: Vec<u8> = b"ab".iter().cycle().take(n).copied().collect();
    let cases: Vec<(&str, &[u8], Option<(usize, usize)>)> = vec![
        ("a*", &a, Some((0, n))),
        ("(?:ab)*", &ab, Some((0, n))),
        (".*x", &a, None),
        ("(a)*", &a, Some((0, n))),
        ("(a|b)*c", &ab, None),
        ("a*?$", &a, Some((0, n))),
        ("(?:a|b)+$", &ab, Some((0, n))),
        ("\\d+", &a, None),
        ("[a-z]+", &a, Some((0, n))),
        ("(?>a+)b", &a, None),
        ("a{2,}", &a, Some((0, n))),
        ("(?:a{1,3})+", &a, Some((0, n))),
        ("(?<x>a)+\\k<x>", &a, Some((0, n))),
        ("x", &a, None),
        ("(?=a)+a*$", &a, Some((0, n))),
        (".*?$", &a, Some((0, n))),
        ("(?:a?)*b", &a[..2000], None),
    ];
    let only = std::env::args().nth(2);
    for (p, s, exp) in cases {
        if let Some(o) = &only { if o != p { continue; } }
        let re = Regex::new(p.as_bytes(), 0).unwrap();
        let t = Instant::now();
        let r = re.search(s, 0).map(|v| v[0].unwrap());
        let dt = t.elapsed();
        println!("{:<20} {:?} {:?} {}", p, r, dt, if r == exp { "ok" } else { "MISMATCH" });
    }
}
