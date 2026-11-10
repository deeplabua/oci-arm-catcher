//! The handful of OCI endpoints the catcher needs.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use oci_request_signer::Result;
use oci_request_signer::{Client, encode};

/// The Always Free Ampere shape.
pub const A1_FLEX: &str = "VM.Standard.A1.Flex";

const API: &str = "/20160918";

/// Body of `LaunchInstance`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchDetails {
    /// Availability domain name, e.g. `Abcd:EU-AMSTERDAM-1-AD-1`.
    pub availability_domain: String,
    /// Compartment to launch into.
    pub compartment_id: String,
    /// Name shown in the console.
    pub display_name: String,
    /// Shape name.
    pub shape: String,
    /// OCPUs and memory for a flexible shape.
    pub shape_config: ShapeConfig,
    /// Boot image.
    pub source_details: SourceDetails,
    /// Primary VNIC.
    pub create_vnic_details: VnicDetails,
    /// Instance metadata; `ssh_authorized_keys` goes here.
    pub metadata: BTreeMap<String, String>,
}

/// Size of a flexible shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShapeConfig {
    /// Number of OCPUs.
    pub ocpus: f32,
    /// Memory in GB.
    #[serde(rename = "memoryInGBs")]
    pub memory_in_gbs: f32,
}

/// Boot from an image.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDetails {
    /// Always `image`.
    pub source_type: String,
    /// Image OCID.
    pub image_id: String,
    /// Boot volume size; the image default when unset.
    #[serde(
        rename = "bootVolumeSizeInGBs",
        skip_serializing_if = "Option::is_none"
    )]
    pub boot_volume_size_in_gbs: Option<u64>,
}

/// The primary VNIC.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VnicDetails {
    /// Subnet OCID.
    pub subnet_id: String,
    /// Whether to give it a public IP.
    pub assign_public_ip: bool,
}

/// A launched instance.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    /// Instance OCID.
    pub id: String,
    /// Name shown in the console.
    pub display_name: Option<String>,
    /// `PROVISIONING`, `RUNNING`, ...
    pub lifecycle_state: Option<String>,
    /// Region the instance lives in.
    pub region: Option<String>,
}

/// An availability domain.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AvailabilityDomain {
    /// Full name, e.g. `Abcd:EU-AMSTERDAM-1-AD-1`.
    pub name: String,
}

/// A subnet.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subnet {
    /// Subnet OCID.
    pub id: String,
    /// Name shown in the console.
    pub display_name: Option<String>,
    /// CIDR block.
    pub cidr_block: Option<String>,
    /// True for private subnets, which cannot hand out public IPs.
    pub prohibit_public_ip_on_vnic: Option<bool>,
}

/// An OS image.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    /// Image OCID.
    pub id: String,
    /// Name shown in the console.
    pub display_name: Option<String>,
}

/// A region the tenancy is subscribed to.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionSubscription {
    /// Region identifier, e.g. `eu-amsterdam-1`.
    pub region_name: String,
    /// True for the tenancy's home region — the only one where Always Free resources are free.
    #[serde(default)]
    pub is_home_region: bool,
}

/// The OCI calls oci-arm-catcher makes, on top of the signed [`Client`].
pub trait Api {
    /// `LaunchInstance`. `retry_token` makes a retried request idempotent: OCI returns the
    /// instance from the first attempt instead of creating a second one.
    fn launch_instance(&self, details: &LaunchDetails, retry_token: &str) -> Result<Instance>;

    /// `ListAvailabilityDomains` for the tenancy.
    fn availability_domains(&self) -> Result<Vec<AvailabilityDomain>>;

    /// `ListRegionSubscriptions` for the tenancy.
    fn region_subscriptions(&self) -> Result<Vec<RegionSubscription>>;

    /// The tenancy's home region, if OCI reports one.
    fn home_region(&self) -> Result<Option<String>>;

    /// `ListSubnets` in a compartment.
    fn subnets(&self, compartment_id: &str) -> Result<Vec<Subnet>>;

    /// Images for `operating_system` that run on A1.Flex, newest first.
    fn a1_images(&self, compartment_id: &str, operating_system: &str) -> Result<Vec<Image>>;
}

impl Api for Client {
    fn launch_instance(&self, details: &LaunchDetails, retry_token: &str) -> Result<Instance> {
        self.post(
            "iaas",
            &format!("{API}/instances/"),
            details,
            &[("opc-retry-token", retry_token)],
        )
    }

    fn availability_domains(&self) -> Result<Vec<AvailabilityDomain>> {
        self.get(
            "identity",
            &format!(
                "{API}/availabilityDomains?compartmentId={}",
                encode(self.tenancy())
            ),
        )
    }

    fn region_subscriptions(&self) -> Result<Vec<RegionSubscription>> {
        self.get(
            "identity",
            &format!("{API}/tenancies/{}/regionSubscriptions", self.tenancy()),
        )
    }

    fn home_region(&self) -> Result<Option<String>> {
        Ok(self
            .region_subscriptions()?
            .into_iter()
            .find(|r| r.is_home_region)
            .map(|r| r.region_name))
    }

    fn subnets(&self, compartment_id: &str) -> Result<Vec<Subnet>> {
        self.get(
            "iaas",
            &format!(
                "{API}/subnets?compartmentId={}&limit=100",
                encode(compartment_id)
            ),
        )
    }

    fn a1_images(&self, compartment_id: &str, operating_system: &str) -> Result<Vec<Image>> {
        self.get(
            "iaas",
            &format!(
                "{API}/images?compartmentId={}&operatingSystem={}&shape={A1_FLEX}\
                 &sortBy=TIMECREATED&sortOrder=DESC&limit=20",
                encode(compartment_id),
                encode(operating_system)
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_subscriptions_deserialize() {
        let regions: Vec<RegionSubscription> = serde_json::from_str(
            r#"[{"regionKey":"AMS","regionName":"eu-amsterdam-1","status":"READY","isHomeRegion":true},
                {"regionKey":"FRA","regionName":"eu-frankfurt-1","status":"READY"}]"#,
        )
        .unwrap();
        assert!(regions[0].is_home_region);
        assert_eq!(regions[0].region_name, "eu-amsterdam-1");
        assert!(!regions[1].is_home_region);
    }

    #[test]
    fn launch_details_serialize_to_oci_field_names() {
        let details = LaunchDetails {
            availability_domain: "Abcd:EU-AMSTERDAM-1-AD-1".into(),
            compartment_id: "ocid1.tenancy".into(),
            display_name: "arm-free-1".into(),
            shape: A1_FLEX.into(),
            shape_config: ShapeConfig {
                ocpus: 2.0,
                memory_in_gbs: 12.0,
            },
            source_details: SourceDetails {
                source_type: "image".into(),
                image_id: "ocid1.image".into(),
                boot_volume_size_in_gbs: None,
            },
            create_vnic_details: VnicDetails {
                subnet_id: "ocid1.subnet".into(),
                assign_public_ip: true,
            },
            metadata: BTreeMap::from([("ssh_authorized_keys".into(), "ssh-ed25519 AAAA".into())]),
        };
        let json = serde_json::to_value(&details).unwrap();
        assert_eq!(json["availabilityDomain"], "Abcd:EU-AMSTERDAM-1-AD-1");
        assert_eq!(json["shapeConfig"]["memoryInGBs"], 12.0);
        assert_eq!(json["sourceDetails"]["sourceType"], "image");
        assert!(json["sourceDetails"].get("bootVolumeSizeInGBs").is_none());
        assert_eq!(json["createVnicDetails"]["assignPublicIp"], true);
        assert_eq!(json["metadata"]["ssh_authorized_keys"], "ssh-ed25519 AAAA");

        let mut with_boot = details;
        with_boot.source_details.boot_volume_size_in_gbs = Some(100);
        let json = serde_json::to_value(&with_boot).unwrap();
        assert_eq!(json["sourceDetails"]["bootVolumeSizeInGBs"], 100);
    }
}
