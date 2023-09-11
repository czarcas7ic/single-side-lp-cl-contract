use crate::contract::SWAP_REPLY_ID;
use crate::state::{SwapMsgReplyState, SWAP_REPLY_STATES};
use cosmwasm_std::Decimal256;
use cosmwasm_std::{
    Coin, DepsMut, Env, MessageInfo, Reply, Response, SubMsg, SubMsgResponse, SubMsgResult, Uint128,
};
use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::MsgCreatePosition;
use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::Pool;
use osmosis_std::types::osmosis::poolmanager::v1beta1::MsgSwapExactAmountInResponse;
use osmosis_std::types::osmosis::poolmanager::v1beta1::PoolmanagerQuerier;
use osmosis_std::types::osmosis::poolmanager::v1beta1::{MsgSwapExactAmountIn, SwapAmountInRoute};
use std::str::FromStr;

use crate::tick::tick_to_price;
use crate::ContractError;
use osmosis_std::types::cosmos::authz::v1beta1::MsgExec;

// swap_for_single_side_lp is the primary entry point for the contract
// The paramaters to note are:
// - pool_id: The id of the pool the position will be created in
// - lower_tick: The desired lower tick of the position
// - upper_tick: The desired upper tick of the position
// - token_provided: The amount of tokens to be provided to the pool. This value must be a length of 1. This will be the token that is swapped for the other token.
// - token_min_amount0: The minimum amount of token0 that will be used to create the position.
// - token_min_amount1: The minimum amount of token1 that will be used to create the position.
pub fn single_sided_swap_and_lp(
    env: &Env,
    info: &MessageInfo,
    deps: DepsMut,
    pool_id: u64,
    lower_tick: i64,
    upper_tick: i64,
    token_provided: Coin,
    token_min_amount0: Uint128,
    token_min_amount1: Uint128,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();

    // Get the pool the position will be created in
    let pool: Pool = PoolmanagerQuerier::new(&deps.querier)
        .pool(pool_id)?
        .pool
        .ok_or(ContractError::PoolNotFound { pool_id: pool_id })?
        .try_into()
        .unwrap();

    // Determine if the swap strategy is one for zero or zero for one
    // Then, determine how much of the token provided is needed to swap for the other token
    let token_out_denom: String;
    let token_in: Coin;
    let token_provided_swap_amount: Uint128;

    let (asset_0_ratio, asset_1_ratio) =
        calc_asset_ratio_from_ticks(upper_tick, pool.current_tick, lower_tick)?;

    if token_provided.denom == pool.token0 {
        if pool.current_tick > upper_tick {
            // If the current tick is greater than the upper tick, we will swap the entire amount of token0 for token1
            token_provided_swap_amount = token_provided.amount;
        } else {
            // Otherwise, we utilize the ratio of assets to determine how much of token0 to swap for token1
            let token_provided_dec =
                Decimal256::from_str(token_provided.amount.to_string().as_str())?;
            token_provided_swap_amount = token_provided_dec
                .checked_mul(asset_1_ratio)?
                .to_uint_floor()
                .try_into()?;
        }
        token_out_denom = pool.token1;
        token_in = Coin {
            denom: pool.token0,
            amount: token_provided_swap_amount,
        };
    } else if token_provided.denom == pool.token1 {
        if pool.current_tick < lower_tick {
            // If the current tick is less than the lower tick, we will swap the entire amount of token1 for token0
            token_provided_swap_amount = token_provided.amount;
        } else {
            // Otherwise, we utilize the ratio of assets to determine how much of token1 to swap for token0
            let token_provided_dec =
                Decimal256::from_str(token_provided.amount.to_string().as_str())?;
            token_provided_swap_amount = token_provided_dec
                .checked_mul(asset_0_ratio)?
                .to_uint_floor()
                .try_into()?;
        }
        token_out_denom = pool.token0;
        token_in = Coin {
            denom: pool.token1,
            amount: token_provided_swap_amount,
        };
    } else {
        return Err(ContractError::DenomNotInPool {
            provided_denom: token_provided.denom,
        });
    }

    // Create the swap message for the amount calculated above
    let swap_msg: MsgSwapExactAmountIn = MsgSwapExactAmountIn {
        sender: sender,
        routes: vec![SwapAmountInRoute {
            pool_id: pool_id,
            token_out_denom: token_out_denom.clone(),
        }],
        token_in: Some(token_in.into()),
        token_out_min_amount: "1".to_string(),
    };

    // Execute the swap on behalf of the user
    let exec_msg: MsgExec = MsgExec {
        grantee: env.contract.address.to_string(),
        msgs: vec![swap_msg.to_any()],
    };

    // Remove the amount of tokens we used from the provided amount and note the remaining amount
    let token_provided_remaining: Uint128 = token_provided
        .amount
        .checked_sub(token_provided_swap_amount)?;

    // Store the remaining amount as a coin
    let token_provided_remaining_coin: Coin = Coin {
        denom: token_provided.denom,
        amount: token_provided_remaining,
    };

    // Save intermediate state
    // We will utilize this state after the swap has been executed
    SWAP_REPLY_STATES.save(
        deps.storage,
        SWAP_REPLY_ID,
        &SwapMsgReplyState {
            pool_id: pool_id,
            original_sender: info.sender.clone(),
            lower_tick: lower_tick,
            upper_tick: upper_tick,
            token_min_amount0: token_min_amount0,
            token_min_amount1: token_min_amount1,
            token_provided_remaining_coin: token_provided_remaining_coin,
            token_out_denom: token_out_denom,
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "swap_for_single_side_lp")
        .add_submessage(SubMsg::reply_on_success(exec_msg, SWAP_REPLY_ID)))
}

// handle_swap_reply is called after the swap has been executed successfully
// This function will create the provided position on behalf of the user with the tokens that were provided and swapped
pub fn handle_swap_reply(
    _deps: DepsMut,
    env: Env,
    msg: Reply,
    swap_msg_reply_state: SwapMsgReplyState,
) -> Result<Response, ContractError> {
    if let SubMsgResult::Ok(SubMsgResponse { data: Some(b), .. }) = msg.result {
        // Parse the swap response
        let res: MsgSwapExactAmountInResponse = b.try_into().map_err(ContractError::Std)?;

        // TODO: For some reason, this is returning a string with a new line character and a few other non relevant characters
        // followed by the correct value. My guess is, the actual response type is MsgExecResponse and not MsgSwapExactAmountInResponse,
        // but I am not aware of how to parse the MsgExecResponse type into MsgSwapExactAmountInResponse. For now, we just filter out the
        // non relevant characters and parse the string into a Uint128.
        let token_out_amount = res
            .token_out_amount
            .chars()
            .filter(|c| c.is_digit(10))
            .collect::<String>();
        let token_out_amount = Uint128::from_str(&token_out_amount)?;

        // Create the tokens provided vector and order it lexicographically
        let token_out_coin = Coin {
            denom: swap_msg_reply_state.token_out_denom,
            amount: token_out_amount,
        };
        let token_in_coin = Coin {
            denom: swap_msg_reply_state.token_provided_remaining_coin.denom,
            amount: swap_msg_reply_state.token_provided_remaining_coin.amount,
        };
        let mut tokens_provided = vec![token_out_coin, token_in_coin];
        tokens_provided.sort_by(|a, b| a.denom.cmp(&b.denom));

        // Create the create position message
        let create_position_msg = MsgCreatePosition {
            pool_id: swap_msg_reply_state.pool_id,
            sender: swap_msg_reply_state.original_sender.to_string(),
            lower_tick: swap_msg_reply_state.lower_tick,
            upper_tick: swap_msg_reply_state.upper_tick,
            tokens_provided: tokens_provided.into_iter().map(|c| c.into()).collect(),
            token_min_amount0: swap_msg_reply_state.token_min_amount0.to_string(),
            token_min_amount1: swap_msg_reply_state.token_min_amount1.to_string(),
        };

        // Execute the create position message on behalf of the user
        let exec_msg: MsgExec = MsgExec {
            grantee: env.contract.address.to_string(),
            msgs: vec![create_position_msg.to_any()],
        };

        return Ok(Response::default().add_message(exec_msg));
    }

    Err(ContractError::FailedSwap {
        reason: msg.result.unwrap_err(),
    })
}

pub fn calc_amount_0_one_unit_liq(
    upper_tick: i64,
    current_tick: i64,
) -> Result<Decimal256, ContractError> {
    let p_upper = tick_to_price(upper_tick)?;
    let p_current = tick_to_price(current_tick)?;
    let delta_x = Decimal256::one()
        .checked_mul(p_upper.sqrt().checked_sub(p_current.sqrt())?)?
        .checked_div(p_upper.sqrt().checked_mul(p_current.sqrt())?)?;

    Ok(delta_x)
}

pub fn calc_amount_1_one_unit_liq(
    lower_tick: i64,
    current_tick: i64,
) -> Result<Decimal256, ContractError> {
    let p_lower = tick_to_price(lower_tick)?;
    let p_current = tick_to_price(current_tick)?;
    let delta_y = Decimal256::one().checked_mul(p_current.sqrt().checked_sub(p_lower.sqrt())?)?;

    Ok(delta_y)
}

pub fn calc_asset_ratio_from_ticks(
    upper_tick: i64,
    current_tick: i64,
    lower_tick: i64,
) -> Result<(Decimal256, Decimal256), ContractError> {
    let delta_x = calc_amount_0_one_unit_liq(upper_tick, current_tick)?;
    let delta_y = calc_amount_1_one_unit_liq(lower_tick, current_tick)?;
    let total_delta = delta_x.checked_add(delta_y)?;
    let asset0_ratio = delta_x.checked_div(total_delta)?;
    let asset1_ratio = Decimal256::one().checked_sub(asset0_ratio)?;

    Ok((asset0_ratio, asset1_ratio))
}
