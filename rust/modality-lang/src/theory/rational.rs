//! Exact rationals on `i128`, with checked arithmetic.
//!
//! Every operation that could overflow returns `None`. The theory maps
//! `None` to `Unknown`, never to a verdict. There is no `f64` here.

use std::cmp::Ordering;
use std::fmt;

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
    /// Anything else (exponents, hex, words) is `None`.
    pub fn parse(text: &str) -> Option<Self> {
        let s = text.trim();
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

    /// Checked comparison; `None` on intermediate overflow.
    pub fn try_cmp(&self, other: &Self) -> Option<Ordering> {
        let l = self.num.checked_mul(other.den)?;
        let r = other.num.checked_mul(self.den)?;
        Some(l.cmp(&r))
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rational {
    /// Total order for deterministic output. Exact when no overflow;
    /// otherwise falls back to a structural comparison. Decision logic
    /// must use `try_cmp`, never this.
    fn cmp(&self, other: &Self) -> Ordering {
        self.try_cmp(other)
            .unwrap_or_else(|| (self.num, self.den).cmp(&(other.num, other.den)))
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
        assert!(Rational::parse("170141183460469231731687303715884105728").is_none());
    }
}
