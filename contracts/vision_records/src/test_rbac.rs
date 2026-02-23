use super::{Permission, Role, VisionRecordsContract, VisionRecordsContractClient};
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env, String};

fn setup_test() -> (Env, VisionRecordsContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VisionRecordsContract, ());
    let client = VisionRecordsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    (env, client, admin)
}

#[test]
fn test_role_hierarchy_and_inheritance() {
    let (env, client, admin) = setup_test();

    let optometrist = Address::generate(&env);
    client.register_user(
        &admin,
        &optometrist,
        &Role::Optometrist,
        &String::from_str(&env, "Opto"),
    );

    let staff = Address::generate(&env);
    client.register_user(
        &admin,
        &staff,
        &Role::Staff,
        &String::from_str(&env, "Staff"),
    );

    let patient = Address::generate(&env);
    client.register_user(
        &admin,
        &patient,
        &Role::Patient,
        &String::from_str(&env, "Pat"),
    );

    // Admin should have all permissions implicitly
    assert!(client.check_permission(&admin, &Permission::SystemAdmin));
    assert!(client.check_permission(&admin, &Permission::ManageUsers));
    assert!(client.check_permission(&admin, &Permission::WriteRecord));

    // Optometrist should have read/write/access/users but NOT SystemAdmin
    assert!(!client.check_permission(&optometrist, &Permission::SystemAdmin));
    assert!(client.check_permission(&optometrist, &Permission::WriteRecord));
    assert!(client.check_permission(&optometrist, &Permission::ManageUsers));

    // Staff should have ManageUsers but NOT WriteRecord
    assert!(client.check_permission(&staff, &Permission::ManageUsers));
    assert!(!client.check_permission(&staff, &Permission::WriteRecord));

    // Patient has no implicit system permissions
    assert!(!client.check_permission(&patient, &Permission::ManageUsers));
    assert!(!client.check_permission(&patient, &Permission::WriteRecord));
}

#[test]
fn test_custom_permission_grants() {
    let (env, client, admin) = setup_test();

    let staff = Address::generate(&env);
    client.register_user(
        &admin,
        &staff,
        &Role::Staff,
        &String::from_str(&env, "Staff"),
    );

    // Staff originally cannot write records
    assert!(!client.check_permission(&staff, &Permission::WriteRecord));

    // Admin grants WriteRecord to staff
    client.grant_custom_permission(&admin, &staff, &Permission::WriteRecord);

    // Staff can now write records
    assert!(client.check_permission(&staff, &Permission::WriteRecord));

    // Admin revokes WriteRecord
    client.revoke_custom_permission(&admin, &staff, &Permission::WriteRecord);

    // Staff again cannot write records
    assert!(!client.check_permission(&staff, &Permission::WriteRecord));
}

#[test]
fn test_custom_permission_revocations() {
    let (env, client, admin) = setup_test();

    let optometrist = Address::generate(&env);
    client.register_user(
        &admin,
        &optometrist,
        &Role::Optometrist,
        &String::from_str(&env, "Opto"),
    );

    // Optometrist initially has ManageUsers
    assert!(client.check_permission(&optometrist, &Permission::ManageUsers));

    // Admin explicitly revokes ManageUsers from this specific Optometrist
    client.revoke_custom_permission(&admin, &optometrist, &Permission::ManageUsers);

    // They no longer have it, even though their base role does
    assert!(!client.check_permission(&optometrist, &Permission::ManageUsers));

    // But they still have WriteRecord
    assert!(client.check_permission(&optometrist, &Permission::WriteRecord));

    // Admin grants it back
    client.grant_custom_permission(&admin, &optometrist, &Permission::ManageUsers);
    assert!(client.check_permission(&optometrist, &Permission::ManageUsers));
}

#[test]
fn test_role_delegation() {
    let (env, client, admin) = setup_test();

    let pt1 = Address::generate(&env);
    let pt2 = Address::generate(&env);

    client.register_user(&admin, &pt1, &Role::Patient, &String::from_str(&env, "Pt1"));
    client.register_user(&admin, &pt2, &Role::Patient, &String::from_str(&env, "Pt2"));

    // pt1 delegates the Optometrist role (which has ManageAccess) to pt2 with an expiration.
    let future_time = env.ledger().timestamp() + 86400; // 1 day
    client.delegate_role(&pt1, &pt2, &Role::Optometrist, &future_time);

    // To test the delegation practically, pt2 tries to grant access to a doctor for pt1's records.
    let doctor = Address::generate(&env);
    client.register_user(
        &admin,
        &doctor,
        &Role::Optometrist,
        &String::from_str(&env, "Doc"),
    );

    // pt2 should be able to grant access acting for pt1
    // (caller: pt2, patient: pt1, grantee: doctor)
    client.grant_access(&pt2, &pt1, &doctor, &super::AccessLevel::Read, &3600);

    assert_eq!(client.check_access(&pt1, &doctor), super::AccessLevel::Read);
}

#[test]
fn test_role_delegation_expiration() {
    let (env, client, admin) = setup_test();

    let delegator = Address::generate(&env);
    let delegatee = Address::generate(&env);

    client.register_user(
        &admin,
        &delegator,
        &Role::Patient,
        &String::from_str(&env, "Delegator"),
    );
    client.register_user(
        &admin,
        &delegatee,
        &Role::Patient,
        &String::from_str(&env, "Delegatee"),
    );

    // Delegate role expiring immediately (timestamp 0 or already passed)
    // env.ledger().timestamp() is typically 0 at setup, we can advance it.
    env.ledger().set_timestamp(100);

    let expire_at = 50; // In the past
    client.delegate_role(&delegator, &delegatee, &Role::Patient, &expire_at);

    let doctor = Address::generate(&env);
    client.register_user(
        &admin,
        &doctor,
        &Role::Optometrist,
        &String::from_str(&env, "Doc"),
    );

    // Delegatee attempts to act for Delegator and should FAIL
    let result = client.try_grant_access(
        &delegatee,
        &delegator,
        &doctor,
        &super::AccessLevel::Read,
        &3600,
    );
    assert!(result.is_err());
}

// ====================== Delegation-only & edge case tests ======================

#[test]
fn test_delegation_only_permission() {
    // User with no useful direct permissions can act through a delegation
    let (env, client, admin) = setup_test();

    let delegatee = Address::generate(&env);
    client.register_user(
        &admin,
        &delegatee,
        &Role::Patient,
        &String::from_str(&env, "Delegatee"),
    );

    // Patient has no ManageUsers permission directly
    assert!(!client.check_permission(&delegatee, &Permission::ManageUsers));
    assert!(!client.check_permission(&delegatee, &Permission::WriteRecord));

    // Admin delegates Optometrist role (has ManageUsers, WriteRecord) to delegatee
    let future = env.ledger().timestamp() + 86400;
    client.delegate_role(&admin, &delegatee, &Role::Optometrist, &future);

    // Delegatee now has permissions through the delegation
    assert!(client.check_permission(&delegatee, &Permission::ManageUsers));
    assert!(client.check_permission(&delegatee, &Permission::WriteRecord));

    // But still no SystemAdmin (Optometrist role doesn't include it)
    assert!(!client.check_permission(&delegatee, &Permission::SystemAdmin));
}

#[test]
fn test_delegation_only_can_register_user() {
    // User with only a delegation that grants ManageUsers can call register_user
    let (env, client, admin) = setup_test();

    let delegatee = Address::generate(&env);
    client.register_user(
        &admin,
        &delegatee,
        &Role::Patient,
        &String::from_str(&env, "Delegatee"),
    );

    // Delegate Staff role (has ManageUsers) from admin to delegatee
    let future = env.ledger().timestamp() + 86400;
    client.delegate_role(&admin, &delegatee, &Role::Staff, &future);

    // Delegatee can now register a new user through delegated ManageUsers
    let new_user = Address::generate(&env);
    client.register_user(
        &delegatee,
        &new_user,
        &Role::Patient,
        &String::from_str(&env, "NewUser"),
    );

    let user_data = client.get_user(&new_user);
    assert_eq!(user_data.role, Role::Patient);
}

#[test]
fn test_expired_assignment_active_delegation() {
    // User's direct assignment expired, but active delegation still grants permission
    let (env, client, admin) = setup_test();

    let contract_id = env.register(VisionRecordsContract, ());

    let user = Address::generate(&env);
    client.register_user(
        &admin,
        &user,
        &Role::Patient,
        &String::from_str(&env, "User"),
    );

    // Overwrite user's assignment with one that expires at timestamp 50
    env.as_contract(&contract_id, || {
        crate::rbac::assign_role(&env, user.clone(), Role::Staff, 50);
    });

    // At timestamp 0, Staff assignment is active → has ManageUsers
    assert!(client.check_permission(&user, &Permission::ManageUsers));

    // Delegate Optometrist role from admin (has WriteRecord) to user
    client.delegate_role(&admin, &user, &Role::Optometrist, &0);

    // Advance time past the assignment expiry
    env.ledger().set_timestamp(100);

    // Direct Staff assignment has expired → no direct ManageUsers
    // But delegation (expires_at=0, never expires) grants WriteRecord
    assert!(client.check_permission(&user, &Permission::WriteRecord));
    assert!(client.check_permission(&user, &Permission::ManageUsers));
}

#[test]
fn test_revoked_permission_blocks_delegation() {
    // Explicit revoke on direct assignment blocks delegation for that permission
    let (env, client, admin) = setup_test();

    let user = Address::generate(&env);
    client.register_user(
        &admin,
        &user,
        &Role::Optometrist,
        &String::from_str(&env, "Opto"),
    );

    // Optometrist has ManageUsers through base role
    assert!(client.check_permission(&user, &Permission::ManageUsers));

    // Admin explicitly revokes ManageUsers from this user
    client.revoke_custom_permission(&admin, &user, &Permission::ManageUsers);
    assert!(!client.check_permission(&user, &Permission::ManageUsers));

    // Another admin delegates Admin role (has ManageUsers) to this user
    let future = env.ledger().timestamp() + 86400;
    client.delegate_role(&admin, &user, &Role::Admin, &future);

    // Revoke still blocks ManageUsers even though delegation would grant it.
    // This prevents circumventing explicit admin revocations.
    assert!(!client.check_permission(&user, &Permission::ManageUsers));

    // Other non-revoked permissions still work through either path
    assert!(client.check_permission(&user, &Permission::WriteRecord));
}

#[test]
fn test_multiple_delegations_evaluated() {
    // User with delegations from multiple delegators
    let (env, client, admin) = setup_test();

    let user = Address::generate(&env);
    client.register_user(
        &admin,
        &user,
        &Role::Patient,
        &String::from_str(&env, "User"),
    );

    let delegator1 = Address::generate(&env);
    let delegator2 = Address::generate(&env);
    client.register_user(
        &admin,
        &delegator1,
        &Role::Patient,
        &String::from_str(&env, "Del1"),
    );
    client.register_user(
        &admin,
        &delegator2,
        &Role::Patient,
        &String::from_str(&env, "Del2"),
    );

    // delegator1 delegates Staff role (ManageUsers only, no WriteRecord)
    let future = env.ledger().timestamp() + 86400;
    client.delegate_role(&delegator1, &user, &Role::Staff, &future);

    // User has ManageUsers but not WriteRecord
    assert!(client.check_permission(&user, &Permission::ManageUsers));
    assert!(!client.check_permission(&user, &Permission::WriteRecord));

    // delegator2 delegates Optometrist role (ManageUsers + WriteRecord)
    client.delegate_role(&delegator2, &user, &Role::Optometrist, &future);

    // Now user has both through combined delegations
    assert!(client.check_permission(&user, &Permission::ManageUsers));
    assert!(client.check_permission(&user, &Permission::WriteRecord));
}
