use crate::contract::SWAP_REPLY_ID;
use crate::state::{SwapMsgReplyState, SWAP_REPLY_STATES};
use cosmwasm_std::{
    Coin, DepsMut, Env, MessageInfo, Reply, Response, SubMsg, SubMsgResponse, SubMsgResult, Uint128,
};
use cosmwasm_std::{Decimal256, QuerierWrapper};
use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::MsgCreatePosition;
use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::Pool;
use osmosis_std::types::osmosis::poolmanager::v1beta1::MsgSwapExactAmountInResponse;
use osmosis_std::types::osmosis::poolmanager::v1beta1::PoolmanagerQuerier;
use osmosis_std::types::osmosis::poolmanager::v1beta1::{MsgSwapExactAmountIn, SwapAmountInRoute};
use std::str::FromStr;

use crate::tick::tick_to_price;
use crate::ContractError;
use cosmwasm_std::Decimal;
use osmosis_std::types::cosmos::authz::v1beta1::MsgExec;

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
    //let sender = env.contract.address.to_string();
    let sender = info.sender.to_string();

    // Get the pool
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
    if token_provided.denom == pool.token0 {
        token_provided_swap_amount = get_single_sided_deposit_0_to_1_swap_amount(
            &deps.querier,
            token_provided.amount,
            lower_tick,
            upper_tick,
            pool.clone(),
        )?;
        token_out_denom = pool.token1;
        token_in = Coin {
            denom: pool.token0,
            amount: token_provided_swap_amount,
        };
    } else if token_provided.denom == pool.token1 {
        token_provided_swap_amount = get_single_sided_deposit_1_to_0_swap_amount(
            &deps.querier,
            token_provided.amount,
            lower_tick,
            upper_tick,
            pool.clone(),
        )?;
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
        token_out_min_amount: "0".to_string(),
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
            swap_msg: swap_msg.clone(),
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "swap_for_single_side_lp")
        .add_submessage(SubMsg::reply_on_success(exec_msg, SWAP_REPLY_ID)))
}

pub fn handle_swap_reply(
    _deps: DepsMut,
    env: Env,
    msg: Reply,
    swap_msg_reply_state: SwapMsgReplyState,
) -> Result<Response, ContractError> {
    if let SubMsgResult::Ok(SubMsgResponse { data: Some(b), .. }) = msg.result {
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

// The methods below this comment are copied from the Quasar cl vault contract.

/// get_spot_price
///
/// gets the spot price of the pool which this vault is managing funds in. This will always return token0 in terms of token1.
pub fn get_spot_price(
    querier: &QuerierWrapper,
    pool_config: Pool,
) -> Result<Decimal, ContractError> {
    let pm_querier = PoolmanagerQuerier::new(querier);
    let spot_price =
        pm_querier.spot_price(pool_config.id, pool_config.token0, pool_config.token1)?;

    Ok(Decimal::from_str(&spot_price.spot_price)?)
}

pub fn get_single_sided_deposit_0_to_1_swap_amount(
    querier: &QuerierWrapper,
    token0_balance: Uint128,
    lower_tick: i64,
    upper_tick: i64,
    pool_config: Pool,
) -> Result<Uint128, ContractError> {
    let spot_price = Decimal256::from(get_spot_price(querier, pool_config)?);
    let lower_price = tick_to_price(lower_tick)?;
    let upper_price = tick_to_price(upper_tick)?;
    let pool_metadata_constant: Uint128 = spot_price
        .checked_mul(lower_price.sqrt())?
        .checked_mul(upper_price.sqrt())?
        .to_uint_floor() // todo: this is big, so should be safe, right?
        .try_into()?;

    let swap_amount = token0_balance.checked_multiply_ratio(
        pool_metadata_constant,
        pool_metadata_constant.checked_add(Uint128::one())?,
    )?;

    Ok(swap_amount)
}

pub fn get_single_sided_deposit_1_to_0_swap_amount(
    querier: &QuerierWrapper,
    token1_balance: Uint128,
    lower_tick: i64,
    upper_tick: i64,
    pool_config: Pool,
) -> Result<Uint128, ContractError> {
    let spot_price = Decimal256::from(get_spot_price(querier, pool_config)?);
    let lower_price = tick_to_price(lower_tick)?;
    let upper_price = tick_to_price(upper_tick)?;
    let pool_metadata_constant: Uint128 = spot_price
        .checked_mul(lower_price.sqrt())?
        .checked_mul(upper_price.sqrt())?
        .to_uint_floor() // todo: this is big, so should be safe, right?
        .try_into()?;

    let swap_amount =
        token1_balance.checked_div(pool_metadata_constant.checked_add(Uint128::one())?)?;

    Ok(swap_amount)
}
