use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cosmwasm_std::{Coin, Decimal256, Env, QuerierWrapper, Storage, Uint128};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    /// Execute a swap and forward it to the receiver address on the specified ibc channel
    CreatePosition {
        pool_id: u64,
        lower_tick: i64,
        upper_tick: i64,
        tokens_provided: Vec<Coin>,
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
