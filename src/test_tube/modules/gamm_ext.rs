use osmosis_test_tube::{
    fn_execute,
    osmosis_std::types::{
        cosmos::base::v1beta1::Coin,
        osmosis::gamm::{
            poolmodels::balancer::v1beta1::{MsgCreateBalancerPool, MsgCreateBalancerPoolResponse},
            poolmodels::stableswap::{
                self,
                v1beta1::{MsgCreateStableswapPool, MsgCreateStableswapPoolResponse},
            },
            v1beta1::{
                MsgJoinPool, MsgJoinPoolResponse, PoolAsset, PoolParams,
                QueryCalcJoinPoolNoSwapSharesRequest, QueryCalcJoinPoolNoSwapSharesResponse,
            },
        },
    },
    Account, Gamm, Module, Runner, RunnerExecuteResult, SigningAccount,
};

pub struct GammExt<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for GammExt<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl<'a, R> GammExt<'a, R>
where
    R: Runner<'a>,
{
    fn_execute! {
        pub create_stableswap_pool: MsgCreateStableswapPool => MsgCreateStableswapPoolResponse
    }

    pub fn calc_and_join_pool_no_swap(
        &self,
        pool_id: u64,
        tokens_in: Vec<Coin>,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgJoinPoolResponse> {
        let QueryCalcJoinPoolNoSwapSharesResponse {
            tokens_out: _,
            shares_out,
        } = self
            .runner
            .query::<QueryCalcJoinPoolNoSwapSharesRequest, _>(
                "/osmosis.gamm.v1beta1.Query/CalcJoinPoolNoSwapShares",
                &QueryCalcJoinPoolNoSwapSharesRequest {
                    pool_id,
                    tokens_in: tokens_in.clone(),
                },
            )?;

        self.runner.execute(
            MsgJoinPool {
                sender: signer.address(),
                pool_id,
                share_out_amount: shares_out,
                token_in_maxs: tokens_in,
            },
            MsgJoinPool::TYPE_URL,
            signer,
        )
    }

    pub fn create_basic_balancer_pool(
        &self,
        initial_liquidity: &[cosmwasm_std::Coin],
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgCreateBalancerPoolResponse> {
        let gamm = Gamm::new(self.runner);
        gamm.create_balancer_pool(
            MsgCreateBalancerPool {
                sender: signer.address(),
                pool_params: Some(PoolParams {
                    swap_fee: "10000000000000000".to_string(),
                    exit_fee: "10000000000000000".to_string(),
                    smooth_weight_change_params: None,
                }),
                pool_assets: initial_liquidity
                    .iter()
                    .map(|c| PoolAsset {
                        token: Some(Coin {
                            denom: c.denom.to_owned(),
                            amount: c.amount.to_string(),
                        }),
                        weight: "1000000".to_string(),
                    })
                    .collect(),
                future_pool_governor: "".to_string(),
            },
            signer,
        )
    }

    pub fn create_basic_stableswap_pool(
        &self,
        initial_liquidity: &[cosmwasm_std::Coin],
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgCreateStableswapPoolResponse> {
        self.create_stableswap_pool(
            MsgCreateStableswapPool {
                sender: signer.address(),
                pool_params: Some(stableswap::v1beta1::PoolParams {
                    swap_fee: "10000000000000000".to_string(),
                    exit_fee: "10000000000000000".to_string(),
                }),
                future_pool_governor: "".to_string(),
                initial_pool_liquidity: initial_liquidity
                    .iter()
                    .map(|c| Coin {
                        denom: c.denom.to_owned(),
                        amount: c.amount.to_string(),
                    })
                    .collect(),
                scaling_factors: initial_liquidity.iter().map(|_| 1).collect(),
                scaling_factor_controller: "".to_string(),
            },
            signer,
        )
    }
}
