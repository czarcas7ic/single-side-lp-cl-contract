use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal256, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct Config {
    pub owner: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct TickExpIndexData {
    pub initial_price: Decimal256,
    pub max_price: Decimal256,
    pub additive_increment_per_tick: Decimal256,
    pub initial_tick: i64,
}

pub const TICK_EXP_CACHE: Map<i64, TickExpIndexData> = Map::new("tick_exp_cache");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct SwapMsgReplyState {
    pub pool_id: u64,
    pub original_sender: Addr,
    pub lower_tick: i64,
    pub upper_tick: i64,
    pub token_min_amount0: Uint128,
    pub token_min_amount1: Uint128,
    pub token_provided_remaining_coin: Coin,
    pub token_out_denom: String,
}

pub const SWAP_REPLY_STATES: Map<u64, SwapMsgReplyState> = Map::new("swap_reply_states");
