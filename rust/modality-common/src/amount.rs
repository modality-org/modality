//! Asset amounts as people read them. A ledger counts smallest units; an
//! asset's display `decimals` say where the point goes.

use anyhow::{anyhow, bail, Result};

/// `amount` smallest units shown with `decimals` places: 150000000 at 8 is
/// "1.5". Without decimals, the smallest units.
pub fn format_amount(amount: u64, decimals: Option<u32>) -> String {
    let places = decimals.unwrap_or(0).min(19);
    if places == 0 {
        return amount.to_string();
    }
    let unit = 10u64.pow(places);
    let fraction = format!("{:0width$}", amount % unit, width = places as usize);
    let fraction = fraction.trim_end_matches('0');
    if fraction.is_empty() {
        (amount / unit).to_string()
    } else {
        format!("{}.{}", amount / unit, fraction)
    }
}

/// "1.5" at `decimals` places as smallest units. Refuses more decimal places
/// than the asset shows.
pub fn parse_amount(text: &str, decimals: Option<u32>) -> Result<u64> {
    let places = decimals.unwrap_or(0).min(19);
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if whole.is_empty() && fraction.is_empty() {
        bail!("amount is empty");
    }
    if fraction.len() > places as usize {
        bail!("{text} has more than {places} decimal place(s)");
    }
    let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    if !digits(whole) || !digits(fraction) {
        bail!("{text} is not an amount");
    }
    let whole: u64 = if whole.is_empty() { 0 } else { whole.parse()? };
    let fraction: u64 = format!("{:0<width$}", fraction, width = places as usize)
        .parse()
        .unwrap_or(0);
    whole
        .checked_mul(10u64.pow(places))
        .and_then(|w| w.checked_add(fraction))
        .filter(|n| *n > 0)
        .ok_or_else(|| anyhow!("{text} is not a positive amount that fits"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_read_and_print_in_whole_units() {
        let mod_places = Some(8);
        assert_eq!(format_amount(150_000_000, mod_places), "1.5");
        assert_eq!(format_amount(5_000_000_000, mod_places), "50");
        assert_eq!(format_amount(1, mod_places), "0.00000001");
        assert_eq!(format_amount(7, Some(0)), "7");
        assert_eq!(format_amount(7, None), "7");

        assert_eq!(parse_amount("1.5", mod_places).unwrap(), 150_000_000);
        assert_eq!(parse_amount("50", mod_places).unwrap(), 5_000_000_000);
        assert_eq!(parse_amount(".25", Some(2)).unwrap(), 25);
        assert!(parse_amount("0.000000001", mod_places).is_err(), "nine places");
        assert!(parse_amount("1.5", None).is_err(), "no decimals shown");
        assert!(parse_amount("0", mod_places).is_err());
        assert!(parse_amount("-1", mod_places).is_err());
        assert!(parse_amount("1e3", mod_places).is_err());
        assert!(parse_amount("999999999999", mod_places).is_err(), "overflow");
    }
}
