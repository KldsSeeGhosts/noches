fn main() {
    for key in [
        "NOCHES_CHANNEL",
        "NOCHES_VERSION",
        "NOCHES_REPOSITORY",
        "NOCHES_COMMIT",
        "NOCHES_BRIDGE_EPOCH",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    println!("cargo:rustc-check-cfg=cfg(noches_bridge_epoch)");
    match std::env::var("NOCHES_BRIDGE_EPOCH").as_deref() {
        Ok("0") => {
            assert!(
                matches!(
                    std::env::var("NOCHES_CHANNEL").as_deref(),
                    Ok("dev" | "stable")
                ),
                "bridge epoch requires a distributed channel"
            );
            println!("cargo:rustc-cfg=noches_bridge_epoch");
        }
        Ok("") | Err(std::env::VarError::NotPresent) => {}
        _ => panic!("NOCHES_BRIDGE_EPOCH must be 0 or unset"),
    }
    let channel = std::env::var("NOCHES_CHANNEL").unwrap_or_else(|_| "local".into());
    let service = match channel.as_str() {
        "dev" => "noches-dev.service",
        "stable" => "noches.service",
        _ => "zeron.service",
    };
    let label = match channel.as_str() {
        "dev" => "io.github.kldsseeghosts.noches.dev",
        "stable" => "io.github.kldsseeghosts.noches",
        _ => "sh.zeron.app",
    };
    println!("cargo:rustc-env=NOCHES_SERVICE_NAME={service}");
    println!("cargo:rustc-env=NOCHES_LAUNCHD_LABEL={label}");
    if let Ok(channel) = std::env::var("NOCHES_CHANNEL") {
        assert!(
            matches!(channel.as_str(), "local" | "dev" | "stable"),
            "invalid NOCHES_CHANNEL"
        );
    }
}
