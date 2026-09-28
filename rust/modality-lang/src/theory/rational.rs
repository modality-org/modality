//! Exact rationals on `i128`.
//!
//! Comparison is exact for every pair: the cross products are formed in
//! 256 bits, so no verdict depends on overflow (the Lean spec compares
//! unbounded integers). Arithmetic, used only to build witnesses, is
//! checked: `None` there means "no witness", never a verdict. There is no
//! `f64` here.

use std::cmp::Ordering;
use std::fmt;

/// `DBL_DIG`: decimals with this many significant digits survive a round
/// trip through `f64`.
pub const MAX_SIGNIFICANT_DIGITS: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    num: i128,
    den: i128, // always > 0, gcd(num, den) == 1
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

impl Rational {
    pub fn new(num: i128, den: i128) -> Option<Self> {
        if den == 0 {
            return None;
        }
        let (num, den) = if den < 0 {
            (num.checked_neg()?, den.checked_neg()?)
        } else {
            (num, den)
        };
        let g = gcd(num, den);
        let g = if g == 0 { 1 } else { g };
        Some(Self {
            num: num / g,
            den: den / g,
        })
    }

    pub fn from_int(n: i128) -> Self {
        Self { num: n, den: 1 }
    }

    /// Parse a decimal literal: optional sign, digits, optional fraction.
    /// Anything else (whitespace, exponents, hex, words) is `None`.
    ///
    /// At most 15 significant digits. Accepted-state numbers are compared
    /// as `f64` by the evaluator; two decimals of at most 15 significant
    /// digits round to distinct doubles in the same order, so within this
    /// domain exact comparison and the evaluator agree. Longer literals are
    /// `None` (the predicate stays opaque).
    pub fn parse(text: &str) -> Option<Self> {
        let s = text;
        if s.is_empty() {
            return None;
        }
        let (neg, body) = match s.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, s.strip_prefix('+').unwrap_or(s)),
        };
        if body.is_empty() {
            return None;
        }
        let (int_part, frac_part) = match body.split_once('.') {
            Some((i, f)) => (i, f),
            None => (body, ""),
        };
        if int_part.is_empty() && frac_part.is_empty() {
            return None;
        }
        if !int_part.chars().all(|c| c.is_ascii_digit())
            || !frac_part.chars().all(|c| c.is_ascii_digit())
        {
            return None;
        }
        let digits: String = int_part.chars().chain(frac_part.chars()).collect();
        let significant = digits.trim_start_matches('0').trim_end_matches('0');
        if significant.len() > MAX_SIGNIFICANT_DIGITS {
            return None;
        }
        let mut num: i128 = 0;
        for c in int_part.chars().chain(frac_part.chars()) {
            num = num.checked_mul(10)?.checked_add((c as u8 - b'0') as i128)?;
        }
        let mut den: i128 = 1;
        for _ in 0..frac_part.len() {
            den = den.checked_mul(10)?;
        }
        if neg {
            num = num.checked_neg()?;
        }
        Self::new(num, den)
    }

    pub fn num(&self) -> i128 {
        self.num
    }

    /// Always positive.
    pub fn den(&self) -> i128 {
        self.den
    }

    /// Exact comparison: `self.num * other.den` against `other.num *
    /// self.den`, in 256 bits.
    pub fn cmp_exact(&self, other: &Self) -> Ordering {
        let l = (
            self.num.signum(),
            wide_mul(self.num.unsigned_abs(), other.den as u128),
        );
        let r = (
            other.num.signum(),
            wide_mul(other.num.unsigned_abs(), self.den as u128),
        );
        match l.0.cmp(&r.0) {
            Ordering::Equal if l.0 < 0 => r.1.cmp(&l.1),
            Ordering::Equal => l.1.cmp(&r.1),
            o => o,
        }
    }

    /// Kept for callers that predate exact comparison; always `Some`.
    pub fn try_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp_exact(other))
    }

    /// Finite decimal spelling (`-12.5`), when the denominator divides a
    /// power of ten.
    pub fn to_decimal(&self) -> Option<String> {
        let mut den = self.den;
        let (mut twos, mut fives) = (0u32, 0u32);
        while den % 2 == 0 {
            den /= 2;
            twos += 1;
        }
        while den % 5 == 0 {
            den /= 5;
            fives += 1;
        }
        if den != 1 {
            return None;
        }
        let places = twos.max(fives);
        let scaled = self
            .num
            .checked_mul(10i128.checked_pow(places)?)?
            .checked_div(self.den)?;
        let digits = scaled.unsigned_abs().to_string();
        let sign = if scaled < 0 { "-" } else { "" };
        if places == 0 {
            return Some(format!("{sign}{digits}"));
        }
        let places = places as usize;
        let padded = format!("{digits:0>width$}", width = places + 1);
        let (int, frac) = padded.split_at(padded.len() - places);
        Some(format!("{sign}{int}.{frac}"))
    }

    pub fn checked_add(&self, other: &Self) -> Option<Self> {
        let n = self
            .num
            .checked_mul(other.den)?
            .checked_add(other.num.checked_mul(self.den)?)?;
        Self::new(n, self.den.checked_mul(other.den)?)
    }

    pub fn checked_sub(&self, other: &Self) -> Option<Self> {
        self.checked_add(&Self::new(other.num.checked_neg()?, other.den)?)
    }

    pub fn checked_mul(&self, other: &Self) -> Option<Self> {
        Self::new(
            self.num.checked_mul(other.num)?,
            self.den.checked_mul(other.den)?,
        )
    }
}

/// `a * b` as `(high, low)` 128-bit halves.
fn wide_mul(a: u128, b: u128) -> (u128, u128) {
    const M: u128 = u64::MAX as u128;
    let (a1, a0) = (a >> 64, a & M);
    let (b1, b0) = (b >> 64, b & M);
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let mid = (p00 >> 64) + (p01 & M) + (p10 & M);
    let lo = (p00 & M) | ((mid & M) << 64);
    let hi = p11 + (p01 >> 64) + (p10 >> 64) + (mid >> 64);
    (hi, lo)
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rational {
    /// Exact, and consistent with `==` (values are normalised).
    fn cmp(&self, other: &Self) -> Ordering {
        self.cmp_exact(other)
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_decimals_exactly() {
        let a = Rational::parse("0.1").unwrap();
        let b = Rational::parse("0.2").unwrap();
        assert_eq!(a.try_cmp(&b), Some(Ordering::Less));
        assert_eq!(Rational::parse("5").unwrap(), Rational::from_int(5));
        assert_eq!(
            Rational::parse("-3.50").unwrap(),
            Rational::new(-7, 2).unwrap()
        );
    }

    #[test]
    fn rejects_non_decimals_and_overflow() {
        assert!(Rational::parse("five").is_none());
        assert!(Rational::parse("1e5").is_none());
        assert!(Rational::parse("").is_none());
        assert!(Rational::parse("5 ").is_none());
        assert!(Rational::parse(" 5").is_none());
        assert!(Rational::parse("170141183460469231731687303715884105728").is_none());
    }

    #[test]
    fn comparison_is_exact_beyond_i128_products() {
        let big = Rational::new(i128::MAX, 1).unwrap();
        let tiny = Rational::new(1, i128::MAX).unwrap();
        let neg = Rational::new(-i128::MAX, 3).unwrap();
        assert_eq!(big.cmp_exact(&tiny), Ordering::Greater);
        assert_eq!(tiny.cmp_exact(&big), Ordering::Less);
        assert_eq!(neg.cmp_exact(&tiny), Ordering::Less);
        let a = Rational::new(i128::MAX - 1, i128::MAX).unwrap();
        let b = Rational::new(i128::MAX - 2, i128::MAX - 1).unwrap();
        assert_eq!(a.cmp_exact(&b), Ordering::Greater);
        assert_eq!(a.cmp_exact(&a), Ordering::Equal);
        let m = Rational::new(i128::MIN + 1, 7).unwrap();
        assert_eq!(m.cmp_exact(&neg), Ordering::Greater);
        assert_eq!(neg.cmp_exact(&m), Ordering::Less);
    }

    #[test]
    fn decimals_spell_back_exactly() {
        for s in ["0", "5", "-3.5", "0.001", "-0.25", "123.456"] {
            let q = Rational::parse(s).unwrap();
            assert_eq!(q.to_decimal().as_deref(), Some(s), "{s}");
        }
        assert_eq!(Rational::new(1, 3).unwrap().to_decimal(), None);
    }

    #[test]
    fn at_most_fifteen_significant_digits() {
        assert!(Rational::parse("123456789012345").is_some());
        assert!(Rational::parse("1234567890123456").is_none());
        assert!(Rational::parse("0.000000000000000000001").is_some());
        assert!(Rational::parse("1000000000000000000000").is_some());
        assert!(Rational::parse("0.10000000000000000001").is_none());
        assert!(Rational::parse("9007199254740993").is_none());
    }
}
