// Patch binding (ASTro docs/idea_code_store.md §7): a node's own copy of its SD, taken from the store's
// relocatable patch.o, with the holes (symbols ASTRUST_HOLE_<op>_<offset>) relocated to the node's values.
//
// The ELF reader and the relocator are safe Rust over byte slices.  unsafe is confined to the
// external interfaces: memfd/mmap (the arena), dlsym, and the one slice over the arena's writable view.
// x86-64 Linux only (SUPPORTED); elsewhere every instantiate fails and the caller keeps the shared SD.
use std::collections::HashMap;
use std::os::raw::{c_char, c_int, c_void};

pub const SUPPORTED: bool = cfg!(all(target_arch = "x86_64", target_os = "linux"));

/// The value of hole `op` at `offset` from the bound node `root` (None: cannot bind it).
pub type HoleFn = fn(op: &str, root: usize, offset: usize) -> Option<u64>;

// ---- ELF64 relocatable object ----------------------------------------------------------

const SHT_SYMTAB: u32 = 2;
const SHT_RELA: u32 = 4;
const SHT_NOBITS: u32 = 8;
const SHF_WRITE: u64 = 1;
const SHF_ALLOC: u64 = 2;
const SHF_TLS: u64 = 0x400;
const SHN_UNDEF: u16 = 0;
const SHN_ABS: u16 = 0xfff1;
const STB_WEAK: u8 = 2;

struct Sec { name: u32, typ: u32, flags: u64, off: usize, size: usize, align: usize, link: u32, info: u32 }
struct Sym { name: u32, info: u8, shndx: u16, value: u64, size: u64 }
struct Rela { off: usize, sym: u32, typ: u32, add: i64 }

fn rd<const N: usize>(d: &[u8], at: usize) -> Result<[u8; N], String> {
    d.get(at..at + N).and_then(|s| s.try_into().ok()).ok_or_else(|| "truncated object".to_string())
}
fn u16_at(d: &[u8], at: usize) -> Result<u16, String> { Ok(u16::from_le_bytes(rd(d, at)?)) }
fn u32_at(d: &[u8], at: usize) -> Result<u32, String> { Ok(u32::from_le_bytes(rd(d, at)?)) }
fn u64_at(d: &[u8], at: usize) -> Result<u64, String> { Ok(u64::from_le_bytes(rd(d, at)?)) }
fn cstr(d: &[u8], at: usize) -> &str {
    let s = d.get(at..).unwrap_or(&[]);
    std::str::from_utf8(&s[..s.iter().position(|&b| b == 0).unwrap_or(s.len())]).unwrap_or("")
}

/// What a relocation's symbol stands for.
#[derive(Clone, Copy, PartialEq)]
enum Target { Sec(usize, u64), Abs(u64), Ext(u32), Root, Hole(u32) }

pub struct PatchObj {
    data: Vec<u8>,
    secs: Vec<Sec>,
    syms: Vec<Sym>,
    strtab: usize,
    shstrtab: usize,
    relas: Vec<Vec<Rela>>,      // by the section they apply to
    entries: HashMap<String, usize>, // global defined symbol -> index (SD_<hash>, SD_<hash>_HOLES)
    per_node: Vec<bool>,         // the section (or one it reaches) refers to a hole or the root
    shared: Vec<usize>,          // exec address of the one copy of a section not per node (0: not yet)
}

impl PatchObj {
    pub fn open(path: &std::path::Path, build_id: &str) -> Result<PatchObj, String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let mut o = PatchObj::parse(data)?;
        let id = o.entry_bytes("ASTRUST_BUILD_ID")?;
        if id.get(..16) != build_id.as_bytes().get(..16) { return Err("baked against another build".into()); }
        o.mark_per_node();
        Ok(o)
    }

    fn parse(data: Vec<u8>) -> Result<PatchObj, String> {
        let d = &data[..];
        // ELFCLASS64, little endian, ET_REL, EM_X86_64
        if d.get(..6) != Some(&[0x7f, b'E', b'L', b'F', 2, 1][..]) || u16_at(d, 16)? != 1 || u16_at(d, 18)? != 62 {
            return Err("not an x86-64 relocatable ELF object".into());
        }
        let shoff = u64_at(d, 0x28)? as usize;
        let shnum = u16_at(d, 0x3c)? as usize;
        let shstrndx = u16_at(d, 0x3e)? as usize;
        let mut secs = Vec::with_capacity(shnum);
        for i in 0..shnum {
            let h = shoff + i * 64;
            secs.push(Sec { name: u32_at(d, h)?, typ: u32_at(d, h + 4)?, flags: u64_at(d, h + 8)?, off: u64_at(d, h + 24)? as usize,
                            size: u64_at(d, h + 32)? as usize, link: u32_at(d, h + 40)?, info: u32_at(d, h + 44)?,
                            align: (u64_at(d, h + 48)? as usize).max(1) });
        }
        let shstrtab = secs.get(shstrndx).ok_or("no section names")?.off;
        let st = secs.iter().position(|s| s.typ == SHT_SYMTAB).ok_or("no symbol table")?;
        let strtab = secs.get(secs[st].link as usize).ok_or("no string table")?.off;
        let mut syms = Vec::with_capacity(secs[st].size / 24);
        for i in 0..secs[st].size / 24 {
            let h = secs[st].off + i * 24;
            syms.push(Sym { name: u32_at(d, h)?, info: d[h + 4], shndx: u16_at(d, h + 6)?, value: u64_at(d, h + 8)?, size: u64_at(d, h + 16)? });
        }
        let mut relas: Vec<Vec<Rela>> = (0..shnum).map(|_| vec![]).collect();
        for s in secs.iter().filter(|s| s.typ == SHT_RELA) {
            let v = relas.get_mut(s.info as usize).ok_or("relocations for no section")?;
            for i in 0..s.size / 24 {
                let h = s.off + i * 24;
                let info = u64_at(d, h + 8)?;
                v.push(Rela { off: u64_at(d, h)? as usize, sym: (info >> 32) as u32, typ: info as u32, add: u64_at(d, h + 16)? as i64 });
            }
        }
        let mut entries = HashMap::new();
        for (i, s) in syms.iter().enumerate() {
            let name = cstr(d, strtab + s.name as usize);
            if s.info >> 4 != 0 && s.shndx != SHN_UNDEF && (name.starts_with("SD_") || name.starts_with("ASTRUST_")) { entries.insert(name.to_string(), i); }
        }
        let n = secs.len();
        Ok(PatchObj { data, secs, syms, strtab, shstrtab, relas, entries, per_node: vec![false; n], shared: vec![0; n] })
    }

    fn sym_name(&self, i: u32) -> &str { self.syms.get(i as usize).map_or("", |s| cstr(&self.data, self.strtab + s.name as usize)) }
    fn sec_name(&self, i: usize) -> &str { cstr(&self.data, self.shstrtab + self.secs[i].name as usize) }

    fn target(&self, sym: u32) -> Result<Target, String> {
        let s = self.syms.get(sym as usize).ok_or("bad symbol index")?;
        Ok(match s.shndx {
            SHN_UNDEF => {
                let name = self.sym_name(sym);
                if name == "ASTRUST_ROOT" { Target::Root } else if name.starts_with("ASTRUST_HOLE_") { Target::Hole(sym) } else { Target::Ext(sym) }
            }
            SHN_ABS => Target::Abs(s.value),
            i if (i as usize) < self.secs.len() => Target::Sec(i as usize, s.value),
            _ => return Err(format!("unsupported symbol section {:#x}", s.shndx)),
        })
    }

    /// The bytes of a defined global symbol.
    fn entry_bytes(&self, name: &str) -> Result<&[u8], String> {
        let s = &self.syms[*self.entries.get(name).ok_or_else(|| format!("no {}", name))?];
        let sec = self.secs.get(s.shndx as usize).ok_or("bad section")?;
        let at = sec.off + s.value as usize;
        self.data.get(at..at + s.size as usize).ok_or_else(|| "truncated symbol".into())
    }

    /// Sections whose copy is the node's own: they refer to a hole or the root, or reach one that does.
    fn mark_per_node(&mut self) {
        let n = self.secs.len();
        let mut users: Vec<Vec<usize>> = vec![vec![]; n];
        let mut work = vec![];
        for s in 0..n {
            for r in &self.relas[s] {
                match self.target(r.sym) {
                    Ok(Target::Root | Target::Hole(_)) => if !self.per_node[s] { self.per_node[s] = true; work.push(s); },
                    Ok(Target::Sec(t, _)) if t != s => users[t].push(s),
                    _ => {}
                }
            }
        }
        while let Some(t) = work.pop() {
            for &u in &users[t] { if !self.per_node[u] { self.per_node[u] = true; work.push(u); } }
        }
    }

    /// Sections reached from `start` through relocations: per node ones, and the shared ones they use.
    fn closure(&self, start: usize, per_node: bool) -> Result<Vec<usize>, String> {
        let mut seen = vec![false; self.secs.len()];
        let mut out = vec![];
        let mut work = vec![start];
        seen[start] = true;
        while let Some(s) = work.pop() {
            let sec = &self.secs[s];
            if sec.flags & SHF_ALLOC == 0 || sec.flags & SHF_TLS != 0 || sec.typ == SHT_NOBITS
                || (sec.flags & SHF_WRITE != 0 && !self.sec_name(s).starts_with(".data.rel.ro")) {
                return Err(format!("unsupported section {}", self.sec_name(s)));
            }
            out.push(s);
            for r in &self.relas[s] {
                if let Target::Sec(t, _) = self.target(r.sym)? {
                    // a per node copy uses the shared copy of a section that needs no binding
                    if !seen[t] && self.per_node[t] == per_node && (per_node || self.shared[t] == 0) { seen[t] = true; work.push(t); }
                }
            }
        }
        Ok(out)
    }
}

// ---- the arena: one memfd, each window of it mapped twice (written through RW, run through RX) ----

extern "C" {
    fn memfd_create(name: *const u8, flags: u32) -> i32;
    fn ftruncate(fd: c_int, len: i64) -> c_int;
    fn mmap(a: *mut u8, l: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut u8;
    fn munmap(a: *mut u8, l: usize) -> i32;
    fn dlsym(h: *mut c_void, sym: *const c_char) -> *mut c_void;
}
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const PROT_EXEC: c_int = 4;
const MAP_SHARED: c_int = 1;
const MAP_FIXED_NOREPLACE: c_int = 0x100000;
const MAP_FAILED: usize = !0;


/// Candidate addresses for `len` bytes in the gaps of /proc/self/maps within rel32 reach of `near`, nearest first.
fn free_near(near: usize, len: usize) -> Vec<usize> {
    let maps = std::fs::read_to_string("/proc/self/maps").unwrap_or_default();
    let mut used: Vec<(usize, usize)> = maps.lines().filter_map(|l| {
        let (a, b) = l.split(' ').next()?.split_once('-')?;
        Some((usize::from_str_radix(a, 16).ok()?, usize::from_str_radix(b, 16).ok()?))
    }).collect();
    used.sort();
    // pad: a guard from the neighbours; slack: the embedder's own extent around `near`
    let (reach, pad, slack) = (1usize << 31, 1usize << 20, 64usize << 20);
    let mut out = vec![];
    let mut prev = 1usize << 32;
    for &(a, b) in used.iter().chain(std::iter::once(&(1usize << 47, 1usize << 47))) {
        if a > prev + len + 2 * pad {
            // the end of the gap nearer `near`
            let at = if prev + pad > near { prev + pad } else { (a - pad - len) & !0xfff };
            let far = if at > near { at + len - near } else { near - at };
            if far + slack < reach { out.push((far, at)); }
        }
        prev = prev.max(b);
    }
    out.sort();
    out.into_iter().map(|x| x.1).collect()
}

/// A range of the memfd mapped twice: written through `rw`, run at `rx`.
struct Window { rw: &'static mut [u8], rx: usize, used: usize }

struct Arena { fd: i32, size: usize, near: usize, wins: Vec<Window>, pub bytes: usize }

const WINDOW: usize = 16 << 20;   // address space only: pages are taken as copies are written

impl Arena {
    /// `near`: an address the copies should reach with rel32 (the embedder's library).
    fn new(near: usize) -> Option<Arena> {
        // SAFETY: a plain syscall; the name is a NUL-terminated literal.
        let fd = unsafe { memfd_create(b"astrust-code\0".as_ptr(), 1 /* MFD_CLOEXEC */) };
        if fd < 0 { return None; }
        Some(Arena { fd, size: 0, near, wins: vec![], bytes: 0 })
    }
    /// A new window of at least `len` bytes: in reach of `near` if a free range allows, else anywhere
    /// (then the loader keeps GOT slots and stubs instead of relaxing to rel32).
    fn grow(&mut self, len: usize) -> Result<(), String> {
        let len = len.max(WINDOW).next_multiple_of(1 << 12);
        let off = self.size;
        // SAFETY: fd is ours; the file only grows (pages are allocated when written).
        if unsafe { ftruncate(self.fd, (off + len) as i64) } != 0 { return Err("memfd".into()); }
        self.size += len;
        // SAFETY: a fresh shared mapping of our own file, never unmapped.
        let rw = unsafe { mmap(std::ptr::null_mut(), len, PROT_READ | PROT_WRITE, MAP_SHARED, self.fd, off as i64) } as usize;
        if rw == MAP_FAILED { return Err("mmap".into()); }
        let mut rx = MAP_FAILED;
        for at in free_near(self.near, len) {
            // SAFETY: MAP_FIXED_NOREPLACE never replaces an existing mapping; a miss is returned or fails.
            let p = unsafe { mmap(at as *mut u8, len, PROT_READ | PROT_EXEC, MAP_SHARED | MAP_FIXED_NOREPLACE, self.fd, off as i64) } as usize;
            if p == at { rx = p; break; }
            // SAFETY: p is a mapping this call just made at another place (a kernel without NOREPLACE).
            if p != MAP_FAILED { unsafe { munmap(p as *mut u8, len); } }
        }
        if rx == MAP_FAILED {
            // SAFETY: as above, at a place the kernel picks.
            rx = unsafe { mmap(std::ptr::null_mut(), len, PROT_READ | PROT_EXEC, MAP_SHARED, self.fd, off as i64) } as usize;
            if rx == MAP_FAILED { return Err("mmap".into()); }
        }
        // SAFETY: rw..rw+len is mapped read-write for the rest of the process and only this Arena uses it.
        let rw = unsafe { std::slice::from_raw_parts_mut(rw as *mut u8, len) };
        self.wins.push(Window { rw, rx, used: 0 });
        Ok(())
    }
    /// Make room for `len` bytes in one window: one copy's sections, slots and stubs lie together.
    fn reserve(&mut self, len: usize) -> Result<(), String> {
        match self.wins.last() { Some(w) if w.used + len <= w.rw.len() => Ok(()), _ => self.grow(len) }
    }
    /// `len` bytes aligned to `align` (within what `reserve` made room for): the exec address.
    fn alloc(&mut self, len: usize, align: usize) -> Result<usize, String> {
        let w = self.wins.last_mut().ok_or("no window")?;
        let at = w.used.next_multiple_of(align);
        if at + len > w.rw.len() { return Err("window full".into()); }
        self.bytes += at + len - w.used;
        w.used = at + len;
        Ok(w.rx + at)
    }
    fn write(&mut self, exec: usize, b: &[u8]) {
        let w = self.wins.iter_mut().rev().find(|w| exec >= w.rx && exec < w.rx + w.rw.len()).expect("address outside the arena");
        let at = exec - w.rx;
        w.rw[at..at + b.len()].copy_from_slice(b);
    }
}


// ---- the loader ----------------------------------------------------------------------------

pub struct Loader {
    arena: Option<Option<Arena>>,   // made at the first instantiate (None inside: no memfd)
    near: usize,
    objs: HashMap<std::path::PathBuf, Result<PatchObj, String>>,
    ext: HashMap<String, usize>,
    pub instances: usize,
    pub failed: usize,
    pub bytes: usize,
    perf_map: Option<std::fs::File>,
}


impl Loader {
    pub fn new(near: usize) -> Loader {
        // ASTRUST_PERF_MAP=1: name each copy in /tmp/perf-<pid>.map for perf
        let perf_map = std::env::var_os("ASTRUST_PERF_MAP").and_then(|_| std::fs::File::create(format!("/tmp/perf-{}.map", std::process::id())).ok());
        Loader { arena: None, near, objs: HashMap::new(), ext: HashMap::new(), instances: 0, failed: 0, bytes: 0, perf_map }
    }

    /// A copy of `entry` (SD_<hash>) from the object at `path`, bound to node `root`: its exec address.
    pub fn instantiate(&mut self, path: &std::path::Path, build_id: &str, entry: &str, root: usize, hole: HoleFn) -> Result<usize, String> {
        let near = self.near;
        let arena = self.arena.get_or_insert_with(|| if SUPPORTED { Arena::new(near) } else { None });
        let obj = self.objs.entry(path.to_path_buf()).or_insert_with(|| PatchObj::open(path, build_id));
        let r = match (arena.as_mut(), obj.as_mut()) {
            (None, _) => Err("no arena".to_string()),
            (_, Err(e)) => Err(e.clone()),
            (Some(a), Ok(o)) => instantiate(o, a, &mut self.ext, entry, root, hole),
        };
        match r {
            Ok((addr, len)) => {
                self.instances += 1; self.bytes += len;
                if let Some(f) = self.perf_map.as_mut() { use std::io::Write; let _ = writeln!(f, "{:x} {:x} {}@{:x}", addr, len, entry, root); }
            }
            Err(_) => self.failed += 1,
        }
        r.map(|x| x.0)
    }
}

/// The node's copy of `entry`, after the shared sections it uses: (exec address, bytes of the copy).
fn instantiate(o: &mut PatchObj, a: &mut Arena, ext: &mut HashMap<String, usize>, entry: &str, root: usize, hole: HoleFn) -> Result<(usize, usize), String> {
    let e = *o.entries.get(entry).ok_or_else(|| format!("{} not in patch.o", entry))?;
    let start = o.syms[e].shndx as usize;
    // the holes this entry may bind: (offset, size) pairs of its binding descriptor
    let desc: Vec<u32> = o.entry_bytes(&format!("{}_HOLES", entry)).map(|b| b.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect()).unwrap_or_default();
    let mine = o.closure(start, o.per_node[start])?;
    // the shared copies first: a node's sections then lie together
    let mut need = vec![];
    for &s in &mine {
        for r in &o.relas[s] { if let Target::Sec(t, _) = o.target(r.sym)? { if !o.per_node[t] && o.shared[t] == 0 { need.push(t); } } }
    }
    if !o.per_node[start] { need.push(start); }
    for t in need { if o.shared[t] == 0 { place(o, a, ext, t, false, root, &desc, hole)?; } }
    if !o.per_node[start] { return Ok((o.shared[start] + o.syms[e].value as usize, 0)); }
    let (base, len) = place(o, a, ext, start, true, root, &desc, hole)?;
    Ok((base + o.syms[e].value as usize, len))
}

/// Copy the closure of `start` (per node or shared) into the arena and relocate it: (address of start, bytes).
/// A call out of rel32 reach goes through a stub, a GOT relocation through a slot, both after the copy.
#[allow(clippy::too_many_arguments)]
fn place(o: &mut PatchObj, a: &mut Arena, ext: &mut HashMap<String, usize>, start: usize, per_node: bool, root: usize, desc: &[u32], hole: HoleFn) -> Result<(usize, usize), String> {
    let secs = o.closure(start, per_node)?;
    // sections with their alignment, and a 16-byte slot or stub per relocation that may need one
    a.reserve(secs.iter().map(|&s| o.secs[s].size + o.secs[s].align.max(16) + 16 * o.relas[s].iter().filter(|r| matches!(r.typ, 4 | 9 | 41 | 42)).count()).sum())?;
    let first = a.bytes;
    let mut addr: HashMap<usize, usize> = HashMap::new();
    for &s in &secs {
        let at = a.alloc(o.secs[s].size, o.secs[s].align.max(16))?;
        a.write(at, o.data.get(o.secs[s].off..o.secs[s].off + o.secs[s].size).ok_or("truncated section")?);
        addr.insert(s, at);
    }
    let mut slots: HashMap<u32, usize> = HashMap::new();   // GOT slots and stubs, by symbol
    for &s in &secs {
        for i in 0..o.relas[s].len() {
            let (off, sym, typ, add) = { let r = &o.relas[s][i]; (r.off, r.sym, r.typ, r.add) };
            let p = addr[&s] + off;
            let sv: u64 = match o.target(sym)? {
                Target::Sec(t, v) => {
                    let at = addr.get(&t).copied().unwrap_or(o.shared[t]);
                    if at == 0 { return Err(format!("section {} not placed", o.sec_name(t))); }
                    (at as u64).wrapping_add(v)
                }
                Target::Abs(v) => v,
                Target::Root => root as u64,
                Target::Hole(h) => {
                    let name = o.sym_name(h);
                    let rest = &name["ASTRUST_HOLE_".len()..];
                    let (op, at) = rest.rsplit_once('_').ok_or("bad hole name")?;
                    let at: usize = at.parse().map_err(|_| "bad hole offset")?;
                    if !desc.chunks_exact(2).any(|d| d[0] as usize == at) { return Err(format!("hole {} not in the descriptor", name)); }
                    hole(op, root, at).ok_or_else(|| format!("hole {} unbound", name))?
                }
                Target::Ext(x) => {
                    let name = o.sym_name(x).to_string();
                    let weak = o.syms[x as usize].info >> 4 == STB_WEAK;
                    resolve(ext, &name, weak)? as u64
                }
            };
            let pc = |v: i128| -> Result<[u8; 4], String> { i32::try_from(v).map(|x| x.to_le_bytes()).map_err(|_| "pc32 out of range".to_string()) };
            let bytes: Vec<u8> = match typ {
                1 => sv.wrapping_add(add as u64).to_le_bytes().to_vec(),                      // R_X86_64_64
                24 => (sv.wrapping_add(add as u64).wrapping_sub(p as u64)).to_le_bytes().to_vec(), // PC64
                2 | 4 => {                                                                    // PC32, PLT32
                    let v = sv as i128 + add as i128 - p as i128;
                    match (pc(v), typ) {
                        (Ok(b), _) => b.to_vec(),
                        (Err(_), 4) => {
                            // a call out of reach: jmp *0(%rip) and the address
                            let stub = match slots.get(&(sym | 1 << 31)) { Some(&x) => x, None => {
                                let x = a.alloc(16, 16)?;
                                let mut b = vec![0xff, 0x25, 0, 0, 0, 0];
                                b.extend_from_slice(&sv.to_le_bytes());
                                a.write(x, &b);
                                slots.insert(sym | 1 << 31, x); x } };
                            pc(stub as i128 + add as i128 - p as i128)?.to_vec()
                        }
                        (Err(e), _) => return Err(format!("{} ({})", e, o.sym_name(sym))),
                    }
                }
                9 | 41 | 42 => {                                                               // GOTPCREL(X), REX_GOTPCRELX
                    // relax as ld does: the instruction takes the value itself when it can
                    let rel = sv as i128 - (p as i128 + 4);
                    let code = &o.data[o.secs[s].off..o.secs[s].off + o.secs[s].size];
                    let (op, modrm, rex) = (code.get(off.wrapping_sub(2)).copied(), code.get(off.wrapping_sub(1)).copied(), code.get(off.wrapping_sub(3)).copied());
                    let relaxed: Option<Vec<(usize, Vec<u8>)>> = if add != -4 { None } else {
                        match (typ, op, modrm) {
                            // call *x(%rip) -> addr32 call x
                            (9 | 41, Some(0xff), Some(0x15)) => pc(rel).ok().map(|b| vec![(p - 2, [&[0x67, 0xe8][..], &b].concat())]),
                            // jmp *x(%rip) -> jmp x; nop
                            (9 | 41, Some(0xff), Some(0x25)) => pc(rel + 1).ok().map(|b| vec![(p - 2, [&[0xe9][..], &b, &[0x90]].concat())]),
                            (9 | 42, Some(0x8b), Some(m)) if m & 0xc7 == 0x05 && rex.is_some_and(|r| r & 0xf8 == 0x48) => {
                                let r = rex.unwrap();
                                if let Ok(v) = i32::try_from(sv as i64) {
                                    // mov x@GOTPCREL(%rip), %reg -> mov $x, %reg (REX.R becomes REX.B)
                                    let rex2 = (r & !4) | ((r & 4) >> 2);
                                    Some(vec![(p - 3, [&[rex2, 0xc7, 0xc0 | ((m >> 3) & 7)][..], &v.to_le_bytes()].concat())])
                                } else {
                                    // -> lea x(%rip), %reg
                                    pc(rel).ok().map(|b| vec![(p - 2, vec![0x8d]), (p, b.to_vec())])
                                }
                            }
                            _ => None,
                        }
                    };
                    if let Some(w) = relaxed { for (at, b) in w { a.write(at, &b); } continue; }
                    let slot = match slots.get(&sym) { Some(&x) => x, None => {
                        let x = a.alloc(8, 8)?; a.write(x, &sv.to_le_bytes()); slots.insert(sym, x); x } };
                    pc(slot as i128 + add as i128 - p as i128)?.to_vec()
                }
                10 => u32::try_from(sv as i128 + add as i128).map_err(|_| format!("32 out of range ({})", o.sym_name(sym)))?.to_le_bytes().to_vec(),
                11 => i32::try_from(sv as i64 as i128 + add as i128).map_err(|_| format!("32S out of range ({})", o.sym_name(sym)))?.to_le_bytes().to_vec(),
                t => return Err(format!("unsupported relocation {} ({})", t, o.sym_name(sym))),
            };
            a.write(p, &bytes);
        }
    }
    if !per_node { for (&s, &at) in &addr { o.shared[s] = at; } }
    Ok((addr[&start], a.bytes - first))
}

fn resolve(ext: &mut HashMap<String, usize>, name: &str, weak: bool) -> Result<usize, String> {
    if let Some(&v) = ext.get(name) { return Ok(v); }
    let c = std::ffi::CString::new(name).map_err(|_| "bad symbol name")?;
    // SAFETY: dlsym with RTLD_DEFAULT (null) and a NUL-terminated name only looks the symbol up.
    let v = unsafe { dlsym(std::ptr::null_mut(), c.as_ptr()) } as usize;
    if v == 0 && !weak { return Err(format!("unresolved {}", name)); }
    ext.insert(name.to_string(), v);
    Ok(v)
}
