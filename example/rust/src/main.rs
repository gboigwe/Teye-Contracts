#![no_std]
use soroban_sdk::{contractimpl, Env, String, Address};

/*
 * Teye-Contracts: Rust Integration Example
 * Add to Cargo.toml: soroban-sdk = "20.0.0" 
 */

// Generate Client bindings from compiled WASM
soroban_sdk::contractimport!(
    file = "../../target/wasm32-unknown-unknown/release/teye_contract.wasm"
);

pub struct ThirdPartyApp;

#[contractimpl]
impl ThirdPartyApp {
    /// Example of a third-party contract interacting with Teye-Contracts
    pub fn interact_with_teye(env: Env, teye_contract_id: Address, user_data: String) -> String {
        let teye_client = Client::new(&env, &teye_contract_id);
        
        // Call the functions directly via the generated bindings
        let result = teye_client.create_record(&user_data);
        result
    }
}