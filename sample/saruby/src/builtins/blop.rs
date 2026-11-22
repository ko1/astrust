// Blocking operations (blop): what a parked green thread waits for (docs/io_design.md).
// The slab owns every in-flight op (and its buffer); a parked thread keeps only the Id and
// takes the op back with its result when it resumes. Engines (poll / epoll) are a trait.
#![forbid(unsafe_code)]
use crate::builtins::sys::{self, EpollEvent, PollFd, Timespec};
use std::cell::RefCell;
use std::sync::atomic::{AtomicI32, Ordering::Relaxed};
use std::time::{Duration, Instant};

pub const EBADF: i64 = 9;
pub const ETIMEDOUT: i64 = 110;
pub const ECANCELED: i64 = 125;
const POLLNVAL: i16 = 32;
/// POLLOUT promises this much room in a pipe; a readiness write never asks for more.
const WRITE_CHUNK: usize = 4096;

/// An in-flight operation. Buffers are owned: the op is moved in and handed back on completion.
pub enum Op {
    /// readiness of the fds; result = number ready, per-fd revents in the Vec (poll(2) form)
    Poll(Vec<PollFd>),
    /// read(2) once into `buf` (its length); result = bytes, valid in buf[..result]
    Read { fd: i32, buf: Vec<u8> },
    /// write(2) once from `buf` (at most WRITE_CHUNK); result = bytes written
    Write { fd: i32, buf: Vec<u8> },
    /// time only: completes Ok(0) at the deadline (none: never, until canceled)
    Timer,
    /// another green thread's notify(key) (Mutex / Queue / ConditionVariable / join)
    Wake(u64),
}
impl Op {
    /// fds this op waits on, with poll(2) events: (index in the op, fd, events)
    fn interest(&self) -> impl Iterator<Item = (u32, i32, i16)> + '_ {
        let (s, one): (&[PollFd], Option<(i32, i16)>) = match self {
            Op::Poll(f) => (f, None),
            Op::Read { fd, .. } => (&[], Some((*fd, sys::POLLIN))),
            Op::Write { fd, .. } => (&[], Some((*fd, sys::POLLOUT))),
            Op::Timer | Op::Wake(_) => (&[], None),
        };
        s.iter().enumerate().map(|(i, p)| (i as u32, p.fd, p.events)).chain(one.map(|(fd, ev)| (0, fd, ev)))
    }
    fn has_fd(&self, fd: i32) -> bool { self.interest().any(|(_, f, _)| f == fd) }
}
/// Ok(count) or Err(errno); ETIMEDOUT = deadline passed, ECANCELED = interrupted, EBADF = fd closed by another thread.
pub type Res = Result<usize, i64>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Id { idx: u32, gen: u32 }

struct Pending { op: Op, deadline: Option<Instant>, deadlockable: bool }
enum Slot { Free, Pending(Pending), Done(Op, Res) }
struct Entry { gen: u32, slot: Slot }

/// In-flight operations keyed by Id (generation-checked: a stale Id never names a reused slot).
pub struct Slab { ents: Vec<Entry>, free: Vec<u32>, npending: usize, just_done: Vec<Id> }
/// One fd of a pending op became ready (engine → slab).
pub struct Ready { pub id: Id, pub idx: u32, pub revents: i16 }

impl Slab {
    fn new() -> Slab { Slab { ents: vec![], free: vec![], npending: 0, just_done: vec![] } }
    fn get(&self, id: Id) -> Option<&Slot> { self.ents.get(id.idx as usize).filter(|e| e.gen == id.gen).map(|e| &e.slot) }
    fn get_mut(&mut self, id: Id) -> Option<&mut Slot> { self.ents.get_mut(id.idx as usize).filter(|e| e.gen == id.gen).map(|e| &mut e.slot) }
    fn insert(&mut self, p: Pending) -> Id {
        self.npending += 1;
        if let Some(i) = self.free.pop() { let e = &mut self.ents[i as usize]; e.slot = Slot::Pending(p); return Id { idx: i, gen: e.gen }; }
        self.ents.push(Entry { gen: 0, slot: Slot::Pending(p) });
        Id { idx: self.ents.len() as u32 - 1, gen: 0 }
    }
    /// Pending → Done (no-op for any other state: completion is posted once).
    pub fn complete(&mut self, id: Id, r: Res) {
        let Some(s) = self.get_mut(id) else { return };
        if !matches!(s, Slot::Pending(_)) { return; }
        let Slot::Pending(p) = std::mem::replace(s, Slot::Free) else { unreachable!() };
        *s = Slot::Done(p.op, r);
        self.npending -= 1;
        self.just_done.push(id);
    }
    fn release(&mut self, id: Id) -> Option<Slot> {
        let e = self.ents.get_mut(id.idx as usize).filter(|e| e.gen == id.gen)?;
        let s = std::mem::replace(&mut e.slot, Slot::Free);
        if matches!(s, Slot::Pending(_)) { self.npending -= 1; }
        e.gen = e.gen.wrapping_add(1);
        self.free.push(id.idx);
        Some(s)
    }
    fn pending(&self) -> impl Iterator<Item = (Id, &Pending)> {
        self.ents.iter().enumerate().filter_map(|(i, e)| match &e.slot { Slot::Pending(p) => Some((Id { idx: i as u32, gen: e.gen }, p)), _ => None })
    }
    /// Pending ops that wait on fds, for engines that collect interest per pump.
    pub fn fd_ops(&self) -> impl Iterator<Item = (Id, &Op)> { self.pending().filter(|(_, p)| p.op.interest().next().is_some()).map(|(id, p)| (id, &p.op)) }
    fn next_deadline(&self) -> Option<Instant> { self.pending().filter_map(|(_, p)| p.deadline).min() }
    fn expire(&mut self, now: Instant) -> usize {
        let due: Vec<(Id, bool)> = self.pending().filter(|(_, p)| p.deadline.is_some_and(|d| d <= now)).map(|(id, p)| (id, matches!(p.op, Op::Timer))).collect();
        for &(id, timer) in &due { self.complete(id, if timer { Ok(0) } else { Err(ETIMEDOUT) }); }
        due.len()
    }
    /// Readiness reports → completions, emulating completion for Read / Write (the I/O runs here).
    fn apply_ready(&mut self, ready: &[Ready]) -> usize {
        let mut polled: Vec<Id> = vec![];
        let mut served: Vec<i32> = vec![];   // one read / write per fd per pump: the next one re-polls
        let mut n = 0;
        for r in ready {
            let Some(Slot::Pending(p)) = self.get_mut(r.id) else { continue };
            let res = match &mut p.op {
                Op::Poll(fds) => {
                    if !polled.contains(&r.id) { polled.push(r.id); for f in fds.iter_mut() { f.revents = 0; } }
                    if let Some(f) = fds.get_mut(r.idx as usize) { f.revents = r.revents; }
                    continue;
                }
                Op::Read { fd, buf } => {
                    if served.contains(fd) { continue; }
                    served.push(*fd);
                    let k = sys::read(*fd, buf);
                    if k < 0 { let e = sys::errno(); if e == sys::EAGAIN || e == sys::EINTR { continue; } Err(e) } else { Ok(k as usize) }
                }
                Op::Write { fd, buf } => {
                    if served.contains(fd) { continue; }
                    served.push(*fd);
                    let k = sys::write(*fd, &buf[..buf.len().min(WRITE_CHUNK)]);
                    if k < 0 { let e = sys::errno(); if e == sys::EAGAIN || e == sys::EINTR { continue; } Err(e) } else { Ok(k as usize) }
                }
                Op::Timer | Op::Wake(_) => continue,
            };
            self.complete(r.id, res); n += 1;
        }
        for id in polled {
            let k = match self.get(id) { Some(Slot::Pending(Pending { op: Op::Poll(f), .. })) => f.iter().filter(|p| p.revents != 0).count(), _ => continue };
            self.complete(id, Ok(k)); n += 1;
        }
        n
    }
}

/// A readiness or completion backend. Readiness engines report `Ready`s; a completion engine
/// (io_uring) would call `Slab::complete` itself.
pub trait Engine {
    fn name(&self) -> &'static str;
    /// `op` entered the slab as `id`.
    fn submit(&mut self, id: Id, op: &Op);
    /// `op` (`id`) leaves before completing, or after (its interest must go either way).
    fn retire(&mut self, id: Id, op: &Op);
    /// Sleep until a registered fd or the wake fd is ready or `timeout` passes (None: no limit).
    fn pump(&mut self, slab: &Slab, timeout: Option<Duration>, ready: &mut Vec<Ready>);
    /// In a forked child: kernel objects shared with the parent are replaced.
    fn after_fork(&mut self, wake_fd: i32);
}
fn timespec(d: Duration) -> Timespec { Timespec { sec: d.as_secs().min(i64::MAX as u64) as i64, nsec: d.subsec_nanos() as i64 } }
fn drain_wake(fd: i32) { let mut b = [0u8; 8]; sys::read(fd, &mut b); }

/// poll(2): interest is rebuilt from the slab at every pump (no registration).
struct PollEngine { wake: i32, fds: Vec<PollFd>, who: Vec<(Id, u32)> }
impl Engine for PollEngine {
    fn name(&self) -> &'static str { "poll" }
    fn submit(&mut self, _id: Id, _op: &Op) {}
    fn retire(&mut self, _id: Id, _op: &Op) {}
    fn pump(&mut self, slab: &Slab, timeout: Option<Duration>, ready: &mut Vec<Ready>) {
        self.fds.clear(); self.who.clear();
        self.fds.push(PollFd { fd: self.wake, events: sys::POLLIN, revents: 0 });
        for (id, op) in slab.fd_ops() {
            for (i, fd, ev) in op.interest() { self.fds.push(PollFd { fd, events: ev, revents: 0 }); self.who.push((id, i)); }
        }
        let ts = timeout.map(timespec);
        if sys::ppoll(&mut self.fds, ts.as_ref()) <= 0 { return; }
        if self.fds[0].revents != 0 { drain_wake(self.wake); }
        for (p, &(id, idx)) in self.fds[1..].iter().zip(&self.who) { if p.revents != 0 { ready.push(Ready { id, idx, revents: p.revents }); } }
    }
    fn after_fork(&mut self, wake_fd: i32) { self.wake = wake_fd; }
}

/// epoll(7), level-triggered: one registration per fd carrying the union of its ops' events.
struct EpollEngine { ep: i32, wake: i32, regs: Vec<(i32, Vec<(Id, u32, i16)>, u32)>, evs: Vec<EpollEvent>, always: Vec<(Id, u32, i16)> }
const WAKE_TOKEN: u64 = u64::MAX;
fn ep_bits(ev: i16) -> u32 { (ev as u16 as u32) & (sys::POLLIN | sys::POLLOUT | 2) as u32 }   // EPOLLIN/OUT/PRI share poll's bits
impl EpollEngine {
    fn open(wake: i32) -> Option<EpollEngine> {
        let ep = sys::epoll_create1(sys::EPOLL_CLOEXEC);
        if ep < 0 { return None; }
        let e = EpollEngine { ep, wake, regs: vec![], evs: vec![EpollEvent { events: 0, data: 0 }; 64], always: vec![] };
        if sys::epoll_ctl(ep, sys::EPOLL_CTL_ADD, wake, EpollEvent { events: sys::POLLIN as u32, data: WAKE_TOKEN }) < 0 { sys::close(ep); return None; }
        // probe epoll_pwait2 (Linux 5.11): ENOSYS makes the caller fall back to poll
        let z = Timespec { sec: 0, nsec: 0 };
        if sys::epoll_pwait2(ep, &mut [EpollEvent { events: 0, data: 0 }], Some(&z)) < 0 && sys::errno() == 38 { sys::close(ep); return None; }
        Some(e)
    }
    fn sync(&mut self, fd: i32) {
        let Some(k) = self.regs.iter().position(|r| r.0 == fd) else { return };
        let want = self.regs[k].1.iter().fold(0u32, |m, w| m | ep_bits(w.2));
        if self.regs[k].1.is_empty() {
            sys::epoll_ctl(self.ep, sys::EPOLL_CTL_DEL, fd, EpollEvent { events: 0, data: 0 });
            self.regs.swap_remove(k);
        } else if want != self.regs[k].2 {
            self.regs[k].2 = want;
            sys::epoll_ctl(self.ep, sys::EPOLL_CTL_MOD, fd, EpollEvent { events: want, data: fd as u64 });
        }
    }
}
impl Engine for EpollEngine {
    fn name(&self) -> &'static str { "epoll" }
    fn submit(&mut self, id: Id, op: &Op) {
        for (i, fd, ev) in op.interest() {
            if let Some(r) = self.regs.iter_mut().find(|r| r.0 == fd) { r.1.push((id, i, ev)); self.sync(fd); continue; }
            let bits = ep_bits(ev);
            if sys::epoll_ctl(self.ep, sys::EPOLL_CTL_ADD, fd, EpollEvent { events: bits, data: fd as u64 }) == 0 { self.regs.push((fd, vec![(id, i, ev)], bits)); continue; }
            // EPERM: a regular file / directory, which poll(2) calls always ready; EBADF: POLLNVAL
            let rev = if sys::errno() == EBADF { POLLNVAL } else { ev & (sys::POLLIN | sys::POLLOUT) };
            self.always.push((id, i, rev));
        }
    }
    fn retire(&mut self, id: Id, op: &Op) {
        if op.interest().next().is_none() { return; }
        self.always.retain(|a| a.0 != id);
        let fds: Vec<i32> = op.interest().map(|x| x.1).collect();
        for fd in fds {
            if let Some(r) = self.regs.iter_mut().find(|r| r.0 == fd) { r.1.retain(|w| w.0 != id); }
            self.sync(fd);
        }
    }
    fn pump(&mut self, _slab: &Slab, timeout: Option<Duration>, ready: &mut Vec<Ready>) {
        if !self.always.is_empty() { ready.extend(self.always.iter().map(|&(id, idx, revents)| Ready { id, idx, revents })); return; }
        let ts = timeout.map(timespec);
        let n = sys::epoll_pwait2(self.ep, &mut self.evs, ts.as_ref());
        for e in &self.evs[..n.max(0) as usize] {
            let (bits, data) = (e.events, e.data);
            if data == WAKE_TOKEN { drain_wake(self.wake); continue; }
            let Some(r) = self.regs.iter().find(|r| r.0 == data as i32) else { continue };
            let rev = bits as u16 as i16;   // EPOLLERR/HUP are poll's bits too
            for &(id, idx, ev) in &r.1 { let got = rev & (ev | sys::POLLERR | sys::POLLHUP); if got != 0 { ready.push(Ready { id, idx, revents: got }); } }
        }
    }
    fn after_fork(&mut self, wake_fd: i32) {
        // the epoll instance is shared with the parent: registrations made here would land in its set
        sys::close(self.ep);
        let fresh = EpollEngine::open(wake_fd).expect("saruby: epoll_create1 after fork");
        *self = fresh;
    }
}

pub struct Blop { slab: Slab, engine: Box<dyn Engine>, wake: i32, ready: Vec<Ready> }
/// The wake channel (an eventfd) for the signal handler: -1 before the first blop.
static WAKE_FD: AtomicI32 = AtomicI32::new(-1);
thread_local! { static B: RefCell<Option<Blop>> = const { RefCell::new(None) }; }

fn new_wake() -> i32 {
    let fd = sys::eventfd(0, sys::EFD_CLOEXEC | sys::EFD_NONBLOCK);
    assert!(fd >= 0, "saruby: eventfd");
    WAKE_FD.store(fd, Relaxed);
    fd
}
/// SARUBY_BLOP_ENGINE=poll (default) | epoll. poll is the default: with the few fds a green-thread
/// program waits on, epoll's per-op epoll_ctl costs more than it saves (docs/io_design.md).
fn new_engine(wake: i32) -> Box<dyn Engine> {
    let want = std::env::var("SARUBY_BLOP_ENGINE").unwrap_or_default();
    if want == "epoll" { if let Some(e) = EpollEngine::open(wake) { return Box::new(e); } }
    Box::new(PollEngine { wake, fds: vec![], who: vec![] })
}
fn with<R>(f: impl FnOnce(&mut Blop) -> R) -> R {
    B.with(|b| {
        let mut b = b.borrow_mut();
        let b = b.get_or_insert_with(|| { let wake = new_wake(); Blop { slab: Slab::new(), engine: new_engine(wake), wake, ready: vec![] } });
        let r = f(b);
        b.retire_done();
        r
    })
}
impl Blop {
    /// Completed ops drop their fd interest at once (a level-triggered fd would keep firing).
    fn retire_done(&mut self) {
        while let Some(id) = self.slab.just_done.pop() {
            if let Some(Slot::Done(op, _)) = self.slab.get(id) { self.engine.retire(id, op); }
        }
    }
}

/// The engine's name ("poll" / "epoll").
pub fn engine_name() -> &'static str { with(|b| b.engine.name()) }
/// Enter `op` into the slab. `deadlockable`: only another thread can end it (deadlock detection).
pub fn submit(op: Op, deadline: Option<Instant>, deadlockable: bool) -> Id {
    with(|b| {
        let id = b.slab.insert(Pending { op, deadline, deadlockable });
        if let Some(Slot::Pending(p)) = b.slab.get(id) { b.engine.submit(id, &p.op); }
        id
    })
}
pub fn is_done(id: Id) -> bool { with(|b| matches!(b.slab.get(id), Some(Slot::Done(..)))) }
/// Take the op back: its result, or Err(ECANCELED) if it had not completed (it is withdrawn).
pub fn take(id: Id) -> (Option<Op>, Res) {
    with(|b| {
        if let Some(Slot::Pending(p)) = b.slab.get(id) { b.engine.retire(id, &p.op); }
        match b.slab.release(id) {
            Some(Slot::Done(op, r)) => (Some(op), r),
            Some(Slot::Pending(p)) => (Some(p.op), Err(ECANCELED)),
            _ => (None, Err(ECANCELED)),
        }
    })
}
/// Complete a pending op with Err(errno) (Thread#raise / #kill / #wakeup): its waiter becomes runnable.
pub fn interrupt(id: Id, errno: i64) { with(|b| b.slab.complete(id, Err(errno))) }
/// Complete every Wake(key) op.
pub fn notify(key: u64) {
    with(|b| {
        if b.slab.npending == 0 { return; }
        for i in 0..b.slab.ents.len() {
            let e = &b.slab.ents[i];
            if matches!(&e.slot, Slot::Pending(Pending { op: Op::Wake(k), .. }) if *k == key) { b.slab.complete(Id { idx: i as u32, gen: e.gen }, Ok(0)); }
        }
    })
}
/// IO#close: every op on `fd` completes with Err(EBADF) (before the fd number can be reused).
pub fn cancel_fd(fd: i32) {
    if fd < 0 { return; }
    B.with(|b| {
        let mut g = b.borrow_mut();
        let Some(b) = g.as_mut() else { return };
        let ids: Vec<Id> = b.slab.pending().filter(|(_, p)| p.op.has_fd(fd)).map(|(id, _)| id).collect();
        for id in ids { b.slab.complete(id, Err(EBADF)); }
        b.retire_done();
    })
}
/// Something could complete without another green thread running (a fd, a deadline, a non-deadlockable wait).
pub fn can_finish_alone() -> bool { with(|b| b.slab.pending().any(|(_, p)| p.deadline.is_some() || !p.deadlockable || p.op.interest().next().is_some())) }
/// Collect completions: expired deadlines, ready fds. `block`: sleep until one arrives (the only
/// place the interpreter thread sleeps while green threads wait). Returns the number completed.
pub fn pump(block: bool) -> usize {
    with(|b| {
        if b.slab.npending == 0 && !block { return 0; }
        let has_fd = b.slab.fd_ops().next().is_some();
        // no clock read unless a deadline is pending (clock_gettime is a syscall on some clocksources)
        let now = b.slab.pending().any(|(_, p)| p.deadline.is_some()).then(Instant::now);
        let mut n = now.map_or(0, |t| b.slab.expire(t));
        if (n > 0 || !block) && !has_fd { return n; }
        let timeout = if n > 0 || !block { Some(Duration::ZERO) } else { now.and_then(|t| b.slab.next_deadline().map(|d| d.saturating_duration_since(t))) };
        let mut ready = std::mem::take(&mut b.ready);
        b.engine.pump(&b.slab, timeout, &mut ready);
        n += b.slab.apply_ready(&ready);
        ready.clear(); b.ready = ready;
        if now.is_some() { n += b.slab.expire(Instant::now()); }
        n
    })
}
/// Called from the signal handler: wake a sleeping pump (write(2) is async-signal-safe).
pub fn signal_wake() { let fd = WAKE_FD.load(Relaxed); if fd >= 0 { sys::write(fd, &1u64.to_ne_bytes()); } }
/// Forked child: the other threads' ops are gone, and the eventfd / epoll set are the parent's.
pub fn after_fork() {
    B.with(|b| {
        let mut g = b.borrow_mut();
        let Some(b) = g.as_mut() else { return };
        for i in 0..b.slab.ents.len() as u32 { let gen = b.slab.ents[i as usize].gen; if !matches!(b.slab.ents[i as usize].slot, Slot::Free) { b.slab.release(Id { idx: i, gen }); } }
        b.slab.just_done.clear();
        sys::close(b.wake);
        b.wake = new_wake();
        b.engine.after_fork(b.wake);
    })
}
