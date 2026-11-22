// Float to text for the json generator: a port of the json gem's vendor/fpconv.c (Grisu2, from
// https://github.com/night-shift/fpconv, Boost Software License 1.0; the json gem appends ".0" to
// plain floats). Grisu2 is not always shortest, so this keeps its digits exactly.
#![forbid(unsafe_code)]

const NPOWERS: i32 = 87;
const STEPPOWERS: i32 = 8;
const FIRSTPOWER: i32 = -348;
const EXPMAX: i32 = -32;
const EXPMIN: i32 = -60;

#[derive(Clone, Copy)]
struct Fp { frac: u64, exp: i32 }

static POWERS_TEN: [(u64, i32); 87] = [
    (18054884314459144840, -1220), (13451937075301367670, -1193), (10022474136428063862, -1166), (14934650266808366570, -1140),
    (11127181549972568877, -1113), (16580792590934885855, -1087), (12353653155963782858, -1060), (18408377700990114895, -1034),
    (13715310171984221708, -1007), (10218702384817765436, -980), (15227053142812498563, -954), (11345038669416679861, -927),
    (16905424996341287883, -901), (12595523146049147757, -874), (9384396036005875287, -847), (13983839803942852151, -821),
    (10418772551374772303, -794), (15525180923007089351, -768), (11567161174868858868, -741), (17236413322193710309, -715),
    (12842128665889583758, -688), (9568131466127621947, -661), (14257626930069360058, -635), (10622759856335341974, -608),
    (15829145694278690180, -582), (11793632577567316726, -555), (17573882009934360870, -529), (13093562431584567480, -502),
    (9755464219737475723, -475), (14536774485912137811, -449), (10830740992659433045, -422), (16139061738043178685, -396),
    (12024538023802026127, -369), (17917957937422433684, -343), (13349918974505688015, -316), (9946464728195732843, -289),
    (14821387422376473014, -263), (11042794154864902060, -236), (16455045573212060422, -210), (12259964326927110867, -183),
    (18268770466636286478, -157), (13611294676837538539, -130), (10141204801825835212, -103), (15111572745182864684, -77),
    (11258999068426240000, -50), (16777216000000000000, -24), (12500000000000000000, 3), (9313225746154785156, 30),
    (13877787807814456755, 56), (10339757656912845936, 83), (15407439555097886824, 109), (11479437019748901445, 136),
    (17105694144590052135, 162), (12744735289059618216, 189), (9495567745759798747, 216), (14149498560666738074, 242),
    (10542197943230523224, 269), (15709099088952724970, 295), (11704190886730495818, 322), (17440603504673385349, 348),
    (12994262207056124023, 375), (9681479787123295682, 402), (14426529090290212157, 428), (10748601772107342003, 455),
    (16016664761464807395, 481), (11933345169920330789, 508), (17782069995880619868, 534), (13248674568444952270, 561),
    (9871031767461413346, 588), (14708983551653345445, 614), (10959046745042015199, 641), (16330252207878254650, 667),
    (12166986024289022870, 694), (18130221999122236476, 720), (13508068024458167312, 747), (10064294952495520794, 774),
    (14996968138956309548, 800), (11173611982879273257, 827), (16649979327439178909, 853), (12405201291620119593, 880),
    (9242595204427927429, 907), (13772540099066387757, 933), (10261342003245940623, 960), (15290591125556738113, 986),
    (11392378155556871081, 1013), (16975966327722178521, 1039), (12648080533535911531, 1066),
];

fn find_cachedpow10(exp: i32, k: &mut i32) -> Fp {
    let one_log_ten = 0.30102999566398114f64;
    let approx = (-(exp + NPOWERS) as f64 * one_log_ten) as i32;
    let mut idx = (approx - FIRSTPOWER) / STEPPOWERS;
    loop {
        let current = exp + POWERS_TEN[idx as usize].1 + 64;
        if current < EXPMIN { idx += 1; continue; }
        if current > EXPMAX { idx -= 1; continue; }
        *k = FIRSTPOWER + idx * STEPPOWERS;
        let (frac, exp) = POWERS_TEN[idx as usize];
        return Fp { frac, exp };
    }
}

const FRACMASK: u64 = 0x000F_FFFF_FFFF_FFFF;
const EXPMASK: u64 = 0x7FF0_0000_0000_0000;
const HIDDENBIT: u64 = 0x0010_0000_0000_0000;
const SIGNMASK: u64 = 0x8000_0000_0000_0000;
const EXPBIAS: i32 = 1023 + 52;

static TENS: [u64; 20] = [
    10000000000000000000, 1000000000000000000, 100000000000000000, 10000000000000000, 1000000000000000, 100000000000000,
    10000000000000, 1000000000000, 100000000000, 10000000000, 1000000000, 100000000, 10000000, 1000000, 100000, 10000, 1000, 100, 10, 1,
];

fn build_fp(d: f64) -> Fp {
    let bits = d.to_bits();
    let mut fp = Fp { frac: bits & FRACMASK, exp: ((bits & EXPMASK) >> 52) as i32 };
    if fp.exp != 0 { fp.frac += HIDDENBIT; fp.exp -= EXPBIAS; } else { fp.exp = -EXPBIAS + 1; }
    fp
}

fn normalize(fp: &mut Fp) {
    while fp.frac & HIDDENBIT == 0 { fp.frac <<= 1; fp.exp -= 1; }
    let shift = 64 - 52 - 1;
    fp.frac <<= shift;
    fp.exp -= shift;
}

fn get_normalized_boundaries(fp: &Fp) -> (Fp, Fp) {
    let mut upper = Fp { frac: (fp.frac << 1) + 1, exp: fp.exp - 1 };
    while upper.frac & (HIDDENBIT << 1) == 0 { upper.frac <<= 1; upper.exp -= 1; }
    let u_shift = 64 - 52 - 2;
    upper.frac <<= u_shift;
    upper.exp -= u_shift;
    let l_shift = if fp.frac == HIDDENBIT { 2 } else { 1 };
    let mut lower = Fp { frac: (fp.frac << l_shift) - 1, exp: fp.exp - l_shift };
    lower.frac <<= lower.exp - upper.exp;
    lower.exp = upper.exp;
    (lower, upper)
}

fn multiply(a: &Fp, b: &Fp) -> Fp {
    let lomask = 0x0000_0000_FFFF_FFFFu64;
    let ah_bl = (a.frac >> 32) * (b.frac & lomask);
    let al_bh = (a.frac & lomask) * (b.frac >> 32);
    let al_bl = (a.frac & lomask) * (b.frac & lomask);
    let ah_bh = (a.frac >> 32) * (b.frac >> 32);
    let mut tmp = (ah_bl & lomask) + (al_bh & lomask) + (al_bl >> 32);
    tmp += 1u64 << 31;
    Fp { frac: ah_bh + (ah_bl >> 32) + (al_bh >> 32) + (tmp >> 32), exp: a.exp + b.exp + 64 }
}

fn round_digit(digits: &mut [u8], ndigits: usize, delta: u64, mut rem: u64, kappa: u64, frac: u64) {
    while rem < frac && delta.wrapping_sub(rem) >= kappa && (rem.wrapping_add(kappa) < frac || frac.wrapping_sub(rem) > rem.wrapping_add(kappa).wrapping_sub(frac)) {
        digits[ndigits - 1] -= 1;
        rem = rem.wrapping_add(kappa);
    }
}

fn generate_digits(fp: &Fp, upper: &Fp, lower: &Fp, digits: &mut [u8; 18], k: &mut i32) -> usize {
    let wfrac = upper.frac.wrapping_sub(fp.frac);
    let mut delta = upper.frac.wrapping_sub(lower.frac);
    let one_exp = upper.exp;
    let one_frac = 1u64 << (-one_exp);
    let mut part1 = upper.frac >> (-one_exp);
    let mut part2 = upper.frac & (one_frac - 1);
    let mut idx = 0usize;
    let mut kappa = 10;
    let mut divp = 10;
    while kappa > 0 {
        let div = TENS[divp];
        let digit = (part1 / div) as u32;
        if digit != 0 || idx != 0 { digits[idx] = digit as u8 + b'0'; idx += 1; }
        part1 -= digit as u64 * div;
        kappa -= 1;
        let tmp = (part1 << (-one_exp)).wrapping_add(part2);
        if tmp <= delta {
            *k += kappa;
            round_digit(digits, idx, delta, tmp, div << (-one_exp), wfrac);
            return idx;
        }
        divp += 1;
    }
    let mut unit = 18usize;
    loop {
        part2 = part2.wrapping_mul(10);
        delta = delta.wrapping_mul(10);
        kappa -= 1;
        let digit = (part2 >> (-one_exp)) as u32;
        if digit != 0 || idx != 0 { digits[idx] = digit as u8 + b'0'; idx += 1; }
        part2 &= one_frac - 1;
        if part2 < delta {
            *k += kappa;
            round_digit(digits, idx, delta, part2, one_frac, wfrac.wrapping_mul(TENS[unit]));
            return idx;
        }
        unit -= 1;
    }
}

fn grisu2(d: f64, digits: &mut [u8; 18], k: &mut i32) -> usize {
    let mut w = build_fp(d);
    let (mut lower, mut upper) = get_normalized_boundaries(&w);
    normalize(&mut w);
    let mut kk = 0;
    let cp = find_cachedpow10(upper.exp, &mut kk);
    w = multiply(&w, &cp);
    upper = multiply(&upper, &cp);
    lower = multiply(&lower, &cp);
    lower.frac += 1;
    upper.frac -= 1;
    *k = -kk;
    generate_digits(&w, &upper, &lower, digits, k)
}

/// emit_digits; `v2` is json 2.x's rule for plain integers (up to 7 trailing zeros, 6 if negative).
fn emit_digits(digits: &[u8], ndigits: usize, out: &mut Vec<u8>, k: i32, neg: bool, v2: bool) {
    let nd = ndigits as i32;
    let mut exp = (k + nd - 1).abs();
    let plain = if v2 { let mtz = if neg { 6 } else { 7 }; k >= 0 && exp < nd + mtz } else { k >= 0 && exp < 15 };
    if plain {
        out.extend_from_slice(&digits[..ndigits]);
        out.extend(std::iter::repeat_n(b'0', k as usize));
        out.extend_from_slice(b".0");
        return;
    }
    if k < 0 && (k > -7 || exp < 10) {
        let offset = nd - k.abs();
        if offset <= 0 {
            out.extend_from_slice(b"0.");
            out.extend(std::iter::repeat_n(b'0', (-offset) as usize));
            out.extend_from_slice(&digits[..ndigits]);
        } else {
            let o = offset as usize;
            out.extend_from_slice(&digits[..o]);
            out.push(b'.');
            out.extend_from_slice(&digits[o..ndigits]);
        }
        return;
    }
    let ndigits = ndigits.min(18 - neg as usize);
    out.push(digits[0]);
    if ndigits > 1 { out.push(b'.'); out.extend_from_slice(&digits[1..ndigits]); }
    out.push(b'e');
    out.push(if k + ndigits as i32 - 1 < 0 { b'-' } else { b'+' });
    let mut cent = 0;
    if exp > 99 { cent = exp / 100; out.push(cent as u8 + b'0'); exp -= cent * 100; }
    if exp > 9 { let dec = exp / 10; out.push(dec as u8 + b'0'); exp -= dec * 10; } else if cent != 0 { out.push(b'0'); }
    out.push((exp % 10) as u8 + b'0');
}

/// fpconv_dtoa of a finite double.
pub fn dtoa(d: f64, out: &mut Vec<u8>, v2: bool) {
    let bits = d.to_bits();
    let neg = bits & SIGNMASK != 0;
    if neg { out.push(b'-'); }
    if d == 0.0 { out.extend_from_slice(b"0.0"); return; }
    if bits & EXPMASK == EXPMASK { out.extend_from_slice(if bits & FRACMASK != 0 { b"nan" } else { b"inf" }); return; }
    let mut digits = [0u8; 18];
    let mut k = 0;
    let nd = grisu2(d, &mut digits, &mut k);
    emit_digits(&digits, nd, out, k, neg, v2);
}
