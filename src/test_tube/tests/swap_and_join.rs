#[cfg(test)]
mod tests {
    use crate::msg::ExecuteMsg;
    use crate::test_tube::TestEnv;
    use crate::tick::{MAX_TICK, MIN_INITIALIZED_TICK};
    use cosmwasm_std::{Coin, Uint128};
    use osmosis_std::types::cosmos::bank::v1beta1::QueryAllBalancesRequest;
    use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::UserPositionsRequest;
    use osmosis_test_tube::OsmosisTestApp;
    use osmosis_test_tube::{Account, ConcentratedLiquidity, Module};

    #[test]
    #[ignore]
    fn test_single_sided_swap_and_join() {
        let app = OsmosisTestApp::new();
        let t = TestEnv::setup(&app);

        let alice = app
            .init_account(&[
                Coin::new(1_000_000_000_000, "uatom"),
                Coin::new(1_000_000_000_000, "uosmo"),
            ])
            .unwrap();

        // Have alice give the contract authz permissions
        t.authz
            .grant_required_authz_for_lp(&alice, t.single_sided_lp_cl.contract_addr.as_str())
            .unwrap();

        // Balance pre
        let resp = t.bank.query_all_balances(&QueryAllBalancesRequest {
            address: alice.address(),
            pagination: None,
        });

        println!("{:?}", resp);
        println!();

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

        // Balance post
        let resp = t.bank.query_all_balances(&QueryAllBalancesRequest {
            address: alice.address(),
            pagination: None,
        });

        println!("{:?}", resp);
        println!();

        // Query concentrated position that was just created
        let resp = cl.query_user_positions(&UserPositionsRequest {
            address: alice.address(),
            pool_id: 1,
            pagination: None,
        });

        println!("{:?}", resp);
        println!();

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
