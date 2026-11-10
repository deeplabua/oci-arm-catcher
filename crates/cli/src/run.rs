//! The subcommands.

use std::time::Duration;

use anyhow::Result;
use oci_arm_catcher_core::{A1_FLEX, Api as _, Client, Error, catcher};
use oci_arm_catcher_core::{DEFAULT_PROFILE, Profile};

use crate::args::{Args, Command, CommonArgs, DiscoverArgs, LaunchArgs};
use crate::exit::UsageError;
use crate::settings::{self, EnvFile};
use crate::ui::{self, Console};
use crate::update::UpdateCheck;

pub fn run(args: &Args, update: &mut UpdateCheck) -> Result<()> {
    let (env_path, env_file) = settings::load_env_file(&args.common)?;
    let env_path = env_path.as_deref();
    match &args.command {
        None => catch(&args.launch, &args.common, &env_file, env_path, update),
        Some(Command::Run(launch)) => catch(launch, &args.common, &env_file, env_path, update),
        Some(Command::Check) => check(&args.common),
        Some(Command::Discover(discover)) => self::discover(discover, &args.common, &env_file),
    }
}

fn client(common: &CommonArgs) -> Result<Client> {
    let path = match &common.oci_config {
        Some(path) => path.clone(),
        None => Profile::default_path()
            .ok_or_else(|| UsageError::new("cannot find your home directory; pass --oci-config"))?,
    };
    let profile = Profile::load(&path, &common.profile).map_err(usage_if_local)?;
    oci_arm_catcher_core::connect(&profile).map_err(usage_if_local)
}

/// A broken config file or key is the user's to fix, like a bad flag.
fn usage_if_local(error: Error) -> anyhow::Error {
    match error {
        Error::Config { .. } | Error::Key { .. } => UsageError::new(error.to_string()).into(),
        other => other.into(),
    }
}

fn catch(
    launch: &LaunchArgs,
    common: &CommonArgs,
    env_file: &EnvFile,
    env_path: Option<&std::path::Path>,
    update: &mut UpdateCheck,
) -> Result<()> {
    let plan = settings::plan(launch, env_file)?;
    let client = client(common)?;

    println!(
        "{}",
        ui::dim(&format!(
            "=== oci-arm-catcher {} ===",
            env!("CARGO_PKG_VERSION")
        ))
    );
    if let Some(path) = env_path {
        println!("{}", ui::dim(&format!("Settings: {}", path.display())));
    }
    if common.profile != DEFAULT_PROFILE {
        println!("{}", ui::dim(&format!("Profile: {}", common.profile)));
    }
    let d = &plan.details;
    println!(
        "Region: {}  |  {A1_FLEX}  |  OCPU: {}  |  RAM: {} GB",
        client.region(),
        d.shape_config.ocpus,
        d.shape_config.memory_in_gbs
    );
    if let Some(gb) = d.source_details.boot_volume_size_in_gbs {
        println!("Boot volume: {gb} GB");
    }
    let ads = format!(
        "Availability domain{}: {}",
        if plan.availability_domains.len() > 1 {
            "s (rotating)"
        } else {
            ""
        },
        plan.availability_domains.join(", ")
    );
    println!("{}", ui::dim(&ads));
    // Best effort: a policy that hides region subscriptions must not stop the hunt.
    if let Ok(Some(home)) = client.home_region()
        && home != client.region()
    {
        println!("{}", ui::yellow(&not_home_region(client.region(), &home)));
    }
    println!(
        "{}\n",
        ui::dim(&format!("Retry interval: {}s", plan.interval.as_secs()))
    );
    // A hunt can run for days; say it now rather than when it ends.
    update.report(Duration::from_millis(1500));

    match catcher::run(&client, &plan, &mut Console::new()) {
        Ok(instance) => {
            let message = format!("SUCCESS! Instance created: {}", instance.id);
            println!("\n{}", ui::green(&format!("✓ {message}")));
            if let Some(state) = &instance.lifecycle_state {
                println!("State: {state}");
            }
            println!(
                "Console: https://cloud.oracle.com/compute/instances/{}?region={}",
                instance.id,
                client.region()
            );
            ui::notify(&message);
            Ok(())
        }
        Err(error) => {
            ui::notify(&format!("Stopped: {error}"));
            Err(match hint_for(&error) {
                Some(hint) => anyhow::anyhow!("{error}\n\nhint: {hint}"),
                None => error.into(),
            })
        }
    }
}

/// A next step for the errors people actually hit.
fn hint_for(error: &Error) -> Option<&'static str> {
    let Error::Api(api) = error else {
        return None;
    };
    Some(match (api.status, api.code.as_str()) {
        (_, "LimitExceeded") => {
            "You are over your A1 allowance (Always Free: 2 OCPU / 12 GB in total). \
             Lower OCPUS / MEMORY_GB or remove existing A1 instances."
        }
        (401, _) => {
            "Authentication failed. Make sure the key's fingerprint in ~/.oci/config matches \
             the API key uploaded in the console, and that `region` is a region your tenancy \
             is subscribed to (normally your home region) — OCI answers 401 for any other."
        }
        (404, _) => {
            "An OCID was not found or you lack access. Check that COMPARTMENT_ID, SUBNET_ID and \
             IMAGE_ID are from the same region as your profile."
        }
        _ => return None,
    })
}

fn check(common: &CommonArgs) -> Result<()> {
    let client = client(common)?;
    let ads = client
        .availability_domains()
        .map_err(|error| match &error {
            Error::Api(api) if api.status == 401 => {
                anyhow::anyhow!("{error}\n\nhint: {}", hint_for(&error).unwrap_or_default())
            }
            _ => anyhow::Error::from(error).context("signed request to OCI failed"),
        })?;
    println!(
        "OK: authenticated to {} as tenancy {}",
        client.region(),
        client.tenancy()
    );
    match client.home_region().ok().flatten() {
        Some(home) if home == client.region() => {
            println!("  home region: {home} (Always Free applies here)")
        }
        Some(home) => println!("{}", ui::yellow(&not_home_region(client.region(), &home))),
        None => {}
    }
    for ad in ads {
        println!("  availability domain: {}", ad.name);
    }
    Ok(())
}

/// Always Free instances can only be created in the tenancy's home region.
fn not_home_region(region: &str, home: &str) -> String {
    format!(
        "warning: {region} is not your home region ({home}). Always Free instances can only be \
         created in the home region; set region={home} in ~/.oci/config or pick another --profile."
    )
}

fn discover(args: &DiscoverArgs, common: &CommonArgs, env_file: &EnvFile) -> Result<()> {
    let client = client(common)?;
    let compartment = args
        .compartment_id
        .clone()
        .or_else(|| env_file.get("COMPARTMENT_ID").cloned())
        .unwrap_or_else(|| client.tenancy().to_owned());

    println!(
        "# Region: {} — paste what you need into .env\n",
        client.region()
    );
    println!("COMPARTMENT_ID=\"{compartment}\"");

    let ads = client.availability_domains()?;
    let names: Vec<_> = ads.iter().map(|ad| ad.name.as_str()).collect();
    if names.len() > 1 {
        println!("AVAILABILITY_DOMAINS=\"{}\"", names.join(","));
    } else if let Some(ad) = names.first() {
        println!("AVAILABILITY_DOMAIN=\"{ad}\"");
    }

    let subnets = client.subnets(&compartment)?;
    let public: Vec<_> = subnets
        .iter()
        .filter(|s| s.prohibit_public_ip_on_vnic != Some(true))
        .collect();
    match public.as_slice() {
        [] => println!(
            "# No public subnet in this compartment. Create a VCN with the console's \
             \"VCN Wizard\" (Internet connectivity), then run discover again."
        ),
        [only] => println!("SUBNET_ID=\"{}\"", only.id),
        many => {
            println!("# Public subnets (pick one):");
            for (i, s) in many.iter().enumerate() {
                let line = format!("SUBNET_ID=\"{}\"", s.id);
                let name = s.display_name.as_deref().unwrap_or("");
                let cidr = s.cidr_block.as_deref().unwrap_or("");
                if i == 0 {
                    println!("{line}  # {name} {cidr}");
                } else {
                    println!("# {line}  # {name} {cidr}");
                }
            }
        }
    }

    let images = client.a1_images(&compartment, &args.os)?;
    match images.first() {
        Some(image) => println!(
            "IMAGE_ID=\"{}\"  # {}",
            image.id,
            image.display_name.as_deref().unwrap_or(&args.os)
        ),
        None => println!(
            "# No A1 image found for \"{}\"; try --os \"Oracle Linux\"",
            args.os
        ),
    }

    for key in ["id_ed25519.pub", "id_rsa.pub"] {
        let path = oci_arm_catcher_core::expand_home(&format!("~/.ssh/{key}"));
        if path.is_file() {
            // $HOME keeps the line portable; .env parsing expands it.
            println!("SSH_KEY_FILE=\"$HOME/.ssh/{key}\"");
            break;
        }
    }
    println!("OCPUS=2\nMEMORY_GB=12");
    Ok(())
}
