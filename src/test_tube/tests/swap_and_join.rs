#[cfg(test)]
mod tests {
    use crate::msg::ExecuteMsg;
    use crate::test_tube::{TestEnv, UBAR, UFOO};
    use cosmwasm_std::{Coin, Uint128};
    use osmosis_std::types::cosmos::bank::v1beta1::QueryAllBalancesRequest;
    use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::{Pool, PoolsRequest};
    use osmosis_test_tube::OsmosisTestApp;
    use osmosis_test_tube::{Account, ConcentratedLiquidity, Module};
    use prost::Message;

    #[test]
    fn test_single_sided_swap_and_join_amt_0_in() {
        let app = OsmosisTestApp::new();
        let t = TestEnv::setup(&app);
        let cl = ConcentratedLiquidity::new(&app);

        let alice = app
            .init_account(&[
                Coin::new(1_000_000, UFOO),
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

        // Pool pre
        let pools = cl.query_pools(&PoolsRequest { pagination: None }).unwrap();
        let pool: Pool = Pool::decode(pools.pools[0].value.as_slice()).unwrap();

        println!("{:?}", pool);
        println!();

        // let resp = cl.query_liquidity_net_in_direction(&LiquidityNetInDirectionRequest {
        //     token_in: UFOO.to_string(),
        //     pool_id: 1,
        //     bound_tick: -1100,
        //     start_tick: 100,
        //     use_cur_tick: true,
        //     use_no_bound: true,
        // });

        // println!("{:?}", resp);
        // println!();
        // END: Debug lines for testing

        t.single_sided_lp_cl
            .execute(
                &ExecuteMsg::SingleSidedSwapAndJoin {
                    pool_id: 1,
                    lower_tick: -200,
                    upper_tick: 342000000,
                    // lower_tick: -108000000,
                    // upper_tick: 342000000,
                    token_provided: Coin::new(1_000_000, UFOO),
                    token_min_amount0: Uint128::zero(),
                    token_min_amount1: Uint128::zero(),
                },
                &[], // nil for the funds parameter
                &alice,
            )
            .unwrap();

        // let resp = cl.query_liquidity_net_in_direction(&LiquidityNetInDirectionRequest {
        //     token_in: UFOO.to_string(),
        //     pool_id: 1,
        //     bound_tick: -1100,
        //     start_tick: 100,
        //     use_cur_tick: true,
        //     use_no_bound: true,
        // });

        // println!("{:?}", resp);
        // println!();

        // Balance post
        let resp = t.bank.query_all_balances(&QueryAllBalancesRequest {
            address: alice.address(),
            pagination: None,
        });

        println!("{:?}", resp);
        println!();

        // Pools post
        let pools = cl.query_pools(&PoolsRequest { pagination: None }).unwrap();
        let pool: Pool = Pool::decode(pools.pools[0].value.as_slice()).unwrap();

        println!("{:?}", pool);
        println!();

        // // Query concentrated position that was just created
        // let resp = cl.query_user_positions(&UserPositionsRequest {
        //     address: alice.address(),
        //     pool_id: 1,
        //     pagination: None,
        // });

        // println!("{:?}", resp);
        // println!();
    }

    #[test]
    fn test_single_sided_swap_and_join_amt_1_in() {
        let app = OsmosisTestApp::new();
        let t = TestEnv::setup(&app);
        let cl = ConcentratedLiquidity::new(&app);

        let alice = app
            .init_account(&[
                Coin::new(1_000_000, UBAR),
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

        // Pools pre
        let pools = cl.query_pools(&PoolsRequest { pagination: None }).unwrap();
        let pool: Pool = Pool::decode(pools.pools[0].value.as_slice()).unwrap();

        println!("{:?}", pool);
        println!();

        // let resp = cl.query_liquidity_net_in_direction(&LiquidityNetInDirectionRequest {
        //     token_in: UBAR.to_string(),
        //     pool_id: 1,
        //     bound_tick: -1100,
        //     start_tick: 100,
        //     use_cur_tick: true,
        //     use_no_bound: true,
        // });

        // println!("{:?}", resp);
        // println!();
        // END: Debug lines for testing

        t.single_sided_lp_cl
            .execute(
                &ExecuteMsg::SingleSidedSwapAndJoin {
                    pool_id: 1,
                    lower_tick: -108000000,
                    upper_tick: 342000,
                    // lower_tick: -108000000,
                    // upper_tick: 342000000,
                    token_provided: Coin::new(1_000_000, UBAR),
                    token_min_amount0: Uint128::zero(),
                    token_min_amount1: Uint128::zero(),
                },
                &[], // nil for the funds parameter
                &alice,
            )
            .unwrap();

        // let resp = cl.query_liquidity_net_in_direction(&LiquidityNetInDirectionRequest {
        //     token_in: UBAR.to_string(),
        //     pool_id: 1,
        //     bound_tick: -1100,
        //     start_tick: 100,
        //     use_cur_tick: true,
        //     use_no_bound: true,
        // });

        // println!("{:?}", resp);
        // println!();

        // Balance post
        let resp = t.bank.query_all_balances(&QueryAllBalancesRequest {
            address: alice.address(),
            pagination: None,
        });

        println!("{:?}", resp);
        println!();

        // Pools post
        let pools = cl.query_pools(&PoolsRequest { pagination: None }).unwrap();
        let pool: Pool = Pool::decode(pools.pools[0].value.as_slice()).unwrap();

        println!("{:?}", pool);
        println!();

        // // Query concentrated position that was just created
        // let resp = cl.query_user_positions(&UserPositionsRequest {
        //     address: alice.address(),
        //     pool_id: 1,
        //     pagination: None,
        // });

        // println!("{:?}", resp);
        // println!();
    }

    #[test]
    fn test_swap_moves_cur_tick_below_lower_tick() {
        let app = OsmosisTestApp::new();
        let t = TestEnv::setup(&app);
        let cl = ConcentratedLiquidity::new(&app);

        let alice = app
            .init_account(&[
                Coin::new(50_000_000, UFOO),
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

        // Pool pre
        let pools = cl.query_pools(&PoolsRequest { pagination: None }).unwrap();
        let pool: Pool = Pool::decode(pools.pools[0].value.as_slice()).unwrap();

        println!("{:?}", pool);
        println!();

        t.single_sided_lp_cl
            .execute(
                &ExecuteMsg::SingleSidedSwapAndJoin {
                    pool_id: 1,
                    lower_tick: -100,
                    upper_tick: 100,
                    // lower_tick: -108000000,
                    // upper_tick: 342000000,
                    token_provided: Coin::new(50_000_000, UFOO),
                    token_min_amount0: Uint128::zero(),
                    token_min_amount1: Uint128::zero(),
                },
                &[], // nil for the funds parameter
                &alice,
            )
            .unwrap();

        // Balance post
        let resp = t.bank.query_all_balances(&QueryAllBalancesRequest {
            address: alice.address(),
            pagination: None,
        });

        println!("{:?}", resp);
        println!();

        // Pools post
        let pools = cl.query_pools(&PoolsRequest { pagination: None }).unwrap();
        let pool: Pool = Pool::decode(pools.pools[0].value.as_slice()).unwrap();

        println!("{:?}", pool);
        println!();
    }

    #[test]
    fn test_swap_moves_cur_tick_above_upper_tick() {
        let app = OsmosisTestApp::new();
        let t = TestEnv::setup(&app);
        let cl = ConcentratedLiquidity::new(&app);

        let alice = app
            .init_account(&[
                Coin::new(75_000_000, UBAR),
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

        // Pool pre
        let pools = cl.query_pools(&PoolsRequest { pagination: None }).unwrap();
        let pool: Pool = Pool::decode(pools.pools[0].value.as_slice()).unwrap();

        println!("{:?}", pool);
        println!();

        t.single_sided_lp_cl
            .execute(
                &ExecuteMsg::SingleSidedSwapAndJoin {
                    pool_id: 1,
                    lower_tick: -100,
                    upper_tick: 100,
                    // lower_tick: -108000000,
                    // upper_tick: 342000000,
                    token_provided: Coin::new(75_000_000, UBAR),
                    token_min_amount0: Uint128::zero(),
                    token_min_amount1: Uint128::zero(),
                },
                &[], // nil for the funds parameter
                &alice,
            )
            .unwrap();

        // Balance post
        let resp = t.bank.query_all_balances(&QueryAllBalancesRequest {
            address: alice.address(),
            pagination: None,
        });

        println!("{:?}", resp);
        println!();

        // Pools post
        let pools = cl.query_pools(&PoolsRequest { pagination: None }).unwrap();
        let pool: Pool = Pool::decode(pools.pools[0].value.as_slice()).unwrap();

        println!("{:?}", pool);
        println!();
    }
}
