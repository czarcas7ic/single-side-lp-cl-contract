#[cfg(not(feature = "imported"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Reply, Response};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::execute::{handle_swap_reply, single_sided_swap_and_lp};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::state::SWAP_REPLY_STATES;
use crate::state::{Config, CONFIG};

// Msg Reply IDs
pub const SWAP_REPLY_ID: u64 = 1u64;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:osmosis-single-sided-swap-and-lp";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Handling contract instantiation
#[cfg_attr(not(feature = "imported"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    let state = Config { owner };

    CONFIG.save(deps.storage, &state)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "imported"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    match msg {}
}

/// Handling contract execution
#[cfg_attr(not(feature = "imported"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SingleSidedSwapAndJoin {
            pool_id,
            lower_tick,
            upper_tick,
            token_provided,
            token_min_amount0,
            token_min_amount1,
        } => single_sided_swap_and_lp(
            &env,
            &info,
            deps,
            pool_id,
            lower_tick,
            upper_tick,
            token_provided,
            token_min_amount0,
            token_min_amount1,
        ),
    }
}

#[cfg_attr(not(feature = "imported"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api
        .debug(&format!("executing swaprouter reply: {msg:?}"));
    if msg.id == SWAP_REPLY_ID {
        // get intermediate swap reply state. Error if not found.
        let swap_msg_state = SWAP_REPLY_STATES.load(deps.storage, msg.id)?;

        // prune intermedate state
        SWAP_REPLY_STATES.remove(deps.storage, msg.id);

        // call reply function to handle the swap return
        handle_swap_reply(deps, env, msg, swap_msg_state)
    } else {
        Ok(Response::new())
    }
}
