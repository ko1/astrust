// STUB (literal matching only) — replaced by the real engine.
pub const OPT_I: u32 = 1;
pub const OPT_X: u32 = 2;
pub const OPT_M: u32 = 4;
pub struct Regex { lit: Vec<u8> }
impl Regex {
    pub fn new(pat: &[u8], _opts: u32) -> Result<Regex, String> { Ok(Regex { lit: pat.to_vec() }) }
    pub fn search(&self, s: &[u8], start: usize) -> Option<Vec<Option<(usize, usize)>>> {
        if self.lit.is_empty() { return Some(vec![Some((start, start))]); }
        let n = self.lit.len();
        (start..=s.len().saturating_sub(n)).find(|&i| s.len() >= i + n && s[i..i + n] == self.lit[..]).map(|i| vec![Some((i, i + n))])
    }
    pub fn match_at(&self, s: &[u8], pos: usize) -> Option<Vec<Option<(usize, usize)>>> {
        let n = self.lit.len();
        if s.len() >= pos + n && s[pos..pos + n] == self.lit[..] { Some(vec![Some((pos, pos + n))]) } else { None }
    }
    pub fn group_count(&self) -> usize { 0 }
    pub fn names(&self) -> Vec<(String, Vec<usize>)> { vec![] }
    pub fn group_index(&self, _name: &str) -> Vec<usize> { vec![] }
}
