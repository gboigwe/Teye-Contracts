#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects
)]

use super::*;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::{symbol_short, Env, IntoVal, TryIntoVal};

#[test]
fn test_initialize() {
    let env = Env::default();
    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);
    let events = env.events().all();

    assert!(client.is_initialized());
    assert_eq!(client.get_admin(), admin);
    let our_events: soroban_sdk::Vec<(
        soroban_sdk::Address,
        soroban_sdk::Vec<soroban_sdk::Val>,
        soroban_sdk::Val,
    )> = events;

    assert!(!our_events.is_empty());
    let event = our_events.get(our_events.len() - 1).unwrap();
    assert_eq!(event.1, (symbol_short!("INIT"),).into_val(&env));
    let payload: events::InitializedEvent = event.2.try_into_val(&env).unwrap();
    assert_eq!(payload.admin, admin);
}

#[test]
fn test_rate_limit_add_record_and_grant_access() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Configure a small window for testing
    client.set_rate_limit_config(&admin, &2, &60);

    let patient = Address::generate(&env);
    let provider = Address::generate(&env);
    let data_hash = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");

    // First two record additions should succeed
    client.add_record(
        &admin,
        &patient,
        &provider,
        &RecordType::Examination,
        &data_hash,
    );
    client.add_record(
        &admin,
        &patient,
        &provider,
        &RecordType::Examination,
        &data_hash,
    );

    // Third should be rate limited
    let res = client.try_add_record(
        &admin,
        &patient,
        &provider,
        &RecordType::Examination,
        &data_hash,
    );
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(matches!(err, Ok(ContractError::RateLimitExceeded)));

    // Advance time beyond the window and ensure the limit resets
    use soroban_sdk::testutils::Ledger;
    let current = env.ledger().timestamp();
    env.ledger().set_timestamp(current + 61);

    let res_after_reset = client.try_add_record(
        &admin,
        &patient,
        &provider,
        &RecordType::Examination,
        &data_hash,
    );
    assert!(res_after_reset.is_ok());

    // Grant access calls should also consume the same per-address budget
    let doctor = Address::generate(&env);
    client.grant_access(&patient, &patient, &doctor, &AccessLevel::Read, &86400);
    client.grant_access(&patient, &patient, &doctor, &AccessLevel::Read, &86400);
    let rate_limited =
        client.try_grant_access(&patient, &patient, &doctor, &AccessLevel::Read, &86400);
    assert!(rate_limited.is_err());
}

#[test]
fn test_permission_without_consent_denied() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let patient = Address::generate(&env);
    let doctor = Address::generate(&env);

    // Grant access but NOT consent
    client.grant_access(&patient, &patient, &doctor, &AccessLevel::Read, &86400);

    // Access denied — no consent
    assert_eq!(client.check_access(&patient, &doctor), AccessLevel::None);
}

#[test]
fn test_consent_and_permission_grants_access() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let patient = Address::generate(&env);
    let doctor = Address::generate(&env);

    // Grant both consent and access
    client.grant_consent(&patient, &doctor, &ConsentType::Treatment, &86400);
    client.grant_access(&patient, &patient, &doctor, &AccessLevel::Read, &86400);

    assert_eq!(client.check_access(&patient, &doctor), AccessLevel::Read);
}

#[test]
fn test_revoked_consent_blocks_access() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let patient = Address::generate(&env);
    let doctor = Address::generate(&env);

    client.grant_consent(&patient, &doctor, &ConsentType::Sharing, &86400);
    client.grant_access(&patient, &patient, &doctor, &AccessLevel::Read, &86400);
    assert_eq!(client.check_access(&patient, &doctor), AccessLevel::Read);

    // Revoke consent
    client.revoke_consent(&patient, &doctor);

    // Access now denied despite active access grant
    assert_eq!(client.check_access(&patient, &doctor), AccessLevel::None);
}

#[test]
fn test_expired_consent_blocks_access() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let patient = Address::generate(&env);
    let doctor = Address::generate(&env);

    // Grant short-lived consent and long-lived access
    client.grant_consent(&patient, &doctor, &ConsentType::Research, &100);
    client.grant_access(&patient, &patient, &doctor, &AccessLevel::Read, &86400);

    assert_eq!(client.check_access(&patient, &doctor), AccessLevel::Read);

    // Advance time past consent expiry
    env.ledger().set_timestamp(200);

    // Consent expired — access denied
    assert_eq!(client.check_access(&patient, &doctor), AccessLevel::None);
}

#[test]
fn test_get_record_consent_required() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let patient = Address::generate(&env);
    let provider = Address::generate(&env);
    let doctor = Address::generate(&env);
    let data_hash = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");

    let record_id = client.add_record(
        &admin,
        &patient,
        &provider,
        &RecordType::Examination,
        &data_hash,
    );

    // Patient can always view own record
    let record = client.get_record(&patient, &record_id);
    assert_eq!(record.patient, patient);

    // Doctor without consent → error (ConsentRequired = 26)
    let result = client.try_get_record(&doctor, &record_id);
    assert!(result.is_err());

    // Grant consent → doctor can view
    client.grant_consent(&patient, &doctor, &ConsentType::Treatment, &86400);
    let record = client.get_record(&doctor, &record_id);
    assert_eq!(record.patient, patient);
}
