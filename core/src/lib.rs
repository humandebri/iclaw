// where: standalone/core/src/lib.rs
// what: standalone shared contracts used only by the extracted canister workspace
// why: the canister/web/tests stack must compile without depending on the root runtime workspace

#![forbid(unsafe_code)]

pub mod memory;
pub mod providers;
pub mod tools;
