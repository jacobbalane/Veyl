//! Read-only endpoint metadata for development; no stream is initialized or captured.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let monitor = veyl_desktop_lib::pc_devices::DeviceMonitor::start()?;
    let mut updates = monitor.subscribe();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while updates.borrow().error.as_deref() == Some("loading") {
            updates.changed().await?;
        }
        Ok::<_, tokio::sync::watch::error::RecvError>(())
    })
    .await??;
    let devices = monitor.snapshot();
    println!(
        "Speakers: {}; microphones: {}; recommendation: {}; error: {}.",
        devices.speakers.len(),
        devices.microphones.len(),
        devices.recommended_source,
        devices.error.as_deref().unwrap_or("none")
    );
    for (kind, endpoints) in [
        ("Render", &devices.speakers),
        ("Capture", &devices.microphones),
    ] {
        for device in endpoints {
            println!(
                "{kind}: {} · {} · raw: {} · Bluetooth: {} · built in: {}",
                device.name,
                device.form_factor,
                device.raw_supported,
                device.bluetooth,
                device.built_in
            );
        }
    }
    monitor.shutdown().await;
    Ok(())
}
