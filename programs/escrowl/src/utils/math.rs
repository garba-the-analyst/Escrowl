//! Fee + sum helpers with checked arithmetic.
use anchor_lang::prelude::*;

use crate::constants::BPS_DENOMINATOR;
use crate::errors::EscrowError;

/// fee = floor(amount * fee_bps / 10_000). Dust stays with seller.
pub fn calc_fee(amount: u64, fee_bps: u16) -> Result<u64> {
    let fee = (amount as u128)
        .checked_mul(fee_bps as u128)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR as u128))
        .ok_or(EscrowError::MathOverflow)?;
    u64::try_from(fee).map_err(|_| error!(EscrowError::MathOverflow))
}

/// seller proceeds = amount - fee.
pub fn calc_seller_amount(amount: u64, fee_bps: u16) -> Result<u64> {
    let fee = calc_fee(amount, fee_bps)?;
    amount
        .checked_sub(fee)
        .ok_or(error!(EscrowError::MathOverflow))
}

/// Checked sum of milestone amounts.
pub fn checked_sum(amounts: &[u64]) -> Result<u64> {
    let mut total: u64 = 0;
    for a in amounts {
        total = total
            .checked_add(*a)
            .ok_or(error!(EscrowError::MathOverflow))?;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn fee_zero_amount_gives_zero_fee() {
        assert_eq!(calc_fee(0, 500).unwrap(), 0);
    }

    #[test]
    fn fee_zero_bps_gives_zero_fee() {
        assert_eq!(calc_fee(1_000_000, 0).unwrap(), 0);
    }

    #[test]
    fn fee_standard_250bps() {
        // I5: 2.5% of 1_000_000 = 25_000.
        assert_eq!(calc_fee(1_000_000, 250).unwrap(), 25_000);
        assert_eq!(calc_seller_amount(1_000_000, 250).unwrap(), 975_000);
    }

    #[test]
    fn fee_floors_and_dust_goes_to_seller() {
        // I5: 1 * 1bps / 10000 floors to 0; seller keeps dust.
        assert_eq!(calc_fee(1, 1).unwrap(), 0);
        assert_eq!(calc_seller_amount(1, 1).unwrap(), 1);
        // 9_999 at 100bps (1%) = 99.99 -> floor 99.
        assert_eq!(calc_fee(9_999, 100).unwrap(), 99);
        assert_eq!(calc_seller_amount(9_999, 100).unwrap(), 9_900);
    }

    #[test]
    fn fee_max_bps_boundary() {
        assert_eq!(calc_fee(10_000, 500).unwrap(), 500);
        assert_eq!(calc_fee(10_000, 501).unwrap(), 501); // program rejects >500 upstream
    }

    #[test]
    fn fee_u64_max_does_not_overflow() {
        // u128 intermediate must hold u64::MAX * 10_000.
        let fee = calc_fee(u64::MAX, 10_000).unwrap();
        assert_eq!(fee, u64::MAX);
        assert_eq!(calc_seller_amount(u64::MAX, 10_000).unwrap(), 0);
    }

    #[test]
    fn checked_sum_ok_and_overflow() {
        assert_eq!(checked_sum(&[100, 200, 300]).unwrap(), 600);
        assert!(checked_sum(&[u64::MAX, 1]).is_err());
    }
}
