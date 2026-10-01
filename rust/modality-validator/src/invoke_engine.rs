use anyhow::{anyhow, Result};
use modality_common::contract_store::CommitAction;
use modality_common::independent_replay::{
    FrozenInvokeContext, InvokeEngine, ReplayWasm, FROZEN_INVOKE_TIMESTAMP,
};
use modality_wasm_runtime::WasmExecutor;
use modality_wasm_validation::{
    decode_program_result, encode_program_input, validate_program_result, ProgramContext,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub struct WasmInvokeEngine {
    gas_limit: u64,
    fuel_limit: Option<u64>,
    /// Fuel the last program used, measured by the runtime.
    pub last_gas_used: u64,
    pub last_program_path: Option<String>,
    pub last_actions_count: usize,
}

impl WasmInvokeEngine {
    pub fn new(gas_limit: u64) -> Self {
        Self {
            gas_limit,
            fuel_limit: None,
            last_gas_used: 0,
            last_program_path: None,
            last_actions_count: 0,
        }
    }
}

pub fn program_context_from_frozen(ctx: &FrozenInvokeContext) -> ProgramContext {
    ProgramContext {
        contract_id: ctx.contract_id.clone(),
        block_height: ctx.block_height,
        timestamp: FROZEN_INVOKE_TIMESTAMP,
        invoker: ctx.invoker.clone(),
        commit_id: ctx.commit_id.clone(),
        parent_commit_id: ctx.parent_commit_id.clone(),
        state: Value::Object(ctx.state.clone()),
        accepted_state_oracle_keys: ctx.accepted_state_oracle_keys.clone(),
    }
}

pub fn execute_wasm_program(
    wasm_bytes: &[u8],
    gas_limit: u64,
    args: Value,
    context: ProgramContext,
) -> Result<modality_wasm_validation::ProgramResult> {
    execute_wasm_program_metered(wasm_bytes, gas_limit, args, context).0
}

/// [`execute_wasm_program`], with the fuel it used whether or not it
/// finished.
pub fn execute_wasm_program_metered(
    wasm_bytes: &[u8],
    gas_limit: u64,
    args: Value,
    context: ProgramContext,
) -> (Result<modality_wasm_validation::ProgramResult>, u64) {
    let input_json = match encode_program_input(args, context) {
        Ok(input) => input,
        Err(err) => return (Err(err), 0),
    };
    let mut executor = WasmExecutor::new(gas_limit);
    let run = executor.execute(wasm_bytes, "execute", &input_json);
    let fuel = executor.fuel_used();
    let result = run
        .map_err(|err| anyhow!("Program execution failed: {err}"))
        .and_then(|json| decode_program_result(&json))
        .and_then(|result| {
            validate_program_result(&result)?;
            Ok(result)
        });
    (result, fuel)
}

impl InvokeEngine for WasmInvokeEngine {
    fn set_fuel_limit(&mut self, limit: u64) {
        self.fuel_limit = Some(limit);
    }

    fn last_fuel(&self) -> u64 {
        self.last_gas_used
    }

    fn execute_invoke(
        &mut self,
        wasm: &ReplayWasm,
        args: &Value,
        ctx: &FrozenInvokeContext,
    ) -> Result<Vec<CommitAction>> {
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &wasm.bytes_b64)
                .map_err(|err| anyhow!("invalid WASM bytes for {}: {err}", wasm.path))?;
        let sha256 = hex::encode(Sha256::digest(&bytes));
        if sha256 != wasm.sha256 {
            anyhow::bail!(
                "WASM hash mismatch for {}: artifact {}, computed {}",
                wasm.path,
                wasm.sha256,
                sha256
            );
        }
        let gas_limit = self.fuel_limit.take().unwrap_or(wasm.gas_limit.min(self.gas_limit));
        self.last_gas_used = 0;
        let (result, fuel) = execute_wasm_program_metered(
            &bytes,
            gas_limit,
            args.clone(),
            program_context_from_frozen(ctx),
        );
        self.last_gas_used = fuel;
        let result = result?;
        if !result.is_success() {
            anyhow::bail!("Program execution failed: {:?}", result.errors);
        }
        self.last_program_path = Some(wasm.path.clone());
        self.last_actions_count = result.actions.len();
        Ok(result
            .actions
            .into_iter()
            .map(|action| CommitAction {
                method: action.method,
                path: action.path,
                value: action.value,
                source_contract: None,
                source_path: None,
                source_commit: None,
                emitted_by: None,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Map;
    use std::collections::BTreeMap;

    #[test]
    fn program_context_preserves_replayed_oracle_key_map() {
        let mut accepted_state_oracle_keys = BTreeMap::new();
        accepted_state_oracle_keys.insert(
            "/oracles/delivery.id".to_string(),
            "delivery_oracle_key".to_string(),
        );
        let ctx = FrozenInvokeContext {
            contract_id: "contract-1".to_string(),
            commit_id: "pending".to_string(),
            parent_commit_id: Some("accepted".to_string()),
            block_height: 3,
            timestamp: FROZEN_INVOKE_TIMESTAMP,
            invoker: "alice_key".to_string(),
            state: Map::new(),
            accepted_state_oracle_keys,
        };

        let program_context = program_context_from_frozen(&ctx);

        assert_eq!(
            program_context
                .accepted_state_oracle_keys
                .get("/oracles/delivery.id")
                .map(String::as_str),
            Some("delivery_oracle_key")
        );
        assert_eq!(program_context.commit_id, "pending");
        assert_eq!(
            program_context.parent_commit_id.as_deref(),
            Some("accepted")
        );
    }
}
