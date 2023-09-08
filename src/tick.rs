use std::str::FromStr;

use cosmwasm_std::{Decimal, Decimal256};

use crate::ContractError;

const MAX_SPOT_PRICE: &str = "100000000000000000000000000000000000000"; // 10^35
const MIN_SPOT_PRICE: &str = "0.000000000001"; // 10^-12
const EXPONENT_AT_PRICE_ONE: i64 = -6;
const MIN_INITIALIZED_TICK: i64 = -108000000;
const MAX_TICK: i128 = 342000000;

// The methods in this file are copied from the Quasar cl vault contract.

pub fn tick_to_price(tick_index: i64) -> Result<Decimal256, ContractError> {
    if tick_index == 0 {
        return Ok(Decimal256::one());
    }

    let geometric_exponent_increment_distance_in_ticks = Decimal::from_str("9")?
        .checked_mul(pow_ten_internal_dec(-EXPONENT_AT_PRICE_ONE)?)?
        .to_string()
        .parse::<i64>()?;

    // Check that the tick index is between min and max value
    if tick_index < MIN_INITIALIZED_TICK {
        return Err(ContractError::TickIndexMinError {});
    }

    if tick_index > MAX_TICK as i64 {
        return Err(ContractError::TickIndexMaxError {});
    }

    // Use floor division to determine what the geometricExponent is now (the delta)
    let geometric_exponent_delta = tick_index / geometric_exponent_increment_distance_in_ticks;

    // Calculate the exponentAtCurrentTick from the starting exponentAtPriceOne and the geometricExponentDelta
    let mut exponent_at_current_tick = EXPONENT_AT_PRICE_ONE + geometric_exponent_delta;

    if tick_index < 0 {
        // We must decrement the exponentAtCurrentTick when entering the negative tick range in order to constantly step up in precision when going further down in ticks
        // Otherwise, from tick 0 to tick -(geometricExponentIncrementDistanceInTicks), we would use the same exponent as the exponentAtPriceOne
        exponent_at_current_tick -= 1
    }

    // Knowing what our exponentAtCurrentTick is, we can then figure out what power of 10 this exponent corresponds to
    // We need to utilize bigDec here since increments can go beyond the 10^-18 limits set by the sdk
    let current_additive_increment_in_ticks = pow_ten_internal_dec_256(exponent_at_current_tick)?;

    // Now, starting at the minimum tick of the current increment, we calculate how many ticks in the current geometricExponent we have passed
    let num_additive_ticks =
        tick_index - (geometric_exponent_delta * geometric_exponent_increment_distance_in_ticks);

    // Finally, we can calculate the price

    let price: Decimal256 = if num_additive_ticks < 0 {
        pow_ten_internal_dec(geometric_exponent_delta)?
            .checked_sub(
                Decimal::from_str(&num_additive_ticks.abs().to_string())?.checked_mul(
                    Decimal::from_str(&current_additive_increment_in_ticks.to_string())?,
                )?,
            )?
            .into()
    } else {
        pow_ten_internal_dec_256(geometric_exponent_delta)?.checked_add(
            Decimal256::from_str(&num_additive_ticks.to_string())?
                .checked_mul(current_additive_increment_in_ticks)?,
        )?
    };

    // defense in depth, this logic would not be reached due to use having checked if given tick is in between
    // min tick and max tick.
    if price > Decimal256::from_str(MAX_SPOT_PRICE)?
        || price < Decimal256::from_str(MIN_SPOT_PRICE)?
    {
        return Err(ContractError::PriceBoundError { price });
    }
    Ok(price)
}

// same as pow_ten_internal but returns a Decimal to work with negative exponents
fn pow_ten_internal_dec(exponent: i64) -> Result<Decimal, ContractError> {
    let p = 10u128
        .checked_pow(exponent.unsigned_abs() as u32)
        .ok_or(ContractError::Overflow {})?;
    if exponent >= 0 {
        Ok(Decimal::from_ratio(p, 1u128))
    } else {
        Ok(Decimal::from_ratio(1u128, p))
    }
}

// same as pow_ten_internal but returns a Decimal to work with negative exponents
fn pow_ten_internal_dec_256(exponent: i64) -> Result<Decimal256, ContractError> {
    let p = Decimal256::from_str("10")?.checked_pow(exponent.unsigned_abs() as u32)?;
    // let p = 10_u128.pow(exponent as u32);
    if exponent >= 0 {
        Ok(p)
    } else {
        Ok(Decimal256::one() / p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_to_price() {
        // example1
        let tick_index = 38035200;
        let expected_price = Decimal256::from_str("30352").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example2
        let tick_index = 38035300;
        let expected_price = Decimal256::from_str("30353").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example3
        let tick_index = -44821000;
        let expected_price = Decimal256::from_str("0.000011790").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example4
        let tick_index = -44820900;
        let expected_price = Decimal256::from_str("0.000011791").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example5
        let tick_index = -12104000;
        let expected_price = Decimal256::from_str("0.068960").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example6
        let tick_index = -12103900;
        let expected_price = Decimal256::from_str("0.068961").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example7
        let tick_index = MAX_TICK as i64 - 100;
        let expected_price =
            Decimal256::from_str("99999000000000000000000000000000000000").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example8
        let tick_index = MAX_TICK as i64;
        let expected_price = Decimal256::from_str(MAX_SPOT_PRICE).unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example9
        let tick_index = -20594000;
        let expected_price = Decimal256::from_str("0.007406").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example10
        let tick_index = -20593900;
        let expected_price = Decimal256::from_str("0.0074061").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example11
        let tick_index = -29204000;
        let expected_price = Decimal256::from_str("0.00077960").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example12
        let tick_index = -29203900;
        let expected_price = Decimal256::from_str("0.00077961").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example13
        let tick_index = -12150000;
        let expected_price = Decimal256::from_str("0.068500").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example14
        let tick_index = -12149900;
        let expected_price = Decimal256::from_str("0.068501").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example15
        let tick_index = 64576000;
        let expected_price = Decimal256::from_str("25760000").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example16
        let tick_index = 64576100;
        let expected_price = Decimal256::from_str("25761000").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example17
        let tick_index = 0;
        let expected_price = Decimal256::from_str("1").unwrap();
        let price = tick_to_price(tick_index).unwrap();
        assert_eq!(price, expected_price);

        // example19
        assert!(tick_to_price(MAX_TICK as i64 + 1).is_err());

        // example20
        assert!(tick_to_price(MIN_INITIALIZED_TICK - 1).is_err());
    }
}
