use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    /// Execute a swap and forward it to the receiver address on the specified ibc channel
    SingleSidedSwapAndJoin {
        pool_id: u64,
        lower_tick: i64,
        upper_tick: i64,
        token_provided: Coin,
        token_min_amount0: Uint128,
        token_min_amount1: Uint128,
    },
}

/// Message type for `migrate` entry_point
#[cw_serde]
pub enum MigrateMsg {}

/// Message type for `query` entry_point
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // This example query variant indicates that any client can query the contract
    // using `YourQuery` and it will return `YourQueryResponse`
    // This `returns` information will be included in contract's schema
    // which is used for client code generation.
    //
    // #[returns(YourQueryResponse)]
    // YourQuery {},
}

// We define a custom struct for each query response
// #[cw_serde]
// pub struct YourQueryResponse {}

// Response for Swap
#[cw_serde]
pub struct SwapResponse {
    pub original_sender: String,
    pub token_out_denom: String,
    pub amount: Uint128,
}
