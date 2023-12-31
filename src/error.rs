use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyRatioError, Coin, ConversionOverflowError, Decimal256,
    Decimal256RangeExceeded, DivideByZeroError, OverflowError, StdError, Uint128,
};
use cw_utils::PaymentError;
use thiserror::Error;

use std::num::ParseIntError;

/// AutocompoundingVault errors
#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Pool-id {pool_id} not found")]
    PoolNotFound { pool_id: u64 },

    #[error("Position Not Found")]
    PositionNotFound,

    #[error("Modify range state item not found")]
    ModifyRangeStateNotFound,

    #[error("Cannot do two swaps at the same time")]
    SwapInProgress,

    #[error("Swap deposit merge state item not found")]
    SwapDepositMergeStateNotFound,

    #[error("Swap failed: {message}")]
    SwapFailed { message: String },

    #[error("Vault shares sent in does not equal amount requested")]
    IncorrectShares,

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("{0}")]
    CheckedMultiplyRatioError(#[from] CheckedMultiplyRatioError),

    #[error("{0}")]
    Decimal256RangeExceededError(#[from] Decimal256RangeExceeded),

    #[error("Overflow")]
    Overflow {},

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    ConversionOverflowError(#[from] ConversionOverflowError),

    #[error("{0}")]
    MultiplyRatioError(#[from] CheckedFromRatioError),

    #[error("This message does no accept funds")]
    NonPayable {},

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    ParseIntError(#[from] ParseIntError),

    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.

    // todo: add apollo errors one by one and see what gives us type errors
    // apollo errors below (remove from above when you add)
    #[error("Unexpected funds sent. Expected: {expected:?}, Actual: {actual:?}")]
    UnexpectedFunds {
        expected: Vec<Coin>,
        actual: Vec<Coin>,
    },

    #[error("Bad token out requested for swap, must be one of: {base_token:?}, {quote_token:?}")]
    BadTokenForSwap {
        base_token: String,
        quote_token: String,
    },

    #[error("Insufficient funds for swap. Have: {balance}, Need: {needed}")]
    InsufficientFundsForSwap { balance: Uint128, needed: Uint128 },

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error("Cannot merge positions that are in different ticks")]
    DifferentTicksInMerge,

    #[error("Tick index minimum error")]
    TickIndexMinError {},

    #[error("Tick index maximum error")]
    TickIndexMaxError {},

    #[error("Price must be between 0.000000000001 and 100000000000000000000000000000000000000. Got {:?}", price)]
    PriceBoundError { price: Decimal256 },

    #[error("Cannot handle negative powers in uints")]
    CannotHandleNegativePowersInUint {},

    #[error("Denom (provided_denom) does not exist in pool")]
    DenomNotInPool { provided_denom: String },

    #[error("Failed Swap: {reason:?}")]
    FailedSwap { reason: String },
}
