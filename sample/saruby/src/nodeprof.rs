// Node execution counts per kind (cargo feature `nodeprof`, interpreter only).
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! { static COUNTS: RefCell<HashMap<&'static str, u64>> = RefCell::new(HashMap::new()); }

pub fn hit(kind: &'static str) { COUNTS.with(|c| *c.borrow_mut().entry(kind).or_insert(0) += 1); }

/// Print the table (most executed first) when SARUBY_NODEPROF is set.
pub fn report() {
    if std::env::var_os("SARUBY_NODEPROF").is_none() { return; }
    COUNTS.with(|c| {
        let mut v: Vec<_> = c.borrow().iter().map(|(k, n)| (*n, *k)).collect();
        v.sort_by(|a, b| b.cmp(a));
        for (n, k) in v { eprintln!("{:>14} {}", n, k); }
    });
}
