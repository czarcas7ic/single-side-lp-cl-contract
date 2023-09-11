use osmosis_std::{
    shim::{Any, Timestamp},
    types::{
        cosmos::authz::v1beta1::{GenericAuthorization, Grant, MsgGrant, MsgGrantResponse},
        osmosis::{
            concentratedliquidity::v1beta1::MsgCreatePosition,
            poolmanager::v1beta1::MsgSwapExactAmountIn,
        },
    },
};
use osmosis_test_tube::{
    Account, Module, OsmosisTestApp, Runner, RunnerError, RunnerExecuteResult, SigningAccount,
};
use prost::Message;

pub struct Authz<'a, R: Runner<'a>> {
    runner: &'a R,
}
impl<'a, R: Runner<'a>> Module<'a, R> for Authz<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl Authz<'_, OsmosisTestApp> {
    pub fn grant_generic_authz(
        &self,
        granter: &SigningAccount,
        grantee: &str,
        type_url: &str,
    ) -> RunnerExecuteResult<MsgGrantResponse> {
        self.runner.execute::<_, MsgGrantResponse>(
            MsgGrant {
                granter: granter.address(),
                grantee: grantee.to_string(),
                grant: Some(Grant {
                    authorization: Some(Any {
                        type_url: GenericAuthorization::TYPE_URL.to_string(),
                        value: GenericAuthorization {
                            msg: type_url.to_string(),
                        }
                        .encode_to_vec(),
                    }),
                    expiration: Some(Timestamp {
                        seconds: 9999999999,
                        nanos: 0,
                    }),
                }),
            },
            MsgGrant::TYPE_URL,
            granter,
        )
    }

    pub fn grant_required_authz_for_lp(
        &self,
        user: &SigningAccount,
        contract_addr: &str,
    ) -> Result<(), RunnerError> {
        self.grant_generic_authz(user, contract_addr, MsgSwapExactAmountIn::TYPE_URL)?;
        self.grant_generic_authz(user, contract_addr, MsgCreatePosition::TYPE_URL)?;

        Ok(())
    }
}
