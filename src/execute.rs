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

use crate::tick::{price_to_tick, tick_to_price};
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
    mut deps: DepsMut,
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
    let spread_factor = Decimal256::from_str(pool.spread_factor.as_str())?;

    // Get a rough estimate of the ratio of assets in the pool at the tick range the user wants to create a position in
    let (asset_0_ratio, asset_1_ratio) =
        calc_asset_ratio_from_ticks(upper_tick, pool.current_tick, lower_tick)?;

    deps.api.debug(&format!(
        "initial asset ratios: asset_0_ratio {asset_0_ratio:?}, asset_1_ratio {asset_1_ratio:?}"
    ));

    let token_out_denom: String;
    let token_in: Coin;
    let token_provided_swap_amount: Uint128;

    // Depending on which asset we are swapping in, utilize the ratio of assets calculated above to determine
    // how much of the the tokenIn we provided will be used for swapping
    if token_provided.denom == pool.token0.clone() {
        if pool.current_tick > upper_tick {
            // If the current tick is greater than the upper tick, we will swap the entire amount of token0 for token1
            token_provided_swap_amount = token_provided.amount;
        } else {
            // Otherwise, we utilize the ratio of assets calculated above to determine how much of token0 to swap for token1
            let token_provided_dec =
                Decimal256::from_str(token_provided.amount.to_string().as_str())?;
            token_provided_swap_amount = token_provided_dec
                .checked_mul(asset_1_ratio.checked_add(spread_factor)?)?
                .to_uint_floor()
                .try_into()?;
        }
        token_out_denom = pool.token1.clone();
        token_in = Coin {
            denom: pool.token0.clone(),
            amount: token_provided_swap_amount,
        };
    } else if token_provided.denom == pool.token1.clone() {
        if pool.current_tick < lower_tick {
            // If the current tick is less than the lower tick, we will swap the entire amount of token1 for token0
            token_provided_swap_amount = token_provided.amount;
        } else {
            // Otherwise, we utilize the ratio of assets calculated above to determine how much of token1 to swap for token0
            let token_provided_dec =
                Decimal256::from_str(token_provided.amount.to_string().as_str())?;
            token_provided_swap_amount = token_provided_dec
                .checked_mul(asset_0_ratio.checked_add(spread_factor)?)?
                .to_uint_floor()
                .try_into()?;
        }
        token_out_denom = pool.token0.clone();
        token_in = Coin {
            denom: pool.token1.clone(),
            amount: token_provided_swap_amount,
        };
    } else {
        return Err(ContractError::DenomNotInPool {
            provided_denom: token_provided.denom,
        });
    }

    deps.api.debug(&format!("token_in PRE {token_in:?}"));

    // Since we have knowledge of liquidity in the pool, we can further refine this ratio since we will have an idea
    // of where the swap will bring the current tick to.
    let refined_token_in = iterative_naive_approach(
        &mut deps,
        token_in,
        token_provided.clone(),
        pool,
        upper_tick,
        lower_tick,
        //1095,
    )?;

    deps.api.debug(&format!(
        "swapping {refined_token_in:?}, for {token_out_denom:?}"
    ));

    // Create the swap execMsg and store the intermediate state
    let exec_msg = create_swap_exec_msg_and_store_state(
        deps,
        env.clone(),
        info.clone(),
        sender.clone(),
        pool_id,
        token_out_denom.clone(),
        token_provided,
        refined_token_in,
        lower_tick,
        upper_tick,
        token_min_amount0,
        token_min_amount1,
    )?;

    Ok(Response::new()
        .add_attribute("action", "swap_for_single_side_lp")
        .add_submessage(SubMsg::reply_on_success(exec_msg, SWAP_REPLY_ID)))
}

// handle_swap_reply is called after the swap has been executed successfully
// This function will create the provided position on behalf of the user with the tokens that were provided and swapped
pub fn handle_swap_reply(
    deps: DepsMut,
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

        deps.api
            .debug(&format!("token_out_amount {token_out_amount:?}"));

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

        deps.api
            .debug(&format!("tokens_provided {tokens_provided:?}"));

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

pub fn create_swap_exec_msg_and_store_state(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: String,
    pool_id: u64,
    token_out_denom: String,
    token_provided: Coin,
    refined_token_in: Coin,
    lower_tick: i64,
    upper_tick: i64,
    token_min_amount0: Uint128,
    token_min_amount1: Uint128,
) -> Result<MsgExec, ContractError> {
    // Create the swap message for the amount calculated above
    let swap_msg: MsgSwapExactAmountIn = MsgSwapExactAmountIn {
        sender: sender,
        routes: vec![SwapAmountInRoute {
            pool_id: pool_id,
            token_out_denom: token_out_denom.clone(),
        }],
        token_in: Some(refined_token_in.clone().into()),
        token_out_min_amount: "1".to_string(),
    };

    // Execute the swap on behalf of the user
    let exec_msg: MsgExec = MsgExec {
        grantee: env.contract.address.to_string(),
        msgs: vec![swap_msg.to_any()],
    };

    // Remove the amount of tokens we used from the provided amount and note the remaining amount
    let token_provided_remaining: Uint128 =
        token_provided.amount.checked_sub(refined_token_in.amount)?;

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

    return Ok(exec_msg);
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
    if upper_tick < current_tick {
        return Ok((Decimal256::zero(), Decimal256::one()));
    }
    if lower_tick > current_tick {
        return Ok((Decimal256::one(), Decimal256::zero()));
    }
    let delta_x = calc_amount_0_one_unit_liq(upper_tick, current_tick)?;
    let delta_y = calc_amount_1_one_unit_liq(lower_tick, current_tick)?;
    let total_delta = delta_x.checked_add(delta_y)?;
    let asset0_ratio = delta_x.checked_div(total_delta)?;
    let asset1_ratio = Decimal256::one().checked_sub(asset0_ratio)?;

    Ok((asset0_ratio, asset1_ratio))
}

fn get_next_sqrt_price_from_amount0_in_round_up(
    liquidity: Decimal256,
    sqrt_price_current: Decimal256,
    token_in: Decimal256,
) -> Decimal256 {
    let numerator = liquidity * sqrt_price_current;
    let denominator = liquidity + (token_in * sqrt_price_current);
    numerator / denominator
}

fn get_next_sqrt_price_from_amount1_in_round_down(
    liquidity: Decimal256,
    sqrt_price_current: Decimal256,
    token_in: Decimal256,
) -> Decimal256 {
    sqrt_price_current + (token_in / liquidity)
}

fn calc_amount_one_delta(
    liquidity: Decimal256,
    mut sqrt_price_current: Decimal256,
    mut sqrt_price_next: Decimal256,
    should_round_up: bool,
) -> Decimal256 {
    if sqrt_price_next > sqrt_price_current {
        std::mem::swap(&mut sqrt_price_current, &mut sqrt_price_next);
    }
    let result = liquidity * (sqrt_price_current.abs_diff(sqrt_price_next));
    match should_round_up {
        true => result.ceil(),
        false => result,
    }
}

fn calc_amount_zero_delta(
    liquidity: Decimal256,
    mut sqrt_price_current: Decimal256,
    mut sqrt_price_next: Decimal256,
    should_round_up: bool,
) -> Decimal256 {
    if sqrt_price_next > sqrt_price_current {
        std::mem::swap(&mut sqrt_price_current, &mut sqrt_price_next);
    }
    let mul1 = liquidity * (sqrt_price_current - sqrt_price_next);
    let result = mul1 / (sqrt_price_current * sqrt_price_next);
    match should_round_up {
        true => result.ceil(),
        false => result,
    }
}

fn iterative_naive_approach(
    deps: &mut DepsMut,
    // liquidity: Decimal,
    // token_out_denom: String,
    // sqrt_price_current: Decimal,
    // sqrt_price_next: Decimal,
    token_to_swap: Coin,
    token_initially_provided: Coin,
    pool: Pool,
    upper_tick: i64,
    lower_tick: i64,
) -> Result<Coin, ContractError> {
    // We are assuming here we have enough liquidity to swap the entire amount of token_in
    // Will change later
    let token_0_in = token_to_swap.denom == pool.token0;
    let token_1_in = token_to_swap.denom == pool.token1;
    let token_provided_dec =
        Decimal256::from_str(token_initially_provided.amount.to_string().as_str())?;

    let original = pool.current_sqrt_price.to_string();
    let parts: Vec<&str> = original.split('.').collect();
    let integer_part = parts[0];
    let fractional_part = &parts[1][0..18]; // Take only first 18 digits after the decimal point
    let rounded = format!("{}.{}", integer_part, fractional_part);

    let pool_cur_sqrt_price = Decimal256::from_str(&rounded).unwrap();

    let spread_factor = Decimal256::from_str(pool.spread_factor.as_str())?;

    // Determine what the next sqrt price will be after the swap
    let sqrt_price_next: Decimal256;
    if token_0_in {
        sqrt_price_next = get_next_sqrt_price_from_amount0_in_round_up(
            Decimal256::from_str(pool.current_tick_liquidity.as_str()).unwrap(),
            pool_cur_sqrt_price,
            Decimal256::from_str(token_to_swap.amount.to_string().as_str()).unwrap(),
        );
    } else {
        sqrt_price_next = get_next_sqrt_price_from_amount1_in_round_down(
            Decimal256::from_str(pool.current_tick_liquidity.as_str()).unwrap(),
            pool_cur_sqrt_price,
            Decimal256::from_str(token_to_swap.amount.to_string().as_str()).unwrap(),
        );
    }

    // Transform the sqrt price into a price
    let price_next = sqrt_price_to_price(sqrt_price_next);

    // Determine the tick this will bring us to
    let tick_next = price_to_tick(deps.storage, price_next)?;
    deps.api.debug(&format!("tick_next {tick_next:?}"));

    // Now that we have the tick the swap will bring us to, we can determine the optimal ratio of assets
    let (asset_0_ratio_new, asset_1_ratio_new) =
        calc_asset_ratio_from_ticks(upper_tick, tick_next as i64, lower_tick)?;
    deps.api.debug(&format!(
        "asset_0_ratio_new {asset_0_ratio_new:?}, asset_1_ratio_new {asset_1_ratio_new:?}"
    ));

    if token_0_in {
        let amount_1_delta = calc_amount_one_delta(
            Decimal256::from_str(pool.current_tick_liquidity.as_str()).unwrap(),
            pool_cur_sqrt_price,
            sqrt_price_next,
            false,
        );

        let amt0_to_swap = token_provided_dec
            .checked_mul(asset_1_ratio_new.checked_add(spread_factor)?)?
            .to_uint_floor();
        let amt0_for_pos = token_initially_provided
            .amount
            .checked_sub(amt0_to_swap.try_into()?);
        let sqrt_price_next = get_next_sqrt_price_from_amount0_in_round_up(
            Decimal256::from_str(pool.current_tick_liquidity.as_str()).unwrap(),
            pool_cur_sqrt_price,
            Decimal256::from_str(amt0_to_swap.to_string().as_str()).unwrap(),
        );
        let price_next = sqrt_price_to_price(sqrt_price_next);

        // Determine the tick this will bring us to
        let tick_next = price_to_tick(deps.storage, price_next)?;
        deps.api.debug(&format!("tick_next {tick_next:?}"));
        deps.api.debug(&format!(
            "final position pair: amt0 {amt0_for_pos:?}, amt1 {amount_1_delta:?}"
        ));
        return Ok(Coin {
            denom: token_to_swap.denom,
            amount: amt0_to_swap.try_into()?,
        });
    } else if token_1_in {
        let amount_0_delta = calc_amount_zero_delta(
            Decimal256::from_str(pool.current_tick_liquidity.as_str()).unwrap(),
            pool_cur_sqrt_price,
            sqrt_price_next,
            true,
        );
        deps.api
            .debug(&format!("amount_0_delta {amount_0_delta:?}"));
        let amt1_to_swap = token_provided_dec
            .checked_mul(asset_0_ratio_new.checked_add(spread_factor)?)?
            .to_uint_floor();
        let amt1_for_pos = token_initially_provided
            .amount
            .checked_sub(amt1_to_swap.try_into()?);
        let sqrt_price_next = get_next_sqrt_price_from_amount1_in_round_down(
            Decimal256::from_str(pool.current_tick_liquidity.as_str()).unwrap(),
            pool_cur_sqrt_price,
            Decimal256::from_str(amt1_to_swap.to_string().as_str()).unwrap(),
        );
        let price_next = sqrt_price_to_price(sqrt_price_next);

        // Determine the tick this will bring us to
        let tick_next = price_to_tick(deps.storage, price_next)?;
        deps.api.debug(&format!("tick_next {tick_next:?}"));
        deps.api.debug(&format!(
            "final position pair: amt0 {amount_0_delta:?}, amt1 {amt1_for_pos:?}"
        ));
        return Ok(Coin {
            denom: token_to_swap.denom,
            amount: amt1_to_swap.try_into()?,
        });
    } else {
        return Err(ContractError::DenomNotInPool {
            provided_denom: token_to_swap.denom,
        });
    }
}

fn sqrt_price_to_price(sqrt_price: Decimal256) -> Decimal256 {
    return sqrt_price * sqrt_price;
}

// fn iterative_naive_approach_new(
//     deps: &mut DepsMut,
//     // liquidity: Decimal,
//     // token_out_denom: String,
//     // sqrt_price_current: Decimal,
//     // sqrt_price_next: Decimal,
//     mut token_to_swap: Coin,
//     token_initially_provided: Coin,
//     pool: Pool,
//     upper_tick: i64,
//     lower_tick: i64,
//     loop_limit: usize,
// ) -> Result<Coin, ContractError> {
//     let token_0_in = token_to_swap.denom == pool.token0;
//     let token_1_in = token_to_swap.denom == pool.token1;
//     let token_provided_dec =
//         Decimal256::from_str(token_initially_provided.amount.to_string().as_str())?;
//     let threshold = Decimal256::from_str("0.000001")?;

//     let original = pool.current_sqrt_price.to_string();
//     let parts: Vec<&str> = original.split('.').collect();
//     let integer_part = parts[0];
//     let fractional_part = &parts[1][0..18]; // Take only first 18 digits after the decimal point
//     let rounded = format!("{}.{}", integer_part, fractional_part);

//     let pool_cur_sqrt_price = Decimal256::from_str(&rounded).unwrap();

//     let spread_factor = Decimal256::from_str(pool.spread_factor.as_str())?;

//     let mut loop_counter = 0;
//     loop {
//         loop_counter += 1;

//         if loop_counter > loop_limit {
//             return Ok(Coin {
//                 denom: token_to_swap.denom,
//                 amount: token_to_swap.amount,
//             });
//         }

//         let sqrt_price_next: Decimal256;
//         if token_0_in {
//             sqrt_price_next = get_next_sqrt_price_from_amount0_in_round_up(
//                 &deps,
//                 Decimal256::from_str(pool.current_tick_liquidity.as_str()).unwrap(),
//                 pool_cur_sqrt_price,
//                 Decimal256::from_str(token_to_swap.amount.to_string().as_str()).unwrap(),
//             );
//         } else {
//             sqrt_price_next = get_next_sqrt_price_from_amount1_in_round_down(
//                 &deps,
//                 Decimal256::from_str(pool.current_tick_liquidity.as_str()).unwrap(),
//                 pool_cur_sqrt_price,
//                 Decimal256::from_str(token_to_swap.amount.to_string().as_str()).unwrap(),
//             );
//         }

//         let price_next = sqrt_price_to_price(sqrt_price_next);
//         let tick_next = price_to_tick(deps.storage, price_next)?;
//         deps.api.debug(&format!("tick_next {tick_next:?}"));

//         let (asset_0_ratio_new, asset_1_ratio_new) =
//             calc_asset_ratio_from_ticks(upper_tick, tick_next as i64, lower_tick)?;

//         let ratio_difference_abs: Decimal256;
//         if token_0_in {
//             let amount_1_delta = calc_amount_one_delta(
//                 Decimal256::from_str(pool.current_tick_liquidity.as_str()).unwrap(),
//                 pool_cur_sqrt_price,
//                 sqrt_price_next,
//                 false,
//             );
//             let amt0_to_swap = token_provided_dec
//                 .checked_mul(asset_1_ratio_new.checked_add(spread_factor)?)?
//                 .to_uint_floor();
//             // let amount_1_delta_rounded = amount_1_delta.round_dp(18);
//             // let amount_1_delta_256 =
//             //     Decimal256::from_str(&amount_1_delta_rounded.to_string().as_str())?;
//             let amt0_to_swap_dec = Decimal256::from_str(&amt0_to_swap.to_string().as_str())?;

//             let internal_ratio =
//                 amount_1_delta.checked_div(amt0_to_swap_dec.checked_add(amount_1_delta)?)?;
//             // let internal_ratio_dec256 = Decimal256::from_str(&internal_ratio.to_string().as_str())?;
//             let ratio_diff = internal_ratio.checked_sub(asset_1_ratio_new)?;
//             ratio_difference_abs = internal_ratio.abs_diff(asset_1_ratio_new);

//             if ratio_difference_abs <= threshold {
//                 let amt_debug = token_to_swap.amount;
//                 deps.api.debug(&format!("token_to_swap {amt_debug:?}"));
//                 deps.api
//                     .debug(&format!("asset_0_ratio_new {asset_0_ratio_new:?}"));
//                 deps.api
//                     .debug(&format!("asset_1_ratio_new {asset_1_ratio_new:?}"));
//                 return Ok(Coin {
//                     denom: token_to_swap.denom,
//                     amount: amt0_to_swap.try_into()?,
//                 });
//             }
//             deps.api
//                 .debug(&format!("ratio_difference {ratio_difference_abs:?}"));
//             deps.api
//                 .debug(&format!("internal_ratio {internal_ratio:?}"));
//             deps.api
//                 .debug(&format!("asset_1_ratio_new {asset_1_ratio_new:?}"));

//             if ratio_diff < Decimal256::zero() {
//                 token_to_swap.amount = token_to_swap.amount.checked_add(Uint128::one())?;
//             } else {
//                 token_to_swap.amount = token_to_swap.amount.checked_sub(Uint128::one())?;
//             }
//         } else if token_1_in {
//             let amount_0_delta = calc_amount_zero_delta(
//                 Decimal256::from_str(pool.current_tick_liquidity.as_str()).unwrap(),
//                 pool_cur_sqrt_price,
//                 sqrt_price_next,
//                 true,
//             );

//             let amt1_to_swap = token_provided_dec
//                 .checked_mul(asset_0_ratio_new.checked_add(spread_factor)?)?
//                 .to_uint_floor();
//             // let amount_1_delta_rounded = amount_1_delta.round_dp(18);
//             // let amount_1_delta_256 =
//             //     Decimal256::from_str(&amount_1_delta_rounded.to_string().as_str())?;
//             let amt1_to_swap_dec = Decimal256::from_str(&amt1_to_swap.to_string().as_str())?;

//             let internal_ratio =
//                 amount_0_delta.checked_div(amt1_to_swap_dec.checked_add(amount_0_delta)?)?;
//             // let internal_ratio_dec256 = Decimal256::from_str(&internal_ratio.to_string().as_str())?;
//             let ratio_diff = internal_ratio.checked_sub(asset_0_ratio_new)?;
//             ratio_difference_abs = internal_ratio.abs_diff(asset_0_ratio_new);

//             if ratio_difference_abs <= threshold {
//                 return Ok(Coin {
//                     denom: token_to_swap.denom,
//                     amount: amt1_to_swap.try_into()?,
//                 });
//             }
//             deps.api
//                 .debug(&format!("ratio_difference {ratio_difference_abs:?}"));
//             deps.api
//                 .debug(&format!("internal_ratio {internal_ratio:?}"));
//             deps.api
//                 .debug(&format!("asset_0_ratio_new {asset_0_ratio_new:?}"));

//             if ratio_diff < Decimal256::zero() {
//                 token_to_swap.amount = token_to_swap.amount.checked_add(Uint128::one())?;
//             } else {
//                 token_to_swap.amount = token_to_swap.amount.checked_sub(Uint128::one())?;
//             }
//         } else {
//             return Err(ContractError::DenomNotInPool {
//                 provided_denom: token_to_swap.denom,
//             });
//         }
//     }
// }
