// are: a minimal grep on astrogre.
//   are [-c] [-o] [-n] [--plain|--aot-compile] PATTERN FILE
// Lines are found by searching the whole buffer (the scanner node walks it), not line by line.
// --aot-compile runs, then specializes the pattern's entry nodes and builds code_store/all.so;
// later runs (without --plain) load it and patch the entry dispatchers.
use astrogre::*;
use std::io::Write;
use std::path::PathBuf;

fn bake_config() -> BakeConfig {
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().to_path_buf();
    let sys = PathBuf::from(env!("ASTROGRE_SYSROOT")).join("lib/rustlib").join(env!("ASTROGRE_TARGET")).join("lib");
    BakeConfig {
        crate_name: "astrogre".into(),
        crate_path: dir.join("libastrogre.rlib"),
        lib_dirs: vec![dir.clone()],
        rpaths: vec![sys],
        prelude: "extern crate astrogre;\nuse astrogre::*;\n".into(),
        build_id: env!("ASTROGRE_BUILD_ID").into(),
        verbose: std::env::var_os("ASTROGRE_VERBOSE").is_some(),
    }
}

fn usage() -> ! {
    eprintln!("usage: are [-c] [-o] [-n] [-i] [--plain|--aot-compile] PATTERN FILE");
    std::process::exit(2)
}

fn main() {
    let mut count = false;
    let mut only = false;
    let mut lineno = false;
    let mut aot = false;
    let mut plain = false;
    let mut opts = 0;
    let mut pos: Vec<String> = vec![];
    for a in std::env::args().skip(1) {
        match a.as_str() {
            "-c" => count = true,
            "-o" => only = true,
            "-n" => lineno = true,
            "-i" => opts |= OPT_I,
            "--aot-compile" => aot = true,
            "--plain" => plain = true,
            s if s.starts_with('-') && s.len() > 1 => usage(),
            _ => pos.push(a),
        }
    }
    if pos.len() != 2 { usage(); }
    let re = match Regex::new(pos[0].as_bytes(), opts) {
        Ok(r) => r,
        Err(e) => { eprintln!("are: {}: /{}/", e, pos[0]); std::process::exit(2) }
    };
    let text = std::fs::read(&pos[1]).unwrap_or_else(|e| { eprintln!("are: {}: {}", pos[1], e); std::process::exit(2) });
    let verbose = std::env::var_os("ASTROGRE_VERBOSE").is_some();

    let cfg = bake_config();
    let mut cs = CodeStore::new("code_store");
    let entries = re.entry_nodes();
    if !plain && cs.load(&cfg.build_id) {
        let mut hits = 0;
        for &n in &entries {
            if let Some(p) = cs.lookup(hash_node(n)) {
                unsafe { (*n).head.dispatcher = std::mem::transmute::<*const std::os::raw::c_void, DispatchFn>(p); }
                hits += 1;
            }
        }
        if verbose { eprintln!("are: {} of {} entries specialized", hits, entries.len()); }
    }

    let t = std::time::Instant::now();
    let out = std::io::stdout();
    let mut out = std::io::BufWriter::new(out.lock());
    let (mut nlines, mut nmatch) = (0usize, 0usize);
    let mut p = 0;
    let mut line_no = 1usize;
    let mut counted_to = 0; // line_no is the number of the line starting at counted_to
    while p <= text.len() {
        let m = match re.search(&text, p) { Some(m) => m, None => break };
        let (a, b) = m[0].unwrap();
        let ls = text[..a].iter().rposition(|&x| x == b'\n').map_or(0, |k| k + 1);
        let le = text[b.max(a)..].iter().position(|&x| x == b'\n').map_or(text.len(), |k| b.max(a) + k);
        if lineno {
            line_no += text[counted_to..ls].iter().filter(|&&x| x == b'\n').count();
            counted_to = ls;
        }
        nmatch += 1;
        if only {
            if !count {
                if lineno { let _ = write!(out, "{}:", line_no); }
                let _ = out.write_all(&text[a..b]);
                let _ = out.write_all(b"\n");
            }
            p = if b > a { b } else { b + 1 };
            if b == a && b < text.len() && text[b] >= 0x80 { while p < text.len() && text[p] & 0xC0 == 0x80 { p += 1; } }
        } else {
            nlines += 1;
            if !count {
                if lineno { let _ = write!(out, "{}:", line_no); }
                let _ = out.write_all(&text[ls..le]);
                let _ = out.write_all(b"\n");
            }
            p = le + 1;
        }
    }
    if count { let _ = writeln!(out, "{}", if only { nmatch } else { nlines }); }
    let _ = out.flush();
    if verbose { eprintln!("are: search {:?}", t.elapsed()); }

    if aot {
        let mut sc = SpecCtx::new();
        for &n in &entries { specialize_entry(&mut sc, n); }
        std::fs::create_dir_all("code_store").unwrap();
        cs.add_items(&sc.out).unwrap();
        if let Err(e) = cs.build(&cfg) { eprintln!("are: bake failed:\n{}", e); std::process::exit(1); }
    }
    std::process::exit(if nmatch > 0 { 0 } else { 1 });
}
