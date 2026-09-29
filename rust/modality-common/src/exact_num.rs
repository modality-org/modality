//! Exact numbers for predicates that do arithmetic on posted `.num` values.
//!
//! A value is a decimal, held as a fraction of big integers, so a product
//! or a fee never rounds and never overflows.

use num_bigint::{BigInt, Sign};
use serde_json::Value;
use std::cmp::Ordering;
use std::fmt;

/// Largest decimal exponent accepted, so `1e999999` cannot cost a huge power.
const MAX_EXPONENT: u32 = 400;

#[derive(Clone, Debug)]
pub struct Exact {
    num: BigInt,
    den: BigInt, // always > 0
}

impl Exact {
    pub fn zero() -> Self {
        Self::from_int(0)
    }

    pub fn from_int(n: i64) -> Self {
        Self {
            num: BigInt::from(n),
            den: BigInt::from(1),
        }
    }

    /// A decimal: optional sign, digits, optional fraction, optional
    /// exponent (`1e3`, as JSON writes large numbers). Anything else is `None`.
    pub fn parse(text: &str) -> Option<Self> {
        let (mantissa, exponent) = match text.find(['e', 'E']) {
            Some(i) => (&text[..i], text[i + 1..].parse::<i64>().ok()?),
            None => (text, 0),
        };
        let (neg, body) = match mantissa.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, mantissa.strip_prefix('+').unwrap_or(mantissa)),
        };
        let (int_part, frac_part) = body.split_once('.').unwrap_or((body, ""));
        if int_part.is_empty() && frac_part.is_empty() {
            return None;
        }
        let digits = format!("{int_part}{frac_part}");
        if !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let exponent = exponent.checked_sub(i64::try_from(frac_part.len()).ok()?)?;
        if exponent.unsigned_abs() > u64::from(MAX_EXPONENT) {
            return None;
        }
        let mut num = BigInt::parse_bytes(digits.as_bytes(), 10)?;
        if neg {
            num = -num;
        }
        let scale = BigInt::from(10).pow(u32::try_from(exponent.unsigned_abs()).ok()?);
        Some(if exponent >= 0 {
            Self {
                num: num * scale,
                den: BigInt::from(1),
            }
        } else {
            Self { num, den: scale }
        })
    }

    /// A JSON number; any other value is `None`.
    pub fn from_json(value: &Value) -> Option<Self> {
        match value {
            Value::Number(n) => Self::parse(&n.to_string()),
            _ => None,
        }
    }

    pub fn is_negative(&self) -> bool {
        self.num.sign() == Sign::Minus
    }

    pub fn mul(&self, other: &Self) -> Self {
        Self {
            num: &self.num * &other.num,
            den: &self.den * &other.den,
        }
    }

    pub fn sub(&self, other: &Self) -> Self {
        Self {
            num: &self.num * &other.den - &other.num * &self.den,
            den: &self.den * &other.den,
        }
    }
}

impl PartialEq for Exact {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Exact {}

impl PartialOrd for Exact {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Exact {
    fn cmp(&self, other: &Self) -> Ordering {
        (&self.num * &other.den).cmp(&(&other.num * &self.den))
    }
}

impl fmt::Display for Exact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let g = gcd(&self.num, &self.den);
        let (num, den) = (&self.num / &g, &self.den / &g);
        if den == BigInt::from(1) {
            write!(f, "{num}")
        } else {
            write!(f, "{num}/{den}")
        }
    }
}

fn gcd(a: &BigInt, b: &BigInt) -> BigInt {
    let (mut a, mut b) = (a.magnitude().clone(), b.magnitude().clone());
    while b != num_bigint::BigUint::from(0u8) {
        let t = &a % &b;
        a = b;
        b = t;
    }
    if a == num_bigint::BigUint::from(0u8) {
        BigInt::from(1)
    } else {
        BigInt::from(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn x(text: &str) -> Exact {
        Exact::parse(text).unwrap()
    }

    #[test]
    fn parses_decimals_exactly() {
        assert_eq!(x("0.1").mul(&x("3")), x("0.3"));
        assert_eq!(x("1e3"), x("1000"));
        assert_eq!(x("-2.50"), x("-2.5"));
        assert_eq!(x("25e-1"), x("2.5"));
        assert!(x("0.1") < x("0.10000000000000001"));
        assert_eq!(x("2.50").to_string(), "5/2");
        assert_eq!(x("1e3").to_string(), "1000");
        for bad in ["", "-", ".", "1.2.3", "0x10", "five", "1e", "1e99999", " 1"] {
            assert!(Exact::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn reads_json_numbers_only() {
        assert_eq!(Exact::from_json(&json!(10)), Some(x("10")));
        assert_eq!(Exact::from_json(&json!(2.5)), Some(x("2.5")));
        assert_eq!(Exact::from_json(&json!("10")), None);
    }

    #[test]
    fn products_do_not_overflow() {
        let big = x("340282366920938463463374607431768211455");
        assert!(big.mul(&big) > big);
        assert_eq!(big.mul(&big).sub(&big.mul(&big)), Exact::zero());
    }
}
