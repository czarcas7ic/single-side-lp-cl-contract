#[cfg(not(feature = "imported"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::execute::create_position;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::state::{Config, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:outpost";
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

    // Store the contract addr and the osmosis channel
    let state = Config { owner: msg.owner };
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
        ExecuteMsg::CreatePosition {
            pool_id,
            lower_tick,
            upper_tick,
            tokens_provided,
            token_min_amount0,
            token_min_amount1,
        } => create_position(
            &env,
            &info,
            pool_id,
            lower_tick,
            upper_tick,
            tokens_provided,
            token_min_amount0,
            token_min_amount1,
        ),
    }
}
