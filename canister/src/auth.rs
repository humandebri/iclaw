// where: standalone/canister/src/auth.rs
// what: Caller allowlist enforcement for the public canister boundary
// why: Internet Identity in the UI is not sufficient; the canister must reject unauthorized principals itself

use crate::types::{ApiError, ApiErrorCode, CanisterConfig};
use candid::Principal;
use std::cell::RefCell;

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
    let allowed_principals = config
        .and_then(|value| value.allowed_principals.clone())
        .unwrap_or_default();

    if allowed_principals.is_empty() {
        ic_cdk::trap("allowed_principals must include at least one operator principal");
    }

    ACCESS_POLICY.with(|cell| {
        *cell.borrow_mut() = AccessPolicy::new(allowed_principals);
    });
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

    #[test]
    fn init_requires_non_empty_allowlist() {
        let result = std::panic::catch_unwind(|| init_access(None));
        assert!(result.is_err());
    }

    #[test]
    fn ensure_allowed_caller_rejects_principal_outside_allowlist() {
        init_access(Some(&operator_config()));
        set_test_caller(Principal::management_canister());

        let error = ensure_allowed_caller().expect_err("unexpected allow");
        assert_eq!(error.code, ApiErrorCode::Unauthorized.as_str());

        clear_test_caller();
    }
}
