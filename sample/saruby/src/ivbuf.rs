// Instance-variable slots of an object.  Slot i belongs to the class's i-th ivar; unset = UNDEF.
//
//   slots 0..INLINE      in the GC slot itself (no malloc, no length to check)
//   slots INLINE..FIXED  in one fixed-size block, made when the object first needs a 4th slot
//   slots FIXED..        in that block's Vec
//
// A slot never moves once it exists, so which tier holds slot i depends on i alone: a site's
// cached index picks the tier with a branch that is constant per site, not per object.
// No unsafe, and no self pointer (the value may be moved, as a moving collector would).
use crate::value::*;

pub const INLINE: usize = 3;
const CHUNK: usize = 5;
const FIXED: usize = INLINE + CHUNK;

pub struct IvBuf { w: [Value; INLINE], ext: Option<Box<Ext>> }
struct Ext { w: [Value; CHUNK], more: Vec<Value> }

impl IvBuf {
    /// At least n slots, all unset.
    #[inline(always)]
    pub fn filled(n: usize) -> IvBuf {
        IvBuf { w: [UNDEF; INLINE], ext: if n <= INLINE { None } else { Some(Ext::filled(n - INLINE)) } }
    }
    /// True when every slot is in the GC slot (nothing to free).
    #[inline(always)]
    pub fn is_inline(&self) -> bool { self.ext.is_none() }
    #[inline]
    pub fn len(&self) -> usize {
        match &self.ext { None => INLINE, Some(e) => FIXED + e.more.len() }
    }
    /// Slot i (UNDEF: unset); None beyond len().
    #[inline(always)]
    pub fn get(&self, i: usize) -> Option<Value> {
        if i < INLINE { return Some(self.w[i]); }
        let e = self.ext.as_ref()?;
        let j = i - INLINE;
        if j < CHUNK { Some(e.w[j]) } else { e.more.get(j - CHUNK).copied() }
    }
    /// Store into an existing slot; false beyond len().
    #[inline(always)]
    pub fn set(&mut self, i: usize, v: Value) -> bool {
        if i < INLINE { self.w[i] = v; return true; }
        let Some(e) = self.ext.as_mut() else { return false };
        let j = i - INLINE;
        if j < CHUNK { e.w[j] = v; return true; }
        // kept inline: an out-of-line call here cost the callers their leaf frames (object AOT +3% instructions)
        match e.more.get_mut(j - CHUNK) { Some(x) => { *x = v; true } None => false }
    }
    /// Make it at least n slots long (new slots unset).
    #[inline(never)]
    pub fn resize(&mut self, n: usize) {
        if n <= INLINE { return; }
        let e = self.ext.get_or_insert_with(|| Ext::filled(0));
        if n > FIXED + e.more.len() { e.more.resize(n - FIXED, UNDEF); }
    }
    /// The slots in index order (unset ones as UNDEF).
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = Value> + '_ {
        let (a, b): (&[Value], &[Value]) = match &self.ext { None => (&[], &[]), Some(e) => (&e.w, &e.more) };
        self.w.iter().chain(a).chain(b).copied()
    }
    /// The slots in place (a moving collector's edge update).
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Value> + '_ {
        let (a, b): (&mut [Value], &mut [Value]) = match &mut self.ext { None => (&mut [], &mut []), Some(e) => (&mut e.w, &mut e.more) };
        self.w.iter_mut().chain(a).chain(b)
    }
}

impl Ext {
    // Allocated first and written in place: built on the stack, the block was copied to the heap
    // by wide loads that stalled on the narrow stores (obj7 AOT +12% cycles).
    #[inline(never)]
    fn filled(k: usize) -> Box<Ext> {
        if k > CHUNK { return Ext::filled_more(k - CHUNK); }
        Box::write(Box::new_uninit(), Ext { w: [UNDEF; CHUNK], more: Vec::new() })
    }
    #[cold]
    #[inline(never)]
    fn filled_more(k: usize) -> Box<Ext> { Box::write(Box::new_uninit(), Ext { w: [UNDEF; CHUNK], more: vec![UNDEF; k] }) }
}

impl Clone for IvBuf {
    fn clone(&self) -> IvBuf {
        IvBuf { w: self.w, ext: self.ext.as_ref().map(|e| Box::new(Ext { w: e.w, more: e.more.clone() })) }
    }
}
