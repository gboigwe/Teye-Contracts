use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env, String};

use crate::*;

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    assert!(client.is_initialized());
    assert_eq!(client.get_admin(), admin);
}

#[test]
fn test_register_user() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let user = Address::generate(&env);
    let name = String::from_str(&env, "Dr. Smith");

    client.register_user(&admin, &user, &Role::Optometrist, &name);

    let user_data = client.get_user(&user);
    assert_eq!(user_data.role, Role::Optometrist);
    assert!(user_data.is_active);
}

#[test]
fn test_add_and_get_record() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let patient = Address::generate(&env);
    let provider = Address::generate(&env);
    let data_hash = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");

    let record_id = client.add_record(
        &admin, // Use admin since they have SystemAdmin permission
        &patient,
        &provider,
        &RecordType::Examination,
        &data_hash,
    );

    assert_eq!(record_id, 1);

    // Admin bypasses consent check
    let record = client.get_record(&admin, &record_id);
    assert_eq!(record.patient, patient);
    assert_eq!(record.provider, provider);
}

#[test]
fn test_access_control() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let patient = Address::generate(&env);
    let doctor = Address::generate(&env);

    // Initially no access (no consent, no grant)
    assert_eq!(client.check_access(&patient, &doctor), AccessLevel::None);

    // Grant consent first, then access
    client.grant_consent(&patient, &doctor, &ConsentType::Treatment, &86400);
    client.grant_access(&patient, &patient, &doctor, &AccessLevel::Read, &86400);

    // Both consent + access grant active → Read
    assert_eq!(client.check_access(&patient, &doctor), AccessLevel::Read);

    // Revoke access
    client.revoke_access(&patient, &doctor);
    assert_eq!(client.check_access(&patient, &doctor), AccessLevel::None);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn test_invalid_register_user() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let user = Address::generate(&env);
    // Invalid name: too short
    let invalid_name = String::from_str(&env, "A");

    // ContractError::InvalidInput = 6
    client.register_user(&admin, &user, &Role::Optometrist, &invalid_name);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn test_invalid_add_record() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let patient = Address::generate(&env);
    let provider = Address::generate(&env);
    // Invalid hash: too short
    let invalid_data_hash = String::from_str(&env, "short_hash");

    client.add_record(
        &admin,
        &patient,
        &provider,
        &RecordType::Examination,
        &invalid_data_hash,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn test_invalid_grant_access() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let patient = Address::generate(&env);
    let doctor = Address::generate(&env);

    // Invalid duration: 0 seconds (too short)
    client.grant_access(&patient, &patient, &doctor, &AccessLevel::Read, &0);
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
