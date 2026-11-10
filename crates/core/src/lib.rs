//! `oci-arm-catcher-core` — talks to the Oracle Cloud API directly (no OCI CLI, no Python)
//! and retries `LaunchInstance` until free Ampere A1 capacity turns up.
//!
//! - [`client`]: the signed client ([`oci_request_signer`] underneath)
//! - [`api`]: the few endpoints we call
//! - [`catcher`]: error classification and the retry loop

pub mod api;
pub mod catcher;
pub mod client;

pub use api::{A1_FLEX, Api, Instance, LaunchDetails};
pub use catcher::{Environment, Event, Plan, RetryReason};
pub use client::connect;
pub use oci_request_signer::{
    ApiError, Client, DEFAULT_PROFILE, Error, Profile, Result, expand_home,
};
