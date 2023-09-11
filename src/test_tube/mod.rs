mod modules;
mod tests;

use crate::test_tube::modules::{Authz, SingleSidedLpCl};

use cosmwasm_std::{Coin, Uint128};
use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::{
    CreateConcentratedLiquidityPoolsProposal, MsgCreatePosition, Pool, PoolRecord, PoolsRequest,
};
use prost::Message;

use crate::tick::{MAX_TICK, MIN_INITIALIZED_TICK};
use osmosis_test_tube::{
    Account, Bank, ConcentratedLiquidity, GovWithAppAccess, Module, OsmosisTestApp,
};

struct TestEnvBuilder<'a> {
    app: &'a OsmosisTestApp,
}

impl<'a> TestEnvBuilder<'a> {
    fn new_with_defaults(app: &'a OsmosisTestApp) -> Self {
        Self { app }
    }

    fn build(self) -> TestEnv<'a> {
        let app = self.app;
        // # Setup
        let authz = Authz::new(app);
        let bank = Bank::new(app);
        let cl = ConcentratedLiquidity::new(app);
        let gov = GovWithAppAccess::new(app);

        let admin = app
            .init_account(&[
                Coin::new(100_000_000_000_000, "uosmo"),
                Coin::new(100_000_000_000_000, "uatom"),
            ])
            .unwrap();

        // 2. deploy singe_sided_lp_cl contract
        let singe_sided_lp_cl = SingleSidedLpCl::deploy(app, &admin).unwrap();

        // // set lockup params
        // app.set_param_set(
        //     "lockup",
        //     cosmrs::Any {
        //         type_url: lockup::Params::TYPE_URL.to_string(),
        //         value: lockup::Params {
        //             force_unlock_allowed_addresses: vec![],
        //         }
        //         .encode_to_vec(),
        //     },
        // )
        // .unwrap();

        // // 3. create gamm pools
        // let pool_creation_fee = Coin::new(1_000_000_000, "uosmo".to_string());
        // let pools: HashMap<String, u64> = self
        //     .pools
        //     .into_iter()
        //     .map(|(pool_name, (pool_type, coins))| {
        //         let lper_balance = sum_duplicated_coins(
        //             vec![
        //                 // add osmo to the pool to make sure the pool has enough osmo to pay for fees
        //                 vec![pool_creation_fee.clone()],
        //                 coins.clone(),
        //             ]
        //             .concat(),
        //         );

        //         let lper = app.init_account(&lper_balance).unwrap();
        //         (
        //             pool_name,
        //             match pool_type {
        //                 PoolType::Balancer => {
        //                     gamm_ext
        //                         .create_basic_balancer_pool(&coins, &lper)
        //                         .unwrap()
        //                         .data
        //                         .pool_id
        //                 }
        //                 PoolType::Stableswap => {
        //                     gamm_ext
        //                         .create_basic_stableswap_pool(&coins, &lper)
        //                         .unwrap()
        //                         .data
        //                         .pool_id
        //                 }
        //                 PoolType::Concentrated => unimplemented!(),
        //                 PoolType::CosmWasm => unimplemented!(),
        //             },
        //         )
        //     })
        //     .collect();

        gov.propose_and_execute(
            CreateConcentratedLiquidityPoolsProposal::TYPE_URL.to_string(),
            CreateConcentratedLiquidityPoolsProposal {
                title: "Create concentrated uosmo:usdc pool".to_string(),
                description: "Create concentrated uosmo:usdc pool, so that we can trade it"
                    .to_string(),
                pool_records: vec![PoolRecord {
                    denom0: "uatom".to_string(),
                    denom1: "uosmo".to_string(),
                    tick_spacing: 100,
                    spread_factor: "100000000000000".to_string(),
                }],
            },
            admin.address(),
            false,
            &admin,
        )
        .unwrap();

        let pools = cl.query_pools(&PoolsRequest { pagination: None }).unwrap();
        let pool = Pool::decode(pools.pools[0].value.as_slice()).unwrap();

        // create a basic position on the pool
        let initial_position = MsgCreatePosition {
            pool_id: pool.id,
            sender: admin.address(),
            lower_tick: MIN_INITIALIZED_TICK,
            upper_tick: MAX_TICK as i64,
            tokens_provided: vec![
                cosmwasm_std::Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::from(10000000000u128),
                }
                .into(),
                cosmwasm_std::Coin {
                    denom: "uosmo".to_string(),
                    amount: Uint128::from(10000000000u128),
                }
                .into(),
            ],
            token_min_amount0: "1".to_string(),
            token_min_amount1: "1".to_string(),
        };
        let _position = cl.create_position(initial_position, &admin).unwrap();

        TestEnv {
            authz,
            bank,
            single_sided_lp_cl: singe_sided_lp_cl,
        }
    }
}

struct TestEnv<'a> {
    pub authz: Authz<'a, OsmosisTestApp>,
    pub bank: Bank<'a, OsmosisTestApp>,
    pub single_sided_lp_cl: SingleSidedLpCl<'a>,
}

impl<'a> TestEnv<'a> {
    fn setup(app: &'a OsmosisTestApp) -> TestEnv<'a> {
        TestEnvBuilder::new_with_defaults(app).build()
    }
}
