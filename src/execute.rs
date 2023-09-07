use cosmwasm_std::{Coin, Decimal256, Env, QuerierWrapper, Storage, Uint128};
use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::{
    ConcentratedliquidityQuerier, FullPositionBreakdown, MsgCreatePosition, MsgWithdrawPosition,
};
use std::str::FromStr;

use crate::math::tick::tick_to_price;
use crate::{error::ContractResult, ContractError};
use crate::{msg::ExecuteMsg, state::CONFIG};
use cosmwasm_std::{Addr, Decimal, Deps, MessageInfo, Response};
use osmosis_std::types::cosmos::authz::v1beta1::MsgExec;

pub fn create_position(
    env: &Env,
    info: &MessageInfo,
    pool_id: u64,
    lower_tick: i64,
    upper_tick: i64,
    tokens_provided: Vec<Coin>,
    token_min_amount0: Uint128,
    token_min_amount1: Uint128,
) -> Result<Response, ContractError> {
    //let sender = env.contract.address.to_string();
    let sender = info.sender.to_string();
    //let tokens_provided = vec![Coin::new(100, "token0"), Coin::new(200, "token1")];

    let create_position_msg = MsgCreatePosition {
        pool_id,
        sender: sender,
        lower_tick,
        upper_tick,
        tokens_provided: tokens_provided.into_iter().map(|c| c.into()).collect(),
        // An sdk.Int in the Go code
        token_min_amount0: token_min_amount0.to_string(),
        // An sdk.Int in the Go code
        token_min_amount1: token_min_amount1.to_string(),
    };

    let exec_msg: MsgExec = MsgExec {
        grantee: env.contract.address.to_string(),
        msgs: vec![create_position_msg.to_any()],
    };

    Ok(Response::default().add_message(exec_msg))
}

// /// get_spot_price
// ///
// /// gets the spot price of the pool which this vault is managing funds in. This will always return token0 in terms of token1 (or would it be the other way around?)
// pub fn get_spot_price(
//     storage: &dyn Storage,
//     querier: &QuerierWrapper,
// ) -> Result<Decimal, ContractError> {
//     let pool_config = POOL_CONFIG.load(storage)?;

//     let pm_querier = PoolmanagerQuerier::new(querier);
//     let spot_price =
//         pm_querier.spot_price(pool_config.pool_id, pool_config.token0, pool_config.token1)?;

//     Ok(Decimal::from_str(&spot_price.spot_price)?)
// }

// // this math is straight from the readme
// pub fn get_single_sided_deposit_0_to_1_swap_amount(
//     storage: &dyn Storage,
//     querier: &QuerierWrapper,
//     token0_balance: Uint128,
//     lower_tick: i64,
//     upper_tick: i64,
// ) -> Result<Uint128, ContractError> {
//     let spot_price = Decimal256::from(get_spot_price(storage, querier)?);
//     let lower_price = tick_to_price(lower_tick)?;
//     let upper_price = tick_to_price(upper_tick)?;
//     let pool_metadata_constant: Uint128 = spot_price
//         .checked_mul(lower_price.sqrt())?
//         .checked_mul(upper_price.sqrt())?
//         .to_uint_floor() // todo: this is big, so should be safe, right?
//         .try_into()?;

//     let swap_amount = token0_balance.checked_multiply_ratio(
//         pool_metadata_constant,
//         pool_metadata_constant.checked_add(Uint128::one())?,
//     )?;

//     Ok(swap_amount)
// }

// pub fn get_single_sided_deposit_1_to_0_swap_amount(
//     storage: &dyn Storage,
//     querier: &QuerierWrapper,
//     token1_balance: Uint128,
//     lower_tick: i64,
//     upper_tick: i64,
// ) -> Result<Uint128, ContractError> {
//     let spot_price = Decimal256::from(get_spot_price(storage, querier)?);
//     let lower_price = tick_to_price(lower_tick)?;
//     let upper_price = tick_to_price(upper_tick)?;
//     let pool_metadata_constant: Uint128 = spot_price
//         .checked_mul(lower_price.sqrt())?
//         .checked_mul(upper_price.sqrt())?
//         .to_uint_floor() // todo: this is big, so should be safe, right?
//         .try_into()?;

//     let swap_amount =
//         token1_balance.checked_div(pool_metadata_constant.checked_add(Uint128::one())?)?;

//     Ok(swap_amount)
// }
