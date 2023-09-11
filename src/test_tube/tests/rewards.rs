#[cfg(test)]
mod tests {
    use crate::msg::ExecuteMsg;
    use crate::tick::{MAX_TICK, MIN_INITIALIZED_TICK};
    use crate::{
        test_tube::{TestEnv, TestEnvBuilder},
        ContractError,
    };
    use cosmwasm_std::{Coin, Uint128};
    use osmosis_std::types::cosmos::authz::v1beta1::{MsgExec, MsgGrant};
    use osmosis_std::types::cosmos::base::v1beta1::Coin as OsmoCoin;
    use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::{
        CreateConcentratedLiquidityPoolsProposal, MsgCreatePosition, Pool, PoolRecord,
        PoolsRequest, UserPositionsRequest,
    };
    use osmosis_std::types::osmosis::poolmanager::v1beta1::{
        MsgSwapExactAmountIn, SwapAmountInRoute,
    };
    use osmosis_test_tube::{Account, Bank, ConcentratedLiquidity, Module, PoolManager, Wasm};
    use osmosis_test_tube::{OsmosisTestApp, RunnerError};

    #[test]
    #[ignore]
    fn test_rewards_single_distribute_claim() {
        let app = OsmosisTestApp::new();
        let t = TestEnv::setup(&app);

        let alice = app
            .init_account(&[
                Coin::new(1_000_000_000_000, "uatom"),
                Coin::new(1_000_000_000_000, "uosmo"),
            ])
            .unwrap();

        // let bob = app
        //     .init_account(&[
        //         Coin::new(1_000_000_000_000, "uatom"),
        //         Coin::new(1_000_000_000_000, "uosmo"),
        //     ])
        //     .unwrap();

        // Have alice give the contract authz permissions
        t.authz
            .grant_required_authz_for_lp(&alice, t.single_sided_lp_cl.contract_addr.as_str())
            .unwrap();

        t.single_sided_lp_cl
            .execute(
                &ExecuteMsg::SingleSidedSwapAndJoin {
                    pool_id: 1,
                    lower_tick: MIN_INITIALIZED_TICK,
                    upper_tick: MAX_TICK as i64,
                    token_provided: Coin::new(1_000_000, "uatom"),
                    token_min_amount0: Uint128::new(1),
                    token_min_amount1: Uint128::new(1),
                },
                &[], // nil for the funds parameter
                &alice,
            )
            .unwrap();

        let cl = ConcentratedLiquidity::new(&app);

        // Query concentrated position that was just created
        let resp = cl.query_user_positions(&UserPositionsRequest {
            address: alice.address(),
            pool_id: 1,
            pagination: None,
        });

        println!("{:?}", resp);

        // let res = wasm
        //     .execute(
        //         contract_address.as_str(),
        //         &ExecuteMsg::VaultExtension(crate::msg::ExtensionExecuteMsg::DistributeRewards {}),
        //         &[],
        //         &alice,
        //     )
        //     .unwrap();

        // println!("{:?}", res.events)
    }
}
