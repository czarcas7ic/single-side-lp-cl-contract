mod modules;
mod tests;

use std::collections::HashMap;

use crate::test_tube::modules::{Authz, GammExt, Lockup, SingleSidedLpCl};

use cosmwasm_std::{Coin, Uint128};
use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::{
    CreateConcentratedLiquidityPoolsProposal, MsgCreatePosition, Pool, PoolRecord, PoolsRequest,
    UserPositionsRequest,
};
use prost::Message;

use crate::tick::{MAX_TICK, MIN_INITIALIZED_TICK};
use osmosis_std::types::osmosis::{lockup, poolmanager::v1beta1::PoolType};
use osmosis_test_tube::{cosmrs, osmosis_std::types::osmosis::lockup::AccountLockedCoinsRequest};
use osmosis_test_tube::{
    osmosis_std::types::cosmos::bank::v1beta1::QueryAllBalancesRequest,
    osmosis_std::{
        shim::Duration,
        types::osmosis::{
            gamm::v1beta1::MsgJoinPoolResponse,
            lockup::{AccountLockedCoinsResponse, MsgLockTokens},
        },
    },
    Account, Bank, ConcentratedLiquidity, GovWithAppAccess, Module, OsmosisTestApp, SigningAccount,
};

struct TestEnvBuilder<'a> {
    app: &'a OsmosisTestApp,
    pools: HashMap<String, (PoolType, Vec<Coin>)>,
    user_locked_lp: Vec<(String, Vec<Coin>)>,
    additional_user_balances: Vec<Coin>,
}

impl<'a> TestEnvBuilder<'a> {
    fn new(app: &'a OsmosisTestApp) -> Self {
        Self {
            app,
            pools: HashMap::new(),
            user_locked_lp: vec![],
            additional_user_balances: vec![],
        }
    }
    fn new_with_defaults(app: &'a OsmosisTestApp) -> Self {
        Self {
            app,
            pools: HashMap::from([
                (
                    "osmo/usdt".to_string(),
                    (
                        PoolType::Balancer,
                        vec![
                            Coin::new(100_000_000_000, "uosmo".to_string()),
                            Coin::new(100_000_000_000, "uusdt".to_string()),
                        ],
                    ),
                ),
                (
                    "osmo/usdc".to_string(),
                    (
                        PoolType::Balancer,
                        vec![
                            Coin::new(100_000_000_000, "uosmo".to_string()),
                            Coin::new(100_000_000_000, "uusdc".to_string()),
                        ],
                    ),
                ),
                (
                    "usdc/usdt".to_string(),
                    (
                        PoolType::Stableswap,
                        vec![
                            Coin::new(100_000_000_000, "uusdc".to_string()),
                            Coin::new(100_000_000_000, "uusdt".to_string()),
                        ],
                    ),
                ),
            ]),
            user_locked_lp: vec![(
                "osmo/usdc".to_string(),
                vec![
                    Coin::new(100_000_000, "uosmo".to_string()),
                    Coin::new(100_000_000, "uusdc".to_string()),
                ],
            )],
            additional_user_balances: vec![],
        }
    }

    fn with_pool(mut self, pool_type: PoolType, pool_name: &str, coins: Vec<Coin>) -> Self {
        let mut coins = coins;
        coins.sort_by(|a, b| a.denom.cmp(&b.denom));
        self.pools.insert(pool_name.to_string(), (pool_type, coins));
        self
    }

    fn with_user_locked_lp(mut self, pool_name: &str, coins: Vec<Coin>) -> Self {
        self.user_locked_lp.push((pool_name.to_string(), coins));
        self
    }

    fn with_addtional_user_balances(mut self, coins: Vec<Coin>) -> Self {
        self.additional_user_balances = coins;
        self
    }

    fn build(self) -> TestEnv<'a> {
        let app = self.app;
        // # Setup
        let authz = Authz::new(app);
        let gamm_ext = GammExt::new(app);
        let bank = Bank::new(app);
        let lockup = Lockup::new(app);
        let cl = ConcentratedLiquidity::new(app);
        let gov = GovWithAppAccess::new(app);

        let user_balances = sum_duplicated_coins(
            self.user_locked_lp
                .iter()
                .flat_map(|(_, coins)| coins.clone())
                .chain(self.additional_user_balances)
                .collect(),
        );

        let user = app.init_account(&user_balances).unwrap();
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
                    tick_spacing: 1,
                    spread_factor: "0".to_string(),
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
            lockup,
            user,
            single_sided_lp_cl: singe_sided_lp_cl,
        }
    }
}

fn sum_duplicated_coins(coins: Vec<Coin>) -> Vec<Coin> {
    coins
        .into_iter()
        .fold(HashMap::new(), |mut acc, coin| {
            let coin_entry = *acc.entry(coin.denom.clone()).or_insert(0);
            acc.insert(coin.denom, coin_entry + coin.amount.u128());
            acc
        })
        .into_iter()
        .map(|(denom, amount)| Coin::new(amount, denom))
        .collect::<Vec<_>>()
}

struct TestEnv<'a> {
    pub authz: Authz<'a, OsmosisTestApp>,
    pub bank: Bank<'a, OsmosisTestApp>,
    pub lockup: Lockup<'a, OsmosisTestApp>,
    pub user: SigningAccount,
    pub single_sided_lp_cl: SingleSidedLpCl<'a>,
}

impl<'a> TestEnv<'a> {
    fn setup(app: &'a OsmosisTestApp) -> TestEnv<'a> {
        TestEnvBuilder::new_with_defaults(app).build()
    }

    fn assert_user_balances(&self, expected_balances: Vec<Coin>) {
        let user_balances: Vec<Coin> = self
            .bank
            .query_all_balances(&QueryAllBalancesRequest {
                address: self.user.address(),
                pagination: None,
            })
            .unwrap()
            .balances
            .into_iter()
            .map(|coin| Coin::new(coin.amount.parse().unwrap(), coin.denom))
            .collect();

        assert_eq!(user_balances, expected_balances);
    }

    fn assert_empty_contract_balance(&self) {
        let contract_balances: Vec<Coin> = self
            .bank
            .query_all_balances(&QueryAllBalancesRequest {
                address: self.single_sided_lp_cl.contract_addr.to_string(),
                pagination: None,
            })
            .unwrap()
            .balances
            .into_iter()
            .map(|coin| Coin::new(coin.amount.parse().unwrap(), coin.denom))
            .collect();

        assert_eq!(contract_balances, vec![]);
    }
}
