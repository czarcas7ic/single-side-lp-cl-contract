use std::path::PathBuf;

use crate::msg::{ExecuteMsg, InstantiateMsg};
use cosmwasm_std::Coin;
use osmosis_std::types::cosmwasm::wasm::v1::MsgExecuteContractResponse;
use osmosis_test_tube::{
    Account, Module, OsmosisTestApp, RunnerError, RunnerExecuteResult, SigningAccount, Wasm,
};

pub struct SingleSidedLpCl<'a> {
    app: &'a OsmosisTestApp,
    pub code_id: u64,
    pub contract_addr: String,
}

impl<'a> SingleSidedLpCl<'a> {
    pub fn deploy(app: &'a OsmosisTestApp, signer: &SigningAccount) -> Result<Self, RunnerError> {
        let wasm = Wasm::new(app);

        let code_id = wasm
            .store_code(&get_wasm_byte_code(), None, signer)?
            .data
            .code_id;
        let contract_addr = wasm
            .instantiate(
                code_id,
                &InstantiateMsg {
                    owner: signer.address(),
                },
                None,
                None,
                &[],
                signer,
            )?
            .data
            .address;

        Ok(Self {
            app,
            code_id,
            contract_addr,
        })
    }

    pub fn execute(
        &self,
        msg: &ExecuteMsg,
        funds: &[Coin],
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let wasm = Wasm::new(self.app);
        wasm.execute(&self.contract_addr, msg, funds, signer)
    }
}

fn get_wasm_byte_code() -> Vec<u8> {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::fs::read(
        manifest_path
            .join(".")
            .join("target")
            .join("wasm32-unknown-unknown")
            .join("release")
            .join("single_sided_lp_cl.wasm"),
    )
    .unwrap()
}
