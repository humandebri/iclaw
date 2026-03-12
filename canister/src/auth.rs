// where: iclaw/canister/src/auth.rs
// what: Operator allowlist enforcement and persistence for the public canister boundary
// why: Runtime allowlist changes must survive upgrades without forcing every operator update through init args

mod store;

use crate::types::{AllowedPrincipalsResponse, ApiError, ApiErrorCode, CanisterConfig};
use anyhow::Result;
use candid::Principal;
use std::cell::RefCell;
use store::{load_persisted_allowlist, persist_seed_allowlist, save_persisted_allowlist};

#[derive(Clone, Debug, Default)]
struct AccessPolicy {
    allowed_principals: Vec<Principal>,
}

impl AccessPolicy {
    fn new(allowed_principals: Vec<Principal>) -> Self {
        Self { allowed_principals }
    }

    fn allows(&self, principal: &Principal) -> bool {
        self.allowed_principals
            .iter()
            .any(|allowed| allowed == principal)
    }
}

thread_local! {
    static ACCESS_POLICY: RefCell<AccessPolicy> = RefCell::new(AccessPolicy::default());
}

#[cfg(test)]
thread_local! {
    static TEST_CALLER: RefCell<Option<Principal>> = const { RefCell::new(None) };
}

pub fn init_access(config: Option<&CanisterConfig>) {
    let seeded_allowlist = config
        .and_then(|value| value.allowed_principals.clone())
        .unwrap_or_default();
    let resolved_allowlist = match load_persisted_allowlist() {
        Ok(Some(persisted)) => persisted,
        Ok(None) => {
            let validated = match normalize_allowlist(seeded_allowlist) {
                Ok(validated) => validated,
                Err(error) => trap_with_error(error),
            };
            if let Err(error) = persist_seed_allowlist(&validated) {
                trap_with_storage_error(error);
            }
            validated
        }
        Err(error) => trap_with_storage_error(error),
    };
    set_access_policy(resolved_allowlist);
}

pub fn ensure_allowed_caller() -> Result<(), ApiError> {
    let caller = current_caller();
    let allowed = ACCESS_POLICY.with(|cell| cell.borrow().allows(&caller));

    if allowed {
        Ok(())
    } else {
        Err(ApiError::new(
            ApiErrorCode::Unauthorized,
            format!("caller {caller} is not allowed to use this canister"),
        ))
    }
}

pub fn allowed_principals_get() -> Result<AllowedPrincipalsResponse, ApiError> {
    ensure_allowed_caller()?;
    Ok(AllowedPrincipalsResponse {
        allowed_principals: current_allowed_principals(),
    })
}

pub fn allowed_principals_set(
    request: AllowedPrincipalsResponse,
) -> Result<AllowedPrincipalsResponse, ApiError> {
    ensure_allowed_caller()?;
    let caller = current_caller();
    let allowed_principals = normalize_allowlist(request.allowed_principals)?;
    if !allowed_principals
        .iter()
        .any(|principal| principal == &caller)
    {
        return Err(ApiError::new(
            ApiErrorCode::InvalidArgument,
            "allowed_principals must include the caller to avoid operator lockout",
        ));
    }
    save_persisted_allowlist(&allowed_principals).map_err(storage_error_to_api)?;
    set_access_policy(allowed_principals.clone());
    Ok(AllowedPrincipalsResponse { allowed_principals })
}

#[cfg(test)]
pub fn set_test_caller(principal: Principal) {
    TEST_CALLER.with(|cell| {
        *cell.borrow_mut() = Some(principal);
    });
}

#[cfg(test)]
pub fn clear_test_caller() {
    TEST_CALLER.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

#[cfg(test)]
pub fn clear_test_persisted_allowlist() {
    store::clear_test_persisted_allowlist();
}

fn current_caller() -> Principal {
    #[cfg(test)]
    {
        TEST_CALLER.with(|cell| (*cell.borrow()).unwrap_or_else(Principal::anonymous))
    }

    #[cfg(not(test))]
    {
        ic_cdk::api::msg_caller()
    }
}

fn current_allowed_principals() -> Vec<Principal> {
    ACCESS_POLICY.with(|cell| cell.borrow().allowed_principals.clone())
}

fn set_access_policy(allowed_principals: Vec<Principal>) {
    ACCESS_POLICY.with(|cell| {
        *cell.borrow_mut() = AccessPolicy::new(allowed_principals);
    });
}

fn normalize_allowlist(allowed_principals: Vec<Principal>) -> Result<Vec<Principal>, ApiError> {
    let mut normalized = Vec::new();
    for principal in allowed_principals {
        if normalized.iter().all(|existing| existing != &principal) {
            normalized.push(principal);
        }
    }
    if normalized.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidArgument,
            "allowed_principals must include at least one operator principal",
        ));
    }
    Ok(normalized)
}

fn trap_with_error(error: ApiError) -> ! {
    ic_cdk::trap(&format!("{}: {}", error.code, error.message))
}

fn trap_with_storage_error(error: anyhow::Error) -> ! {
    ic_cdk::trap(&format!("failed to load persisted allowlist: {error}"))
}

fn storage_error_to_api(error: anyhow::Error) -> ApiError {
    ApiError::new(
        ApiErrorCode::MemoryError,
        format!("failed to persist allowlist: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operator_config() -> CanisterConfig {
        CanisterConfig {
            provider: None,
            context: None,
            allowed_principals: Some(vec![Principal::anonymous()]),
        }
    }

    fn reset_test_state() {
        clear_test_caller();
        clear_test_persisted_allowlist();
        set_access_policy(Vec::new());
    }

    #[test]
    fn init_requires_non_empty_allowlist() {
        reset_test_state();
        let result = std::panic::catch_unwind(init_access_without_seed);
        assert!(result.is_err());
        reset_test_state();
    }

    #[test]
    fn ensure_allowed_caller_rejects_principal_outside_allowlist() {
        reset_test_state();
        init_access(Some(&operator_config()));
        set_test_caller(Principal::management_canister());

        let error = ensure_allowed_caller().expect_err("unexpected allow");
        assert_eq!(error.code, ApiErrorCode::Unauthorized.as_str());
        reset_test_state();
    }

    #[test]
    fn allowed_principals_set_rejects_empty_allowlist() {
        reset_test_state();
        init_access(Some(&operator_config()));

        let error = allowed_principals_set(AllowedPrincipalsResponse {
            allowed_principals: Vec::new(),
        })
        .expect_err("empty allowlist should fail");
        assert_eq!(error.code, ApiErrorCode::InvalidArgument.as_str());
        reset_test_state();
    }

    #[test]
    fn allowed_principals_set_rejects_caller_lockout() {
        reset_test_state();
        init_access(Some(&operator_config()));

        let error = allowed_principals_set(AllowedPrincipalsResponse {
            allowed_principals: vec![Principal::management_canister()],
        })
        .expect_err("caller lockout should fail");
        assert_eq!(error.code, ApiErrorCode::InvalidArgument.as_str());
        reset_test_state();
    }

    #[test]
    fn allowed_principals_get_and_set_require_authorized_caller() {
        reset_test_state();
        init_access(Some(&operator_config()));
        set_test_caller(Principal::management_canister());

        let get_error = allowed_principals_get().expect_err("unauthorized get should fail");
        let set_error = allowed_principals_set(AllowedPrincipalsResponse {
            allowed_principals: vec![Principal::management_canister()],
        })
        .expect_err("unauthorized set should fail");
        assert_eq!(get_error.code, ApiErrorCode::Unauthorized.as_str());
        assert_eq!(set_error.code, ApiErrorCode::Unauthorized.as_str());
        reset_test_state();
    }

    #[test]
    fn allowed_principals_set_persists_and_get_returns_updated_list() {
        reset_test_state();
        init_access(Some(&operator_config()));
        let response = allowed_principals_set(AllowedPrincipalsResponse {
            allowed_principals: vec![Principal::anonymous(), Principal::management_canister()],
        })
        .expect("allowlist update should succeed");
        assert_eq!(response.allowed_principals.len(), 2);

        let fetched = allowed_principals_get().expect("allowlist get should succeed");
        assert_eq!(fetched.allowed_principals, response.allowed_principals);
        reset_test_state();
    }

    fn init_access_without_seed() {
        init_access(None);
    }
}
