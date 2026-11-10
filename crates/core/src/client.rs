//! The signed OCI client, set up the way oci-arm-catcher talks to OCI.

use oci_request_signer::{Client, Profile, Result};

/// Send every request to this base URL instead of `https://<service>.<region>.oraclecloud.com`.
/// For tests and demos against a local mock; not for real use.
pub const ENDPOINT_OVERRIDE_VAR: &str = "OCI_ARM_CATCHER_ENDPOINT";

/// A client for the profile's tenancy and home region, with oci-arm-catcher's user agent and
/// [`ENDPOINT_OVERRIDE_VAR`] honoured.
pub fn connect(profile: &Profile) -> Result<Client> {
    Client::builder(profile)
        .user_agent(concat!("oci-arm-catcher/", env!("CARGO_PKG_VERSION")))
        .endpoint(std::env::var(ENDPOINT_OVERRIDE_VAR).ok())
        .build()
}
